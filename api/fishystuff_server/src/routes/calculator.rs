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
    CalculatorMasteryPrizeRateEntry, CalculatorOptionEntry, CalculatorPetCatalog,
    CalculatorPetSignals, CalculatorPriceOverrideSignals, CalculatorSessionPresetEntry,
    CalculatorSignals, CalculatorZoneGroupRateEntry,
};
use fishystuff_api::models::zones::ZoneEntry;

use crate::error::{with_timeout, AppError, AppResult};
use crate::routes::meta::map_request_id;
use crate::state::{RequestId, SharedState};
use crate::store::{CalculatorZoneLootEntry, FishLang};

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

#[derive(Debug, Deserialize)]
pub struct CalculatorSearchableOptionQuery {
    pub lang: Option<String>,
    pub r#ref: Option<String>,
    pub kind: Option<String>,
    pub q: Option<String>,
    pub results_id: Option<String>,
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
    item_drr_text: String,
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
    fish_multiplier_raw: f64,
    loot_total_catches_raw: f64,
    loot_fish_per_hour_raw: f64,
    loot_profit_per_catch_raw: f64,
    loot_total_catches: String,
    loot_fish_per_hour: String,
    loot_fish_multiplier_text: String,
    loot_total_profit: String,
    loot_profit_per_hour: String,
    trade_bargain_bonus_text: String,
    trade_sale_multiplier_text: String,
    fish_group_distribution_chart: DistributionChartSignal,
    loot_distribution_chart: DistributionChartSignal,
    loot_sankey_chart: LootSankeySignal,
    debug_json: String,
}

#[derive(Debug, Clone)]
struct FishGroupChartRow {
    label: &'static str,
    fill_color: &'static str,
    stroke_color: &'static str,
    text_color: &'static str,
    connector_color: &'static str,
    bonus_text: String,
    base_share_pct: f64,
    weight_pct: f64,
    current_share_pct: f64,
}

#[derive(Debug, Clone)]
struct FishGroupChart {
    available: bool,
    note: String,
    raw_prize_rate_text: String,
    mastery_text: String,
    rows: Vec<FishGroupChartRow>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct LootChartRow {
    label: &'static str,
    fill_color: &'static str,
    stroke_color: &'static str,
    text_color: &'static str,
    connector_color: &'static str,
    expected_count_raw: f64,
    expected_count_text: String,
    expected_profit_text: String,
    current_share_pct: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
struct LootChart {
    available: bool,
    note: String,
    fish_multiplier_text: String,
    trade_bargain_bonus_text: String,
    trade_sale_multiplier_text: String,
    show_silver_amounts: bool,
    total_profit_raw: f64,
    total_profit_text: String,
    profit_per_hour_raw: f64,
    profit_per_hour_text: String,
    profit_per_catch_raw: f64,
    rows: Vec<LootChartRow>,
    species_rows: Vec<LootSpeciesRow>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct LootSpeciesRow {
    slot_idx: u8,
    group_label: &'static str,
    label: String,
    fill_color: &'static str,
    stroke_color: &'static str,
    text_color: &'static str,
    connector_color: &'static str,
    expected_count_raw: f64,
    expected_count_text: String,
    expected_profit_text: String,
    evidence_text: String,
}

#[derive(Debug, Clone, serde::Serialize)]
struct DistributionChartSegment {
    label: String,
    value_text: String,
    width_pct: f64,
    fill_color: &'static str,
    stroke_color: &'static str,
    text_color: &'static str,
    connector_color: &'static str,
}

#[derive(Debug, Clone, serde::Serialize)]
struct DistributionChartSignal {
    segments: Vec<DistributionChartSegment>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct LootSankeySignal {
    show_silver_amounts: bool,
    rows: Vec<LootChartRow>,
    species_rows: Vec<LootSpeciesRow>,
}

#[derive(Debug)]
struct CalculatorData {
    catalog: CalculatorCatalogResponse,
    cdn_base_url: String,
    lang: FishLang,
    zones: Vec<ZoneEntry>,
    zone_group_rates: HashMap<String, CalculatorZoneGroupRateEntry>,
    zone_loot_entries: Vec<CalculatorZoneLootEntry>,
}

#[derive(Debug, Clone, Copy)]
struct SelectOption<'a> {
    value: &'a str,
    label: &'a str,
    icon: Option<&'a str>,
    item: Option<&'a CalculatorItemEntry>,
    lifeskill_level: Option<&'a CalculatorLifeskillLevelEntry>,
}

struct SearchableDropdownConfig<'a> {
    catalog_html: Option<&'a str>,
    compact: bool,
    root_id: &'a str,
    input_id: &'a str,
    label: &'a str,
    selected_content_html: &'a str,
    value: &'a str,
    search_url: &'a str,
    search_url_root: Option<&'a str>,
    search_placeholder: &'a str,
}

struct SearchableMultiselectConfig<'a> {
    root_id: &'a str,
    bind_key: &'a str,
    search_placeholder: &'a str,
    helper_text: Option<&'a str>,
}

const SEARCHABLE_DROPDOWN_RESULT_LIMIT: usize = 24;

const NONE_SELECT_OPTION: SelectOption<'static> = SelectOption {
    value: "",
    label: "None",
    icon: None,
    item: None,
    lifeskill_level: None,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CalculatorSearchableOptionKind {
    FishingLevel,
    LifeskillLevel,
    TradeLevel,
    Rod,
    Float,
    Chair,
    LightstoneSet,
    Backpack,
    PetSpecial,
    PetTalent,
    PetTier,
    SessionUnit,
}

impl CalculatorSearchableOptionKind {
    fn from_param(value: Option<&str>) -> Option<Self> {
        match value?.trim() {
            "fishing_level" => Some(Self::FishingLevel),
            "lifeskill_level" => Some(Self::LifeskillLevel),
            "trade_level" => Some(Self::TradeLevel),
            "rod" => Some(Self::Rod),
            "float" => Some(Self::Float),
            "chair" => Some(Self::Chair),
            "lightstone_set" => Some(Self::LightstoneSet),
            "backpack" => Some(Self::Backpack),
            "pet_special" => Some(Self::PetSpecial),
            "pet_talent" => Some(Self::PetTalent),
            "pet_tier" => Some(Self::PetTier),
            "session_unit" => Some(Self::SessionUnit),
            _ => None,
        }
    }

    fn param(self) -> &'static str {
        match self {
            Self::FishingLevel => "fishing_level",
            Self::LifeskillLevel => "lifeskill_level",
            Self::TradeLevel => "trade_level",
            Self::Rod => "rod",
            Self::Float => "float",
            Self::Chair => "chair",
            Self::LightstoneSet => "lightstone_set",
            Self::Backpack => "backpack",
            Self::PetSpecial => "pet_special",
            Self::PetTalent => "pet_talent",
            Self::PetTier => "pet_tier",
            Self::SessionUnit => "session_unit",
        }
    }
}

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
    let (data, normalized_signals, derived) =
        load_calculator_runtime_data(&state, lang, query.r#ref.clone(), &request_id, raw_signals)
            .await?;
    calculator_datastar_init_response(&data, normalized_signals, derived)
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
    let (data, normalized_signals, derived) =
        load_calculator_runtime_data(&state, lang, query.r#ref.clone(), &request_id, raw_signals)
            .await?;
    calculator_datastar_init_response(&data, normalized_signals, derived)
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
    let (data, normalized_signals, derived) =
        load_calculator_runtime_data(&state, lang, query.r#ref.clone(), &request_id, raw_signals)
            .await?;
    let items_by_key = data
        .catalog
        .items
        .iter()
        .map(|item| (item.key.as_str(), item))
        .collect::<HashMap<_, _>>();
    let fish_group_chart = derive_fish_group_chart(&normalized_signals, &data, &items_by_key);
    let loot_chart = derive_loot_chart(
        &normalized_signals,
        &data,
        &fish_group_chart,
        derived.loot_total_catches_raw,
        derived.fish_multiplier_raw,
    );
    let events = vec![
        calculator_signals_event(
            &data,
            &normalized_signals,
            &derived,
            CalculatorPatchMode::Eval,
        )?
        .into_datastar_event(),
        PatchElements::new(render_fish_group_chart(&fish_group_chart))
            .selector("#calculator-fish-group-chart")
            .mode(ElementPatchMode::Outer)
            .into_datastar_event(),
        PatchElements::new(render_loot_chart(&loot_chart))
            .selector("#calculator-loot-chart")
            .mode(ElementPatchMode::Outer)
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

pub async fn get_calculator_datastar_option_search(
    State(state): State<SharedState>,
    query: Result<Query<CalculatorSearchableOptionQuery>, QueryRejection>,
    Extension(request_id): Extension<RequestId>,
) -> AppResult<impl IntoResponse> {
    let Query(query) = query.map_err(|err| {
        AppError::invalid_argument(err.to_string()).with_request_id(request_id.0.clone())
    })?;

    let kind =
        CalculatorSearchableOptionKind::from_param(query.kind.as_deref()).ok_or_else(|| {
            AppError::invalid_argument("missing or invalid calculator searchable option kind")
                .with_request_id(request_id.0.clone())
        })?;
    let lang = FishLang::from_param(query.lang.as_deref());
    let data = load_calculator_data(&state, lang, query.r#ref.clone(), &request_id).await?;
    let selected_value = query.selected.as_deref().unwrap_or_default();
    let search_text = query.q.unwrap_or_default();
    let results_id = query
        .results_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("calculator-search-results");
    let (options, include_none) = searchable_options_for_kind(&data, kind);
    let fragment = render_searchable_select_results(
        data.cdn_base_url.as_str(),
        results_id,
        &with_optional_none(&options, include_none),
        selected_value,
        &search_text,
    );
    let mut headers = HeaderMap::new();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok((headers, Html(fragment)))
}

fn calculator_datastar_init_response(
    data: &CalculatorData,
    normalized_signals: CalculatorSignals,
    derived: CalculatorDerivedSignals,
) -> AppResult<impl IntoResponse> {
    let app = render_calculator_app(data, &normalized_signals, &derived)?;
    let events = vec![
        calculator_signals_event(
            data,
            &normalized_signals,
            &derived,
            CalculatorPatchMode::Init,
        )?
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
    data: &CalculatorData,
    signals: &CalculatorSignals,
    derived: &CalculatorDerivedSignals,
    mode: CalculatorPatchMode,
) -> AppResult<PatchSignals> {
    let mut patch = match mode {
        CalculatorPatchMode::Init => init_signals_patch_map(data, signals)?,
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
    coerce_object_f64(&mut object, "mastery");
    coerce_object_f64(&mut object, "resources");
    coerce_object_f64(&mut object, "tradeDistanceBonus");
    coerce_object_f64(&mut object, "tradePriceCurve");
    coerce_object_price_override_map(&mut object, "priceOverrides");
    coerce_object_f64(&mut object, "catchTimeActive");
    coerce_object_f64(&mut object, "catchTimeAfk");
    coerce_object_f64(&mut object, "timespanAmount");
    coerce_object_bool(&mut object, "brand");
    coerce_object_bool(&mut object, "active");
    coerce_object_bool(&mut object, "debug");
    coerce_object_bool(&mut object, "applyTradeModifiers");
    coerce_object_bool(&mut object, "showSilverAmounts");
    coerce_object_string(&mut object, "discardGrade");
    coerce_object_string_array(&mut object, "outfit");
    coerce_object_string_array(&mut object, "food");
    coerce_object_string_array(&mut object, "buff");

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

fn coerce_object_string(object: &mut serde_json::Map<String, Value>, key: &str) {
    if let Some(value) = object.get_mut(key) {
        if let Some(string) = value.as_str() {
            *value = Value::String(normalize_discard_grade(string).to_string());
        } else if let Some(number) = value.as_i64() {
            *value = Value::String(number.to_string());
        }
    }
}

fn normalize_price_override_key(value: &str) -> Option<String> {
    let trimmed = value.trim();
    let numeric = trimmed.strip_prefix("item:").unwrap_or(trimmed);
    let parsed = numeric.parse::<i32>().ok()?;
    (parsed > 0).then(|| parsed.to_string())
}

fn coerce_object_price_override_map(object: &mut serde_json::Map<String, Value>, key: &str) {
    if let Some(Value::Object(map)) = object.get_mut(key) {
        let normalized = map
            .iter()
            .filter_map(|(raw_key, value)| {
                let key = normalize_price_override_key(raw_key)?;
                let Value::Object(entry) = value else {
                    return None;
                };
                let trade_price_curve_percent = entry
                    .get("tradePriceCurvePercent")
                    .and_then(|value| match value {
                        Value::Number(number) => number.as_f64(),
                        Value::String(string) => string.trim().parse::<f64>().ok(),
                        _ => None,
                    })
                    .map(|value| value.max(0.0));
                let base_price = entry
                    .get("basePrice")
                    .and_then(|value| match value {
                        Value::Number(number) => number.as_f64(),
                        Value::String(string) => string.trim().parse::<f64>().ok(),
                        _ => None,
                    })
                    .map(|value| value.max(0.0));
                if trade_price_curve_percent.is_none() && base_price.is_none() {
                    return None;
                }
                Some((
                    key,
                    json!({
                        "tradePriceCurvePercent": trade_price_curve_percent,
                        "basePrice": base_price,
                    }),
                ))
            })
            .collect::<serde_json::Map<_, _>>();
        *map = normalized;
    }
}

fn coerce_object_string_array(object: &mut serde_json::Map<String, Value>, key: &str) {
    if let Some(value) = object.get_mut(key) {
        coerce_value_string_array(value);
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
        coerce_value_string_array(value);
    }
}

fn coerce_value_string_array(value: &mut Value) {
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
        Value::Object(object) => {
            let mut keyed_values = object
                .iter()
                .filter_map(|(key, value)| {
                    key.parse::<usize>().ok().and_then(|index| match value {
                        Value::String(string) => Some((index, Value::String(string.clone()))),
                        Value::Number(number) => Some((index, Value::String(number.to_string()))),
                        _ => None,
                    })
                })
                .collect::<Vec<_>>();
            keyed_values.sort_by_key(|(index, _)| *index);
            *value = Value::Array(
                keyed_values
                    .into_iter()
                    .map(|(_, value)| value)
                    .collect::<Vec<_>>(),
            );
        }
        _ => {}
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
    let zone_group_rates = catalog
        .zone_group_rates
        .iter()
        .cloned()
        .map(|entry| (entry.zone_rgb_key.clone(), entry))
        .collect::<HashMap<_, _>>();
    Ok(CalculatorData {
        catalog,
        cdn_base_url: state.config.runtime_cdn_base_url.clone(),
        lang,
        zones,
        zone_group_rates,
        zone_loot_entries: Vec::new(),
    })
}

async fn load_calculator_runtime_data(
    state: &SharedState,
    lang: FishLang,
    ref_id: Option<String>,
    request_id: &RequestId,
    raw_signals: CalculatorSignals,
) -> AppResult<(CalculatorData, CalculatorSignals, CalculatorDerivedSignals)> {
    let mut data = load_calculator_data(state, lang, ref_id.clone(), request_id).await?;
    let mut signals = raw_signals;
    normalize_signals(&mut signals, &data);
    data.zone_loot_entries = with_timeout(
        state.config.request_timeout_secs,
        state
            .store
            .calculator_zone_loot(lang, ref_id, signals.zone.clone()),
    )
    .await
    .map_err(|err| map_request_id(err, request_id))?;
    let derived = derive_signals(&signals, &data);
    Ok((data, signals, derived))
}

fn lang_param(lang: FishLang) -> &'static str {
    match lang {
        FishLang::En => "en",
        FishLang::Ko => "ko",
    }
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
    let trade_level_name_to_key = data
        .catalog
        .trade_levels
        .iter()
        .map(|level| (normalize_lookup_value(&level.label), level.key.clone()))
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
    let pet_value_aliases = build_pet_value_aliases(&data.catalog.pets);
    let items_by_key = data
        .catalog
        .items
        .iter()
        .map(|item| (item.key.as_str(), item))
        .collect::<HashMap<_, _>>();

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
    let valid_trade_level_keys = data
        .catalog
        .trade_levels
        .iter()
        .map(|level| level.key.clone())
        .collect::<std::collections::HashSet<_>>();
    let valid_zone_keys = data
        .zones
        .iter()
        .map(|zone| zone.rgb_key.0.clone())
        .collect::<std::collections::HashSet<_>>();

    signals.level = signals.level.clamp(0, 5);
    signals.mastery = signals.mastery.max(0.0);
    signals.resources = signals.resources.clamp(0.0, 100.0);
    signals.trade_distance_bonus = signals.trade_distance_bonus.max(0.0);
    signals.trade_price_curve = signals.trade_price_curve.max(0.0);
    signals.catch_time_active = signals.catch_time_active.max(0.0);
    signals.catch_time_afk = signals.catch_time_afk.max(0.0);
    signals.timespan_amount = signals.timespan_amount.max(0.0);
    signals.trade_level = normalize_named_value_with_fuzzy(
        &signals.trade_level,
        &valid_trade_level_keys,
        &trade_level_name_to_key,
        None,
        defaults.trade_level.clone(),
        false,
        false,
    );

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
        None,
    );
    signals.food = normalize_named_array(
        &signals.food,
        &valid_item_keys,
        &item_name_to_key,
        Some(&item_legacy_aliases),
        defaults.food.clone(),
        Some(&items_by_key),
    );
    signals.buff = normalize_named_array(
        &signals.buff,
        &valid_item_keys,
        &item_name_to_key,
        Some(&item_legacy_aliases),
        defaults.buff.clone(),
        Some(&items_by_key),
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

fn build_pet_value_aliases(catalog: &CalculatorPetCatalog) -> HashMap<String, String> {
    let mut aliases = HashMap::new();
    for option in catalog
        .specials
        .iter()
        .chain(catalog.talents.iter())
        .chain(catalog.skills.iter())
    {
        if option.key.is_empty() {
            continue;
        }
        aliases.insert(normalize_lookup_value(&option.label), option.key.clone());
        aliases.insert(normalize_lookup_value(&option.key), option.key.clone());
        aliases.insert(
            normalize_lookup_value(&option.key.replace('_', " ")),
            option.key.clone(),
        );
    }
    aliases.insert(
        normalize_lookup_value("Auto-Fishing Time Reduction"),
        "auto_fishing_time_reduction".to_string(),
    );
    aliases.insert(
        normalize_lookup_value("Durability Reduction Resistance"),
        "durability_reduction_resistance".to_string(),
    );
    aliases.insert(normalize_lookup_value("Life EXP"), "life_exp".to_string());
    aliases.insert(
        normalize_lookup_value("Fishing EXP"),
        "fishing_exp".to_string(),
    );
    aliases
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
    data: &CalculatorData,
    signals: &CalculatorSignals,
) -> AppResult<serde_json::Map<String, Value>> {
    let mut patch = signals_patch_map(signals)?;
    mirror_resources_signal(&mut patch);
    expand_checkbox_group_signal_arrays(data, signals, &mut patch);
    Ok(patch)
}

fn mirror_resources_signal(patch: &mut serde_json::Map<String, Value>) {
    if let Some(value) = patch.get("resources").cloned() {
        patch.insert("_resources".to_string(), value);
    }
}

fn expand_checkbox_group_signal_arrays(
    data: &CalculatorData,
    signals: &CalculatorSignals,
    patch: &mut serde_json::Map<String, Value>,
) {
    patch.insert(
        "outfit".to_string(),
        Value::Array(indexed_checkbox_values(
            &signals.outfit,
            &item_options_by_type(&data.catalog.items, "outfit"),
        )),
    );

    for (slot, pet) in [
        ("pet1", &signals.pet1),
        ("pet2", &signals.pet2),
        ("pet3", &signals.pet3),
        ("pet4", &signals.pet4),
        ("pet5", &signals.pet5),
    ] {
        let Some(Value::Object(pet_patch)) = patch.get_mut(slot) else {
            continue;
        };
        pet_patch.insert(
            "skills".to_string(),
            Value::Array(indexed_checkbox_values(
                &pet.skills,
                &select_options_from_catalog(&data.catalog.pets.skills),
            )),
        );
    }
}

fn indexed_checkbox_values(selected_values: &[String], options: &[SelectOption<'_>]) -> Vec<Value> {
    let selected = selected_values
        .iter()
        .map(String::as_str)
        .collect::<std::collections::HashSet<_>>();
    options
        .iter()
        .map(|option| {
            Value::String(if selected.contains(option.value) {
                option.value.to_string()
            } else {
                String::new()
            })
        })
        .collect()
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
    items_by_key: Option<&HashMap<&str, &CalculatorItemEntry>>,
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
    let normalized = collapse_named_array_by_buff_category(normalized, items_by_key);
    if normalized.is_empty() {
        default_values
    } else {
        normalized
    }
}

fn collapse_named_array_by_buff_category(
    values: Vec<String>,
    items_by_key: Option<&HashMap<&str, &CalculatorItemEntry>>,
) -> Vec<String> {
    let Some(items_by_key) = items_by_key else {
        return values
            .into_iter()
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();
    };

    #[derive(Clone)]
    struct Candidate {
        value: String,
        position: usize,
        category_key: Option<String>,
        category_level: i32,
    }

    let latest_positions = values
        .iter()
        .enumerate()
        .filter_map(|(position, value)| (!value.is_empty()).then_some((value.clone(), position)))
        .collect::<HashMap<_, _>>();

    let mut winners_by_category = HashMap::<String, Candidate>::new();
    let mut winners = Vec::<Candidate>::new();
    for (position, value) in values.into_iter().enumerate() {
        if value.is_empty() || latest_positions.get(&value) != Some(&position) {
            continue;
        }
        let Some(item) = items_by_key.get(value.as_str()) else {
            winners.push(Candidate {
                value,
                position,
                category_key: None,
                category_level: 0,
            });
            continue;
        };
        let candidate = Candidate {
            value,
            position,
            category_key: item.buff_category_key.clone(),
            category_level: item.buff_category_level.unwrap_or(0),
        };
        if let Some(category_key) = candidate.category_key.clone() {
            match winners_by_category.get(&category_key) {
                Some(existing)
                    if existing.category_level > candidate.category_level
                        || (existing.category_level == candidate.category_level
                            && existing.position > candidate.position) => {}
                _ => {
                    winners_by_category.insert(category_key, candidate);
                }
            }
        } else {
            winners.push(candidate);
        }
    }

    winners.extend(winners_by_category.into_values());
    winners.sort_by_key(|candidate| candidate.position);
    winners
        .into_iter()
        .map(|candidate| candidate.value)
        .collect()
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

    let item_drr_raw = pet_drr_sum
        + sum_item_property(
            &items_by_key,
            &[
                &signals.rod,
                &signals.chair,
                &signals.backpack,
                &signals.lightstone_set,
            ],
            &[&signals.buff, &signals.outfit],
            |item| item.item_drr.map(f64::from),
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

    let lifeskill_level = levels_by_key.get(signals.lifeskill_level.as_str()).copied();
    let lifeskill_level_drr_raw = lifeskill_level
        .map(|level| f64::from(level.lifeskill_level_drr))
        .unwrap_or_default();
    let brandstone_durability_factor = if signals.brand { 0.5 } else { 1.0 };
    let chance_to_reduce_raw =
        brandstone_durability_factor * (1.0 - item_drr_raw) * (1.0 - lifeskill_level_drr_raw);
    let fish_group_chart = derive_fish_group_chart(signals, data, &items_by_key);
    let fish_multiplier_raw = effective_fish_multiplier(signals, &items_by_key);

    let timespan_seconds = timespan_seconds(signals.timespan_amount, &signals.timespan_unit);
    let timespan_text = timespan_text(signals.timespan_amount, &signals.timespan_unit);
    let casts_average_raw = if total_time_raw > 0.0 {
        timespan_seconds / total_time_raw
    } else {
        0.0
    };
    let loot_total_catches_raw = casts_average_raw * fish_multiplier_raw;
    let loot_fish_per_hour_raw = if total_time_raw > 0.0 {
        (3600.0 / total_time_raw) * fish_multiplier_raw
    } else {
        0.0
    };
    let durability_loss_average_raw = casts_average_raw * chance_to_reduce_raw;

    let loot_chart = derive_loot_chart(
        signals,
        data,
        &fish_group_chart,
        loot_total_catches_raw,
        fish_multiplier_raw,
    );

    let debug_json = serde_json::to_string_pretty(&json!({
        "inputs": signals,
        "derived": {
            "zoneName": zone_name,
            "petFishingExp": pet_fishing_exp,
            "petLifeExp": pet_life_exp,
            "afrUncapped": afr_uncapped_raw,
            "afr": afr_raw,
            "itemDrr": item_drr_raw,
            "lifeskillLevelDrr": lifeskill_level_drr_raw,
            "brandstoneDurabilityFactor": brandstone_durability_factor,
            "biteTime": bite_time_raw,
            "totalTime": total_time_raw,
            "chanceToConsumeDurability": chance_to_reduce_raw,
            "castsAverage": casts_average_raw,
            "durabilityLossAverage": durability_loss_average_raw,
            "fishMultiplier": fish_multiplier_raw,
            "loot": {
                "totalCatches": loot_total_catches_raw,
                "fishPerHour": loot_fish_per_hour_raw,
                "totalProfit": loot_chart.total_profit_raw,
                "profitPerHour": loot_chart.profit_per_hour_raw,
                "profitPerCatch": loot_chart.profit_per_catch_raw,
                "tradeBargainBonusText": loot_chart.trade_bargain_bonus_text,
                "tradeSaleMultiplierText": loot_chart.trade_sale_multiplier_text,
                "rows": fish_group_chart.rows.iter().map(|row| json!({
                    "label": row.label,
                    "expectedCount": loot_total_catches_raw * (row.current_share_pct / 100.0),
                    "currentSharePct": row.current_share_pct,
                })).collect::<Vec<_>>(),
            },
            "fishGroups": {
                "available": fish_group_chart.available,
                "rawPrizeCatchRateText": fish_group_chart.raw_prize_rate_text,
                "rows": fish_group_chart.rows.iter().map(|row| json!({
                    "label": row.label,
                    "bonusText": row.bonus_text,
                    "baseSharePct": row.base_share_pct,
                    "weightPct": row.weight_pct,
                    "currentSharePct": row.current_share_pct,
                })).collect::<Vec<_>>(),
            },
        }
    }))
    .unwrap_or_else(|_| "{}".to_string());
    let fish_group_distribution_chart = DistributionChartSignal {
        segments: fish_group_distribution_segments(&fish_group_chart.rows),
    };
    let loot_distribution_chart = DistributionChartSignal {
        segments: loot_distribution_segments(&loot_chart.rows),
    };
    let loot_sankey_chart = LootSankeySignal {
        show_silver_amounts: loot_chart.show_silver_amounts,
        rows: loot_chart.rows.clone(),
        species_rows: loot_chart.species_rows.clone(),
    };

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
        item_drr_text: format!("{:.0}%", item_drr_raw * 100.0),
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
        fish_multiplier_raw,
        loot_total_catches_raw,
        loot_fish_per_hour_raw,
        loot_profit_per_catch_raw: loot_chart.profit_per_catch_raw,
        loot_total_catches: fmt2(loot_total_catches_raw),
        loot_fish_per_hour: fmt2(loot_fish_per_hour_raw),
        loot_fish_multiplier_text: format!("×{}", trim_float(fish_multiplier_raw)),
        loot_total_profit: loot_chart.total_profit_text.clone(),
        loot_profit_per_hour: loot_chart.profit_per_hour_text.clone(),
        trade_bargain_bonus_text: loot_chart.trade_bargain_bonus_text.clone(),
        trade_sale_multiplier_text: loot_chart.trade_sale_multiplier_text.clone(),
        fish_group_distribution_chart,
        loot_distribution_chart,
        loot_sankey_chart,
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

fn effective_fish_multiplier(
    signals: &CalculatorSignals,
    items_by_key: &HashMap<&str, &CalculatorItemEntry>,
) -> f64 {
    let mut multiplier = 1.0_f64;
    for key in [
        signals.rod.as_str(),
        signals.float.as_str(),
        signals.chair.as_str(),
        signals.lightstone_set.as_str(),
        signals.backpack.as_str(),
    ] {
        if let Some(value) = items_by_key
            .get(key)
            .and_then(|item| item.fish_multiplier.map(f64::from))
            .filter(|value| *value > multiplier)
        {
            multiplier = value;
        }
    }
    for key in signals
        .outfit
        .iter()
        .chain(signals.food.iter())
        .chain(signals.buff.iter())
        .map(String::as_str)
    {
        if let Some(value) = items_by_key
            .get(key)
            .and_then(|item| item.fish_multiplier.map(f64::from))
            .filter(|value| *value > multiplier)
        {
            multiplier = value;
        }
    }
    multiplier
}

fn mastery_prize_rate_for_bracket(
    curve: &[CalculatorMasteryPrizeRateEntry],
    mastery: f64,
) -> f64 {
    let mastery = mastery.max(0.0);
    curve
        .iter()
        .rev()
        .find(|entry| mastery >= f64::from(entry.fishing_mastery))
        .or_else(|| curve.first())
        .map(|entry| f64::from(entry.high_drop_rate_raw) / 1_000_000.0)
        .unwrap_or_default()
}

fn derive_fish_group_chart(
    signals: &CalculatorSignals,
    data: &CalculatorData,
    items_by_key: &HashMap<&str, &CalculatorItemEntry>,
) -> FishGroupChart {
    let Some(zone_group_rate) = data.zone_group_rates.get(&signals.zone) else {
        return FishGroupChart {
            available: false,
            note: "Fish group data is unavailable for this zone.".to_string(),
            raw_prize_rate_text: "0.00%".to_string(),
            mastery_text: trim_float(signals.mastery),
            rows: Vec::new(),
        };
    };

    let mastery_prize_rate =
        mastery_prize_rate_for_bracket(&data.catalog.mastery_prize_curve, signals.mastery);
    let rare_bonus = sum_item_property(
        items_by_key,
        &[
            &signals.rod,
            &signals.float,
            &signals.chair,
            &signals.lightstone_set,
            &signals.backpack,
        ],
        &[&signals.outfit, &signals.food, &signals.buff],
        |item| item.bonus_rare.map(f64::from),
    );
    let high_quality_bonus = sum_item_property(
        items_by_key,
        &[
            &signals.rod,
            &signals.float,
            &signals.chair,
            &signals.lightstone_set,
            &signals.backpack,
        ],
        &[&signals.outfit, &signals.food, &signals.buff],
        |item| item.bonus_big.map(f64::from),
    );

    let rare_base = f64::from(zone_group_rate.rare_rate_raw.max(0)) / 1_000_000.0;
    let high_quality_base = f64::from(zone_group_rate.high_quality_rate_raw.max(0)) / 1_000_000.0;
    let general_base = f64::from(zone_group_rate.general_rate_raw.max(0)) / 1_000_000.0;
    let trash_base = f64::from(zone_group_rate.trash_rate_raw.max(0)) / 1_000_000.0;

    let rare_weight = rare_base * (1.0 + rare_bonus.max(0.0));
    let high_quality_weight = high_quality_base * (1.0 + high_quality_bonus.max(0.0));
    let general_weight = general_base;
    let trash_weight = trash_base;
    let prize_weight = mastery_prize_rate.max(0.0);
    let total_weight =
        prize_weight + rare_weight + high_quality_weight + general_weight + trash_weight;

    let current_share = |weight: f64| {
        if total_weight <= 0.0 {
            0.0
        } else {
            (weight / total_weight) * 100.0
        }
    };

    FishGroupChart {
        available: true,
        note: "Zone groups are renormalized to 100% after applying Rare and High-Quality bonuses plus prize weight from mastery.".to_string(),
        raw_prize_rate_text: format!("{}%", trim_float(prize_weight * 100.0)),
        mastery_text: trim_float(signals.mastery),
        rows: vec![
            FishGroupChartRow {
                label: "Prize",
                fill_color: "#fda4af",
                stroke_color: "#f87171",
                text_color: "#450a0a",
                connector_color: "rgb(248 113 113 / 0.48)",
                bonus_text: format!(
                    "Mastery {} → {}% raw prize",
                    trim_float(signals.mastery),
                    trim_float(prize_weight * 100.0)
                ),
                base_share_pct: 0.0,
                weight_pct: prize_weight * 100.0,
                current_share_pct: current_share(prize_weight),
            },
            FishGroupChartRow {
                label: "Rare",
                fill_color: "#fde68a",
                stroke_color: "#facc15",
                text_color: "#422006",
                connector_color: "rgb(250 204 21 / 0.48)",
                bonus_text: if rare_bonus > 0.0 {
                    format!("+{}% Rare", trim_float(rare_bonus * 100.0))
                } else {
                    "No bonus".to_string()
                },
                base_share_pct: rare_base * 100.0,
                weight_pct: rare_weight * 100.0,
                current_share_pct: current_share(rare_weight),
            },
            FishGroupChartRow {
                label: "High-Quality",
                fill_color: "#93c5fd",
                stroke_color: "#60a5fa",
                text_color: "#172554",
                connector_color: "rgb(96 165 250 / 0.48)",
                bonus_text: if high_quality_bonus > 0.0 {
                    format!("+{}% HQ", trim_float(high_quality_bonus * 100.0))
                } else {
                    "No bonus".to_string()
                },
                base_share_pct: high_quality_base * 100.0,
                weight_pct: high_quality_weight * 100.0,
                current_share_pct: current_share(high_quality_weight),
            },
            FishGroupChartRow {
                label: "General",
                fill_color: "#86efac",
                stroke_color: "#4ade80",
                text_color: "#052e16",
                connector_color: "rgb(74 222 128 / 0.48)",
                bonus_text: "No bonus".to_string(),
                base_share_pct: general_base * 100.0,
                weight_pct: general_weight * 100.0,
                current_share_pct: current_share(general_weight),
            },
            FishGroupChartRow {
                label: "Trash",
                fill_color: "var(--color-base-100)",
                stroke_color: "color-mix(in srgb, var(--color-base-content) 16%, transparent)",
                text_color: "var(--color-base-content)",
                connector_color: "color-mix(in srgb, var(--color-base-content) 24%, transparent)",
                bonus_text: "No bonus".to_string(),
                base_share_pct: trash_base * 100.0,
                weight_pct: trash_weight * 100.0,
                current_share_pct: current_share(trash_weight),
            },
        ],
    }
}

fn trade_bargain_bonus_from_level_key(level_key: &str) -> f64 {
    let index = level_key.trim().parse::<i32>().unwrap_or_default().max(0);
    0.05 + 0.005 * f64::from(index)
}

fn trade_sale_multiplier(signals: &CalculatorSignals) -> f64 {
    trade_sale_multiplier_for_curve_percent(signals, signals.trade_price_curve)
}

fn price_override_for_species(
    signals: &CalculatorSignals,
    item_id: i32,
) -> Option<&CalculatorPriceOverrideSignals> {
    signals.price_overrides.get(&item_id.to_string())
}

fn trade_price_curve_percent_for_species(signals: &CalculatorSignals, item_id: i32) -> f64 {
    price_override_for_species(signals, item_id)
        .and_then(|override_values| override_values.trade_price_curve_percent)
        .unwrap_or(signals.trade_price_curve)
}

fn base_price_for_species(
    signals: &CalculatorSignals,
    item_id: i32,
    source_base_price: f64,
) -> f64 {
    price_override_for_species(signals, item_id)
        .and_then(|override_values| override_values.base_price)
        .unwrap_or(source_base_price)
}

fn trade_sale_multiplier_for_curve_percent(
    signals: &CalculatorSignals,
    trade_price_curve_percent: f64,
) -> f64 {
    if !signals.apply_trade_modifiers {
        return 1.0;
    }
    let distance_bonus = (signals.trade_distance_bonus.max(0.0) / 100.0).min(1.5);
    let trade_price_curve = trade_price_curve_percent.max(0.0) / 100.0;
    let bargain_bonus = trade_bargain_bonus_from_level_key(&signals.trade_level);
    (1.0 + distance_bonus) * trade_price_curve * (1.0 + bargain_bonus)
}

fn trade_sale_multiplier_for_species(signals: &CalculatorSignals, item_id: i32) -> f64 {
    trade_sale_multiplier_for_curve_percent(
        signals,
        trade_price_curve_percent_for_species(signals, item_id),
    )
}

fn normalize_discard_grade(value: &str) -> &str {
    match value.trim().to_ascii_lowercase().as_str() {
        "white" => "white",
        "green" => "green",
        "blue" => "blue",
        "yellow" => "yellow",
        _ => "none",
    }
}

fn discard_grade_threshold(value: &str) -> Option<u8> {
    match normalize_discard_grade(value) {
        "white" => Some(0),
        "green" => Some(1),
        "blue" => Some(2),
        "yellow" => Some(3),
        _ => None,
    }
}

fn fish_grade_rank(grade: &str) -> Option<u8> {
    match grade {
        "Trash" => Some(0),
        "General" => Some(1),
        "HighQuality" => Some(2),
        "Rare" => Some(3),
        "Prize" => Some(4),
        _ => None,
    }
}

fn discard_grade_enabled(signals: &CalculatorSignals, grade: Option<&str>) -> bool {
    let Some(threshold) = discard_grade_threshold(&signals.discard_grade) else {
        return false;
    };
    let Some(rank) = grade.and_then(fish_grade_rank) else {
        return false;
    };
    rank <= threshold && rank < 4
}

fn fmt_silver(value: f64) -> String {
    let rounded = value.max(0.0).round() as i64;
    let negative = rounded < 0;
    let digits = rounded.abs().to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, ch) in digits.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    let mut grouped = grouped.chars().rev().collect::<String>();
    if negative {
        grouped.insert(0, '-');
    }
    grouped
}

fn loot_species_evidence_text(entry: &CalculatorZoneLootEntry) -> String {
    let db_rate_text = entry
        .evidence
        .iter()
        .find(|evidence| evidence.source_family == "database" && evidence.claim_kind == "in_group_rate")
        .and_then(|evidence| evidence.rate)
        .map(|rate| format!("DB {}%", trim_float(rate * 100.0)))
        .unwrap_or_else(|| format!("DB {}%", trim_float(entry.within_group_rate * 100.0)));

    let Some(community) = entry
        .evidence
        .iter()
        .find(|evidence| evidence.source_family == "community" && evidence.claim_kind == "presence")
    else {
        return db_rate_text;
    };

    let status = match community.status.as_deref().unwrap_or_default() {
        "confirmed" => "Community confirmed",
        "data_incomplete" => "Community incomplete",
        _ => "Community unconfirmed",
    };
    let claims = community
        .claim_count
        .map(|count| format!("×{count}"))
        .unwrap_or_default();
    let scope = match community.scope.as_str() {
        "group_inferred" => "group-inferred",
        "group" => "group",
        _ => "zone-only",
    };
    format!("{db_rate_text} · {status}{claims} · {scope}")
}

fn derive_loot_chart(
    signals: &CalculatorSignals,
    data: &CalculatorData,
    fish_group_chart: &FishGroupChart,
    total_catches_raw: f64,
    fish_multiplier_raw: f64,
) -> LootChart {
    if !fish_group_chart.available {
        return LootChart {
            available: false,
            note: "Expected loot data is unavailable for this zone.".to_string(),
            fish_multiplier_text: "×1".to_string(),
            trade_bargain_bonus_text: "0.00%".to_string(),
            trade_sale_multiplier_text: "×1".to_string(),
            show_silver_amounts: signals.show_silver_amounts,
            total_profit_raw: 0.0,
            total_profit_text: "0".to_string(),
            profit_per_hour_raw: 0.0,
            profit_per_hour_text: "0".to_string(),
            profit_per_catch_raw: 0.0,
            rows: Vec::new(),
            species_rows: Vec::new(),
        };
    }

    let bargain_bonus_raw = trade_bargain_bonus_from_level_key(&signals.trade_level);
    let sale_multiplier_raw = trade_sale_multiplier(signals);
    let timespan_seconds = timespan_seconds(signals.timespan_amount, &signals.timespan_unit);
    let fish_per_hour_raw = if timespan_seconds > 0.0 {
        (total_catches_raw / timespan_seconds) * 3600.0
    } else {
        0.0
    };

    let group_share_by_slot = fish_group_chart
        .rows
        .iter()
        .enumerate()
        .map(|(index, row)| ((index + 1) as u8, row.current_share_pct / 100.0))
        .collect::<HashMap<_, _>>();
    let group_row_by_slot = fish_group_chart
        .rows
        .iter()
        .enumerate()
        .map(|(index, row)| ((index + 1) as u8, row))
        .collect::<HashMap<_, _>>();

    let mut group_profit_by_slot = HashMap::<u8, f64>::new();
    let mut species_rows = Vec::new();
    for entry in &data.zone_loot_entries {
        let Some(group_row) = group_row_by_slot.get(&entry.slot_idx) else {
            continue;
        };
        let group_share = group_share_by_slot
            .get(&entry.slot_idx)
            .copied()
            .unwrap_or_default();
        let expected_count_raw = total_catches_raw * group_share * entry.within_group_rate;
        let source_vendor_price_raw = entry.vendor_price.unwrap_or_default() as f64;
        let base_price_raw =
            base_price_for_species(signals, entry.item_id, source_vendor_price_raw);
        let sale_multiplier_raw = trade_sale_multiplier_for_species(signals, entry.item_id);
        let discarded = entry.is_fish && discard_grade_enabled(signals, entry.grade.as_deref());
        let expected_profit_raw = if discarded {
            0.0
        } else {
            expected_count_raw * base_price_raw * sale_multiplier_raw
        };
        *group_profit_by_slot.entry(entry.slot_idx).or_default() += expected_profit_raw;
        species_rows.push(LootSpeciesRow {
            slot_idx: entry.slot_idx,
            group_label: group_row.label,
            label: entry.name.clone(),
            fill_color: group_row.fill_color,
            stroke_color: group_row.stroke_color,
            text_color: group_row.text_color,
            connector_color: group_row.connector_color,
            expected_count_raw,
            expected_count_text: trim_float(expected_count_raw),
            expected_profit_text: fmt_silver(expected_profit_raw),
            evidence_text: loot_species_evidence_text(entry),
        });
    }
    species_rows.sort_by(|left, right| {
        left.slot_idx
            .cmp(&right.slot_idx)
            .then_with(|| {
                right
                    .expected_count_raw
                    .partial_cmp(&left.expected_count_raw)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| left.label.to_lowercase().cmp(&right.label.to_lowercase()))
    });

    let rows = fish_group_chart
        .rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let slot_idx = (index + 1) as u8;
            let expected_count_raw = total_catches_raw * (row.current_share_pct / 100.0);
            LootChartRow {
                label: row.label,
                fill_color: row.fill_color,
                stroke_color: row.stroke_color,
                text_color: row.text_color,
                connector_color: row.connector_color,
                expected_count_raw,
                expected_count_text: trim_float(expected_count_raw),
                expected_profit_text: fmt_silver(
                    group_profit_by_slot
                        .get(&slot_idx)
                        .copied()
                        .unwrap_or_default(),
                ),
                current_share_pct: row.current_share_pct,
            }
        })
        .collect::<Vec<_>>();
    let total_profit_raw = group_profit_by_slot.values().sum::<f64>();
    let profit_per_catch_raw = if total_catches_raw > 0.0 {
        total_profit_raw / total_catches_raw
    } else {
        0.0
    };
    let profit_per_hour_raw = fish_per_hour_raw * profit_per_catch_raw;

    LootChart {
        available: true,
        note: "Expected loot uses average session casts, the current Fish multiplier, normalized group shares, and actual source-backed item prices. Species rows show DB in-group rates separately from community presence evidence. Fish auto-discard applies only to fish, not non-fish loot.".to_string(),
        fish_multiplier_text: format!("×{}", trim_float(fish_multiplier_raw)),
        trade_bargain_bonus_text: format!("+{}%", trim_float(bargain_bonus_raw * 100.0)),
        trade_sale_multiplier_text: if signals.apply_trade_modifiers {
            format!("×{}", trim_float(sale_multiplier_raw))
        } else {
            "Off (×1)".to_string()
        },
        show_silver_amounts: signals.show_silver_amounts,
        total_profit_raw,
        total_profit_text: fmt_silver(total_profit_raw),
        profit_per_hour_raw,
        profit_per_hour_text: fmt_silver(profit_per_hour_raw),
        profit_per_catch_raw,
        rows,
        species_rows,
    }
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

fn absolute_public_asset_url(cdn_base_url: &str, raw_path: &str) -> String {
    let normalized_base = cdn_base_url.trim().trim_end_matches('/');
    let normalized_path = raw_path.trim();
    if normalized_path.starts_with("http://")
        || normalized_path.starts_with("https://")
        || normalized_path.starts_with("data:")
    {
        return normalized_path.to_string();
    }
    if normalized_base.is_empty() {
        return normalized_path.to_string();
    }
    if normalized_path.starts_with('/') {
        format!("{normalized_base}{normalized_path}")
    } else {
        format!("{normalized_base}/{normalized_path}")
    }
}

fn render_calculator_app(
    data: &CalculatorData,
    signals: &CalculatorSignals,
    derived: &CalculatorDerivedSignals,
) -> AppResult<String> {
    let items_by_key = data
        .catalog
        .items
        .iter()
        .map(|item| (item.key.as_str(), item))
        .collect::<HashMap<_, _>>();
    let fish_group_chart = derive_fish_group_chart(signals, data, &items_by_key);
    let loot_chart = derive_loot_chart(
        signals,
        data,
        &fish_group_chart,
        derived.loot_total_catches_raw,
        derived.fish_multiplier_raw,
    );
    let fishing_levels = select_options_from_catalog(&data.catalog.fishing_levels);
    let lifeskill_levels = sorted_lifeskill_options(&data.catalog.lifeskill_levels);
    let trade_levels = select_options_from_catalog(&data.catalog.trade_levels);
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
    let zone_selected_content = render_searchable_dropdown_text_content(&derived.zone_name);
    let zone_results = render_zone_search_results(
        "calculator-zone-search-results",
        &data.zones,
        &signals.zone,
        "",
    );
    let zone_dropdown = render_searchable_dropdown(
        &SearchableDropdownConfig {
            catalog_html: None,
            compact: false,
            root_id: "calculator-zone-picker",
            input_id: "calculator-zone-value",
            label: &derived.zone_name,
            selected_content_html: &zone_selected_content,
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
                        <div class="stat-title">Item DRR</div>
                        <div class="stat-value text-2xl" data-text="$_live.item_drr_text"></div>
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

        __FISH_GROUP_WINDOW__
        __LOOT_WINDOW__

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
            render_searchable_select_control(
                data.cdn_base_url.as_str(),
                data.lang,
                "calculator-level-picker",
                "calculator-level-value",
                "level",
                CalculatorSearchableOptionKind::FishingLevel,
                &signals.level.to_string(),
                &fishing_levels,
                false,
                "Search fishing levels",
                false,
            ),
        ),
        (
            "__TIMESPAN_UNIT_SELECT__",
            render_searchable_select_control(
                data.cdn_base_url.as_str(),
                data.lang,
                "calculator-session-unit-picker",
                "calculator-session-unit-value",
                "timespanUnit",
                CalculatorSearchableOptionKind::SessionUnit,
                &signals.timespan_unit,
                &session_units,
                false,
                "Search session units",
                true,
            ),
        ),
        (
            "__SESSION_PRESETS__",
            render_session_presets(&data.catalog.session_presets, "session_presets"),
        ),
        (
            "__LIFESKILL_LEVEL_SELECT__",
            render_searchable_select_control(
                data.cdn_base_url.as_str(),
                data.lang,
                "calculator-lifeskill-level-picker",
                "calculator-lifeskill-level-value",
                "lifeskill_level",
                CalculatorSearchableOptionKind::LifeskillLevel,
                &signals.lifeskill_level,
                &lifeskill_levels,
                false,
                "Search lifeskill levels",
                false,
            ),
        ),
        (
            "__FISH_GROUP_WINDOW__",
            render_fish_group_window(&fish_group_chart, signals.mastery),
        ),
        (
            "__LOOT_WINDOW__",
            render_loot_window(data, signals, &trade_levels, &loot_chart),
        ),
        (
            "__ROD_SELECT__",
            render_searchable_select_control(
                data.cdn_base_url.as_str(),
                data.lang,
                "calculator-rod-picker",
                "calculator-rod-value",
                "rod",
                CalculatorSearchableOptionKind::Rod,
                &signals.rod,
                &rods,
                false,
                "Search rods",
                false,
            ),
        ),
        (
            "__FLOAT_SELECT__",
            render_searchable_select_control(
                data.cdn_base_url.as_str(),
                data.lang,
                "calculator-float-picker",
                "calculator-float-value",
                "float",
                CalculatorSearchableOptionKind::Float,
                &signals.float,
                &floats,
                true,
                "Search floats",
                false,
            ),
        ),
        (
            "__CHAIR_SELECT__",
            render_searchable_select_control(
                data.cdn_base_url.as_str(),
                data.lang,
                "calculator-chair-picker",
                "calculator-chair-value",
                "chair",
                CalculatorSearchableOptionKind::Chair,
                &signals.chair,
                &chairs,
                true,
                "Search chairs",
                false,
            ),
        ),
        (
            "__LIGHTSTONE_SET_SELECT__",
            render_searchable_select_control(
                data.cdn_base_url.as_str(),
                data.lang,
                "calculator-lightstone-set-picker",
                "calculator-lightstone-set-value",
                "lightstone_set",
                CalculatorSearchableOptionKind::LightstoneSet,
                &signals.lightstone_set,
                &lightstone_sets,
                true,
                "Search lightstone sets",
                false,
            ),
        ),
        (
            "__BACKPACK_SELECT__",
            render_searchable_select_control(
                data.cdn_base_url.as_str(),
                data.lang,
                "calculator-backpack-picker",
                "calculator-backpack-value",
                "backpack",
                CalculatorSearchableOptionKind::Backpack,
                &signals.backpack,
                &backpacks,
                true,
                "Search backpacks",
                false,
            ),
        ),
        (
            "__OUTFITS__",
            render_checkbox_group(
                data.cdn_base_url.as_str(),
                "outfits",
                "outfit",
                &signals.outfit,
                &outfits,
                None,
            ),
        ),
        (
            "__FOODS__",
            render_searchable_multiselect_control(
                data.cdn_base_url.as_str(),
                &SearchableMultiselectConfig {
                    root_id: "calculator-food-picker",
                    bind_key: "food",
                    search_placeholder: "Search foods by name or effect",
                    helper_text: Some(
                        "Only one food family applies at a time. Higher-tier foods replace lower-tier ones in the same family.",
                    ),
                },
                &signals.food,
                &foods,
            ),
        ),
        (
            "__BUFFS__",
            render_searchable_multiselect_control(
                data.cdn_base_url.as_str(),
                &SearchableMultiselectConfig {
                    root_id: "calculator-buff-picker",
                    bind_key: "buff",
                    search_placeholder: "Search buffs by name or effect",
                    helper_text: Some(
                        "Selecting another buff in the same buff group replaces the previous one.",
                    ),
                },
                &signals.buff,
                &buffs,
            ),
        ),
        (
            "__PETS__",
            render_pet_cards(
                data.cdn_base_url.as_str(),
                data.lang,
                &data.catalog.pets,
                signals,
            ),
        ),
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
            item: None,
            lifeskill_level: None,
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

fn render_searchable_dropdown_text_content(label: &str) -> String {
    format!(
        "<span class=\"truncate font-medium\">{}</span>",
        escape_html(label)
    )
}

fn romanize_category_level(level: i32) -> &'static str {
    match level {
        0 => "I",
        1 => "II",
        2 => "III",
        3 => "IV",
        4 => "V",
        _ => "",
    }
}

fn buff_category_label(item: &CalculatorItemEntry) -> Option<String> {
    let base = match item.buff_category_id {
        Some(1) => Some("Meal"),
        Some(2) => Some("Elixir"),
        Some(6) => Some("Perfume"),
        Some(18) => Some("Housekeeper"),
        Some(2002) => Some("Event"),
        _ => None,
    };
    if let Some(base) = base {
        let suffix = romanize_category_level(item.buff_category_level.unwrap_or(0));
        return if suffix.is_empty() {
            Some(base.to_string())
        } else {
            Some(format!("{base} {suffix}"))
        };
    }

    match item.buff_category_key.as_deref() {
        Some(key) if key.starts_with("skill-family:") => {
            Some(format!("Skill {}", &key["skill-family:".len()..]))
        }
        Some(key) if key.starts_with("buff-category:") => {
            let suffix = romanize_category_level(item.buff_category_level.unwrap_or(0));
            let label = format!("Category {}", &key["buff-category:".len()..]);
            if suffix.is_empty() {
                Some(label)
            } else {
                Some(format!("{label} {suffix}"))
            }
        }
        Some(key) => Some(key.to_string()),
        None => None,
    }
}

fn format_effect_percent(value: f32) -> String {
    trim_float(f64::from(value) * 100.0)
}

fn render_effect_badge(label: &str, class_name: &str) -> String {
    format!(
        "<span class=\"badge badge-xs whitespace-nowrap border font-medium {class_name}\">{}</span>",
        escape_html(label)
    )
}

fn render_distribution_chart(chart_id: &str, aria_label: &str, signal_path: &str) -> String {
    format!(
        "<fishy-distribution-chart id=\"{}\" class=\"distribution-chart\" aria-label=\"{}\" signal-path=\"{}\"></fishy-distribution-chart>",
        escape_html(chart_id),
        escape_html(aria_label),
        escape_html(signal_path),
    )
}

fn fish_group_distribution_segments(rows: &[FishGroupChartRow]) -> Vec<DistributionChartSegment> {
    rows.iter()
        .map(|row| DistributionChartSegment {
            label: row.label.to_string(),
            value_text: format!("{}%", trim_float(row.current_share_pct)),
            width_pct: row.current_share_pct,
            fill_color: row.fill_color,
            stroke_color: row.stroke_color,
            text_color: row.text_color,
            connector_color: row.connector_color,
        })
        .collect()
}

fn loot_distribution_segments(rows: &[LootChartRow]) -> Vec<DistributionChartSegment> {
    rows.iter()
        .map(|row| DistributionChartSegment {
            label: row.label.to_string(),
            value_text: row.expected_count_text.clone(),
            width_pct: row.current_share_pct,
            fill_color: row.fill_color,
            stroke_color: row.stroke_color,
            text_color: row.text_color,
            connector_color: row.connector_color,
        })
        .collect()
}

fn render_loot_sankey(chart: &LootChart) -> String {
    if chart.species_rows.is_empty() {
        return "<div class=\"rounded-box border border-dashed border-base-300 bg-base-200 p-4 text-sm text-base-content/70\">No source-backed species rows are available for this zone yet.</div>".to_string();
    }
    format!(
        "<div class=\"rounded-box border border-base-300 bg-base-200 p-4\"><div class=\"mb-3 flex items-center justify-between gap-3\"><div><div class=\"text-sm font-medium\">Species Sankey</div><div class=\"text-xs text-base-content/70\">Each flow starts at a fish group and ends at a source-backed species row. Node sizes stay count-based; enabling silver adds value labels without changing the flow widths.</div></div><div class=\"text-right text-xs text-base-content/70\">{}</div></div><div class=\"overflow-x-auto loot-sankey-scroll\"><fishy-loot-sankey class=\"loot-sankey\" aria-label=\"Expected loot flows from groups to species\" signal-path=\"_calc.loot_sankey_chart\"></fishy-loot-sankey></div></div>",
        if chart.show_silver_amounts {
            "count + silver labels"
        } else {
            "count labels only"
        },
    )
}

fn render_fish_group_chart(chart: &FishGroupChart) -> String {
    if !chart.available {
        return format!(
            "<div id=\"calculator-fish-group-chart\" class=\"rounded-box border border-dashed border-base-300 bg-base-200 p-4 text-sm text-base-content/70\">{}</div>",
            escape_html(&chart.note)
        );
    }

    let mut html = String::new();
    html.push_str("<div id=\"calculator-fish-group-chart\" class=\"grid gap-4\">");
    write!(
        html,
        "<div class=\"rounded-box border border-base-300 bg-base-200 p-4\"><div class=\"mb-3 flex items-center justify-between gap-3\"><div><div class=\"text-sm font-medium\">Raw Prize Catch Rate</div><div class=\"text-xs text-base-content/70\">Mastery {} drives the direct prize-rate formula before normalization.</div></div><div class=\"text-right\"><div class=\"text-2xl font-semibold\">{}</div><div class=\"text-xs text-base-content/70\">before zone-group normalization</div></div></div><div class=\"mb-3 text-xs text-base-content/70\">{}</div>{}",
        escape_html(&chart.mastery_text),
        escape_html(&chart.raw_prize_rate_text),
        escape_html(&chart.note),
        render_distribution_chart(
            "fish-group-distribution-chart",
            "Current fish group distribution",
            "_calc.fish_group_distribution_chart",
        ),
    )
    .unwrap();
    html.push_str("</div></div>");
    html
}

fn render_loot_chart(chart: &LootChart) -> String {
    if !chart.available {
        return format!(
            "<div id=\"calculator-loot-chart\" class=\"rounded-box border border-dashed border-base-300 bg-base-200 p-4 text-sm text-base-content/70\">{}</div>",
            escape_html(&chart.note)
        );
    }

    let mut html = String::new();
    html.push_str("<div id=\"calculator-loot-chart\" class=\"grid gap-4\">");
    write!(
        html,
        "<div class=\"rounded-box border border-base-300 bg-base-200 p-4\"><div class=\"mb-3 flex items-center justify-between gap-3\"><div><div class=\"text-sm font-medium\">Expected Group Haul</div><div class=\"text-xs text-base-content/70\">{}</div></div><div class=\"text-right\"><div class=\"text-lg font-semibold\">Fish {}</div><div class=\"text-xs text-base-content/70\">bargain {} • sale {}</div></div></div>{}</div>{}",
        escape_html(&chart.note),
        escape_html(&chart.fish_multiplier_text),
        escape_html(&chart.trade_bargain_bonus_text),
        escape_html(&chart.trade_sale_multiplier_text),
        render_distribution_chart(
            "loot-distribution-chart",
            "Expected loot distribution by fish group",
            "_calc.loot_distribution_chart",
        ),
        render_loot_sankey(chart),
    )
    .unwrap();
    html.push_str("</div>");
    html
}

fn render_fish_group_window(chart: &FishGroupChart, mastery: f64) -> String {
    format!(
        "<fieldset id=\"calculator-fish-group-window\" class=\"card card-border bg-base-100 xl:col-span-2\">\
            <legend class=\"fieldset-legend ml-6 px-2\">Fish Groups</legend>\
            <div class=\"card-body gap-4 pt-0\">\
                <div class=\"grid gap-4 lg:grid-cols-[minmax(0,14rem)_minmax(0,1fr)] lg:items-start\">\
                    <fieldset class=\"fieldset\">\
                        <legend class=\"fieldset-legend\">Mastery</legend>\
                        <input type=\"number\" min=\"0\" step=\"50\" class=\"input input-sm w-full\" data-bind=\"mastery\" value=\"{}\">\
                        <span class=\"label text-xs\">Enter your consolidated fishing mastery directly.</span>\
                    </fieldset>\
                    {}\
                </div>\
            </div>\
        </fieldset>",
        escape_html(&trim_float(mastery)),
        render_fish_group_chart(chart),
    )
}

fn render_loot_window(
    data: &CalculatorData,
    signals: &CalculatorSignals,
    trade_levels: &[SelectOption<'_>],
    chart: &LootChart,
) -> String {
    format!(
        "<fieldset id=\"calculator-loot-window\" class=\"card card-border bg-base-100 xl:col-span-2\">\
            <legend class=\"fieldset-legend ml-6 px-2\">Loot</legend>\
            <div class=\"card-body gap-4 pt-0\">\
                <div class=\"grid gap-4 xl:grid-cols-[minmax(0,20rem)_minmax(0,1fr)] xl:items-start\">\
                    <div class=\"grid gap-4\">\
                        <div class=\"stats stats-vertical rounded-box border border-base-300 bg-base-100 shadow-none\">\
                            <div class=\"stat\">\
                                <div class=\"stat-title whitespace-normal leading-snug\">Expected Catches (<span data-text=\"$_live.timespan_text || '8 hours'\"></span>)</div>\
                                <div class=\"stat-value text-2xl\" data-text=\"$_live.loot_total_catches\"></div>\
                                <div class=\"stat-desc\">using <span data-text=\"$_live.loot_fish_multiplier_text\"></span> per cast</div>\
                            </div>\
                            <div class=\"stat\">\
                                <div class=\"stat-title\">Expected Catches / Hour</div>\
                                <div class=\"stat-value text-2xl\" data-text=\"$_live.loot_fish_per_hour\"></div>\
                            </div>\
                            <div class=\"stat\">\
                                <div class=\"stat-title whitespace-normal leading-snug\">Expected Profit (<span data-text=\"$_live.timespan_text || '8 hours'\"></span>)</div>\
                                <div class=\"stat-value text-2xl\" data-text=\"$_live.loot_total_profit\"></div>\
                                <div class=\"stat-desc\">sale <span data-text=\"$_calc.trade_sale_multiplier_text\"></span></div>\
                            </div>\
                            <div class=\"stat\">\
                                <div class=\"stat-title\">Profit / Hour</div>\
                                <div class=\"stat-value text-2xl\" data-text=\"$_live.loot_profit_per_hour\"></div>\
                            </div>\
                        </div>\
                        <fieldset class=\"fieldset rounded-box border border-base-300 bg-base-200 p-4\">\
                            <legend class=\"fieldset-legend\">Trade</legend>\
                            <div class=\"grid gap-3\">\
                                <fieldset class=\"fieldset\">\
                                    <legend class=\"fieldset-legend\">Trade Level</legend>\
                                    {}\
                                </fieldset>\
                                <div class=\"grid gap-3 sm:grid-cols-2\">\
                                    <fieldset class=\"fieldset\">\
                                        <legend class=\"fieldset-legend\">Distance Bonus</legend>\
                                        <input type=\"number\" min=\"0\" step=\"any\" class=\"input input-sm w-full\" data-bind=\"tradeDistanceBonus\">\
                                        <span class=\"label text-xs\">manual % bonus, capped at +150% in the sale formula</span>\
                                    </fieldset>\
                                    <fieldset class=\"fieldset\">\
                                        <legend class=\"fieldset-legend\">Trade Price Curve</legend>\
                                        <input type=\"number\" min=\"0\" step=\"any\" class=\"input input-sm w-full\" data-bind=\"tradePriceCurve\">\
                                        <span class=\"label text-xs\">manual % curve, commonly around 105–130%</span>\
                                    </fieldset>\
                                </div>\
                                <label class=\"label cursor-pointer justify-start gap-3 rounded-box border border-base-300 bg-base-100 px-3 py-3\">\
                                    <input data-bind=\"applyTradeModifiers\" type=\"checkbox\" class=\"checkbox checkbox-primary checkbox-sm\">\
                                    <span class=\"text-sm font-medium\">Apply Trade Settings</span>\
                                </label>\
                                <div class=\"grid gap-3 sm:grid-cols-2\">\
                                    <div class=\"rounded-box border border-base-300 bg-base-100 px-3 py-2 text-sm\"><span class=\"block text-xs text-base-content/70\">Bargain Bonus</span><span class=\"font-medium\" data-text=\"$_calc.trade_bargain_bonus_text\"></span></div>\
                                    <div class=\"rounded-box border border-base-300 bg-base-100 px-3 py-2 text-sm\"><span class=\"block text-xs text-base-content/70\">Sale Multiplier</span><span class=\"font-medium\" data-text=\"$_calc.trade_sale_multiplier_text\"></span></div>\
                                </div>\
                            </div>\
                        </fieldset>\
                        <fieldset class=\"fieldset rounded-box border border-base-300 bg-base-200 p-4\">\
                            <legend class=\"fieldset-legend\">Loot View</legend>\
                            <div class=\"grid gap-3\">\
                                <label class=\"label cursor-pointer justify-start gap-3 rounded-box border border-base-300 bg-base-100 px-3 py-3\">\
                                    <input data-bind=\"showSilverAmounts\" type=\"checkbox\" class=\"checkbox checkbox-primary checkbox-sm\">\
                                    <span class=\"text-sm font-medium\">Show silver amounts for groups and species</span>\
                                </label>\
                            </div>\
                        </fieldset>\
                        <fieldset class=\"fieldset rounded-box border border-base-300 bg-base-200 p-4\">\
                            <legend class=\"fieldset-legend\">Auto-Discard Fish</legend>\
                            <div class=\"grid gap-3\">\
                                <fieldset class=\"fieldset\">\
                                    <legend class=\"fieldset-legend\">Discard Up To Grade</legend>\
                                    <select data-bind=\"discardGrade\" class=\"select select-sm w-full\">\
                                        <option value=\"none\">Do not discard</option>\
                                        <option value=\"white\">White</option>\
                                        <option value=\"green\">Green</option>\
                                        <option value=\"blue\">Blue</option>\
                                        <option value=\"yellow\">Yellow</option>\
                                    </select>\
                                </fieldset>\
                            </div>\
                            <span class=\"label text-xs\">Only fish are discarded. Non-fish loot remains included. Red fish are always kept.</span>\
                        </fieldset>\
                    </div>\
                    <div class=\"grid gap-4\">\
                        {}\
                    </div>\
                </div>\
            </div>\
        </fieldset>",
        render_searchable_select_control(
            data.cdn_base_url.as_str(),
            data.lang,
            "calculator-trade-level-picker",
            "calculator-trade-level-value",
            "trade_level",
            CalculatorSearchableOptionKind::TradeLevel,
            &signals.trade_level,
            trade_levels,
            false,
            "Search trade levels",
            false,
        ),
        render_loot_chart(chart),
    )
}

fn render_item_effect_badges(item: &CalculatorItemEntry) -> String {
    let mut badges = Vec::new();
    if let Some(category_label) = buff_category_label(item) {
        badges.push(render_effect_badge(
            &category_label,
            "border-base-content/15 bg-base-300 text-base-content",
        ));
    }
    if let Some(afr) = item.afr.filter(|value| *value > 0.0) {
        badges.push(render_effect_badge(
            &format!("-{}% AFT", format_effect_percent(afr)),
            "border-blue-400 bg-blue-300 text-blue-950",
        ));
    }
    if let Some(bonus_rare) = item.bonus_rare.filter(|value| *value > 0.0) {
        badges.push(render_effect_badge(
            &format!("+{}% Rare", format_effect_percent(bonus_rare)),
            "border-yellow-400 bg-yellow-300 text-yellow-950",
        ));
    }
    if let Some(bonus_big) = item.bonus_big.filter(|value| *value > 0.0) {
        badges.push(render_effect_badge(
            &format!("+{}% HQ", format_effect_percent(bonus_big)),
            "border-blue-400 bg-blue-300 text-blue-950",
        ));
    }
    if let Some(item_drr) = item.item_drr.filter(|value| *value > 0.0) {
        badges.push(render_effect_badge(
            &format!("+{}% Item DRR", format_effect_percent(item_drr)),
            "border-amber-400 bg-amber-300 text-amber-950",
        ));
    }
    if let Some(fish_multiplier) = item
        .fish_multiplier
        .filter(|value| *value > 0.0 && (*value - 1.0).abs() > 0.0001)
    {
        badges.push(render_effect_badge(
            &format!("Fish ×{}", trim_float(f64::from(fish_multiplier))),
            "border-base-content/15 bg-base-300 text-base-content",
        ));
    }
    if let Some(exp_fish) = item.exp_fish.filter(|value| *value > 0.0) {
        badges.push(render_effect_badge(
            &format!("+{}% Fish EXP", format_effect_percent(exp_fish)),
            "border-cyan-400 bg-cyan-300 text-cyan-950",
        ));
    }
    if let Some(exp_life) = item.exp_life.filter(|value| *value > 0.0) {
        badges.push(render_effect_badge(
            &format!("+{}% Life EXP", format_effect_percent(exp_life)),
            "border-green-400 bg-green-300 text-green-950",
        ));
    }
    if badges.is_empty() && item.r#type == "outfit" {
        badges.push(render_effect_badge(
            "Set effect",
            "border-base-content/15 bg-base-300 text-base-content",
        ));
    }
    if badges.is_empty() {
        return String::new();
    }
    format!(
        "<span class=\"mt-1 flex flex-wrap gap-1\">{}</span>",
        badges.join("")
    )
}

fn render_item_effect_search_text(item: &CalculatorItemEntry) -> String {
    let mut parts = Vec::<String>::new();
    if item.afr.filter(|value| *value > 0.0).is_some() {
        parts.extend(
            ["aft", "auto fishing", "auto-fishing", "auto fish time"]
                .into_iter()
                .map(ToOwned::to_owned),
        );
    }
    if item.bonus_rare.filter(|value| *value > 0.0).is_some() {
        parts.extend(["rare", "rare fish"].into_iter().map(ToOwned::to_owned));
    }
    if item.bonus_big.filter(|value| *value > 0.0).is_some() {
        parts.extend(
            ["hq", "high quality", "high-quality", "big fish"]
                .into_iter()
                .map(ToOwned::to_owned),
        );
    }
    if item.item_drr.filter(|value| *value > 0.0).is_some() {
        parts.extend(
            ["item drr", "durability reduction resistance", "durability"]
                .into_iter()
                .map(ToOwned::to_owned),
        );
    }
    if item.exp_fish.filter(|value| *value > 0.0).is_some() {
        parts.extend(
            ["fish exp", "fishing exp", "fishing experience"]
                .into_iter()
                .map(ToOwned::to_owned),
        );
    }
    if item.exp_life.filter(|value| *value > 0.0).is_some() {
        parts.extend(
            ["life exp", "life experience"]
                .into_iter()
                .map(ToOwned::to_owned),
        );
    }
    if item
        .fish_multiplier
        .filter(|value| *value > 0.0 && (*value - 1.0).abs() > 0.0001)
        .is_some()
    {
        parts.extend(
            ["fish multiplier", "fish value"]
                .into_iter()
                .map(ToOwned::to_owned),
        );
    }
    if item.r#type == "outfit" {
        parts.extend(
            ["set effect", "set bonus"]
                .into_iter()
                .map(ToOwned::to_owned),
        );
    }
    if let Some(category_label) = buff_category_label(item) {
        parts.push(category_label);
        parts.push("buff category".to_string());
        parts.push("exclusive group".to_string());
        if matches!(item.buff_category_id, Some(1)) {
            parts.push("meal".to_string());
        }
    }
    parts.join(" ")
}

fn render_searchable_dropdown_option_content_html(
    cdn_base_url: &str,
    option: SelectOption<'_>,
) -> String {
    let mut html = String::new();
    if let Some(icon) = option.icon {
        write!(
            html,
            "<img aria-hidden=\"true\" src=\"{}\" class=\"item-icon\" alt=\"{} icon\"/>",
            escape_html(&absolute_public_asset_url(cdn_base_url, icon)),
            escape_html(option.label)
        )
        .unwrap();
    }
    let badges = option
        .item
        .map(render_item_effect_badges)
        .or_else(|| {
            option.lifeskill_level.map(|level| {
                format!(
                    "<span class=\"mt-1 flex flex-wrap gap-1\">{}</span>",
                    render_effect_badge(
                        &format!(
                            "+{}% Lv DRR",
                            format_effect_percent(level.lifeskill_level_drr)
                        ),
                        "border-amber-400 bg-amber-300 text-amber-950",
                    )
                )
            })
        })
        .unwrap_or_default();
    write!(
        html,
        "<span class=\"min-w-0 flex-1\"><span class=\"block truncate font-medium\">{}</span>{}</span>",
        escape_html(option.label),
        badges,
    )
    .unwrap();
    html
}

fn with_optional_none<'a>(
    options: &[SelectOption<'a>],
    include_none: bool,
) -> Vec<SelectOption<'a>> {
    let mut values = Vec::with_capacity(options.len() + usize::from(include_none));
    if include_none {
        values.push(NONE_SELECT_OPTION);
    }
    values.extend_from_slice(options);
    values
}

fn searchable_options_for_kind<'a>(
    data: &'a CalculatorData,
    kind: CalculatorSearchableOptionKind,
) -> (Vec<SelectOption<'a>>, bool) {
    match kind {
        CalculatorSearchableOptionKind::FishingLevel => (
            select_options_from_catalog(&data.catalog.fishing_levels),
            false,
        ),
        CalculatorSearchableOptionKind::SessionUnit => (
            select_options_from_catalog(&data.catalog.session_units),
            false,
        ),
        CalculatorSearchableOptionKind::LifeskillLevel => (
            sorted_lifeskill_options(&data.catalog.lifeskill_levels),
            false,
        ),
        CalculatorSearchableOptionKind::TradeLevel => (
            select_options_from_catalog(&data.catalog.trade_levels),
            false,
        ),
        CalculatorSearchableOptionKind::Rod => {
            (item_options_by_type(&data.catalog.items, "rod"), false)
        }
        CalculatorSearchableOptionKind::Float => {
            (item_options_by_type(&data.catalog.items, "float"), true)
        }
        CalculatorSearchableOptionKind::Chair => {
            (item_options_by_type(&data.catalog.items, "chair"), true)
        }
        CalculatorSearchableOptionKind::LightstoneSet => (
            item_options_by_type(&data.catalog.items, "lightstone_set"),
            true,
        ),
        CalculatorSearchableOptionKind::Backpack => {
            (item_options_by_type(&data.catalog.items, "backpack"), true)
        }
        CalculatorSearchableOptionKind::PetTier => {
            (select_options_from_catalog(&data.catalog.pets.tiers), false)
        }
        CalculatorSearchableOptionKind::PetSpecial => (
            select_options_from_catalog(&data.catalog.pets.specials),
            false,
        ),
        CalculatorSearchableOptionKind::PetTalent => (
            select_options_from_catalog(&data.catalog.pets.talents),
            false,
        ),
    }
}

fn fuzzy_select_matches<'a>(
    options: &[SelectOption<'a>],
    query: &str,
    current_value: &str,
) -> Vec<SelectOption<'a>> {
    let mut options = options.to_vec();
    options.sort_by(|left, right| left.label.cmp(right.label));

    let trimmed = query.trim();
    if trimmed.is_empty() {
        options.sort_by_key(|option| {
            (
                if option.value == current_value { 0 } else { 1 },
                option.label.to_string(),
            )
        });
        options.truncate(SEARCHABLE_DROPDOWN_RESULT_LIMIT);
        return options;
    }

    let matcher = SkimMatcherV2::default();
    let normalized_query = normalize_lookup_value(trimmed);
    let mut scored = options
        .into_iter()
        .filter_map(|option| {
            matcher
                .fuzzy_match(&normalize_lookup_value(option.label), &normalized_query)
                .map(|score| (option, score))
        })
        .collect::<Vec<_>>();
    scored.sort_by_key(|(option, score)| (Reverse(*score), option.label.to_string()));
    scored.truncate(SEARCHABLE_DROPDOWN_RESULT_LIMIT);
    scored.into_iter().map(|(option, _)| option).collect()
}

fn render_searchable_dropdown_catalog_html(
    cdn_base_url: &str,
    options: &[SelectOption<'_>],
) -> String {
    let mut html = String::new();
    html.push_str("<div data-role=\"selected-content-catalog\" hidden>");
    for option in options {
        write!(
            html,
            "<template data-role=\"selected-content\" data-value=\"{}\" data-label=\"{}\">{}</template>",
            escape_html(option.value),
            escape_html(option.label),
            render_searchable_dropdown_option_content_html(cdn_base_url, *option),
        )
        .unwrap();
    }
    html.push_str("</div>");
    html
}

fn render_searchable_select_results(
    cdn_base_url: &str,
    results_list_id: &str,
    options: &[SelectOption<'_>],
    current_value: &str,
    query: &str,
) -> String {
    let matches = fuzzy_select_matches(options, query, current_value);
    let mut html = String::new();
    write!(
        html,
        "<ul id=\"{}\" tabindex=\"-1\" data-role=\"results\" class=\"menu menu-sm max-h-96 w-full gap-1 overflow-auto p-1\">",
        escape_html(results_list_id),
    )
    .unwrap();
    if matches.is_empty() {
        html.push_str("<li class=\"menu-disabled\"><span>No matching options</span></li>");
    } else {
        for option in matches {
            let is_selected = option.value == current_value;
            write!(
                html,
                "<li><button type=\"button\" class=\"justify-between gap-3 text-left{}\" data-searchable-dropdown-option data-value=\"{}\" data-label=\"{}\"><span data-role=\"option-content\" class=\"flex min-w-0 flex-1 items-center gap-3\">{}</span>{}</button></li>",
                if is_selected { " menu-active" } else { "" },
                escape_html(option.value),
                escape_html(option.label),
                render_searchable_dropdown_option_content_html(cdn_base_url, option),
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

fn render_calculator_option_search_url(
    lang: FishLang,
    kind: CalculatorSearchableOptionKind,
    results_id: &str,
) -> String {
    format!(
        "/api/v1/calculator/datastar/option-search?lang={}&kind={}&results_id={}",
        lang_param(lang),
        kind.param(),
        results_id,
    )
}

fn render_searchable_select_control(
    cdn_base_url: &str,
    lang: FishLang,
    root_id: &str,
    input_id: &str,
    bind_key: &str,
    kind: CalculatorSearchableOptionKind,
    selected_value: &str,
    options: &[SelectOption<'_>],
    include_none: bool,
    search_placeholder: &str,
    compact: bool,
) -> String {
    let results_id = format!("{root_id}-results");
    let options = with_optional_none(options, include_none);
    let selected_option = options
        .iter()
        .copied()
        .find(|option| option.value == selected_value);
    let selected_label = selected_option
        .map(|option| option.label)
        .unwrap_or_else(|| {
            if selected_value.trim().is_empty() {
                NONE_SELECT_OPTION.label
            } else {
                selected_value
            }
        });
    let selected_content_html = selected_option
        .map(|option| render_searchable_dropdown_option_content_html(cdn_base_url, option))
        .unwrap_or_else(|| render_searchable_dropdown_text_content(selected_label));
    let catalog_html = render_searchable_dropdown_catalog_html(cdn_base_url, &options);
    let results_html =
        render_searchable_select_results(cdn_base_url, &results_id, &options, selected_value, "");
    let search_url = render_calculator_option_search_url(lang, kind, &results_id);
    let dropdown = render_searchable_dropdown(
        &SearchableDropdownConfig {
            catalog_html: Some(&catalog_html),
            compact,
            root_id,
            input_id,
            label: selected_label,
            selected_content_html: &selected_content_html,
            value: selected_value,
            search_url: &search_url,
            search_url_root: Some("api"),
            search_placeholder,
        },
        &results_html,
    );

    format!(
        "<input id=\"{}\" type=\"hidden\" data-bind=\"{}\" value=\"{}\">{}",
        escape_html(input_id),
        escape_html(bind_key),
        escape_html(selected_value),
        dropdown,
    )
}

fn render_searchable_multiselect_search_text(option: SelectOption<'_>) -> String {
    let mut parts = vec![option.label.to_string()];
    if let Some(item) = option.item {
        let effect_terms = render_item_effect_search_text(item);
        if !effect_terms.is_empty() {
            parts.push(effect_terms);
        }
    }
    parts.join(" ")
}

fn render_searchable_multiselect_catalog_html(
    cdn_base_url: &str,
    options: &[SelectOption<'_>],
) -> String {
    let mut html = String::new();
    html.push_str("<div data-role=\"catalog\" hidden>");
    for option in options {
        let category_key_attr = option
            .item
            .and_then(|item| item.buff_category_key.as_deref())
            .map(|value| format!(" data-category-key=\"{}\"", escape_html(value)))
            .unwrap_or_default();
        write!(
            html,
            "<template data-role=\"option-template\" data-value=\"{}\" data-label=\"{}\" data-search-text=\"{}\"{}>{}</template>",
            escape_html(option.value),
            escape_html(option.label),
            escape_html(&render_searchable_multiselect_search_text(*option)),
            category_key_attr,
            render_searchable_dropdown_option_content_html(cdn_base_url, *option),
        )
        .unwrap();
    }
    html.push_str("</div>");
    html
}

fn render_searchable_multiselect_inputs_html(
    bind_key: &str,
    selected_values: &[String],
    options: &[SelectOption<'_>],
) -> String {
    let selected = selected_values
        .iter()
        .map(|value| value.as_str())
        .collect::<std::collections::HashSet<_>>();
    let mut html = String::new();
    html.push_str("<div data-role=\"bound-inputs\" hidden>");
    for option in options {
        let checked_attr = if selected.contains(option.value) {
            " checked"
        } else {
            ""
        };
        let category_key_attr = option
            .item
            .and_then(|item| item.buff_category_key.as_deref())
            .map(|value| format!(" data-category-key=\"{}\"", escape_html(value)))
            .unwrap_or_default();
        write!(
            html,
            "<input data-role=\"bound-option\" data-bind=\"{}\" data-label=\"{}\" type=\"checkbox\" value=\"{}\"{}{}>",
            escape_html(bind_key),
            escape_html(option.label),
            escape_html(option.value),
            checked_attr,
            category_key_attr,
        )
        .unwrap();
    }
    html.push_str("</div>");
    html
}

fn render_searchable_multiselect_selection_html(
    cdn_base_url: &str,
    selected_values: &[String],
    options: &[SelectOption<'_>],
) -> String {
    let selected_lookup = options
        .iter()
        .copied()
        .map(|option| (option.value, option))
        .collect::<HashMap<_, _>>();
    let mut html = String::new();
    for value in selected_values {
        let Some(option) = selected_lookup.get(value.as_str()).copied() else {
            continue;
        };
        write!(
            html,
            "<div class=\"join items-stretch rounded-box border border-base-300 bg-base-100 p-1 text-base-content shadow-sm\"><span class=\"inline-flex min-w-0 items-center px-2 py-1 text-sm\">{}</span><button type=\"button\" class=\"btn btn-ghost btn-xs btn-circle join-item h-7 min-h-0 w-7 border-0 text-base-content/70\" data-searchable-multiselect-remove data-value=\"{}\" aria-label=\"Remove {}\">×</button></div>",
            render_searchable_dropdown_option_content_html(cdn_base_url, option),
            escape_html(option.value),
            escape_html(option.label),
        )
        .unwrap();
    }
    html
}

fn render_searchable_multiselect_results_html(
    cdn_base_url: &str,
    options: &[SelectOption<'_>],
    selected_values: &[String],
    query: &str,
) -> String {
    let selected = selected_values
        .iter()
        .map(|value| value.as_str())
        .collect::<std::collections::HashSet<_>>();
    let normalized_query = normalize_lookup_value(query);
    let mut matches = options
        .iter()
        .copied()
        .filter(|option| {
            if normalized_query.is_empty() {
                return true;
            }
            let haystack =
                normalize_lookup_value(&render_searchable_multiselect_search_text(*option));
            normalized_query
                .split_whitespace()
                .all(|part| haystack.contains(part))
        })
        .collect::<Vec<_>>();
    matches.sort_by_key(|option| (selected.contains(option.value), option.label.to_string()));
    matches.truncate(SEARCHABLE_DROPDOWN_RESULT_LIMIT);

    let mut html = String::new();
    html.push_str(
        "<ul data-role=\"results\" class=\"menu menu-sm max-h-96 w-full gap-1 overflow-auto p-1\">",
    );
    if matches.is_empty() {
        html.push_str("<li class=\"menu-disabled\"><span>No matching options</span></li>");
    } else {
        for option in matches {
            let is_selected = selected.contains(option.value);
            write!(
                html,
                "<li><button type=\"button\" class=\"justify-between gap-3 text-left{}\" data-searchable-multiselect-option data-selected=\"{}\" data-value=\"{}\" data-label=\"{}\"><span class=\"flex min-w-0 flex-1 items-center gap-3\">{}</span>{}</button></li>",
                if is_selected { " opacity-75" } else { "" },
                if is_selected { "true" } else { "false" },
                escape_html(option.value),
                escape_html(option.label),
                render_searchable_dropdown_option_content_html(cdn_base_url, option),
                if is_selected {
                    "<span class=\"badge badge-soft badge-primary badge-xs\">Added</span>"
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

fn render_searchable_multiselect_control(
    cdn_base_url: &str,
    config: &SearchableMultiselectConfig<'_>,
    selected_values: &[String],
    options: &[SelectOption<'_>],
) -> String {
    let mut options = options.to_vec();
    options.sort_by(|left, right| left.label.cmp(right.label));

    let panel_id = format!("{}-panel", config.root_id);
    let search_input_id = format!("{}-search-input", config.root_id);
    let selection_html =
        render_searchable_multiselect_selection_html(cdn_base_url, selected_values, &options);
    let results_html =
        render_searchable_multiselect_results_html(cdn_base_url, &options, selected_values, "");
    let catalog_html = render_searchable_multiselect_catalog_html(cdn_base_url, &options);
    let inputs_html =
        render_searchable_multiselect_inputs_html(config.bind_key, selected_values, &options);
    let selection_hidden_attr = if selection_html.is_empty() {
        " hidden"
    } else {
        ""
    };
    let helper_text_html = config
        .helper_text
        .map(|helper_text| {
            format!(
                "<p class=\"text-xs text-base-content/60\">{}</p>",
                escape_html(helper_text)
            )
        })
        .unwrap_or_default();

    format!(
        r#"<fishy-searchable-multiselect id="{root_id}" class="relative block w-full" placeholder="{search_placeholder}">
    <div data-role="shell" class="flex min-h-11 w-full flex-wrap items-center gap-2 rounded-box border border-base-300 bg-base-100 px-3 py-2 shadow-sm">
        <div data-role="selection" class="flex flex-wrap gap-2"{selection_hidden_attr}>{selection_html}</div>
        <label class="flex min-w-[12rem] flex-1 items-center gap-2 text-sm">
            <svg class="fishy-icon size-4 opacity-60" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="/img/icons.svg?v=20260325-2#fishy-search-field"></use></svg>
            <input id="{search_input_id}"
                   data-role="search-input"
                   type="search"
                   class="w-full min-w-0 border-0 bg-transparent p-0 shadow-none outline-none"
                   style="outline: none; box-shadow: none;"
                   placeholder="{search_placeholder}"
                   autocomplete="off"
                   spellcheck="false">
        </label>
    </div>
    <div id="{panel_id}" data-role="panel" class="absolute left-0 top-full z-50 mt-2 w-full min-w-full max-w-full" hidden>
        <div class="grid w-full min-w-full overflow-hidden rounded-box border border-base-300 bg-base-100 shadow-lg">
            <div class="px-1 py-1">
                {results_html}
            </div>
        </div>
    </div>
    {helper_text_html}
    {catalog_html}
    {inputs_html}
</fishy-searchable-multiselect>"#,
        root_id = escape_html(config.root_id),
        panel_id = escape_html(&panel_id),
        search_input_id = escape_html(&search_input_id),
        selection_hidden_attr = selection_hidden_attr,
        selection_html = selection_html,
        search_placeholder = escape_html(config.search_placeholder),
        results_html = results_html,
        helper_text_html = helper_text_html,
        catalog_html = catalog_html,
        inputs_html = inputs_html,
    )
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
            let option_content = render_searchable_dropdown_text_content(label);
            write!(
                html,
                "<li><button type=\"button\" class=\"justify-between gap-3 text-left{}\" data-searchable-dropdown-option data-value=\"{}\" data-label=\"{}\"><span data-role=\"option-content\" class=\"flex min-w-0 flex-1 items-center gap-3\">{}</span>{}</button></li>",
                active_class,
                escape_html(&zone.rgb_key.0),
                escape_html(label),
                option_content,
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
    let catalog_html = config.catalog_html.unwrap_or("");
    let trigger_class = if config.compact {
        "flex min-h-10 w-full items-center justify-between gap-3 rounded-box border border-base-300 bg-base-100 px-3 py-2 text-left text-sm shadow-sm"
    } else {
        "flex min-h-11 w-full items-center justify-between gap-3 rounded-box border border-base-300 bg-base-100 px-4 py-3 text-left shadow-sm"
    };
    let search_shell_class = if config.compact {
        "flex min-h-10 w-full min-w-full items-center gap-3 bg-base-100 px-3 py-2 text-sm"
    } else {
        "flex min-h-11 w-full min-w-full items-center gap-3 bg-base-100 px-4 py-3"
    };
    let selected_content_class = if config.compact {
        "flex min-w-0 flex-1 items-center gap-3 text-sm"
    } else {
        "flex min-w-0 flex-1 items-center gap-3"
    };
    let search_url_root_attr = config
        .search_url_root
        .map(|value| format!(" search-url-root=\"{}\"", escape_html(value)))
        .unwrap_or_default();
    let mut html = String::new();
    write!(
        html,
        r#"<fishy-searchable-dropdown id="{root_id}"
     class="relative block w-full"
     input-id="{input_id}"
     label="{label}"
     value="{value}"
     search-url="{search_url}"{search_url_root_attr}
     placeholder="{search_placeholder}">
    <button type="button"
            data-role="trigger"
            class="{trigger_class}"
            aria-haspopup="listbox"
            aria-expanded="false"
            aria-controls="{panel_id}">
        <span data-role="selected-content" class="{selected_content_class}">{selected_content_html}</span>
        <svg class="fishy-icon size-4 opacity-60" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="/img/icons.svg?v=20260325-2#fishy-caret-down"></use></svg>
    </button>

    <div id="{panel_id}" data-role="panel" class="absolute left-0 top-0 z-50 w-full min-w-full max-w-full" hidden>
        <div class="grid w-full min-w-full overflow-hidden rounded-box border border-base-300 bg-base-100 shadow-lg">
            <label class="{search_shell_class}">
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
    {catalog_html}
</fishy-searchable-dropdown>"#,
        catalog_html = catalog_html,
        root_id = escape_html(config.root_id),
        input_id = escape_html(config.input_id),
        label = escape_html(config.label),
        selected_content_class = selected_content_class,
        selected_content_html = config.selected_content_html,
        search_shell_class = search_shell_class,
        value = escape_html(config.value),
        search_url = escape_html(config.search_url),
        search_url_root_attr = search_url_root_attr,
        panel_id = escape_html(&panel_id),
        search_input_id = escape_html(&search_input_id),
        search_placeholder = escape_html(config.search_placeholder),
        results_html = results_html,
        trigger_class = trigger_class,
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
            item: None,
            lifeskill_level: Some(level),
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
            item: Some(item),
            lifeskill_level: None,
        })
        .collect()
}

fn render_checkbox_group(
    cdn_base_url: &str,
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
    write!(html, "<div id=\"{}\" class=\"block\">", escape_html(id)).unwrap();
    write!(
        html,
        "<div class=\"grid gap-2 sm:grid-cols-2\" {}>",
        change_attr,
    )
    .unwrap();
    for (index, option) in options.iter().enumerate() {
        let checked = if selected.contains(option.value) {
            " checked"
        } else {
            ""
        };
        let category_key_attr = option
            .item
            .and_then(|item| item.buff_category_key.as_deref())
            .map(|value| format!(" data-category-key=\"{}\"", escape_html(value)))
            .unwrap_or_default();
        write!(
            html,
            "<label class=\"label cursor-pointer items-start justify-start gap-3 rounded-box border border-base-300 bg-base-100 px-3 py-2 text-sm\"><input type=\"checkbox\" data-bind=\"{}.{}\" class=\"checkbox checkbox-primary checkbox-sm mt-0.5 shrink-0\" value=\"{}\"{}{}>",
            escape_html(bind_key),
            index,
            escape_html(option.value),
            checked,
            category_key_attr,
        )
        .unwrap();
        if let Some(icon) = option.icon {
            write!(
                html,
                "<img aria-hidden=\"true\" src=\"{}\" class=\"item-icon\" alt=\"{} icon\"/>",
                escape_html(&absolute_public_asset_url(cdn_base_url, icon)),
                escape_html(option.label)
            )
            .unwrap();
        }
        let badges = option
            .item
            .map(render_item_effect_badges)
            .unwrap_or_default();
        write!(
            html,
            "<span class=\"min-w-0 flex-1\"><span class=\"block font-medium\">{}</span>{}</span></label>",
            escape_html(option.label),
            badges,
        )
        .unwrap();
    }
    html.push_str("</div></div>");
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
            "<button type=\"button\" class=\"btn btn-soft btn-sm join-item\" data-on:click=\"$timespanAmount = {}; $timespanUnit = '{}'; window.__fishystuffCalculator.persist($); window.__fishystuffCalculator.requestEval()\">{}</button>",
            trim_float(preset.amount),
            escape_html(&preset.unit),
            escape_html(&preset.label)
        )
        .unwrap();
    }
    html.push_str("</div>");
    html
}

fn render_pet_cards(
    cdn_base_url: &str,
    lang: FishLang,
    catalog: &CalculatorPetCatalog,
    signals: &CalculatorSignals,
) -> String {
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
        html.push_str(&render_searchable_select_control(
            cdn_base_url,
            lang,
            &format!("calculator-pet{slot}-tier-picker"),
            &format!("calculator-pet{slot}-tier-value"),
            &format!("{}.tier", bind_prefix),
            CalculatorSearchableOptionKind::PetTier,
            &pet.tier,
            &tier_options,
            false,
            "Search pet tiers",
            true,
        ));
        html.push_str("</fieldset>");
        html.push_str(
            "<fieldset class=\"fieldset\"><legend class=\"fieldset-legend\">Special</legend>",
        );
        html.push_str(&render_searchable_select_control(
            cdn_base_url,
            lang,
            &format!("calculator-pet{slot}-special-picker"),
            &format!("calculator-pet{slot}-special-value"),
            &format!("{}.special", bind_prefix),
            CalculatorSearchableOptionKind::PetSpecial,
            &pet.special,
            &special_options,
            false,
            "Search pet specials",
            true,
        ));
        html.push_str("</fieldset>");
        html.push_str(
            "<fieldset class=\"fieldset\"><legend class=\"fieldset-legend\">Talent</legend>",
        );
        html.push_str(&render_searchable_select_control(
            cdn_base_url,
            lang,
            &format!("calculator-pet{slot}-talent-picker"),
            &format!("calculator-pet{slot}-talent-value"),
            &format!("{}.talent", bind_prefix),
            CalculatorSearchableOptionKind::PetTalent,
            &pet.talent,
            &talent_options,
            false,
            "Search pet talents",
            true,
        ));
        html.push_str("</fieldset></div>");
        html.push_str("<fieldset class=\"fieldset mt-3 gap-2\"><legend class=\"fieldset-legend\">Skills</legend>");
        html.push_str(&render_checkbox_group(
            "",
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
        CalculatorMasteryPrizeRateEntry, CalculatorOptionEntry, CalculatorPetCatalog,
        CalculatorPetSignals, CalculatorPriceOverrideSignals, CalculatorSignals,
        CalculatorZoneGroupRateEntry,
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
    use serde_json::json;

    use crate::config::{AppConfig, ZoneStatusConfig};
    use crate::error::AppResult;
    use crate::state::{AppState, RequestId};
    use crate::store::{FishLang, Store};

    use super::{
        base_price_for_species, buff_category_label, build_pet_value_aliases,
        discard_grade_enabled, get_calculator_datastar_init, get_calculator_datastar_option_search,
        get_calculator_datastar_zone_search, init_signals_patch_map, normalize_lookup_value,
        mastery_prize_rate_for_bracket, normalize_named_array, parse_calculator_signals_value,
        post_calculator_datastar_eval, trade_sale_multiplier_for_species, CalculatorData,
        CalculatorDatastarQuery, CalculatorQuery, CalculatorSearchableOptionQuery,
        CalculatorZoneSearchQuery,
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
                        icon: Some("/images/items/00016162.webp".to_string()),
                        r#type: "rod".to_string(),
                        afr: Some(0.1),
                        ..CalculatorItemEntry::default()
                    },
                    CalculatorItemEntry {
                        key: "item:705539".to_string(),
                        name: "Manos Fishing Chair".to_string(),
                        icon: Some("/images/items/00705539.webp".to_string()),
                        r#type: "chair".to_string(),
                        afr: Some(0.1),
                        ..CalculatorItemEntry::default()
                    },
                    CalculatorItemEntry {
                        key: "lightstone-set:30".to_string(),
                        name: "Blacksmith's Blessing".to_string(),
                        r#type: "lightstone_set".to_string(),
                        item_drr: Some(0.3),
                        ..CalculatorItemEntry::default()
                    },
                    CalculatorItemEntry {
                        key: "lightstone-set:160".to_string(),
                        name: "Nibbles".to_string(),
                        r#type: "lightstone_set".to_string(),
                        afr: Some(0.15),
                        exp_fish: Some(0.1),
                        ..CalculatorItemEntry::default()
                    },
                    CalculatorItemEntry {
                        key: "item:830150".to_string(),
                        name: "Lil' Otter Fishing Carrier".to_string(),
                        icon: Some("/images/items/00830150.webp".to_string()),
                        r#type: "backpack".to_string(),
                        item_drr: Some(0.05),
                        ..CalculatorItemEntry::default()
                    },
                    CalculatorItemEntry {
                        key: "effect:8-piece-outfit-set-effect".to_string(),
                        name: "8-Piece Outfit Set Effect".to_string(),
                        r#type: "outfit".to_string(),
                        item_drr: Some(0.1),
                        ..CalculatorItemEntry::default()
                    },
                    CalculatorItemEntry {
                        key: "effect:mainhand-weapon-outfit".to_string(),
                        name: "Mainhand Weapon Outfit".to_string(),
                        r#type: "outfit".to_string(),
                        item_drr: Some(0.1),
                        ..CalculatorItemEntry::default()
                    },
                    CalculatorItemEntry {
                        key: "effect:awakening-weapon-outfit".to_string(),
                        name: "Awakening Weapon Outfit".to_string(),
                        r#type: "outfit".to_string(),
                        item_drr: Some(0.1),
                        ..CalculatorItemEntry::default()
                    },
                    CalculatorItemEntry {
                        key: "item:14330".to_string(),
                        name: "Professional Fisher's Uniform (Costume)".to_string(),
                        r#type: "outfit".to_string(),
                        exp_fish: Some(0.1),
                        ..CalculatorItemEntry::default()
                    },
                    CalculatorItemEntry {
                        key: "item:9359".to_string(),
                        name: "Balacs Lunchbox".to_string(),
                        r#type: "food".to_string(),
                        buff_category_key: Some("buff-category:1".to_string()),
                        buff_category_id: Some(1),
                        afr: Some(0.05),
                        ..CalculatorItemEntry::default()
                    },
                    CalculatorItemEntry {
                        key: "item:721092".to_string(),
                        name: "Treant's Tear".to_string(),
                        r#type: "buff".to_string(),
                        buff_category_key: Some("buff-category:6".to_string()),
                        buff_category_id: Some(6),
                        exp_life: Some(0.3),
                        ..CalculatorItemEntry::default()
                    },
                ],
                lifeskill_levels: vec![CalculatorLifeskillLevelEntry {
                    key: "100".to_string(),
                    name: "Guru 20".to_string(),
                    index: 100,
                    order: 100,
                    lifeskill_level_drr: 0.6,
                }],
                mastery_prize_curve: vec![
                    CalculatorMasteryPrizeRateEntry {
                        fishing_mastery: 0,
                        high_drop_rate_raw: 0,
                        high_drop_rate: 0.0,
                    },
                    CalculatorMasteryPrizeRateEntry {
                        fishing_mastery: 1000,
                        high_drop_rate_raw: 25_000,
                        high_drop_rate: 0.025,
                    },
                    CalculatorMasteryPrizeRateEntry {
                        fishing_mastery: 2000,
                        high_drop_rate_raw: 50_000,
                        high_drop_rate: 0.05,
                    },
                ],
                zone_group_rates: vec![CalculatorZoneGroupRateEntry {
                    zone_rgb_key: "240,74,74".to_string(),
                    prize_main_group_key: Some(16424),
                    rare_rate_raw: 100_000,
                    high_quality_rate_raw: 217_500,
                    general_rate_raw: 620_000,
                    trash_rate_raw: 62_500,
                }],
                trade_levels: vec![CalculatorOptionEntry {
                    key: "73".to_string(),
                    label: "Master 23".to_string(),
                }],
                defaults: CalculatorSignals {
                    level: 5,
                    lifeskill_level: "100".to_string(),
                    mastery: 0.0,
                    trade_level: "73".to_string(),
                    zone: "240,74,74".to_string(),
                    resources: 0.0,
                    rod: "item:16162".to_string(),
                    float: String::new(),
                    chair: "item:705539".to_string(),
                    lightstone_set: "lightstone-set:30".to_string(),
                    backpack: "item:830150".to_string(),
                    outfit: vec![
                        "effect:8-piece-outfit-set-effect".to_string(),
                        "effect:mainhand-weapon-outfit".to_string(),
                        "effect:awakening-weapon-outfit".to_string(),
                        "item:14330".to_string(),
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
                    trade_distance_bonus: 134.15,
                    trade_price_curve: 120.0,
                    price_overrides: Default::default(),
                    catch_time_active: 17.5,
                    catch_time_afk: 6.5,
                    timespan_amount: 8.0,
                    timespan_unit: "hours".to_string(),
                    apply_trade_modifiers: true,
                    show_silver_amounts: false,
                    discard_grade: "none".to_string(),
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
            runtime_cdn_base_url: "http://127.0.0.1:4040".to_string(),
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
        assert!(text.contains("data-role=\"selected-content\""));
        assert!(text.contains("kind=rod"));
        assert!(text.contains("calculator-rod-picker"));
        assert!(text.contains("calculator-pet1-tier-picker"));
        assert!(text.contains("<fishy-searchable-multiselect"));
        assert!(text.contains("data-bind=\"outfit.0\""));
        assert!(text.contains("calculator-food-picker"));
        assert!(text.contains("calculator-buff-picker"));
        assert!(text.contains("Search foods by name or effect"));
        assert!(text.contains("data-bind=\"mastery\""));
        assert!(text.contains("step=\"50\""));
        assert!(text.contains("Raw Prize Catch Rate"));
        assert!(text.contains("Expected Group Haul"));
        assert!(text.contains("Expected Catches / Hour"));
        assert!(text.contains("calculator-loot-window"));
        assert!(text.contains("<fishy-distribution-chart"));
        assert!(text.contains("signal-path=\"_calc.fish_group_distribution_chart\""));
        assert!(text.contains("signal-path=\"_calc.loot_distribution_chart\""));
        assert!(text.contains("No source-backed species rows are available for this zone yet."));
        assert!(text.contains("data-category-key=\"buff-category:1\""));
        assert!(text.contains("Meal"));
        assert!(text.contains("value=\"effect:8-piece-outfit-set-effect\" checked"));
        assert!(text.contains("value=\"effect:mainhand-weapon-outfit\" checked"));
        assert!(text.contains("value=\"effect:awakening-weapon-outfit\" checked"));
        assert!(text.contains("value=\"item:14330\" checked"));
        assert!(text.contains("src=\"http://127.0.0.1:4040/images/items/00016162.webp\""));
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
        assert!(text.contains("event:datastar-patch-elements"));
        assert!(text.contains("data:selector #calculator-fish-group-chart"));
        assert!(text.contains("data:selector #calculator-loot-chart"));
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
        assert!(text.contains("\"auto_fish_time\":\""));
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
        assert!(text.contains("data-role=\"option-content\""));
        assert!(text.contains("data-value=\"240,74,74\""));
        assert!(text.contains("Velia Beach"));
        assert!(text.contains("Selected"));
    }

    #[tokio::test]
    async fn option_search_returns_fuzzy_item_results_with_rich_content() {
        let response = get_calculator_datastar_option_search(
            State(test_state()),
            Ok(Query(CalculatorSearchableOptionQuery {
                lang: Some("en".to_string()),
                r#ref: None,
                kind: Some("rod".to_string()),
                q: Some("baleno".to_string()),
                results_id: Some("calculator-rod-picker-results".to_string()),
                selected: Some("item:16162".to_string()),
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
        assert!(text.contains("id=\"calculator-rod-picker-results\""));
        assert!(text.contains("data-role=\"results\""));
        assert!(text.contains("data-searchable-dropdown-option"));
        assert!(text.contains("data-role=\"option-content\""));
        assert!(text.contains("item-icon"));
        assert!(text.contains("/images/items/00016162.webp"));
        assert!(text.contains("Balenos Fishing Rod"));
        assert!(text.contains("-10% AFT"));
        assert!(text.contains("Selected"));
    }

    #[tokio::test]
    async fn option_search_returns_source_backed_lightstone_translation_and_badges() {
        let response = get_calculator_datastar_option_search(
            State(test_state()),
            Ok(Query(CalculatorSearchableOptionQuery {
                lang: Some("en".to_string()),
                r#ref: None,
                kind: Some("lightstone_set".to_string()),
                q: Some("blacksmith".to_string()),
                results_id: Some("calculator-lightstone-picker-results".to_string()),
                selected: Some("lightstone-set:30".to_string()),
            })),
            Extension(RequestId("req-test".to_string())),
        )
        .await
        .unwrap()
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body()).await.unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("id=\"calculator-lightstone-picker-results\""));
        assert!(text.contains("data-searchable-dropdown-option"));
        assert!(text.contains("Blacksmith&#39;s Blessing"));
        assert!(text.contains("+30% Item DRR"));
        assert!(text.contains("Selected"));
    }

    #[tokio::test]
    async fn option_search_returns_lifeskill_level_drr_badges() {
        let response = get_calculator_datastar_option_search(
            State(test_state()),
            Ok(Query(CalculatorSearchableOptionQuery {
                lang: Some("en".to_string()),
                r#ref: None,
                kind: Some("lifeskill_level".to_string()),
                q: Some("guru".to_string()),
                results_id: Some("calculator-lifeskill-level-picker-results".to_string()),
                selected: Some("100".to_string()),
            })),
            Extension(RequestId("req-test".to_string())),
        )
        .await
        .unwrap()
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body()).await.unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("Guru 20"));
        assert!(text.contains("+60% Lv DRR"));
        assert!(text.contains("Selected"));
    }

    #[test]
    fn normalize_named_array_keeps_explicit_empty_selection() {
        let valid_keys = std::collections::HashSet::from(["item:1".to_string()]);
        let lookup = HashMap::from([(normalize_lookup_value("Item One"), "item:1".to_string())]);

        let normalized = normalize_named_array(
            &[],
            &valid_keys,
            &lookup,
            None,
            vec!["item:1".to_string()],
            None,
        );

        assert!(normalized.is_empty());
    }

    #[test]
    fn init_signals_patch_map_expands_checkbox_groups_to_option_slots() {
        let data = CalculatorData {
            catalog: CalculatorCatalogResponse {
                items: vec![
                    CalculatorItemEntry {
                        key: "effect:8-piece-outfit-set-effect".to_string(),
                        name: "8-Piece Outfit Set Effect".to_string(),
                        r#type: "outfit".to_string(),
                        ..CalculatorItemEntry::default()
                    },
                    CalculatorItemEntry {
                        key: "effect:awakening-weapon-outfit".to_string(),
                        name: "Awakening Weapon Outfit".to_string(),
                        r#type: "outfit".to_string(),
                        ..CalculatorItemEntry::default()
                    },
                    CalculatorItemEntry {
                        key: "effect:mainhand-weapon-outfit".to_string(),
                        name: "Mainhand Weapon Outfit".to_string(),
                        r#type: "outfit".to_string(),
                        ..CalculatorItemEntry::default()
                    },
                    CalculatorItemEntry {
                        key: "item:14071".to_string(),
                        name: "Professional Fisher's Uniform".to_string(),
                        r#type: "outfit".to_string(),
                        ..CalculatorItemEntry::default()
                    },
                    CalculatorItemEntry {
                        key: "item:14330".to_string(),
                        name: "Professional Fisher's Uniform (Costume)".to_string(),
                        r#type: "outfit".to_string(),
                        ..CalculatorItemEntry::default()
                    },
                ],
                lifeskill_levels: Vec::new(),
                mastery_prize_curve: Vec::new(),
                zone_group_rates: Vec::new(),
                trade_levels: Vec::new(),
                defaults: CalculatorSignals::default(),
                fishing_levels: Vec::new(),
                pets: CalculatorPetCatalog::default(),
                session_units: Vec::new(),
                session_presets: Vec::new(),
            },
            cdn_base_url: "http://127.0.0.1:4040".to_string(),
            lang: FishLang::En,
            zones: Vec::new(),
            zone_group_rates: HashMap::new(),
            zone_loot_entries: Vec::new(),
        };
        let signals = CalculatorSignals {
            outfit: vec![
                "effect:8-piece-outfit-set-effect".to_string(),
                "effect:awakening-weapon-outfit".to_string(),
                "effect:mainhand-weapon-outfit".to_string(),
                "item:14330".to_string(),
            ],
            ..CalculatorSignals::default()
        };

        let patch = init_signals_patch_map(&data, &signals).unwrap();

        assert_eq!(
            patch.get("outfit"),
            Some(&json!([
                "effect:8-piece-outfit-set-effect",
                "effect:awakening-weapon-outfit",
                "effect:mainhand-weapon-outfit",
                "",
                "item:14330"
            ]))
        );
    }

    #[test]
    fn normalize_named_array_keeps_only_last_item_per_buff_category() {
        let valid_keys = std::collections::HashSet::from([
            "item:1".to_string(),
            "item:2".to_string(),
            "item:3".to_string(),
        ]);
        let lookup = HashMap::from([
            (normalize_lookup_value("Item One"), "item:1".to_string()),
            (normalize_lookup_value("Item Two"), "item:2".to_string()),
            (normalize_lookup_value("Item Three"), "item:3".to_string()),
        ]);
        let item_one = CalculatorItemEntry {
            key: "item:1".to_string(),
            buff_category_key: Some("buff-category:1".to_string()),
            ..CalculatorItemEntry::default()
        };
        let item_two = CalculatorItemEntry {
            key: "item:2".to_string(),
            buff_category_key: Some("buff-category:1".to_string()),
            ..CalculatorItemEntry::default()
        };
        let item_three = CalculatorItemEntry {
            key: "item:3".to_string(),
            ..CalculatorItemEntry::default()
        };
        let items_by_key = HashMap::from([
            ("item:1", &item_one),
            ("item:2", &item_two),
            ("item:3", &item_three),
        ]);

        let normalized = normalize_named_array(
            &[
                "item:1".to_string(),
                "item:2".to_string(),
                "item:3".to_string(),
            ],
            &valid_keys,
            &lookup,
            None,
            Vec::new(),
            Some(&items_by_key),
        );

        assert_eq!(normalized, vec!["item:2".to_string(), "item:3".to_string()]);
    }

    #[test]
    fn normalize_named_array_prefers_higher_buff_category_level() {
        let valid_keys =
            std::collections::HashSet::from(["item:1".to_string(), "item:2".to_string()]);
        let lookup = HashMap::from([
            (normalize_lookup_value("Meal I"), "item:1".to_string()),
            (normalize_lookup_value("Meal II"), "item:2".to_string()),
        ]);
        let item_one = CalculatorItemEntry {
            key: "item:1".to_string(),
            buff_category_key: Some("buff-category:1".to_string()),
            buff_category_level: Some(0),
            ..CalculatorItemEntry::default()
        };
        let item_two = CalculatorItemEntry {
            key: "item:2".to_string(),
            buff_category_key: Some("buff-category:1".to_string()),
            buff_category_level: Some(1),
            ..CalculatorItemEntry::default()
        };
        let items_by_key = HashMap::from([("item:1", &item_one), ("item:2", &item_two)]);

        let normalized = normalize_named_array(
            &["item:2".to_string(), "item:1".to_string()],
            &valid_keys,
            &lookup,
            None,
            Vec::new(),
            Some(&items_by_key),
        );

        assert_eq!(normalized, vec!["item:2".to_string()]);
    }

    #[test]
    fn build_pet_value_aliases_includes_catalog_labels_and_keys() {
        let aliases = build_pet_value_aliases(&CalculatorPetCatalog {
            specials: vec![CalculatorOptionEntry {
                key: "auto_fishing_time_reduction".to_string(),
                label: "자동 낚시 시간 감소".to_string(),
            }],
            talents: vec![CalculatorOptionEntry {
                key: "life_exp".to_string(),
                label: "생활 경험치".to_string(),
            }],
            skills: vec![CalculatorOptionEntry {
                key: "fishing_exp".to_string(),
                label: "낚시 경험치".to_string(),
            }],
            ..CalculatorPetCatalog::default()
        });

        assert_eq!(
            aliases.get(&normalize_lookup_value("자동 낚시 시간 감소")),
            Some(&"auto_fishing_time_reduction".to_string())
        );
        assert_eq!(
            aliases.get(&normalize_lookup_value("life exp")),
            Some(&"life_exp".to_string())
        );
        assert_eq!(
            aliases.get(&normalize_lookup_value("Fishing EXP")),
            Some(&"fishing_exp".to_string())
        );
    }

    #[test]
    fn buff_category_label_uses_unique_label_for_skill_family_groups() {
        let item = CalculatorItemEntry {
            buff_category_key: Some("skill-family:59778".to_string()),
            ..CalculatorItemEntry::default()
        };

        assert_eq!(buff_category_label(&item).as_deref(), Some("Skill 59778"));
    }

    #[test]
    fn buff_category_label_uses_unique_label_for_unknown_buff_categories() {
        let item = CalculatorItemEntry {
            buff_category_key: Some("buff-category:7".to_string()),
            buff_category_level: Some(1),
            ..CalculatorItemEntry::default()
        };

        assert_eq!(buff_category_label(&item).as_deref(), Some("Category 7 II"));
    }

    #[test]
    fn parse_calculator_signals_value_coerces_top_level_string_arrays() {
        let parsed = parse_calculator_signals_value(
            serde_json::json!({
                "outfit": "effect:mainhand-weapon-outfit",
                "food": "item:9359",
                "buff": "item:721092"
            }),
            &CalculatorSignals::default(),
            &RequestId("req-test".to_string()),
        )
        .expect("top-level arrays should coerce");

        assert_eq!(
            parsed.outfit,
            vec!["effect:mainhand-weapon-outfit".to_string()]
        );
        assert_eq!(parsed.food, vec!["item:9359".to_string()]);
        assert_eq!(parsed.buff, vec!["item:721092".to_string()]);
    }

    #[test]
    fn parse_calculator_signals_value_coerces_indexed_object_arrays() {
        let parsed = parse_calculator_signals_value(
            serde_json::json!({
                "outfit": {
                    "0": "effect:8-piece-outfit-set-effect",
                    "1": "",
                    "2": "effect:awakening-weapon-outfit"
                },
                "food": {
                    "1": "item:9359",
                    "0": ""
                },
                "buff": {
                    "0": "item:721092"
                },
                "pet1": {
                    "skills": {
                        "1": "fishing_exp",
                        "0": "life_exp"
                    }
                }
            }),
            &CalculatorSignals::default(),
            &RequestId("req-test".to_string()),
        )
        .expect("indexed objects should coerce");

        assert_eq!(
            parsed.outfit,
            vec![
                "effect:8-piece-outfit-set-effect".to_string(),
                "".to_string(),
                "effect:awakening-weapon-outfit".to_string()
            ]
        );
        assert_eq!(parsed.food, vec!["".to_string(), "item:9359".to_string()]);
        assert_eq!(parsed.buff, vec!["item:721092".to_string()]);
        assert_eq!(
            parsed.pet1.skills,
            vec!["life_exp".to_string(), "fishing_exp".to_string()]
        );
    }

    #[test]
    fn parse_calculator_signals_value_normalizes_discard_grade() {
        let parsed = parse_calculator_signals_value(
            serde_json::json!({
                "discardGrade": "YELLOW"
            }),
            &CalculatorSignals::default(),
            &RequestId("req-test".to_string()),
        )
        .expect("discard grade should normalize");

        assert_eq!(parsed.discard_grade, "yellow");
    }

    #[test]
    fn parse_calculator_signals_value_normalizes_price_overrides() {
        let parsed = parse_calculator_signals_value(
            serde_json::json!({
                "priceOverrides": {
                    "item:8473": {
                        "tradePriceCurvePercent": "130",
                        "basePrice": "8800000"
                    },
                    "8476": {
                        "tradePriceCurvePercent": 115
                    },
                    "bad": {
                        "tradePriceCurvePercent": 999
                    }
                }
            }),
            &CalculatorSignals::default(),
            &RequestId("req-test".to_string()),
        )
        .expect("price overrides should normalize");

        assert_eq!(
            parsed
                .price_overrides
                .get("8473")
                .and_then(|entry| entry.trade_price_curve_percent),
            Some(130.0)
        );
        assert_eq!(
            parsed
                .price_overrides
                .get("8473")
                .and_then(|entry| entry.base_price),
            Some(8_800_000.0)
        );
        assert_eq!(
            parsed
                .price_overrides
                .get("8476")
                .and_then(|entry| entry.trade_price_curve_percent),
            Some(115.0)
        );
        assert!(!parsed.price_overrides.contains_key("bad"));
    }

    #[test]
    fn discard_grade_threshold_keeps_prize_fish() {
        let signals = CalculatorSignals {
            discard_grade: "yellow".to_string(),
            ..CalculatorSignals::default()
        };

        assert!(discard_grade_enabled(&signals, Some("Trash")));
        assert!(discard_grade_enabled(&signals, Some("General")));
        assert!(discard_grade_enabled(&signals, Some("HighQuality")));
        assert!(discard_grade_enabled(&signals, Some("Rare")));
        assert!(!discard_grade_enabled(&signals, Some("Prize")));
    }

    #[test]
    fn trade_sale_multiplier_for_species_prefers_species_override() {
        let mut signals = CalculatorSignals {
            trade_distance_bonus: 100.0,
            trade_price_curve: 120.0,
            trade_level: "73".to_string(),
            apply_trade_modifiers: true,
            ..CalculatorSignals::default()
        };
        signals.price_overrides.insert(
            "8473".to_string(),
            CalculatorPriceOverrideSignals {
                trade_price_curve_percent: Some(130.0),
                base_price: None,
            },
        );

        let default_multiplier = trade_sale_multiplier_for_species(&signals, 8476);
        let override_multiplier = trade_sale_multiplier_for_species(&signals, 8473);

        assert!(override_multiplier > default_multiplier);
    }

    #[test]
    fn mastery_prize_rate_uses_last_reached_bracket() {
        let curve = vec![
            CalculatorMasteryPrizeRateEntry {
                fishing_mastery: 0,
                high_drop_rate_raw: 0,
                high_drop_rate: 0.0,
            },
            CalculatorMasteryPrizeRateEntry {
                fishing_mastery: 50,
                high_drop_rate_raw: 1_250,
                high_drop_rate: 0.00125,
            },
            CalculatorMasteryPrizeRateEntry {
                fishing_mastery: 100,
                high_drop_rate_raw: 2_500,
                high_drop_rate: 0.0025,
            },
        ];

        assert_eq!(mastery_prize_rate_for_bracket(&curve, 0.0), 0.0);
        assert_eq!(mastery_prize_rate_for_bracket(&curve, 50.0), 0.00125);
        assert_eq!(mastery_prize_rate_for_bracket(&curve, 99.0), 0.00125);
        assert_eq!(mastery_prize_rate_for_bracket(&curve, 100.0), 0.0025);
    }

    #[test]
    fn base_price_for_species_prefers_species_override() {
        let mut signals = CalculatorSignals::default();
        signals.price_overrides.insert(
            "8473".to_string(),
            CalculatorPriceOverrideSignals {
                trade_price_curve_percent: None,
                base_price: Some(8_800_000.0),
            },
        );

        assert_eq!(
            base_price_for_species(&signals, 8473, 8_000_000.0),
            8_800_000.0
        );
        assert_eq!(
            base_price_for_species(&signals, 8476, 16_000_000.0),
            16_000_000.0
        );
    }
}
