use std::collections::HashMap;
use std::fmt::Write as _;

use axum::body::Bytes;
use axum::extract::{rejection::QueryRejection, Extension, Query, State};
use axum::http::{header, HeaderMap, HeaderValue};
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use fishystuff_api::models::calculator::{
    CalculatorCatalogResponse, CalculatorItemEntry, CalculatorLifeskillLevelEntry,
    CalculatorPetSignals, CalculatorSignals,
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

#[derive(Debug, Clone, serde::Serialize)]
struct CalculatorDerivedSignals {
    zone_name: String,
    abundance_label: String,
    zone_bite_min: String,
    zone_bite_max: String,
    effective_bite_min: String,
    effective_bite_max: String,
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
    zones: Vec<ZoneEntry>,
}

#[derive(Debug, Clone)]
struct SelectOption<'a> {
    value: &'a str,
    label: &'a str,
    icon: Option<&'a str>,
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
    let response = with_timeout(
        state.config.request_timeout_secs,
        state.store.calculator_catalog(lang, query.r#ref),
    )
    .await
    .map_err(|err| map_request_id(err, &request_id))?;

    let mut headers = HeaderMap::new();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok((headers, Json(response)))
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
            serde_json::from_str(payload).map_err(|err| {
                AppError::invalid_argument(format!("invalid datastar query payload: {err}"))
                    .with_request_id(request_id.0.clone())
            })?
        }
        _ => CalculatorSignals::default(),
    };

    let (normalized_signals, derived) = normalize_and_derive(raw_signals.clone(), &data);
    let fragments = render_calculator_controls(&data, &normalized_signals);
    let mut patch = serde_json::Map::new();
    if normalized_signals != raw_signals {
        let value = serde_json::to_value(&normalized_signals).map_err(|err| {
            AppError::internal(format!("serialize normalized calculator signals: {err}"))
        })?;
        if let Value::Object(obj) = value {
            patch.extend(obj);
        }
    }
    patch.insert(
        "_calc".to_string(),
        serde_json::to_value(&derived).map_err(|err| {
            AppError::internal(format!("serialize calculator derived signals: {err}"))
        })?,
    );
    let sse = datastar_sse_response(Some(fragments), Some(Value::Object(patch)))?;
    Ok(datastar_response(sse))
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
    let raw_signals = parse_calculator_signals_body(&body, &request_id)?;
    let (normalized_signals, derived) = normalize_and_derive(raw_signals.clone(), &data);

    let mut patch = serde_json::Map::new();
    if normalized_signals != raw_signals {
        let value = serde_json::to_value(&normalized_signals).map_err(|err| {
            AppError::internal(format!("serialize normalized calculator signals: {err}"))
        })?;
        if let Value::Object(obj) = value {
            patch.extend(obj);
        }
    }
    patch.insert(
        "_calc".to_string(),
        serde_json::to_value(&derived).map_err(|err| {
            AppError::internal(format!("serialize calculator derived signals: {err}"))
        })?,
    );

    let sse = datastar_sse_response(None, Some(Value::Object(patch)))?;
    Ok(datastar_response(sse))
}

fn datastar_response(body: String) -> (HeaderMap, String) {
    let mut headers = HeaderMap::new();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/event-stream"),
    );
    (headers, body)
}

fn datastar_sse_response(fragments: Option<String>, signals: Option<Value>) -> AppResult<String> {
    let mut body = String::new();
    if let Some(fragments) = fragments {
        write!(
            body,
            "event: datastar-merge-fragments\ndata: fragments {}\n\n",
            fragments
        )
        .map_err(|err| AppError::internal(format!("write datastar fragments payload: {err}")))?;
    }
    if let Some(signals) = signals {
        let serialized = serde_json::to_string(&signals).map_err(|err| {
            AppError::internal(format!("serialize datastar signals payload: {err}"))
        })?;
        write!(
            body,
            "event: datastar-merge-signals\ndata: signals {}\n\n",
            serialized
        )
        .map_err(|err| AppError::internal(format!("write datastar signals payload: {err}")))?;
    }
    Ok(body)
}

fn parse_calculator_signals_body(
    body: &Bytes,
    request_id: &RequestId,
) -> AppResult<CalculatorSignals> {
    if body.is_empty() {
        return Ok(CalculatorSignals::default());
    }
    serde_json::from_slice(body).map_err(|err| {
        AppError::invalid_argument(format!("invalid calculator request body: {err}"))
            .with_request_id(request_id.0.clone())
    })
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
    Ok(CalculatorData { catalog, zones })
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
    let defaults = CalculatorSignals::default();
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

    signals.zone = normalize_named_value(
        &signals.zone,
        &valid_zone_keys,
        &zone_name_to_key,
        None,
        defaults.zone.clone(),
        false,
    );
    signals.lifeskill_level = normalize_named_value(
        &signals.lifeskill_level,
        &valid_level_keys,
        &level_name_to_key,
        None,
        defaults.lifeskill_level.clone(),
        false,
    );
    signals.rod = normalize_named_value(
        &signals.rod,
        &valid_item_keys,
        &item_name_to_key,
        Some(&item_legacy_aliases),
        defaults.rod.clone(),
        false,
    );
    signals.float = normalize_named_value(
        &signals.float,
        &valid_item_keys,
        &item_name_to_key,
        Some(&item_legacy_aliases),
        String::new(),
        true,
    );
    signals.chair = normalize_named_value(
        &signals.chair,
        &valid_item_keys,
        &item_name_to_key,
        Some(&item_legacy_aliases),
        defaults.chair.clone(),
        true,
    );
    signals.lightstone_set = normalize_named_value(
        &signals.lightstone_set,
        &valid_item_keys,
        &item_name_to_key,
        Some(&item_legacy_aliases),
        defaults.lightstone_set.clone(),
        true,
    );
    signals.backpack = normalize_named_value(
        &signals.backpack,
        &valid_item_keys,
        &item_name_to_key,
        Some(&item_legacy_aliases),
        defaults.backpack.clone(),
        true,
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
    if allow_empty {
        String::new()
    } else {
        default_value
    }
}

fn normalize_named_array(
    values: &[String],
    valid_keys: &std::collections::HashSet<String>,
    lookup: &HashMap<String, String>,
    aliases: Option<&HashMap<String, String>>,
    default_values: Vec<String>,
) -> Vec<String> {
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
    let auto_fish_time_raw = if signals.active {
        0.0
    } else {
        (180.0 * (1.0 - afr_raw)).max(60.0)
    };

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
        effective_bite_min: fmt2(effective_bite_min_raw),
        effective_bite_max: fmt2(effective_bite_max_raw),
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

fn render_calculator_controls(data: &CalculatorData, signals: &CalculatorSignals) -> String {
    let mut fragments = String::new();

    let zones = sorted_zone_options(&data.zones);
    fragments.push_str(&render_select(
        "zone",
        "select w-full",
        "data-bind-zone",
        &signals.zone,
        &zones,
        false,
    ));

    let lifeskill_levels = sorted_lifeskill_options(&data.catalog.lifeskill_levels);
    fragments.push_str(&render_select(
        "lifeskill_level",
        "select w-full",
        "data-bind-lifeskill_level",
        &signals.lifeskill_level,
        &lifeskill_levels,
        false,
    ));

    let rods = item_options_by_type(&data.catalog.items, "rod");
    fragments.push_str(&render_select(
        "rods",
        "select w-full",
        "data-bind-rod",
        &signals.rod,
        &rods,
        false,
    ));

    let floats = item_options_by_type(&data.catalog.items, "float");
    fragments.push_str(&render_select(
        "floats",
        "select w-full",
        "data-bind-float",
        &signals.float,
        &floats,
        true,
    ));

    let chairs = item_options_by_type(&data.catalog.items, "chair");
    fragments.push_str(&render_select(
        "chairs",
        "select w-full",
        "data-bind-chair",
        &signals.chair,
        &chairs,
        true,
    ));

    let lightstone_sets = item_options_by_type(&data.catalog.items, "lightstone_set");
    fragments.push_str(&render_select(
        "lightstone_sets",
        "select w-full",
        "data-bind-lightstone_set",
        &signals.lightstone_set,
        &lightstone_sets,
        true,
    ));

    let backpacks = item_options_by_type(&data.catalog.items, "backpack");
    fragments.push_str(&render_select(
        "backpacks",
        "select w-full",
        "data-bind-backpack",
        &signals.backpack,
        &backpacks,
        true,
    ));

    let outfits = item_options_by_type(&data.catalog.items, "outfit");
    fragments.push_str(&render_checkbox_group(
        "outfits",
        "outfit",
        &signals.outfit,
        &outfits,
    ));

    let foods = item_options_by_type(&data.catalog.items, "food");
    fragments.push_str(&render_checkbox_group(
        "foods",
        "food",
        &signals.food,
        &foods,
    ));

    let buffs = item_options_by_type(&data.catalog.items, "buff");
    fragments.push_str(&render_checkbox_group(
        "buffs",
        "buff",
        &signals.buff,
        &buffs,
    ));

    fragments
}

fn sorted_zone_options(zones: &[ZoneEntry]) -> Vec<SelectOption<'_>> {
    let mut zones = zones
        .iter()
        .filter(|zone| zone.bite_time_min.is_some() && zone.bite_time_max.is_some())
        .collect::<Vec<_>>();
    zones.sort_by(|a, b| a.name.cmp(&b.name));
    zones
        .into_iter()
        .map(|zone| SelectOption {
            value: zone.rgb_key.0.as_str(),
            label: zone.name.as_deref().unwrap_or(zone.rgb_key.0.as_str()),
            icon: None,
        })
        .collect()
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
) -> String {
    let mut html = String::new();
    write!(
        html,
        "<select id=\"{}\" class=\"{}\" {}><button class=\"w-full justify-between\"><div><selectedcontent></selectedcontent></div></button>",
        escape_html(id),
        escape_html(class_name),
        bind_attr,
    )
    .unwrap();
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
) -> String {
    let selected = selected_values
        .iter()
        .map(|value| value.as_str())
        .collect::<std::collections::HashSet<_>>();
    let mut html = String::new();
    write!(
        html,
        "<div id=\"{}\" class=\"grid gap-2 sm:grid-cols-2\">",
        escape_html(id)
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
            "<label class=\"label cursor-pointer justify-start gap-3 rounded-box border border-base-300 bg-base-100 px-3 py-2 text-sm font-medium\"><input data-bind-{} type=\"checkbox\" class=\"checkbox checkbox-primary checkbox-sm shrink-0\" value=\"{}\"{}>",
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
    use std::sync::Arc;

    use async_trait::async_trait;
    use axum::body::Bytes;
    use axum::extract::{Extension, Query, State};
    use axum::http::{header, StatusCode};
    use axum::response::IntoResponse;
    use fishystuff_api::ids::{Rgb, RgbKey};
    use fishystuff_api::models::calculator::{
        CalculatorCatalogResponse, CalculatorItemEntry, CalculatorLifeskillLevelEntry,
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
        get_calculator_datastar_init, post_calculator_datastar_eval, CalculatorDatastarQuery,
        CalculatorQuery,
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
                        name: "Karki Suit".to_string(),
                        r#type: "chair".to_string(),
                        afr: Some(0.1),
                        drr: Some(0.1),
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
                        name: "Sute Tea".to_string(),
                        r#type: "food".to_string(),
                        afr: Some(0.05),
                        ..CalculatorItemEntry::default()
                    },
                    CalculatorItemEntry {
                        key: "item:721092".to_string(),
                        name: "Verdure Draught".to_string(),
                        r#type: "buff".to_string(),
                        afr: Some(0.05),
                        drr: Some(0.05),
                        ..CalculatorItemEntry::default()
                    },
                ],
                lifeskill_levels: vec![CalculatorLifeskillLevelEntry {
                    key: "100".to_string(),
                    name: "Guru 20".to_string(),
                    index: 100,
                    order: 100,
                }],
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
    async fn init_returns_datastar_fragments_and_calc_signals() {
        let response = get_calculator_datastar_init(
            State(test_state()),
            Ok(Query(CalculatorDatastarQuery {
                lang: Some("en".to_string()),
                r#ref: None,
                datastar: None,
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
        assert!(text.contains("event: datastar-merge-fragments"));
        assert!(text.contains("<select id=\"zone\""));
        assert!(text.contains("event: datastar-merge-signals"));
        assert!(text.contains("\"zone_name\":\"Velia Beach\""));
    }

    #[tokio::test]
    async fn eval_normalizes_legacy_values_and_returns_calc_signals() {
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

        let body = to_bytes(response.into_body()).await.unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("event: datastar-merge-signals"));
        assert!(text.contains("\"zone\":\"240,74,74\""));
        assert!(text.contains("\"rod\":\"item:16162\""));
        assert!(text.contains("\"special\":\"auto_fishing_time_reduction\""));
        assert!(text.contains("\"zone_name\":\"Velia Beach\""));
    }
}
