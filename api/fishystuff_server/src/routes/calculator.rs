use std::cmp::Reverse;
use std::collections::HashMap;
use std::convert::Infallible;
use std::fmt::Write as _;

use async_stream::stream;
use axum::body::Bytes;
use axum::extract::{rejection::QueryRejection, Extension, Query, State};
use axum::http::{header, HeaderMap, HeaderValue};
use axum::response::{sse::Event, Html, IntoResponse, Sse};
use axum::Json;
use datastar::prelude::{DatastarEvent, ElementPatchMode, PatchElements, PatchSignals};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use serde::Deserialize;
use serde_json::{json, Value};

use fishystuff_api::models::calculator::{
    CalculatorCatalogResponse, CalculatorItemEntry, CalculatorLifeskillLevelEntry,
    CalculatorOptionEntry, CalculatorPetCatalog, CalculatorPetSignals,
    CalculatorSessionPresetEntry, CalculatorSignals,
};
use fishystuff_api::models::zones::ZoneEntry;

use crate::error::{with_timeout, AppError, AppResult};
use crate::routes::meta::map_request_id;
use crate::state::{RequestId, SharedState};
use crate::store::FishLang;

#[derive(Debug, Deserialize)]
pub struct CalculatorQuery {
    pub lang: Option<String>,
    pub r#ref: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CalculatorDatastarQuery {
    pub lang: Option<String>,
    pub r#ref: Option<String>,
    pub datastar: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CalculatorZoneSearchQuery {
    pub lang: Option<String>,
    pub r#ref: Option<String>,
    pub q: Option<String>,
    pub selected: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct CalculatorDerivedSignals {
    zone_name: String,
    abundance_label: String,
    zone_bite_min: String,
    zone_bite_max: String,
    zone_bite_avg: String,
    effective_bite_min: String,
    effective_bite_max: String,
    effective_bite_avg: String,
    total_time: String,
    bite_time: String,
    auto_fish_time: String,
    auto_fish_time_reduction_text: String,
    casts_title: String,
    casts_average: String,
    durability_reduction_resistance_text: String,
    chance_to_consume_durability_text: String,
    durability_loss_title: String,
    durability_loss_average: String,
    timespan_text: String,
    bite_time_title: String,
    auto_fish_time_title: String,
    catch_time_title: String,
    unoptimized_time_title: String,
    show_auto_fishing: bool,
    percent_bite: String,
    percent_af: String,
    percent_catch: String,
    debug_json: String,
}

#[derive(Debug)]
struct CalculatorData {
    catalog: CalculatorCatalogResponse,
    lang: FishLang,
    zones: Vec<ZoneEntry>,
}

#[derive(Debug, Clone)]
struct SelectOption<'a> {
    value: &'a str,
    label: &'a str,
    icon: Option<&'a str>,
}

struct SearchableDropdownConfig<'a> {
    root_id: &'a str,
    input_id: &'a str,
    label: &'a str,
    value: &'a str,
    search_url: &'a str,
    search_url_root: Option<&'a str>,
    search_placeholder: &'a str,
}

const SEARCHABLE_DROPDOWN_RESULT_LIMIT: usize = 24;

pub async fn get_calculator_catalog(
    State(state): State<SharedState>,
    query: Result<Query<CalculatorQuery>, QueryRejection>,
    Extension(request_id): Extension<RequestId>,
) -> AppResult<(HeaderMap, Json<CalculatorCatalogResponse>)> {
    let Query(query) = query.map_err(|err| {
        AppError::invalid_argument(err.to_string()).with_request_id(request_id.0.clone())
    })?;

    let lang = FishLang::from_param(query.lang.as_deref());
    let data = load_calculator_data(&state, lang, query.r#ref, &request_id).await?;

    let mut headers = HeaderMap::new();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok((headers, Json(data.catalog)))
}

pub async fn get_calculator_datastar_init(
    State(state): State<SharedState>,
    query: Result<Query<CalculatorDatastarQuery>, QueryRejection>,
    Extension(request_id): Extension<RequestId>,
) -> AppResult<impl IntoResponse> {
    let Query(query) = query.map_err(|err| {
        AppError::invalid_argument(err.to_string()).with_request_id(request_id.0.clone())
    })?;

    let lang = FishLang::from_param(query.lang.as_deref());
    let data = load_calculator_data(&state, lang, query.r#ref.clone(), &request_id).await?;
    let raw_signals = match query.datastar.as_deref() {
        Some(payload) if !payload.trim().is_empty() => {
            let value = serde_json::from_str::<Value>(payload).map_err(|err| {
                AppError::invalid_argument(format!("invalid datastar query payload: {err}"))
                    .with_request_id(request_id.0.clone())
            })?;
            parse_calculator_signals_value(value, &data.catalog.defaults, &request_id)?
        }
        _ => data.catalog.defaults.clone(),
    };

    calculator_datastar_init_response(&data, raw_signals)
}

pub async fn post_calculator_datastar_init(
    State(state): State<SharedState>,
    query: Result<Query<CalculatorQuery>, QueryRejection>,
    Extension(request_id): Extension<RequestId>,
    body: Bytes,
) -> AppResult<impl IntoResponse> {
    let Query(query) = query.map_err(|err| {
        AppError::invalid_argument(err.to_string()).with_request_id(request_id.0.clone())
    })?;

    let lang = FishLang::from_param(query.lang.as_deref());
    let data = load_calculator_data(&state, lang, query.r#ref.clone(), &request_id).await?;
    let raw_signals = parse_calculator_signals_body(&body, &data.catalog.defaults, &request_id)?;

    calculator_datastar_init_response(&data, raw_signals)
}

pub async fn post_calculator_datastar_eval(
    State(state): State<SharedState>,
    query: Result<Query<CalculatorQuery>, QueryRejection>,
    Extension(request_id): Extension<RequestId>,
    body: Bytes,
) -> AppResult<impl IntoResponse> {
    let Query(query) = query.map_err(|err| {
        AppError::invalid_argument(err.to_string()).with_request_id(request_id.0.clone())
    })?;

    let lang = FishLang::from_param(query.lang.as_deref());
    let data = load_calculator_data(&state, lang, query.r#ref.clone(), &request_id).await?;
    let raw_signals = parse_calculator_signals_body(&body, &data.catalog.defaults, &request_id)?;
    let (normalized_signals, derived) = normalize_and_derive(raw_signals, &data);
    let events =
        vec![
            calculator_signals_event(&normalized_signals, &derived, CalculatorPatchMode::Eval)?
                .into_datastar_event(),
        ];
    Ok(calculator_datastar_response(events))
}

pub async fn get_calculator_datastar_zone_search(
    State(state): State<SharedState>,
    query: Result<Query<CalculatorZoneSearchQuery>, QueryRejection>,
    Extension(request_id): Extension<RequestId>,
) -> AppResult<impl IntoResponse> {
    let Query(query) = query.map_err(|err| {
        AppError::invalid_argument(err.to_string()).with_request_id(request_id.0.clone())
    })?;

    let lang = FishLang::from_param(query.lang.as_deref());
    let data = load_calculator_data(&state, lang, query.r#ref.clone(), &request_id).await?;
    let selected_zone = query
        .selected
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(data.catalog.defaults.zone.as_str());
    let search_text = query.q.unwrap_or_default();
    let fragment = render_zone_search_results(
        "calculator-zone-search-results",
        &data.zones,
        selected_zone,
        &search_text,
    );
    let mut headers = HeaderMap::new();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok((headers, Html(fragment)))
}

fn calculator_datastar_init_response(
    data: &CalculatorData,
    raw_signals: CalculatorSignals,
) -> AppResult<impl IntoResponse> {
    let (normalized_signals, derived) = normalize_and_derive(raw_signals, data);
    let app = render_calculator_app(data, &normalized_signals, &derived)?;
    let events = vec![
        calculator_signals_event(&normalized_signals, &derived, CalculatorPatchMode::Init)?
            .into_datastar_event(),
        PatchElements::new(app)
            .selector("#calculator-app")
            .mode(ElementPatchMode::Outer)
            .into_datastar_event(),
    ];
    Ok(calculator_datastar_response(events))
}

fn calculator_datastar_response(events: Vec<DatastarEvent>) -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    let stream = stream! {
        for event in events {
            yield Ok::<Event, Infallible>(datastar_event_to_axum_event(event));
        }
    };
    (headers, Sse::new(stream))
}

fn datastar_event_to_axum_event(event: DatastarEvent) -> Event {
    let event_name = match event.event {
        datastar::consts::EventType::PatchElements => "datastar-patch-elements",
        datastar::consts::EventType::PatchSignals => "datastar-patch-signals",
    };
    let event_builder = Event::default().event(event_name);
    let event_builder =
        if event.retry.as_millis() != datastar::consts::DEFAULT_SSE_RETRY_DURATION as u128 {
            event_builder.retry(event.retry)
        } else {
            event_builder
        };
    let event_builder = match event.id.as_deref() {
        Some(id) => event_builder.id(id),
        None => event_builder,
    };
    event_builder.data(event.data.join("\n"))
}

fn parse_calculator_signals_body(
    body: &Bytes,
    defaults: &CalculatorSignals,
    request_id: &RequestId,
) -> AppResult<CalculatorSignals> {
    if body.is_empty() {
        return Ok(defaults.clone());
    }
    let value = serde_json::from_slice::<Value>(body).map_err(|err| {
        AppError::invalid_argument(format!("invalid calculator request body: {err}"))
            .with_request_id(request_id.0.clone())
    })?;
    parse_calculator_signals_value(value, defaults, request_id)
}

fn calculator_signals_event(
    signals: &CalculatorSignals,
    derived: &CalculatorDerivedSignals,
    mode: CalculatorPatchMode,
) -> AppResult<PatchSignals> {
    let mut patch = match mode {
        CalculatorPatchMode::Init => init_signals_patch_map(signals)?,
        CalculatorPatchMode::Eval => serde_json::Map::new(),
    };
    if matches!(mode, CalculatorPatchMode::Init) {
        patch.insert("_loading".to_string(), Value::Bool(false));
    }
    patch.insert(
        "_calc".to_string(),
        serde_json::to_value(derived).map_err(|err| {
            AppError::internal(format!("serialize calculator derived signals: {err}"))
        })?,
    );
    let signals = serde_json::to_string(&Value::Object(patch))
        .map_err(|err| AppError::internal(format!("serialize calculator patch signals: {err}")))?;
    Ok(PatchSignals::new(signals))
}

#[derive(Clone, Copy)]
enum CalculatorPatchMode {
    Init,
    Eval,
}

fn parse_calculator_signals_value(
    mut value: Value,
    defaults: &CalculatorSignals,
    request_id: &RequestId,
) -> AppResult<CalculatorSignals> {
    merge_missing_signal_values(
        &mut value,
        &serde_json::to_value(defaults)
            .map_err(|err| AppError::internal(format!("serialize calculator defaults: {err}")))?,
    );

    let mut object = match value {
        Value::Object(object) => object,
        _ => {
            return Err(
                AppError::invalid_argument("calculator payload must be a JSON object")
                    .with_request_id(request_id.0.clone()),
            );
        }
    };

    apply_local_signal_aliases(&mut object);

    coerce_object_i64(&mut object, "level");
    coerce_object_f64(&mut object, "resources");
    coerce_object_f64(&mut object, "catchTimeActive");
    coerce_object_f64(&mut object, "catchTimeAfk");
    coerce_object_f64(&mut object, "timespanAmount");
    coerce_object_bool(&mut object, "brand");
    coerce_object_bool(&mut object, "active");
    coerce_object_bool(&mut object, "debug");

    for key in ["pet1", "pet2", "pet3", "pet4", "pet5"] {
        if let Some(Value::Object(pet)) = object.get_mut(key) {
            coerce_nested_string(pet, "tier");
            coerce_nested_string(pet, "special");
            coerce_nested_string(pet, "talent");
            coerce_nested_string_array(pet, "skills");
        }
    }

    serde_json::from_value(Value::Object(object)).map_err(|err| {
        AppError::invalid_argument(format!("invalid calculator payload after coercion: {err}"))
            .with_request_id(request_id.0.clone())
    })
}

fn apply_local_signal_aliases(object: &mut serde_json::Map<String, Value>) {
    alias_local_signal(object, "_resources", "resources");
}

fn alias_local_signal(object: &mut serde_json::Map<String, Value>, alias: &str, key: &str) {
    if object.contains_key(key) {
        object.remove(alias);
        return;
    }
    if let Some(value) = object.remove(alias) {
        object.insert(key.to_string(), value);
    }
}

fn merge_missing_signal_values(value: &mut Value, defaults: &Value) {
    if let (Value::Object(object), Value::Object(default_object)) = (&mut *value, defaults) {
        for (key, default_value) in default_object {
            if let Some(current_value) = object.get_mut(key) {
                merge_missing_signal_values(current_value, default_value);
            } else {
                object.insert(key.clone(), default_value.clone());
            }
        }
        return;
    }

    if matches!(value, Value::Null) {
        *value = defaults.clone();
    }
}

fn coerce_object_i64(object: &mut serde_json::Map<String, Value>, key: &str) {
    if let Some(value) = object.get_mut(key) {
        coerce_value_i64(value);
    }
}

fn coerce_object_f64(object: &mut serde_json::Map<String, Value>, key: &str) {
    if let Some(value) = object.get_mut(key) {
        coerce_value_f64(value);
    }
}

fn coerce_object_bool(object: &mut serde_json::Map<String, Value>, key: &str) {
    if let Some(value) = object.get_mut(key) {
        coerce_value_bool(value);
    }
}

fn coerce_nested_string(object: &mut serde_json::Map<String, Value>, key: &str) {
    if let Some(value) = object.get_mut(key) {
        if let Some(string) = value.as_str() {
            *value = Value::String(string.to_string());
        } else if let Some(number) = value.as_i64() {
            *value = Value::String(number.to_string());
        }
    }
}

fn coerce_nested_string_array(object: &mut serde_json::Map<String, Value>, key: &str) {
    if let Some(value) = object.get_mut(key) {
        match value {
            Value::String(string) => {
                *value = Value::Array(vec![Value::String(string.clone())]);
            }
            Value::Array(values) => {
                *values = values
                    .iter()
                    .filter_map(|value| match value {
                        Value::String(string) => Some(Value::String(string.clone())),
                        Value::Number(number) => Some(Value::String(number.to_string())),
                        _ => None,
                    })
                    .collect();
            }
            _ => {}
        }
    }
}

fn coerce_value_i64(value: &mut Value) {
    if let Value::String(string) = value {
        if let Ok(parsed) = string.trim().parse::<i64>() {
            *value = Value::Number(parsed.into());
        }
    }
}

fn coerce_value_f64(value: &mut Value) {
    if let Value::String(string) = value {
        if let Ok(parsed) = string.trim().parse::<f64>() {
            if let Some(number) = serde_json::Number::from_f64(parsed) {
                *value = Value::Number(number);
            }
        }
    }
}

fn coerce_value_bool(value: &mut Value) {
    if let Value::String(string) = value {
        match string.trim().to_ascii_lowercase().as_str() {
            "true" => *value = Value::Bool(true),
            "false" => *value = Value::Bool(false),
            _ => {}
        }
    }
}

async fn load_calculator_data(
    state: &SharedState,
    lang: FishLang,
    ref_id: Option<String>,
    request_id: &RequestId,
) -> AppResult<CalculatorData> {
    let catalog = with_timeout(
        state.config.request_timeout_secs,
        state.store.calculator_catalog(lang, ref_id.clone()),
    )
    .await
    .map_err(|err| map_request_id(err, request_id))?;
    let zones = with_timeout(
        state.config.request_timeout_secs,
        state.store.list_zones(ref_id),
    )
    .await
    .map_err(|err| map_request_id(err, request_id))?;
    Ok(CalculatorData {
        catalog,
        lang,
        zones,
    })
}

fn lang_param(lang: FishLang) -> &'static str {
    match lang {
        FishLang::En => "en",
        FishLang::Ko => "ko",
    }
}

fn normalize_and_derive(
    raw_signals: CalculatorSignals,
    data: &CalculatorData,
) -> (CalculatorSignals, CalculatorDerivedSignals) {
    let mut signals = raw_signals;
    normalize_signals(&mut signals, data);
    let derived = derive_signals(&signals, data);
    (signals, derived)
}

fn normalize_signals(signals: &mut CalculatorSignals, data: &CalculatorData) {
    let defaults = data.catalog.defaults.clone();
    let item_name_to_key = data
        .catalog
        .items
        .iter()
        .map(|item| (normalize_lookup_value(&item.name), item.key.clone()))
        .collect::<HashMap<_, _>>();
    let level_name_to_key = data
        .catalog
        .lifeskill_levels
        .iter()
        .map(|level| (normalize_lookup_value(&level.name), level.key.clone()))
        .collect::<HashMap<_, _>>();
    let zone_name_to_key = data
        .zones
        .iter()
        .filter_map(|zone| {
            zone.name
                .as_ref()
                .map(|name| (normalize_lookup_value(name), zone.rgb_key.to_string()))
        })
        .collect::<HashMap<_, _>>();
    let item_legacy_aliases = HashMap::from([(
        normalize_lookup_value("lil' otter fishing carrier 🦦"),
        "item:830150".to_string(),
    )]);
    let pet_value_aliases = HashMap::from([
        (
            normalize_lookup_value("Auto-Fishing Time Reduction"),
            "auto_fishing_time_reduction".to_string(),
        ),
        (
            normalize_lookup_value("Durability Reduction Resistance"),
            "durability_reduction_resistance".to_string(),
        ),
        (normalize_lookup_value("Life EXP"), "life_exp".to_string()),
        (
            normalize_lookup_value("Fishing EXP"),
            "fishing_exp".to_string(),
        ),
    ]);

    let valid_item_keys = data
        .catalog
        .items
        .iter()
        .map(|item| item.key.clone())
        .collect::<std::collections::HashSet<_>>();
    let valid_level_keys = data
        .catalog
        .lifeskill_levels
        .iter()
        .map(|level| level.key.clone())
        .collect::<std::collections::HashSet<_>>();
    let valid_zone_keys = data
        .zones
        .iter()
        .map(|zone| zone.rgb_key.0.clone())
        .collect::<std::collections::HashSet<_>>();

    signals.level = signals.level.clamp(0, 5);
    signals.resources = signals.resources.clamp(0.0, 100.0);
    signals.catch_time_active = signals.catch_time_active.max(0.0);
    signals.catch_time_afk = signals.catch_time_afk.max(0.0);
    signals.timespan_amount = signals.timespan_amount.max(0.0);

    signals.zone = normalize_named_value_with_fuzzy(
        &signals.zone,
        &valid_zone_keys,
        &zone_name_to_key,
        None,
        defaults.zone.clone(),
        false,
        true,
    );
    signals.lifeskill_level = normalize_named_value_with_fuzzy(
        &signals.lifeskill_level,
        &valid_level_keys,
        &level_name_to_key,
        None,
        defaults.lifeskill_level.clone(),
        false,
        false,
    );
    signals.rod = normalize_named_value_with_fuzzy(
        &signals.rod,
        &valid_item_keys,
        &item_name_to_key,
        Some(&item_legacy_aliases),
        defaults.rod.clone(),
        false,
        false,
    );
    signals.float = normalize_named_value_with_fuzzy(
        &signals.float,
        &valid_item_keys,
        &item_name_to_key,
        Some(&item_legacy_aliases),
        String::new(),
        true,
        false,
    );
    signals.chair = normalize_named_value_with_fuzzy(
        &signals.chair,
        &valid_item_keys,
        &item_name_to_key,
        Some(&item_legacy_aliases),
        defaults.chair.clone(),
        true,
        false,
    );
    signals.lightstone_set = normalize_named_value_with_fuzzy(
        &signals.lightstone_set,
        &valid_item_keys,
        &item_name_to_key,
        Some(&item_legacy_aliases),
        defaults.lightstone_set.clone(),
        true,
        false,
    );
    signals.backpack = normalize_named_value_with_fuzzy(
        &signals.backpack,
        &valid_item_keys,
        &item_name_to_key,
        Some(&item_legacy_aliases),
        defaults.backpack.clone(),
        true,
        false,
    );
    signals.outfit = normalize_named_array(
        &signals.outfit,
        &valid_item_keys,
        &item_name_to_key,
        Some(&item_legacy_aliases),
        defaults.outfit.clone(),
    );
    signals.food = normalize_named_array(
        &signals.food,
        &valid_item_keys,
        &item_name_to_key,
        Some(&item_legacy_aliases),
        defaults.food.clone(),
    );
    signals.buff = normalize_named_array(
        &signals.buff,
        &valid_item_keys,
        &item_name_to_key,
        Some(&item_legacy_aliases),
        defaults.buff.clone(),
    );

    normalize_pet(&mut signals.pet1, defaults.pet1.clone(), &pet_value_aliases);
    normalize_pet(&mut signals.pet2, defaults.pet2.clone(), &pet_value_aliases);
    normalize_pet(&mut signals.pet3, defaults.pet3.clone(), &pet_value_aliases);
    normalize_pet(&mut signals.pet4, defaults.pet4.clone(), &pet_value_aliases);
    normalize_pet(&mut signals.pet5, defaults.pet5.clone(), &pet_value_aliases);

    if !matches!(
        signals.timespan_unit.as_str(),
        "minutes" | "hours" | "days" | "weeks"
    ) {
        signals.timespan_unit = defaults.timespan_unit;
    }
}

fn signals_patch_map(signals: &CalculatorSignals) -> AppResult<serde_json::Map<String, Value>> {
    let value = serde_json::to_value(signals).map_err(|err| {
        AppError::internal(format!("serialize normalized calculator signals: {err}"))
    })?;
    match value {
        Value::Object(obj) => Ok(obj),
        _ => Err(AppError::internal(
            "calculator signals serialization did not produce an object",
        )),
    }
}

fn init_signals_patch_map(
    signals: &CalculatorSignals,
) -> AppResult<serde_json::Map<String, Value>> {
    let mut patch = signals_patch_map(signals)?;
    mirror_resources_signal(&mut patch);
    Ok(patch)
}

fn mirror_resources_signal(patch: &mut serde_json::Map<String, Value>) {
    if let Some(value) = patch.get("resources").cloned() {
        patch.insert("_resources".to_string(), value);
    }
}

fn normalize_pet(
    pet: &mut CalculatorPetSignals,
    defaults: CalculatorPetSignals,
    aliases: &HashMap<String, String>,
) {
    let mut tier = pet.tier.trim().parse::<i32>().unwrap_or(4);
    tier = tier.clamp(1, 5);
    pet.tier = tier.to_string();
    pet.special = normalize_pet_value(&pet.special, aliases);
    pet.talent = normalize_pet_value(&pet.talent, aliases);
    pet.skills = pet
        .skills
        .iter()
        .map(|value| normalize_pet_value(value, aliases))
        .filter(|value| !value.is_empty())
        .collect();

    if pet.special != "auto_fishing_time_reduction" {
        pet.special.clear();
    }
    if !matches!(
        pet.talent.as_str(),
        "" | "durability_reduction_resistance" | "life_exp" | "fishing_exp"
    ) {
        pet.talent = defaults.talent;
    }
    pet.skills
        .retain(|value| matches!(value.as_str(), "fishing_exp"));
}

fn normalize_pet_value(value: &str, aliases: &HashMap<String, String>) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let normalized = normalize_lookup_value(trimmed);
    aliases
        .get(&normalized)
        .cloned()
        .unwrap_or_else(|| normalized.replace(' ', "_"))
}

fn normalize_named_value(
    value: &str,
    valid_keys: &std::collections::HashSet<String>,
    lookup: &HashMap<String, String>,
    aliases: Option<&HashMap<String, String>>,
    default_value: String,
    allow_empty: bool,
) -> String {
    normalize_named_value_with_fuzzy(
        value,
        valid_keys,
        lookup,
        aliases,
        default_value,
        allow_empty,
        false,
    )
}

fn normalize_named_value_with_fuzzy(
    value: &str,
    valid_keys: &std::collections::HashSet<String>,
    lookup: &HashMap<String, String>,
    aliases: Option<&HashMap<String, String>>,
    default_value: String,
    allow_empty: bool,
    allow_fuzzy_lookup: bool,
) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return if allow_empty {
            String::new()
        } else {
            default_value
        };
    }
    if valid_keys.contains(trimmed) {
        return trimmed.to_string();
    }
    let normalized = normalize_lookup_value(trimmed);
    if let Some(key) = lookup.get(&normalized) {
        return key.clone();
    }
    if let Some(aliases) = aliases {
        if let Some(key) = aliases.get(&normalized) {
            return key.clone();
        }
    }
    if allow_fuzzy_lookup {
        if let Some(key) = fuzzy_lookup_value(trimmed, lookup) {
            return key;
        }
    }
    if allow_empty {
        String::new()
    } else {
        default_value
    }
}

fn fuzzy_lookup_value(value: &str, lookup: &HashMap<String, String>) -> Option<String> {
    let matcher = SkimMatcherV2::default();
    let normalized_input = normalize_lookup_value(value);
    lookup
        .iter()
        .filter_map(|(candidate, resolved)| {
            matcher
                .fuzzy_match(candidate, &normalized_input)
                .map(|score| (score, resolved))
        })
        .max_by_key(|(score, _)| *score)
        .map(|(_, resolved)| resolved.clone())
}

fn normalize_named_array(
    values: &[String],
    valid_keys: &std::collections::HashSet<String>,
    lookup: &HashMap<String, String>,
    aliases: Option<&HashMap<String, String>>,
    default_values: Vec<String>,
) -> Vec<String> {
    if values.is_empty() {
        return Vec::new();
    }
    let normalized = values
        .iter()
        .map(|value| {
            if value.trim().is_empty() {
                String::new()
            } else {
                normalize_named_value(value, valid_keys, lookup, aliases, String::new(), true)
            }
        })
        .collect::<Vec<_>>();
    if normalized.is_empty() {
        default_values
    } else {
        normalized
    }
}

fn normalize_lookup_value(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn derive_signals(signals: &CalculatorSignals, data: &CalculatorData) -> CalculatorDerivedSignals {
    let zones_with_bite_times = data
        .zones
        .iter()
        .filter(|zone| zone.bite_time_min.is_some() && zone.bite_time_max.is_some())
        .collect::<Vec<_>>();
    let zone = zones_with_bite_times
        .iter()
        .find(|zone| zone.rgb_key.to_string() == signals.zone)
        .copied()
        .or_else(|| zones_with_bite_times.first().copied());
    let zone_name = zone
        .and_then(|zone| zone.name.clone())
        .unwrap_or_else(|| signals.zone.clone());
    let zone_bite_min_raw = zone
        .and_then(|zone| zone.bite_time_min)
        .map(|value| value as f64)
        .unwrap_or(0.0);
    let zone_bite_max_raw = zone
        .and_then(|zone| zone.bite_time_max)
        .map(|value| value as f64)
        .unwrap_or(0.0);
    let zone_bite_avg_raw = (zone_bite_min_raw + zone_bite_max_raw) / 2.0;

    let factor_level = 1.0
        - [0.15, 0.30, 0.35, 0.40, 0.45, 0.50]
            .get(signals.level as usize)
            .copied()
            .unwrap_or(0.0);
    let factor_resources = 2.0 - (signals.resources / 100.0);
    let bite_factor = factor_level * factor_resources;
    let effective_bite_min_raw = zone_bite_min_raw * bite_factor;
    let effective_bite_max_raw = zone_bite_max_raw * bite_factor;
    let bite_time_raw = zone_bite_avg_raw * bite_factor;

    let items_by_key = data
        .catalog
        .items
        .iter()
        .map(|item| (item.key.as_str(), item))
        .collect::<HashMap<_, _>>();
    let levels_by_key = data
        .catalog
        .lifeskill_levels
        .iter()
        .map(|level| (level.key.as_str(), level))
        .collect::<HashMap<_, _>>();

    let pet_stats = [
        (0.0, 0.0),
        (0.2, 0.01),
        (0.2, 0.02),
        (0.25, 0.03),
        (0.3, 0.04),
        (0.3, 0.05),
    ];

    let pets = [
        &signals.pet1,
        &signals.pet2,
        &signals.pet3,
        &signals.pet4,
        &signals.pet5,
    ];
    let pet_afr_max = pets
        .iter()
        .map(|pet| pet_afr(pet, &pet_stats))
        .fold(0.0_f64, f64::max);
    let pet_drr_sum = pets.iter().map(|pet| pet_drr(pet, &pet_stats)).sum::<f64>();
    let pet_fishing_exp = pets
        .iter()
        .map(|pet| pet_fishing_exp(pet, &pet_stats))
        .sum::<f64>();
    let pet_life_exp = pets
        .iter()
        .map(|pet| pet_life_exp(pet, &pet_stats))
        .sum::<f64>();

    let afr_uncapped_raw = pet_afr_max
        + sum_item_property(
            &items_by_key,
            &[
                &signals.rod,
                &signals.chair,
                &signals.lightstone_set,
                &signals.float,
            ],
            &[&signals.buff, &signals.food],
            |item| item.afr.map(f64::from),
        );
    let afr_raw = afr_uncapped_raw.min(2.0 / 3.0);
    // Keep the passive AFT baseline in backend-derived state even when the local
    // active-fishing toggle is enabled. The frontend switches between `0` and
    // this baseline locally, so server-backed control changes must not poison it.
    let auto_fish_time_raw = (180.0 * (1.0 - afr_raw)).max(60.0);

    let drr_raw = pet_drr_sum
        + sum_item_property(
            &items_by_key,
            &[
                &signals.rod,
                &signals.chair,
                &signals.backpack,
                &signals.lightstone_set,
            ],
            &[&signals.buff, &signals.outfit],
            |item| item.drr.map(f64::from),
        );

    let catch_time_active_raw = signals.catch_time_active.max(0.0);
    let catch_time_afk_raw = signals.catch_time_afk.max(0.0);
    let catch_time_raw = if signals.active {
        catch_time_active_raw
    } else {
        catch_time_afk_raw
    };
    let total_time_raw = if signals.active {
        bite_time_raw + catch_time_active_raw
    } else {
        bite_time_raw + auto_fish_time_raw + catch_time_afk_raw
    };
    let unoptimized_time_raw = zone_bite_avg_raw
        + if signals.active {
            catch_time_active_raw
        } else {
            catch_time_afk_raw + 180.0
        };

    let percent_bite = percentage_of_average_time(bite_time_raw, unoptimized_time_raw);
    let percent_catch = percentage_of_average_time(catch_time_raw, unoptimized_time_raw);
    let percent_af = percentage_of_average_time(auto_fish_time_raw, unoptimized_time_raw);
    let percent_improvement =
        100.0 - percentage_of_average_time(total_time_raw, unoptimized_time_raw);

    let lifeskill_index = levels_by_key
        .get(signals.lifeskill_level.as_str())
        .map(|level| level.index)
        .unwrap_or_default() as f64;
    let chance_to_reduce_raw = (if signals.brand { 0.5 } else { 1.0 })
        * (1.0 - drr_raw)
        * (0.9 - 0.005 * lifeskill_index).max(0.4);

    let timespan_seconds = timespan_seconds(signals.timespan_amount, &signals.timespan_unit);
    let timespan_text = timespan_text(signals.timespan_amount, &signals.timespan_unit);
    let casts_average_raw = if total_time_raw > 0.0 {
        timespan_seconds / total_time_raw
    } else {
        0.0
    };
    let durability_loss_average_raw = casts_average_raw * chance_to_reduce_raw;

    let debug_json = serde_json::to_string_pretty(&json!({
        "inputs": signals,
        "derived": {
            "zoneName": zone_name,
            "petFishingExp": pet_fishing_exp,
            "petLifeExp": pet_life_exp,
            "afrUncapped": afr_uncapped_raw,
            "afr": afr_raw,
            "drr": drr_raw,
            "biteTime": bite_time_raw,
            "totalTime": total_time_raw,
            "chanceToReduce": chance_to_reduce_raw,
            "castsAverage": casts_average_raw,
            "durabilityLossAverage": durability_loss_average_raw,
        }
    }))
    .unwrap_or_else(|_| "{}".to_string());

    CalculatorDerivedSignals {
        zone_name,
        abundance_label: calc_abundance_label(signals.resources),
        zone_bite_min: fmt2(zone_bite_min_raw),
        zone_bite_max: fmt2(zone_bite_max_raw),
        zone_bite_avg: fmt2(zone_bite_avg_raw),
        effective_bite_min: fmt2(effective_bite_min_raw),
        effective_bite_max: fmt2(effective_bite_max_raw),
        effective_bite_avg: fmt2(bite_time_raw),
        total_time: fmt2(total_time_raw),
        bite_time: fmt2(bite_time_raw),
        auto_fish_time: fmt2(auto_fish_time_raw),
        auto_fish_time_reduction_text: format!("{:.0}%", afr_uncapped_raw * 100.0),
        casts_title: format!("Average Casts ({timespan_text})"),
        casts_average: fmt2(casts_average_raw),
        durability_reduction_resistance_text: format!("{:.0}%", drr_raw * 100.0),
        chance_to_consume_durability_text: format!("{:.2}%", chance_to_reduce_raw * 100.0),
        durability_loss_title: format!("Average Durability Loss ({timespan_text})"),
        durability_loss_average: fmt2(durability_loss_average_raw),
        timespan_text: timespan_text.clone(),
        bite_time_title: format!(
            "Bitetime: {}s ({}%)",
            fmt2(bite_time_raw),
            fmt2(percent_bite)
        ),
        auto_fish_time_title: format!(
            "Auto-Fishing Time: {}s ({}%)",
            fmt2(auto_fish_time_raw),
            fmt2(percent_af)
        ),
        catch_time_title: format!(
            "Catch Time: {}s ({}%)",
            fmt2(catch_time_raw),
            fmt2(percent_catch)
        ),
        unoptimized_time_title: format!(
            "Average Unoptimized Time: {}s ({}%)",
            fmt2(unoptimized_time_raw),
            fmt2(percent_improvement)
        ),
        show_auto_fishing: !signals.active,
        percent_bite: fmt2(percent_bite),
        percent_af: fmt2(percent_af),
        percent_catch: fmt2(percent_catch),
        debug_json,
    }
}

fn pet_afr(pet: &CalculatorPetSignals, pet_stats: &[(f64, f64)]) -> f64 {
    let tier = pet.tier.parse::<usize>().unwrap_or(4).clamp(1, 5);
    if pet.special == "auto_fishing_time_reduction" {
        pet_stats[tier].0
    } else {
        0.0
    }
}

fn pet_drr(pet: &CalculatorPetSignals, pet_stats: &[(f64, f64)]) -> f64 {
    let tier = pet.tier.parse::<usize>().unwrap_or(4).clamp(1, 5);
    if pet.talent == "durability_reduction_resistance" {
        pet_stats[tier].1
    } else {
        0.0
    }
}

fn pet_fishing_exp(pet: &CalculatorPetSignals, pet_stats: &[(f64, f64)]) -> f64 {
    let tier = pet.tier.parse::<usize>().unwrap_or(4).clamp(1, 5);
    let base = if pet.talent == "fishing_exp" {
        pet_stats[tier].1
    } else {
        0.0
    };
    let skills = pet
        .skills
        .iter()
        .filter(|skill| skill.as_str() == "fishing_exp")
        .count() as f64
        * 0.05;
    base + skills
}

fn pet_life_exp(pet: &CalculatorPetSignals, pet_stats: &[(f64, f64)]) -> f64 {
    let tier = pet.tier.parse::<usize>().unwrap_or(4).clamp(1, 5);
    if pet.talent == "life_exp" {
        pet_stats[tier].1
    } else {
        0.0
    }
}

fn sum_item_property(
    items_by_key: &HashMap<&str, &CalculatorItemEntry>,
    singles: &[&String],
    groups: &[&Vec<String>],
    value: impl Fn(&CalculatorItemEntry) -> Option<f64>,
) -> f64 {
    let mut total = 0.0;
    for key in singles {
        total += items_by_key
            .get(key.as_str())
            .and_then(|item| value(item))
            .unwrap_or(0.0);
    }
    for group in groups {
        for key in group.iter().filter(|key| !key.trim().is_empty()) {
            total += items_by_key
                .get(key.as_str())
                .and_then(|item| value(item))
                .unwrap_or(0.0);
        }
    }
    total
}

fn percentage_of_average_time(time: f64, unoptimized_time: f64) -> f64 {
    if unoptimized_time <= 0.0 {
        0.0
    } else {
        (time / unoptimized_time) * 100.0
    }
}

fn calc_abundance_label(resources: f64) -> String {
    if resources <= 14.0 {
        "Exhausted".to_string()
    } else if resources <= 45.0 {
        "Low".to_string()
    } else if resources <= 70.0 {
        "Average".to_string()
    } else {
        "Abundant".to_string()
    }
}

fn timespan_seconds(amount: f64, unit: &str) -> f64 {
    let unit_seconds = match unit {
        "minutes" => 60.0,
        "hours" => 3600.0,
        "days" => 86400.0,
        _ => 604800.0,
    };
    amount.max(0.0) * unit_seconds
}

fn timespan_text(amount: f64, unit: &str) -> String {
    let normalized = amount.max(0.0);
    let label = match unit {
        "minutes" => {
            if normalized == 1.0 {
                "minute"
            } else {
                "minutes"
            }
        }
        "hours" => {
            if normalized == 1.0 {
                "hour"
            } else {
                "hours"
            }
        }
        "days" => {
            if normalized == 1.0 {
                "day"
            } else {
                "days"
            }
        }
        _ => {
            if normalized == 1.0 {
                "week"
            } else {
                "weeks"
            }
        }
    };
    format!("{} {label}", trim_float(normalized))
}

fn fmt2(value: f64) -> String {
    format!("{value:.2}")
}

fn trim_float(value: f64) -> String {
    let fixed = format!("{value:.2}");
    fixed
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn render_calculator_app(
    data: &CalculatorData,
    signals: &CalculatorSignals,
    derived: &CalculatorDerivedSignals,
) -> AppResult<String> {
    let fishing_levels = select_options_from_catalog(&data.catalog.fishing_levels);
    let lifeskill_levels = sorted_lifeskill_options(&data.catalog.lifeskill_levels);
    let session_units = select_options_from_catalog(&data.catalog.session_units);
    let rods = item_options_by_type(&data.catalog.items, "rod");
    let floats = item_options_by_type(&data.catalog.items, "float");
    let chairs = item_options_by_type(&data.catalog.items, "chair");
    let lightstone_sets = item_options_by_type(&data.catalog.items, "lightstone_set");
    let backpacks = item_options_by_type(&data.catalog.items, "backpack");
    let outfits = item_options_by_type(&data.catalog.items, "outfit");
    let foods = item_options_by_type(&data.catalog.items, "food");
    let buffs = item_options_by_type(&data.catalog.items, "buff");
    let active_checked = if signals.active { " checked" } else { "" };
    let debug_checked = if signals.debug { " checked" } else { "" };
    let zone_search_url = format!(
        "/api/v1/calculator/datastar/zone-search?lang={}",
        lang_param(data.lang)
    );
    let zone_results = render_zone_search_results(
        "calculator-zone-search-results",
        &data.zones,
        &signals.zone,
        "",
    );
    let zone_dropdown = render_searchable_dropdown(
        &SearchableDropdownConfig {
            root_id: "calculator-zone-picker",
            input_id: "calculator-zone-value",
            label: &derived.zone_name,
            value: &signals.zone,
            search_url: &zone_search_url,
            search_url_root: Some("api"),
            search_placeholder: "Search zones",
        },
        &zone_results,
    );
    let mut html = r####"
<div id="calculator-app" class="grid gap-6">
    <div class="hidden"
         data-computed:resources="$_resources"
         data-computed:_live="window.__fishystuffCalculator.liveCalc($level, $_resources, $active, $catchTimeActive, $catchTimeAfk, $timespanAmount, $timespanUnit, $_calc)"></div>
    <div class="hidden"
         data-on-signal-patch__debounce.150ms="window.__fishystuffCalculator.persist($)"
         data-on-signal-patch-filter="window.__fishystuffCalculator.persistSignalPatchFilter()"></div>
    <div class="hidden"
         data-on-signal-patch__debounce.150ms="@post(window.__fishystuffCalculator.evalUrl())"
         data-on-signal-patch-filter="window.__fishystuffCalculator.serverSignalPatchFilter()"></div>

    <section class="card card-border bg-base-100">
        <div class="card-body gap-5">
            <div class="flex flex-col gap-3 xl:flex-row xl:items-start xl:justify-between">
                <div class="flex flex-wrap gap-3">
                    <label class="label cursor-pointer justify-start gap-3 rounded-box border border-base-300 bg-base-200 px-4 py-3 font-medium">
                        <input type="checkbox" class="checkbox checkbox-primary" data-bind="active"__ACTIVE_CHECKED__>
                        <span>Active Fishing</span>
                    </label>
                    <label class="label cursor-pointer justify-start gap-3 rounded-box border border-base-300 bg-base-200 px-4 py-3 font-medium">
                        <input type="checkbox" class="checkbox checkbox-primary" data-bind="debug"__DEBUG_CHECKED__>
                        <span>Debug</span>
                    </label>
                </div>

                <div class="flex flex-wrap gap-2">
                    <button class="btn btn-soft btn-secondary"
                            data-on:click="window.__fishystuffToast.copyText(window.__fishystuffCalculator.presetUrl($), { success: 'Preset URL copied.' })">
                        <svg class="fishy-icon size-6" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="/img/icons.svg?v=20260325-2#fishy-link"></use></svg>
                        Copy URL
                    </button>
                    <button class="btn btn-soft btn-secondary"
                            data-on:click="window.__fishystuffToast.copyText(window.__fishystuffCalculator.shareText($), { success: 'Share text copied.' })">
                        <svg class="fishy-icon size-6" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="/img/icons.svg?v=20260325-2#fishy-share-nodes"></use></svg>
                        Copy Share
                    </button>
                    <button class="btn btn-dash btn-error"
                            data-on:click="window.__fishystuffCalculator.clear(); window.__fishystuffToast.info('Calculator cleared.')">
                        <svg class="fishy-icon size-6" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="/img/icons.svg?v=20260325-2#fishy-x-circle"></use></svg>
                        Clear
                    </button>
                </div>
            </div>

            <div class="rounded-box border border-base-300 bg-base-200 p-4">
                <div id="fishing-timeline">
                    <div data-attr:title="$_live.bite_time_title"
                         data-attr="{style: 'flex-basis:' + ($_live.percent_bite || '0.00') + '%;'}"
                         class="slider slider-bitetime"></div>
                    <div data-attr:title="$_live.auto_fish_time_title"
                         data-attr="{style: 'flex-basis:' + ($_live.percent_af || '0.00') + '%;'}"
                         class="slider slider-aftime"></div>
                    <div data-attr:title="$_live.catch_time_title"
                         data-attr="{style: 'flex-basis:' + ($_live.percent_catch || '0.00') + '%;'}"
                         class="slider slider-catchtime"></div>
                    <div data-attr:title="$_live.unoptimized_time_title" class="slider slider-empty"></div>
                </div>
            </div>

            <div class="grid gap-4">
                <div class="stats stats-vertical rounded-box border border-base-300 bg-base-100 xl:stats-horizontal">
                    <div class="stat">
                        <div class="stat-title">Average Total Fishing Time</div>
                        <div class="stat-value text-2xl" data-text="$_live.total_time"></div>
                        <div class="stat-desc">seconds</div>
                    </div>
                    <div class="stat">
                        <div class="stat-title">Average Bite Time</div>
                        <div class="stat-value text-2xl" data-text="$_live.bite_time"></div>
                        <div class="stat-desc">seconds</div>
                    </div>
                    <div class="stat">
                        <div class="stat-title">Auto-Fishing Time (AFT)</div>
                        <div class="stat-value text-2xl" data-text="$_live.auto_fish_time"></div>
                        <div class="stat-desc">seconds</div>
                    </div>
                    <div class="stat">
                        <div class="stat-title">Auto-Fishing Time Reduction (AFR)</div>
                        <div class="stat-value text-2xl" data-text="$_live.auto_fish_time_reduction_text"></div>
                    </div>
                </div>

                <div class="stats stats-vertical rounded-box border border-base-300 bg-base-100 xl:stats-horizontal">
                    <div class="stat">
                        <div class="stat-title whitespace-normal leading-snug" data-text="$_live.casts_title"></div>
                        <div class="stat-value text-2xl" data-text="$_live.casts_average"></div>
                    </div>
                    <div class="stat">
                        <div class="stat-title">Durability Reduction Resistance (DRR)</div>
                        <div class="stat-value text-2xl" data-text="$_live.durability_reduction_resistance_text"></div>
                    </div>
                    <div class="stat">
                        <div class="stat-title">Chance to consume Durability</div>
                        <div class="stat-value text-2xl" data-text="$_live.chance_to_consume_durability_text"></div>
                    </div>
                    <div class="stat">
                        <div class="stat-title whitespace-normal leading-snug" data-text="$_live.durability_loss_title"></div>
                        <div class="stat-value text-2xl" data-text="$_live.durability_loss_average"></div>
                    </div>
                </div>
            </div>

            <code data-show="$debug" class="rounded-box border border-base-300 bg-base-200 p-4 text-sm">
                <pre class="overflow-x-auto whitespace-pre-wrap break-all" data-text="$_calc.debug_json"></pre>
            </code>
        </div>
    </section>

    <div class="grid gap-6 lg:grid-cols-2">
        <fieldset class="card card-border bg-base-100">
            <legend class="fieldset-legend ml-6 px-2">Zone</legend>
            <div class="card-body gap-4 pt-0">
                <div class="grid gap-4">
                    <input id="calculator-zone-value" type="hidden" data-bind="zone" value="__ZONE_VALUE__">
                    __ZONE_SEARCH_DROPDOWN__
                    <div class="stats stats-horizontal rounded-box border border-base-300 bg-base-100 shadow-none">
                        <div class="stat px-4 py-3">
                            <div class="stat-title">Min</div>
                            <div class="stat-value text-lg" data-text="$_live.zone_bite_min"></div>
                            <div class="stat-desc">seconds</div>
                        </div>
                        <div class="stat px-4 py-3">
                            <div class="stat-title">Average</div>
                            <div class="stat-value text-lg" data-text="$_live.zone_bite_avg"></div>
                            <div class="stat-desc">seconds</div>
                        </div>
                        <div class="stat px-4 py-3">
                            <div class="stat-title">Max</div>
                            <div class="stat-value text-lg" data-text="$_live.zone_bite_max"></div>
                            <div class="stat-desc">seconds</div>
                        </div>
                    </div>
                </div>
            </div>
        </fieldset>

        <fieldset class="card card-border bg-base-100">
            <legend class="fieldset-legend ml-6 px-2">Bite Time</legend>
            <div class="card-body gap-4 pt-0">
                <div class="grid gap-4">
                    <fieldset class="fieldset">
                        <legend class="fieldset-legend">Fishing Level</legend>
                        __LEVEL_SELECT__
                    </fieldset>
                    <fieldset class="fieldset">
                        <legend class="fieldset-legend">Fishing Resources</legend>
                        <input data-bind="_resources" type="range" class="range-xs range-secondary w-full" min="0" max="100">
                        <span class="label text-sm font-medium" data-text="$_resources + '% (' + ($_live.abundance_label || 'Exhausted') + ')'"></span>
                    </fieldset>
                    <div class="stats stats-horizontal rounded-box border border-base-300 bg-base-100 shadow-none">
                        <div class="stat px-4 py-3">
                            <div class="stat-title">Effective Min</div>
                            <div class="stat-value text-lg" data-text="$_live.effective_bite_min"></div>
                            <div class="stat-desc">seconds</div>
                        </div>
                        <div class="stat px-4 py-3">
                            <div class="stat-title">Effective Average</div>
                            <div class="stat-value text-lg" data-text="$_live.effective_bite_avg"></div>
                            <div class="stat-desc">seconds</div>
                        </div>
                        <div class="stat px-4 py-3">
                            <div class="stat-title">Effective Max</div>
                            <div class="stat-value text-lg" data-text="$_live.effective_bite_max"></div>
                            <div class="stat-desc">seconds</div>
                        </div>
                    </div>
                </div>
            </div>
        </fieldset>

        <fieldset class="card card-border bg-base-100">
            <legend class="fieldset-legend ml-6 px-2">Catch Time</legend>
            <div class="card-body gap-4 pt-0">
                <div class="grid gap-3 sm:grid-cols-2">
                    <fieldset class="fieldset">
                        <legend class="fieldset-legend">Active</legend>
                        <input type="number" min="0" step="any" class="input input-sm w-full" data-bind="catchTimeActive">
                        <span class="label text-xs">seconds</span>
                    </fieldset>
                    <fieldset class="fieldset">
                        <legend class="fieldset-legend">AFK</legend>
                        <input type="number" min="0" step="any" class="input input-sm w-full" data-bind="catchTimeAfk">
                        <span class="label text-xs">seconds</span>
                    </fieldset>
                </div>
            </div>
        </fieldset>

        <fieldset class="card card-border bg-base-100">
            <legend class="fieldset-legend ml-6 px-2">Session (<span data-text="$_live.timespan_text || '8 hours'"></span>)</legend>
            <div class="card-body gap-3 pt-0">
                <div class="grid gap-3">
                    <div class="grid grid-cols-2 gap-3">
                        <fieldset class="fieldset">
                            <legend class="fieldset-legend">Amount</legend>
                            <input type="number" min="0" step="any" class="input input-sm w-full" id="timespan_amount" data-bind="timespanAmount" name="timespan_amount">
                        </fieldset>
                        <fieldset class="fieldset">
                            <legend class="fieldset-legend">Unit</legend>
                            __TIMESPAN_UNIT_SELECT__
                        </fieldset>
                    </div>

                    __SESSION_PRESETS__
                </div>
            </div>
        </fieldset>

        <fieldset class="card card-border bg-base-100 xl:col-span-2">
            <legend class="fieldset-legend ml-6 px-2">Gear</legend>
            <div class="card-body pt-0">
                <div id="items" class="grid gap-4 md:grid-cols-2">
                    <fieldset class="fieldset">
                        <legend class="fieldset-legend">Lifeskill Level</legend>
                        __LIFESKILL_LEVEL_SELECT__
                    </fieldset>
                    <fieldset class="fieldset">
                        <legend class="fieldset-legend">Fishing Rod</legend>
                        __ROD_SELECT__
                    </fieldset>
                    <fieldset class="fieldset">
                        <legend class="fieldset-legend">Brand</legend>
                        <label class="label cursor-pointer justify-start gap-3 rounded-box border border-base-300 bg-base-200 px-3 py-3 font-medium">
                            <input data-bind="brand" type="checkbox" class="checkbox checkbox-primary">
                        </label>
                    </fieldset>
                    <fieldset class="fieldset">
                        <legend class="fieldset-legend">Float</legend>
                        __FLOAT_SELECT__
                    </fieldset>
                    <fieldset class="fieldset">
                        <legend class="fieldset-legend">Chair</legend>
                        __CHAIR_SELECT__
                    </fieldset>
                    <fieldset class="fieldset">
                        <legend class="fieldset-legend">Lightstone Set</legend>
                        __LIGHTSTONE_SET_SELECT__
                    </fieldset>
                    <fieldset class="fieldset">
                        <legend class="fieldset-legend">Backpack</legend>
                        __BACKPACK_SELECT__
                    </fieldset>
                    <fieldset class="fieldset rounded-box border border-base-300 bg-base-200 p-4 md:col-span-2">
                        <legend class="fieldset-legend">Outfit</legend>
                        __OUTFITS__
                    </fieldset>
                    <fieldset class="fieldset rounded-box border border-base-300 bg-base-200 p-4 md:col-span-2">
                        <legend class="fieldset-legend">Food</legend>
                        __FOODS__
                    </fieldset>
                    <fieldset class="fieldset rounded-box border border-base-300 bg-base-200 p-4 md:col-span-2">
                        <legend class="fieldset-legend">Buffs</legend>
                        __BUFFS__
                    </fieldset>
                </div>
            </div>
        </fieldset>

        <fieldset class="card card-border bg-base-100 xl:col-span-2">
            <legend class="fieldset-legend ml-6 px-2">Pets</legend>
            <div class="card-body pt-0">
                __PETS__
            </div>
        </fieldset>
    </div>
</div>
"####
    .to_string();

    let replacements = [
        ("__ZONE_SEARCH_DROPDOWN__", zone_dropdown),
        ("__ZONE_VALUE__", escape_html(&signals.zone)),
        (
            "__LEVEL_SELECT__",
            render_select(
                "level",
                "select w-full",
                "data-bind=\"level\"",
                &signals.level.to_string(),
                &fishing_levels,
                false,
                None,
            ),
        ),
        (
            "__TIMESPAN_UNIT_SELECT__",
            render_select(
                "timespan_unit",
                "select select-sm w-full",
                "data-bind=\"timespanUnit\"",
                &signals.timespan_unit,
                &session_units,
                false,
                None,
            ),
        ),
        (
            "__SESSION_PRESETS__",
            render_session_presets(&data.catalog.session_presets, "session_presets"),
        ),
        (
            "__LIFESKILL_LEVEL_SELECT__",
            render_select(
                "lifeskill_level",
                "select w-full",
                "data-bind=\"lifeskill_level\"",
                &signals.lifeskill_level,
                &lifeskill_levels,
                false,
                None,
            ),
        ),
        (
            "__ROD_SELECT__",
            render_select(
                "rods",
                "select w-full",
                "data-bind=\"rod\"",
                &signals.rod,
                &rods,
                false,
                None,
            ),
        ),
        (
            "__FLOAT_SELECT__",
            render_select(
                "floats",
                "select w-full",
                "data-bind=\"float\"",
                &signals.float,
                &floats,
                true,
                None,
            ),
        ),
        (
            "__CHAIR_SELECT__",
            render_select(
                "chairs",
                "select w-full",
                "data-bind=\"chair\"",
                &signals.chair,
                &chairs,
                true,
                None,
            ),
        ),
        (
            "__LIGHTSTONE_SET_SELECT__",
            render_select(
                "lightstone_sets",
                "select w-full",
                "data-bind=\"lightstone_set\"",
                &signals.lightstone_set,
                &lightstone_sets,
                true,
                None,
            ),
        ),
        (
            "__BACKPACK_SELECT__",
            render_select(
                "backpacks",
                "select w-full",
                "data-bind=\"backpack\"",
                &signals.backpack,
                &backpacks,
                true,
                None,
            ),
        ),
        (
            "__OUTFITS__",
            render_checkbox_group("outfits", "outfit", &signals.outfit, &outfits, None),
        ),
        (
            "__FOODS__",
            render_checkbox_group("foods", "food", &signals.food, &foods, None),
        ),
        (
            "__BUFFS__",
            render_checkbox_group("buffs", "buff", &signals.buff, &buffs, None),
        ),
        ("__PETS__", render_pet_cards(&data.catalog.pets, signals)),
    ];

    for (token, replacement) in replacements {
        html = html.replace(token, &replacement);
    }
    html = html.replace("__ACTIVE_CHECKED__", active_checked);
    html = html.replace("__DEBUG_CHECKED__", debug_checked);
    Ok(html)
}

fn select_options_from_catalog(options: &[CalculatorOptionEntry]) -> Vec<SelectOption<'_>> {
    options
        .iter()
        .map(|option| SelectOption {
            value: option.key.as_str(),
            label: option.label.as_str(),
            icon: None,
        })
        .collect()
}

fn zone_name(zone: &ZoneEntry) -> &str {
    zone.name.as_deref().unwrap_or(zone.rgb_key.0.as_str())
}

fn searchable_zones(zones: &[ZoneEntry]) -> Vec<&ZoneEntry> {
    zones
        .iter()
        .filter(|zone| zone.bite_time_min.is_some() && zone.bite_time_max.is_some())
        .collect()
}

fn fuzzy_zone_matches<'a>(
    zones: &'a [ZoneEntry],
    query: &str,
    current_zone: &str,
) -> Vec<&'a ZoneEntry> {
    let mut zones = searchable_zones(zones);
    zones.sort_by(|left, right| zone_name(left).cmp(zone_name(right)));

    let trimmed = query.trim();
    if trimmed.is_empty() {
        zones.sort_by_key(|zone| {
            (
                if zone.rgb_key.0 == current_zone { 0 } else { 1 },
                zone_name(zone).to_string(),
            )
        });
        zones.truncate(SEARCHABLE_DROPDOWN_RESULT_LIMIT);
        return zones;
    }

    let matcher = SkimMatcherV2::default();
    let normalized_query = normalize_lookup_value(trimmed);
    let mut scored = zones
        .into_iter()
        .filter_map(|zone| {
            matcher
                .fuzzy_match(&normalize_lookup_value(zone_name(zone)), &normalized_query)
                .map(|score| (zone, score))
        })
        .collect::<Vec<_>>();
    scored.sort_by_key(|(zone, score)| (Reverse(*score), zone_name(zone).to_string()));
    scored.truncate(SEARCHABLE_DROPDOWN_RESULT_LIMIT);
    scored.into_iter().map(|(zone, _)| zone).collect()
}

fn render_zone_search_results(
    results_list_id: &str,
    zones: &[ZoneEntry],
    current_zone: &str,
    query: &str,
) -> String {
    let matches = fuzzy_zone_matches(zones, query, current_zone);
    let mut html = String::new();
    write!(
        html,
        "<ul id=\"{}\" tabindex=\"-1\" data-role=\"results\" class=\"menu menu-sm max-h-96 w-full gap-1 overflow-auto p-1\">",
        escape_html(results_list_id),
    )
    .unwrap();
    if matches.is_empty() {
        html.push_str("<li class=\"menu-disabled\"><span>No matching zones</span></li>");
    } else {
        for zone in matches {
            let label = zone_name(zone);
            let is_selected = zone.rgb_key.0 == current_zone;
            let active_class = if is_selected { " menu-active" } else { "" };
            write!(
                html,
                "<li><button type=\"button\" class=\"justify-between text-left{}\" data-searchable-dropdown-option data-value=\"{}\" data-label=\"{}\"><span>{}</span>{}</button></li>",
                active_class,
                escape_html(&zone.rgb_key.0),
                escape_html(label),
                escape_html(label),
                if is_selected {
                    "<span class=\"badge badge-soft badge-primary badge-xs\">Selected</span>"
                } else {
                    ""
                }
            )
            .unwrap();
        }
    }
    html.push_str("</ul>");
    html
}

fn render_searchable_dropdown(config: &SearchableDropdownConfig<'_>, results_html: &str) -> String {
    let panel_id = format!("{}-panel", config.root_id);
    let search_input_id = format!("{}-search-input", config.root_id);
    let search_url_root_attr = config
        .search_url_root
        .map(|value| format!(" search-url-root=\"{}\"", escape_html(value)))
        .unwrap_or_default();
    let mut html = String::new();
    write!(
        html,
        r#"<fishy-searchable-dropdown id="{root_id}"
     class="relative z-30 block w-full"
     input-id="{input_id}"
     label="{label}"
     value="{value}"
     search-url="{search_url}"{search_url_root_attr}
     placeholder="{search_placeholder}">
    <button type="button"
            data-role="trigger"
            class="flex min-h-11 w-full items-center justify-between gap-3 rounded-box border border-base-300 bg-base-100 px-4 py-3 text-left shadow-sm"
            aria-haspopup="listbox"
            aria-expanded="false"
            aria-controls="{panel_id}">
        <span data-role="selected-label" class="truncate font-medium">{label}</span>
        <svg class="fishy-icon size-4 opacity-60" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="/img/icons.svg?v=20260325-2#fishy-caret-down"></use></svg>
    </button>

    <div id="{panel_id}" data-role="panel" class="absolute left-0 top-0 z-40 w-full min-w-full max-w-full" hidden>
        <div class="grid w-full min-w-full overflow-hidden rounded-box border border-base-300 bg-base-100 shadow-lg">
            <label class="flex min-h-11 w-full min-w-full items-center gap-3 bg-base-100 px-4 py-3">
                <svg class="fishy-icon size-4 opacity-60" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="/img/icons.svg?v=20260325-2#fishy-search-field"></use></svg>
                <input id="{search_input_id}"
                       data-role="search-input"
                       type="search"
                       class="w-full border-0 bg-transparent p-0 shadow-none outline-none"
                       style="outline: none; box-shadow: none;"
                       placeholder="{search_placeholder}"
                       autocomplete="off"
                       spellcheck="false">
            </label>
            <div class="px-1 pb-1">
                {results_html}
            </div>
        </div>
    </div>
</fishy-searchable-dropdown>"#,
        root_id = escape_html(config.root_id),
        input_id = escape_html(config.input_id),
        label = escape_html(config.label),
        value = escape_html(config.value),
        search_url = escape_html(config.search_url),
        search_url_root_attr = search_url_root_attr,
        panel_id = escape_html(&panel_id),
        search_input_id = escape_html(&search_input_id),
        search_placeholder = escape_html(config.search_placeholder),
        results_html = results_html,
    )
    .unwrap();
    html
}

fn sorted_lifeskill_options(levels: &[CalculatorLifeskillLevelEntry]) -> Vec<SelectOption<'_>> {
    let mut levels = levels.iter().collect::<Vec<_>>();
    levels.sort_by_key(|level| level.order);
    levels
        .into_iter()
        .map(|level| SelectOption {
            value: level.key.as_str(),
            label: level.name.as_str(),
            icon: None,
        })
        .collect()
}

fn item_options_by_type<'a>(
    items: &'a [CalculatorItemEntry],
    item_type: &str,
) -> Vec<SelectOption<'a>> {
    items
        .iter()
        .filter(|item| item.r#type.contains(item_type))
        .map(|item| SelectOption {
            value: item.key.as_str(),
            label: item.name.as_str(),
            icon: item.icon.as_deref(),
        })
        .collect()
}

fn render_select(
    id: &str,
    class_name: &str,
    bind_attr: &str,
    selected_value: &str,
    options: &[SelectOption<'_>],
    include_none: bool,
    change_attr: Option<&str>,
) -> String {
    let mut html = String::new();
    let change_attr = change_attr.unwrap_or("");
    if id.is_empty() {
        write!(
            html,
            "<select class=\"{}\" {} {}><button class=\"w-full justify-between\"><div><selectedcontent></selectedcontent></div></button>",
            escape_html(class_name),
            bind_attr,
            change_attr,
        )
        .unwrap();
    } else {
        write!(
            html,
            "<select id=\"{}\" class=\"{}\" {} {}><button class=\"w-full justify-between\"><div><selectedcontent></selectedcontent></div></button>",
            escape_html(id),
            escape_html(class_name),
            bind_attr,
            change_attr,
        )
        .unwrap();
    }
    if include_none {
        html.push_str("<option value=\"\"><span>None</span></option>");
    }
    for option in options {
        let selected = if option.value == selected_value {
            " selected"
        } else {
            ""
        };
        write!(
            html,
            "<option value=\"{}\"{}>",
            escape_html(option.value),
            selected
        )
        .unwrap();
        if let Some(icon) = option.icon {
            write!(
                html,
                "<img aria-hidden=\"true\" src=\"{}\" class=\"item-icon\" alt=\"{} icon\"/>",
                escape_html(icon),
                escape_html(option.label)
            )
            .unwrap();
        }
        write!(html, "<span>{}</span></option>", escape_html(option.label)).unwrap();
    }
    html.push_str("</select>");
    html
}

fn render_checkbox_group(
    id: &str,
    bind_key: &str,
    selected_values: &[String],
    options: &[SelectOption<'_>],
    change_attr: Option<&str>,
) -> String {
    let selected = selected_values
        .iter()
        .map(|value| value.as_str())
        .collect::<std::collections::HashSet<_>>();
    let mut html = String::new();
    let change_attr = change_attr.unwrap_or("");
    write!(
        html,
        "<div id=\"{}\" class=\"grid gap-2 sm:grid-cols-2\" {}>",
        escape_html(id),
        change_attr,
    )
    .unwrap();
    for option in options {
        let checked = if selected.contains(option.value) {
            " checked"
        } else {
            ""
        };
        write!(
            html,
            "<label class=\"label cursor-pointer justify-start gap-3 rounded-box border border-base-300 bg-base-100 px-3 py-2 text-sm font-medium\"><input data-bind=\"{}\" type=\"checkbox\" class=\"checkbox checkbox-primary checkbox-sm shrink-0\" value=\"{}\"{}>",
            escape_html(bind_key),
            escape_html(option.value),
            checked
        )
        .unwrap();
        if let Some(icon) = option.icon {
            write!(
                html,
                "<img aria-hidden=\"true\" src=\"{}\" class=\"item-icon\" alt=\"{} icon\"/>",
                escape_html(icon),
                escape_html(option.label)
            )
            .unwrap();
        }
        write!(html, "<span>{}</span></label>", escape_html(option.label)).unwrap();
    }
    html.push_str("</div>");
    html
}

fn render_session_presets(presets: &[CalculatorSessionPresetEntry], id: &str) -> String {
    let mut html = String::new();
    write!(
        html,
        "<div id=\"{}\" class=\"join join-vertical sm:join-horizontal\">",
        escape_html(id)
    )
    .unwrap();
    for preset in presets {
        write!(
            html,
            "<button type=\"button\" class=\"btn btn-soft btn-sm join-item\" data-on:click=\"$timespanAmount = {}; $timespanUnit = '{}'; window.__fishystuffCalculator.persist($)\">{}</button>",
            trim_float(preset.amount),
            escape_html(&preset.unit),
            escape_html(&preset.label)
        )
        .unwrap();
    }
    html.push_str("</div>");
    html
}

fn render_pet_cards(catalog: &CalculatorPetCatalog, signals: &CalculatorSignals) -> String {
    let tier_options = select_options_from_catalog(&catalog.tiers);
    let special_options = select_options_from_catalog(&catalog.specials);
    let talent_options = select_options_from_catalog(&catalog.talents);

    let mut html = String::new();
    html.push_str("<div id=\"pets\" class=\"grid gap-4 md:grid-cols-2\">");
    for slot in 1..=catalog.slots.max(1) {
        let pet = match slot {
            1 => &signals.pet1,
            2 => &signals.pet2,
            3 => &signals.pet3,
            4 => &signals.pet4,
            _ => &signals.pet5,
        };
        let bind_prefix = format!("pet{slot}");
        let skill_bind = format!("{}.skills", bind_prefix);
        let skills_id = format!("pet{slot}_skills");
        write!(
            html,
            "<div class=\"pet rounded-box border border-base-300 bg-base-200 p-3\"><div class=\"grid gap-3\">"
        )
        .unwrap();
        html.push_str(
            "<fieldset class=\"fieldset\"><legend class=\"fieldset-legend\">Tier</legend>",
        );
        html.push_str(&render_select(
            "",
            "select select-sm w-full",
            &format!("data-bind=\"{}.tier\"", bind_prefix),
            &pet.tier,
            &tier_options,
            false,
            None,
        ));
        html.push_str("</fieldset>");
        html.push_str(
            "<fieldset class=\"fieldset\"><legend class=\"fieldset-legend\">Special</legend>",
        );
        html.push_str(&render_select(
            "",
            "select select-sm w-full",
            &format!("data-bind=\"{}.special\"", bind_prefix),
            &pet.special,
            &special_options,
            false,
            None,
        ));
        html.push_str("</fieldset>");
        html.push_str(
            "<fieldset class=\"fieldset\"><legend class=\"fieldset-legend\">Talent</legend>",
        );
        html.push_str(&render_select(
            "",
            "select select-sm w-full",
            &format!("data-bind=\"{}.talent\"", bind_prefix),
            &pet.talent,
            &talent_options,
            false,
            None,
        ));
        html.push_str("</fieldset></div>");
        html.push_str("<fieldset class=\"fieldset mt-3 gap-2\"><legend class=\"fieldset-legend\">Skills</legend>");
        html.push_str(&render_checkbox_group(
            &skills_id,
            &skill_bind,
            &pet.skills,
            &select_options_from_catalog(&catalog.skills),
            None,
        ));
        html.push_str("</fieldset></div>");
    }
    html.push_str("</div>");
    html
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use async_trait::async_trait;
    use axum::body::Bytes;
    use axum::extract::{Extension, Query, State};
    use axum::http::{header, StatusCode};
    use axum::response::IntoResponse;
    use fishystuff_api::ids::{Rgb, RgbKey};
    use fishystuff_api::models::calculator::{
        CalculatorCatalogResponse, CalculatorItemEntry, CalculatorLifeskillLevelEntry,
        CalculatorPetSignals, CalculatorSignals,
    };
    use fishystuff_api::models::effort::{EffortGridRequest, EffortGridResponse};
    use fishystuff_api::models::events::{EventsSnapshotMetaResponse, EventsSnapshotResponse};
    use fishystuff_api::models::fish::FishListResponse;
    use fishystuff_api::models::meta::{MetaDefaults, MetaResponse};
    use fishystuff_api::models::region_groups::RegionGroupsResponse;
    use fishystuff_api::models::zone_profile_v2::{ZoneProfileV2Request, ZoneProfileV2Response};
    use fishystuff_api::models::zone_stats::{ZoneStatsRequest, ZoneStatsResponse};
    use fishystuff_api::models::zones::ZoneEntry;
    use hyper::body::to_bytes;

    use crate::config::{AppConfig, ZoneStatusConfig};
    use crate::error::AppResult;
    use crate::state::{AppState, RequestId};
    use crate::store::{FishLang, Store};

    use super::{
        get_calculator_datastar_init, get_calculator_datastar_zone_search, normalize_lookup_value,
        normalize_named_array, post_calculator_datastar_eval, CalculatorDatastarQuery,
        CalculatorQuery, CalculatorZoneSearchQuery,
    };

    struct MockStore;

    #[async_trait]
    impl Store for MockStore {
        async fn get_meta(&self) -> AppResult<MetaResponse> {
            panic!("unused in test")
        }

        async fn get_region_groups(
            &self,
            _map_version_id: Option<String>,
        ) -> AppResult<RegionGroupsResponse> {
            panic!("unused in test")
        }

        async fn list_fish(
            &self,
            _lang: FishLang,
            _ref_id: Option<String>,
        ) -> AppResult<FishListResponse> {
            panic!("unused in test")
        }

        async fn calculator_catalog(
            &self,
            _lang: FishLang,
            _ref_id: Option<String>,
        ) -> AppResult<CalculatorCatalogResponse> {
            Ok(CalculatorCatalogResponse {
                items: vec![
                    CalculatorItemEntry {
                        key: "item:16162".to_string(),
                        name: "Balenos Fishing Rod".to_string(),
                        r#type: "rod".to_string(),
                        afr: Some(0.1),
                        ..CalculatorItemEntry::default()
                    },
                    CalculatorItemEntry {
                        key: "item:705539".to_string(),
                        name: "Manos Fishing Chair".to_string(),
                        r#type: "chair".to_string(),
                        afr: Some(0.1),
                        ..CalculatorItemEntry::default()
                    },
                    CalculatorItemEntry {
                        key: "effect:blacksmith-s-blessing".to_string(),
                        name: "Blacksmith's Blessing".to_string(),
                        r#type: "lightstone_set".to_string(),
                        afr: Some(0.1),
                        drr: Some(0.1),
                        ..CalculatorItemEntry::default()
                    },
                    CalculatorItemEntry {
                        key: "item:830150".to_string(),
                        name: "Lil' Otter Fishing Carrier".to_string(),
                        r#type: "backpack".to_string(),
                        drr: Some(0.05),
                        ..CalculatorItemEntry::default()
                    },
                    CalculatorItemEntry {
                        key: "item:9359".to_string(),
                        name: "Balacs Lunchbox".to_string(),
                        r#type: "food".to_string(),
                        afr: Some(0.05),
                        ..CalculatorItemEntry::default()
                    },
                    CalculatorItemEntry {
                        key: "item:721092".to_string(),
                        name: "Treant's Tear".to_string(),
                        r#type: "buff".to_string(),
                        exp_life: Some(0.3),
                        ..CalculatorItemEntry::default()
                    },
                ],
                lifeskill_levels: vec![CalculatorLifeskillLevelEntry {
                    key: "100".to_string(),
                    name: "Guru 20".to_string(),
                    index: 100,
                    order: 100,
                }],
                defaults: CalculatorSignals {
                    level: 5,
                    lifeskill_level: "100".to_string(),
                    zone: "240,74,74".to_string(),
                    resources: 0.0,
                    rod: "item:16162".to_string(),
                    float: String::new(),
                    chair: "item:705539".to_string(),
                    lightstone_set: "effect:blacksmith-s-blessing".to_string(),
                    backpack: "item:830150".to_string(),
                    outfit: vec![
                        "effect:8-piece-outfit-set-effect".to_string(),
                        "effect:awakening-weapon-outfit".to_string(),
                        "effect:mainhand-weapon-outfit".to_string(),
                    ],
                    food: vec!["item:9359".to_string()],
                    buff: vec!["".to_string(), "item:721092".to_string()],
                    pet1: CalculatorPetSignals {
                        tier: "5".to_string(),
                        special: "auto_fishing_time_reduction".to_string(),
                        talent: "durability_reduction_resistance".to_string(),
                        skills: vec!["fishing_exp".to_string()],
                    },
                    pet2: CalculatorPetSignals {
                        tier: "4".to_string(),
                        special: String::new(),
                        talent: "durability_reduction_resistance".to_string(),
                        skills: vec!["fishing_exp".to_string()],
                    },
                    pet3: CalculatorPetSignals {
                        tier: "4".to_string(),
                        special: String::new(),
                        talent: "durability_reduction_resistance".to_string(),
                        skills: vec!["fishing_exp".to_string()],
                    },
                    pet4: CalculatorPetSignals {
                        tier: "4".to_string(),
                        special: String::new(),
                        talent: "durability_reduction_resistance".to_string(),
                        skills: vec!["fishing_exp".to_string()],
                    },
                    pet5: CalculatorPetSignals {
                        tier: "4".to_string(),
                        special: String::new(),
                        talent: "durability_reduction_resistance".to_string(),
                        skills: vec!["fishing_exp".to_string()],
                    },
                    catch_time_active: 17.5,
                    catch_time_afk: 6.5,
                    timespan_amount: 8.0,
                    timespan_unit: "hours".to_string(),
                    brand: true,
                    active: false,
                    debug: false,
                },
                ..CalculatorCatalogResponse::default()
            })
        }

        async fn list_zones(&self, _ref_id: Option<String>) -> AppResult<Vec<ZoneEntry>> {
            Ok(vec![ZoneEntry {
                rgb_u32: 0,
                rgb: Rgb::new(240, 74, 74),
                rgb_key: RgbKey("240,74,74".to_string()),
                name: Some("Velia Beach".to_string()),
                active: Some(true),
                confirmed: Some(true),
                index: Some(1),
                bite_time_min: Some(120),
                bite_time_max: Some(180),
            }])
        }

        async fn zone_stats(
            &self,
            _request: ZoneStatsRequest,
            _status_cfg: ZoneStatusConfig,
        ) -> AppResult<ZoneStatsResponse> {
            panic!("unused in test")
        }

        async fn zone_profile_v2(
            &self,
            _request: ZoneProfileV2Request,
            _status_cfg: ZoneStatusConfig,
        ) -> AppResult<ZoneProfileV2Response> {
            panic!("unused in test")
        }

        async fn effort_grid(&self, _request: EffortGridRequest) -> AppResult<EffortGridResponse> {
            panic!("unused in test")
        }

        async fn events_snapshot_meta(&self) -> AppResult<EventsSnapshotMetaResponse> {
            panic!("unused in test")
        }

        async fn events_snapshot(
            &self,
            _requested_revision: Option<String>,
        ) -> AppResult<EventsSnapshotResponse> {
            panic!("unused in test")
        }

        async fn healthcheck(&self) -> AppResult<()> {
            panic!("unused in test")
        }
    }

    fn test_state() -> Arc<AppState> {
        let config = AppConfig {
            bind: "127.0.0.1:0".to_string(),
            database_url: "mysql://unused".to_string(),
            cors_allowed_origins: vec!["https://fishystuff.fish".to_string()],
            terrain_manifest_url: None,
            terrain_drape_manifest_url: None,
            terrain_height_tiles_url: None,
            defaults: MetaDefaults::default(),
            status_cfg: ZoneStatusConfig::default(),
            cache_zone_stats_max: 4,
            cache_effort_max: 4,
            cache_log: false,
            request_timeout_secs: 5,
        };
        AppState::for_tests(config, Arc::new(MockStore))
    }

    #[tokio::test]
    async fn init_returns_html_fragment_with_initial_signals() {
        let response = get_calculator_datastar_init(
            State(test_state()),
            Ok(Query(CalculatorDatastarQuery {
                lang: Some("en".to_string()),
                r#ref: None,
                datastar: Some("{}".to_string()),
            })),
            Extension(RequestId("req-test".to_string())),
        )
        .await
        .unwrap()
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("text/event-stream")
        );
        let body = to_bytes(response.into_body()).await.unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("event:datastar-patch-signals"));
        assert!(text.contains("data:signals {"));
        assert!(text.contains("\"catchTimeActive\":17.5"));
        assert!(text.contains("\"timespanAmount\":8.0"));
        assert!(text.contains("\"active\":false"));
        assert!(text.contains("\"_resources\":0.0"));
        assert!(text.contains("\"chair\":\"item:705539\""));
        assert!(text.contains("\"zone_name\":\"Velia Beach"));
        assert!(text.contains("event:datastar-patch-elements"));
        assert!(text.contains("data:selector #calculator-app"));
        assert!(text.contains("<div id=\"calculator-app\""));
        assert!(text.contains("placeholder=\"Search zones\""));
        assert!(text.contains("<fishy-searchable-dropdown"));
        assert!(text.contains("input-id=\"calculator-zone-value\""));
        assert!(text.contains("search-url=\"/api/v1/calculator/datastar/zone-search?lang=en\""));
        assert!(text.contains("search-url-root=\"api\""));
    }

    #[tokio::test]
    async fn eval_normalizes_legacy_values_and_returns_calc_signals_sse() {
        let response = post_calculator_datastar_eval(
            State(test_state()),
            Ok(Query(CalculatorQuery {
                lang: Some("en".to_string()),
                r#ref: None,
            })),
            Extension(RequestId("req-test".to_string())),
            Bytes::from_static(
                br#"{"zone":"Velia Beach","rod":"Balenos Fishing Rod","pet1":{"tier":"5","special":"Auto-Fishing Time Reduction","talent":"Durability Reduction Resistance","skills":["Fishing EXP"]}}"#,
            ),
        )
        .await
        .unwrap()
        .into_response();

        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("text/event-stream")
        );
        let body = to_bytes(response.into_body()).await.unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("event:datastar-patch-signals"));
        assert!(text.contains("\"zone_name\":\"Velia Beach\""));
        assert!(!text.contains("\"zone\":\"240,74,74\""));
        assert!(!text.contains("\"rod\":\"item:16162\""));
        assert!(!text.contains("\"_resources\":0.0"));
    }

    #[tokio::test]
    async fn eval_keeps_passive_auto_fish_time_when_active_is_true() {
        let response = post_calculator_datastar_eval(
            State(test_state()),
            Ok(Query(CalculatorQuery {
                lang: Some("en".to_string()),
                r#ref: None,
            })),
            Extension(RequestId("req-test".to_string())),
            Bytes::from_static(br#"{"active":true,"rod":"item:16162"}"#),
        )
        .await
        .unwrap()
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body()).await.unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("\"auto_fish_time\":\"63.00\""));
        assert!(!text.contains("\"active\":true"));
    }

    #[tokio::test]
    async fn init_fuzzy_matches_zone_name() {
        let response = get_calculator_datastar_init(
            State(test_state()),
            Ok(Query(CalculatorDatastarQuery {
                lang: Some("en".to_string()),
                r#ref: None,
                datastar: Some(r#"{"zone":"vlia bech"}"#.to_string()),
            })),
            Extension(RequestId("req-test".to_string())),
        )
        .await
        .unwrap()
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body()).await.unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("\"zone\":\"240,74,74\""));
        assert!(text.contains("\"zone_name\":\"Velia Beach\""));
    }

    #[tokio::test]
    async fn zone_search_returns_fuzzy_dropdown_results() {
        let response = get_calculator_datastar_zone_search(
            State(test_state()),
            Ok(Query(CalculatorZoneSearchQuery {
                lang: Some("en".to_string()),
                r#ref: None,
                q: Some("vlia bech".to_string()),
                selected: Some("240,74,74".to_string()),
            })),
            Extension(RequestId("req-test".to_string())),
        )
        .await
        .unwrap()
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("text/html; charset=utf-8")
        );
        let body = to_bytes(response.into_body()).await.unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("id=\"calculator-zone-search-results\""));
        assert!(text.contains("data-role=\"results\""));
        assert!(text.contains("data-searchable-dropdown-option"));
        assert!(text.contains("data-value=\"240,74,74\""));
        assert!(text.contains("Velia Beach"));
        assert!(text.contains("Selected"));
    }

    #[test]
    fn normalize_named_array_keeps_explicit_empty_selection() {
        let valid_keys = std::collections::HashSet::from(["item:1".to_string()]);
        let lookup = HashMap::from([(normalize_lookup_value("Item One"), "item:1".to_string())]);

        let normalized =
            normalize_named_array(&[], &valid_keys, &lookup, None, vec!["item:1".to_string()]);

        assert!(normalized.is_empty());
    }
}
