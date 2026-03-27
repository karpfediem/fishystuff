use async_stream::stream;
use axum::body::Bytes;
use axum::extract::{rejection::QueryRejection, Extension, Query, State};
use axum::http::{header, HeaderMap, HeaderValue};
use axum::response::{sse::Event, IntoResponse, Sse};
use axum::Json;
use datastar::prelude::{DatastarEvent, ElementPatchMode, PatchElements, PatchSignals};
use serde::Deserialize;
use serde_json::{json, Value};
use std::cmp::Ordering;
use std::convert::Infallible;
use std::fmt::Write as _;

use fishystuff_api::models::fish::{FishEntry, FishListResponse};

use crate::error::{with_timeout, AppError, AppResult};
use crate::routes::meta::map_request_id;
use crate::state::{RequestId, SharedState};
use crate::store::FishLang;

const GRADE_FILTER_COLOR_ORDER: [&str; 5] = ["red", "yellow", "blue", "green", "white"];
const GRADE_COLOR_ORDER: [&str; 6] = ["red", "yellow", "blue", "green", "white", "unknown"];
const METHOD_ORDER: [&str; 2] = ["rod", "harpoon"];
const ICON_SPRITE_URL: &str = "/img/icons.svg";
const DETAILS_SPOTS_NOTE: &str =
    "Planned input: evidence locations mapped back to fishing zones, then ranked by rarity and bite-time behavior in each zone.";

#[derive(Debug, Deserialize)]
pub struct FishQuery {
    pub lang: Option<String>,
    pub r#ref: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FishDatastarQuery {
    pub lang: Option<String>,
    pub r#ref: Option<String>,
    pub datastar: Option<String>,
}

#[derive(Debug, Clone)]
struct FishDexSignals {
    search_query: String,
    caught_filter: String,
    favourite_filter: bool,
    grade_filters: Vec<String>,
    method_filters: Vec<String>,
    show_dried: bool,
    sort_field: String,
    sort_direction: String,
    caught_ids: Vec<i32>,
    favourite_ids: Vec<i32>,
    selected_fish_id: Option<i32>,
    revision: String,
    total_count: usize,
    catalog_count: usize,
    visible_count: usize,
    caught_count: usize,
    red_total_count: usize,
    red_caught_count: usize,
    yellow_total_count: usize,
    yellow_caught_count: usize,
    blue_total_count: usize,
    blue_caught_count: usize,
    green_total_count: usize,
    green_caught_count: usize,
    white_total_count: usize,
    white_caught_count: usize,
    supports_grade_filter: bool,
    supports_method_filter: bool,
    supports_dried_filter: bool,
}

impl Default for FishDexSignals {
    fn default() -> Self {
        Self {
            search_query: String::new(),
            caught_filter: "all".to_string(),
            favourite_filter: false,
            grade_filters: Vec::new(),
            method_filters: Vec::new(),
            show_dried: false,
            sort_field: "price".to_string(),
            sort_direction: "desc".to_string(),
            caught_ids: Vec::new(),
            favourite_ids: Vec::new(),
            selected_fish_id: None,
            revision: String::new(),
            total_count: 0,
            catalog_count: 0,
            visible_count: 0,
            caught_count: 0,
            red_total_count: 0,
            red_caught_count: 0,
            yellow_total_count: 0,
            yellow_caught_count: 0,
            blue_total_count: 0,
            blue_caught_count: 0,
            green_total_count: 0,
            green_caught_count: 0,
            white_total_count: 0,
            white_caught_count: 0,
            supports_grade_filter: false,
            supports_method_filter: false,
            supports_dried_filter: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct GradeProgress {
    total: usize,
    caught: usize,
}

#[derive(Debug, Clone)]
struct FishDexDerived {
    filtered: Vec<FishEntry>,
    selected_fish: Option<FishEntry>,
}

pub async fn list_fish(
    State(state): State<SharedState>,
    query: Result<Query<FishQuery>, QueryRejection>,
    Extension(request_id): Extension<RequestId>,
) -> AppResult<(HeaderMap, Json<FishListResponse>)> {
    let Query(query) = query.map_err(|err| {
        AppError::invalid_argument(err.to_string()).with_request_id(request_id.0.clone())
    })?;

    let lang = FishLang::from_param(query.lang.as_deref());
    let response = load_fish_response(&state, lang, query.r#ref, &request_id).await?;

    let mut response_headers = HeaderMap::new();
    response_headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok((response_headers, Json(response)))
}

pub async fn get_fish_datastar_init(
    State(state): State<SharedState>,
    query: Result<Query<FishDatastarQuery>, QueryRejection>,
    Extension(request_id): Extension<RequestId>,
) -> AppResult<impl IntoResponse> {
    let Query(query) = query.map_err(|err| {
        AppError::invalid_argument(err.to_string()).with_request_id(request_id.0.clone())
    })?;

    let lang = FishLang::from_param(query.lang.as_deref());
    let response = load_fish_response(&state, lang, query.r#ref, &request_id).await?;
    let raw_signals = match query.datastar.as_deref() {
        Some(payload) if !payload.trim().is_empty() => {
            let value = serde_json::from_str::<Value>(payload).map_err(|err| {
                AppError::invalid_argument(format!("invalid fish datastar query payload: {err}"))
                    .with_request_id(request_id.0.clone())
            })?;
            parse_fish_dex_signals_value(value, &request_id)?
        }
        _ => FishDexSignals::default(),
    };

    fish_dex_datastar_response(response, raw_signals)
}

pub async fn post_fish_datastar_eval(
    State(state): State<SharedState>,
    query: Result<Query<FishQuery>, QueryRejection>,
    Extension(request_id): Extension<RequestId>,
    body: Bytes,
) -> AppResult<impl IntoResponse> {
    let Query(query) = query.map_err(|err| {
        AppError::invalid_argument(err.to_string()).with_request_id(request_id.0.clone())
    })?;

    let lang = FishLang::from_param(query.lang.as_deref());
    let response = load_fish_response(&state, lang, query.r#ref, &request_id).await?;
    let raw_signals = parse_fish_dex_signals_body(&body, &request_id)?;

    fish_dex_datastar_response(response, raw_signals)
}

async fn load_fish_response(
    state: &SharedState,
    lang: FishLang,
    ref_id: Option<String>,
    request_id: &RequestId,
) -> AppResult<FishListResponse> {
    with_timeout(
        state.config.request_timeout_secs,
        state.store.list_fish(lang, ref_id),
    )
    .await
    .map_err(|err| map_request_id(err, request_id))
}

fn fish_dex_datastar_response(
    response: FishListResponse,
    raw_signals: FishDexSignals,
) -> AppResult<impl IntoResponse> {
    let (signals, derived) = normalize_and_derive(raw_signals, &response);
    let events = vec![
        fish_dex_signals_event(&signals)?.into_datastar_event(),
        PatchElements::new(render_fishydex_grid(&signals, &derived))
            .selector("#fishydex-grid")
            .mode(ElementPatchMode::Outer)
            .into_datastar_event(),
        PatchElements::new(render_fishydex_details_shell(&signals, &derived))
            .selector("#fishydex-details-shell")
            .mode(ElementPatchMode::Outer)
            .into_datastar_event(),
    ];
    Ok(datastar_response(events))
}

fn datastar_response(events: Vec<DatastarEvent>) -> impl IntoResponse {
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

fn parse_fish_dex_signals_body(body: &Bytes, request_id: &RequestId) -> AppResult<FishDexSignals> {
    if body.is_empty() {
        return Ok(FishDexSignals::default());
    }
    let value = serde_json::from_slice::<Value>(body).map_err(|err| {
        AppError::invalid_argument(format!("invalid fish datastar request body: {err}"))
            .with_request_id(request_id.0.clone())
    })?;
    parse_fish_dex_signals_value(value, request_id)
}

fn parse_fish_dex_signals_value(value: Value, request_id: &RequestId) -> AppResult<FishDexSignals> {
    let object = match value {
        Value::Object(object) => object,
        _ => {
            return Err(
                AppError::invalid_argument("fish datastar payload must be a JSON object")
                    .with_request_id(request_id.0.clone()),
            );
        }
    };

    let search_query = object
        .get("search_query")
        .or_else(|| object.get("search_draft"));

    Ok(FishDexSignals {
        search_query: normalize_string_value(search_query).trim().to_string(),
        caught_filter: normalize_caught_filter(object.get("caught_filter")),
        favourite_filter: normalize_bool_value(object.get("favourite_filter")),
        grade_filters: normalize_grade_filters(object.get("grade_filters")),
        method_filters: normalize_method_filters(object.get("method_filters")),
        show_dried: normalize_bool_value(object.get("show_dried")),
        sort_field: normalize_sort_field(object.get("sort_field")),
        sort_direction: normalize_sort_direction(object.get("sort_direction")),
        caught_ids: normalize_id_list(object.get("caught_ids")),
        favourite_ids: normalize_id_list(object.get("favourite_ids")),
        selected_fish_id: normalize_optional_i32(object.get("selected_fish_id")),
        ..FishDexSignals::default()
    })
}

fn fish_dex_signals_event(signals: &FishDexSignals) -> AppResult<PatchSignals> {
    let patch = json!({
        "search_query": signals.search_query,
        "caught_filter": signals.caught_filter,
        "favourite_filter": signals.favourite_filter,
        "grade_filters": signals.grade_filters,
        "method_filters": signals.method_filters,
        "show_dried": signals.show_dried,
        "sort_field": signals.sort_field,
        "sort_direction": signals.sort_direction,
        "caught_ids": signals.caught_ids,
        "favourite_ids": signals.favourite_ids,
        "revision": signals.revision,
        "total_count": signals.total_count,
        "catalog_count": signals.catalog_count,
        "visible_count": signals.visible_count,
        "caught_count": signals.caught_count,
        "red_total_count": signals.red_total_count,
        "red_caught_count": signals.red_caught_count,
        "yellow_total_count": signals.yellow_total_count,
        "yellow_caught_count": signals.yellow_caught_count,
        "blue_total_count": signals.blue_total_count,
        "blue_caught_count": signals.blue_caught_count,
        "green_total_count": signals.green_total_count,
        "green_caught_count": signals.green_caught_count,
        "white_total_count": signals.white_total_count,
        "white_caught_count": signals.white_caught_count,
        "supports_grade_filter": signals.supports_grade_filter,
        "supports_method_filter": signals.supports_method_filter,
        "supports_dried_filter": signals.supports_dried_filter,
    });
    let serialized = serde_json::to_string(&patch)
        .map_err(|err| AppError::internal(format!("serialize fish datastar signals: {err}")))?;
    Ok(PatchSignals::new(serialized))
}

fn normalize_and_derive(
    mut signals: FishDexSignals,
    response: &FishListResponse,
) -> (FishDexSignals, FishDexDerived) {
    let search_query = signals.search_query.trim().to_ascii_lowercase();
    let selected_fish_id = signals.selected_fish_id;
    let caught_ids = signals.caught_ids.clone();

    let catalog_entries = response
        .fish
        .iter()
        .filter(|entry| signals.show_dried || !entry_is_dried(entry))
        .cloned()
        .collect::<Vec<_>>();
    let filtered = response
        .fish
        .iter()
        .filter(|entry| entry_matches_filters(entry, &signals, &search_query))
        .cloned()
        .collect::<Vec<_>>();

    let mut sorted = filtered;
    sorted.sort_by(|left, right| compare_fish_entries(left, right, &signals));

    let red = grade_progress_for("red", &catalog_entries, &caught_ids);
    let yellow = grade_progress_for("yellow", &catalog_entries, &caught_ids);
    let blue = grade_progress_for("blue", &catalog_entries, &caught_ids);
    let green = grade_progress_for("green", &catalog_entries, &caught_ids);
    let white = grade_progress_for("white", &catalog_entries, &caught_ids);

    signals.revision = response.revision.clone();
    signals.total_count = response.fish.len();
    signals.catalog_count = catalog_entries.len();
    signals.visible_count = sorted.len();
    signals.caught_count = catalog_entries
        .iter()
        .filter(|entry| caught_ids.contains(&entry.item_id))
        .count();
    signals.red_total_count = red.total;
    signals.red_caught_count = red.caught;
    signals.yellow_total_count = yellow.total;
    signals.yellow_caught_count = yellow.caught;
    signals.blue_total_count = blue.total;
    signals.blue_caught_count = blue.caught;
    signals.green_total_count = green.total;
    signals.green_caught_count = green.caught;
    signals.white_total_count = white.total;
    signals.white_caught_count = white.caught;
    signals.supports_grade_filter = response
        .fish
        .iter()
        .any(|entry| entry.is_prize.is_some() || entry.grade.is_some());
    signals.supports_method_filter = !response.fish.is_empty();
    signals.supports_dried_filter = response.fish.iter().any(entry_is_dried);

    let selected_fish = selected_fish_id.and_then(|fish_id| {
        response
            .fish
            .iter()
            .find(|entry| entry.item_id == fish_id)
            .cloned()
    });

    (
        signals,
        FishDexDerived {
            filtered: sorted,
            selected_fish,
        },
    )
}

fn grade_progress_for(grade: &str, entries: &[FishEntry], caught_ids: &[i32]) -> GradeProgress {
    let mut progress = GradeProgress::default();
    for entry in entries {
        if filter_grade_for_entry(entry) != grade {
            continue;
        }
        progress.total += 1;
        if caught_ids.contains(&entry.item_id) {
            progress.caught += 1;
        }
    }
    progress
}

fn entry_matches_filters(entry: &FishEntry, signals: &FishDexSignals, search_query: &str) -> bool {
    if !signals.show_dried && entry_is_dried(entry) {
        return false;
    }

    if !search_query.is_empty() {
        let haystack = format!("{} {}", entry.item_id, entry.name)
            .trim()
            .to_ascii_lowercase();
        if !haystack.contains(search_query) {
            return false;
        }
    }

    if signals.caught_filter == "caught" && !signals.caught_ids.contains(&entry.item_id) {
        return false;
    }
    if signals.caught_filter == "missing" && signals.caught_ids.contains(&entry.item_id) {
        return false;
    }
    if signals.favourite_filter && !signals.favourite_ids.contains(&entry.item_id) {
        return false;
    }
    if !signals.grade_filters.is_empty()
        && !signals
            .grade_filters
            .iter()
            .any(|grade| grade == filter_grade_for_entry(entry))
    {
        return false;
    }

    let entry_methods = entry_catch_methods(entry);
    if !signals.method_filters.is_empty()
        && !signals
            .method_filters
            .iter()
            .all(|method| entry_methods.iter().any(|candidate| candidate == method))
    {
        return false;
    }

    true
}

fn compare_fish_entries(left: &FishEntry, right: &FishEntry, signals: &FishDexSignals) -> Ordering {
    if signals.sort_field == "price" {
        let left_price = entry_vendor_price(left);
        let right_price = entry_vendor_price(right);
        if left_price.is_none() && right_price.is_some() {
            return Ordering::Greater;
        }
        if left_price.is_some() && right_price.is_none() {
            return Ordering::Less;
        }
        if let (Some(left_price), Some(right_price)) = (left_price, right_price) {
            if left_price != right_price {
                return if signals.sort_direction == "desc" {
                    right_price.cmp(&left_price)
                } else {
                    left_price.cmp(&right_price)
                };
            }
        }
    }

    let left_name = left.name.to_ascii_lowercase();
    let right_name = right.name.to_ascii_lowercase();
    let name_order = left_name.cmp(&right_name);
    if name_order != Ordering::Equal {
        return if signals.sort_field == "name" && signals.sort_direction == "desc" {
            name_order.reverse()
        } else {
            name_order
        };
    }

    left.item_id.cmp(&right.item_id)
}

fn normalize_string_value(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(string)) => string.clone(),
        Some(Value::Number(number)) => number.to_string(),
        Some(Value::Bool(boolean)) => boolean.to_string(),
        _ => String::new(),
    }
}

fn normalize_bool_value(value: Option<&Value>) -> bool {
    match value {
        Some(Value::Bool(boolean)) => *boolean,
        Some(Value::String(string)) => {
            matches!(string.trim().to_ascii_lowercase().as_str(), "true" | "1")
        }
        Some(Value::Number(number)) => number.as_i64().unwrap_or_default() != 0,
        _ => false,
    }
}

fn normalize_optional_i32(value: Option<&Value>) -> Option<i32> {
    match value {
        Some(Value::Number(number)) => number.as_i64().and_then(|value| i32::try_from(value).ok()),
        Some(Value::String(string)) => string.trim().parse::<i32>().ok(),
        _ => None,
    }
}

fn normalize_id_list(value: Option<&Value>) -> Vec<i32> {
    let mut ids = Vec::new();
    match value {
        Some(Value::Array(values)) => {
            for value in values {
                if let Some(fish_id) = normalize_optional_i32(Some(value)) {
                    if fish_id > 0 && !ids.contains(&fish_id) {
                        ids.push(fish_id);
                    }
                }
            }
        }
        Some(Value::Object(values)) => {
            for (key, enabled) in values {
                if !normalize_bool_value(Some(enabled)) {
                    continue;
                }
                if let Ok(fish_id) = key.parse::<i32>() {
                    if fish_id > 0 && !ids.contains(&fish_id) {
                        ids.push(fish_id);
                    }
                }
            }
        }
        _ => {}
    }
    ids.sort_unstable();
    ids
}

fn normalize_caught_filter(value: Option<&Value>) -> String {
    match normalize_string_value(value)
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "caught" => "caught".to_string(),
        "missing" => "missing".to_string(),
        _ => "all".to_string(),
    }
}

fn normalize_sort_field(value: Option<&Value>) -> String {
    match normalize_string_value(value)
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "name" => "name".to_string(),
        _ => "price".to_string(),
    }
}

fn normalize_sort_direction(value: Option<&Value>) -> String {
    match normalize_string_value(value)
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "asc" => "asc".to_string(),
        _ => "desc".to_string(),
    }
}

fn normalize_method_filters(value: Option<&Value>) -> Vec<String> {
    let raw = match value {
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| normalize_string_value(Some(value)))
            .collect::<Vec<_>>(),
        Some(value) => vec![normalize_string_value(Some(value))],
        None => Vec::new(),
    };
    let mut methods = Vec::new();
    for value in raw {
        let method = match value.trim().to_ascii_lowercase().as_str() {
            "harpoon" => Some("harpoon"),
            "rod" => Some("rod"),
            _ => None,
        };
        if let Some(method) = method {
            if !methods.iter().any(|candidate| candidate == method) {
                methods.push(method.to_string());
            }
        }
    }
    METHOD_ORDER
        .iter()
        .filter(|method| methods.iter().any(|candidate| candidate == **method))
        .map(|method| (*method).to_string())
        .collect()
}

fn normalize_grade_filters(value: Option<&Value>) -> Vec<String> {
    let raw = match value {
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| normalize_string_value(Some(value)))
            .collect::<Vec<_>>(),
        Some(value) => vec![normalize_string_value(Some(value))],
        None => Vec::new(),
    };
    let mut filters = Vec::new();
    for value in raw {
        let normalized = normalize_grade_key(&value);
        if GRADE_FILTER_COLOR_ORDER.contains(&normalized.as_str())
            && !filters.iter().any(|candidate| candidate == &normalized)
        {
            filters.push(normalized);
        }
    }
    GRADE_FILTER_COLOR_ORDER
        .iter()
        .filter(|grade| filters.iter().any(|candidate| candidate == **grade))
        .map(|grade| (*grade).to_string())
        .collect()
}

fn normalize_grade_key(value: &str) -> String {
    match value.trim() {
        "Prize" | "red" => "red".to_string(),
        "Rare" | "yellow" => "yellow".to_string(),
        "HighQuality" | "blue" => "blue".to_string(),
        "General" | "green" => "green".to_string(),
        "Trash" | "white" => "white".to_string(),
        _ => "unknown".to_string(),
    }
}

fn filter_grade_for_entry(entry: &FishEntry) -> &'static str {
    if entry.is_prize == Some(true) || entry.grade.as_deref() == Some("Prize") {
        return "red";
    }
    match entry.grade.as_deref() {
        Some("Rare") => "yellow",
        Some("HighQuality") => "blue",
        Some("General") => "green",
        Some("Trash") => "white",
        _ => "unknown",
    }
}

fn grade_label_for_key(value: &str) -> &'static str {
    match value {
        "red" => "Red",
        "yellow" => "Yellow",
        "blue" => "Blue",
        "green" => "Green",
        "white" => "White",
        _ => "Unknown",
    }
}

fn entry_catch_methods(entry: &FishEntry) -> Vec<String> {
    let mut methods = Vec::new();
    for value in &entry.catch_methods {
        let method = match value.trim().to_ascii_lowercase().as_str() {
            "harpoon" => Some("harpoon"),
            "rod" => Some("rod"),
            _ => None,
        };
        if let Some(method) = method {
            if !methods.iter().any(|candidate| candidate == method) {
                methods.push(method.to_string());
            }
        }
    }
    if methods.is_empty() {
        methods.push("rod".to_string());
    }
    methods
}

fn entry_is_dried(entry: &FishEntry) -> bool {
    entry.is_dried
}

fn entry_vendor_price(entry: &FishEntry) -> Option<i64> {
    entry.vendor_price.filter(|amount| *amount > 0)
}

fn render_fishydex_grid(signals: &FishDexSignals, derived: &FishDexDerived) -> String {
    let mut html = String::from(r#"<div id="fishydex-grid" class="fishydex-groups">"#);

    for grade in GRADE_COLOR_ORDER {
        let group = derived
            .filtered
            .iter()
            .filter(|entry| filter_grade_for_entry(entry) == grade)
            .collect::<Vec<_>>();
        if group.is_empty() {
            continue;
        }

        write!(
            html,
            r#"<fieldset class="fishydex-group card card-border bg-base-100"><legend class="fishydex-group-title fieldset-legend ml-6 px-2">{}</legend><div class="card-body pt-0"><div class="fishydex-group-header"><span class="fishydex-group-count badge badge-ghost">{} fish</span></div><div class="fishydex-card-grid">"#,
            escape_html(grade_label_for_key(grade)),
            group.len()
        )
        .unwrap();

        for entry in group {
            html.push_str(&render_fish_card(signals, entry));
        }

        html.push_str("</div></div></fieldset>");
    }

    if derived.filtered.is_empty() {
        let has_active_filters = !signals.search_query.is_empty()
            || signals.caught_filter != "all"
            || signals.favourite_filter
            || !signals.grade_filters.is_empty()
            || !signals.method_filters.is_empty();
        let detail = if has_active_filters {
            "Try a broader search or clear some filters."
        } else {
            "The fish catalog is empty."
        };
        write!(
            html,
            r#"<div class="fishydex-empty card card-dash bg-base-100"><div class="card-body items-center"><h3 class="fishydex-empty-title">No fish match this filter.</h3><p class="fishydex-subtle">{}</p></div></div>"#,
            escape_html(detail)
        )
        .unwrap();
    }

    html.push_str("</div>");
    html
}

fn render_fish_card(signals: &FishDexSignals, entry: &FishEntry) -> String {
    let item_id = entry.item_id;
    let fish_name = if entry.name.trim().is_empty() {
        format!("Fish {item_id}")
    } else {
        entry.name.clone()
    };
    let is_caught = signals.caught_ids.contains(&item_id);
    let is_favourite = signals.favourite_ids.contains(&item_id);
    let grade = filter_grade_for_entry(entry);
    let open_expr = format!(
        "window.Fishydex.clearRequestUi(); $selected_fish_id = {item_id}; @post(window.Fishydex.datastarEvalUrl())"
    );
    let caught_expr = format!(
        "evt.stopPropagation(); window.Fishydex.rememberViewport({item_id}, evt); $caught_ids = window.Fishydex.toggleFishIds($caught_ids, {item_id}); window.Fishydex.persistCaughtIds($caught_ids); window.Fishydex.queueStamp('_caught_stamp_fish_id', $caught_ids.includes({item_id}) ? {item_id} : null); window.Fishydex.clearRequestUi(); @post(window.Fishydex.datastarEvalUrl())"
    );
    let favourite_expr = format!(
        "evt.stopPropagation(); window.Fishydex.rememberViewport({item_id}, evt); $favourite_ids = window.Fishydex.toggleFishIds($favourite_ids, {item_id}); window.Fishydex.persistFavouriteIds($favourite_ids); window.Fishydex.queueStamp('_favourite_stamp_fish_id', $favourite_ids.includes({item_id}) ? {item_id} : null); window.Fishydex.clearRequestUi(); @post(window.Fishydex.datastarEvalUrl())"
    );

    let mut html = String::new();
    write!(
        html,
        r#"<article class="fishydex-card card card-border bg-base-100{caught_class}" data-fish-id="{item_id}"><button type="button" class="fishydex-card-open" data-action="open-details" aria-haspopup="dialog" aria-label="Open details for {fish_name}" data-indicator:_details_loading data-on:click="{open_expr}"></button><div class="fishydex-card-content card-body"><div class="fishydex-card-top"><div class="fishydex-card-actions"><button type="button" class="fishydex-favourite-button btn btn-sm btn-circle btn-ghost{favourite_class}" data-action="toggle-favourite" aria-pressed="{favourite_pressed}" aria-label="{favourite_label}" data-class:is-stamping="$_favourite_stamp_fish_id === {item_id}" data-on:click="{favourite_expr}">{favourite_icon}</button><button type="button" class="fishydex-caught-button btn btn-sm btn-circle btn-ghost{caught_class}" data-action="toggle-caught" aria-pressed="{caught_pressed}" aria-label="{caught_label}" data-class:is-stamping="$_caught_stamp_fish_id === {item_id}" data-on:click="{caught_expr}">{caught_icon}</button></div></div><div class="fishydex-card-main"><div class="fishydex-icon-wrap grade-{grade}" data-fish-item-icon="{item_id}" data-fish-icon-alt="{icon_alt}" data-init="window.Fishydex.hydrateItemIcon(el)"><img class="fishydex-icon" hidden><div class="fishydex-placeholder">?</div></div><div class="fishydex-name">{fish_name}</div>{vendor_price}</div></div></article>"#,
        caught_class = if is_caught { " is-caught" } else { "" },
        item_id = item_id,
        fish_name = escape_html(&fish_name),
        open_expr = escape_html(&open_expr),
        favourite_class = if is_favourite { " is-favourite" } else { "" },
        favourite_pressed = if is_favourite { "true" } else { "false" },
        favourite_label = escape_html(&format!(
            "{} {} {} favourites",
            if is_favourite { "Remove" } else { "Add" },
            fish_name,
            if is_favourite { "from" } else { "to" }
        )),
        favourite_expr = escape_html(&favourite_expr),
        favourite_icon = sprite_icon(
            if is_favourite { "heart-fill" } else { "heart-line" },
            "fishy-icon--inline size-6"
        ),
        caught_pressed = if is_caught { "true" } else { "false" },
        caught_label = escape_html(&format!(
            "Mark {} as {}",
            fish_name,
            if is_caught { "not caught" } else { "caught" }
        )),
        caught_expr = escape_html(&caught_expr),
        caught_icon = sprite_icon(
            if is_caught {
                "check-badge-solid"
            } else {
                "check-circle-dash-line"
            },
            "fishy-icon--inline size-7"
        ),
        grade = escape_html(grade),
        icon_alt = escape_html(&format!("{fish_name} icon")),
        vendor_price = render_vendor_price_markup("div", "fishydex-price fishydex-card-price", entry_vendor_price(entry)),
    )
    .unwrap();
    html
}

fn render_fishydex_details_shell(signals: &FishDexSignals, derived: &FishDexDerived) -> String {
    let Some(entry) = derived.selected_fish.as_ref() else {
        return r#"<div id="fishydex-details-shell"></div>"#.to_string();
    };

    let item_id = entry.item_id;
    let fish_name = if entry.name.trim().is_empty() {
        format!("Fish {item_id}")
    } else {
        entry.name.clone()
    };
    let is_caught = signals.caught_ids.contains(&item_id);
    let is_favourite = signals.favourite_ids.contains(&item_id);
    let caught_expr = format!(
        "$caught_ids = window.Fishydex.toggleFishIds($caught_ids, {item_id}); window.Fishydex.persistCaughtIds($caught_ids); window.Fishydex.queueStamp('_caught_stamp_fish_id', $caught_ids.includes({item_id}) ? {item_id} : null); window.Fishydex.clearRequestUi(); @post(window.Fishydex.datastarEvalUrl())"
    );
    let favourite_expr = format!(
        "$favourite_ids = window.Fishydex.toggleFishIds($favourite_ids, {item_id}); window.Fishydex.persistFavouriteIds($favourite_ids); window.Fishydex.queueStamp('_favourite_stamp_fish_id', $favourite_ids.includes({item_id}) ? {item_id} : null); window.Fishydex.clearRequestUi(); @post(window.Fishydex.datastarEvalUrl())"
    );
    let methods = entry_catch_methods(entry);
    let grade = filter_grade_for_entry(entry);

    let mut badges = String::new();
    if is_favourite {
        badges.push_str(r#"<span class="badge badge-soft badge-error">Favourite</span>"#);
    }
    write!(
        badges,
        r#"<span class="fishydex-caught badge badge-soft {caught_badge}">{caught_text}</span><span class="fishydex-grade badge badge-soft grade-{grade}">{grade_label}</span>"#,
        caught_badge = if is_caught { "badge-success" } else { "grade-unknown" },
        caught_text = if is_caught { "Caught" } else { "Not Caught" },
        grade = escape_html(grade),
        grade_label = escape_html(grade_label_for_key(grade)),
    )
    .unwrap();
    if methods.iter().any(|method| method == "rod") {
        badges.push_str(r#"<span class="fishydex-method badge badge-soft method-rod">Rod</span>"#);
    }
    if methods.iter().any(|method| method == "harpoon") {
        badges.push_str(
            r#"<span class="fishydex-method badge badge-soft method-harpoon">Harpoon</span>"#,
        );
    }
    if entry_is_dried(entry) {
        badges.push_str(
            r#"<span class="fishydex-method badge badge-soft method-dried">Dried</span>"#,
        );
    }

    let item_url = format!("https://bdolytics.com/en/NA/db/item/{item_id}");
    let mut html = String::from(r#"<div id="fishydex-details-shell">"#);
    write!(
        html,
        r#"<section class="fishydex-details-panel modal-box card card-border bg-base-100 w-11/12 max-w-5xl" role="dialog" aria-modal="true" aria-labelledby="fishydex-details-title"><div class="fishydex-details-header"><div class="fishydex-details-header-main"><div class="fishydex-details-icon-wrap grade-{grade}" data-fish-item-icon="{item_id}" data-fish-icon-alt="{item_alt}" data-init="window.Fishydex.hydrateItemIcon(el)"><img class="fishydex-details-icon" hidden><div class="fishydex-placeholder">?</div></div><div class="fishydex-details-copy"><div class="fishydex-details-title-row"><h3 id="fishydex-details-title" class="fishydex-details-title">{fish_name}</h3><button type="button" class="fishydex-favourite-button btn btn-sm btn-circle btn-ghost{favourite_class}" aria-pressed="{favourite_pressed}" aria-label="{favourite_label}" data-indicator:_details_loading data-class:is-stamping="$_favourite_stamp_fish_id === {item_id}" data-on:click="{favourite_expr}">{favourite_icon}</button><button type="button" class="fishydex-caught-button btn btn-sm btn-circle btn-ghost{caught_class}" aria-pressed="{caught_pressed}" aria-label="{caught_label}" data-indicator:_details_loading data-class:is-stamping="$_caught_stamp_fish_id === {item_id}" data-on:click="{caught_expr}">{caught_icon}</button></div><div class="fishydex-details-badges">{badges}</div></div></div><button type="button" class="btn btn-sm btn-outline fishydex-button fishydex-details-close" data-on:click="window.Fishydex.focusFishCardAction($selected_fish_id, 'open-details'); $selected_fish_id = null">Close</button></div><div class="fishydex-details-guide"><div class="fishydex-details-guide-frame rounded-box border border-base-300 bg-base-200" data-fish-encyclopedia-icon="{encyclopedia_id}" data-fish-icon-alt="{guide_alt}" data-init="window.Fishydex.hydrateEncyclopediaIcon(el)"><img class="fishydex-details-guide-image" hidden><div class="fishydex-placeholder">?</div></div></div><dl class="fishydex-details-meta"><div class="fishydex-details-meta-card card card-border bg-base-200"><dt>Item Key</dt><dd><a class="fishydex-details-link link link-hover" href="{item_url}" target="_blank" rel="noreferrer noopener">{item_id}</a></dd></div><div class="fishydex-details-meta-card card card-border bg-base-200"><dt>Vendor Price</dt><dd>{vendor_price}</dd></div></dl><div class="fishydex-details-stack"><section class="fishydex-details-section card card-border bg-base-200"><h4>Best Spots</h4><p class="fishydex-details-note">{spots_note}</p></section></div></section></div>"#,
        grade = escape_html(grade),
        item_id = item_id,
        item_alt = escape_html(&format!("{fish_name} icon")),
        fish_name = escape_html(&fish_name),
        favourite_class = if is_favourite { " is-favourite" } else { "" },
        favourite_pressed = if is_favourite { "true" } else { "false" },
        favourite_label = escape_html(&format!(
            "{} {} {} favourites",
            if is_favourite { "Remove" } else { "Add" },
            fish_name,
            if is_favourite { "from" } else { "to" }
        )),
        favourite_expr = escape_html(&favourite_expr),
        favourite_icon = sprite_icon(
            if is_favourite { "heart-fill" } else { "heart-line" },
            "fishy-icon--inline size-6"
        ),
        caught_class = if is_caught { " is-caught" } else { "" },
        caught_pressed = if is_caught { "true" } else { "false" },
        caught_label = escape_html(&format!(
            "Mark {} as {}",
            fish_name,
            if is_caught { "not caught" } else { "caught" }
        )),
        caught_expr = escape_html(&caught_expr),
        caught_icon = sprite_icon(
            if is_caught {
                "check-badge-solid"
            } else {
                "check-circle-dash-line"
            },
            "fishy-icon--inline size-7"
        ),
        badges = badges,
        encyclopedia_id = entry
            .encyclopedia_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
        guide_alt = escape_html(&format!("{fish_name} guide image")),
        item_url = escape_html(&item_url),
        vendor_price = render_vendor_price_markup("span", "fishydex-price fishydex-details-price", entry_vendor_price(entry)),
        spots_note = escape_html(DETAILS_SPOTS_NOTE),
    )
    .unwrap();
    html
}

fn render_vendor_price_markup(tag: &str, class_name: &str, amount: Option<i64>) -> String {
    let mut html = String::new();
    write!(
        html,
        r#"<{tag} class="{class_name}"><span class="fishydex-price-icon">{coin_icon}</span><span class="fishydex-price-value">{amount}</span></{tag}>"#,
        tag = escape_html(tag),
        class_name = escape_html(class_name),
        coin_icon = sprite_icon("coin-stack", ""),
        amount = escape_html(&format_silver(amount)),
    )
    .unwrap();
    html
}

fn format_silver(value: Option<i64>) -> String {
    let Some(mut amount) = value else {
        return "Unavailable".to_string();
    };
    if amount <= 0 {
        return "Unavailable".to_string();
    }

    let mut chunks = Vec::new();
    while amount >= 1000 {
        chunks.push(format!("{:03}", amount % 1000));
        amount /= 1000;
    }
    let mut formatted = amount.to_string();
    for chunk in chunks.iter().rev() {
        formatted.push(',');
        formatted.push_str(chunk);
    }
    formatted
}

fn sprite_icon(name: &str, class_name: &str) -> String {
    let classes = if class_name.is_empty() {
        "fishy-icon".to_string()
    } else {
        format!("fishy-icon {}", class_name)
    };
    format!(
        r#"<svg class="{classes}" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="{sprite}#fishy-{name}"></use></svg>"#,
        classes = escape_html(&classes),
        sprite = escape_html(ICON_SPRITE_URL),
        name = escape_html(name),
    )
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
    use super::*;
    use std::sync::Arc;

    use async_trait::async_trait;
    use axum::http::StatusCode;
    use fishystuff_api::ids::MapVersionId;
    use fishystuff_api::models::calculator::CalculatorCatalogResponse;
    use fishystuff_api::models::effort::{EffortGridRequest, EffortGridResponse};
    use fishystuff_api::models::events::{EventsSnapshotMetaResponse, EventsSnapshotResponse};
    use fishystuff_api::models::meta::{MetaDefaults, MetaResponse};
    use fishystuff_api::models::region_groups::RegionGroupsResponse;
    use fishystuff_api::models::zone_profile_v2::{ZoneProfileV2Request, ZoneProfileV2Response};
    use fishystuff_api::models::zone_stats::{ZoneStatsRequest, ZoneStatsResponse};
    use fishystuff_api::models::zones::ZoneEntry;
    use hyper::body::to_bytes;

    use crate::config::{AppConfig, ZoneStatusConfig};
    use crate::state::AppState;
    use crate::store::Store;

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
            Ok(FishListResponse {
                revision: "dolt:test-fish-rev".to_string(),
                count: 2,
                fish: vec![
                    FishEntry {
                        item_id: 8474,
                        encyclopedia_key: Some(8474),
                        encyclopedia_id: Some(9474),
                        name: "Pirarucu".to_string(),
                        grade: Some("Prize".to_string()),
                        is_prize: Some(true),
                        is_dried: false,
                        catch_methods: vec!["rod".to_string()],
                        vendor_price: Some(120_000_000),
                    },
                    FishEntry {
                        item_id: 8201,
                        encyclopedia_key: Some(821001),
                        encyclopedia_id: Some(8501),
                        name: "Mudskipper".to_string(),
                        grade: Some("General".to_string()),
                        is_prize: Some(false),
                        is_dried: true,
                        catch_methods: vec!["rod".to_string()],
                        vendor_price: Some(16_560),
                    },
                ],
            })
        }

        async fn calculator_catalog(
            &self,
            _lang: FishLang,
            _ref_id: Option<String>,
        ) -> AppResult<CalculatorCatalogResponse> {
            panic!("unused in test")
        }

        async fn list_zones(&self, _ref_id: Option<String>) -> AppResult<Vec<ZoneEntry>> {
            panic!("unused in test")
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
            defaults: MetaDefaults {
                tile_px: 32,
                sigma_tiles: 3.0,
                half_life_days: None,
                alpha0: 1.0,
                top_k: 30,
                map_version_id: Some(MapVersionId("v1".to_string())),
            },
            status_cfg: ZoneStatusConfig::default(),
            cache_zone_stats_max: 4,
            cache_effort_max: 4,
            cache_log: false,
            request_timeout_secs: 5,
        };
        AppState::for_tests(config, Arc::new(MockStore))
    }

    #[tokio::test]
    async fn list_fish_route_returns_revisioned_json_and_no_store_headers() {
        let response = list_fish(
            State(test_state()),
            Ok(Query(FishQuery {
                lang: None,
                r#ref: None,
            })),
            Extension(RequestId("req-test".to_string())),
        )
        .await
        .expect("fish response")
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(header::CACHE_CONTROL)
                .and_then(|value| value.to_str().ok()),
            Some("no-store")
        );

        let body = to_bytes(response.into_body()).await.expect("body bytes");
        let payload: serde_json::Value = serde_json::from_slice(&body).expect("json body");
        assert_eq!(payload["revision"], "dolt:test-fish-rev");
        assert_eq!(payload["count"], 2);
        let fish = payload["fish"].as_array().expect("fish array");
        assert_eq!(fish.len(), 2);
        assert_eq!(fish[0]["item_id"], 8474);
        assert_eq!(fish[1]["is_dried"], true);
    }

    #[tokio::test]
    async fn fish_datastar_init_returns_signal_and_element_patches() {
        let response = get_fish_datastar_init(
            State(test_state()),
            Ok(Query(FishDatastarQuery {
                lang: None,
                r#ref: None,
                datastar: Some(r#"{"caught_ids":[8474],"selected_fish_id":8474}"#.to_string()),
            })),
            Extension(RequestId("req-test".to_string())),
        )
        .await
        .expect("fish datastar init response")
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(header::CACHE_CONTROL)
                .and_then(|value| value.to_str().ok()),
            Some("no-store")
        );

        let body = to_bytes(response.into_body()).await.expect("body bytes");
        let text = String::from_utf8(body.to_vec()).expect("utf8 body");
        assert!(text.contains("event:datastar-patch-signals"));
        assert!(text.contains("event:datastar-patch-elements"));
        assert!(text.contains("\"caught_count\":1"));
        assert!(text.contains("fishydex-grid"));
        assert!(text.contains("Pirarucu"));
        assert!(text.contains("fishydex-details-shell"));
    }

    #[tokio::test]
    async fn fish_datastar_eval_filters_search_results() {
        let response = post_fish_datastar_eval(
            State(test_state()),
            Ok(Query(FishQuery {
                lang: None,
                r#ref: None,
            })),
            Extension(RequestId("req-test".to_string())),
            Bytes::from_static(br#"{"search_query":"pira"}"#),
        )
        .await
        .expect("fish datastar eval response")
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body()).await.expect("body bytes");
        let text = String::from_utf8(body.to_vec()).expect("utf8 body");
        assert!(text.contains("\"visible_count\":1"));
        assert!(text.contains("Pirarucu"));
        assert!(!text.contains("Mudskipper"));
    }
}
