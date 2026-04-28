use std::cmp::{Ordering, Reverse};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::convert::Infallible;
use std::fmt::Write as _;
use std::sync::LazyLock;

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
    CalculatorPetEntry, CalculatorPetOptionEntry, CalculatorPetSignals, CalculatorPetTierEntry,
    CalculatorPriceOverrideSignals, CalculatorSessionPresetEntry, CalculatorSignals,
    CalculatorZoneGroupRateEntry, CalculatorZoneOverlaySignals,
};
use fishystuff_api::models::zone_loot_summary::{
    ZoneLootSummaryConditionOption, ZoneLootSummaryGroupRow, ZoneLootSummaryRequest,
    ZoneLootSummaryResponse, ZoneLootSummarySpeciesRow,
};
use fishystuff_api::models::zones::ZoneEntry;

use crate::error::{with_timeout, AppError, AppResult};
use crate::routes::meta::map_request_id;
use crate::state::{RequestId, SharedState};
use crate::store::{
    CalculatorZoneLootEntry, CalculatorZoneLootEvidence, CalculatorZoneLootOverlayMeta,
    CalculatorZoneLootRateContribution, DataLang,
};

#[derive(Debug, Deserialize)]
pub struct CalculatorQuery {
    pub lang: Option<String>,
    pub locale: Option<String>,
    pub r#ref: Option<String>,
    pub pet_cards: Option<bool>,
    pub target_fish_select: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CalculatorDatastarQuery {
    pub lang: Option<String>,
    pub locale: Option<String>,
    pub r#ref: Option<String>,
    pub datastar: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CalculatorZoneSearchQuery {
    pub lang: Option<String>,
    pub locale: Option<String>,
    pub r#ref: Option<String>,
    pub q: Option<String>,
    pub offset: Option<usize>,
    pub selected: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CalculatorSearchableOptionQuery {
    pub lang: Option<String>,
    pub locale: Option<String>,
    pub r#ref: Option<String>,
    pub kind: Option<String>,
    pub q: Option<String>,
    pub offset: Option<usize>,
    pub results_id: Option<String>,
    pub selected: Option<String>,
    pub tier: Option<String>,
    pub zone: Option<String>,
    pub pack_leader: Option<bool>,
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
    raw_prize_rate_text: String,
    raw_prize_mastery_text: String,
    fish_group_distribution_chart: DistributionChartSignal,
    fish_group_silver_distribution_chart: DistributionChartSignal,
    target_fish_pmf_chart: PmfChartSignal,
    loot_sankey_chart: LootSankeySignal,
    target_fish_selected_label: String,
    target_fish_pmf_count_hint: String,
    target_fish_expected_title: String,
    target_fish_expected_count: String,
    target_fish_per_day: String,
    target_fish_time_to_target: String,
    target_fish_time_to_target_helper: String,
    target_fish_probability_at_least_title: String,
    target_fish_probability_at_least: String,
    target_fish_status_text: String,
    stat_breakdowns: CalculatorStatBreakdownSignals,
    fishing_timeline_chart: TimelineChartSignal,
    overlay_editor: CalculatorOverlayEditorSignal,
    debug_json: String,
}

#[derive(Debug, Clone, serde::Serialize, Default)]
struct CalculatorOverlayEditorSignal {
    zone_rgb_key: String,
    zone_name: String,
    groups: Vec<CalculatorOverlayEditorGroupRow>,
    items: Vec<CalculatorOverlayEditorItemRow>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct CalculatorOverlayEditorGroupRow {
    slot_idx: u8,
    label: String,
    default_present: bool,
    default_raw_rate_pct: f64,
    default_raw_rate_text: String,
    current_raw_rate_pct: f64,
    current_raw_rate_text: String,
    bonus_rate_pct: f64,
    bonus_rate_text: String,
    effective_raw_weight_pct: f64,
    effective_raw_weight_text: String,
    normalized_share_pct: f64,
    normalized_share_text: String,
    bonus_rate_breakdown: String,
    normalized_share_breakdown: String,
}

#[derive(Debug, Clone, serde::Serialize)]
struct CalculatorOverlayEditorItemRow {
    item_id: i32,
    default_present: bool,
    overlay_added: bool,
    slot_idx: u8,
    group_label: String,
    label: String,
    icon_url: Option<String>,
    icon_grade_tone: String,
    default_raw_rate_pct: f64,
    default_raw_rate_text: String,
    normalized_rate_pct: f64,
    normalized_rate_text: String,
    base_price_raw: f64,
    base_price_text: String,
    is_fish: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
struct CalculatorStatBreakdownSignals {
    total_time: String,
    bite_time: String,
    auto_fish_time: String,
    catch_time: String,
    time_saved: String,
    auto_fish_time_reduction: String,
    casts_average: String,
    item_drr: String,
    chance_to_consume_durability: String,
    durability_loss_average: String,
    zone_bite_min: String,
    zone_bite_avg: String,
    zone_bite_max: String,
    effective_bite_min: String,
    effective_bite_avg: String,
    effective_bite_max: String,
    loot_total_catches: String,
    loot_fish_per_hour: String,
    loot_total_profit: String,
    loot_profit_per_hour: String,
    raw_prize_rate: String,
    target_expected_count: String,
    target_time_to_target: String,
    target_probability_at_least: String,
}

#[derive(Debug, Clone, serde::Serialize)]
struct ComputedStatBreakdownRow {
    label: String,
    value_text: String,
    detail_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    icon_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    grade_tone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    formula_part: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    formula_part_order: Option<u8>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct ComputedStatFormulaTerm {
    label: String,
    value_text: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    aliases: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct ComputedStatBreakdownSection {
    label: String,
    rows: Vec<ComputedStatBreakdownRow>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct ComputedStatBreakdown {
    kind_label: String,
    title: String,
    value_text: String,
    summary_text: String,
    formula_text: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    formula_terms: Vec<ComputedStatFormulaTerm>,
    sections: Vec<ComputedStatBreakdownSection>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct TimelineChartSegment {
    label: String,
    value_text: String,
    detail_text: String,
    width_pct: f64,
    fill_color: &'static str,
    stroke_color: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    breakdown: Option<Value>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct TimelineChartSignal {
    segments: Vec<TimelineChartSegment>,
}

impl ComputedStatBreakdown {
    fn with_formula_terms(mut self, formula_terms: Vec<ComputedStatFormulaTerm>) -> Self {
        self.formula_terms = formula_terms;
        self
    }
}

#[derive(Debug, Clone)]
struct FishGroupChartRow {
    label: &'static str,
    fill_color: &'static str,
    stroke_color: &'static str,
    text_color: &'static str,
    connector_color: &'static str,
    drop_rate_source_kind: String,
    drop_rate_tooltip: String,
    bonus_text: String,
    base_share_pct: f64,
    #[allow(dead_code)]
    default_weight_pct: f64,
    weight_pct: f64,
    current_share_pct: f64,
    rate_inputs: Vec<ComputedStatBreakdownRow>,
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
    drop_rate_source_kind: String,
    drop_rate_tooltip: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    condition_text: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    condition_tooltip: String,
    expected_count_raw: f64,
    expected_profit_raw: f64,
    expected_count_text: String,
    expected_profit_text: String,
    current_share_pct: f64,
    count_share_text: String,
    silver_share_text: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    count_breakdown: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    silver_breakdown: String,
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
    item_id: i32,
    group_label: &'static str,
    label: String,
    icon_url: Option<String>,
    icon_grade_tone: String,
    fill_color: &'static str,
    stroke_color: &'static str,
    text_color: &'static str,
    connector_color: &'static str,
    expected_count_raw: f64,
    expected_profit_raw: f64,
    expected_count_text: String,
    expected_profit_text: String,
    silver_share_text: String,
    rate_text: String,
    rate_source_kind: String,
    rate_tooltip: String,
    drop_rate_text: String,
    drop_rate_source_kind: String,
    drop_rate_tooltip: String,
    presence_text: Option<String>,
    presence_source_kind: String,
    presence_tooltip: Option<String>,
    evidence_text: String,
    #[serde(skip_serializing)]
    catch_methods: Vec<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    count_breakdown: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    silver_breakdown: String,
    #[serde(skip_serializing)]
    within_group_rate_raw: f64,
    #[serde(skip_serializing)]
    base_price_raw: f64,
    #[serde(skip_serializing)]
    sale_multiplier_raw: f64,
    #[serde(skip_serializing)]
    discarded: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
struct DistributionChartSegment {
    label: String,
    value_text: String,
    detail_text: String,
    width_pct: f64,
    fill_color: &'static str,
    stroke_color: &'static str,
    text_color: &'static str,
    connector_color: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    breakdown: Option<ComputedStatBreakdown>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct DistributionChartSignal {
    segments: Vec<DistributionChartSegment>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct PmfChartBar {
    label: String,
    value_text: String,
    probability_pct: f64,
    highlight: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
struct PmfChartSignal {
    bars: Vec<PmfChartBar>,
    expected_value_text: String,
}

#[derive(Debug, Clone, serde::Serialize)]
struct LootSankeySignal {
    show_silver_amounts: bool,
    rows: Vec<LootChartRow>,
    species_rows: Vec<LootSpeciesRow>,
}

#[derive(Debug, Clone)]
struct TargetFishSummary {
    selected_label: String,
    target_amount: u32,
    target_amount_text: String,
    pmf_count_hint_text: String,
    expected_count_text: String,
    per_day_text: String,
    time_to_target_text: String,
    probability_at_least_text: String,
    session_distribution: Vec<TargetFishDistributionBucket>,
    status_text: String,
}

#[derive(Debug, Clone)]
struct TargetFishDistributionBucket {
    label: String,
    probability_pct: f64,
    probability_text: String,
}

#[derive(Debug, Clone)]
struct CalculatorData {
    catalog: CalculatorCatalogResponse,
    cdn_base_url: String,
    lang: CalculatorLocale,
    api_lang: DataLang,
    zones: Vec<ZoneEntry>,
    zone_group_rates: HashMap<String, CalculatorZoneGroupRateEntry>,
    zone_loot_entries: Vec<CalculatorZoneLootEntry>,
}

const CALCULATOR_ICON_SPRITE_URL: &str = "/img/icons.svg?v=20260423-3";
const CALCULATOR_MODE_ROD_TEXTURE_URL: &str = "/img/calculator/fishing-mode-rod.png";
const CALCULATOR_MODE_HARPOON_TEXTURE_URL: &str = "/img/calculator/fishing-mode-harpoon.png";
const CALCULATOR_COMBINED_GROUP_RATE_SCALE: f64 = 1_000_000.0 * 1_000_000.0;
type CalculatorRouteCatalog = HashMap<String, String>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CalculatorLocale {
    EnUs,
    DeDe,
    KoKr,
}

impl CalculatorLocale {
    fn from_query(locale: Option<&str>, _lang: Option<&str>) -> Self {
        let value = locale.unwrap_or_default().trim().to_ascii_lowercase();
        match value.split(['-', '_']).next().unwrap_or_default() {
            "ko" => Self::KoKr,
            "de" => Self::DeDe,
            _ => Self::EnUs,
        }
    }
}

static CALCULATOR_ROUTE_CATALOG_EN: LazyLock<CalculatorRouteCatalog> = LazyLock::new(|| {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../site/i18n/en-US.ziggy"
    )))
    .expect("valid en-US calculator route catalog")
});

static CALCULATOR_ROUTE_CATALOG_DE: LazyLock<CalculatorRouteCatalog> = LazyLock::new(|| {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../site/i18n/de-DE.ziggy"
    )))
    .expect("valid de-DE calculator route catalog")
});

static CALCULATOR_ROUTE_CATALOG_KO: LazyLock<CalculatorRouteCatalog> = LazyLock::new(|| {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../site/i18n/ko-KR.ziggy"
    )))
    .expect("valid ko-KR calculator route catalog")
});

#[derive(Debug, Clone, Copy)]
struct SelectOption<'a> {
    value: &'a str,
    label: &'a str,
    icon: Option<&'a str>,
    grade_tone: &'a str,
    pet_variant_talent: Option<&'a CalculatorPetOptionEntry>,
    pet_variant_special: Option<&'a CalculatorPetOptionEntry>,
    pet_skill: Option<&'a CalculatorPetOptionEntry>,
    pet_effective_talent_effects: Option<PetEffectiveTalentEffects>,
    pet_skill_learn_chance: Option<f32>,
    item: Option<&'a CalculatorItemEntry>,
    lifeskill_level: Option<&'a CalculatorLifeskillLevelEntry>,
    presentation: SelectOptionPresentation,
    sort_priority: u8,
}

#[derive(Debug, Clone, Copy)]
struct PetEffectiveTalentEffects {
    item_drr: Option<f64>,
    life_exp: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectOptionPresentation {
    Default,
    PetCard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchableDropdownTriggerSize {
    Fill,
    Content,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchableDropdownResultsLayout {
    List,
    Cards,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchableDropdownPanelPlacement {
    Adjacent,
    OverlayAnchor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PetFixedOptionKind {
    Special,
    Talent,
}

struct SearchableDropdownConfig<'a> {
    catalog_html: Option<&'a str>,
    compact: bool,
    trigger_size: SearchableDropdownTriggerSize,
    trigger_width: Option<&'a str>,
    trigger_min_height: Option<&'a str>,
    panel_width: Option<&'a str>,
    panel_placement: SearchableDropdownPanelPlacement,
    results_layout: SearchableDropdownResultsLayout,
    root_id: &'a str,
    input_id: &'a str,
    label: &'a str,
    selected_content_html: &'a str,
    value: &'a str,
    search_url: &'a str,
    search_url_root: Option<&'a str>,
    exclude_selected_inputs: Option<&'a str>,
    search_placeholder: &'a str,
}

struct SearchableMultiselectConfig<'a> {
    lang: CalculatorLocale,
    root_id: &'a str,
    bind_key: &'a str,
    search_placeholder: &'a str,
    helper_text: Option<&'a str>,
}

struct SearchableDropdownPage<T> {
    items: Vec<T>,
    next_offset: Option<usize>,
}

fn paginate_searchable_dropdown_items<T: Clone>(
    items: Vec<T>,
    offset: usize,
) -> SearchableDropdownPage<T> {
    let start = offset.min(items.len());
    let end = start
        .saturating_add(SEARCHABLE_DROPDOWN_PAGE_SIZE)
        .min(items.len());
    SearchableDropdownPage {
        items: items[start..end].to_vec(),
        next_offset: (end < items.len()).then_some(end),
    }
}

const SEARCHABLE_DROPDOWN_PAGE_SIZE: usize = 24;
const SEARCHABLE_MULTISELECT_RESULT_LIMIT: usize = 24;

static NONE_SELECT_OPTION_EN: LazyLock<SelectOption<'static>> = LazyLock::new(|| SelectOption {
    value: "",
    label: Box::leak(
        calculator_route_text(CalculatorLocale::EnUs, "calculator.server.option.none")
            .into_boxed_str(),
    ),
    icon: None,
    grade_tone: "unknown",
    pet_variant_talent: None,
    pet_variant_special: None,
    pet_skill: None,
    pet_effective_talent_effects: None,
    pet_skill_learn_chance: None,
    item: None,
    lifeskill_level: None,
    presentation: SelectOptionPresentation::Default,
    sort_priority: 1,
});

static NONE_SELECT_OPTION_DE: LazyLock<SelectOption<'static>> = LazyLock::new(|| SelectOption {
    value: "",
    label: Box::leak(
        calculator_route_text(CalculatorLocale::DeDe, "calculator.server.option.none")
            .into_boxed_str(),
    ),
    icon: None,
    grade_tone: "unknown",
    pet_variant_talent: None,
    pet_variant_special: None,
    pet_skill: None,
    pet_effective_talent_effects: None,
    pet_skill_learn_chance: None,
    item: None,
    lifeskill_level: None,
    presentation: SelectOptionPresentation::Default,
    sort_priority: 1,
});

static NONE_SELECT_OPTION_KO: LazyLock<SelectOption<'static>> = LazyLock::new(|| SelectOption {
    value: "",
    label: Box::leak(
        calculator_route_text(CalculatorLocale::KoKr, "calculator.server.option.none")
            .into_boxed_str(),
    ),
    icon: None,
    grade_tone: "unknown",
    pet_variant_talent: None,
    pet_variant_special: None,
    pet_skill: None,
    pet_effective_talent_effects: None,
    pet_skill_learn_chance: None,
    item: None,
    lifeskill_level: None,
    presentation: SelectOptionPresentation::Default,
    sort_priority: 1,
});

fn none_select_option(lang: CalculatorLocale) -> SelectOption<'static> {
    match lang {
        CalculatorLocale::EnUs => *NONE_SELECT_OPTION_EN,
        CalculatorLocale::DeDe => *NONE_SELECT_OPTION_DE,
        CalculatorLocale::KoKr => *NONE_SELECT_OPTION_KO,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CalculatorSearchableOptionKind {
    FishingLevel,
    LifeskillLevel,
    TradeLevel,
    TargetFish,
    Rod,
    Float,
    Chair,
    LightstoneSet,
    Backpack,
    Pet,
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
            "target_fish" => Some(Self::TargetFish),
            "rod" => Some(Self::Rod),
            "float" => Some(Self::Float),
            "chair" => Some(Self::Chair),
            "lightstone_set" => Some(Self::LightstoneSet),
            "backpack" => Some(Self::Backpack),
            "pet" => Some(Self::Pet),
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
            Self::TargetFish => "target_fish",
            Self::Rod => "rod",
            Self::Float => "float",
            Self::Chair => "chair",
            Self::LightstoneSet => "lightstone_set",
            Self::Backpack => "backpack",
            Self::Pet => "pet",
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

    let lang = data_lang_from_query(query.lang.as_deref(), &request_id)?;
    let catalog = with_timeout(
        state.config.request_timeout_secs,
        state.store.calculator_catalog(lang, query.r#ref),
    )
    .await
    .map_err(|err| map_request_id(err, &request_id))?;

    let mut headers = HeaderMap::new();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok((headers, Json(catalog)))
}

pub async fn post_zone_loot_summary(
    State(state): State<SharedState>,
    query: Result<Query<CalculatorQuery>, QueryRejection>,
    Extension(request_id): Extension<RequestId>,
    payload: Result<Json<ZoneLootSummaryRequest>, axum::extract::rejection::JsonRejection>,
) -> AppResult<Json<ZoneLootSummaryResponse>> {
    let Query(query) = query.map_err(|err| {
        AppError::invalid_argument(err.to_string()).with_request_id(request_id.0.clone())
    })?;
    let Json(payload) = payload.map_err(|err| {
        AppError::invalid_argument(err.to_string()).with_request_id(request_id.0.clone())
    })?;

    let lang = data_lang_from_query(query.lang.as_deref(), &request_id)?;
    let locale = CalculatorLocale::from_query(query.locale.as_deref(), query.lang.as_deref());
    let summary = load_zone_loot_summary_data(
        &state,
        lang,
        locale,
        query.r#ref.clone(),
        &request_id,
        payload,
    )
    .await?;
    Ok(Json(summary))
}

pub async fn get_calculator_datastar_init(
    State(state): State<SharedState>,
    query: Result<Query<CalculatorDatastarQuery>, QueryRejection>,
    Extension(request_id): Extension<RequestId>,
) -> AppResult<impl IntoResponse> {
    let Query(query) = query.map_err(|err| {
        AppError::invalid_argument(err.to_string()).with_request_id(request_id.0.clone())
    })?;

    let lang = data_lang_from_query(query.lang.as_deref(), &request_id)?;
    let locale = CalculatorLocale::from_query(query.locale.as_deref(), query.lang.as_deref());
    let data =
        load_calculator_data(&state, &lang, locale, query.r#ref.clone(), &request_id).await?;
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
    let (data, normalized_signals, derived) = load_calculator_runtime_data(
        &state,
        lang,
        locale,
        query.r#ref.clone(),
        &request_id,
        raw_signals,
    )
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

    let lang = data_lang_from_query(query.lang.as_deref(), &request_id)?;
    let locale = CalculatorLocale::from_query(query.locale.as_deref(), query.lang.as_deref());
    let data =
        load_calculator_data(&state, &lang, locale, query.r#ref.clone(), &request_id).await?;
    let raw_signals = parse_calculator_signals_body(&body, &data.catalog.defaults, &request_id)?;
    let (data, normalized_signals, derived) = load_calculator_runtime_data(
        &state,
        lang,
        locale,
        query.r#ref.clone(),
        &request_id,
        raw_signals,
    )
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

    let lang = data_lang_from_query(query.lang.as_deref(), &request_id)?;
    let locale = CalculatorLocale::from_query(query.locale.as_deref(), query.lang.as_deref());
    let data =
        load_calculator_data(&state, &lang, locale, query.r#ref.clone(), &request_id).await?;
    let raw_signals = parse_calculator_signals_body(&body, &data.catalog.defaults, &request_id)?;
    let (data, normalized_signals, derived) = load_calculator_runtime_data(
        &state,
        lang,
        locale,
        query.r#ref.clone(),
        &request_id,
        raw_signals,
    )
    .await?;
    let include_pet_cards = query.pet_cards.unwrap_or(true);
    let include_target_fish_select = query.target_fish_select.unwrap_or(false);
    let mut events = vec![calculator_signals_event(
        &normalized_signals,
        &derived,
        CalculatorPatchMode::Eval,
        None,
    )?
    .into_datastar_event()];
    if include_pet_cards {
        events.push(
            PatchElements::new(render_pet_cards(
                data.cdn_base_url.as_str(),
                &data.api_lang,
                data.lang,
                &data.catalog.pets,
                &normalized_signals,
            ))
            .selector("#pets")
            .mode(ElementPatchMode::Outer)
            .into_datastar_event(),
        );
    }
    if include_target_fish_select {
        let target_fishes = target_fish_options(&data);
        events.push(
            PatchElements::new(render_target_fish_select_control(
                &data,
                &normalized_signals,
                &target_fishes,
            ))
            .selector("#calculator-target-fish-control")
            .mode(ElementPatchMode::Inner)
            .into_datastar_event(),
        );
    }
    if !include_pet_cards {
        events.extend(render_pet_talent_fixed_option_patch_events(
            data.lang,
            &data.catalog.pets,
            &normalized_signals,
        ));
    }
    Ok(calculator_datastar_response(events))
}

fn render_pet_talent_fixed_option_patch_events(
    lang: CalculatorLocale,
    catalog: &CalculatorPetCatalog,
    signals: &CalculatorSignals,
) -> Vec<DatastarEvent> {
    let total_slots = catalog.slots.max(1);
    let mut events = Vec::new();
    for slot in 1..=total_slots {
        let pet = match slot {
            1 => &signals.pet1,
            2 => &signals.pet2,
            3 => &signals.pet3,
            4 => &signals.pet4,
            _ => &signals.pet5,
        };
        let selected_talent = pet_option_by_key(&catalog.talents, &pet.talent);
        let input_id = format!("calculator-pet{slot}-talent-value");
        let content_html = render_pet_fixed_talent_content(lang, pet, catalog, selected_talent);
        let selector = format!("#{}", render_pet_fixed_option_content_id(&input_id));
        events.push(
            PatchElements::new(render_pet_fixed_option_display(&input_id, &content_html))
                .selector(&selector)
                .mode(ElementPatchMode::Outer)
                .into_datastar_event(),
        );
    }
    events
}

pub async fn get_calculator_datastar_zone_search(
    State(state): State<SharedState>,
    query: Result<Query<CalculatorZoneSearchQuery>, QueryRejection>,
    Extension(request_id): Extension<RequestId>,
) -> AppResult<impl IntoResponse> {
    let Query(query) = query.map_err(|err| {
        AppError::invalid_argument(err.to_string()).with_request_id(request_id.0.clone())
    })?;

    let lang = data_lang_from_query(query.lang.as_deref(), &request_id)?;
    let locale = CalculatorLocale::from_query(query.locale.as_deref(), query.lang.as_deref());
    let data =
        load_calculator_data(&state, &lang, locale, query.r#ref.clone(), &request_id).await?;
    let selected_zone = query
        .selected
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(data.catalog.defaults.zone.as_str());
    let search_text = query.q.unwrap_or_default();
    let offset = query.offset.unwrap_or(0);
    let fragment = render_zone_search_results(
        data.lang,
        "calculator-zone-search-results",
        &data.zones,
        selected_zone,
        &search_text,
        offset,
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
    let lang = data_lang_from_query(query.lang.as_deref(), &request_id)?;
    let locale = CalculatorLocale::from_query(query.locale.as_deref(), query.lang.as_deref());
    let mut data =
        load_calculator_data(&state, &lang, locale, query.r#ref.clone(), &request_id).await?;
    if kind == CalculatorSearchableOptionKind::TargetFish {
        let zone = query
            .zone
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(data.catalog.defaults.zone.as_str())
            .to_string();
        data.zone_loot_entries = with_timeout(
            state.config.request_timeout_secs,
            state
                .store
                .calculator_zone_loot(lang, query.r#ref.clone(), zone),
        )
        .await
        .map_err(|err| map_request_id(err, &request_id))?;
    }
    let selected_value = query.selected.as_deref().unwrap_or_default();
    let search_text = query.q.unwrap_or_default();
    let offset = query.offset.unwrap_or(0);
    let results_id = query
        .results_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("calculator-search-results");
    let pet_context = (kind == CalculatorSearchableOptionKind::Pet).then(|| CalculatorPetSignals {
        pet: selected_value.to_string(),
        tier: query
            .tier
            .clone()
            .unwrap_or_else(|| data.catalog.defaults.pet1.tier.clone()),
        pack_leader: query.pack_leader.unwrap_or(false),
        ..CalculatorPetSignals::default()
    });
    let (options, include_none) = searchable_options_for_kind(
        &data,
        kind,
        query.tier.as_deref(),
        Some(selected_value),
        pet_context.as_ref(),
    );
    let fragment = render_searchable_select_results(
        data.lang,
        data.cdn_base_url.as_str(),
        results_id,
        &with_optional_none(&options, include_none, data.lang),
        selected_value,
        &search_text,
        offset,
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
            &normalized_signals,
            &derived,
            CalculatorPatchMode::Init,
            Some(&data.catalog.defaults),
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
    signals: &CalculatorSignals,
    derived: &CalculatorDerivedSignals,
    mode: CalculatorPatchMode,
    defaults: Option<&CalculatorSignals>,
) -> AppResult<PatchSignals> {
    let mut patch = match mode {
        CalculatorPatchMode::Init => init_signals_patch_map(signals)?,
        CalculatorPatchMode::Eval => serde_json::Map::new(),
    };
    if matches!(mode, CalculatorPatchMode::Init) {
        patch.insert("_loading".to_string(), Value::Bool(false));
        if let Some(defaults) = defaults {
            patch.insert(
                "_defaults".to_string(),
                Value::Object(default_reset_signals_patch_map(defaults)?),
            );
        }
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
    coerce_object_calculator_overlay_map(&mut object, "overlay");
    coerce_object_f64(&mut object, "catchTimeActive");
    coerce_object_f64(&mut object, "catchTimeAfk");
    coerce_object_f64(&mut object, "timespanAmount");
    coerce_object_string(&mut object, "fishingMode");
    coerce_object_bool(&mut object, "brand");
    coerce_object_bool(&mut object, "active");
    coerce_object_bool(&mut object, "debug");
    coerce_object_bool(&mut object, "applyTradeModifiers");
    coerce_object_bool(&mut object, "showSilverAmounts");
    coerce_object_bool(&mut object, "showNormalizedSelectRates");
    coerce_object_string(&mut object, "discardGrade");
    coerce_object_string_array(&mut object, "outfit");
    coerce_object_string_array(&mut object, "food");
    coerce_object_string_array(&mut object, "buff");

    for key in ["pet1", "pet2", "pet3", "pet4", "pet5"] {
        if let Some(Value::Object(pet)) = object.get_mut(key) {
            coerce_nested_string(pet, "pet");
            coerce_nested_string(pet, "tier");
            coerce_nested_bool(pet, "packLeader");
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

fn normalize_overlay_zone_key(value: &str) -> Option<String> {
    let normalized = value.trim();
    (!normalized.is_empty()).then(|| normalized.to_string())
}

fn normalize_group_overlay_key(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if let Ok(parsed) = trimmed.parse::<u8>() {
        return (1..=5).contains(&parsed).then(|| parsed.to_string());
    }
    fish_group_slot_idx(trimmed).map(|slot_idx| slot_idx.to_string())
}

fn normalize_group_overlay_slot_idx(value: Option<&Value>) -> Option<u8> {
    match value {
        Some(Value::Number(number)) => number.as_u64().and_then(|value| u8::try_from(value).ok()),
        Some(Value::String(string)) => {
            let trimmed = string.trim();
            if let Ok(parsed) = trimmed.parse::<u8>() {
                Some(parsed)
            } else {
                fish_group_slot_idx(trimmed)
            }
        }
        _ => None,
    }
    .filter(|slot_idx| (1..=5).contains(slot_idx))
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

fn coerce_object_calculator_overlay_map(object: &mut serde_json::Map<String, Value>, key: &str) {
    let Some(Value::Object(overlay)) = object.get_mut(key) else {
        return;
    };
    let Some(Value::Object(zones)) = overlay.get_mut("zones") else {
        *overlay = serde_json::Map::new();
        return;
    };
    let normalized_zones = zones
        .iter()
        .filter_map(|(raw_zone_key, value)| {
            let zone_key = normalize_overlay_zone_key(raw_zone_key)?;
            let Value::Object(zone_entry) = value else {
                return None;
            };
            let normalized_groups = zone_entry
                .get("groups")
                .and_then(Value::as_object)
                .map(|groups| {
                    groups
                        .iter()
                        .filter_map(|(raw_group_key, value)| {
                            let group_key = normalize_group_overlay_key(raw_group_key)?;
                            let Value::Object(group_entry) = value else {
                                return None;
                            };
                            let present =
                                group_entry.get("present").and_then(|value| match value {
                                    Value::Bool(value) => Some(*value),
                                    Value::String(string) => {
                                        match string.trim().to_ascii_lowercase().as_str() {
                                            "true" | "1" | "yes" | "on" => Some(true),
                                            "false" | "0" | "no" | "off" => Some(false),
                                            _ => None,
                                        }
                                    }
                                    Value::Number(number) => {
                                        number.as_i64().map(|value| value != 0)
                                    }
                                    _ => None,
                                });
                            let raw_rate_percent = group_entry
                                .get("rawRatePercent")
                                .and_then(|value| match value {
                                    Value::Number(number) => number.as_f64(),
                                    Value::String(string) => string.trim().parse::<f64>().ok(),
                                    _ => None,
                                })
                                .map(|value| value.clamp(0.0, 100.0));
                            if present.is_none() && raw_rate_percent.is_none() {
                                return None;
                            }
                            Some((
                                group_key,
                                json!({
                                    "present": present,
                                    "rawRatePercent": raw_rate_percent,
                                }),
                            ))
                        })
                        .collect::<serde_json::Map<_, _>>()
                })
                .unwrap_or_default();
            let normalized_items = zone_entry
                .get("items")
                .and_then(Value::as_object)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|(raw_item_key, value)| {
                            let item_key = normalize_price_override_key(raw_item_key)?;
                            let Value::Object(item_entry) = value else {
                                return None;
                            };
                            let present = item_entry.get("present").and_then(|value| match value {
                                Value::Bool(value) => Some(*value),
                                Value::String(string) => {
                                    match string.trim().to_ascii_lowercase().as_str() {
                                        "true" | "1" | "yes" | "on" => Some(true),
                                        "false" | "0" | "no" | "off" => Some(false),
                                        _ => None,
                                    }
                                }
                                Value::Number(number) => number.as_i64().map(|value| value != 0),
                                _ => None,
                            });
                            let slot_idx =
                                normalize_group_overlay_slot_idx(item_entry.get("slotIdx"));
                            let raw_rate_percent = item_entry
                                .get("rawRatePercent")
                                .and_then(|value| match value {
                                    Value::Number(number) => number.as_f64(),
                                    Value::String(string) => string.trim().parse::<f64>().ok(),
                                    _ => None,
                                })
                                .map(|value| value.clamp(0.0, 100.0));
                            let name = item_entry
                                .get("name")
                                .and_then(Value::as_str)
                                .map(str::trim)
                                .filter(|value| !value.is_empty())
                                .map(ToOwned::to_owned);
                            let grade = item_entry
                                .get("grade")
                                .and_then(Value::as_str)
                                .map(str::trim)
                                .filter(|value| !value.is_empty())
                                .map(ToOwned::to_owned);
                            let is_fish = item_entry.get("isFish").and_then(|value| match value {
                                Value::Bool(value) => Some(*value),
                                Value::String(string) => {
                                    match string.trim().to_ascii_lowercase().as_str() {
                                        "true" | "1" | "yes" | "on" => Some(true),
                                        "false" | "0" | "no" | "off" => Some(false),
                                        _ => None,
                                    }
                                }
                                Value::Number(number) => number.as_i64().map(|value| value != 0),
                                _ => None,
                            });
                            if present.is_none()
                                && slot_idx.is_none()
                                && raw_rate_percent.is_none()
                                && name.is_none()
                                && grade.is_none()
                                && is_fish.is_none()
                            {
                                return None;
                            }
                            Some((
                                item_key,
                                json!({
                                    "present": present,
                                    "slotIdx": slot_idx,
                                    "rawRatePercent": raw_rate_percent,
                                    "name": name,
                                    "grade": grade,
                                    "isFish": is_fish,
                                }),
                            ))
                        })
                        .collect::<serde_json::Map<_, _>>()
                })
                .unwrap_or_default();
            if normalized_groups.is_empty() && normalized_items.is_empty() {
                return None;
            }
            Some((
                zone_key,
                json!({
                    "groups": normalized_groups,
                    "items": normalized_items,
                }),
            ))
        })
        .collect::<serde_json::Map<_, _>>();
    let mut normalized_overlay = serde_json::Map::new();
    normalized_overlay.insert("zones".to_string(), Value::Object(normalized_zones));
    *overlay = normalized_overlay;
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

fn coerce_nested_bool(object: &mut serde_json::Map<String, Value>, key: &str) {
    if let Some(value) = object.get_mut(key) {
        coerce_value_bool(value);
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
    api_lang: &DataLang,
    lang: CalculatorLocale,
    ref_id: Option<String>,
    request_id: &RequestId,
) -> AppResult<CalculatorData> {
    let catalog_ref_id = ref_id.clone();
    let zones_ref_id = ref_id;
    let (catalog, zones) = tokio::try_join!(
        async {
            with_timeout(
                state.config.request_timeout_secs,
                state
                    .store
                    .calculator_catalog(api_lang.clone(), catalog_ref_id),
            )
            .await
            .map_err(|err| map_request_id(err, request_id))
        },
        async {
            with_timeout(
                state.config.request_timeout_secs,
                state.store.list_zones(zones_ref_id),
            )
            .await
            .map_err(|err| map_request_id(err, request_id))
        }
    )?;
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
        api_lang: api_lang.clone(),
        zones,
        zone_group_rates,
        zone_loot_entries: Vec::new(),
    })
}

async fn load_calculator_runtime_data(
    state: &SharedState,
    api_lang: DataLang,
    lang: CalculatorLocale,
    ref_id: Option<String>,
    request_id: &RequestId,
    raw_signals: CalculatorSignals,
) -> AppResult<(CalculatorData, CalculatorSignals, CalculatorDerivedSignals)> {
    let mut data = load_calculator_data(state, &api_lang, lang, ref_id.clone(), request_id).await?;
    let mut signals = raw_signals;
    normalize_signals(&mut signals, &data);
    let base_zone_loot_entries = with_timeout(
        state.config.request_timeout_secs,
        state
            .store
            .calculator_zone_loot(api_lang, ref_id, signals.zone.clone()),
    )
    .await
    .map_err(|err| map_request_id(err, request_id))?;
    data.zone_loot_entries =
        apply_calculator_condition_context_to_loot_entries(&signals, &base_zone_loot_entries);
    data.zone_loot_entries =
        apply_zone_overlay_to_loot_entries(&signals, &signals.zone, &data.zone_loot_entries);
    normalize_zone_target_fish(&mut signals, &data);
    let derived = derive_signals(&signals, &data);
    Ok((data, signals, derived))
}

async fn load_zone_loot_summary_data(
    state: &SharedState,
    api_lang: DataLang,
    lang: CalculatorLocale,
    ref_id: Option<String>,
    request_id: &RequestId,
    request: ZoneLootSummaryRequest,
) -> AppResult<ZoneLootSummaryResponse> {
    let mut data = load_calculator_data(state, &api_lang, lang, ref_id.clone(), request_id).await?;
    let requested_zone_key = request.rgb.0.trim().to_string();
    let zone = data
        .zones
        .iter()
        .find(|zone| zone.rgb_key.0 == requested_zone_key)
        .cloned()
        .ok_or_else(|| {
            AppError::invalid_argument(format!(
                "unknown zone rgb key for zone loot summary: {}",
                request.rgb.0
            ))
            .with_request_id(request_id.0.clone())
        })?;

    let mut signals = data.catalog.defaults.clone();
    signals.zone = zone.rgb_key.0.clone();
    signals.show_silver_amounts = false;
    signals.overlay = request.overlay;
    normalize_signals(&mut signals, &data);
    let base_zone_loot_entries = with_timeout(
        state.config.request_timeout_secs,
        state
            .store
            .calculator_zone_loot(api_lang, ref_id, signals.zone.clone()),
    )
    .await
    .map_err(|err| map_request_id(err, request_id))?;
    let condition_options_by_slot =
        build_zone_loot_summary_condition_options(&signals, &data, &base_zone_loot_entries);
    data.zone_loot_entries =
        apply_calculator_condition_context_to_loot_entries(&signals, &base_zone_loot_entries);
    data.zone_loot_entries =
        apply_zone_overlay_to_loot_entries(&signals, &signals.zone, &data.zone_loot_entries);

    Ok(derive_zone_loot_summary_response_with_condition_options(
        &signals,
        &data,
        &zone,
        &condition_options_by_slot,
    ))
}

fn lang_param(lang: &DataLang) -> &str {
    lang.code()
}

fn data_lang_from_query(lang: Option<&str>, request_id: &RequestId) -> AppResult<DataLang> {
    DataLang::from_param(lang).map_err(|err| map_request_id(err, request_id))
}

fn locale_param(lang: CalculatorLocale) -> &'static str {
    match lang {
        CalculatorLocale::EnUs => "en-US",
        CalculatorLocale::DeDe => "de-DE",
        CalculatorLocale::KoKr => "ko-KR",
    }
}

fn calculator_route_catalog(lang: CalculatorLocale) -> &'static CalculatorRouteCatalog {
    match lang {
        CalculatorLocale::EnUs => &CALCULATOR_ROUTE_CATALOG_EN,
        CalculatorLocale::DeDe => &CALCULATOR_ROUTE_CATALOG_DE,
        CalculatorLocale::KoKr => &CALCULATOR_ROUTE_CATALOG_KO,
    }
}

fn calculator_route_text(lang: CalculatorLocale, key: &str) -> String {
    calculator_route_catalog(lang)
        .get(key)
        .cloned()
        .unwrap_or_else(|| key.to_string())
}

fn calculator_route_text_with_vars(
    lang: CalculatorLocale,
    key: &str,
    vars: &[(&str, &str)],
) -> String {
    let mut text = calculator_route_text(lang, key);
    for (name, value) in vars {
        text = text.replace(&format!("{{${}}}", name), value);
    }
    text
}

fn calculator_section_icon_alias(section_id: &str) -> Option<&'static str> {
    match section_id {
        "mode" => Some("fish-fill"),
        "overview" => Some("information-fill"),
        "zone" => Some("fullscreen-fill"),
        "bite_time" => Some("stopwatch-2-fill"),
        "catch_time" => Some("stopwatch-fill"),
        "session" => Some("time-fill"),
        "distribution" => Some("chart-pie-2-fill"),
        "loot" => Some("trending-up-fill"),
        "trade" => Some("wheel-fill"),
        "food" => Some("dinner-fill"),
        "buffs" => Some("arrows-up-fill"),
        "pets" => Some("paw-fill"),
        "overlay" => Some("edit-4-fill"),
        "debug" => Some("bug-fill"),
        _ => None,
    }
}

fn normalize_calculator_fishing_mode(value: &str, default: &str) -> String {
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "rod" | "hotspot" | "harpoon" => normalized,
        _ => {
            let fallback = default.trim().to_ascii_lowercase();
            match fallback.as_str() {
                "rod" | "hotspot" | "harpoon" => fallback,
                _ => "rod".to_string(),
            }
        }
    }
}

fn calculator_effective_active(fishing_mode: &str, active: bool) -> bool {
    normalize_calculator_fishing_mode(fishing_mode, "rod") == "harpoon" || active
}

fn render_calculator_icon(alias: &str, size_class: &str) -> String {
    format!(
        r#"<svg class="fishy-icon fishy-icon--inline {}" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="{}#fishy-{}"></use></svg>"#,
        escape_html(size_class),
        CALCULATOR_ICON_SPRITE_URL,
        escape_html(alias),
    )
}

fn render_calculator_tab_label(section_id: &str, title: &str) -> String {
    let title = escape_html(title);
    let section_icon = calculator_section_icon_alias(section_id)
        .map(|icon_alias| {
            format!(
                r#"<span class="fishy-calculator-tab-icon shrink-0">{}</span>"#,
                render_calculator_icon(icon_alias, "size-4"),
            )
        })
        .unwrap_or_default();
    let pin_icon = render_calculator_icon("pin", "size-3");
    format!(
        r#"<span class="fishy-calculator-tab-label inline-flex items-center gap-2"><span class="fishy-calculator-tab-main inline-flex min-w-0 items-center gap-2">{}<span>{}</span></span><span class="fishy-calculator-tab-pin shrink-0" aria-hidden="true">{}</span></span>"#,
        section_icon, title, pin_icon,
    )
}

fn render_calculator_panel_legend(
    lang: CalculatorLocale,
    section_id: &str,
    title: &str,
    icon_alias: Option<&str>,
) -> String {
    let drag_label = escape_html(&calculator_route_text(
        lang,
        "calculator.server.action.drag_section_generic",
    ));
    let pin_label = escaped_js_string_literal(&calculator_route_text_with_vars(
        lang,
        "calculator.server.action.pin_section",
        &[("label", title)],
    ));
    let unpin_label = escaped_js_string_literal(&calculator_route_text_with_vars(
        lang,
        "calculator.server.action.unpin_section",
        &[("label", title)],
    ));
    let label_icon = icon_alias
        .or_else(|| calculator_section_icon_alias(section_id))
        .map_or_else(String::new, |icon_alias| {
            render_calculator_icon(icon_alias, "size-5")
        });
    let section_id = escape_html(section_id);
    format!(
        r#"<legend class="fishy-calculator-panel-legend fishy-calculator-panel-legend--split fieldset-legend ml-6 px-2">
            <span class="fishy-calculator-panel-heading">
                <span class="fishy-calculator-panel-label">{}<span>{}</span></span>
                <span class="fishy-calculator-panel-controls">
                    <button type="button"
                            class="fishy-calculator-panel-control fishy-calculator-panel-control--drag btn btn-ghost btn-xs btn-circle"
                            data-calculator-section-drag
                            data-calculator-section-id="{}"
                            aria-label="{}"
                            title="{}"
                            data-i18n-attr-aria-label="calculator.server.action.drag_section_generic"
                            data-i18n-attr-title="calculator.server.action.drag_section_generic"><svg class="fishy-icon fishy-icon--inline size-4" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="{}#fishy-drag-handle"></use></svg></button>
                </span>
            </span>
            <span class="fishy-calculator-panel-rule" aria-hidden="true"></span>
            <span class="fishy-calculator-panel-pin-slot">
                <button type="button"
                        class="fishy-calculator-panel-control fishy-calculator-panel-control--pin btn btn-ghost btn-xs btn-circle"
                        data-calculator-section-id="{}"
                        data-class:fishy-calculator-panel-control--active="window.__fishystuffCalculator.isPinnedSection($_calculator_ui, '{}')"
                        data-attr:aria-pressed="window.__fishystuffCalculator.isPinnedSection($_calculator_ui, '{}').toString()"
                        data-attr:aria-label="window.__fishystuffCalculator.isPinnedSection($_calculator_ui, '{}') ? {} : {}"
                        data-attr:title="window.__fishystuffCalculator.isPinnedSection($_calculator_ui, '{}') ? {} : {}"
                        data-on:click="window.__fishystuffCalculator.blurActiveElement(); window.__fishystuffCalculator.togglePinnedSectionInPlace($_calculator_ui, '{}')"><svg class="fishy-icon fishy-icon--inline size-4" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="{}#fishy-pin"></use></svg></button>
            </span>
        </legend>"#,
        label_icon,
        escape_html(title),
        section_id,
        drag_label,
        drag_label,
        CALCULATOR_ICON_SPRITE_URL,
        section_id,
        section_id,
        section_id,
        section_id,
        unpin_label,
        pin_label,
        section_id,
        unpin_label,
        pin_label,
        section_id,
        CALCULATOR_ICON_SPRITE_URL,
    )
}

fn render_calculator_unpinned_slot_handle(lang: CalculatorLocale) -> String {
    let drag_label = escape_html(&calculator_route_text(
        lang,
        "calculator.server.action.drag_unpinned_slot",
    ));
    format!(
        r#"<button type="button"
                  class="fishy-calculator-unpinned-slot-handle"
                  data-calculator-unpinned-slot-drag
                  aria-label="{}"
                  title="{}"
                  data-i18n-attr-aria-label="calculator.server.action.drag_unpinned_slot"
                  data-i18n-attr-title="calculator.server.action.drag_unpinned_slot"
                  hidden><svg class="fishy-icon fishy-icon--inline size-4" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="{}#fishy-dots-fill"></use></svg></button>
           <div class="fishy-calculator-unpinned-slot-handle fishy-calculator-unpinned-slot-handle--projection"
                data-calculator-unpinned-slot-projection
                aria-hidden="true"
                hidden><svg class="fishy-icon fishy-icon--inline size-4" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="{}#fishy-dots-fill"></use></svg></div>"#,
        drag_label, drag_label, CALCULATOR_ICON_SPRITE_URL, CALCULATOR_ICON_SPRITE_URL,
    )
}

fn render_calculator_mode_texture_icon(src: &str) -> String {
    format!(
        r#"<img class="fishy-calculator-mode-choice__image" src="{}" alt="" aria-hidden="true" loading="lazy" decoding="async">"#,
        escape_html(src),
    )
}

fn render_calculator_mode_sprite_icon(alias: &str) -> String {
    format!(
        r#"<span class="fishy-calculator-mode-choice__sprite" aria-hidden="true">{}</span>"#,
        render_calculator_icon(alias, "size-10"),
    )
}

fn render_calculator_mode_choice(
    label: &str,
    selected_expr: &str,
    click_expr: &str,
    icon_html: &str,
    disabled_expr: Option<&str>,
) -> String {
    let disabled_class_attr = disabled_expr
        .map(|expr| {
            format!(
                r#" data-class:fishy-calculator-mode-choice--disabled="{}""#,
                escape_html(expr),
            )
        })
        .unwrap_or_default();
    let disabled_aria_attr = disabled_expr
        .map(|expr| {
            format!(
                r#" data-attr:aria-disabled="({}).toString()""#,
                escape_html(expr),
            )
        })
        .unwrap_or_default();
    format!(
        r#"<button type="button"
                class="fishy-calculator-mode-choice"
                data-class:fishy-calculator-mode-choice--selected="{}"{}{}
                data-attr:aria-pressed="({}).toString()"
                data-on:click="{}">
                <span class="fishy-calculator-mode-choice__icon-frame">{}</span>
                <span class="fishy-calculator-mode-choice__label">{}</span>
            </button>"#,
        escape_html(selected_expr),
        disabled_class_attr,
        disabled_aria_attr,
        escape_html(selected_expr),
        escape_html(click_expr),
        icon_html,
        escape_html(label),
    )
}

fn render_calculator_mode_window(lang: CalculatorLocale) -> String {
    let section_title = calculator_route_text(lang, "calculator.server.section.mode");
    let fishing_mode_label = calculator_route_text(lang, "calculator.server.field.fishing_mode");
    let fishing_method_label =
        calculator_route_text(lang, "calculator.server.field.fishing_method");
    let harpoon_note = calculator_route_text(lang, "calculator.server.mode.harpoon_forces_active");
    let normalized_mode = "window.__fishystuffCalculator.normalizeFishingMode($fishingMode)";
    let effective_active = "window.__fishystuffCalculator.effectiveActivity($fishingMode, $active)";
    let harpoon_selected =
        "window.__fishystuffCalculator.normalizeFishingMode($fishingMode) === 'harpoon'";
    let mode_choices = [
        render_calculator_mode_choice(
            &calculator_route_text(lang, "calculator.server.mode.rod"),
            &format!("{normalized_mode} === 'rod'"),
            "$fishingMode = 'rod'",
            &render_calculator_mode_texture_icon(CALCULATOR_MODE_ROD_TEXTURE_URL),
            None,
        ),
        render_calculator_mode_choice(
            &calculator_route_text(lang, "calculator.server.mode.hotspot"),
            &format!("{normalized_mode} === 'hotspot'"),
            "$fishingMode = 'hotspot'",
            &render_calculator_mode_sprite_icon("fish-fill"),
            None,
        ),
        render_calculator_mode_choice(
            &calculator_route_text(lang, "calculator.server.mode.harpoon"),
            &format!("{normalized_mode} === 'harpoon'"),
            "$fishingMode = 'harpoon'",
            &render_calculator_mode_texture_icon(CALCULATOR_MODE_HARPOON_TEXTURE_URL),
            None,
        ),
    ]
    .join("");
    let method_choices = [
        render_calculator_mode_choice(
            &calculator_route_text(lang, "calculator.server.field.afk"),
            &format!("!{effective_active}"),
            &format!("$active = {harpoon_selected} ? $active : false"),
            &render_calculator_icon("time-fill", "size-10"),
            Some(harpoon_selected),
        ),
        render_calculator_mode_choice(
            &calculator_route_text(lang, "calculator.server.field.active"),
            effective_active,
            "$active = true",
            &render_calculator_icon("stopwatch-fill", "size-10"),
            None,
        ),
    ]
    .join("");

    format!(
        r#"<div data-show="window.__fishystuffCalculator.sectionVisible('mode', $_calculator_ui.top_level_tab, $_calculator_ui.pinned_sections)"
         class="grid gap-6 fishy-calculator-section-card"
         data-calculator-section-card
         data-calculator-section-id="mode"
         data-class:fishy-calculator-section-card--pinned="window.__fishystuffCalculator.isPinnedSection($_calculator_ui.pinned_sections, 'mode')">
        <fieldset class="card card-border bg-base-100">
            {}
            <div class="card-body gap-5 pt-0">
                <fieldset class="fieldset">
                    <legend class="fieldset-legend">{}</legend>
                    <div class="grid grid-cols-3 gap-3">{}</div>
                </fieldset>
                <fieldset class="fieldset">
                    <legend class="fieldset-legend">{}</legend>
                    <div class="grid grid-cols-2 gap-3">{}</div>
                    <p class="pt-1 text-xs text-base-content/60" data-show="{}">{}</p>
                </fieldset>
            </div>
        </fieldset>
    </div>"#,
        render_calculator_panel_legend(lang, "mode", &section_title, Some("fish-fill")),
        escape_html(&fishing_mode_label),
        mode_choices,
        escape_html(&fishing_method_label),
        method_choices,
        escape_html(harpoon_selected),
        escape_html(&harpoon_note),
    )
}

fn calculator_group_label_key(slot_idx: u8) -> Option<&'static str> {
    match slot_idx {
        1 => Some("calculator.server.group.prize"),
        2 => Some("calculator.server.group.rare"),
        3 => Some("calculator.server.group.high_quality"),
        4 => Some("calculator.server.group.general"),
        5 => Some("calculator.server.group.trash"),
        6 => Some("calculator.server.group.harpoon"),
        0 => Some("calculator.breakdown.label.unassigned"),
        _ => None,
    }
}

fn calculator_group_display_label_for_slot(lang: CalculatorLocale, slot_idx: u8) -> Option<String> {
    calculator_group_label_key(slot_idx).map(|key| calculator_route_text(lang, key))
}

fn calculator_group_display_label(lang: CalculatorLocale, label: &str) -> String {
    fish_group_slot_idx(label)
        .and_then(|slot_idx| calculator_group_display_label_for_slot(lang, slot_idx))
        .unwrap_or_else(|| label.to_string())
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
    signals.mastery = signals.mastery.clamp(0.0, 3000.0);
    signals.resources = signals.resources.clamp(0.0, 100.0);
    signals.trade_distance_bonus = signals.trade_distance_bonus.max(0.0);
    signals.trade_price_curve = signals.trade_price_curve.max(0.0);
    signals.catch_time_active = signals.catch_time_active.max(0.0);
    signals.catch_time_afk = signals.catch_time_afk.max(0.0);
    signals.timespan_amount = signals.timespan_amount.max(0.0);
    signals.target_fish_amount = signals.target_fish_amount.max(1.0);
    signals.target_fish_pmf_count = signals.target_fish_pmf_count.max(0.0);
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
    signals.fishing_mode =
        normalize_calculator_fishing_mode(&signals.fishing_mode, &defaults.fishing_mode);
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

    normalize_pet(
        &mut signals.pet1,
        defaults.pet1.clone(),
        &data.catalog.pets,
        &pet_value_aliases,
    );
    normalize_pet(
        &mut signals.pet2,
        defaults.pet2.clone(),
        &data.catalog.pets,
        &pet_value_aliases,
    );
    normalize_pet(
        &mut signals.pet3,
        defaults.pet3.clone(),
        &data.catalog.pets,
        &pet_value_aliases,
    );
    normalize_pet(
        &mut signals.pet4,
        defaults.pet4.clone(),
        &data.catalog.pets,
        &pet_value_aliases,
    );
    normalize_pet(
        &mut signals.pet5,
        defaults.pet5.clone(),
        &data.catalog.pets,
        &pet_value_aliases,
    );
    normalize_pack_leader_selection([
        &mut signals.pet1,
        &mut signals.pet2,
        &mut signals.pet3,
        &mut signals.pet4,
        &mut signals.pet5,
    ]);

    if !matches!(
        signals.timespan_unit.as_str(),
        "minutes" | "hours" | "days" | "weeks"
    ) {
        signals.timespan_unit = defaults.timespan_unit;
    }
}

fn normalize_zone_target_fish(signals: &mut CalculatorSignals, data: &CalculatorData) {
    let valid_target_fish = target_fish_options(data)
        .into_iter()
        .map(|option| option.value.to_string())
        .collect::<HashSet<_>>();

    if signals.target_fish.trim().is_empty() {
        return;
    }

    if !valid_target_fish.contains(signals.target_fish.as_str()) {
        signals.target_fish.clear();
    }
}

fn zone_overlay_for_signals<'a>(
    signals: &'a CalculatorSignals,
    zone_key: &str,
) -> Option<&'a CalculatorZoneOverlaySignals> {
    signals
        .overlay
        .zones
        .get(zone_key)
        .and_then(|zone_overlay| {
            (!zone_overlay.groups.is_empty() || !zone_overlay.items.is_empty())
                .then_some(zone_overlay)
        })
}

fn zone_overlay_has_changes(zone_overlay: Option<&CalculatorZoneOverlaySignals>) -> bool {
    zone_overlay.is_some_and(|zone_overlay| {
        !zone_overlay.groups.is_empty() || !zone_overlay.items.is_empty()
    })
}

fn overlay_editor_default_slot_idx(entry: &CalculatorZoneLootEntry) -> u8 {
    if entry.overlay.added {
        return 0;
    }
    entry
        .evidence
        .iter()
        .find_map(|evidence| evidence.slot_idx)
        .unwrap_or(entry.slot_idx)
}

fn overlay_editor_default_raw_rate_pct(entry: &CalculatorZoneLootEntry) -> f64 {
    if entry.overlay.added {
        return 0.0;
    }
    loot_species_rate_evidence(entry)
        .and_then(|evidence| evidence.rate)
        .map(|rate| (rate * 100.0).max(0.0))
        .unwrap_or_else(|| (entry.within_group_rate * 100.0).max(0.0))
}

fn overlay_editor_percent_value_text(value_pct: f64) -> String {
    let max_decimals = if value_pct.abs() < 0.0001 {
        12
    } else if value_pct.abs() < 0.01 {
        10
    } else if value_pct.abs() < 1.0 {
        8
    } else if value_pct.abs() < 100.0 {
        4
    } else {
        2
    };
    let compact = trim_float_to(value_pct, max_decimals);
    if compact == "0" && value_pct != 0.0 {
        format!("{}%", trim_float_to(value_pct, 14))
    } else {
        format!("{compact}%")
    }
}

fn parse_calculator_condition_comparison<'a>(
    predicate: &'a str,
    subject: &str,
) -> Option<(&'a str, f64)> {
    let rest = predicate.trim().strip_prefix(subject)?.trim();
    for op in [">=", "<=", "==", "!=", ">", "<", "="] {
        if let Some(value) = rest.strip_prefix(op) {
            return value.trim().parse::<f64>().ok().map(|value| (op, value));
        }
    }
    None
}

fn calculator_condition_comparison_matches(actual: f64, op: &str, expected: f64) -> bool {
    match op {
        ">" => actual > expected,
        ">=" => actual >= expected,
        "<" => actual < expected,
        "<=" => actual <= expected,
        "=" | "==" => (actual - expected).abs() <= f64::EPSILON,
        "!=" => (actual - expected).abs() > f64::EPSILON,
        _ => true,
    }
}

#[derive(Debug, Clone)]
struct CalculatorLootBranchOption {
    option_idx: u32,
    conditions: Vec<String>,
}

fn calculator_lifeskill_level_index(signals: &CalculatorSignals) -> f64 {
    signals
        .lifeskill_level
        .trim()
        .parse::<f64>()
        .unwrap_or_default()
        .max(0.0)
}

fn calculator_condition_predicate_matches(predicate: &str, signals: &CalculatorSignals) -> bool {
    let predicate = predicate.trim();
    if predicate.is_empty() {
        return true;
    }
    if let Some((op, value)) = parse_calculator_condition_comparison(predicate, "lifestat(1,1)") {
        return calculator_condition_comparison_matches(signals.mastery.max(0.0), op, value);
    }
    if let Some((op, value)) = parse_calculator_condition_comparison(predicate, "getLifeLevel(1)") {
        return calculator_condition_comparison_matches(
            calculator_lifeskill_level_index(signals),
            op,
            value,
        );
    }

    // Other source predicates are content/global gates, not current calculator user inputs.
    // Keep them active here so only evaluable user-state predicates affect personalized rates.
    true
}

fn calculator_condition_predicate_is_user_state(predicate: &str) -> bool {
    parse_calculator_condition_comparison(predicate, "lifestat(1,1)").is_some()
        || parse_calculator_condition_comparison(predicate, "getLifeLevel(1)").is_some()
}

fn calculator_condition_has_mastery_predicate(condition_raw: &str) -> bool {
    condition_raw.split(';').any(|predicate| {
        parse_calculator_condition_comparison(predicate, "lifestat(1,1)").is_some()
    })
}

fn calculator_condition_has_lifeskill_predicate(condition_raw: &str) -> bool {
    condition_raw.split(';').any(|predicate| {
        parse_calculator_condition_comparison(predicate, "getLifeLevel(1)").is_some()
    })
}

fn calculator_condition_lifeskill_lower_bound(condition_raw: &str) -> Option<f64> {
    condition_raw.split(';').find_map(|predicate| {
        let (op, value) = parse_calculator_condition_comparison(predicate, "getLifeLevel(1)")?;
        matches!(op, ">" | ">=" | "=" | "==").then_some(value)
    })
}

fn calculator_condition_matches(condition_raw: &str, signals: &CalculatorSignals) -> bool {
    condition_raw
        .split(';')
        .all(|predicate| calculator_condition_predicate_matches(predicate, signals))
}

fn calculator_condition_matches_for_forced_branch(
    condition_raw: &str,
    signals: &CalculatorSignals,
) -> bool {
    condition_raw.split(';').all(|predicate| {
        if calculator_condition_predicate_is_user_state(predicate.trim()) {
            true
        } else {
            calculator_condition_predicate_matches(predicate, signals)
        }
    })
}

fn calculator_branch_option_matches(
    option: &CalculatorLootBranchOption,
    signals: &CalculatorSignals,
) -> bool {
    option
        .conditions
        .iter()
        .all(|condition| calculator_condition_matches(condition, signals))
}

fn calculator_branch_option_has_mastery(option: &CalculatorLootBranchOption) -> bool {
    option
        .conditions
        .iter()
        .any(|condition| calculator_condition_has_mastery_predicate(condition))
}

fn calculator_branch_option_has_lifeskill(option: &CalculatorLootBranchOption) -> bool {
    option
        .conditions
        .iter()
        .any(|condition| calculator_condition_has_lifeskill_predicate(condition))
}

fn calculator_branch_option_lifeskill_lower_bound(
    option: &CalculatorLootBranchOption,
) -> Option<f64> {
    option
        .conditions
        .iter()
        .filter_map(|condition| calculator_condition_lifeskill_lower_bound(condition))
        .max_by(|left, right| left.partial_cmp(right).unwrap_or(Ordering::Equal))
}

fn calculator_selected_branch_option_idx(
    options: &[CalculatorLootBranchOption],
    signals: &CalculatorSignals,
) -> Option<u32> {
    let has_user_state_options = options.iter().any(|option| {
        calculator_branch_option_has_mastery(option)
            || calculator_branch_option_has_lifeskill(option)
    });
    if !has_user_state_options {
        return None;
    }

    let mut ordered = options.to_vec();
    ordered.sort_by_key(|option| option.option_idx);
    ordered.dedup_by_key(|option| option.option_idx);

    if let Some(option) = ordered.iter().find(|option| {
        calculator_branch_option_has_mastery(option)
            && calculator_branch_option_matches(option, signals)
    }) {
        return Some(option.option_idx);
    }

    if let Some(option) = ordered
        .iter()
        .filter(|option| {
            calculator_branch_option_has_lifeskill(option)
                && calculator_branch_option_matches(option, signals)
        })
        .max_by(|left, right| {
            calculator_branch_option_lifeskill_lower_bound(left)
                .partial_cmp(&calculator_branch_option_lifeskill_lower_bound(right))
                .unwrap_or(Ordering::Equal)
                .then_with(|| right.option_idx.cmp(&left.option_idx))
        })
    {
        return Some(option.option_idx);
    }

    ordered
        .iter()
        .find(|option| option.conditions.is_empty())
        .map(|option| option.option_idx)
}

fn calculator_loot_branch_options(
    base_entries: &[CalculatorZoneLootEntry],
) -> HashMap<(u8, i64), Vec<CalculatorLootBranchOption>> {
    let mut branch_options = HashMap::<(u8, i64), Vec<CalculatorLootBranchOption>>::new();
    for entry in base_entries {
        for contribution in &entry.rate_contributions {
            let (Some(item_main_group_key), Some(option_idx)) =
                (contribution.item_main_group_key, contribution.option_idx)
            else {
                continue;
            };
            let options = branch_options
                .entry((entry.slot_idx, item_main_group_key))
                .or_default();
            if options.iter().any(|option| option.option_idx == option_idx) {
                continue;
            }
            options.push(CalculatorLootBranchOption {
                option_idx,
                conditions: contribution.group_conditions_raw.clone(),
            });
        }
    }
    branch_options
}

fn calculator_selected_loot_branch_options(
    signals: &CalculatorSignals,
    base_entries: &[CalculatorZoneLootEntry],
) -> HashMap<(u8, i64), u32> {
    calculator_loot_branch_options(base_entries)
        .into_iter()
        .filter_map(|(branch_key, options)| {
            calculator_selected_branch_option_idx(&options, signals)
                .map(|option_idx| (branch_key, option_idx))
        })
        .collect()
}

fn calculator_rate_contribution_active(
    slot_idx: u8,
    contribution: &CalculatorZoneLootRateContribution,
    signals: &CalculatorSignals,
    selected_branch_options: &HashMap<(u8, i64), u32>,
    forced_branch_options: &HashMap<(u8, i64), u32>,
) -> bool {
    if let (Some(item_main_group_key), Some(option_idx)) =
        (contribution.item_main_group_key, contribution.option_idx)
    {
        if let Some(forced_option_idx) = forced_branch_options.get(&(slot_idx, item_main_group_key))
        {
            return option_idx == *forced_option_idx
                && contribution.group_conditions_raw.iter().all(|condition| {
                    calculator_condition_matches_for_forced_branch(condition, signals)
                });
        }
        if let Some(selected_option_idx) =
            selected_branch_options.get(&(slot_idx, item_main_group_key))
        {
            if option_idx != *selected_option_idx {
                return false;
            }
        }
    }

    contribution
        .group_conditions_raw
        .iter()
        .all(|condition| calculator_condition_matches(condition, signals))
}

fn calculator_active_contribution_conditions(
    contributions: &[CalculatorZoneLootRateContribution],
) -> Vec<String> {
    let mut conditions = Vec::new();
    for contribution in contributions {
        for condition in &contribution.group_conditions_raw {
            if !condition.trim().is_empty() && !conditions.contains(condition) {
                conditions.push(condition.clone());
            }
        }
    }
    conditions
}

fn update_calculator_rate_evidence(
    entry: &mut CalculatorZoneLootEntry,
    active_weight: f64,
    total_weight: f64,
) {
    let db_weight = entry
        .rate_contributions
        .iter()
        .filter(|contribution| contribution.source_family == "database")
        .map(|contribution| contribution.weight.max(0.0))
        .sum::<f64>();
    let community_weight = entry
        .rate_contributions
        .iter()
        .filter(|contribution| contribution.source_family == "community")
        .map(|contribution| contribution.weight.max(0.0))
        .sum::<f64>();
    let normalized = if total_weight > 0.0 {
        active_weight / total_weight
    } else {
        0.0
    };
    let unique_subgroup = {
        let mut subgroup_keys = entry
            .rate_contributions
            .iter()
            .filter_map(|contribution| contribution.subgroup_key)
            .collect::<Vec<_>>();
        subgroup_keys.sort_unstable();
        subgroup_keys.dedup();
        (subgroup_keys.len() == 1).then_some(subgroup_keys[0])
    };

    entry.within_group_rate = normalized;
    for evidence in &mut entry.evidence {
        if evidence.source_family == "database" && evidence.claim_kind == "in_group_rate" {
            if db_weight > 0.0 {
                evidence.rate = Some(db_weight / CALCULATOR_COMBINED_GROUP_RATE_SCALE);
                evidence.normalized_rate = Some(db_weight / total_weight);
                evidence.subgroup_key = evidence.subgroup_key.or(unique_subgroup);
            }
        } else if evidence.source_family == "community"
            && evidence.claim_kind == "guessed_in_group_rate"
            && community_weight > 0.0
        {
            evidence.normalized_rate = Some(community_weight / total_weight);
            evidence.subgroup_key = evidence.subgroup_key.or(unique_subgroup);
        }
    }
}

fn apply_calculator_condition_context_to_loot_entries(
    signals: &CalculatorSignals,
    base_entries: &[CalculatorZoneLootEntry],
) -> Vec<CalculatorZoneLootEntry> {
    apply_calculator_condition_context_to_loot_entries_with_branch_overrides(
        signals,
        base_entries,
        &HashMap::new(),
    )
}

fn apply_calculator_condition_context_to_loot_entries_with_branch_overrides(
    signals: &CalculatorSignals,
    base_entries: &[CalculatorZoneLootEntry],
    forced_branch_options: &HashMap<(u8, i64), u32>,
) -> Vec<CalculatorZoneLootEntry> {
    let mut entries_with_weights = Vec::<(CalculatorZoneLootEntry, Option<f64>)>::new();
    let mut slot_totals = HashMap::<u8, f64>::new();
    let mut selected_branch_options =
        calculator_selected_loot_branch_options(signals, base_entries);
    for (branch_key, option_idx) in forced_branch_options {
        selected_branch_options.insert(*branch_key, *option_idx);
    }

    for base_entry in base_entries {
        if base_entry.rate_contributions.is_empty() {
            entries_with_weights.push((base_entry.clone(), None));
            continue;
        }

        let active_contributions = base_entry
            .rate_contributions
            .iter()
            .filter(|contribution| {
                calculator_rate_contribution_active(
                    base_entry.slot_idx,
                    contribution,
                    signals,
                    &selected_branch_options,
                    forced_branch_options,
                )
            })
            .cloned()
            .collect::<Vec<_>>();
        let active_weight = active_contributions
            .iter()
            .map(|contribution| contribution.weight.max(0.0))
            .sum::<f64>();
        if active_weight <= 0.0 {
            continue;
        }

        let mut entry = base_entry.clone();
        entry.group_conditions_raw =
            calculator_active_contribution_conditions(&active_contributions);
        entry.rate_contributions = active_contributions;
        *slot_totals.entry(entry.slot_idx).or_default() += active_weight;
        entries_with_weights.push((entry, Some(active_weight)));
    }

    let mut entries = entries_with_weights
        .into_iter()
        .filter_map(|(mut entry, active_weight)| {
            let Some(active_weight) = active_weight else {
                return Some(entry);
            };
            let total_weight = slot_totals
                .get(&entry.slot_idx)
                .copied()
                .unwrap_or_default();
            if total_weight <= 0.0 {
                return None;
            }
            update_calculator_rate_evidence(&mut entry, active_weight, total_weight);
            Some(entry)
        })
        .collect::<Vec<_>>();

    entries.sort_by(|left, right| {
        zone_loot_slot_sort_key(left.slot_idx)
            .cmp(&zone_loot_slot_sort_key(right.slot_idx))
            .then_with(|| {
                right
                    .within_group_rate
                    .partial_cmp(&left.within_group_rate)
                    .unwrap_or(Ordering::Equal)
            })
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            .then_with(|| left.item_id.cmp(&right.item_id))
    });
    entries
}

fn overlay_editor_group_input_rows(
    row: &FishGroupChartRow,
    lang: CalculatorLocale,
) -> Vec<ComputedStatBreakdownRow> {
    if row.rate_inputs.is_empty() {
        vec![computed_stat_breakdown_row(
            calculator_route_text(lang, "calculator.breakdown.section.inputs"),
            calculator_route_text(lang, "calculator.server.value.unavailable"),
            calculator_route_text(lang, "calculator.overlay.breakdown.detail.no_direct_inputs"),
        )]
    } else {
        row.rate_inputs.clone()
    }
}

fn overlay_editor_group_bonus_breakdown(
    row: &FishGroupChartRow,
    lang: CalculatorLocale,
    current_present: bool,
    current_raw_rate_pct: f64,
    bonus_rate_pct: f64,
    effective_raw_weight_pct: f64,
    normalized_share_pct: f64,
) -> String {
    let text = |key: &str| calculator_route_text(lang, key);
    let text_with_vars =
        |key: &str, vars: &[(&str, &str)]| calculator_route_text_with_vars(lang, key, vars);
    let breakdown_inputs = text("calculator.breakdown.section.inputs");
    let breakdown_composition = text("calculator.breakdown.section.composition");
    let group_label = calculator_group_display_label(lang, &row.label);
    let summary_text = if current_present {
        if bonus_rate_pct > 0.0 {
            text("calculator.overlay.breakdown.summary.accrued_bonus.active.some")
        } else {
            text("calculator.overlay.breakdown.summary.accrued_bonus.active.none")
        }
    } else {
        text("calculator.overlay.breakdown.summary.accrued_bonus.inactive")
    };
    stat_breakdown_json(
        computed_stat_breakdown(
            text_with_vars(
                "calculator.overlay.breakdown.title.accrued_bonus",
                &[("group", &group_label)],
            ),
            overlay_editor_percent_value_text(bonus_rate_pct),
            summary_text,
            text("calculator.overlay.breakdown.formula.accrued_bonus"),
            vec![
                computed_stat_breakdown_section(
                    breakdown_inputs,
                    overlay_editor_group_input_rows(row, lang),
                ),
                computed_stat_breakdown_section(
                    breakdown_composition,
                    vec![
                        computed_stat_breakdown_row(
                            text("calculator.overlay.breakdown.label.current_raw_base_rate"),
                            overlay_editor_percent_value_text(current_raw_rate_pct),
                            text("calculator.overlay.breakdown.detail.current_raw_base_rate.overlay"),
                        ),
                        computed_stat_breakdown_row(
                            text("calculator.overlay.breakdown.label.effective_raw_weight"),
                            overlay_editor_percent_value_text(effective_raw_weight_pct),
                            if current_present {
                                text("calculator.overlay.breakdown.detail.effective_raw_weight.active")
                            } else {
                                text(
                                    "calculator.overlay.breakdown.detail.effective_raw_weight.inactive",
                                )
                            },
                        ),
                        computed_stat_breakdown_row(
                            text("calculator.overlay.breakdown.label.accrued_bonus"),
                            overlay_editor_percent_value_text(bonus_rate_pct),
                            if bonus_rate_pct > 0.0 {
                                text("calculator.overlay.breakdown.detail.accrued_bonus.some")
                            } else {
                                text("calculator.overlay.breakdown.detail.accrued_bonus.none")
                            },
                        ),
                        computed_stat_breakdown_row(
                            text("calculator.overlay.breakdown.label.normalized_share"),
                            overlay_editor_percent_value_text(normalized_share_pct),
                            text("calculator.overlay.breakdown.detail.normalized_share.reference"),
                        ),
                    ],
                ),
            ],
        )
        .with_formula_terms(vec![
            computed_stat_formula_term(
                text("calculator.overlay.breakdown.label.accrued_bonus"),
                overlay_editor_percent_value_text(bonus_rate_pct),
            ),
            computed_stat_formula_term(
                text("calculator.overlay.breakdown.label.effective_raw_weight"),
                overlay_editor_percent_value_text(effective_raw_weight_pct),
            ),
            computed_stat_formula_term(
                text("calculator.overlay.breakdown.label.current_raw_base_rate"),
                overlay_editor_percent_value_text(current_raw_rate_pct),
            ),
        ]),
    )
}

fn overlay_editor_group_normalized_breakdown(
    row: &FishGroupChartRow,
    lang: CalculatorLocale,
    current_present: bool,
    current_raw_rate_pct: f64,
    bonus_rate_pct: f64,
    effective_raw_weight_pct: f64,
    total_effective_raw_weight_pct: f64,
    normalized_share_pct: f64,
) -> String {
    let text = |key: &str| calculator_route_text(lang, key);
    let text_with_vars =
        |key: &str, vars: &[(&str, &str)]| calculator_route_text_with_vars(lang, key, vars);
    let breakdown_inputs = text("calculator.breakdown.section.inputs");
    let breakdown_composition = text("calculator.breakdown.section.composition");
    let group_label = calculator_group_display_label(lang, &row.label);
    let summary_text = if current_present {
        text("calculator.overlay.breakdown.summary.normalized_share.active")
    } else {
        text("calculator.overlay.breakdown.summary.normalized_share.inactive")
    };
    stat_breakdown_json(
        computed_stat_breakdown(
            text_with_vars(
                "calculator.overlay.breakdown.title.normalized_share",
                &[("group", &group_label)],
            ),
            overlay_editor_percent_value_text(normalized_share_pct),
            summary_text,
            text("calculator.overlay.breakdown.formula.normalized_share"),
            vec![
                computed_stat_breakdown_section(
                    breakdown_inputs,
                    overlay_editor_group_input_rows(row, lang),
                ),
                computed_stat_breakdown_section(
                    breakdown_composition,
                    vec![
                        computed_stat_breakdown_row(
                            text("calculator.overlay.breakdown.label.current_raw_base_rate"),
                            overlay_editor_percent_value_text(current_raw_rate_pct),
                            text(
                                "calculator.overlay.breakdown.detail.current_raw_base_rate.before_bonuses",
                            ),
                        ),
                        computed_stat_breakdown_row(
                            text("calculator.overlay.breakdown.label.accrued_bonus"),
                            overlay_editor_percent_value_text(bonus_rate_pct),
                            text("calculator.overlay.breakdown.detail.accrued_bonus.before_normalization"),
                        ),
                        computed_stat_breakdown_row(
                            text("calculator.overlay.breakdown.label.effective_raw_weight"),
                            overlay_editor_percent_value_text(effective_raw_weight_pct),
                            if current_present {
                                text("calculator.overlay.breakdown.detail.effective_raw_weight.combined")
                            } else {
                                text(
                                    "calculator.overlay.breakdown.detail.effective_raw_weight.inactive",
                                )
                            },
                        ),
                        computed_stat_breakdown_row(
                            text("calculator.overlay.breakdown.label.all_effective_raw_weights"),
                            overlay_editor_percent_value_text(total_effective_raw_weight_pct),
                            text(
                                "calculator.overlay.breakdown.detail.all_effective_raw_weights.denominator",
                            ),
                        ),
                        computed_stat_breakdown_row(
                            text("calculator.overlay.breakdown.label.normalized_share"),
                            overlay_editor_percent_value_text(normalized_share_pct),
                            text("calculator.overlay.breakdown.detail.normalized_share.final"),
                        ),
                    ],
                ),
            ],
        )
        .with_formula_terms(vec![
            computed_stat_formula_term(
                text("calculator.overlay.breakdown.label.normalized_share"),
                overlay_editor_percent_value_text(normalized_share_pct),
            ),
            computed_stat_formula_term(
                text("calculator.overlay.breakdown.label.effective_raw_weight"),
                overlay_editor_percent_value_text(effective_raw_weight_pct),
            ),
            computed_stat_formula_term(
                text("calculator.overlay.breakdown.label.all_effective_raw_weights"),
                overlay_editor_percent_value_text(total_effective_raw_weight_pct),
            ),
        ]),
    )
}

fn build_overlay_editor_signal(
    signals: &CalculatorSignals,
    data: &CalculatorData,
    fish_group_chart: &FishGroupChart,
) -> CalculatorOverlayEditorSignal {
    let zone_overlay = signals.overlay.zones.get(&signals.zone);
    let zone_supports_prize_group = data
        .zone_group_rates
        .get(&signals.zone)
        .and_then(|zone_group_rate| zone_group_rate.prize_main_group_key)
        .is_some();
    let total_effective_raw_weight_pct = fish_group_chart
        .rows
        .iter()
        .map(|row| row.weight_pct.max(0.0))
        .sum::<f64>();
    let group_rows = fish_group_chart
        .rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let slot_idx = (index + 1) as u8;
            let default_raw_rate_pct = row.base_share_pct.max(0.0);
            let group_overlay = zone_overlay
                .and_then(|zone_overlay| zone_overlay.groups.get(slot_idx.to_string().as_str()));
            let default_present = default_raw_rate_pct > 0.0
                || (slot_idx == 1 && zone_supports_prize_group)
                || data.zone_loot_entries.iter().any(|entry| {
                    !entry.overlay.added && overlay_editor_default_slot_idx(entry) == slot_idx
                });
            let current_present = group_overlay
                .and_then(|group_overlay| group_overlay.present)
                .unwrap_or(default_present);
            let current_raw_rate_pct = group_overlay
                .and_then(|group_overlay| group_overlay.raw_rate_percent)
                .map(|value| value.max(0.0))
                .unwrap_or(default_raw_rate_pct);
            let effective_raw_weight_pct = if current_present {
                row.weight_pct.max(0.0)
            } else {
                0.0
            };
            let normalized_share_pct = if current_present {
                row.current_share_pct.max(0.0)
            } else {
                0.0
            };
            let bonus_rate_pct = if current_present {
                (effective_raw_weight_pct - current_raw_rate_pct.max(0.0)).max(0.0)
            } else {
                0.0
            };
            CalculatorOverlayEditorGroupRow {
                slot_idx,
                label: calculator_group_display_label(data.lang, &row.label),
                default_present,
                default_raw_rate_pct,
                default_raw_rate_text: overlay_editor_percent_value_text(default_raw_rate_pct),
                current_raw_rate_pct,
                current_raw_rate_text: overlay_editor_percent_value_text(current_raw_rate_pct),
                bonus_rate_pct,
                bonus_rate_text: overlay_editor_percent_value_text(bonus_rate_pct),
                effective_raw_weight_pct,
                effective_raw_weight_text: overlay_editor_percent_value_text(
                    effective_raw_weight_pct,
                ),
                normalized_share_pct,
                normalized_share_text: overlay_editor_percent_value_text(normalized_share_pct),
                bonus_rate_breakdown: overlay_editor_group_bonus_breakdown(
                    row,
                    data.lang,
                    current_present,
                    current_raw_rate_pct,
                    bonus_rate_pct,
                    effective_raw_weight_pct,
                    normalized_share_pct,
                ),
                normalized_share_breakdown: overlay_editor_group_normalized_breakdown(
                    row,
                    data.lang,
                    current_present,
                    current_raw_rate_pct,
                    bonus_rate_pct,
                    effective_raw_weight_pct,
                    total_effective_raw_weight_pct,
                    normalized_share_pct,
                ),
            }
        })
        .collect::<Vec<_>>();
    let mut item_rows = data
        .zone_loot_entries
        .iter()
        .map(|entry| {
            let default_slot_idx = overlay_editor_default_slot_idx(entry);
            let default_raw_rate_pct = overlay_editor_default_raw_rate_pct(entry);
            CalculatorOverlayEditorItemRow {
                item_id: entry.item_id,
                default_present: !entry.overlay.added,
                overlay_added: entry.overlay.added,
                slot_idx: default_slot_idx,
                group_label: calculator_group_display_label_for_slot(data.lang, default_slot_idx)
                    .unwrap_or_else(|| {
                        calculator_route_text(data.lang, "calculator.breakdown.label.unassigned")
                    }),
                label: entry.name.clone(),
                icon_url: entry
                    .icon
                    .as_deref()
                    .map(|icon| absolute_public_asset_url(data.cdn_base_url.as_str(), icon)),
                icon_grade_tone: item_grade_tone(entry.grade.as_deref()).to_string(),
                default_raw_rate_pct,
                default_raw_rate_text: overlay_editor_percent_value_text(default_raw_rate_pct),
                normalized_rate_pct: (entry.within_group_rate * 100.0).max(0.0),
                normalized_rate_text: overlay_editor_percent_value_text(
                    (entry.within_group_rate * 100.0).max(0.0),
                ),
                base_price_raw: entry.vendor_price.unwrap_or_default() as f64,
                base_price_text: fmt_silver(entry.vendor_price.unwrap_or_default() as f64),
                is_fish: entry.is_fish,
            }
        })
        .collect::<Vec<_>>();
    item_rows.sort_by(|left, right| {
        zone_loot_slot_sort_key(left.slot_idx)
            .cmp(&zone_loot_slot_sort_key(right.slot_idx))
            .then_with(|| {
                right
                    .normalized_rate_pct
                    .partial_cmp(&left.normalized_rate_pct)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| left.label.to_lowercase().cmp(&right.label.to_lowercase()))
            .then_with(|| left.item_id.cmp(&right.item_id))
    });

    CalculatorOverlayEditorSignal {
        zone_rgb_key: signals.zone.clone(),
        zone_name: data
            .zones
            .iter()
            .find(|zone| zone.rgb_key.to_string() == signals.zone)
            .and_then(|zone| zone.name.clone())
            .unwrap_or_else(|| signals.zone.clone()),
        groups: group_rows,
        items: item_rows,
    }
}

fn normalize_raw_pct_values<K>(
    base_pct_by_key: &HashMap<K, f64>,
    explicit_pct_by_key: &HashMap<K, f64>,
    active_keys: &HashSet<K>,
) -> HashMap<K, f64>
where
    K: Copy + Eq + std::hash::Hash,
{
    let mut effective = HashMap::new();
    if active_keys.is_empty() {
        return effective;
    }

    let mut total_raw_pct = 0.0;
    for key in active_keys {
        let raw_pct = explicit_pct_by_key
            .get(key)
            .copied()
            .or_else(|| base_pct_by_key.get(key).copied())
            .unwrap_or_default()
            .max(0.0);
        total_raw_pct += raw_pct;
        effective.insert(*key, raw_pct);
    }

    if total_raw_pct > 0.0 {
        for value in effective.values_mut() {
            *value = (*value / total_raw_pct) * 100.0;
        }
    } else {
        for value in effective.values_mut() {
            *value = 0.0;
        }
    }

    effective
}

fn apply_zone_overlay_to_loot_entries(
    signals: &CalculatorSignals,
    zone_key: &str,
    base_entries: &[CalculatorZoneLootEntry],
) -> Vec<CalculatorZoneLootEntry> {
    let Some(zone_overlay) = zone_overlay_for_signals(signals, zone_key) else {
        return base_entries.to_vec();
    };

    let removed_group_slots = zone_overlay
        .groups
        .iter()
        .filter_map(|(slot_key, group_overlay)| {
            (group_overlay.present == Some(false))
                .then(|| slot_key.parse::<u8>().ok())
                .flatten()
        })
        .collect::<HashSet<_>>();
    let mut slot_overlay_active = removed_group_slots.clone();
    let mut matched_item_ids = HashSet::new();
    let mut entries = Vec::with_capacity(base_entries.len() + zone_overlay.items.len());

    for base_entry in base_entries {
        let item_key = base_entry.item_id.to_string();
        let overlay_item = zone_overlay.items.get(&item_key);
        if overlay_item.is_some() {
            matched_item_ids.insert(item_key.clone());
        }
        if overlay_item.and_then(|item| item.present) == Some(false) {
            slot_overlay_active.insert(base_entry.slot_idx);
            continue;
        }

        let mut entry = base_entry.clone();
        if let Some(overlay_item) = overlay_item {
            if let Some(slot_idx) = overlay_item
                .slot_idx
                .filter(|slot_idx| (1..=5).contains(slot_idx))
            {
                if slot_idx != entry.slot_idx {
                    slot_overlay_active.insert(entry.slot_idx);
                    slot_overlay_active.insert(slot_idx);
                    entry.slot_idx = slot_idx;
                }
            }
            if let Some(rate_percent) = overlay_item.raw_rate_percent {
                slot_overlay_active.insert(entry.slot_idx);
                entry.overlay.explicit_rate_percent = Some(rate_percent);
            }
            if let Some(name) = overlay_item
                .name
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                entry.name = name.to_string();
            }
            if let Some(grade) = overlay_item
                .grade
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                entry.grade = Some(grade.to_string());
            }
            if let Some(is_fish) = overlay_item.is_fish {
                entry.is_fish = is_fish;
            }
        }
        entries.push(entry);
    }

    for (item_key, overlay_item) in &zone_overlay.items {
        if matched_item_ids.contains(item_key) || overlay_item.present == Some(false) {
            continue;
        }
        let item_id = match item_key.parse::<i32>() {
            Ok(item_id) if item_id > 0 => item_id,
            _ => continue,
        };
        let Some(slot_idx) = overlay_item
            .slot_idx
            .filter(|slot_idx| (1..=5).contains(slot_idx))
        else {
            continue;
        };
        let Some(rate_percent) = overlay_item.raw_rate_percent else {
            continue;
        };
        slot_overlay_active.insert(slot_idx);
        let name = overlay_item
            .name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| item_id.to_string());
        entries.push(CalculatorZoneLootEntry {
            slot_idx,
            item_id,
            name,
            icon: Some(format!("/images/items/{item_id:08}.webp")),
            vendor_price: None,
            grade: overlay_item
                .grade
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned),
            is_fish: overlay_item.is_fish.unwrap_or(true),
            catch_methods: vec!["rod".to_string()],
            group_conditions_raw: Vec::new(),
            within_group_rate: 0.0,
            rate_contributions: Vec::new(),
            evidence: Vec::new(),
            overlay: CalculatorZoneLootOverlayMeta {
                added: true,
                slot_overlay_active: true,
                explicit_rate_percent: Some(rate_percent),
            },
        });
    }

    entries.retain(|entry| !removed_group_slots.contains(&entry.slot_idx));

    let mut effective_by_slot_and_item = HashMap::<(u8, i32), f64>::new();
    let mut item_ids_by_slot = HashMap::<u8, Vec<i32>>::new();
    let mut base_pct_by_slot_and_item = HashMap::<(u8, i32), f64>::new();
    let mut explicit_pct_by_slot_and_item = HashMap::<(u8, i32), f64>::new();
    for entry in &entries {
        let key = (entry.slot_idx, entry.item_id);
        item_ids_by_slot
            .entry(entry.slot_idx)
            .or_default()
            .push(entry.item_id);
        base_pct_by_slot_and_item.insert(key, overlay_editor_default_raw_rate_pct(entry));
        if let Some(rate_percent) = entry.overlay.explicit_rate_percent {
            explicit_pct_by_slot_and_item.insert(key, rate_percent.max(0.0));
        }
    }

    for (slot_idx, item_ids) in item_ids_by_slot {
        let active_keys = item_ids
            .into_iter()
            .map(|item_id| (slot_idx, item_id))
            .collect::<HashSet<_>>();
        let effective_pct = normalize_raw_pct_values(
            &base_pct_by_slot_and_item,
            &explicit_pct_by_slot_and_item,
            &active_keys,
        );
        for (key, value) in effective_pct {
            effective_by_slot_and_item.insert(key, value / 100.0);
        }
    }

    for entry in &mut entries {
        entry.within_group_rate = effective_by_slot_and_item
            .get(&(entry.slot_idx, entry.item_id))
            .copied()
            .unwrap_or_default();
        entry.overlay.slot_overlay_active = slot_overlay_active.contains(&entry.slot_idx);
    }

    entries.sort_by(|left, right| {
        left.slot_idx
            .cmp(&right.slot_idx)
            .then_with(|| {
                right
                    .within_group_rate
                    .partial_cmp(&left.within_group_rate)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            .then_with(|| left.item_id.cmp(&right.item_id))
    });
    entries
}

const LEGACY_PET_SKILL_FISHING_EXP: &str = "legacy:pet:skill:fishing_exp";

#[derive(Default)]
struct CalculatorPetAliasIndex {
    pets: HashMap<String, String>,
    options: HashMap<String, String>,
}

fn build_pet_value_aliases(catalog: &CalculatorPetCatalog) -> CalculatorPetAliasIndex {
    let mut aliases = CalculatorPetAliasIndex::default();
    for pet in &catalog.pets {
        if pet.key.is_empty() {
            continue;
        }
        aliases
            .pets
            .insert(normalize_lookup_value(&pet.key), pet.key.clone());
        for alias_key in &pet.alias_keys {
            if alias_key.trim().is_empty() {
                continue;
            }
            aliases
                .pets
                .insert(normalize_lookup_value(alias_key), pet.key.clone());
        }
        aliases
            .pets
            .entry(normalize_lookup_value(&pet.label))
            .or_insert_with(|| pet.key.clone());
    }
    for option in catalog
        .specials
        .iter()
        .chain(catalog.talents.iter())
        .chain(catalog.skills.iter())
    {
        if option.key.is_empty() {
            continue;
        }
        aliases
            .options
            .insert(normalize_lookup_value(&option.label), option.key.clone());
        aliases
            .options
            .insert(normalize_lookup_value(&option.key), option.key.clone());
        for english_alias in [
            option
                .auto_fishing_time_reduction
                .map(|_| "Auto-Fishing Time Reduction"),
            option
                .durability_reduction_resistance
                .map(|_| "Durability Reduction Resistance"),
            option.life_exp.map(|_| "Life EXP"),
            option.fishing_exp.map(|_| "Fishing EXP"),
        ]
        .into_iter()
        .flatten()
        {
            aliases
                .options
                .entry(normalize_lookup_value(english_alias))
                .or_insert_with(|| option.key.clone());
        }
    }
    aliases
        .options
        .entry(normalize_lookup_value("Fishing EXP"))
        .or_insert_with(|| LEGACY_PET_SKILL_FISHING_EXP.to_string());
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
    signals: &CalculatorSignals,
) -> AppResult<serde_json::Map<String, Value>> {
    let mut patch = signals_patch_map(signals)?;
    mirror_resources_signal(&mut patch);
    patch_checkbox_transport_signals(signals, &mut patch);
    Ok(patch)
}

fn default_reset_signals_patch_map(
    defaults: &CalculatorSignals,
) -> AppResult<serde_json::Map<String, Value>> {
    let mut patch = init_signals_patch_map(defaults)?;
    patch.insert(
        "_calculator_ui".to_string(),
        json!({
            "top_level_tab": "mode",
            "distribution_tab": "groups",
            "pinned_layout": [[["overview"]], [["zone"], ["session"]], [["bite_time"], ["loot"]]],
            "pinned_sections": ["overview", "zone", "session", "bite_time", "loot"],
            "unpinned_insert_index": [0, 0],
        }),
    );
    Ok(patch)
}

fn mirror_resources_signal(patch: &mut serde_json::Map<String, Value>) {
    if let Some(value) = patch.get("resources").cloned() {
        patch.insert("_resources".to_string(), value);
    }
}

fn patch_checkbox_transport_signals(
    signals: &CalculatorSignals,
    patch: &mut serde_json::Map<String, Value>,
) {
    patch.insert(
        "_outfit_slots".to_string(),
        Value::Array(
            signals
                .outfit
                .iter()
                .cloned()
                .map(Value::String)
                .collect::<Vec<_>>(),
        ),
    );
    patch.insert(
        "_food_slots".to_string(),
        Value::Array(
            signals
                .food
                .iter()
                .cloned()
                .map(Value::String)
                .collect::<Vec<_>>(),
        ),
    );
    patch.insert(
        "_buff_slots".to_string(),
        Value::Array(
            signals
                .buff
                .iter()
                .cloned()
                .map(Value::String)
                .collect::<Vec<_>>(),
        ),
    );

    for (slot, pet) in [
        ("pet1", &signals.pet1),
        ("pet2", &signals.pet2),
        ("pet3", &signals.pet3),
        ("pet4", &signals.pet4),
        ("pet5", &signals.pet5),
    ] {
        patch.insert(
            format!("_{slot}_skill_slots"),
            Value::Array(
                pet.skills
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect::<Vec<_>>(),
            ),
        );
        for index in 0..3 {
            patch.insert(
                format!("_{slot}_skill_slot{}", index + 1),
                Value::String(pet.skills.get(index).cloned().unwrap_or_default()),
            );
        }
    }
}

fn render_canonical_checkbox_signal_computeds(pet_slots: usize) -> String {
    let mut html = String::from(
        r#"<div class="hidden"
         data-computed:resources="$_resources"
         data-computed:outfit="Array.isArray($_outfit_slots) ? $_outfit_slots : []"
         data-computed:food="Array.isArray($_food_slots) ? $_food_slots : []"
         data-computed:buff="Array.isArray($_buff_slots) ? $_buff_slots : []""#,
    );
    for slot in 1..=pet_slots.max(1) {
        write!(
            html,
            r#"
         data-computed:pet{slot}.skills="window.__fishystuffCalculator.petSkillSlots($pet{slot}.tier, $_pet{slot}_skill_slot1, $_pet{slot}_skill_slot2, $_pet{slot}_skill_slot3)""#,
        )
        .unwrap();
    }
    html.push_str(
        r#"
         data-computed:_live="window.__fishystuffCalculator.liveCalc($level, $_resources, window.__fishystuffCalculator.effectiveActivity($fishingMode, $active), $catchTimeActive, $catchTimeAfk, $timespanAmount, $timespanUnit, $_calc)"></div>"#,
    );
    html
}

fn normalize_pet(
    pet: &mut CalculatorPetSignals,
    defaults: CalculatorPetSignals,
    catalog: &CalculatorPetCatalog,
    aliases: &CalculatorPetAliasIndex,
) {
    pet.pet = normalize_pet_lookup_value(&pet.pet, &aliases.pets);
    pet.tier = pet
        .tier
        .trim()
        .parse::<i32>()
        .ok()
        .map(|tier| tier.clamp(1, 5).to_string())
        .or_else(|| (!defaults.tier.trim().is_empty()).then(|| defaults.tier.clone()))
        .unwrap_or_default();
    pet.special = normalize_pet_lookup_value(&pet.special, &aliases.options);
    pet.talent = normalize_pet_lookup_value(&pet.talent, &aliases.options);
    pet.skills = pet
        .skills
        .iter()
        .map(|value| normalize_pet_lookup_value(value, &aliases.options))
        .filter(|value| !value.is_empty())
        .collect();
    pet.skills = std::mem::take(&mut pet.skills)
        .into_iter()
        .fold(Vec::new(), |mut out, value| {
            if !out.iter().any(|existing| existing == &value) {
                out.push(value);
            }
            out
        });

    if pet.pet.trim().is_empty() {
        pet.pack_leader = false;
        pet.special.clear();
        pet.talent.clear();
        pet.skills.clear();
        return;
    }

    let Some(selected_pet_entry) = catalog.pets.iter().find(|entry| entry.key == pet.pet) else {
        pet.pet.clear();
        pet.pack_leader = false;
        pet.special.clear();
        pet.talent.clear();
        pet.skills.clear();
        return;
    };

    let pet_entry = if pet_entry_has_tier(selected_pet_entry, &pet.tier) {
        selected_pet_entry
    } else if let Some(lineage_entry) =
        find_pet_lineage_entry_for_tier(catalog, selected_pet_entry, &pet.tier)
    {
        pet.pet = lineage_entry.key.clone();
        lineage_entry
    } else {
        pet.pet.clear();
        pet.pack_leader = false;
        pet.special.clear();
        pet.talent.clear();
        pet.skills.clear();
        return;
    };

    let Some(mut tier_entry) = pet_entry.tiers.iter().find(|tier| tier.key == pet.tier) else {
        pet.pet.clear();
        pet.pack_leader = false;
        pet.special.clear();
        pet.talent.clear();
        pet.skills.clear();
        return;
    };

    if let Some((variant_entry, variant_tier)) = find_pet_same_tier_variant_for_requested_options(
        catalog,
        pet_entry,
        &pet.tier,
        &pet.special,
        &pet.talent,
    ) {
        pet.pet = variant_entry.key.clone();
        tier_entry = variant_tier;
    }

    pet.special = resolve_fixed_pet_option_selection(&tier_entry.specials);
    pet.talent = resolve_fixed_pet_option_selection(&tier_entry.talents);
    pet.skills = resolve_pet_skill_selection(
        &pet.skills,
        &tier_entry.skills,
        &catalog.skills,
        pet_skill_limit_for_tier_key(&tier_entry.key),
    );
    if tier_entry.key.trim() != "5" {
        pet.pack_leader = false;
    }
}

fn normalize_pack_leader_selection(mut pets: [&mut CalculatorPetSignals; 5]) {
    let first_selected = pets
        .iter()
        .position(|pet| pet.pack_leader && !pet.pet.trim().is_empty() && pet.tier.trim() == "5");
    for (index, pet) in pets.iter_mut().enumerate() {
        pet.pack_leader = matches!(first_selected, Some(selected) if selected == index);
    }
}

fn normalize_pet_lookup_value(value: &str, aliases: &HashMap<String, String>) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let normalized = normalize_lookup_value(trimmed);
    aliases
        .get(&normalized)
        .cloned()
        .unwrap_or_else(|| trimmed.to_string())
}

fn pet_entry_has_tier(entry: &CalculatorPetEntry, tier_key: &str) -> bool {
    entry.tiers.iter().any(|tier| tier.key == tier_key)
}

fn pet_entries_share_lineage(left: &CalculatorPetEntry, right: &CalculatorPetEntry) -> bool {
    !left.lineage_keys.is_empty()
        && left.lineage_keys.iter().any(|left_key| {
            right
                .lineage_keys
                .iter()
                .any(|right_key| right_key == left_key)
        })
}

fn pet_entries_share_variant_group(left: &CalculatorPetEntry, right: &CalculatorPetEntry) -> bool {
    !left.variant_group_keys.is_empty()
        && left.variant_group_keys.iter().any(|left_key| {
            right
                .variant_group_keys
                .iter()
                .any(|right_key| right_key == left_key)
        })
}

fn find_pet_lineage_entry_for_tier<'a>(
    catalog: &'a CalculatorPetCatalog,
    selected_pet: &CalculatorPetEntry,
    tier_key: &str,
) -> Option<&'a CalculatorPetEntry> {
    catalog
        .pets
        .iter()
        .filter(|entry| entry.key != selected_pet.key)
        .filter(|entry| pet_entry_has_tier(entry, tier_key))
        .filter(|entry| pet_entries_share_lineage(selected_pet, entry))
        .min_by(|left, right| {
            left.label
                .cmp(&right.label)
                .then_with(|| left.key.cmp(&right.key))
        })
}

fn pet_tier_has_fixed_option(
    tier: &CalculatorPetTierEntry,
    kind: PetFixedOptionKind,
    option_key: &str,
) -> bool {
    if option_key.trim().is_empty() {
        return false;
    }
    match kind {
        PetFixedOptionKind::Special => tier.specials.iter().any(|key| key == option_key),
        PetFixedOptionKind::Talent => tier.talents.iter().any(|key| key == option_key),
    }
}

fn pet_same_tier_variant_entries<'a>(
    catalog: &'a CalculatorPetCatalog,
    selected_pet: &'a CalculatorPetEntry,
    tier_key: &str,
) -> Vec<(&'a CalculatorPetEntry, &'a CalculatorPetTierEntry)> {
    catalog
        .pets
        .iter()
        .filter(|entry| {
            entry.key == selected_pet.key || pet_entries_share_variant_group(selected_pet, entry)
        })
        .filter_map(|entry| {
            entry
                .tiers
                .iter()
                .find(|tier| tier.key == tier_key)
                .map(|tier| (entry, tier))
        })
        .collect()
}

fn find_pet_same_tier_variant_for_requested_options<'a>(
    catalog: &'a CalculatorPetCatalog,
    selected_pet: &'a CalculatorPetEntry,
    tier_key: &str,
    requested_special: &str,
    requested_talent: &str,
) -> Option<(&'a CalculatorPetEntry, &'a CalculatorPetTierEntry)> {
    let current_tier = selected_pet
        .tiers
        .iter()
        .find(|tier| tier.key == tier_key)?;
    let needs_special = !requested_special.trim().is_empty()
        && !pet_tier_has_fixed_option(current_tier, PetFixedOptionKind::Special, requested_special);
    let needs_talent = !requested_talent.trim().is_empty()
        && !pet_tier_has_fixed_option(current_tier, PetFixedOptionKind::Talent, requested_talent);
    if !needs_special && !needs_talent {
        return None;
    }

    pet_same_tier_variant_entries(catalog, selected_pet, tier_key)
        .into_iter()
        .filter(|(entry, _)| entry.key != selected_pet.key)
        .filter(|(_, tier)| {
            (!needs_special
                || pet_tier_has_fixed_option(tier, PetFixedOptionKind::Special, requested_special))
                && (!needs_talent
                    || pet_tier_has_fixed_option(
                        tier,
                        PetFixedOptionKind::Talent,
                        requested_talent,
                    ))
        })
        .max_by(|(left_entry, left_tier), (right_entry, right_tier)| {
            let left_score = usize::from(pet_tier_has_fixed_option(
                left_tier,
                PetFixedOptionKind::Special,
                requested_special,
            )) + usize::from(pet_tier_has_fixed_option(
                left_tier,
                PetFixedOptionKind::Talent,
                requested_talent,
            ));
            let right_score = usize::from(pet_tier_has_fixed_option(
                right_tier,
                PetFixedOptionKind::Special,
                requested_special,
            )) + usize::from(pet_tier_has_fixed_option(
                right_tier,
                PetFixedOptionKind::Talent,
                requested_talent,
            ));
            left_score
                .cmp(&right_score)
                .then_with(|| right_entry.label.cmp(&left_entry.label))
                .then_with(|| right_entry.key.cmp(&left_entry.key))
        })
}

fn pet_option_by_key<'a>(
    options: &'a [CalculatorPetOptionEntry],
    key: &str,
) -> Option<&'a CalculatorPetOptionEntry> {
    options.iter().find(|option| option.key == key)
}

fn resolve_fixed_pet_option_selection(allowed_keys: &[String]) -> String {
    if allowed_keys.is_empty() {
        return String::new();
    }
    allowed_keys[0].clone()
}

fn resolve_pet_skill_selection(
    current_values: &[String],
    allowed_keys: &[String],
    options: &[CalculatorPetOptionEntry],
    skill_limit: usize,
) -> Vec<String> {
    let skill_limit = skill_limit.max(1);
    let allowed_lookup = allowed_keys.iter().collect::<HashSet<_>>();
    let mut selected = current_values
        .iter()
        .filter(|key| allowed_lookup.contains(key))
        .cloned()
        .collect::<Vec<_>>();
    if selected.is_empty()
        && current_values
            .iter()
            .any(|value| value == LEGACY_PET_SKILL_FISHING_EXP)
    {
        if let Some(best) = allowed_keys
            .iter()
            .filter_map(|key| {
                pet_option_by_key(options, key).map(|option| {
                    (
                        key,
                        option.fishing_exp.unwrap_or_default(),
                        option.label.as_str(),
                    )
                })
            })
            .max_by(
                |(left_key, left_score, left_label), (right_key, right_score, right_label)| {
                    left_score
                        .partial_cmp(right_score)
                        .unwrap_or(Ordering::Equal)
                        .then_with(|| left_label.cmp(right_label))
                        .then_with(|| left_key.cmp(right_key))
                },
            )
            .map(|(key, _, _)| key.clone())
        {
            selected.push(best);
        }
    }
    if selected.is_empty() && !allowed_keys.is_empty() {
        selected.push(allowed_keys[0].clone());
    }
    selected.truncate(skill_limit);
    selected
}

fn pet_skill_limit_for_tier_key(tier_key: &str) -> usize {
    match tier_key.trim() {
        "1" | "2" => 1,
        "3" => 2,
        "4" | "5" => 3,
        _ => 1,
    }
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
    if values.iter().all(|value| value.trim().is_empty()) {
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

    let pets = [
        &signals.pet1,
        &signals.pet2,
        &signals.pet3,
        &signals.pet4,
        &signals.pet5,
    ];
    let pet_afr_max = pets
        .iter()
        .map(|pet| pet_afr(pet, &data.catalog.pets))
        .fold(0.0_f64, f64::max);
    let pet_drr_sum = pets
        .iter()
        .map(|pet| pet_drr(pet, &data.catalog.pets))
        .sum::<f64>();
    let pet_fishing_exp = pets
        .iter()
        .map(|pet| pet_fishing_exp(pet, &data.catalog.pets))
        .sum::<f64>();
    let pet_life_exp = pets
        .iter()
        .map(|pet| pet_life_exp(pet, &data.catalog.pets))
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

    let active = calculator_effective_active(&signals.fishing_mode, signals.active);
    let catch_time_active_raw = signals.catch_time_active.max(0.0);
    let catch_time_afk_raw = signals.catch_time_afk.max(0.0);
    let catch_time_raw = if active {
        catch_time_active_raw
    } else {
        catch_time_afk_raw
    };
    let total_time_raw = if active {
        bite_time_raw + catch_time_active_raw
    } else {
        bite_time_raw + auto_fish_time_raw + catch_time_afk_raw
    };
    let unoptimized_time_raw = zone_bite_avg_raw
        + if active {
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
    let overlay_editor = build_overlay_editor_signal(signals, data, &fish_group_chart);
    let fish_multiplier_raw = effective_fish_multiplier(signals, &items_by_key);

    let timespan_seconds = timespan_seconds(signals.timespan_amount, &signals.timespan_unit);
    let timespan_text = timespan_text(data.lang, signals.timespan_amount, &signals.timespan_unit);
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
    let target_fish_summary = derive_target_fish_summary(
        signals,
        data,
        &fish_group_chart,
        loot_total_catches_raw,
        timespan_seconds,
    );
    let stat_breakdowns = derive_stat_breakdowns(
        signals,
        data,
        &items_by_key,
        lifeskill_level,
        &pets,
        &data.catalog.pets,
        &fish_group_chart,
        &loot_chart,
        &target_fish_summary,
        &zone_name,
        zone_bite_min_raw,
        zone_bite_max_raw,
        zone_bite_avg_raw,
        factor_level,
        factor_resources,
        effective_bite_min_raw,
        effective_bite_max_raw,
        bite_time_raw,
        afr_uncapped_raw,
        afr_raw,
        auto_fish_time_raw,
        item_drr_raw,
        lifeskill_level_drr_raw,
        brandstone_durability_factor,
        chance_to_reduce_raw,
        catch_time_active_raw,
        catch_time_afk_raw,
        total_time_raw,
        timespan_seconds,
        &timespan_text,
        casts_average_raw,
        durability_loss_average_raw,
        fish_multiplier_raw,
        loot_total_catches_raw,
        loot_fish_per_hour_raw,
    );
    let fishing_timeline_chart = fishing_timeline_chart(
        data.lang,
        active,
        bite_time_raw,
        auto_fish_time_raw,
        catch_time_active_raw,
        catch_time_afk_raw,
        total_time_raw,
        zone_bite_avg_raw,
        stat_breakdown_from_json(&stat_breakdowns.bite_time),
        stat_breakdown_from_json(&stat_breakdowns.auto_fish_time),
        stat_breakdown_from_json(&stat_breakdowns.catch_time),
        stat_breakdown_from_json(&stat_breakdowns.time_saved),
    );

    let debug_json = serde_json::to_string_pretty(&json!({
        "inputs": signals,
        "derived": {
            "effectiveActive": active,
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
        segments: groups_distribution_segments(
            &fish_group_chart.rows,
            loot_total_catches_raw,
            signals.show_normalized_select_rates,
            data.lang,
        ),
    };
    let fish_group_silver_distribution_chart = DistributionChartSignal {
        segments: group_silver_distribution_segments(
            &loot_chart.rows,
            &loot_chart.species_rows,
            data.lang,
        ),
    };
    let target_fish_pmf_chart = target_fish_pmf_chart(&target_fish_summary);
    let loot_sankey_chart = LootSankeySignal {
        show_silver_amounts: loot_chart.show_silver_amounts,
        rows: filtered_loot_flow_rows(&loot_chart.rows, &loot_chart.species_rows),
        species_rows: loot_chart.species_rows.clone(),
    };

    CalculatorDerivedSignals {
        zone_name,
        abundance_label: calc_abundance_label(data.lang, signals.resources),
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
        casts_title: calculator_route_text_with_vars(
            data.lang,
            "calculator.title.casts_average",
            &[("timespan", &timespan_text)],
        ),
        casts_average: fmt2(casts_average_raw),
        item_drr_text: format!("{:.0}%", item_drr_raw * 100.0),
        chance_to_consume_durability_text: format!("{:.2}%", chance_to_reduce_raw * 100.0),
        durability_loss_title: calculator_route_text_with_vars(
            data.lang,
            "calculator.title.durability_loss_average",
            &[("timespan", &timespan_text)],
        ),
        durability_loss_average: fmt2(durability_loss_average_raw),
        timespan_text: timespan_text.clone(),
        bite_time_title: calculator_route_text_with_vars(
            data.lang,
            "calculator.title.bite_time",
            &[
                ("seconds", &fmt2(bite_time_raw)),
                ("percent", &fmt2(percent_bite)),
            ],
        ),
        auto_fish_time_title: calculator_route_text_with_vars(
            data.lang,
            "calculator.title.auto_fishing_time",
            &[
                ("seconds", &fmt2(auto_fish_time_raw)),
                ("percent", &fmt2(percent_af)),
            ],
        ),
        catch_time_title: calculator_route_text_with_vars(
            data.lang,
            "calculator.title.catch_time",
            &[
                ("seconds", &fmt2(catch_time_raw)),
                ("percent", &fmt2(percent_catch)),
            ],
        ),
        unoptimized_time_title: calculator_route_text_with_vars(
            data.lang,
            "calculator.title.unoptimized_time",
            &[
                ("seconds", &fmt2(unoptimized_time_raw)),
                ("percent", &fmt2(percent_improvement)),
            ],
        ),
        show_auto_fishing: !active,
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
        raw_prize_rate_text: fish_group_chart.raw_prize_rate_text.clone(),
        raw_prize_mastery_text: fish_group_chart.mastery_text.clone(),
        fish_group_distribution_chart,
        fish_group_silver_distribution_chart,
        target_fish_pmf_chart,
        loot_sankey_chart,
        target_fish_selected_label: target_fish_summary.selected_label.clone(),
        target_fish_pmf_count_hint: target_fish_summary.pmf_count_hint_text.clone(),
        target_fish_expected_title: calculator_route_text_with_vars(
            data.lang,
            "calculator.server.target.expected",
            &[("timespan", &timespan_text)],
        ),
        target_fish_expected_count: target_fish_summary.expected_count_text.clone(),
        target_fish_per_day: target_fish_summary.per_day_text.clone(),
        target_fish_time_to_target: target_fish_summary.time_to_target_text.clone(),
        target_fish_time_to_target_helper: if target_fish_summary.selected_label.is_empty() {
            calculator_route_text(data.lang, "calculator.server.helper.select_target_fish")
        } else {
            calculator_route_text_with_vars(
                data.lang,
                "calculator.server.helper.target_status_per_day",
                &[
                    ("label", &target_fish_summary.selected_label),
                    ("per_day", &target_fish_summary.per_day_text),
                ],
            )
        },
        target_fish_probability_at_least_title: calculator_route_text_with_vars(
            data.lang,
            "calculator.server.target.chance_at_least",
            &[("amount", &target_fish_summary.target_amount_text)],
        ),
        target_fish_probability_at_least: target_fish_summary.probability_at_least_text.clone(),
        target_fish_status_text: target_fish_summary.status_text.clone(),
        stat_breakdowns,
        fishing_timeline_chart,
        overlay_editor,
        debug_json,
    }
}

fn pet_effect_sum_by_keys<'a>(
    keys: impl IntoIterator<Item = &'a str>,
    catalog: &'a [CalculatorPetOptionEntry],
    effect: impl Fn(&CalculatorPetOptionEntry) -> Option<f32> + Copy,
) -> f64 {
    let mut selected_keys = keys
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .collect::<Vec<_>>();
    selected_keys.sort();
    selected_keys.dedup();
    selected_keys
        .into_iter()
        .filter_map(|key| pet_option_by_key(catalog, key))
        .filter_map(|option| effect(option))
        .map(f64::from)
        .sum()
}

fn pet_talent_effect(
    pet: &CalculatorPetSignals,
    catalog: &CalculatorPetCatalog,
    effect: impl Fn(&CalculatorPetOptionEntry) -> Option<f32> + Copy,
) -> f64 {
    if pet.pet.trim().is_empty() {
        return 0.0;
    }
    let Some(option) = pet_option_by_key(&catalog.talents, &pet.talent) else {
        return 0.0;
    };
    let base = effect(option).map(f64::from).unwrap_or_default();
    if base <= 0.0 {
        return 0.0;
    }
    if pet.pack_leader && pet.tier.trim() == "5" {
        base + 0.01
    } else {
        base
    }
}

fn pet_afr(pet: &CalculatorPetSignals, catalog: &CalculatorPetCatalog) -> f64 {
    if pet.pet.trim().is_empty() {
        return 0.0;
    }
    pet_effect_sum_by_keys(
        std::iter::once(pet.special.as_str()),
        &catalog.specials,
        |option| option.auto_fishing_time_reduction,
    )
}

fn pet_drr(pet: &CalculatorPetSignals, catalog: &CalculatorPetCatalog) -> f64 {
    pet_talent_effect(pet, catalog, |option| {
        option.durability_reduction_resistance
    })
}

fn pet_fishing_exp(pet: &CalculatorPetSignals, catalog: &CalculatorPetCatalog) -> f64 {
    if pet.pet.trim().is_empty() {
        return 0.0;
    }
    pet_effect_sum_by_keys(
        pet.skills.iter().map(String::as_str),
        &catalog.skills,
        |option| option.fishing_exp,
    )
}

fn pet_life_exp(pet: &CalculatorPetSignals, catalog: &CalculatorPetCatalog) -> f64 {
    pet_talent_effect(pet, catalog, |option| option.life_exp)
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

fn computed_stat_breakdown_row(
    label: impl Into<String>,
    value_text: impl Into<String>,
    detail_text: impl Into<String>,
) -> ComputedStatBreakdownRow {
    ComputedStatBreakdownRow {
        label: label.into(),
        value_text: value_text.into(),
        detail_text: detail_text.into(),
        kind: None,
        icon_url: None,
        grade_tone: None,
        formula_part: None,
        formula_part_order: None,
    }
}

fn computed_stat_breakdown_item_row(
    item: &CalculatorItemEntry,
    cdn_base_url: &str,
    value_text: impl Into<String>,
    detail_text: impl Into<String>,
) -> ComputedStatBreakdownRow {
    ComputedStatBreakdownRow {
        label: item.name.clone(),
        value_text: value_text.into(),
        detail_text: detail_text.into(),
        kind: Some("item"),
        icon_url: item
            .icon
            .as_deref()
            .map(|icon| absolute_public_asset_url(cdn_base_url, icon)),
        grade_tone: Some(item_grade_tone(item.grade.as_deref()).to_string()),
        formula_part: None,
        formula_part_order: None,
    }
}

fn computed_stat_breakdown_loot_species_row(
    row: &LootSpeciesRow,
    value_text: impl Into<String>,
    detail_text: impl Into<String>,
) -> ComputedStatBreakdownRow {
    ComputedStatBreakdownRow {
        label: row.label.clone(),
        value_text: value_text.into(),
        detail_text: detail_text.into(),
        kind: Some("item"),
        icon_url: row.icon_url.clone(),
        grade_tone: Some(row.icon_grade_tone.clone()),
        formula_part: None,
        formula_part_order: None,
    }
}

fn computed_stat_breakdown_row_with_formula_part(
    mut row: ComputedStatBreakdownRow,
    formula_part: impl Into<String>,
    formula_part_order: u8,
) -> ComputedStatBreakdownRow {
    row.formula_part = Some(formula_part.into());
    row.formula_part_order = Some(formula_part_order);
    row
}

fn computed_stat_breakdown_rows_with_formula_part(
    rows: Vec<ComputedStatBreakdownRow>,
    formula_part: impl Into<String>,
    formula_part_order: u8,
) -> Vec<ComputedStatBreakdownRow> {
    let formula_part = formula_part.into();
    rows.into_iter()
        .map(|row| {
            computed_stat_breakdown_row_with_formula_part(
                row,
                formula_part.clone(),
                formula_part_order,
            )
        })
        .collect()
}

fn collect_item_property_breakdown_rows(
    items_by_key: &HashMap<&str, &CalculatorItemEntry>,
    cdn_base_url: &str,
    singles: &[&String],
    groups: &[&Vec<String>],
    value: impl Fn(&CalculatorItemEntry) -> Option<f64> + Copy,
    detail_text: &str,
) -> Vec<ComputedStatBreakdownRow> {
    let mut rows = Vec::new();
    for key in singles {
        if let Some(item) = items_by_key.get(key.as_str()) {
            if let Some(amount) = value(item).filter(|amount| *amount > 0.0) {
                rows.push(computed_stat_breakdown_item_row(
                    item,
                    cdn_base_url,
                    format!("+{}%", trim_float(amount * 100.0)),
                    detail_text.to_string(),
                ));
            }
        }
    }
    for group in groups {
        for key in group.iter().filter(|key| !key.trim().is_empty()) {
            if let Some(item) = items_by_key.get(key.as_str()) {
                if let Some(amount) = value(item).filter(|amount| *amount > 0.0) {
                    rows.push(computed_stat_breakdown_item_row(
                        item,
                        cdn_base_url,
                        format!("+{}%", trim_float(amount * 100.0)),
                        detail_text.to_string(),
                    ));
                }
            }
        }
    }
    rows
}

fn computed_stat_breakdown_section(
    label: impl Into<String>,
    rows: Vec<ComputedStatBreakdownRow>,
) -> ComputedStatBreakdownSection {
    ComputedStatBreakdownSection {
        label: label.into(),
        rows,
    }
}

fn computed_stat_formula_term(
    label: impl Into<String>,
    value_text: impl Into<String>,
) -> ComputedStatFormulaTerm {
    ComputedStatFormulaTerm {
        label: label.into(),
        value_text: value_text.into(),
        aliases: Vec::new(),
    }
}

fn computed_stat_formula_term_with_aliases<I, S>(
    label: impl Into<String>,
    value_text: impl Into<String>,
    aliases: I,
) -> ComputedStatFormulaTerm
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    ComputedStatFormulaTerm {
        label: label.into(),
        value_text: value_text.into(),
        aliases: aliases.into_iter().map(Into::into).collect(),
    }
}

fn join_formula_term_values<I, S>(values: I, separator: &str, fallback: &str) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let parts = values
        .into_iter()
        .map(|value| value.as_ref().trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if parts.is_empty() {
        fallback.to_string()
    } else {
        parts.join(separator)
    }
}

fn computed_stat_breakdown(
    title: impl Into<String>,
    value_text: impl Into<String>,
    summary_text: impl Into<String>,
    formula_text: impl Into<String>,
    sections: Vec<ComputedStatBreakdownSection>,
) -> ComputedStatBreakdown {
    ComputedStatBreakdown {
        kind_label: String::new(),
        title: title.into(),
        value_text: value_text.into(),
        summary_text: summary_text.into(),
        formula_text: formula_text.into(),
        formula_terms: Vec::new(),
        sections,
    }
}

fn stat_breakdown_json(payload: ComputedStatBreakdown) -> String {
    serde_json::to_string(&payload).unwrap_or_default()
}

fn stat_breakdown_from_json(raw: &str) -> Option<Value> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    serde_json::from_str(trimmed).ok()
}

fn option_label(options: &[CalculatorOptionEntry], key: &str) -> String {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    options
        .iter()
        .find(|option| option.key == trimmed)
        .map(|option| option.label.clone())
        .unwrap_or_else(|| trimmed.to_string())
}

fn pet_option_label(options: &[CalculatorPetOptionEntry], key: &str) -> String {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    options
        .iter()
        .find(|option| option.key == trimmed)
        .map(|option| option.label.clone())
        .unwrap_or_else(|| trimmed.to_string())
}

fn selected_items<'a>(
    items_by_key: &HashMap<&str, &'a CalculatorItemEntry>,
    singles: &[&String],
    groups: &[&Vec<String>],
) -> Vec<&'a CalculatorItemEntry> {
    let mut rows = Vec::new();
    for key in singles {
        if let Some(item) = items_by_key.get(key.as_str()) {
            rows.push(*item);
        }
    }
    for group in groups {
        for key in group.iter().filter(|key| !key.trim().is_empty()) {
            if let Some(item) = items_by_key.get(key.as_str()) {
                rows.push(*item);
            }
        }
    }
    rows
}

fn collect_selected_item_rows(
    items_by_key: &HashMap<&str, &CalculatorItemEntry>,
    cdn_base_url: &str,
    singles: &[&String],
    groups: &[&Vec<String>],
    build: impl Fn(&CalculatorItemEntry) -> Option<(String, String)>,
) -> Vec<ComputedStatBreakdownRow> {
    selected_items(items_by_key, singles, groups)
        .into_iter()
        .filter_map(|item| {
            build(item).map(|(value_text, detail_text)| {
                computed_stat_breakdown_item_row(item, cdn_base_url, value_text, detail_text)
            })
        })
        .collect()
}

fn computed_stat_breakdown_zone_loot_item_row(
    entry: &CalculatorZoneLootEntry,
    cdn_base_url: &str,
    value_text: impl Into<String>,
    detail_text: impl Into<String>,
) -> ComputedStatBreakdownRow {
    ComputedStatBreakdownRow {
        label: entry.name.clone(),
        value_text: value_text.into(),
        detail_text: detail_text.into(),
        kind: Some("item"),
        icon_url: entry
            .icon
            .as_deref()
            .map(|icon| absolute_public_asset_url(cdn_base_url, icon)),
        grade_tone: Some(item_grade_tone(entry.grade.as_deref()).to_string()),
        formula_part: None,
        formula_part_order: None,
    }
}

fn pet_slot_name(lang: CalculatorLocale, slot_idx: usize) -> String {
    calculator_route_text_with_vars(
        lang,
        "calculator.breakdown.label.pet_slot",
        &[("slot", &slot_idx.to_string())],
    )
}

fn calculator_pet_pack_leader_label(lang: CalculatorLocale) -> String {
    calculator_route_text(lang, "calculator.server.field.pack_leader")
}

fn pet_talent_breakdown_detail(
    lang: CalculatorLocale,
    pet: &CalculatorPetSignals,
    catalog: &CalculatorPetCatalog,
) -> String {
    let talent_label = pet_option_label(&catalog.talents, &pet.talent);
    let tier_label = option_label(&catalog.tiers, &pet.tier);
    if pet.pack_leader {
        format!(
            "{talent_label} · {tier_label} · {}",
            calculator_pet_pack_leader_label(lang)
        )
    } else {
        format!("{talent_label} · {tier_label}")
    }
}

fn collect_pet_afr_breakdown_rows(
    lang: CalculatorLocale,
    pets: &[&CalculatorPetSignals],
    catalog: &CalculatorPetCatalog,
) -> Vec<ComputedStatBreakdownRow> {
    pets.iter()
        .enumerate()
        .filter_map(|(index, pet)| {
            let amount = pet_afr(pet, catalog);
            if amount <= 0.0 {
                return None;
            }
            let special_label = pet_option_label(&catalog.specials, &pet.special);
            let tier_label = option_label(&catalog.tiers, &pet.tier);
            Some(computed_stat_breakdown_row(
                pet_slot_name(lang, index + 1),
                format!("+{}%", trim_float(amount * 100.0)),
                format!("{special_label} · {tier_label}"),
            ))
        })
        .collect()
}

fn collect_pet_drr_breakdown_rows(
    lang: CalculatorLocale,
    pets: &[&CalculatorPetSignals],
    catalog: &CalculatorPetCatalog,
) -> Vec<ComputedStatBreakdownRow> {
    pets.iter()
        .enumerate()
        .filter_map(|(index, pet)| {
            let amount = pet_drr(pet, catalog);
            if amount <= 0.0 {
                return None;
            }
            Some(computed_stat_breakdown_row(
                pet_slot_name(lang, index + 1),
                format!("+{}%", trim_float(amount * 100.0)),
                pet_talent_breakdown_detail(lang, pet, catalog),
            ))
        })
        .collect()
}

fn collect_fish_multiplier_breakdown_rows(
    lang: CalculatorLocale,
    items_by_key: &HashMap<&str, &CalculatorItemEntry>,
    cdn_base_url: &str,
    signals: &CalculatorSignals,
    applied_multiplier: f64,
) -> Vec<ComputedStatBreakdownRow> {
    collect_selected_item_rows(
        items_by_key,
        cdn_base_url,
        &[
            &signals.rod,
            &signals.float,
            &signals.chair,
            &signals.lightstone_set,
            &signals.backpack,
        ],
        &[&signals.outfit, &signals.food, &signals.buff],
        |item| {
            item.fish_multiplier
                .map(f64::from)
                .filter(|value| *value > 1.0)
                .map(|value| {
                    let detail = if (value - applied_multiplier).abs() < 0.0001 {
                        calculator_route_text(
                            lang,
                            "calculator.breakdown.detail.applied_highest_fish_multiplier",
                        )
                    } else {
                        calculator_route_text(
                            lang,
                            "calculator.breakdown.detail.lower_source_highest_multiplier_applies",
                        )
                    };
                    (format!("×{}", trim_float(value)), detail)
                })
        },
    )
}

fn loot_group_profit_breakdown_rows(
    loot_rows: &[LootChartRow],
    lang: CalculatorLocale,
) -> Vec<ComputedStatBreakdownRow> {
    loot_rows
        .iter()
        .filter(|row| row.expected_profit_raw > 0.0)
        .map(|row| {
            computed_stat_breakdown_row(
                calculator_group_display_label(lang, &row.label),
                row.expected_profit_text.clone(),
                calculator_route_text_with_vars(
                    lang,
                    "calculator.breakdown.detail.group_expected_silver_share",
                    &[("share", &row.silver_share_text)],
                ),
            )
        })
        .collect()
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

fn mastery_prize_rate_for_bracket(curve: &[CalculatorMasteryPrizeRateEntry], mastery: f64) -> f64 {
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
    let text = |key: &str| calculator_route_text(data.lang, key);
    let text_with_vars =
        |key: &str, vars: &[(&str, &str)]| calculator_route_text_with_vars(data.lang, key, vars);
    let Some(zone_group_rate) = data.zone_group_rates.get(&signals.zone) else {
        return FishGroupChart {
            available: false,
            note: text("calculator.server.chart.group_distribution_note.unavailable"),
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
    let rare_bonus_inputs = collect_item_property_breakdown_rows(
        items_by_key,
        data.cdn_base_url.as_str(),
        &[
            &signals.rod,
            &signals.float,
            &signals.chair,
            &signals.lightstone_set,
            &signals.backpack,
        ],
        &[&signals.outfit, &signals.food, &signals.buff],
        |item| item.bonus_rare.map(f64::from),
        "",
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
    let high_quality_bonus_inputs = collect_item_property_breakdown_rows(
        items_by_key,
        data.cdn_base_url.as_str(),
        &[
            &signals.rod,
            &signals.float,
            &signals.chair,
            &signals.lightstone_set,
            &signals.backpack,
        ],
        &[&signals.outfit, &signals.food, &signals.buff],
        |item| item.bonus_big.map(f64::from),
        "",
    );

    let rare_base = f64::from(zone_group_rate.rare_rate_raw.max(0)) / 1_000_000.0;
    let high_quality_base = f64::from(zone_group_rate.high_quality_rate_raw.max(0)) / 1_000_000.0;
    let general_base = f64::from(zone_group_rate.general_rate_raw.max(0)) / 1_000_000.0;
    let trash_base = f64::from(zone_group_rate.trash_rate_raw.max(0)) / 1_000_000.0;

    let zone_overlay = zone_overlay_for_signals(signals, &signals.zone);
    let removed_group_slots = zone_overlay
        .into_iter()
        .flat_map(|zone_overlay| zone_overlay.groups.iter())
        .filter_map(|(slot_key, group_overlay)| {
            (group_overlay.present == Some(false))
                .then(|| slot_key.parse::<u8>().ok())
                .flatten()
        })
        .collect::<HashSet<_>>();
    let explicit_group_raw_pct_by_slot = zone_overlay
        .into_iter()
        .flat_map(|zone_overlay| zone_overlay.groups.iter())
        .filter_map(|(slot_key, group_overlay)| {
            let slot_idx = slot_key.parse::<u8>().ok()?;
            group_overlay
                .raw_rate_percent
                .map(|raw_rate_percent| (slot_idx, raw_rate_percent.max(0.0)))
        })
        .collect::<HashMap<_, _>>();
    let group_overlay_active = zone_overlay_has_changes(zone_overlay);

    let mut available_slots = data
        .zone_loot_entries
        .iter()
        .filter_map(|entry| (1..=5).contains(&entry.slot_idx).then_some(entry.slot_idx))
        .collect::<HashSet<_>>();
    if zone_group_rate.prize_main_group_key.is_some() {
        available_slots.insert(1);
    }
    if let Some(zone_overlay) = zone_overlay {
        for (slot_key, group_overlay) in &zone_overlay.groups {
            let Some(slot_idx) = slot_key
                .parse::<u8>()
                .ok()
                .filter(|slot_idx| (1..=5).contains(slot_idx))
            else {
                continue;
            };
            if group_overlay.present == Some(false) {
                available_slots.remove(&slot_idx);
                continue;
            }
            if group_overlay.present == Some(true) || group_overlay.raw_rate_percent.is_some() {
                available_slots.insert(slot_idx);
            }
        }
    }

    let rare_weight = if available_slots.contains(&2) {
        rare_base + rare_bonus.max(0.0)
    } else {
        0.0
    };
    let high_quality_weight = if available_slots.contains(&3) {
        high_quality_base + high_quality_bonus.max(0.0)
    } else {
        0.0
    };
    let general_weight = if available_slots.contains(&4) {
        general_base
    } else {
        0.0
    };
    let trash_weight = if available_slots.contains(&5) {
        trash_base
    } else {
        0.0
    };
    let prize_weight = if available_slots.contains(&1) {
        mastery_prize_rate.max(0.0)
    } else {
        0.0
    };
    let total_weight =
        prize_weight + rare_weight + high_quality_weight + general_weight + trash_weight;

    let current_share = |weight: f64| {
        if total_weight <= 0.0 {
            0.0
        } else {
            (weight / total_weight) * 100.0
        }
    };
    let base_rate_pct_by_slot = HashMap::from([
        (1_u8, 0.0),
        (2_u8, rare_base * 100.0),
        (3_u8, high_quality_base * 100.0),
        (4_u8, general_base * 100.0),
        (5_u8, trash_base * 100.0),
    ]);
    let bonus_weight_pct_by_slot = HashMap::from([
        (
            1_u8,
            if available_slots.contains(&1) {
                mastery_prize_rate.max(0.0) * 100.0
            } else {
                0.0
            },
        ),
        (
            2_u8,
            if available_slots.contains(&2) {
                rare_bonus.max(0.0) * 100.0
            } else {
                0.0
            },
        ),
        (
            3_u8,
            if available_slots.contains(&3) {
                high_quality_bonus.max(0.0) * 100.0
            } else {
                0.0
            },
        ),
        (4_u8, 0.0),
        (5_u8, 0.0),
    ]);
    let active_group_slots = available_slots
        .iter()
        .copied()
        .filter(|slot_idx| !removed_group_slots.contains(slot_idx))
        .chain(explicit_group_raw_pct_by_slot.keys().copied())
        .collect::<HashSet<_>>();
    let effective_base_rate_pct_by_slot = active_group_slots
        .iter()
        .copied()
        .map(|slot_idx| {
            (
                slot_idx,
                explicit_group_raw_pct_by_slot
                    .get(&slot_idx)
                    .copied()
                    .unwrap_or_else(|| {
                        base_rate_pct_by_slot
                            .get(&slot_idx)
                            .copied()
                            .unwrap_or_default()
                            .max(0.0)
                    }),
            )
        })
        .collect::<HashMap<_, _>>();
    let effective_weight_pct_by_slot = active_group_slots
        .iter()
        .copied()
        .map(|slot_idx| {
            (
                slot_idx,
                effective_base_rate_pct_by_slot
                    .get(&slot_idx)
                    .copied()
                    .unwrap_or_default()
                    + bonus_weight_pct_by_slot
                        .get(&slot_idx)
                        .copied()
                        .unwrap_or_default(),
            )
        })
        .collect::<HashMap<_, _>>();
    let effective_total_weight_pct = effective_weight_pct_by_slot
        .values()
        .copied()
        .map(|value| value.max(0.0))
        .sum::<f64>();
    let effective_current_share_by_slot = if effective_total_weight_pct > 0.0 {
        effective_weight_pct_by_slot
            .iter()
            .map(|(slot_idx, weight_pct)| {
                (
                    *slot_idx,
                    (weight_pct.max(0.0) / effective_total_weight_pct) * 100.0,
                )
            })
            .collect::<HashMap<_, _>>()
    } else {
        active_group_slots
            .iter()
            .copied()
            .map(|slot_idx| (slot_idx, 0.0))
            .collect::<HashMap<_, _>>()
    };
    let base_current_share_by_slot = HashMap::from([
        (1_u8, current_share(prize_weight)),
        (2_u8, current_share(rare_weight)),
        (3_u8, current_share(high_quality_weight)),
        (4_u8, current_share(general_weight)),
        (5_u8, current_share(trash_weight)),
    ]);

    let prize_inputs = vec![
        computed_stat_breakdown_row(
            text("calculator.server.field.mastery"),
            trim_float(signals.mastery),
            text("calculator.breakdown.detail.current_mastery_input_for_prize_curve_lookup"),
        ),
        computed_stat_breakdown_row(
            text("calculator.server.group.prize_curve_result"),
            percent_value_text(prize_weight * 100.0),
            text("calculator.server.group.prize_curve_result_detail"),
        ),
    ];

    let mut rare_inputs = vec![computed_stat_breakdown_row(
        text("calculator.server.group.zone_base_rate"),
        percent_value_text(rare_base * 100.0),
        text("calculator.server.group.zone_base_rate_detail"),
    )];
    rare_inputs.extend(rare_bonus_inputs);

    let mut high_quality_inputs = vec![computed_stat_breakdown_row(
        text("calculator.server.group.zone_base_rate"),
        percent_value_text(high_quality_base * 100.0),
        text("calculator.server.group.zone_base_rate_detail"),
    )];
    high_quality_inputs.extend(high_quality_bonus_inputs);

    let general_inputs = vec![computed_stat_breakdown_row(
        text("calculator.server.group.zone_base_rate"),
        percent_value_text(general_base * 100.0),
        text("calculator.server.group.zone_base_rate_detail"),
    )];
    let trash_inputs = vec![computed_stat_breakdown_row(
        text("calculator.server.group.zone_base_rate"),
        percent_value_text(trash_base * 100.0),
        text("calculator.server.group.zone_base_rate_detail"),
    )];

    let build_group_row =
        |slot_idx: u8,
         label: &'static str,
         fill_color: &'static str,
         stroke_color: &'static str,
         text_color: &'static str,
         connector_color: &'static str,
         default_bonus_text: String,
         default_base_share_pct: f64,
         default_weight_pct: f64,
         default_current_share_pct: f64,
         default_rate_inputs: Vec<ComputedStatBreakdownRow>| {
            let explicit_raw_pct = explicit_group_raw_pct_by_slot.get(&slot_idx).copied();
            let bonus_weight_pct = bonus_weight_pct_by_slot
                .get(&slot_idx)
                .copied()
                .unwrap_or_default();
            let effective_weight_pct = effective_weight_pct_by_slot
                .get(&slot_idx)
                .copied()
                .unwrap_or_default();
            let effective_current_share_pct = effective_current_share_by_slot
                .get(&slot_idx)
                .copied()
                .unwrap_or_default();
            let is_overlay_row = group_overlay_active
                && (active_group_slots.contains(&slot_idx) || explicit_raw_pct.is_some());
            let mut rate_inputs = default_rate_inputs;
            if let Some(explicit_raw_pct) = explicit_raw_pct {
                rate_inputs.insert(
                    0,
                    computed_stat_breakdown_row(
                        text("calculator.server.group.personal_overlay_raw_base_rate"),
                        percent_value_text(explicit_raw_pct),
                        text("calculator.server.group.personal_overlay_raw_base_rate_detail"),
                    ),
                );
            } else if is_overlay_row
                && (effective_current_share_pct - default_current_share_pct).abs() > f64::EPSILON
            {
                rate_inputs.insert(
                    0,
                    computed_stat_breakdown_row(
                        text("calculator.server.group.overlay_adjusted_normalized_share"),
                        percent_value_text(effective_current_share_pct),
                        text("calculator.server.group.overlay_adjusted_normalized_share_detail"),
                    ),
                );
            }
            let display_label = calculator_group_display_label(data.lang, label);
            FishGroupChartRow {
                label,
                fill_color,
                stroke_color,
                text_color,
                connector_color,
                drop_rate_source_kind: if is_overlay_row
                    && (effective_current_share_pct > 0.0 || explicit_raw_pct.is_some())
                {
                    "overlay".to_string()
                } else if default_current_share_pct > 0.0 {
                    "database".to_string()
                } else {
                    String::new()
                },
                drop_rate_tooltip: if let Some(explicit_raw_pct) = explicit_raw_pct {
                    if bonus_weight_pct > 0.0 {
                        text_with_vars(
                            "calculator.server.group.tooltip.overlay_explicit_with_bonus",
                            &[
                                ("base", &percent_value_text(explicit_raw_pct)),
                                ("bonus", &percent_value_text(bonus_weight_pct)),
                                ("weight", &percent_value_text(effective_weight_pct)),
                                ("share", &percent_value_text(effective_current_share_pct)),
                            ],
                        )
                    } else {
                        text_with_vars(
                            "calculator.server.group.tooltip.overlay_explicit",
                            &[
                                ("base", &percent_value_text(explicit_raw_pct)),
                                ("share", &percent_value_text(effective_current_share_pct)),
                            ],
                        )
                    }
                } else if is_overlay_row && effective_current_share_pct > 0.0 {
                    text_with_vars(
                        "calculator.server.group.tooltip.overlay_adjusted",
                        &[
                            ("group", &display_label),
                            ("share", &percent_value_text(effective_current_share_pct)),
                        ],
                    )
                } else if default_current_share_pct > 0.0 {
                    text_with_vars(
                        "calculator.server.group.tooltip.source_backed_share",
                        &[("group", &display_label)],
                    )
                } else {
                    String::new()
                },
                bonus_text: if let Some(explicit_raw_pct) = explicit_raw_pct {
                    if bonus_weight_pct > 0.0 {
                        text_with_vars(
                            "calculator.server.group.bonus.base_plus_bonus",
                            &[
                                ("base", &percent_value_text(explicit_raw_pct)),
                                ("bonus", &percent_value_text(bonus_weight_pct)),
                            ],
                        )
                    } else {
                        text_with_vars(
                            "calculator.server.group.bonus.base_only",
                            &[("base", &percent_value_text(explicit_raw_pct))],
                        )
                    }
                } else if is_overlay_row {
                    text("calculator.server.group.bonus.normalized_from_active_weights")
                } else {
                    default_bonus_text
                },
                base_share_pct: default_base_share_pct,
                default_weight_pct,
                weight_pct: effective_weight_pct,
                current_share_pct: effective_current_share_pct,
                rate_inputs,
            }
        };

    FishGroupChart {
        available: true,
        note: if group_overlay_active {
            text("calculator.server.chart.group_distribution_note.overlay_active")
        } else {
            text("calculator.server.chart.group_distribution_note.default")
        },
        raw_prize_rate_text: format!("{}%", trim_float(prize_weight * 100.0)),
        mastery_text: trim_float(signals.mastery),
        rows: vec![
            build_group_row(
                1,
                "Prize",
                "#fda4af",
                "#f87171",
                "#450a0a",
                "rgb(248 113 113 / 0.48)",
                text_with_vars(
                    "calculator.server.group.bonus.mastery_raw_prize",
                    &[
                        ("mastery", &trim_float(signals.mastery)),
                        ("rate", &percent_value_text(prize_weight * 100.0)),
                    ],
                ),
                0.0,
                prize_weight * 100.0,
                base_current_share_by_slot
                    .get(&1)
                    .copied()
                    .unwrap_or_default(),
                prize_inputs,
            ),
            build_group_row(
                2,
                "Rare",
                "#fde68a",
                "#facc15",
                "#422006",
                "rgb(250 204 21 / 0.48)",
                if rare_bonus > 0.0 {
                    text_with_vars(
                        "calculator.server.group.bonus.rare",
                        &[("rate", &trim_float(rare_bonus * 100.0))],
                    )
                } else {
                    text("calculator.server.group.bonus.none")
                },
                rare_base * 100.0,
                rare_weight * 100.0,
                base_current_share_by_slot
                    .get(&2)
                    .copied()
                    .unwrap_or_default(),
                rare_inputs,
            ),
            build_group_row(
                3,
                "High-Quality",
                "#93c5fd",
                "#60a5fa",
                "#172554",
                "rgb(96 165 250 / 0.48)",
                if high_quality_bonus > 0.0 {
                    text_with_vars(
                        "calculator.server.group.bonus.high_quality",
                        &[("rate", &trim_float(high_quality_bonus * 100.0))],
                    )
                } else {
                    text("calculator.server.group.bonus.none")
                },
                high_quality_base * 100.0,
                high_quality_weight * 100.0,
                base_current_share_by_slot
                    .get(&3)
                    .copied()
                    .unwrap_or_default(),
                high_quality_inputs,
            ),
            build_group_row(
                4,
                "General",
                "#86efac",
                "#4ade80",
                "#052e16",
                "rgb(74 222 128 / 0.48)",
                text("calculator.server.group.bonus.none"),
                general_base * 100.0,
                general_weight * 100.0,
                base_current_share_by_slot
                    .get(&4)
                    .copied()
                    .unwrap_or_default(),
                general_inputs,
            ),
            build_group_row(
                5,
                "Trash",
                "var(--color-base-100)",
                "color-mix(in srgb, var(--color-base-content) 16%, transparent)",
                "var(--color-base-content)",
                "color-mix(in srgb, var(--color-base-content) 24%, transparent)",
                text("calculator.server.group.bonus.none"),
                trash_base * 100.0,
                trash_weight * 100.0,
                base_current_share_by_slot
                    .get(&5)
                    .copied()
                    .unwrap_or_default(),
                trash_inputs,
            ),
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

fn item_grade_tone(grade: Option<&str>) -> &'static str {
    match grade {
        Some("Prize") => "red",
        Some("Rare") => "yellow",
        Some("HighQuality") => "blue",
        Some("General") => "green",
        Some("Trash") => "white",
        _ => "unknown",
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

fn compact_silver_text(value: f64) -> String {
    let absolute = value.abs();
    let (divisor, suffix) = if absolute >= 1_000_000_000_000.0 {
        (1_000_000_000_000.0, "T")
    } else if absolute >= 1_000_000_000.0 {
        (1_000_000_000.0, "B")
    } else if absolute >= 1_000_000.0 {
        (1_000_000.0, "M")
    } else if absolute >= 1_000.0 {
        (1_000.0, "K")
    } else {
        return fmt_silver(value);
    };

    format!("{}{}", trim_float_to(value / divisor, 1), suffix)
}

fn evidence_display_rate(
    signals: &CalculatorSignals,
    evidence: &CalculatorZoneLootEvidence,
) -> Option<f64> {
    if signals.show_normalized_select_rates {
        evidence.normalized_rate.or(evidence.rate)
    } else {
        evidence.rate
    }
}

fn loot_species_rate_evidence(
    entry: &CalculatorZoneLootEntry,
) -> Option<&CalculatorZoneLootEvidence> {
    entry
        .evidence
        .iter()
        .find(|evidence| {
            evidence.source_family == "database" && evidence.claim_kind == "in_group_rate"
        })
        .or_else(|| {
            entry.evidence.iter().find(|evidence| {
                evidence.source_family == "community"
                    && evidence.claim_kind == "guessed_in_group_rate"
            })
        })
}

fn fish_group_label(slot_idx: u8) -> Option<&'static str> {
    match slot_idx {
        1 => Some("Prize"),
        2 => Some("Rare"),
        3 => Some("High-Quality"),
        4 => Some("General"),
        5 => Some("Trash"),
        6 => Some("Harpoon"),
        _ => None,
    }
}

fn fish_group_slot_idx(label: &str) -> Option<u8> {
    match label.trim() {
        "Prize" => Some(1),
        "Rare" => Some(2),
        "High-Quality" => Some(3),
        "General" => Some(4),
        "Trash" => Some(5),
        "Harpoon" => Some(6),
        _ => None,
    }
}

fn zone_loot_slot_sort_key(slot_idx: u8) -> u8 {
    if slot_idx == 0 {
        u8::MAX
    } else {
        slot_idx
    }
}

fn default_zone_loot_group_values(
    slot_idx: u8,
) -> Option<(&'static str, &'static str, &'static str, &'static str)> {
    match slot_idx {
        1 => Some(("Prize", "#fda4af", "#f87171", "#450a0a")),
        2 => Some(("Rare", "#fde68a", "#facc15", "#422006")),
        3 => Some(("High-Quality", "#93c5fd", "#60a5fa", "#172554")),
        4 => Some(("General", "#86efac", "#4ade80", "#052e16")),
        5 => Some((
            "Trash",
            "var(--color-base-100)",
            "color-mix(in srgb, var(--color-base-content) 16%, transparent)",
            "var(--color-base-content)",
        )),
        6 => Some(("Harpoon", "#c7f9f1", "#2dd4bf", "#083344")),
        0 => Some((
            "Unassigned",
            "var(--color-base-200)",
            "var(--color-base-300)",
            "var(--color-base-content)",
        )),
        _ => None,
    }
}

fn zone_loot_group_values(
    lang: CalculatorLocale,
    slot_idx: u8,
    chart_row: Option<&FishGroupChartRow>,
) -> (String, String, String, String) {
    if let Some(chart_row) = chart_row {
        return (
            calculator_group_display_label(lang, &chart_row.label),
            chart_row.fill_color.to_string(),
            chart_row.stroke_color.to_string(),
            chart_row.text_color.to_string(),
        );
    }
    if let Some((label, fill_color, stroke_color, text_color)) =
        default_zone_loot_group_values(slot_idx)
    {
        return (
            calculator_group_display_label(lang, label),
            fill_color.to_string(),
            stroke_color.to_string(),
            text_color.to_string(),
        );
    }
    (
        calculator_route_text(lang, "calculator.breakdown.label.unassigned"),
        "var(--color-base-200)".to_string(),
        "var(--color-base-300)".to_string(),
        "var(--color-base-content)".to_string(),
    )
}

fn fish_group_drop_rate_source_kind(row: &FishGroupChartRow) -> String {
    row.drop_rate_source_kind.clone()
}

fn fish_group_drop_rate_tooltip(row: &FishGroupChartRow) -> String {
    row.drop_rate_tooltip.clone()
}

fn fish_group_distribution_breakdown(
    row: &FishGroupChartRow,
    total_catches_raw: f64,
    total_weight_pct: f64,
    show_normalized_rates: bool,
    lang: CalculatorLocale,
) -> ComputedStatBreakdown {
    let text = |key: &str| calculator_route_text(lang, key);
    let text_with_vars =
        |key: &str, vars: &[(&str, &str)]| calculator_route_text_with_vars(lang, key, vars);
    let group_label = calculator_group_display_label(lang, &row.label);
    let input_rows = if row.rate_inputs.is_empty() {
        vec![computed_stat_breakdown_row(
            text("calculator.breakdown.section.inputs"),
            text("calculator.server.value.unavailable"),
            text("calculator.breakdown.detail.no_direct_inputs"),
        )]
    } else {
        row.rate_inputs.clone()
    };

    let raw_weight_detail = if row.weight_pct <= 0.0 && row.base_share_pct > 0.0 {
        text("calculator.server.group.breakdown.detail.raw_group_weight.zeroed_out")
    } else if row.label == "Prize" {
        text("calculator.server.group.breakdown.detail.raw_group_weight.prize_curve")
    } else if row.weight_pct > row.base_share_pct {
        text("calculator.server.group.breakdown.detail.raw_group_weight.bonus_sources")
    } else {
        text("calculator.server.group.breakdown.detail.raw_group_weight.no_bonus")
    };

    let mut composition_rows = vec![
        computed_stat_breakdown_row(
            text("calculator.server.group.breakdown.label.raw_group_weight"),
            percent_value_text(row.weight_pct),
            raw_weight_detail,
        ),
        computed_stat_breakdown_row(
            text("calculator.server.group.breakdown.label.all_group_weight_total"),
            percent_value_text(total_weight_pct),
            text("calculator.server.group.breakdown.detail.all_group_weight_total.denominator"),
        ),
    ];
    if show_normalized_rates {
        composition_rows.push(computed_stat_breakdown_row(
            text("calculator.server.group.breakdown.label.current_share"),
            percent_value_text(row.current_share_pct),
            text("calculator.server.group.breakdown.detail.current_share.normalized"),
        ));
    } else {
        composition_rows.push(computed_stat_breakdown_row(
            text("calculator.overlay.breakdown.label.normalized_share"),
            percent_value_text(row.current_share_pct),
            text("calculator.server.group.breakdown.detail.normalized_share.expected_catches"),
        ));
    }
    composition_rows.push(computed_stat_breakdown_row(
        text("calculator.breakdown.label.expected_catches"),
        trim_float(total_catches_raw * (row.current_share_pct / 100.0)),
        text("calculator.server.group.breakdown.detail.expected_catches.session_size"),
    ));

    ComputedStatBreakdown {
        kind_label: text("calculator.breakdown.kind.computed_stat"),
        title: text_with_vars(
            "calculator.server.group.breakdown.title.group",
            &[("group", &group_label)],
        ),
        value_text: percent_value_text(if show_normalized_rates {
            row.current_share_pct
        } else {
            row.weight_pct
        }),
        summary_text: if show_normalized_rates {
            text("calculator.server.group.breakdown.summary.normalized")
        } else {
            text("calculator.server.group.breakdown.summary.raw")
        },
        formula_text: if show_normalized_rates {
            text("calculator.server.group.breakdown.formula.normalized")
        } else {
            text("calculator.server.group.breakdown.formula.raw")
        },
        sections: vec![
            ComputedStatBreakdownSection {
                label: text("calculator.breakdown.section.inputs"),
                rows: input_rows,
            },
            ComputedStatBreakdownSection {
                label: text("calculator.breakdown.section.composition"),
                rows: composition_rows,
            },
        ],
        formula_terms: vec![
            computed_stat_formula_term(
                if show_normalized_rates {
                    text("calculator.server.group.breakdown.label.current_share")
                } else {
                    text("calculator.server.group.breakdown.label.current_value")
                },
                percent_value_text(if show_normalized_rates {
                    row.current_share_pct
                } else {
                    row.weight_pct
                }),
            ),
            computed_stat_formula_term(
                text("calculator.server.group.breakdown.label.raw_group_weight"),
                percent_value_text(row.weight_pct),
            ),
            computed_stat_formula_term(
                text("calculator.server.group.breakdown.label.all_group_weight_total"),
                percent_value_text(total_weight_pct),
            ),
        ],
    }
}

fn loot_species_silver_breakdown_detail(row: &LootSpeciesRow, lang: CalculatorLocale) -> String {
    let mut parts = Vec::new();
    if !row.drop_rate_text.is_empty() {
        parts.push(calculator_route_text_with_vars(
            lang,
            "calculator.server.loot.breakdown.detail.in_group_rate_value",
            &[("rate", &row.drop_rate_text)],
        ));
    }
    if !row.expected_count_text.is_empty() {
        parts.push(calculator_route_text_with_vars(
            lang,
            "calculator.server.loot.breakdown.detail.expected_catches_value",
            &[("count", &row.expected_count_text)],
        ));
    }
    if row.expected_profit_raw <= 0.0 && row.expected_count_raw > 0.0 {
        parts.push(calculator_route_text(
            lang,
            "calculator.server.loot.breakdown.detail.zero_silver_after_pricing_or_discard",
        ));
    }
    parts.join(" · ")
}

fn loot_species_count_breakdown(
    row: &LootSpeciesRow,
    total_catches_raw: f64,
    group_share_pct: f64,
    lang: CalculatorLocale,
) -> ComputedStatBreakdown {
    let text = |key: &str| calculator_route_text(lang, key);
    let text_with_vars =
        |key: &str, vars: &[(&str, &str)]| calculator_route_text_with_vars(lang, key, vars);
    let group_share_text = percent_value_text(group_share_pct);
    let in_group_rate_text = percent_value_text(row.within_group_rate_raw * 100.0);
    let group_label = calculator_group_display_label(lang, &row.group_label);

    computed_stat_breakdown(
        text_with_vars(
            "calculator.server.loot.breakdown.title.species_expected_catches",
            &[("species", &row.label)],
        ),
        row.expected_count_text.clone(),
        if row.expected_count_raw > 0.0 {
            text("calculator.server.loot.breakdown.summary.species_expected_catches.active")
        } else {
            text("calculator.server.loot.breakdown.summary.species_expected_catches.none")
        },
        text("calculator.server.loot.breakdown.formula.species_expected_catches"),
        vec![
            computed_stat_breakdown_section(
                text("calculator.breakdown.section.inputs"),
                vec![
                    computed_stat_breakdown_row_with_formula_part(
                        computed_stat_breakdown_row(
                            text_with_vars(
                                "calculator.server.loot.breakdown.label.group_share_for_group",
                                &[("group", &group_label)],
                            ),
                            group_share_text.clone(),
                            text_with_vars(
                                "calculator.server.loot.breakdown.detail.group_share_before_loot_weighting",
                                &[("group", &group_label)],
                            ),
                        ),
                        text("calculator.breakdown.label.group_share"),
                        1,
                    ),
                    computed_stat_breakdown_row_with_formula_part(
                        computed_stat_breakdown_row(
                            text("calculator.breakdown.label.in_group_rate"),
                            in_group_rate_text.clone(),
                            row.drop_rate_tooltip.clone(),
                        ),
                        text("calculator.breakdown.label.in_group_rate"),
                        2,
                    ),
                    computed_stat_breakdown_row_with_formula_part(
                        computed_stat_breakdown_row(
                            text("calculator.server.loot.breakdown.label.session_catches"),
                            trim_float(total_catches_raw),
                            text("calculator.server.loot.breakdown.detail.session_catches"),
                        ),
                        text("calculator.server.loot.breakdown.label.session_catches"),
                        3,
                    ),
                ],
            ),
            computed_stat_breakdown_section(
                text("calculator.breakdown.section.composition"),
                vec![computed_stat_breakdown_row(
                    text("calculator.breakdown.label.expected_catches"),
                    row.expected_count_text.clone(),
                    text("calculator.server.loot.breakdown.detail.species_expected_catches"),
                )],
            ),
        ],
    )
    .with_formula_terms(vec![
        computed_stat_formula_term(
            text("calculator.breakdown.label.expected_catches"),
            row.expected_count_text.clone(),
        ),
        computed_stat_formula_term(text("calculator.breakdown.label.group_share"), group_share_text),
        computed_stat_formula_term(
            text("calculator.breakdown.label.in_group_rate"),
            in_group_rate_text,
        ),
        computed_stat_formula_term(
            text("calculator.server.loot.breakdown.label.session_catches"),
            trim_float(total_catches_raw),
        ),
    ])
}

fn loot_species_silver_share_breakdown(
    row: &LootSpeciesRow,
    total_profit_raw: f64,
    lang: CalculatorLocale,
) -> ComputedStatBreakdown {
    let text = |key: &str| calculator_route_text(lang, key);
    let text_with_vars =
        |key: &str, vars: &[(&str, &str)]| calculator_route_text_with_vars(lang, key, vars);
    let base_price_text = fmt_silver(row.base_price_raw);
    let sale_multiplier_text = format!("×{}", trim_float(row.sale_multiplier_raw));
    let expected_silver_detail = if row.discarded {
        text("calculator.server.loot.breakdown.detail.silver_share.discarded")
    } else if row.expected_profit_raw <= 0.0 {
        text("calculator.server.loot.breakdown.detail.silver_share.no_priced_output")
    } else {
        text("calculator.server.loot.breakdown.detail.silver_share.expected_silver")
    };

    computed_stat_breakdown(
        text_with_vars(
            "calculator.server.loot.breakdown.title.species_silver_share",
            &[("species", &row.label)],
        ),
        row.silver_share_text.clone(),
        if row.expected_profit_raw > 0.0 {
            text("calculator.server.loot.breakdown.summary.species_silver_share.active")
        } else {
            text("calculator.server.loot.breakdown.summary.species_silver_share.none")
        },
        text("calculator.server.loot.breakdown.formula.species_silver_share"),
        vec![
            computed_stat_breakdown_section(
                text("calculator.breakdown.section.inputs"),
                vec![
                    computed_stat_breakdown_row_with_formula_part(
                        computed_stat_breakdown_row(
                            text("calculator.breakdown.label.expected_catches"),
                            row.expected_count_text.clone(),
                            text("calculator.server.loot.breakdown.detail.average_catches_this_row"),
                        ),
                        text("calculator.breakdown.label.expected_catches"),
                        1,
                    ),
                    computed_stat_breakdown_row_with_formula_part(
                        computed_stat_breakdown_row(
                            text("calculator.server.loot.breakdown.label.base_price"),
                            base_price_text.clone(),
                            text("calculator.server.loot.breakdown.detail.base_price"),
                        ),
                        text("calculator.server.loot.breakdown.label.base_price"),
                        2,
                    ),
                    computed_stat_breakdown_row_with_formula_part(
                        computed_stat_breakdown_row(
                            text("calculator.breakdown.label.trade_sale_multiplier"),
                            sale_multiplier_text.clone(),
                            text("calculator.breakdown.detail.current_sale_multiplier_after_trade_settings"),
                        ),
                        text("calculator.breakdown.label.trade_sale_multiplier"),
                        3,
                    ),
                    computed_stat_breakdown_row_with_formula_part(
                        computed_stat_breakdown_row(
                            text("calculator.server.loot.breakdown.label.all_item_expected_silver_total"),
                            fmt_silver(total_profit_raw),
                            text("calculator.server.loot.breakdown.detail.all_item_expected_silver_total"),
                        ),
                        text("calculator.server.loot.breakdown.label.all_item_expected_silver_total"),
                        4,
                    ),
                ],
            ),
            computed_stat_breakdown_section(
                text("calculator.breakdown.section.composition"),
                vec![
                    computed_stat_breakdown_row(
                        text("calculator.server.loot.breakdown.label.item_expected_silver"),
                        row.expected_profit_text.clone(),
                        expected_silver_detail,
                    ),
                    computed_stat_breakdown_row(
                        text("calculator.breakdown.label.silver_share"),
                        row.silver_share_text.clone(),
                        text("calculator.server.loot.breakdown.detail.silver_share.from_total"),
                    ),
                ],
            ),
        ],
    )
    .with_formula_terms(vec![
        computed_stat_formula_term(
            text("calculator.server.loot.breakdown.label.item_expected_silver"),
            row.expected_profit_text.clone(),
        ),
        computed_stat_formula_term(
            text("calculator.breakdown.label.expected_catches"),
            row.expected_count_text.clone(),
        ),
        computed_stat_formula_term(text("calculator.server.loot.breakdown.label.base_price"), base_price_text),
        computed_stat_formula_term(text("calculator.breakdown.label.trade_sale_multiplier"), sale_multiplier_text),
        computed_stat_formula_term(
            text("calculator.breakdown.label.silver_share"),
            row.silver_share_text.clone(),
        ),
        computed_stat_formula_term(
            text("calculator.server.loot.breakdown.label.all_item_expected_silver_total"),
            fmt_silver(total_profit_raw),
        ),
    ])
}

fn group_silver_distribution_breakdown(
    row: &LootChartRow,
    species_rows: &[LootSpeciesRow],
    total_profit_raw: f64,
    lang: CalculatorLocale,
) -> ComputedStatBreakdown {
    let text = |key: &str| calculator_route_text(lang, key);
    let text_with_vars =
        |key: &str, vars: &[(&str, &str)]| calculator_route_text_with_vars(lang, key, vars);
    let group_label = calculator_group_display_label(lang, &row.label);
    let mut contributing_rows = species_rows
        .iter()
        .filter(|species_row| {
            species_row.group_label == row.label && species_row.expected_count_raw > 0.0
        })
        .collect::<Vec<_>>();
    contributing_rows.sort_by(|left, right| {
        right
            .expected_profit_raw
            .partial_cmp(&left.expected_profit_raw)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.label.to_lowercase().cmp(&right.label.to_lowercase()))
    });

    let input_rows = if contributing_rows.is_empty() {
        vec![computed_stat_breakdown_row(
            text("calculator.server.loot.breakdown.label.contributing_loot"),
            text("calculator.server.value.unavailable"),
            text("calculator.server.loot.breakdown.detail.contributing_loot.unavailable"),
        )]
    } else {
        contributing_rows
            .into_iter()
            .map(|species_row| {
                computed_stat_breakdown_loot_species_row(
                    species_row,
                    species_row.expected_profit_text.clone(),
                    loot_species_silver_breakdown_detail(species_row, lang),
                )
            })
            .collect()
    };

    let composition_rows = vec![
        computed_stat_breakdown_row(
            text("calculator.server.loot.breakdown.label.normalized_group_share"),
            row.count_share_text.clone(),
            text("calculator.server.loot.breakdown.detail.normalized_group_share"),
        ),
        computed_stat_breakdown_row(
            text("calculator.breakdown.label.expected_catches"),
            row.expected_count_text.clone(),
            text("calculator.server.loot.breakdown.detail.group_expected_catches"),
        ),
        computed_stat_breakdown_row(
            text("calculator.breakdown.label.group_expected_silver"),
            row.expected_profit_text.clone(),
            text("calculator.server.loot.breakdown.detail.group_expected_silver"),
        ),
        computed_stat_breakdown_row(
            text("calculator.server.loot.breakdown.label.all_group_expected_silver_total"),
            fmt_silver(total_profit_raw),
            text("calculator.server.loot.breakdown.detail.all_group_expected_silver_total"),
        ),
        computed_stat_breakdown_row(
            text("calculator.breakdown.label.silver_share"),
            row.silver_share_text.clone(),
            text("calculator.server.loot.breakdown.detail.group_silver_share_from_total"),
        ),
    ];

    ComputedStatBreakdown {
        kind_label: text("calculator.breakdown.kind.computed_stat"),
        title: text_with_vars(
            "calculator.server.loot.breakdown.title.group_silver_share",
            &[("group", &group_label)],
        ),
        value_text: row.silver_share_text.clone(),
        summary_text: if row.expected_profit_raw > 0.0 {
            text("calculator.server.loot.breakdown.summary.group_silver_share.active")
        } else {
            text("calculator.server.loot.breakdown.summary.group_silver_share.none")
        },
        formula_text: text("calculator.server.loot.breakdown.formula.group_silver_share"),
        sections: vec![
            ComputedStatBreakdownSection {
                label: text("calculator.breakdown.section.inputs"),
                rows: input_rows,
            },
            ComputedStatBreakdownSection {
                label: text("calculator.breakdown.section.composition"),
                rows: composition_rows,
            },
        ],
        formula_terms: vec![
            computed_stat_formula_term(
                text("calculator.breakdown.label.silver_share"),
                row.silver_share_text.clone(),
            ),
            computed_stat_formula_term(
                text("calculator.breakdown.label.group_expected_silver"),
                row.expected_profit_text.clone(),
            ),
            computed_stat_formula_term(
                text("calculator.server.loot.breakdown.label.all_group_expected_silver_total"),
                fmt_silver(total_profit_raw),
            ),
        ],
    }
}

fn zone_loot_group_drop_rate_fields(
    chart_row: Option<&FishGroupChartRow>,
) -> (String, String, String) {
    let Some(chart_row) = chart_row else {
        return (String::new(), String::new(), String::new());
    };
    if chart_row.current_share_pct <= 0.0 {
        return (String::new(), String::new(), String::new());
    }
    (
        percent_value_text(chart_row.current_share_pct),
        fish_group_drop_rate_source_kind(chart_row),
        fish_group_drop_rate_tooltip(chart_row),
    )
}

fn zone_loot_catch_methods(methods: &[String]) -> Vec<String> {
    let mut has_rod = false;
    let mut has_harpoon = false;
    for method in methods {
        match method.trim().to_ascii_lowercase().as_str() {
            "rod" => has_rod = true,
            "harpoon" => has_harpoon = true,
            _ => {}
        }
    }

    let mut normalized = Vec::with_capacity(2);
    if has_rod {
        normalized.push("rod".to_string());
    }
    if has_harpoon {
        normalized.push("harpoon".to_string());
    }
    if normalized.is_empty() {
        normalized.push("rod".to_string());
    }
    normalized
}

fn zone_loot_chart_condition_fields(
    row: &FishGroupChartRow,
    lang: CalculatorLocale,
) -> (String, String) {
    let fallback_text = if row.base_share_pct > 0.0 {
        format!(
            "{} {}",
            calculator_route_text(lang, "calculator.server.group.zone_base_rate"),
            percent_value_text(row.base_share_pct),
        )
    } else {
        row.bonus_text.clone()
    };

    let condition_text = row
        .rate_inputs
        .iter()
        .filter(|input| !input.label.trim().is_empty() && !input.value_text.trim().is_empty())
        .take(2)
        .map(|input| format!("{} {}", input.label, input.value_text))
        .collect::<Vec<_>>()
        .join(" · ");

    let condition_tooltip = row
        .rate_inputs
        .iter()
        .filter(|input| !input.label.trim().is_empty() && !input.value_text.trim().is_empty())
        .map(|input| format!("{}: {}", input.label, input.value_text))
        .collect::<Vec<_>>()
        .join(" | ");

    (
        if condition_text.is_empty() {
            fallback_text
        } else {
            condition_text
        },
        condition_tooltip,
    )
}

fn zone_loot_parse_condition_value(predicate: &str, prefix: &str) -> Option<i64> {
    predicate
        .strip_prefix(prefix)
        .and_then(|value| value.trim().parse::<i64>().ok())
}

fn zone_loot_lifeskill_label(order: i64, catalog: &CalculatorCatalogResponse) -> Option<String> {
    catalog
        .lifeskill_levels
        .iter()
        .find(|entry| i64::from(entry.order) == order)
        .map(|entry| entry.name.clone())
}

fn zone_loot_humanize_condition(
    raw: &str,
    lang: CalculatorLocale,
    catalog: &CalculatorCatalogResponse,
) -> Vec<String> {
    let predicates = raw
        .split(';')
        .map(str::trim)
        .filter(|predicate| !predicate.is_empty())
        .collect::<Vec<_>>();
    if predicates.is_empty() {
        return Vec::new();
    }

    let mut labels = Vec::new();
    let mastery_min = predicates.iter().find_map(|predicate| {
        zone_loot_parse_condition_value(predicate, "lifestat(1,1)>").map(|value| value + 1)
    });
    let mastery_max = predicates.iter().find_map(|predicate| {
        zone_loot_parse_condition_value(predicate, "lifestat(1,1)<").map(|value| value - 1)
    });
    if mastery_min.is_some() || mastery_max.is_some() {
        let label = calculator_route_text(lang, "calculator.server.field.mastery");
        labels.push(match (mastery_min, mastery_max) {
            (Some(min), Some(max)) if min <= max => format!("{label} {min}-{max}"),
            (Some(min), _) => format!("{label} {min}+"),
            (_, Some(max)) => format!("{label} <= {max}"),
            _ => label,
        });
    }

    if let Some(level_threshold) = predicates.iter().find_map(|predicate| {
        zone_loot_parse_condition_value(predicate, "getLifeLevel(1)>").map(|value| value + 1)
    }) {
        let label = calculator_route_text(lang, "calculator.server.field.fishing_level");
        let level_name = zone_loot_lifeskill_label(level_threshold, catalog)
            .unwrap_or_else(|| level_threshold.to_string());
        labels.push(format!("{label} {level_name}+"));
    }

    if labels.is_empty() {
        labels.push(raw.trim().to_string());
    }
    labels
}

fn zone_loot_raw_condition_fields(
    conditions: &[String],
    lang: CalculatorLocale,
    catalog: &CalculatorCatalogResponse,
) -> Option<(String, String)> {
    if conditions.is_empty() {
        return None;
    }

    let mut labels = Vec::<String>::new();
    for condition in conditions {
        for label in zone_loot_humanize_condition(condition, lang, catalog) {
            if !label.trim().is_empty() && !labels.contains(&label) {
                labels.push(label);
            }
        }
    }
    if labels.is_empty() {
        return None;
    }

    Some((labels.join(" · "), labels.join(" | ")))
}

fn zone_loot_group_condition_fields(
    chart_row: Option<&FishGroupChartRow>,
    conditions: &[String],
    data: &CalculatorData,
) -> (String, String) {
    if let Some(fields) = zone_loot_raw_condition_fields(conditions, data.lang, &data.catalog) {
        return fields;
    }
    chart_row
        .map(|chart_row| zone_loot_chart_condition_fields(chart_row, data.lang))
        .unwrap_or_else(|| (String::new(), String::new()))
}

fn zone_loot_group_conditions_by_slot(
    entries: &[CalculatorZoneLootEntry],
) -> HashMap<u8, Vec<String>> {
    let mut group_conditions_by_slot = HashMap::<u8, Vec<String>>::new();
    for entry in entries {
        for condition in &entry.group_conditions_raw {
            let conditions = group_conditions_by_slot.entry(entry.slot_idx).or_default();
            if !condition.trim().is_empty() && !conditions.contains(condition) {
                conditions.push(condition.clone());
            }
        }
    }
    group_conditions_by_slot
}

fn zone_loot_summary_species_row_from_loot_species(
    row: &LootSpeciesRow,
    data: &CalculatorData,
) -> ZoneLootSummarySpeciesRow {
    ZoneLootSummarySpeciesRow {
        slot_idx: row.slot_idx,
        group_label: calculator_group_display_label(data.lang, &row.group_label),
        label: row.label.clone(),
        icon_url: row.icon_url.clone(),
        icon_grade_tone: row.icon_grade_tone.clone(),
        fill_color: row.fill_color.to_string(),
        stroke_color: row.stroke_color.to_string(),
        text_color: row.text_color.to_string(),
        drop_rate_text: row.drop_rate_text.clone(),
        drop_rate_source_kind: row.drop_rate_source_kind.clone(),
        drop_rate_tooltip: row.drop_rate_tooltip.clone(),
        presence_text: row.presence_text.clone(),
        presence_source_kind: row.presence_source_kind.clone(),
        presence_tooltip: row.presence_tooltip.clone().unwrap_or_default(),
        catch_methods: row.catch_methods.clone(),
    }
}

fn zone_loot_summary_species_row_from_entry(
    signals: &CalculatorSignals,
    data: &CalculatorData,
    entry: &CalculatorZoneLootEntry,
    chart_row: Option<&FishGroupChartRow>,
) -> ZoneLootSummarySpeciesRow {
    let (group_label, fill_color, stroke_color, text_color) =
        zone_loot_group_values(data.lang, entry.slot_idx, chart_row);
    ZoneLootSummarySpeciesRow {
        slot_idx: entry.slot_idx,
        group_label,
        label: entry.name.clone(),
        icon_url: entry
            .icon
            .as_deref()
            .map(|icon| absolute_public_asset_url(data.cdn_base_url.as_str(), icon)),
        icon_grade_tone: item_grade_tone(entry.grade.as_deref()).to_string(),
        fill_color,
        stroke_color,
        text_color,
        drop_rate_text: loot_species_drop_rate_text(signals, entry),
        drop_rate_source_kind: loot_species_drop_rate_source_kind(entry).to_string(),
        drop_rate_tooltip: loot_species_drop_rate_tooltip(signals, entry, data.lang),
        presence_text: loot_species_presence_text(entry, data.lang),
        presence_source_kind: loot_species_presence_source_kind(entry),
        presence_tooltip: loot_species_presence_tooltip(entry, data.lang).unwrap_or_default(),
        catch_methods: zone_loot_catch_methods(&entry.catch_methods),
    }
}

fn sort_zone_loot_summary_species_rows(rows: &mut [ZoneLootSummarySpeciesRow]) {
    rows.sort_by(|left, right| {
        zone_loot_slot_sort_key(left.slot_idx)
            .cmp(&zone_loot_slot_sort_key(right.slot_idx))
            .then_with(|| left.label.to_lowercase().cmp(&right.label.to_lowercase()))
    });
}

fn zone_loot_summary_condition_options_for_slot(
    condition_options_by_slot: &HashMap<u8, Vec<ZoneLootSummaryConditionOption>>,
    slot_idx: u8,
) -> Vec<ZoneLootSummaryConditionOption> {
    condition_options_by_slot
        .get(&slot_idx)
        .cloned()
        .unwrap_or_default()
}

fn zone_loot_summary_condition_option_fields(
    option: &CalculatorLootBranchOption,
    data: &CalculatorData,
) -> (String, String) {
    zone_loot_raw_condition_fields(&option.conditions, data.lang, &data.catalog).unwrap_or_else(
        || {
            let default_text =
                calculator_route_text(data.lang, "calculator.layout_presets.default");
            (default_text.clone(), default_text)
        },
    )
}

fn build_zone_loot_summary_condition_options(
    signals: &CalculatorSignals,
    data: &CalculatorData,
    base_entries: &[CalculatorZoneLootEntry],
) -> HashMap<u8, Vec<ZoneLootSummaryConditionOption>> {
    let items_by_key = data
        .catalog
        .items
        .iter()
        .map(|item| (item.key.as_str(), item))
        .collect::<HashMap<_, _>>();
    let fish_group_chart = derive_fish_group_chart(signals, data, &items_by_key);
    let group_row_by_slot = fish_group_chart
        .rows
        .iter()
        .enumerate()
        .map(|(index, row)| ((index + 1) as u8, row))
        .collect::<HashMap<_, _>>();
    let mut condition_options_by_slot = HashMap::<u8, Vec<ZoneLootSummaryConditionOption>>::new();

    for ((slot_idx, item_main_group_key), mut options) in
        calculator_loot_branch_options(base_entries)
    {
        options.sort_by_key(|option| option.option_idx);
        options.dedup_by_key(|option| option.option_idx);
        let has_scalar_conditions = options.iter().any(|option| {
            calculator_branch_option_has_mastery(option)
                || calculator_branch_option_has_lifeskill(option)
        });
        if options.len() <= 1 || !has_scalar_conditions {
            continue;
        }

        let active_option_idx = calculator_selected_branch_option_idx(&options, signals)
            .or_else(|| options.first().map(|option| option.option_idx));
        let slot_options = condition_options_by_slot.entry(slot_idx).or_default();
        for option in options {
            let mut forced_branch_options = HashMap::new();
            forced_branch_options.insert((slot_idx, item_main_group_key), option.option_idx);
            let mut option_entries =
                apply_calculator_condition_context_to_loot_entries_with_branch_overrides(
                    signals,
                    base_entries,
                    &forced_branch_options,
                );
            option_entries =
                apply_zone_overlay_to_loot_entries(signals, &signals.zone, &option_entries);
            let chart_row = group_row_by_slot.get(&slot_idx).copied();
            let mut species_rows = option_entries
                .iter()
                .filter(|entry| entry.slot_idx == slot_idx && entry.within_group_rate > 0.0)
                .map(|entry| {
                    zone_loot_summary_species_row_from_entry(signals, data, entry, chart_row)
                })
                .collect::<Vec<_>>();
            sort_zone_loot_summary_species_rows(&mut species_rows);

            let (condition_text, condition_tooltip) =
                zone_loot_summary_condition_option_fields(&option, data);
            if slot_options
                .iter()
                .any(|existing| existing.condition_text == condition_text)
            {
                continue;
            }
            slot_options.push(ZoneLootSummaryConditionOption {
                condition_text,
                condition_tooltip,
                active: active_option_idx == Some(option.option_idx),
                species_rows,
            });
        }
    }

    condition_options_by_slot
        .retain(|_, options| options.len() > 1 && options.iter().any(|option| option.active));
    condition_options_by_slot
}

fn loot_species_presence_scope_text(
    evidence: &CalculatorZoneLootEvidence,
    include_structural_ids: bool,
    lang: CalculatorLocale,
) -> String {
    let text = |key: &str| calculator_route_text(lang, key);
    let text_with_vars =
        |key: &str, vars: &[(&str, &str)]| calculator_route_text_with_vars(lang, key, vars);
    if evidence.source_family == "ranking" {
        return match evidence.scope.as_str() {
            "ring_full" => text("calculator.server.presence.scope.ring_full"),
            "ring_partial" => text("calculator.server.presence.scope.ring_partial"),
            _ => text("calculator.server.presence.scope.ring_default"),
        };
    }
    if let Some(subgroup_key) = evidence.subgroup_key {
        if let Some(slot_idx) = evidence.slot_idx {
            let group_label = calculator_group_display_label_for_slot(lang, slot_idx)
                .unwrap_or_else(|| fish_group_label(slot_idx).unwrap_or_default().to_string());
            if include_structural_ids {
                return text_with_vars(
                    "calculator.server.presence.scope.subgroup.group_key",
                    &[("group", &group_label), ("key", &subgroup_key.to_string())],
                );
            }
            return text_with_vars(
                "calculator.server.presence.scope.subgroup.group",
                &[("group", &group_label)],
            );
        }
        if include_structural_ids {
            return text_with_vars(
                "calculator.server.presence.scope.subgroup.key",
                &[("key", &subgroup_key.to_string())],
            );
        }
        return text("calculator.server.presence.scope.subgroup");
    }
    if let Some(slot_idx) = evidence.slot_idx {
        let slot_label = calculator_group_display_label_for_slot(lang, slot_idx)
            .unwrap_or_else(|| fish_group_label(slot_idx).unwrap_or_default().to_string());
        if let Some(item_main_group_key) = evidence.item_main_group_key {
            if include_structural_ids {
                return text_with_vars(
                    "calculator.server.presence.scope.group.group_key",
                    &[
                        ("group", &slot_label),
                        ("key", &item_main_group_key.to_string()),
                    ],
                );
            }
        }
        return text_with_vars(
            "calculator.server.presence.scope.group.group",
            &[("group", &slot_label)],
        );
    }
    if let Some(item_main_group_key) = evidence.item_main_group_key {
        if include_structural_ids {
            return text_with_vars(
                "calculator.server.presence.scope.group.key",
                &[("key", &item_main_group_key.to_string())],
            );
        }
        return text("calculator.server.presence.scope.group");
    }
    match evidence.scope.as_str() {
        "group_inferred" => text("calculator.server.presence.scope.group_inferred"),
        "group" => text("calculator.server.presence.scope.group"),
        _ => text("calculator.server.presence.scope.zone_only"),
    }
}

fn loot_species_presence_priority(evidence: &CalculatorZoneLootEvidence) -> u8 {
    match (
        evidence.source_family.as_str(),
        evidence.scope.as_str(),
        evidence.status.as_deref(),
    ) {
        ("ranking", "ring_full", _) => 5,
        ("community", _, Some("confirmed")) => 4,
        ("community", _, Some("guessed")) => 4,
        ("ranking", "ring_partial", _) => 3,
        ("community", _, Some("unconfirmed")) => 2,
        ("community", _, Some("data_incomplete")) => 1,
        _ => 0,
    }
}

fn loot_species_presence_evidence(
    entry: &CalculatorZoneLootEntry,
) -> Vec<&CalculatorZoneLootEvidence> {
    let mut evidence = entry.evidence.iter().collect::<Vec<_>>();
    evidence.retain(|evidence| evidence.claim_kind == "presence");
    evidence.sort_by(|left, right| {
        loot_species_presence_priority(right)
            .cmp(&loot_species_presence_priority(left))
            .then_with(|| {
                right
                    .claim_count
                    .unwrap_or_default()
                    .cmp(&left.claim_count.unwrap_or_default())
            })
            .then_with(|| left.source_family.cmp(&right.source_family))
            .then_with(|| left.scope.cmp(&right.scope))
    });
    evidence
}

fn loot_species_presence_line(
    evidence: &CalculatorZoneLootEvidence,
    include_structural_ids: bool,
    include_source_id: bool,
    lang: CalculatorLocale,
) -> String {
    let text = |key: &str| calculator_route_text(lang, key);
    let text_with_vars =
        |key: &str, vars: &[(&str, &str)]| calculator_route_text_with_vars(lang, key, vars);
    let compact_claims = evidence
        .claim_count
        .map(|count| format!("×{count}"))
        .unwrap_or_default();
    let spaced_claims = evidence
        .claim_count
        .map(|count| format!(" ×{count}"))
        .unwrap_or_default();
    match evidence.source_family.as_str() {
        "ranking" => {
            let scope = loot_species_presence_scope_text(evidence, include_structural_ids, lang);
            let mut value = text_with_vars(
                "calculator.server.presence.line.ranking",
                &[("scope", &scope), ("claims", &spaced_claims)],
            );
            if include_source_id {
                if let Some(source_id) = evidence
                    .source_id
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                {
                    value = text_with_vars(
                        "calculator.server.presence.line.ranking_with_source",
                        &[
                            ("scope", &scope),
                            ("claims", &spaced_claims),
                            ("source", source_id),
                        ],
                    );
                }
            }
            value
        }
        "community" => {
            let status = match evidence.status.as_deref().unwrap_or_default() {
                "confirmed" => text("calculator.server.presence.status.community.confirmed"),
                "guessed" => text("calculator.server.presence.status.community.guessed"),
                "data_incomplete" => {
                    text("calculator.server.presence.status.community.data_incomplete")
                }
                _ => text("calculator.server.presence.status.community.unconfirmed"),
            };
            let scope = loot_species_presence_scope_text(evidence, include_structural_ids, lang);
            let mut value = text_with_vars(
                "calculator.server.presence.line.community",
                &[
                    ("status", &status),
                    ("claims", &compact_claims),
                    ("scope", &scope),
                ],
            );
            if include_source_id {
                if let Some(source_id) = evidence
                    .source_id
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                {
                    value = text_with_vars(
                        "calculator.server.presence.line.community_with_source",
                        &[
                            ("status", &status),
                            ("claims", &compact_claims),
                            ("scope", &scope),
                            ("source", source_id),
                        ],
                    );
                }
            }
            value
        }
        _ => {
            let scope = loot_species_presence_scope_text(evidence, include_structural_ids, lang);
            text_with_vars(
                "calculator.server.presence.line.generic",
                &[("claims", &compact_claims), ("scope", &scope)],
            )
        }
    }
}

fn loot_species_presence_text(
    entry: &CalculatorZoneLootEntry,
    lang: CalculatorLocale,
) -> Option<String> {
    loot_species_presence_evidence(entry)
        .into_iter()
        .next()
        .map(|evidence| loot_species_presence_line(evidence, false, false, lang))
}

fn loot_species_presence_tooltip(
    entry: &CalculatorZoneLootEntry,
    lang: CalculatorLocale,
) -> Option<String> {
    let parts = loot_species_presence_evidence(entry)
        .into_iter()
        .map(|evidence| loot_species_presence_line(evidence, true, true, lang))
        .collect::<Vec<_>>();
    (!parts.is_empty()).then_some(parts.join(" | "))
}

fn loot_species_presence_source_kind(entry: &CalculatorZoneLootEntry) -> String {
    let mut families = loot_species_presence_evidence(entry)
        .into_iter()
        .map(|evidence| evidence.source_family.as_str())
        .collect::<Vec<_>>();
    families.sort_unstable();
    families.dedup();
    match families.as_slice() {
        [] => String::new(),
        ["community"] => "community".to_string(),
        ["ranking"] => "ranking".to_string(),
        ["database"] => "database".to_string(),
        [_] => families[0].to_string(),
        _ => "mixed".to_string(),
    }
}

fn loot_species_drop_rate_text(
    signals: &CalculatorSignals,
    entry: &CalculatorZoneLootEntry,
) -> String {
    let rate = loot_species_rate_evidence(entry)
        .and_then(|evidence| evidence_display_rate(signals, evidence))
        .unwrap_or(entry.within_group_rate);
    format!("{}%", format_evidence_percent(rate))
}

fn loot_species_drop_rate_source_kind(entry: &CalculatorZoneLootEntry) -> &'static str {
    if entry.overlay.slot_overlay_active {
        return "overlay";
    }
    loot_species_rate_evidence(entry)
        .map(|evidence| match evidence.source_family.as_str() {
            "database" => "database",
            "community" => "community",
            _ => "derived",
        })
        .unwrap_or("derived")
}

fn loot_species_drop_rate_tooltip(
    signals: &CalculatorSignals,
    entry: &CalculatorZoneLootEntry,
    lang: CalculatorLocale,
) -> String {
    let text_with_vars =
        |key: &str, vars: &[(&str, &str)]| calculator_route_text_with_vars(lang, key, vars);
    if entry.overlay.slot_overlay_active {
        let effective_rate_text = percent_value_text(entry.within_group_rate * 100.0);
        let base_detail = if let Some(rate) = entry
            .evidence
            .iter()
            .find(|evidence| {
                evidence.source_family == "database" && evidence.claim_kind == "in_group_rate"
            })
            .and_then(|evidence| evidence.rate)
        {
            text_with_vars(
                "calculator.server.loot.tooltip.overlay.base_db_raw_rate",
                &[("rate", &format!("{}%", format_evidence_percent(rate)))],
            )
        } else {
            String::new()
        };
        if let Some(explicit_rate_percent) = entry.overlay.explicit_rate_percent {
            return text_with_vars(
                "calculator.server.loot.tooltip.overlay.explicit",
                &[
                    ("base", &percent_value_text(explicit_rate_percent)),
                    ("rate", &effective_rate_text),
                    ("base_detail", &base_detail),
                ],
            );
        }
        if entry.overlay.added {
            return text_with_vars(
                "calculator.server.loot.tooltip.overlay.added",
                &[
                    ("rate", &effective_rate_text),
                    ("base_detail", &base_detail),
                ],
            );
        }
        return text_with_vars(
            "calculator.server.loot.tooltip.overlay.changed",
            &[
                ("rate", &effective_rate_text),
                ("base_detail", &base_detail),
            ],
        );
    }

    let db_rate_text = entry
        .evidence
        .iter()
        .find(|evidence| {
            evidence.source_family == "database" && evidence.claim_kind == "in_group_rate"
        })
        .and_then(|evidence| evidence_display_rate(signals, evidence))
        .map(|rate| {
            text_with_vars(
                "calculator.server.loot.tooltip.db_rate",
                &[("rate", &format!("{}%", format_evidence_percent(rate)))],
            )
        });

    let guessed_rate_text = entry
        .evidence
        .iter()
        .find(|evidence| {
            evidence.source_family == "community" && evidence.claim_kind == "guessed_in_group_rate"
        })
        .and_then(|evidence| evidence_display_rate(signals, evidence))
        .map(|rate| {
            text_with_vars(
                "calculator.server.loot.tooltip.community_guess",
                &[("rate", &format!("{}%", format_evidence_percent(rate)))],
            )
        });

    let mut parts = Vec::new();
    if let Some(text) = db_rate_text {
        parts.push(text);
    }
    if let Some(text) = guessed_rate_text {
        parts.push(text);
    }
    if parts.is_empty() {
        return text_with_vars(
            "calculator.server.loot.tooltip.derived",
            &[(
                "rate",
                &format!("{}%", format_evidence_percent(entry.within_group_rate)),
            )],
        );
    }
    parts.join(" · ")
}

fn loot_species_evidence_text(
    signals: &CalculatorSignals,
    entry: &CalculatorZoneLootEntry,
    lang: CalculatorLocale,
) -> String {
    let mut parts = Vec::new();
    if loot_species_drop_rate_source_kind(entry) != "derived"
        || loot_species_presence_evidence(entry).is_empty()
    {
        parts.push(loot_species_drop_rate_tooltip(signals, entry, lang));
    }
    if let Some(presence_text) = loot_species_presence_tooltip(entry, lang)
        .or_else(|| loot_species_presence_text(entry, lang))
    {
        parts.push(presence_text);
    }
    if parts.is_empty() {
        parts.push(loot_species_drop_rate_tooltip(signals, entry, lang));
    }
    parts.join(" · ")
}

fn percent_value_text(value_pct: f64) -> String {
    let max_decimals = if value_pct.abs() < 1.0 { 4 } else { 2 };
    let compact = trim_float_to(value_pct, max_decimals);
    if compact == "0" && value_pct != 0.0 {
        format!("{}%", trim_float_to(value_pct, 6))
    } else {
        format!("{compact}%")
    }
}

fn derive_loot_chart(
    signals: &CalculatorSignals,
    data: &CalculatorData,
    fish_group_chart: &FishGroupChart,
    total_catches_raw: f64,
    fish_multiplier_raw: f64,
) -> LootChart {
    let text = |key: &str| calculator_route_text(data.lang, key);
    let text_with_vars =
        |key: &str, vars: &[(&str, &str)]| calculator_route_text_with_vars(data.lang, key, vars);
    if !fish_group_chart.available {
        return LootChart {
            available: false,
            note: text("calculator.server.loot.note.unavailable"),
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
    let total_group_weight_pct = fish_group_chart
        .rows
        .iter()
        .map(|row| row.weight_pct.max(0.0))
        .sum::<f64>();

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
    let group_conditions_by_slot = zone_loot_group_conditions_by_slot(&data.zone_loot_entries);

    let mut group_profit_by_slot = HashMap::<u8, f64>::new();
    let mut species_rows = Vec::new();
    for entry in &data.zone_loot_entries {
        if entry.within_group_rate <= 0.0 {
            continue;
        }
        let Some(group_row) = group_row_by_slot.get(&entry.slot_idx) else {
            continue;
        };
        if group_row.current_share_pct <= 0.0 {
            continue;
        }
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
        let drop_rate_text = loot_species_drop_rate_text(signals, entry);
        let drop_rate_source_kind = loot_species_drop_rate_source_kind(entry).to_string();
        let drop_rate_tooltip = loot_species_drop_rate_tooltip(signals, entry, data.lang);
        let presence_text = loot_species_presence_text(entry, data.lang);
        let presence_tooltip = loot_species_presence_tooltip(entry, data.lang);
        *group_profit_by_slot.entry(entry.slot_idx).or_default() += expected_profit_raw;
        species_rows.push(LootSpeciesRow {
            slot_idx: entry.slot_idx,
            item_id: entry.item_id,
            group_label: group_row.label,
            label: entry.name.clone(),
            icon_url: entry
                .icon
                .as_deref()
                .map(|icon| absolute_public_asset_url(data.cdn_base_url.as_str(), icon)),
            icon_grade_tone: item_grade_tone(entry.grade.as_deref()).to_string(),
            fill_color: group_row.fill_color,
            stroke_color: group_row.stroke_color,
            text_color: group_row.text_color,
            connector_color: group_row.connector_color,
            expected_count_raw,
            expected_profit_raw,
            expected_count_text: trim_float(expected_count_raw),
            expected_profit_text: fmt_silver(expected_profit_raw),
            silver_share_text: String::new(),
            rate_text: drop_rate_text.clone(),
            rate_source_kind: drop_rate_source_kind.clone(),
            rate_tooltip: drop_rate_tooltip.clone(),
            drop_rate_text,
            drop_rate_source_kind,
            drop_rate_tooltip,
            presence_text: presence_text.clone(),
            presence_source_kind: loot_species_presence_source_kind(entry),
            presence_tooltip,
            evidence_text: loot_species_evidence_text(signals, entry, data.lang),
            catch_methods: zone_loot_catch_methods(&entry.catch_methods),
            count_breakdown: String::new(),
            silver_breakdown: String::new(),
            within_group_rate_raw: entry.within_group_rate,
            base_price_raw,
            sale_multiplier_raw,
            discarded,
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

    let total_profit_raw = group_profit_by_slot.values().sum::<f64>();

    for species_row in &mut species_rows {
        let silver_share = if total_profit_raw > 0.0 {
            (species_row.expected_profit_raw / total_profit_raw) * 100.0
        } else {
            0.0
        };
        species_row.silver_share_text = percent_value_text(silver_share);
        let group_share_pct = group_share_by_slot
            .get(&species_row.slot_idx)
            .copied()
            .unwrap_or_default()
            * 100.0;
        species_row.count_breakdown = stat_breakdown_json(loot_species_count_breakdown(
            species_row,
            total_catches_raw,
            group_share_pct,
            data.lang,
        ));
        species_row.silver_breakdown = stat_breakdown_json(loot_species_silver_share_breakdown(
            species_row,
            total_profit_raw,
            data.lang,
        ));
        if signals.show_silver_amounts {
            species_row.rate_text = percent_value_text(silver_share);
            species_row.rate_source_kind = "derived".to_string();
            species_row.rate_tooltip = text_with_vars(
                "calculator.server.loot.tooltip.derived_total_expected_silver",
                &[("share", &percent_value_text(silver_share))],
            );
        } else {
            species_row.rate_text = species_row.drop_rate_text.clone();
            species_row.rate_source_kind = species_row.drop_rate_source_kind.clone();
            species_row.rate_tooltip = species_row.drop_rate_tooltip.clone();
        }
    }

    let mut rows = fish_group_chart
        .rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let slot_idx = (index + 1) as u8;
            let (condition_text, condition_tooltip) = zone_loot_raw_condition_fields(
                group_conditions_by_slot
                    .get(&slot_idx)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
                data.lang,
                &data.catalog,
            )
            .unwrap_or_else(|| (String::new(), String::new()));
            let expected_count_raw = total_catches_raw * (row.current_share_pct / 100.0);
            let expected_profit_raw = group_profit_by_slot
                .get(&slot_idx)
                .copied()
                .unwrap_or_default();
            let silver_share_pct = if total_profit_raw > 0.0 {
                (expected_profit_raw / total_profit_raw) * 100.0
            } else {
                0.0
            };
            LootChartRow {
                label: row.label,
                fill_color: row.fill_color,
                stroke_color: row.stroke_color,
                text_color: row.text_color,
                connector_color: row.connector_color,
                drop_rate_source_kind: fish_group_drop_rate_source_kind(row),
                drop_rate_tooltip: fish_group_drop_rate_tooltip(row),
                condition_text,
                condition_tooltip,
                expected_count_raw,
                expected_profit_raw,
                expected_count_text: trim_float(expected_count_raw),
                expected_profit_text: fmt_silver(expected_profit_raw),
                current_share_pct: row.current_share_pct,
                count_share_text: percent_value_text(row.current_share_pct),
                silver_share_text: percent_value_text(silver_share_pct),
                count_breakdown: stat_breakdown_json(fish_group_distribution_breakdown(
                    row,
                    total_catches_raw,
                    total_group_weight_pct,
                    true,
                    data.lang,
                )),
                silver_breakdown: String::new(),
            }
        })
        .collect::<Vec<_>>();
    for row in &mut rows {
        row.silver_breakdown = stat_breakdown_json(group_silver_distribution_breakdown(
            row,
            &species_rows,
            total_profit_raw,
            data.lang,
        ));
    }
    let profit_per_catch_raw = if total_catches_raw > 0.0 {
        total_profit_raw / total_catches_raw
    } else {
        0.0
    };
    let profit_per_hour_raw = fish_per_hour_raw * profit_per_catch_raw;

    LootChart {
        available: true,
        note: text("calculator.server.loot.note.available"),
        fish_multiplier_text: format!("×{}", trim_float(fish_multiplier_raw)),
        trade_bargain_bonus_text: format!("+{}%", trim_float(bargain_bonus_raw * 100.0)),
        trade_sale_multiplier_text: if signals.apply_trade_modifiers {
            format!("×{}", trim_float(sale_multiplier_raw))
        } else {
            text("calculator.server.loot.trade_sale_multiplier.off")
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

#[cfg(test)]
fn derive_zone_loot_summary_response(
    signals: &CalculatorSignals,
    data: &CalculatorData,
    zone: &ZoneEntry,
) -> ZoneLootSummaryResponse {
    derive_zone_loot_summary_response_with_condition_options(signals, data, zone, &HashMap::new())
}

fn derive_zone_loot_summary_response_with_condition_options(
    signals: &CalculatorSignals,
    data: &CalculatorData,
    zone: &ZoneEntry,
    condition_options_by_slot: &HashMap<u8, Vec<ZoneLootSummaryConditionOption>>,
) -> ZoneLootSummaryResponse {
    let text = |key: &str| calculator_route_text(data.lang, key);
    let items_by_key = data
        .catalog
        .items
        .iter()
        .map(|item| (item.key.as_str(), item))
        .collect::<HashMap<_, _>>();
    let fish_group_chart = derive_fish_group_chart(signals, data, &items_by_key);
    let derived = derive_signals(signals, data);
    let loot_chart = derive_loot_chart(
        signals,
        data,
        &fish_group_chart,
        derived.loot_total_catches_raw,
        derived.fish_multiplier_raw,
    );
    let group_row_by_slot = fish_group_chart
        .rows
        .iter()
        .enumerate()
        .map(|(index, row)| ((index + 1) as u8, row))
        .collect::<HashMap<_, _>>();
    let group_conditions_by_slot = zone_loot_group_conditions_by_slot(&data.zone_loot_entries);
    let mut group_methods_by_slot = HashMap::<u8, Vec<String>>::new();
    let mut rod_entries = Vec::<&CalculatorZoneLootEntry>::new();
    let mut harpoon_entries = Vec::<&CalculatorZoneLootEntry>::new();
    for entry in &data.zone_loot_entries {
        let methods = zone_loot_catch_methods(&entry.catch_methods);
        if !methods.is_empty() {
            let group_methods = group_methods_by_slot.entry(entry.slot_idx).or_default();
            for method in &methods {
                if !group_methods.contains(method) {
                    group_methods.push(method.clone());
                }
            }
        }

        if methods.iter().any(|method| method == "rod") {
            rod_entries.push(entry);
        }
        if methods.iter().any(|method| method == "harpoon") {
            harpoon_entries.push(entry);
        }
    }
    let zone_overlay = zone_overlay_for_signals(signals, &signals.zone);
    let overlay_active = zone_overlay_has_changes(zone_overlay);
    let rows = filtered_loot_flow_rows(&loot_chart.rows, &loot_chart.species_rows);
    let visible_group_labels = rows.iter().map(|row| row.label).collect::<HashSet<_>>();
    let default_group_methods = |slot_idx: u8| {
        if slot_idx == 6 {
            vec!["harpoon".to_string()]
        } else {
            vec!["rod".to_string()]
        }
    };
    let mut summary_groups = rows
        .iter()
        .map(|row| {
            let slot_idx = fish_group_slot_idx(row.label).unwrap_or(0);
            let (condition_text, condition_tooltip) = zone_loot_group_condition_fields(
                group_row_by_slot.get(&slot_idx).copied(),
                group_conditions_by_slot
                    .get(&slot_idx)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
                data,
            );
            ZoneLootSummaryGroupRow {
                slot_idx,
                label: calculator_group_display_label(data.lang, row.label),
                fill_color: row.fill_color.to_string(),
                stroke_color: row.stroke_color.to_string(),
                text_color: row.text_color.to_string(),
                drop_rate_text: row.count_share_text.clone(),
                drop_rate_source_kind: row.drop_rate_source_kind.clone(),
                drop_rate_tooltip: row.drop_rate_tooltip.clone(),
                condition_text,
                condition_tooltip,
                catch_methods: group_methods_by_slot
                    .get(&slot_idx)
                    .cloned()
                    .unwrap_or_else(|| default_group_methods(slot_idx)),
                condition_options: zone_loot_summary_condition_options_for_slot(
                    condition_options_by_slot,
                    slot_idx,
                ),
            }
        })
        .collect::<Vec<_>>();
    let mut seen_group_slots = summary_groups
        .iter()
        .map(|row| row.slot_idx)
        .collect::<HashSet<_>>();
    if overlay_active {
        for (slot_idx, chart_row) in &group_row_by_slot {
            let group_overlay = zone_overlay
                .and_then(|zone_overlay| zone_overlay.groups.get(&slot_idx.to_string()));
            let keep_visible = chart_row.current_share_pct > 0.0
                || group_overlay.is_some_and(|group_overlay| group_overlay.present == Some(true))
                || group_overlay
                    .is_some_and(|group_overlay| group_overlay.raw_rate_percent.is_some());
            if !keep_visible || !seen_group_slots.insert(*slot_idx) {
                continue;
            }
            let (condition_text, condition_tooltip) = zone_loot_group_condition_fields(
                Some(chart_row),
                group_conditions_by_slot
                    .get(slot_idx)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
                data,
            );
            summary_groups.push(ZoneLootSummaryGroupRow {
                slot_idx: *slot_idx,
                label: calculator_group_display_label(data.lang, &chart_row.label),
                fill_color: chart_row.fill_color.to_string(),
                stroke_color: chart_row.stroke_color.to_string(),
                text_color: chart_row.text_color.to_string(),
                drop_rate_text: if chart_row.current_share_pct > 0.0 {
                    percent_value_text(chart_row.current_share_pct)
                } else {
                    String::new()
                },
                drop_rate_source_kind: fish_group_drop_rate_source_kind(chart_row),
                drop_rate_tooltip: fish_group_drop_rate_tooltip(chart_row),
                condition_text,
                condition_tooltip,
                catch_methods: group_methods_by_slot
                    .get(slot_idx)
                    .cloned()
                    .unwrap_or_else(|| default_group_methods(*slot_idx)),
                condition_options: zone_loot_summary_condition_options_for_slot(
                    condition_options_by_slot,
                    *slot_idx,
                ),
            });
        }
    }
    let weighted_species_rows = loot_chart
        .species_rows
        .iter()
        .filter(|row| visible_group_labels.contains(row.group_label))
        .map(|row| zone_loot_summary_species_row_from_loot_species(row, data))
        .collect::<Vec<_>>();
    let mut weighted_harpoon_entries = harpoon_entries
        .iter()
        .copied()
        .filter(|entry| entry.within_group_rate > 0.0)
        .collect::<Vec<_>>();
    weighted_harpoon_entries.sort_by(|left, right| {
        zone_loot_slot_sort_key(left.slot_idx)
            .cmp(&zone_loot_slot_sort_key(right.slot_idx))
            .then_with(|| {
                right
                    .within_group_rate
                    .partial_cmp(&left.within_group_rate)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    let mut weighted_harpoon_species_rows = weighted_harpoon_entries
        .into_iter()
        .map(|entry| {
            let chart_row = group_row_by_slot.get(&entry.slot_idx).copied();
            let (group_label, fill_color, stroke_color, text_color) =
                zone_loot_group_values(data.lang, entry.slot_idx, chart_row);
            if seen_group_slots.insert(entry.slot_idx) {
                let (condition_text, condition_tooltip) = zone_loot_group_condition_fields(
                    chart_row,
                    group_conditions_by_slot
                        .get(&entry.slot_idx)
                        .map(Vec::as_slice)
                        .unwrap_or(&[]),
                    data,
                );
                summary_groups.push(ZoneLootSummaryGroupRow {
                    slot_idx: entry.slot_idx,
                    label: group_label.clone(),
                    fill_color: fill_color.clone(),
                    stroke_color: stroke_color.clone(),
                    text_color: text_color.clone(),
                    drop_rate_text: percent_value_text(100.0),
                    drop_rate_source_kind: "database".to_string(),
                    drop_rate_tooltip: String::new(),
                    condition_text,
                    condition_tooltip,
                    catch_methods: group_methods_by_slot
                        .get(&entry.slot_idx)
                        .cloned()
                        .unwrap_or_else(|| default_group_methods(entry.slot_idx)),
                    condition_options: zone_loot_summary_condition_options_for_slot(
                        condition_options_by_slot,
                        entry.slot_idx,
                    ),
                });
            }
            ZoneLootSummarySpeciesRow {
                slot_idx: entry.slot_idx,
                group_label,
                label: entry.name.clone(),
                icon_url: entry
                    .icon
                    .as_deref()
                    .map(|icon| absolute_public_asset_url(data.cdn_base_url.as_str(), icon)),
                icon_grade_tone: item_grade_tone(entry.grade.as_deref()).to_string(),
                fill_color,
                stroke_color,
                text_color,
                drop_rate_text: loot_species_drop_rate_text(signals, entry),
                drop_rate_source_kind: loot_species_drop_rate_source_kind(entry).to_string(),
                drop_rate_tooltip: loot_species_drop_rate_tooltip(signals, entry, data.lang),
                presence_text: loot_species_presence_text(entry, data.lang),
                presence_source_kind: loot_species_presence_source_kind(entry),
                presence_tooltip: loot_species_presence_tooltip(entry, data.lang)
                    .unwrap_or_default(),
                catch_methods: zone_loot_catch_methods(&entry.catch_methods),
            }
        })
        .collect::<Vec<_>>();
    let mut presence_only_species_rows = rod_entries
        .iter()
        .copied()
        .filter(|entry| entry.within_group_rate <= 0.0)
        .filter_map(|entry| {
            let presence_text = loot_species_presence_text(entry, data.lang)?;
            let chart_row = group_row_by_slot.get(&entry.slot_idx).copied();
            let (group_label, fill_color, stroke_color, text_color) =
                zone_loot_group_values(data.lang, entry.slot_idx, chart_row);
            let (drop_rate_text, drop_rate_source_kind, drop_rate_tooltip) =
                zone_loot_group_drop_rate_fields(chart_row);
            if seen_group_slots.insert(entry.slot_idx) {
                let (condition_text, condition_tooltip) = zone_loot_group_condition_fields(
                    chart_row,
                    group_conditions_by_slot
                        .get(&entry.slot_idx)
                        .map(Vec::as_slice)
                        .unwrap_or(&[]),
                    data,
                );
                summary_groups.push(ZoneLootSummaryGroupRow {
                    slot_idx: entry.slot_idx,
                    label: group_label.clone(),
                    fill_color: fill_color.clone(),
                    stroke_color: stroke_color.clone(),
                    text_color: text_color.clone(),
                    drop_rate_text: drop_rate_text.clone(),
                    drop_rate_source_kind: drop_rate_source_kind.clone(),
                    drop_rate_tooltip: drop_rate_tooltip.clone(),
                    condition_text,
                    condition_tooltip,
                    catch_methods: group_methods_by_slot
                        .get(&entry.slot_idx)
                        .cloned()
                        .unwrap_or_else(|| default_group_methods(entry.slot_idx)),
                    condition_options: zone_loot_summary_condition_options_for_slot(
                        condition_options_by_slot,
                        entry.slot_idx,
                    ),
                });
            }
            Some(ZoneLootSummarySpeciesRow {
                slot_idx: entry.slot_idx,
                group_label,
                label: entry.name.clone(),
                icon_url: entry
                    .icon
                    .as_deref()
                    .map(|icon| absolute_public_asset_url(data.cdn_base_url.as_str(), icon)),
                icon_grade_tone: item_grade_tone(entry.grade.as_deref()).to_string(),
                fill_color,
                stroke_color,
                text_color,
                drop_rate_text: String::new(),
                drop_rate_source_kind: String::new(),
                drop_rate_tooltip: String::new(),
                presence_text: Some(presence_text),
                presence_source_kind: loot_species_presence_source_kind(entry),
                presence_tooltip: loot_species_presence_tooltip(entry, data.lang)
                    .unwrap_or_default(),
                catch_methods: zone_loot_catch_methods(&entry.catch_methods),
            })
        })
        .collect::<Vec<_>>();
    let mut harpoon_presence_only_species_rows = harpoon_entries
        .iter()
        .copied()
        .filter(|entry| entry.within_group_rate <= 0.0)
        .filter_map(|entry| {
            let presence_text = loot_species_presence_text(entry, data.lang)?;
            let chart_row = group_row_by_slot.get(&entry.slot_idx).copied();
            let (group_label, fill_color, stroke_color, text_color) =
                zone_loot_group_values(data.lang, entry.slot_idx, chart_row);
            if seen_group_slots.insert(entry.slot_idx) {
                let (condition_text, condition_tooltip) = zone_loot_group_condition_fields(
                    chart_row,
                    group_conditions_by_slot
                        .get(&entry.slot_idx)
                        .map(Vec::as_slice)
                        .unwrap_or(&[]),
                    data,
                );
                summary_groups.push(ZoneLootSummaryGroupRow {
                    slot_idx: entry.slot_idx,
                    label: group_label.clone(),
                    fill_color: fill_color.clone(),
                    stroke_color: stroke_color.clone(),
                    text_color: text_color.clone(),
                    drop_rate_text: String::new(),
                    drop_rate_source_kind: String::new(),
                    drop_rate_tooltip: String::new(),
                    condition_text,
                    condition_tooltip,
                    catch_methods: group_methods_by_slot
                        .get(&entry.slot_idx)
                        .cloned()
                        .unwrap_or_else(|| default_group_methods(entry.slot_idx)),
                    condition_options: zone_loot_summary_condition_options_for_slot(
                        condition_options_by_slot,
                        entry.slot_idx,
                    ),
                });
            }
            Some(ZoneLootSummarySpeciesRow {
                slot_idx: entry.slot_idx,
                group_label,
                label: entry.name.clone(),
                icon_url: entry
                    .icon
                    .as_deref()
                    .map(|icon| absolute_public_asset_url(data.cdn_base_url.as_str(), icon)),
                icon_grade_tone: item_grade_tone(entry.grade.as_deref()).to_string(),
                fill_color,
                stroke_color,
                text_color,
                drop_rate_text: String::new(),
                drop_rate_source_kind: String::new(),
                drop_rate_tooltip: String::new(),
                presence_text: Some(presence_text),
                presence_source_kind: loot_species_presence_source_kind(entry),
                presence_tooltip: loot_species_presence_tooltip(entry, data.lang)
                    .unwrap_or_default(),
                catch_methods: zone_loot_catch_methods(&entry.catch_methods),
            })
        })
        .collect::<Vec<_>>();
    summary_groups.sort_by(|left, right| {
        zone_loot_slot_sort_key(left.slot_idx)
            .cmp(&zone_loot_slot_sort_key(right.slot_idx))
            .then_with(|| left.label.cmp(&right.label))
    });
    weighted_harpoon_species_rows.sort_by(|left, right| {
        zone_loot_slot_sort_key(left.slot_idx)
            .cmp(&zone_loot_slot_sort_key(right.slot_idx))
            .then_with(|| left.label.to_lowercase().cmp(&right.label.to_lowercase()))
    });
    presence_only_species_rows.sort_by(|left, right| {
        zone_loot_slot_sort_key(left.slot_idx)
            .cmp(&zone_loot_slot_sort_key(right.slot_idx))
            .then_with(|| left.label.to_lowercase().cmp(&right.label.to_lowercase()))
    });
    harpoon_presence_only_species_rows.sort_by(|left, right| {
        zone_loot_slot_sort_key(left.slot_idx)
            .cmp(&zone_loot_slot_sort_key(right.slot_idx))
            .then_with(|| left.label.to_lowercase().cmp(&right.label.to_lowercase()))
    });
    let has_weighted_rows =
        !weighted_species_rows.is_empty() || !weighted_harpoon_species_rows.is_empty();
    let has_presence_only_rows =
        !presence_only_species_rows.is_empty() || !harpoon_presence_only_species_rows.is_empty();
    let available = has_weighted_rows || has_presence_only_rows;
    let mut species_rows = weighted_species_rows;
    species_rows.extend(weighted_harpoon_species_rows);
    species_rows.extend(presence_only_species_rows);
    species_rows.extend(harpoon_presence_only_species_rows);

    ZoneLootSummaryResponse {
        available,
        zone_name: zone.name.clone(),
        data_quality_note: if available {
            text("calculator.server.loot.note.available")
        } else {
            String::new()
        },
        note: if available {
            if overlay_active && has_presence_only_rows && !fish_group_chart.available {
                text("calculator.server.zone_loot_summary.note.overlay_presence_without_groups")
            } else if overlay_active && has_presence_only_rows {
                text("calculator.server.zone_loot_summary.note.overlay_presence")
            } else if overlay_active {
                text("calculator.server.zone_loot_summary.note.overlay")
            } else if has_presence_only_rows && !fish_group_chart.available {
                text("calculator.server.zone_loot_summary.note.presence_without_groups")
            } else if has_presence_only_rows {
                text("calculator.server.zone_loot_summary.note.presence")
            } else {
                text("calculator.server.zone_loot_summary.note.default")
            }
        } else {
            if overlay_active {
                text("calculator.server.zone_loot_summary.note.unavailable.overlay")
            } else {
                text("calculator.server.zone_loot_summary.note.unavailable.default")
            }
        },
        profile_label: if overlay_active {
            text("calculator.server.zone_loot_summary.profile.overlay")
        } else {
            text("calculator.server.zone_loot_summary.profile.default")
        },
        groups: summary_groups,
        species_rows,
    }
}

fn derive_target_fish_summary(
    signals: &CalculatorSignals,
    data: &CalculatorData,
    fish_group_chart: &FishGroupChart,
    total_catches_raw: f64,
    timespan_seconds: f64,
) -> TargetFishSummary {
    let selected_label = signals.target_fish.trim().to_string();
    let target_amount = signals.target_fish_amount.max(1.0).round() as u32;
    if selected_label.is_empty() {
        return TargetFishSummary {
            selected_label: String::new(),
            target_amount,
            target_amount_text: trim_float(f64::from(target_amount)),
            pmf_count_hint_text: calculator_route_text(
                data.lang,
                "calculator.server.helper.target_pmf_auto_short",
            ),
            expected_count_text: "—".to_string(),
            per_day_text: "—".to_string(),
            time_to_target_text: "—".to_string(),
            probability_at_least_text: "—".to_string(),
            session_distribution: Vec::new(),
            status_text: calculator_route_text(
                data.lang,
                "calculator.server.helper.target_select_zone_item",
            ),
        };
    }

    let group_share_by_slot = fish_group_chart
        .rows
        .iter()
        .enumerate()
        .map(|(index, row)| ((index + 1) as u8, row.current_share_pct / 100.0))
        .collect::<HashMap<_, _>>();

    let expected_count_raw = data
        .zone_loot_entries
        .iter()
        .filter(|entry| entry.name == selected_label)
        .map(|entry| {
            total_catches_raw
                * group_share_by_slot
                    .get(&entry.slot_idx)
                    .copied()
                    .unwrap_or_default()
                * entry.within_group_rate
        })
        .sum::<f64>();

    let per_day_raw = if timespan_seconds > 0.0 {
        (expected_count_raw / timespan_seconds) * 86_400.0
    } else {
        0.0
    };
    let time_to_target_text = if per_day_raw > 0.0 {
        human_duration_text((f64::from(target_amount) / per_day_raw) * 86_400.0)
    } else {
        calculator_route_text(data.lang, "calculator.server.value.unavailable")
    };
    let probability_at_least = poisson_probability_at_least(expected_count_raw, target_amount);
    let pmf_is_auto = signals.target_fish_pmf_count <= 0.0;
    let pmf_tail_count = if pmf_is_auto {
        auto_target_fish_pmf_tail_count(expected_count_raw)
    } else {
        signals.target_fish_pmf_count.round() as u32
    }
    .max(1);

    let status_text = if expected_count_raw > 0.0 {
        calculator_route_text_with_vars(
            data.lang,
            "calculator.server.helper.target_per_day_at_spot",
            &[("per_day", &trim_float(per_day_raw))],
        )
    } else {
        calculator_route_text(data.lang, "calculator.server.helper.target_missing_at_spot")
    };

    TargetFishSummary {
        selected_label,
        target_amount,
        target_amount_text: trim_float(f64::from(target_amount)),
        pmf_count_hint_text: if pmf_is_auto {
            calculator_route_text_with_vars(
                data.lang,
                "calculator.server.helper.target_pmf_auto",
                &[("count", &pmf_tail_count.to_string())],
            )
        } else {
            calculator_route_text_with_vars(
                data.lang,
                "calculator.server.helper.target_pmf_fixed",
                &[("count", &pmf_tail_count.to_string())],
            )
        },
        expected_count_text: trim_float(expected_count_raw),
        per_day_text: trim_float(per_day_raw),
        time_to_target_text,
        probability_at_least_text: percent_value_text(probability_at_least * 100.0),
        session_distribution: target_fish_session_distribution(expected_count_raw, pmf_tail_count),
        status_text,
    }
}

#[allow(clippy::too_many_arguments)]
fn derive_stat_breakdowns(
    signals: &CalculatorSignals,
    data: &CalculatorData,
    items_by_key: &HashMap<&str, &CalculatorItemEntry>,
    lifeskill_level: Option<&CalculatorLifeskillLevelEntry>,
    pets: &[&CalculatorPetSignals],
    pet_catalog: &CalculatorPetCatalog,
    fish_group_chart: &FishGroupChart,
    loot_chart: &LootChart,
    target_fish_summary: &TargetFishSummary,
    zone_name: &str,
    zone_bite_min_raw: f64,
    zone_bite_max_raw: f64,
    zone_bite_avg_raw: f64,
    factor_level: f64,
    factor_resources: f64,
    effective_bite_min_raw: f64,
    effective_bite_max_raw: f64,
    bite_time_raw: f64,
    afr_uncapped_raw: f64,
    afr_raw: f64,
    auto_fish_time_raw: f64,
    item_drr_raw: f64,
    lifeskill_level_drr_raw: f64,
    brandstone_durability_factor: f64,
    chance_to_reduce_raw: f64,
    catch_time_active_raw: f64,
    catch_time_afk_raw: f64,
    total_time_raw: f64,
    timespan_seconds: f64,
    timespan_text: &str,
    casts_average_raw: f64,
    durability_loss_average_raw: f64,
    fish_multiplier_raw: f64,
    loot_total_catches_raw: f64,
    loot_fish_per_hour_raw: f64,
) -> CalculatorStatBreakdownSignals {
    let breakdown_text = |key: &str| calculator_route_text(data.lang, key);
    let breakdown_text_with_vars =
        |key: &str, vars: &[(&str, &str)]| calculator_route_text_with_vars(data.lang, key, vars);
    let breakdown_inputs = breakdown_text("calculator.breakdown.section.inputs");
    let breakdown_composition = breakdown_text("calculator.breakdown.section.composition");
    let abundance_label = calc_abundance_label(data.lang, signals.resources);
    let active = calculator_effective_active(&signals.fishing_mode, signals.active);
    let afr_item_rows = collect_item_property_breakdown_rows(
        items_by_key,
        data.cdn_base_url.as_str(),
        &[
            &signals.rod,
            &signals.chair,
            &signals.lightstone_set,
            &signals.float,
        ],
        &[&signals.buff, &signals.food],
        |item| item.afr.map(f64::from),
        "",
    );
    let pet_afr_rows = collect_pet_afr_breakdown_rows(data.lang, pets, pet_catalog);
    let highest_pet_afr_raw = pets
        .iter()
        .map(|pet| pet_afr(pet, pet_catalog))
        .fold(0.0_f64, f64::max);
    let additive_item_afr_raw = sum_item_property(
        items_by_key,
        &[
            &signals.rod,
            &signals.chair,
            &signals.lightstone_set,
            &signals.float,
        ],
        &[&signals.buff, &signals.food],
        |item| item.afr.map(f64::from),
    );
    let mut afr_input_rows = Vec::new();
    afr_input_rows.extend(computed_stat_breakdown_rows_with_formula_part(
        afr_item_rows,
        breakdown_text("calculator.breakdown.label.uncapped_afr"),
        1,
    ));
    afr_input_rows.extend(computed_stat_breakdown_rows_with_formula_part(
        pet_afr_rows,
        breakdown_text("calculator.breakdown.label.uncapped_afr"),
        1,
    ));
    if afr_input_rows.is_empty() {
        afr_input_rows.push(computed_stat_breakdown_row_with_formula_part(
            computed_stat_breakdown_row(
                breakdown_text("calculator.breakdown.label.afr_sources"),
                "0%",
                breakdown_text("calculator.breakdown.detail.no_active_afr_sources"),
            ),
            breakdown_text("calculator.breakdown.label.uncapped_afr"),
            1,
        ));
    }

    let item_drr_item_rows = collect_item_property_breakdown_rows(
        items_by_key,
        data.cdn_base_url.as_str(),
        &[
            &signals.rod,
            &signals.chair,
            &signals.backpack,
            &signals.lightstone_set,
        ],
        &[&signals.buff, &signals.outfit],
        |item| item.item_drr.map(f64::from),
        "",
    );
    let pet_drr_rows = collect_pet_drr_breakdown_rows(data.lang, pets, pet_catalog);
    let mut item_drr_input_rows = Vec::new();
    item_drr_input_rows.extend(computed_stat_breakdown_rows_with_formula_part(
        item_drr_item_rows.clone(),
        breakdown_text("calculator.server.stat.item_drr"),
        1,
    ));
    item_drr_input_rows.extend(computed_stat_breakdown_rows_with_formula_part(
        pet_drr_rows.clone(),
        breakdown_text("calculator.server.stat.item_drr"),
        1,
    ));
    if item_drr_input_rows.is_empty() {
        item_drr_input_rows.push(computed_stat_breakdown_row_with_formula_part(
            computed_stat_breakdown_row(
                breakdown_text("calculator.breakdown.label.item_drr_sources"),
                "0%",
                breakdown_text("calculator.breakdown.detail.no_active_item_drr_sources"),
            ),
            breakdown_text("calculator.server.stat.item_drr"),
            1,
        ));
    }

    let fish_multiplier_rows = collect_fish_multiplier_breakdown_rows(
        data.lang,
        items_by_key,
        data.cdn_base_url.as_str(),
        signals,
        fish_multiplier_raw,
    );
    let fish_multiplier_input_rows = if fish_multiplier_rows.is_empty() {
        vec![computed_stat_breakdown_row(
            breakdown_text("calculator.breakdown.label.fish_multiplier_sources"),
            "×1",
            breakdown_text("calculator.breakdown.detail.no_selected_item_raises_fish_multiplier"),
        )]
    } else {
        fish_multiplier_rows
    };
    let session_seconds_text = trim_float(timespan_seconds);
    let session_hours_text = trim_float(timespan_seconds / 3600.0);
    let applied_fish_multiplier_text = format!("×{}", trim_float(fish_multiplier_raw));

    let mastery_prize_rate =
        mastery_prize_rate_for_bracket(&data.catalog.mastery_prize_curve, signals.mastery);
    let raw_prize_breakdown = computed_stat_breakdown(
        breakdown_text("calculator.server.stat.raw_prize_catch_rate"),
        fish_group_chart.raw_prize_rate_text.clone(),
        breakdown_text("calculator.breakdown.summary.raw_prize_rate"),
        breakdown_text("calculator.breakdown.formula.raw_prize_rate"),
        vec![
            computed_stat_breakdown_section(
                breakdown_inputs.clone(),
                vec![computed_stat_breakdown_row(
                    breakdown_text("calculator.breakdown.label.fishing_mastery"),
                    trim_float(signals.mastery),
                    breakdown_text(
                        "calculator.breakdown.detail.current_mastery_input_for_prize_curve_lookup",
                    ),
                )],
            ),
            computed_stat_breakdown_section(
                breakdown_composition.clone(),
                vec![
                    computed_stat_breakdown_row(
                        breakdown_text("calculator.breakdown.label.resolved_curve_rate"),
                        percent_value_text(mastery_prize_rate * 100.0),
                        breakdown_text(
                            "calculator.breakdown.detail.prize_rate_from_current_mastery_bracket",
                        ),
                    ),
                    computed_stat_breakdown_row(
                        breakdown_text("calculator.breakdown.label.raw_rate"),
                        fish_group_chart.raw_prize_rate_text.clone(),
                        breakdown_text(
                            "calculator.breakdown.detail.shown_before_zone_group_normalization",
                        ),
                    ),
                ],
            ),
        ],
    )
    .with_formula_terms(vec![
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.raw_rate"),
            fish_group_chart.raw_prize_rate_text.clone(),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.resolved_mastery_prize_curve_entry"),
            fish_group_chart.raw_prize_rate_text.clone(),
        ),
    ]);

    let total_time_breakdown = computed_stat_breakdown(
        breakdown_text("calculator.breakdown.title.total_time"),
        fmt2(total_time_raw),
        if active {
            breakdown_text("calculator.breakdown.summary.total_time.active")
        } else {
            breakdown_text("calculator.breakdown.summary.total_time.afk")
        },
        if active {
            breakdown_text("calculator.breakdown.formula.total_time.active")
        } else {
            breakdown_text("calculator.breakdown.formula.total_time.afk")
        },
        vec![
            computed_stat_breakdown_section(breakdown_inputs.clone(), {
                let mut rows = vec![computed_stat_breakdown_row_with_formula_part(
                    computed_stat_breakdown_row(
                        breakdown_text("calculator.breakdown.label.average_bite_time"),
                        fmt2(bite_time_raw),
                        breakdown_text(
                            "calculator.breakdown.detail.effective_average_after_modifiers",
                        ),
                    ),
                    breakdown_text("calculator.breakdown.label.average_bite_time"),
                    1,
                )];
                if active {
                    rows.push(computed_stat_breakdown_row_with_formula_part(
                        computed_stat_breakdown_row(
                            breakdown_text("calculator.breakdown.label.active_catch_time"),
                            fmt2(catch_time_active_raw),
                            breakdown_text("calculator.breakdown.detail.manual_catch_time_active"),
                        ),
                        breakdown_text("calculator.breakdown.label.active_catch_time"),
                        2,
                    ));
                } else {
                    rows.push(computed_stat_breakdown_row_with_formula_part(
                        computed_stat_breakdown_row(
                            breakdown_text("calculator.breakdown.label.auto_fishing_time"),
                            fmt2(auto_fish_time_raw),
                            breakdown_text("calculator.breakdown.detail.passive_waiting_after_afr"),
                        ),
                        breakdown_text("calculator.breakdown.label.auto_fishing_time"),
                        2,
                    ));
                    rows.push(computed_stat_breakdown_row_with_formula_part(
                        computed_stat_breakdown_row(
                            breakdown_text("calculator.breakdown.label.afk_catch_time"),
                            fmt2(catch_time_afk_raw),
                            breakdown_text("calculator.breakdown.detail.manual_catch_time_afk"),
                        ),
                        breakdown_text("calculator.breakdown.label.afk_catch_time"),
                        3,
                    ));
                }
                rows
            }),
            computed_stat_breakdown_section(
                breakdown_composition.clone(),
                vec![computed_stat_breakdown_row(
                    breakdown_text("calculator.breakdown.label.average_total"),
                    fmt2(total_time_raw),
                    breakdown_text("calculator.breakdown.detail.average_cycle_downstream"),
                )],
            ),
        ],
    )
    .with_formula_terms(if active {
        vec![
            computed_stat_formula_term(
                breakdown_text("calculator.breakdown.label.average_total"),
                fmt2(total_time_raw),
            ),
            computed_stat_formula_term(
                breakdown_text("calculator.breakdown.label.average_bite_time"),
                fmt2(bite_time_raw),
            ),
            computed_stat_formula_term(
                breakdown_text("calculator.breakdown.label.active_catch_time"),
                fmt2(catch_time_active_raw),
            ),
        ]
    } else {
        vec![
            computed_stat_formula_term(
                breakdown_text("calculator.breakdown.label.average_total"),
                fmt2(total_time_raw),
            ),
            computed_stat_formula_term(
                breakdown_text("calculator.breakdown.label.average_bite_time"),
                fmt2(bite_time_raw),
            ),
            computed_stat_formula_term(
                breakdown_text("calculator.breakdown.label.auto_fishing_time"),
                fmt2(auto_fish_time_raw),
            ),
            computed_stat_formula_term(
                breakdown_text("calculator.breakdown.label.afk_catch_time"),
                fmt2(catch_time_afk_raw),
            ),
        ]
    });

    let bite_time_factor_rows = vec![
        computed_stat_breakdown_row_with_formula_part(
            computed_stat_breakdown_row(
                breakdown_text("calculator.breakdown.label.zone_average_bite_time"),
                fmt2(zone_bite_avg_raw),
                breakdown_text_with_vars(
                    "calculator.breakdown.detail.derived_from_zone_bite_metadata",
                    &[("zone", zone_name)],
                ),
            ),
            breakdown_text("calculator.breakdown.label.zone_average_bite_time"),
            1,
        ),
        computed_stat_breakdown_row_with_formula_part(
            computed_stat_breakdown_row(
                breakdown_text("calculator.breakdown.label.level_factor"),
                format!("×{}", trim_float(factor_level)),
                breakdown_text_with_vars(
                    "calculator.breakdown.detail.fishing_level_reduces_base_window",
                    &[("level", &signals.level.to_string())],
                ),
            ),
            breakdown_text("calculator.breakdown.label.level_factor"),
            2,
        ),
        computed_stat_breakdown_row_with_formula_part(
            computed_stat_breakdown_row(
                breakdown_text("calculator.breakdown.label.abundance_factor"),
                format!("×{}", trim_float(factor_resources)),
                breakdown_text_with_vars(
                    "calculator.breakdown.detail.resources_scale_bite_window",
                    &[
                        ("resources", &trim_float(signals.resources)),
                        ("abundance", &abundance_label),
                    ],
                ),
            ),
            breakdown_text("calculator.breakdown.label.abundance_factor"),
            3,
        ),
    ];
    let bite_time_breakdown = computed_stat_breakdown(
        breakdown_text("calculator.breakdown.title.bite_time"),
        fmt2(bite_time_raw),
        breakdown_text("calculator.breakdown.summary.bite_time"),
        breakdown_text("calculator.breakdown.formula.bite_time"),
        vec![
            computed_stat_breakdown_section(
                breakdown_inputs.clone(),
                bite_time_factor_rows.clone(),
            ),
            computed_stat_breakdown_section(
                breakdown_composition.clone(),
                vec![computed_stat_breakdown_row(
                    breakdown_text("calculator.breakdown.label.average_bite_time"),
                    fmt2(bite_time_raw),
                    breakdown_text("calculator.breakdown.detail.used_in_total_fishing_time_calc"),
                )],
            ),
        ],
    )
    .with_formula_terms(vec![
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.average_bite_time"),
            fmt2(bite_time_raw),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.zone_average_bite_time"),
            fmt2(zone_bite_avg_raw),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.level_factor"),
            format!("×{}", trim_float(factor_level)),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.abundance_factor"),
            format!("×{}", trim_float(factor_resources)),
        ),
    ]);

    let auto_fish_time_breakdown = computed_stat_breakdown(
        breakdown_text("calculator.breakdown.title.auto_fish_time"),
        fmt2(auto_fish_time_raw),
        breakdown_text("calculator.breakdown.summary.auto_fish_time"),
        breakdown_text("calculator.breakdown.formula.auto_fish_time"),
        vec![
            computed_stat_breakdown_section(
                breakdown_inputs.clone(),
                vec![
                    computed_stat_breakdown_row_with_formula_part(
                        computed_stat_breakdown_row(
                            breakdown_text("calculator.breakdown.label.baseline_auto_fishing_time"),
                            "180",
                            breakdown_text(
                                "calculator.breakdown.detail.backend_passive_afk_baseline",
                            ),
                        ),
                        breakdown_text("calculator.breakdown.label.baseline_auto_fishing_time"),
                        1,
                    ),
                    computed_stat_breakdown_row_with_formula_part(
                        computed_stat_breakdown_row(
                            breakdown_text("calculator.breakdown.label.applied_afr"),
                            format!("{}%", trim_float(afr_raw * 100.0)),
                            breakdown_text("calculator.breakdown.detail.capped_afr_passive_timer"),
                        ),
                        breakdown_text("calculator.breakdown.label.applied_afr"),
                        2,
                    ),
                    computed_stat_breakdown_row_with_formula_part(
                        computed_stat_breakdown_row(
                            breakdown_text("calculator.breakdown.label.minimum_auto_fishing_time"),
                            "60",
                            breakdown_text("calculator.breakdown.detail.passive_timer_minimum"),
                        ),
                        breakdown_text("calculator.breakdown.label.minimum_auto_fishing_time"),
                        3,
                    ),
                ],
            ),
            computed_stat_breakdown_section(
                breakdown_composition.clone(),
                vec![computed_stat_breakdown_row(
                    breakdown_text("calculator.breakdown.label.auto_fishing_time"),
                    fmt2(auto_fish_time_raw),
                    breakdown_text("calculator.breakdown.detail.used_only_in_afk_total_calc"),
                )],
            ),
        ],
    )
    .with_formula_terms(vec![
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.auto_fishing_time"),
            fmt2(auto_fish_time_raw),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.baseline_auto_fishing_time"),
            "180",
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.applied_afr"),
            format!("{}%", trim_float(afr_raw * 100.0)),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.minimum_auto_fishing_time"),
            "60",
        ),
    ]);

    let catch_time_raw = if active {
        catch_time_active_raw
    } else {
        catch_time_afk_raw
    };
    let catch_time_input_label = if active {
        breakdown_text("calculator.breakdown.label.active_catch_time")
    } else {
        breakdown_text("calculator.breakdown.label.afk_catch_time")
    };
    let catch_time_formula_part = catch_time_input_label.clone();
    let catch_time_formula_label = catch_time_input_label.clone();
    let catch_time_detail = if active {
        breakdown_text("calculator.breakdown.detail.manual_catch_time_active")
    } else {
        breakdown_text("calculator.breakdown.detail.manual_catch_after_passive_timer")
    };
    let catch_time_breakdown = computed_stat_breakdown(
        breakdown_text("calculator.breakdown.title.catch_time"),
        fmt2(catch_time_raw),
        if active {
            breakdown_text("calculator.breakdown.summary.catch_time.active")
        } else {
            breakdown_text("calculator.breakdown.summary.catch_time.afk")
        },
        if active {
            breakdown_text("calculator.breakdown.formula.catch_time.active")
        } else {
            breakdown_text("calculator.breakdown.formula.catch_time.afk")
        },
        vec![
            computed_stat_breakdown_section(
                breakdown_inputs.clone(),
                vec![computed_stat_breakdown_row_with_formula_part(
                    computed_stat_breakdown_row(
                        catch_time_input_label,
                        fmt2(catch_time_raw),
                        catch_time_detail,
                    ),
                    catch_time_formula_part,
                    1,
                )],
            ),
            computed_stat_breakdown_section(
                breakdown_composition.clone(),
                vec![computed_stat_breakdown_row(
                    breakdown_text("calculator.breakdown.label.catch_time"),
                    fmt2(catch_time_raw),
                    breakdown_text("calculator.breakdown.detail.used_in_total_fishing_time_calc"),
                )],
            ),
        ],
    )
    .with_formula_terms(vec![
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.catch_time"),
            fmt2(catch_time_raw),
        ),
        computed_stat_formula_term(catch_time_formula_label, fmt2(catch_time_raw)),
    ]);

    let unoptimized_time_raw = zone_bite_avg_raw
        + if active {
            catch_time_active_raw
        } else {
            catch_time_afk_raw + 180.0
        };
    let time_saved_raw = (unoptimized_time_raw - total_time_raw).max(0.0);
    let time_saved_share_text = percent_value_text(
        (100.0 - percentage_of_average_time(total_time_raw, unoptimized_time_raw)).max(0.0),
    );
    let time_saved_breakdown = computed_stat_breakdown(
        breakdown_text("calculator.breakdown.title.time_saved"),
        time_saved_share_text.clone(),
        if time_saved_raw > 0.0 {
            breakdown_text("calculator.breakdown.summary.time_saved.some")
        } else {
            breakdown_text("calculator.breakdown.summary.time_saved.none")
        },
        breakdown_text("calculator.breakdown.formula.time_saved"),
        vec![
            computed_stat_breakdown_section(
                breakdown_inputs.clone(),
                vec![
                    computed_stat_breakdown_row_with_formula_part(
                        computed_stat_breakdown_row(
                            breakdown_text("calculator.breakdown.label.average_unoptimized_time"),
                            fmt2(unoptimized_time_raw),
                            if active {
                                breakdown_text("calculator.breakdown.detail.baseline_active_cycle")
                            } else {
                                breakdown_text("calculator.breakdown.detail.baseline_afk_cycle")
                            },
                        ),
                        breakdown_text("calculator.breakdown.label.average_unoptimized_time"),
                        1,
                    ),
                    computed_stat_breakdown_row_with_formula_part(
                        computed_stat_breakdown_row(
                            breakdown_text("calculator.breakdown.label.average_total_fishing_time"),
                            fmt2(total_time_raw),
                            breakdown_text(
                                "calculator.breakdown.detail.current_optimized_cycle_duration",
                            ),
                        ),
                        breakdown_text("calculator.breakdown.label.average_total_fishing_time"),
                        2,
                    ),
                ],
            ),
            computed_stat_breakdown_section(
                breakdown_composition.clone(),
                vec![
                    computed_stat_breakdown_row(
                        breakdown_text("calculator.breakdown.label.time_saved"),
                        fmt2(time_saved_raw),
                        breakdown_text("calculator.breakdown.detail.absolute_seconds_removed"),
                    ),
                    computed_stat_breakdown_row(
                        breakdown_text("calculator.breakdown.label.saved_share"),
                        time_saved_share_text.clone(),
                        breakdown_text("calculator.breakdown.detail.baseline_cycle_portion"),
                    ),
                ],
            ),
        ],
    )
    .with_formula_terms(vec![
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.time_saved"),
            fmt2(time_saved_raw),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.average_unoptimized_time"),
            fmt2(unoptimized_time_raw),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.average_total_fishing_time"),
            fmt2(total_time_raw),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.saved_share"),
            time_saved_share_text.clone(),
        ),
    ]);

    let auto_fish_time_reduction_breakdown = computed_stat_breakdown(
        breakdown_text("calculator.server.stat.auto_fishing_time_reduction_afr"),
        format!("{}%", trim_float(afr_uncapped_raw * 100.0)),
        breakdown_text("calculator.breakdown.summary.auto_fish_time_reduction"),
        breakdown_text("calculator.breakdown.formula.auto_fish_time_reduction"),
        vec![
            computed_stat_breakdown_section(breakdown_inputs.clone(), afr_input_rows),
            computed_stat_breakdown_section(
                breakdown_composition.clone(),
                vec![
                    computed_stat_breakdown_row(
                        breakdown_text("calculator.breakdown.label.uncapped_afr"),
                        format!("{}%", trim_float(afr_uncapped_raw * 100.0)),
                        breakdown_text(
                            "calculator.breakdown.detail.shown_on_stat_card_before_timing_cap",
                        ),
                    ),
                    computed_stat_breakdown_row(
                        breakdown_text("calculator.breakdown.label.timing_cap"),
                        "66.67%",
                        breakdown_text(
                            "calculator.breakdown.detail.auto_fishing_time_cannot_be_reduced_more_than_two_thirds",
                        ),
                        ),
                    computed_stat_breakdown_row(
                        breakdown_text("calculator.breakdown.label.applied_afr"),
                        format!("{}%", trim_float(afr_raw * 100.0)),
                        breakdown_text(
                            "calculator.breakdown.detail.value_used_to_derive_auto_fishing_time",
                        ),
                    ),
                ],
            ),
        ],
    )
    .with_formula_terms(vec![
        computed_stat_formula_term_with_aliases(
            breakdown_text("calculator.breakdown.label.uncapped_afr"),
            format!("{}%", trim_float(afr_uncapped_raw * 100.0)),
            ["AFR"],
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.highest_pet_afr"),
            format!("{}%", trim_float(highest_pet_afr_raw * 100.0)),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.additive_item_afr"),
            format!("{}%", trim_float(additive_item_afr_raw * 100.0)),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.applied_afr"),
            format!("{}%", trim_float(afr_raw * 100.0)),
        ),
    ]);

    let casts_average_breakdown = computed_stat_breakdown(
        breakdown_text_with_vars(
            "calculator.breakdown.title.casts_average",
            &[("timespan", timespan_text)],
        ),
        fmt2(casts_average_raw),
        breakdown_text("calculator.breakdown.summary.casts_average"),
        breakdown_text("calculator.breakdown.formula.casts_average"),
        vec![
            computed_stat_breakdown_section(
                breakdown_inputs.clone(),
                vec![
                    computed_stat_breakdown_row_with_formula_part(
                        computed_stat_breakdown_row(
                            breakdown_text("calculator.breakdown.label.session_duration"),
                            human_duration_text(timespan_seconds),
                            breakdown_text_with_vars(
                                "calculator.breakdown.detail.session_duration_seconds",
                                &[
                                    ("timespan", timespan_text),
                                    ("seconds", &trim_float(timespan_seconds)),
                                ],
                            ),
                        ),
                        breakdown_text("calculator.breakdown.label.session_duration"),
                        1,
                    ),
                    computed_stat_breakdown_row_with_formula_part(
                        computed_stat_breakdown_row(
                            breakdown_text("calculator.breakdown.label.average_total_fishing_time"),
                            fmt2(total_time_raw),
                            breakdown_text(
                                "calculator.breakdown.detail.denominator_average_cycle_duration",
                            ),
                        ),
                        breakdown_text("calculator.breakdown.label.average_total_fishing_time"),
                        2,
                    ),
                ],
            ),
            computed_stat_breakdown_section(
                breakdown_composition.clone(),
                vec![computed_stat_breakdown_row(
                    breakdown_text("calculator.breakdown.label.average_casts"),
                    fmt2(casts_average_raw),
                    breakdown_text("calculator.breakdown.detail.average_completed_casts_session"),
                )],
            ),
        ],
    )
    .with_formula_terms(vec![
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.average_casts"),
            fmt2(casts_average_raw),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.session_seconds"),
            session_seconds_text.clone(),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.average_total_fishing_time"),
            fmt2(total_time_raw),
        ),
    ]);

    let item_drr_breakdown = computed_stat_breakdown(
        breakdown_text("calculator.server.stat.item_drr"),
        format!("{}%", trim_float(item_drr_raw * 100.0)),
        breakdown_text("calculator.breakdown.summary.item_drr"),
        breakdown_text("calculator.breakdown.formula.item_drr"),
        vec![
            computed_stat_breakdown_section(breakdown_inputs.clone(), item_drr_input_rows),
            computed_stat_breakdown_section(
                breakdown_composition.clone(),
                vec![computed_stat_breakdown_row(
                    breakdown_text("calculator.server.stat.item_drr"),
                    format!("{}%", trim_float(item_drr_raw * 100.0)),
                    breakdown_text(
                        "calculator.breakdown.detail.used_as_durability_resistance_term",
                    ),
                )],
            ),
        ],
    )
    .with_formula_terms(vec![
        computed_stat_formula_term(
            breakdown_text("calculator.server.stat.item_drr"),
            format!("{}%", trim_float(item_drr_raw * 100.0)),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.sum_additive_drr_sources"),
            format!("{}%", trim_float(item_drr_raw * 100.0)),
        ),
    ]);

    let chance_to_consume_durability_breakdown = computed_stat_breakdown(
        breakdown_text("calculator.server.stat.chance_to_consume_durability"),
        format!("{:.2}%", chance_to_reduce_raw * 100.0),
        breakdown_text("calculator.breakdown.summary.chance_to_consume_durability"),
        breakdown_text("calculator.breakdown.formula.chance_to_consume_durability"),
        vec![
            computed_stat_breakdown_section(
                breakdown_inputs.clone(),
                {
                    let mut rows = vec![computed_stat_breakdown_row_with_formula_part(
                        computed_stat_breakdown_row(
                            breakdown_text("calculator.breakdown.label.brandstone_factor"),
                            format!("×{}", trim_float(brandstone_durability_factor)),
                            if signals.brand {
                                breakdown_text(
                                    "calculator.breakdown.detail.brandstone_halves_durability_consumption",
                                )
                            } else {
                                breakdown_text(
                                    "calculator.breakdown.detail.no_brandstone_reduction_active",
                                )
                            },
                        ),
                        breakdown_text("calculator.breakdown.label.brandstone_factor"),
                        1,
                    )];
                    rows.extend(computed_stat_breakdown_rows_with_formula_part(
                        item_drr_item_rows,
                        breakdown_text("calculator.server.stat.item_drr"),
                        2,
                    ));
                    rows.extend(computed_stat_breakdown_rows_with_formula_part(
                        pet_drr_rows,
                        breakdown_text("calculator.server.stat.item_drr"),
                        2,
                    ));
                    rows.push(computed_stat_breakdown_row_with_formula_part(
                        computed_stat_breakdown_row(
                            lifeskill_level
                                .map(|level| level.name.clone())
                                .unwrap_or_else(|| {
                                    breakdown_text("calculator.breakdown.label.lifeskill_drr")
                                }),
                            format!("+{}%", trim_float(lifeskill_level_drr_raw * 100.0)),
                            breakdown_text(
                                "calculator.breakdown.detail.fishing_lifeskill_level_durability_resistance",
                            ),
                        ),
                        breakdown_text("calculator.breakdown.label.lifeskill_drr"),
                        3,
                    ));
                    rows
                },
            ),
            computed_stat_breakdown_section(
                breakdown_composition.clone(),
                vec![
                    computed_stat_breakdown_row(
                        breakdown_text("calculator.breakdown.label.total_item_drr"),
                        format!("{}%", trim_float(item_drr_raw * 100.0)),
                        breakdown_text(
                            "calculator.breakdown.detail.combined_additive_item_and_pet_durability_resistance",
                        ),
                    ),
                    computed_stat_breakdown_row(
                        breakdown_text("calculator.breakdown.label.chance"),
                        format!("{:.2}%", chance_to_reduce_raw * 100.0),
                        breakdown_text(
                            "calculator.breakdown.detail.final_per_cast_consumption_chance",
                        ),
                    ),
                ],
            ),
        ],
    )
    .with_formula_terms(vec![
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.chance"),
            format!("{:.2}%", chance_to_reduce_raw * 100.0),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.brandstone_factor"),
            format!("×{}", trim_float(brandstone_durability_factor)),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.server.stat.item_drr"),
            format!("{}%", trim_float(item_drr_raw * 100.0)),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.lifeskill_drr"),
            format!("+{}%", trim_float(lifeskill_level_drr_raw * 100.0)),
        ),
    ]);

    let durability_loss_average_breakdown = computed_stat_breakdown(
        breakdown_text_with_vars(
            "calculator.breakdown.title.durability_loss_average",
            &[("timespan", timespan_text)],
        ),
        fmt2(durability_loss_average_raw),
        breakdown_text("calculator.breakdown.summary.durability_loss_average"),
        breakdown_text("calculator.breakdown.formula.durability_loss_average"),
        vec![
            computed_stat_breakdown_section(
                breakdown_inputs.clone(),
                vec![
                    computed_stat_breakdown_row_with_formula_part(
                        computed_stat_breakdown_row(
                            breakdown_text("calculator.breakdown.label.average_casts"),
                            fmt2(casts_average_raw),
                            breakdown_text_with_vars(
                                "calculator.breakdown.detail.average_casts_for_timespan",
                                &[("timespan", timespan_text)],
                            ),
                        ),
                        breakdown_text("calculator.breakdown.label.average_casts"),
                        1,
                    ),
                    computed_stat_breakdown_row_with_formula_part(
                        computed_stat_breakdown_row(
                            breakdown_text(
                                "calculator.breakdown.label.chance_to_consume_durability",
                            ),
                            format!("{:.2}%", chance_to_reduce_raw * 100.0),
                            breakdown_text(
                                "calculator.breakdown.detail.final_per_cast_consumption_chance",
                            ),
                        ),
                        breakdown_text("calculator.breakdown.label.chance_to_consume_durability"),
                        2,
                    ),
                ],
            ),
            computed_stat_breakdown_section(
                breakdown_composition.clone(),
                vec![computed_stat_breakdown_row(
                    breakdown_text("calculator.breakdown.label.average_loss"),
                    fmt2(durability_loss_average_raw),
                    breakdown_text(
                        "calculator.breakdown.detail.expected_durability_consumed_session",
                    ),
                )],
            ),
        ],
    )
    .with_formula_terms(vec![
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.average_loss"),
            fmt2(durability_loss_average_raw),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.average_casts"),
            fmt2(casts_average_raw),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.chance_to_consume_durability"),
            format!("{:.2}%", chance_to_reduce_raw * 100.0),
        ),
    ]);

    let zone_bite_min_breakdown = computed_stat_breakdown(
        breakdown_text("calculator.breakdown.title.zone_bite_min"),
        fmt2(zone_bite_min_raw),
        breakdown_text("calculator.breakdown.summary.zone_bite_min"),
        breakdown_text("calculator.breakdown.formula.zone_bite_min"),
        vec![computed_stat_breakdown_section(
            breakdown_inputs.clone(),
            vec![computed_stat_breakdown_row(
                breakdown_text("calculator.breakdown.label.selected_zone"),
                fmt2(zone_bite_min_raw),
                zone_name.to_string(),
            )],
        )],
    )
    .with_formula_terms(vec![
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.zone_bite_min"),
            fmt2(zone_bite_min_raw),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.selected_zone_minimum_bite_time_entry"),
            fmt2(zone_bite_min_raw),
        ),
    ]);

    let zone_bite_avg_breakdown = computed_stat_breakdown(
        breakdown_text("calculator.breakdown.title.zone_bite_avg"),
        fmt2(zone_bite_avg_raw),
        breakdown_text("calculator.breakdown.summary.zone_bite_avg"),
        breakdown_text("calculator.breakdown.formula.zone_bite_avg"),
        vec![
            computed_stat_breakdown_section(
                breakdown_inputs.clone(),
                vec![
                    computed_stat_breakdown_row_with_formula_part(
                        computed_stat_breakdown_row(
                            breakdown_text("calculator.breakdown.label.zone_min"),
                            fmt2(zone_bite_min_raw),
                            zone_name.to_string(),
                        ),
                        breakdown_text("calculator.breakdown.label.zone_min"),
                        1,
                    ),
                    computed_stat_breakdown_row_with_formula_part(
                        computed_stat_breakdown_row(
                            breakdown_text("calculator.breakdown.label.zone_max"),
                            fmt2(zone_bite_max_raw),
                            zone_name.to_string(),
                        ),
                        breakdown_text("calculator.breakdown.label.zone_max"),
                        2,
                    ),
                ],
            ),
            computed_stat_breakdown_section(
                breakdown_composition.clone(),
                vec![computed_stat_breakdown_row(
                    breakdown_text("calculator.breakdown.label.zone_bite_average"),
                    fmt2(zone_bite_avg_raw),
                    breakdown_text("calculator.breakdown.detail.base_zone_average_before_scaling"),
                )],
            ),
        ],
    )
    .with_formula_terms(vec![
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.zone_bite_average"),
            fmt2(zone_bite_avg_raw),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.zone_bite_min"),
            fmt2(zone_bite_min_raw),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.zone_bite_max"),
            fmt2(zone_bite_max_raw),
        ),
    ]);

    let zone_bite_max_breakdown = computed_stat_breakdown(
        breakdown_text("calculator.breakdown.title.zone_bite_max"),
        fmt2(zone_bite_max_raw),
        breakdown_text("calculator.breakdown.summary.zone_bite_max"),
        breakdown_text("calculator.breakdown.formula.zone_bite_max"),
        vec![computed_stat_breakdown_section(
            breakdown_inputs.clone(),
            vec![computed_stat_breakdown_row(
                breakdown_text("calculator.breakdown.label.selected_zone"),
                fmt2(zone_bite_max_raw),
                zone_name.to_string(),
            )],
        )],
    )
    .with_formula_terms(vec![
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.zone_bite_max"),
            fmt2(zone_bite_max_raw),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.selected_zone_maximum_bite_time_entry"),
            fmt2(zone_bite_max_raw),
        ),
    ]);

    let effective_bite_min_breakdown = computed_stat_breakdown(
        breakdown_text("calculator.breakdown.title.effective_bite_min"),
        fmt2(effective_bite_min_raw),
        breakdown_text("calculator.breakdown.summary.effective_bite_min"),
        breakdown_text("calculator.breakdown.formula.effective_bite_min"),
        vec![
            computed_stat_breakdown_section(
                breakdown_inputs.clone(),
                vec![
                    computed_stat_breakdown_row_with_formula_part(
                        computed_stat_breakdown_row(
                            breakdown_text("calculator.breakdown.label.zone_min"),
                            fmt2(zone_bite_min_raw),
                            zone_name.to_string(),
                        ),
                        breakdown_text("calculator.breakdown.label.zone_min"),
                        1,
                    ),
                    computed_stat_breakdown_row_with_formula_part(
                        computed_stat_breakdown_row(
                            breakdown_text("calculator.breakdown.label.level_factor"),
                            format!("×{}", trim_float(factor_level)),
                            breakdown_text_with_vars(
                                "calculator.breakdown.detail.fishing_level_modifier",
                                &[("level", &signals.level.to_string())],
                            ),
                        ),
                        breakdown_text("calculator.breakdown.label.level_factor"),
                        2,
                    ),
                    computed_stat_breakdown_row_with_formula_part(
                        computed_stat_breakdown_row(
                            breakdown_text("calculator.breakdown.label.abundance_factor"),
                            format!("×{}", trim_float(factor_resources)),
                            breakdown_text_with_vars(
                                "calculator.breakdown.detail.resources_abundance",
                                &[
                                    ("resources", &trim_float(signals.resources)),
                                    ("abundance", &abundance_label),
                                ],
                            ),
                        ),
                        breakdown_text("calculator.breakdown.label.abundance_factor"),
                        3,
                    ),
                ],
            ),
            computed_stat_breakdown_section(
                breakdown_composition.clone(),
                vec![computed_stat_breakdown_row(
                    breakdown_text("calculator.breakdown.label.effective_min"),
                    fmt2(effective_bite_min_raw),
                    breakdown_text("calculator.breakdown.detail.lower_end_effective_window"),
                )],
            ),
        ],
    )
    .with_formula_terms(vec![
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.effective_bite_min"),
            fmt2(effective_bite_min_raw),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.zone_bite_min"),
            fmt2(zone_bite_min_raw),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.level_factor"),
            format!("×{}", trim_float(factor_level)),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.abundance_factor"),
            format!("×{}", trim_float(factor_resources)),
        ),
    ]);

    let effective_bite_avg_breakdown = computed_stat_breakdown(
        breakdown_text("calculator.breakdown.title.effective_bite_avg"),
        fmt2(bite_time_raw),
        breakdown_text("calculator.breakdown.summary.effective_bite_avg"),
        breakdown_text("calculator.breakdown.formula.effective_bite_avg"),
        vec![
            computed_stat_breakdown_section(breakdown_inputs.clone(), bite_time_factor_rows),
            computed_stat_breakdown_section(
                breakdown_composition.clone(),
                vec![computed_stat_breakdown_row(
                    breakdown_text("calculator.breakdown.label.effective_average"),
                    fmt2(bite_time_raw),
                    breakdown_text("calculator.breakdown.detail.matches_average_bite_time_stat"),
                )],
            ),
        ],
    )
    .with_formula_terms(vec![
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.effective_bite_average"),
            fmt2(bite_time_raw),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.zone_bite_average"),
            fmt2(zone_bite_avg_raw),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.level_factor"),
            format!("×{}", trim_float(factor_level)),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.abundance_factor"),
            format!("×{}", trim_float(factor_resources)),
        ),
    ]);

    let effective_bite_max_breakdown = computed_stat_breakdown(
        breakdown_text("calculator.breakdown.title.effective_bite_max"),
        fmt2(effective_bite_max_raw),
        breakdown_text("calculator.breakdown.summary.effective_bite_max"),
        breakdown_text("calculator.breakdown.formula.effective_bite_max"),
        vec![
            computed_stat_breakdown_section(
                breakdown_inputs.clone(),
                vec![
                    computed_stat_breakdown_row_with_formula_part(
                        computed_stat_breakdown_row(
                            breakdown_text("calculator.breakdown.label.zone_max"),
                            fmt2(zone_bite_max_raw),
                            zone_name.to_string(),
                        ),
                        breakdown_text("calculator.breakdown.label.zone_max"),
                        1,
                    ),
                    computed_stat_breakdown_row_with_formula_part(
                        computed_stat_breakdown_row(
                            breakdown_text("calculator.breakdown.label.level_factor"),
                            format!("×{}", trim_float(factor_level)),
                            breakdown_text_with_vars(
                                "calculator.breakdown.detail.fishing_level_modifier",
                                &[("level", &signals.level.to_string())],
                            ),
                        ),
                        breakdown_text("calculator.breakdown.label.level_factor"),
                        2,
                    ),
                    computed_stat_breakdown_row_with_formula_part(
                        computed_stat_breakdown_row(
                            breakdown_text("calculator.breakdown.label.abundance_factor"),
                            format!("×{}", trim_float(factor_resources)),
                            breakdown_text_with_vars(
                                "calculator.breakdown.detail.resources_abundance",
                                &[
                                    ("resources", &trim_float(signals.resources)),
                                    ("abundance", &abundance_label),
                                ],
                            ),
                        ),
                        breakdown_text("calculator.breakdown.label.abundance_factor"),
                        3,
                    ),
                ],
            ),
            computed_stat_breakdown_section(
                breakdown_composition.clone(),
                vec![computed_stat_breakdown_row(
                    breakdown_text("calculator.breakdown.label.effective_max"),
                    fmt2(effective_bite_max_raw),
                    breakdown_text("calculator.breakdown.detail.upper_end_effective_window"),
                )],
            ),
        ],
    )
    .with_formula_terms(vec![
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.effective_bite_max"),
            fmt2(effective_bite_max_raw),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.zone_bite_max"),
            fmt2(zone_bite_max_raw),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.level_factor"),
            format!("×{}", trim_float(factor_level)),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.abundance_factor"),
            format!("×{}", trim_float(factor_resources)),
        ),
    ]);

    let loot_total_catches_breakdown = computed_stat_breakdown(
        breakdown_text_with_vars(
            "calculator.breakdown.title.loot_total_catches",
            &[("timespan", timespan_text)],
        ),
        fmt2(loot_total_catches_raw),
        breakdown_text("calculator.breakdown.summary.loot_total_catches"),
        breakdown_text("calculator.breakdown.formula.loot_total_catches"),
        vec![
            computed_stat_breakdown_section(breakdown_inputs.clone(), {
                let mut rows = vec![computed_stat_breakdown_row_with_formula_part(
                    computed_stat_breakdown_row(
                        breakdown_text("calculator.breakdown.label.average_casts"),
                        fmt2(casts_average_raw),
                        breakdown_text_with_vars(
                            "calculator.breakdown.detail.average_casts_during_timespan",
                            &[("timespan", timespan_text)],
                        ),
                    ),
                    breakdown_text("calculator.breakdown.label.average_casts"),
                    1,
                )];
                rows.extend(computed_stat_breakdown_rows_with_formula_part(
                    fish_multiplier_input_rows,
                    breakdown_text("calculator.breakdown.label.applied_fish_multiplier"),
                    2,
                ));
                rows
            }),
            computed_stat_breakdown_section(
                breakdown_composition.clone(),
                vec![computed_stat_breakdown_row(
                    breakdown_text("calculator.breakdown.label.expected_catches"),
                    fmt2(loot_total_catches_raw),
                    breakdown_text("calculator.breakdown.detail.expected_catches_selected_session"),
                )],
            ),
        ],
    )
    .with_formula_terms(vec![
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.expected_catches"),
            fmt2(loot_total_catches_raw),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.average_casts"),
            fmt2(casts_average_raw),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.applied_fish_multiplier"),
            applied_fish_multiplier_text.clone(),
        ),
    ]);

    let loot_fish_per_hour_breakdown = computed_stat_breakdown(
        breakdown_text("calculator.breakdown.title.loot_fish_per_hour"),
        fmt2(loot_fish_per_hour_raw),
        breakdown_text("calculator.breakdown.summary.loot_fish_per_hour"),
        breakdown_text("calculator.breakdown.formula.loot_fish_per_hour"),
        vec![
            computed_stat_breakdown_section(breakdown_inputs.clone(), {
                let mut rows = vec![computed_stat_breakdown_row_with_formula_part(
                    computed_stat_breakdown_row(
                        breakdown_text("calculator.breakdown.label.average_total_fishing_time"),
                        fmt2(total_time_raw),
                        breakdown_text("calculator.breakdown.detail.average_seconds_full_cycle"),
                    ),
                    breakdown_text("calculator.breakdown.label.average_total_fishing_time"),
                    1,
                )];
                rows.extend(computed_stat_breakdown_rows_with_formula_part(
                    collect_fish_multiplier_breakdown_rows(
                        data.lang,
                        items_by_key,
                        data.cdn_base_url.as_str(),
                        signals,
                        fish_multiplier_raw,
                    ),
                    breakdown_text("calculator.breakdown.label.applied_fish_multiplier"),
                    2,
                ));
                rows
            }),
            computed_stat_breakdown_section(
                breakdown_composition.clone(),
                vec![computed_stat_breakdown_row(
                    breakdown_text("calculator.breakdown.label.catches_per_hour"),
                    fmt2(loot_fish_per_hour_raw),
                    breakdown_text("calculator.breakdown.detail.expected_hourly_catch_throughput"),
                )],
            ),
        ],
    )
    .with_formula_terms(vec![
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.catches_per_hour"),
            fmt2(loot_fish_per_hour_raw),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.average_total_fishing_time"),
            fmt2(total_time_raw),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.applied_fish_multiplier"),
            applied_fish_multiplier_text.clone(),
        ),
    ]);

    let loot_group_rows = loot_group_profit_breakdown_rows(&loot_chart.rows, data.lang);
    let loot_group_silver_terms = join_formula_term_values(
        loot_group_rows.iter().map(|row| row.value_text.as_str()),
        " + ",
        "0",
    );
    let loot_total_profit_breakdown = computed_stat_breakdown(
        breakdown_text_with_vars(
            "calculator.breakdown.title.loot_total_profit",
            &[("timespan", timespan_text)],
        ),
        loot_chart.total_profit_text.clone(),
        breakdown_text("calculator.breakdown.summary.loot_total_profit"),
        breakdown_text("calculator.breakdown.formula.loot_total_profit"),
        vec![
            computed_stat_breakdown_section(
                breakdown_inputs.clone(),
                if loot_group_rows.is_empty() {
                    vec![computed_stat_breakdown_row(
                        breakdown_text("calculator.breakdown.label.group_silver"),
                        "0",
                        breakdown_text(
                            "calculator.breakdown.detail.no_source_backed_loot_rows_contributing_expected_silver",
                        ),
                    )]
                } else {
                    loot_group_rows
                },
            ),
            computed_stat_breakdown_section(
                breakdown_composition.clone(),
                vec![
                    computed_stat_breakdown_row(
                        breakdown_text("calculator.breakdown.label.trade_sale_multiplier"),
                        loot_chart.trade_sale_multiplier_text.clone(),
                        breakdown_text(
                            "calculator.breakdown.detail.current_sale_multiplier_after_trade_settings",
                        ),
                    ),
                    computed_stat_breakdown_row(
                        breakdown_text("calculator.breakdown.label.expected_profit"),
                        loot_chart.total_profit_text.clone(),
                        breakdown_text("calculator.breakdown.detail.expected_silver_selected_session"),
                    ),
                ],
            ),
        ],
    )
    .with_formula_terms(vec![
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.expected_profit"),
            loot_chart.total_profit_text.clone(),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.group_expected_silver"),
            loot_group_silver_terms,
        ),
    ]);

    let loot_profit_per_hour_breakdown = computed_stat_breakdown(
        breakdown_text("calculator.breakdown.title.loot_profit_per_hour"),
        loot_chart.profit_per_hour_text.clone(),
        breakdown_text("calculator.breakdown.summary.loot_profit_per_hour"),
        breakdown_text("calculator.breakdown.formula.loot_profit_per_hour"),
        vec![
            computed_stat_breakdown_section(
                breakdown_inputs.clone(),
                vec![
                    computed_stat_breakdown_row_with_formula_part(
                        computed_stat_breakdown_row(
                            breakdown_text_with_vars(
                                "calculator.breakdown.label.expected_profit_for_timespan",
                                &[("timespan", timespan_text)],
                            ),
                            loot_chart.total_profit_text.clone(),
                            breakdown_text(
                                "calculator.breakdown.detail.expected_silver_current_session",
                            ),
                        ),
                        breakdown_text("calculator.breakdown.label.expected_profit"),
                        1,
                    ),
                    computed_stat_breakdown_row_with_formula_part(
                        computed_stat_breakdown_row(
                            breakdown_text("calculator.breakdown.label.session_duration"),
                            human_duration_text(timespan_seconds),
                            breakdown_text_with_vars(
                                "calculator.breakdown.detail.session_duration_seconds",
                                &[
                                    ("timespan", timespan_text),
                                    ("seconds", &trim_float(timespan_seconds)),
                                ],
                            ),
                        ),
                        breakdown_text("calculator.breakdown.label.session_hours"),
                        2,
                    ),
                ],
            ),
            computed_stat_breakdown_section(
                breakdown_composition.clone(),
                vec![computed_stat_breakdown_row(
                    breakdown_text("calculator.breakdown.label.profit_per_hour"),
                    loot_chart.profit_per_hour_text.clone(),
                    breakdown_text("calculator.breakdown.detail.expected_hourly_silver_throughput"),
                )],
            ),
        ],
    )
    .with_formula_terms(vec![
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.profit_per_hour"),
            loot_chart.profit_per_hour_text.clone(),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.expected_profit"),
            loot_chart.total_profit_text.clone(),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.session_hours"),
            session_hours_text.clone(),
        ),
    ]);

    let target_group_share_by_slot = fish_group_chart
        .rows
        .iter()
        .enumerate()
        .map(|(index, row)| ((index + 1) as u8, row))
        .collect::<HashMap<_, _>>();
    let target_matching_entries = data
        .zone_loot_entries
        .iter()
        .filter(|entry| {
            entry.name == target_fish_summary.selected_label && entry.within_group_rate > 0.0
        })
        .collect::<Vec<_>>();
    let target_group_share_terms = join_formula_term_values(
        target_matching_entries.iter().map(|entry| {
            percent_value_text(
                target_group_share_by_slot
                    .get(&entry.slot_idx)
                    .map(|row| row.current_share_pct)
                    .unwrap_or_default(),
            )
        }),
        ", ",
        "0%",
    );
    let target_in_group_rate_terms = join_formula_term_values(
        target_matching_entries
            .iter()
            .map(|entry| percent_value_text(entry.within_group_rate * 100.0)),
        ", ",
        "0%",
    );
    let mut target_input_rows = target_matching_entries
        .iter()
        .map(|entry| {
            let group_row = target_group_share_by_slot.get(&entry.slot_idx).copied();
            let group_label = group_row
                .map(|row| row.label.to_string())
                .unwrap_or_else(|| breakdown_text("calculator.breakdown.label.unassigned"));
            let group_share = group_row
                .map(|row| row.current_share_pct / 100.0)
                .unwrap_or_default();
            let expected_count_raw = loot_total_catches_raw * group_share * entry.within_group_rate;
            computed_stat_breakdown_zone_loot_item_row(
                entry,
                data.cdn_base_url.as_str(),
                trim_float(expected_count_raw),
                breakdown_text_with_vars(
                    "calculator.breakdown.detail.target_row_share_in_group_rate",
                    &[
                        ("group", &group_label),
                        ("share", &percent_value_text(group_share * 100.0)),
                        ("rate", &percent_value_text(entry.within_group_rate * 100.0)),
                    ],
                ),
            )
        })
        .collect::<Vec<_>>();
    target_input_rows.sort_by(|left, right| {
        right
            .value_text
            .partial_cmp(&left.value_text)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let target_expected_count_breakdown = computed_stat_breakdown(
        breakdown_text_with_vars(
            "calculator.server.target.expected",
            &[("timespan", timespan_text)],
        ),
        target_fish_summary.expected_count_text.clone(),
        if target_fish_summary.selected_label.is_empty() {
            breakdown_text("calculator.breakdown.summary.target_expected_count.empty")
        } else {
            breakdown_text("calculator.breakdown.summary.target_expected_count.selected")
        },
        breakdown_text("calculator.breakdown.formula.target_expected_count"),
        vec![
            computed_stat_breakdown_section(
                breakdown_inputs.clone(),
                if target_input_rows.is_empty() {
                    vec![computed_stat_breakdown_row(
                        breakdown_text("calculator.breakdown.label.target_rows"),
                        breakdown_text("calculator.server.value.unavailable"),
                        breakdown_text(
                            "calculator.breakdown.detail.no_matching_source_backed_target_rows",
                        ),
                    )]
                } else {
                    target_input_rows
                },
            ),
            computed_stat_breakdown_section(
                breakdown_composition.clone(),
                vec![
                    computed_stat_breakdown_row(
                        breakdown_text_with_vars(
                            "calculator.breakdown.title.loot_total_catches",
                            &[("timespan", timespan_text)],
                        ),
                        fmt2(loot_total_catches_raw),
                        breakdown_text(
                            "calculator.breakdown.detail.session_catch_volume_used_target_calc",
                        ),
                    ),
                    computed_stat_breakdown_row(
                        breakdown_text("calculator.breakdown.label.expected_count"),
                        target_fish_summary.expected_count_text.clone(),
                        breakdown_text(
                            "calculator.breakdown.detail.combined_expected_count_matching_target_rows",
                        ),
                    ),
                ],
            ),
        ],
    )
    .with_formula_terms(vec![
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.expected_count"),
            target_fish_summary.expected_count_text.clone(),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.expected_catches"),
            fmt2(loot_total_catches_raw),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.group_share"),
            target_group_share_terms,
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.in_group_rate"),
            target_in_group_rate_terms,
        ),
    ]);

    let target_time_to_target_breakdown = computed_stat_breakdown(
        breakdown_text("calculator.server.target.time_to_target"),
        target_fish_summary.time_to_target_text.clone(),
        breakdown_text("calculator.breakdown.summary.target_time_to_target"),
        breakdown_text("calculator.breakdown.formula.target_time_to_target"),
        vec![
            computed_stat_breakdown_section(
                breakdown_inputs.clone(),
                vec![
                    computed_stat_breakdown_row(
                        breakdown_text("calculator.breakdown.label.target_amount"),
                        target_fish_summary.target_amount_text.clone(),
                        breakdown_text("calculator.breakdown.detail.current_target_amount_input"),
                    ),
                    computed_stat_breakdown_row(
                        breakdown_text("calculator.breakdown.label.expected_per_day"),
                        target_fish_summary.per_day_text.clone(),
                        breakdown_text(
                            "calculator.breakdown.detail.expected_daily_count_selected_target",
                        ),
                    ),
                ],
            ),
            computed_stat_breakdown_section(
                breakdown_composition.clone(),
                vec![computed_stat_breakdown_row(
                    breakdown_text("calculator.server.target.time_to_target"),
                    target_fish_summary.time_to_target_text.clone(),
                    breakdown_text("calculator.breakdown.detail.estimated_time_to_reach_target"),
                )],
            ),
        ],
    )
    .with_formula_terms(vec![
        computed_stat_formula_term(
            breakdown_text("calculator.server.target.time_to_target"),
            target_fish_summary.time_to_target_text.clone(),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.target_amount"),
            target_fish_summary.target_amount_text.clone(),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.expected_catches_per_day"),
            target_fish_summary.per_day_text.clone(),
        ),
    ]);

    let target_probability_breakdown = computed_stat_breakdown(
        breakdown_text_with_vars(
            "calculator.server.target.chance_at_least",
            &[("amount", &target_fish_summary.target_amount_text)],
        ),
        target_fish_summary.probability_at_least_text.clone(),
        breakdown_text("calculator.breakdown.summary.target_probability"),
        breakdown_text("calculator.breakdown.formula.target_probability"),
        vec![
            computed_stat_breakdown_section(
                breakdown_inputs.clone(),
                vec![
                    computed_stat_breakdown_row(
                        breakdown_text("calculator.breakdown.label.expected_session_count"),
                        target_fish_summary.expected_count_text.clone(),
                        breakdown_text(
                            "calculator.breakdown.detail.expected_count_current_target_session",
                        ),
                    ),
                    computed_stat_breakdown_row(
                        breakdown_text("calculator.breakdown.label.target_amount"),
                        target_fish_summary.target_amount_text.clone(),
                        breakdown_text(
                            "calculator.breakdown.detail.threshold_used_for_tail_probability",
                        ),
                    ),
                ],
            ),
            computed_stat_breakdown_section(
                breakdown_composition.clone(),
                vec![computed_stat_breakdown_row(
                    breakdown_text("calculator.breakdown.label.probability"),
                    target_fish_summary.probability_at_least_text.clone(),
                    breakdown_text(
                        "calculator.breakdown.detail.probability_meeting_or_exceeding_target",
                    ),
                )],
            ),
        ],
    )
    .with_formula_terms(vec![
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.probability"),
            target_fish_summary.probability_at_least_text.clone(),
        ),
        computed_stat_formula_term(
            breakdown_text("calculator.breakdown.label.target_amount_minus_one"),
            target_fish_summary
                .target_amount
                .saturating_sub(1)
                .to_string(),
        ),
        computed_stat_formula_term_with_aliases(
            breakdown_text("calculator.breakdown.label.expected_session_count"),
            target_fish_summary.expected_count_text.clone(),
            ["λ"],
        ),
    ]);

    CalculatorStatBreakdownSignals {
        total_time: stat_breakdown_json(total_time_breakdown),
        bite_time: stat_breakdown_json(bite_time_breakdown),
        auto_fish_time: stat_breakdown_json(auto_fish_time_breakdown),
        catch_time: stat_breakdown_json(catch_time_breakdown),
        time_saved: stat_breakdown_json(time_saved_breakdown),
        auto_fish_time_reduction: stat_breakdown_json(auto_fish_time_reduction_breakdown),
        casts_average: stat_breakdown_json(casts_average_breakdown),
        item_drr: stat_breakdown_json(item_drr_breakdown),
        chance_to_consume_durability: stat_breakdown_json(chance_to_consume_durability_breakdown),
        durability_loss_average: stat_breakdown_json(durability_loss_average_breakdown),
        zone_bite_min: stat_breakdown_json(zone_bite_min_breakdown),
        zone_bite_avg: stat_breakdown_json(zone_bite_avg_breakdown),
        zone_bite_max: stat_breakdown_json(zone_bite_max_breakdown),
        effective_bite_min: stat_breakdown_json(effective_bite_min_breakdown),
        effective_bite_avg: stat_breakdown_json(effective_bite_avg_breakdown),
        effective_bite_max: stat_breakdown_json(effective_bite_max_breakdown),
        loot_total_catches: stat_breakdown_json(loot_total_catches_breakdown),
        loot_fish_per_hour: stat_breakdown_json(loot_fish_per_hour_breakdown),
        loot_total_profit: stat_breakdown_json(loot_total_profit_breakdown),
        loot_profit_per_hour: stat_breakdown_json(loot_profit_per_hour_breakdown),
        raw_prize_rate: stat_breakdown_json(raw_prize_breakdown),
        target_expected_count: stat_breakdown_json(target_expected_count_breakdown),
        target_time_to_target: stat_breakdown_json(target_time_to_target_breakdown),
        target_probability_at_least: stat_breakdown_json(target_probability_breakdown),
    }
}

fn poisson_probability_at_least(lambda: f64, target_amount: u32) -> f64 {
    if target_amount == 0 {
        return 1.0;
    }
    (1.0 - poisson_probability_below(lambda, target_amount)).clamp(0.0, 1.0)
}

fn poisson_probability_below(lambda: f64, exclusive_upper: u32) -> f64 {
    if exclusive_upper == 0 {
        return 0.0;
    }
    if !lambda.is_finite() || lambda <= 0.0 {
        return 1.0;
    }

    let mut term = (-lambda).exp();
    let mut sum = term;
    for k in 1..exclusive_upper {
        term *= lambda / f64::from(k);
        sum += term;
    }
    sum.clamp(0.0, 1.0)
}

fn target_fish_session_distribution(
    lambda: f64,
    pmf_tail_count: u32,
) -> Vec<TargetFishDistributionBucket> {
    if pmf_tail_count == 0 {
        return Vec::new();
    }

    let mut buckets = Vec::new();
    if pmf_tail_count <= 12 {
        for count in 0..pmf_tail_count {
            let probability_pct = poisson_exact_probability(lambda, count) * 100.0;
            buckets.push(TargetFishDistributionBucket {
                label: count.to_string(),
                probability_pct,
                probability_text: percent_value_text(probability_pct),
            });
        }
    } else {
        let desired_bucket_count =
            (((f64::from(pmf_tail_count)).sqrt() * 2.0).round() as u32).clamp(6, 10);
        let probabilities = (0..pmf_tail_count)
            .map(|count| poisson_exact_probability(lambda, count))
            .collect::<Vec<_>>();
        let bounds = quantile_bucket_bounds(&probabilities, desired_bucket_count);
        let mut start = 0u32;
        for end in bounds {
            let probability_pct = poisson_probability_range(lambda, start, end) * 100.0;
            buckets.push(TargetFishDistributionBucket {
                label: if start == end {
                    start.to_string()
                } else {
                    format!("{start}–{end}")
                },
                probability_pct,
                probability_text: percent_value_text(probability_pct),
            });
            start = end + 1;
        }
        if start < pmf_tail_count {
            let end = pmf_tail_count - 1;
            let probability_pct = poisson_probability_range(lambda, start, end) * 100.0;
            buckets.push(TargetFishDistributionBucket {
                label: if start == end {
                    start.to_string()
                } else {
                    format!("{start}–{end}")
                },
                probability_pct,
                probability_text: percent_value_text(probability_pct),
            });
        }
    }

    let probability_pct = poisson_probability_at_least(lambda, pmf_tail_count) * 100.0;
    buckets.push(TargetFishDistributionBucket {
        label: format!("≥{pmf_tail_count}"),
        probability_pct,
        probability_text: percent_value_text(probability_pct),
    });

    buckets
}

fn quantile_bucket_bounds(probabilities: &[f64], desired_bucket_count: u32) -> Vec<u32> {
    if probabilities.is_empty() || desired_bucket_count <= 1 {
        return Vec::new();
    }

    let total_mass = probabilities.iter().sum::<f64>();
    if total_mass <= 0.0 {
        return Vec::new();
    }

    let bucket_mass_target = total_mass / f64::from(desired_bucket_count);
    let mut next_target = bucket_mass_target;
    let mut cumulative = 0.0;
    let mut boundaries = Vec::new();

    for (index, probability) in probabilities.iter().enumerate() {
        cumulative += *probability;
        while cumulative + f64::EPSILON >= next_target
            && boundaries.len() + 1 < desired_bucket_count as usize
        {
            if index + 1 < probabilities.len() && boundaries.last().copied() != Some(index as u32) {
                boundaries.push(index as u32);
            }
            next_target += bucket_mass_target;
        }
    }

    boundaries
}

fn auto_target_fish_pmf_tail_count(lambda: f64) -> u32 {
    const TARGET_TAIL_PROBABILITY_PCT: f64 = 0.5;
    const MAX_AUTO_COUNT: u32 = 200;

    if !lambda.is_finite() || lambda <= 0.0 {
        return 1;
    }

    for count in 1..=MAX_AUTO_COUNT {
        if poisson_probability_at_least(lambda, count) * 100.0 <= TARGET_TAIL_PROBABILITY_PCT {
            return count;
        }
    }

    MAX_AUTO_COUNT
}

fn poisson_probability_range(lambda: f64, start: u32, end_inclusive: u32) -> f64 {
    if start > end_inclusive {
        return 0.0;
    }
    (poisson_probability_below(lambda, end_inclusive + 1)
        - poisson_probability_below(lambda, start))
    .clamp(0.0, 1.0)
}

fn poisson_exact_probability(lambda: f64, count: u32) -> f64 {
    if !lambda.is_finite() || lambda < 0.0 {
        return 0.0;
    }
    if lambda == 0.0 {
        return if count == 0 { 1.0 } else { 0.0 };
    }

    let mut term = (-lambda).exp();
    for k in 1..=count {
        term *= lambda / f64::from(k);
    }
    term.clamp(0.0, 1.0)
}

fn human_duration_text(total_seconds: f64) -> String {
    if !total_seconds.is_finite() || total_seconds <= 0.0 {
        return "0m".to_string();
    }

    let mut remaining = total_seconds.round() as i64;
    let days = remaining / 86_400;
    remaining %= 86_400;
    let hours = remaining / 3_600;
    remaining %= 3_600;
    let minutes = remaining / 60;

    if days > 0 {
        format!("{days}d {hours}h")
    } else if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{}m", minutes.max(1))
    }
}

fn percentage_of_average_time(time: f64, unoptimized_time: f64) -> f64 {
    if unoptimized_time <= 0.0 {
        0.0
    } else {
        (time / unoptimized_time) * 100.0
    }
}

fn calc_abundance_label(lang: CalculatorLocale, resources: f64) -> String {
    if resources <= 14.0 {
        calculator_route_text(lang, "calculator.resource.exhausted")
    } else if resources <= 45.0 {
        calculator_route_text(lang, "calculator.resource.low")
    } else if resources <= 70.0 {
        calculator_route_text(lang, "calculator.resource.average")
    } else {
        calculator_route_text(lang, "calculator.resource.abundant")
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

fn timespan_text(lang: CalculatorLocale, amount: f64, unit: &str) -> String {
    let normalized = amount.max(0.0);
    let unit_key = match unit {
        "minutes" => "minute",
        "hours" => "hour",
        "days" => "day",
        _ => "week",
    };
    let plurality = if (normalized - 1.0).abs() < f64::EPSILON {
        "one"
    } else {
        "other"
    };
    let label = calculator_route_text(
        lang,
        &format!("calculator.timespan.unit.{unit_key}.{plurality}"),
    );
    format!("{} {label}", trim_float(normalized))
}

fn fmt2(value: f64) -> String {
    format!("{value:.2}")
}

fn trim_float_to(value: f64, decimals: usize) -> String {
    let fixed = format!("{value:.decimals$}");
    fixed
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn trim_float(value: f64) -> String {
    trim_float_to(value, 2)
}

fn format_evidence_percent(rate: f64) -> String {
    let percent = rate * 100.0;
    let max_decimals = if percent.abs() < 1.0 { 4 } else { 2 };
    let compact = trim_float_to(percent, max_decimals);
    if compact == "0" && percent != 0.0 {
        trim_float_to(percent, 6)
    } else {
        compact
    }
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
    let target_fishes = target_fish_options(data);
    let target_fish_summary = derive_target_fish_summary(
        signals,
        data,
        &fish_group_chart,
        loot_chart
            .rows
            .iter()
            .map(|row| row.expected_count_raw)
            .sum(),
        timespan_seconds(signals.timespan_amount, &signals.timespan_unit),
    );
    let outfits = item_options_by_type(&data.catalog.items, "outfit");
    let foods = item_options_by_type(&data.catalog.items, "food");
    let buffs = item_options_by_type(&data.catalog.items, "buff");
    let zone_search_url = format!(
        "/api/v1/calculator/datastar/zone-search?lang={}&locale={}",
        lang_param(&data.api_lang),
        locale_param(data.lang)
    );
    let zone_selected_content = render_searchable_dropdown_text_content(&derived.zone_name);
    let zone_results = render_zone_search_results(
        data.lang,
        "calculator-zone-search-results",
        &data.zones,
        &signals.zone,
        "",
        0,
    );
    let zone_dropdown = render_searchable_dropdown(
        &SearchableDropdownConfig {
            catalog_html: None,
            compact: false,
            trigger_size: SearchableDropdownTriggerSize::Fill,
            trigger_width: None,
            trigger_min_height: None,
            panel_width: Some("32rem"),
            panel_placement: SearchableDropdownPanelPlacement::Adjacent,
            results_layout: SearchableDropdownResultsLayout::List,
            root_id: "calculator-zone-picker",
            input_id: "calculator-zone-value",
            label: &derived.zone_name,
            selected_content_html: &zone_selected_content,
            value: &signals.zone,
            search_url: &zone_search_url,
            search_url_root: Some("api"),
            exclude_selected_inputs: None,
            search_placeholder: &calculator_route_text(data.lang, "calculator.server.search.zones"),
        },
        &zone_results,
    );
    let canonical_signal_computeds =
        render_canonical_checkbox_signal_computeds(data.catalog.pets.slots as usize);
    let mut html = r####"
<div id="calculator-app" class="grid gap-6">
    __CANONICAL_SIGNAL_COMPUTEDS__
    <div class="hidden"
         data-on-signal-patch__debounce.150ms="@post(window.__fishystuffCalculator.evalUrl(patch))"
         data-on-signal-patch-filter="window.__fishystuffCalculator.evalSignalPatchFilter()"></div>

    <section class="card card-border bg-base-100">
        <div class="card-body gap-4">
            <div class="flex flex-col gap-3 xl:flex-row xl:items-start xl:justify-end">
                <div class="flex flex-wrap gap-2">
                    <button class="btn btn-soft btn-secondary"
                            data-on:click="$_calculator_actions.copyUrlToken = (($_calculator_actions && $_calculator_actions.copyUrlToken) || 0) + 1">
                        <svg class="fishy-icon size-6" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="__CALCULATOR_ICON_SPRITE_URL__#fishy-link"></use></svg>
                        __TEXT_COPY_URL__
                    </button>
                    <button class="btn btn-soft btn-secondary"
                            data-on:click="$_calculator_actions.copyShareToken = (($_calculator_actions && $_calculator_actions.copyShareToken) || 0) + 1">
                        <svg class="fishy-icon size-6" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="__CALCULATOR_ICON_SPRITE_URL__#fishy-share-nodes"></use></svg>
                        __TEXT_COPY_SHARE__
                    </button>
                    <button class="btn btn-dash btn-error"
                            data-on:click="$_calculator_actions.resetLayoutToken = (($_calculator_actions && $_calculator_actions.resetLayoutToken) || 0) + 1">
                        <svg class="fishy-icon size-6" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="__CALCULATOR_ICON_SPRITE_URL__#fishy-x-circle"></use></svg>
                        __TEXT_RESET_LAYOUT__
                    </button>
                    <button class="btn btn-dash btn-error"
                            data-on:click="$_calculator_actions.clearToken = (($_calculator_actions && $_calculator_actions.clearToken) || 0) + 1">
                        <svg class="fishy-icon size-6" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="__CALCULATOR_ICON_SPRITE_URL__#fishy-x-circle"></use></svg>
                        __TEXT_CLEAR__
                    </button>
                    <fishy-preset-manager class="fishy-calculator-presets"
                                          data-preset-collection="calculator-presets"></fishy-preset-manager>
                    <fishy-preset-manager class="fishy-calculator-layout-presets"
                                          data-preset-collection="calculator-layouts"></fishy-preset-manager>
                </div>
            </div>
            <div class="pb-1">
                <div role="tablist"
                     class="fishy-calculator-top-tabs tabs tabs-box tabs-sm md:tabs-md w-full max-w-full bg-base-200/80 p-1"
                     aria-label="__TOP_LEVEL_TABS_ARIA__">
                    <button type="button" class="tab fishy-calculator-tab whitespace-nowrap" data-class:tab-active="$_calculator_ui.top_level_tab === 'mode'" data-class:fishy-calculator-tab--pinned="window.__fishystuffCalculator.isPinnedSection($_calculator_ui.pinned_sections, 'mode')" data-attr:aria-selected="($_calculator_ui.top_level_tab === 'mode').toString()" data-on:click="$_calculator_ui.top_level_tab = 'mode'">__TAB_MODE__</button>
                    <button type="button" class="tab fishy-calculator-tab whitespace-nowrap" data-class:tab-active="$_calculator_ui.top_level_tab === 'overview'" data-class:fishy-calculator-tab--pinned="window.__fishystuffCalculator.isPinnedSection($_calculator_ui.pinned_sections, 'overview')" data-attr:aria-selected="($_calculator_ui.top_level_tab === 'overview').toString()" data-on:click="$_calculator_ui.top_level_tab = 'overview'">__TAB_OVERVIEW__</button>
                    <button type="button" class="tab fishy-calculator-tab whitespace-nowrap" data-class:tab-active="$_calculator_ui.top_level_tab === 'zone'" data-class:fishy-calculator-tab--pinned="window.__fishystuffCalculator.isPinnedSection($_calculator_ui.pinned_sections, 'zone')" data-attr:aria-selected="($_calculator_ui.top_level_tab === 'zone').toString()" data-on:click="$_calculator_ui.top_level_tab = 'zone'">__TAB_ZONE__</button>
                    <button type="button" class="tab fishy-calculator-tab whitespace-nowrap" data-class:tab-active="$_calculator_ui.top_level_tab === 'bite_time'" data-class:fishy-calculator-tab--pinned="window.__fishystuffCalculator.isPinnedSection($_calculator_ui.pinned_sections, 'bite_time')" data-attr:aria-selected="($_calculator_ui.top_level_tab === 'bite_time').toString()" data-on:click="$_calculator_ui.top_level_tab = 'bite_time'">__TAB_BITE_TIME__</button>
                    <button type="button" class="tab fishy-calculator-tab whitespace-nowrap" data-class:tab-active="$_calculator_ui.top_level_tab === 'catch_time'" data-class:fishy-calculator-tab--pinned="window.__fishystuffCalculator.isPinnedSection($_calculator_ui.pinned_sections, 'catch_time')" data-attr:aria-selected="($_calculator_ui.top_level_tab === 'catch_time').toString()" data-on:click="$_calculator_ui.top_level_tab = 'catch_time'">__TAB_CATCH_TIME__</button>
                    <button type="button" class="tab fishy-calculator-tab whitespace-nowrap" data-class:tab-active="$_calculator_ui.top_level_tab === 'session'" data-class:fishy-calculator-tab--pinned="window.__fishystuffCalculator.isPinnedSection($_calculator_ui.pinned_sections, 'session')" data-attr:aria-selected="($_calculator_ui.top_level_tab === 'session').toString()" data-on:click="$_calculator_ui.top_level_tab = 'session'">__TAB_SESSION__</button>
                    <button type="button" class="tab fishy-calculator-tab whitespace-nowrap" data-class:tab-active="$_calculator_ui.top_level_tab === 'distribution'" data-class:fishy-calculator-tab--pinned="window.__fishystuffCalculator.isPinnedSection($_calculator_ui.pinned_sections, 'distribution')" data-attr:aria-selected="($_calculator_ui.top_level_tab === 'distribution').toString()" data-on:click="$_calculator_ui.top_level_tab = 'distribution'">__SECTION_DISTRIBUTION__</button>
                    <button type="button" class="tab fishy-calculator-tab whitespace-nowrap" data-class:tab-active="$_calculator_ui.top_level_tab === 'loot'" data-class:fishy-calculator-tab--pinned="window.__fishystuffCalculator.isPinnedSection($_calculator_ui.pinned_sections, 'loot')" data-attr:aria-selected="($_calculator_ui.top_level_tab === 'loot').toString()" data-on:click="$_calculator_ui.top_level_tab = 'loot'">__SECTION_LOOT__</button>
                    <button type="button" class="tab fishy-calculator-tab whitespace-nowrap" data-class:tab-active="$_calculator_ui.top_level_tab === 'trade'" data-class:fishy-calculator-tab--pinned="window.__fishystuffCalculator.isPinnedSection($_calculator_ui.pinned_sections, 'trade')" data-attr:aria-selected="($_calculator_ui.top_level_tab === 'trade').toString()" data-on:click="$_calculator_ui.top_level_tab = 'trade'">__SECTION_TRADE__</button>
                    <button type="button" class="tab fishy-calculator-tab whitespace-nowrap" data-class:tab-active="$_calculator_ui.top_level_tab === 'gear'" data-class:fishy-calculator-tab--pinned="window.__fishystuffCalculator.isPinnedSection($_calculator_ui.pinned_sections, 'gear')" data-attr:aria-selected="($_calculator_ui.top_level_tab === 'gear').toString()" data-on:click="$_calculator_ui.top_level_tab = 'gear'">__SECTION_GEAR__</button>
                    <button type="button" class="tab fishy-calculator-tab whitespace-nowrap" data-class:tab-active="$_calculator_ui.top_level_tab === 'food'" data-class:fishy-calculator-tab--pinned="window.__fishystuffCalculator.isPinnedSection($_calculator_ui.pinned_sections, 'food')" data-attr:aria-selected="($_calculator_ui.top_level_tab === 'food').toString()" data-on:click="$_calculator_ui.top_level_tab = 'food'">__SECTION_FOOD__</button>
                    <button type="button" class="tab fishy-calculator-tab whitespace-nowrap" data-class:tab-active="$_calculator_ui.top_level_tab === 'buffs'" data-class:fishy-calculator-tab--pinned="window.__fishystuffCalculator.isPinnedSection($_calculator_ui.pinned_sections, 'buffs')" data-attr:aria-selected="($_calculator_ui.top_level_tab === 'buffs').toString()" data-on:click="$_calculator_ui.top_level_tab = 'buffs'">__SECTION_BUFFS__</button>
                    <button type="button" class="tab fishy-calculator-tab whitespace-nowrap" data-class:tab-active="$_calculator_ui.top_level_tab === 'pets'" data-class:fishy-calculator-tab--pinned="window.__fishystuffCalculator.isPinnedSection($_calculator_ui.pinned_sections, 'pets')" data-attr:aria-selected="($_calculator_ui.top_level_tab === 'pets').toString()" data-on:click="$_calculator_ui.top_level_tab = 'pets'">__SECTION_PETS__</button>
                    <button type="button" class="tab fishy-calculator-tab whitespace-nowrap" data-class:tab-active="$_calculator_ui.top_level_tab === 'overlay'" data-class:fishy-calculator-tab--pinned="window.__fishystuffCalculator.isPinnedSection($_calculator_ui.pinned_sections, 'overlay')" data-attr:aria-selected="($_calculator_ui.top_level_tab === 'overlay').toString()" data-on:click="$_calculator_ui.top_level_tab = 'overlay'">__TAB_OVERLAY__</button>
                    <button type="button" class="tab fishy-calculator-tab whitespace-nowrap" data-class:tab-active="$_calculator_ui.top_level_tab === 'debug'" data-class:fishy-calculator-tab--pinned="window.__fishystuffCalculator.isPinnedSection($_calculator_ui.pinned_sections, 'debug')" data-attr:aria-selected="($_calculator_ui.top_level_tab === 'debug').toString()" data-on:click="$_calculator_ui.top_level_tab = 'debug'">__TAB_DEBUG__</button>
                </div>
            </div>
        </div>
    </section>

    <fishy-calculator-section-stack class="fishy-calculator-section-stack flex flex-col gap-6">
    __UNPINNED_SLOT_HANDLE__
    <div class="fishy-calculator-pin-dropzone rounded-box border border-dashed border-base-300 bg-base-100/85 px-4 py-3"
         data-calculator-pin-dropzone>
        <div class="fishy-calculator-pin-dropzone__body">
            <span class="fishy-calculator-pin-dropzone__icon fishy-calculator-pin-dropzone__icon--pin" aria-hidden="true"><svg class="fishy-icon size-5" viewBox="0 0 24 24"><use width="100%" height="100%" href="__CALCULATOR_ICON_SPRITE_URL__#fishy-pin"></use></svg></span>
            <span class="fishy-calculator-pin-dropzone__icon fishy-calculator-pin-dropzone__icon--slot" aria-hidden="true"><svg class="fishy-icon size-5" viewBox="0 0 24 24"><use width="100%" height="100%" href="__CALCULATOR_ICON_SPRITE_URL__#fishy-arrow-to-down-fill"></use></svg></span>
            <div class="fishy-calculator-pin-dropzone__copy">
                <div class="fishy-calculator-pin-dropzone__copy-mode fishy-calculator-pin-dropzone__copy-mode--pin">
                    <div class="fishy-calculator-pin-dropzone__title" data-i18n-text="calculator.server.action.pin_dropzone_title">__PIN_DROPZONE_TITLE__</div>
                    <div class="fishy-calculator-pin-dropzone__detail" data-i18n-text="calculator.server.action.pin_dropzone_detail">__PIN_DROPZONE_DETAIL__</div>
                </div>
                <div class="fishy-calculator-pin-dropzone__copy-mode fishy-calculator-pin-dropzone__copy-mode--slot">
                    <div class="fishy-calculator-pin-dropzone__title" data-i18n-text="calculator.server.action.unpinned_dropzone_title">__UNPINNED_DROPZONE_TITLE__</div>
                    <div class="fishy-calculator-pin-dropzone__detail" data-i18n-text="calculator.server.action.unpinned_dropzone_detail">__UNPINNED_DROPZONE_DETAIL__</div>
                </div>
            </div>
        </div>
    </div>
    __MODE_WINDOW__
    <div data-show="window.__fishystuffCalculator.sectionVisible('overview', $_calculator_ui.top_level_tab, $_calculator_ui.pinned_sections)"
         class="grid gap-6 fishy-calculator-section-card"
         data-calculator-section-card
         data-calculator-section-id="overview"
         data-class:fishy-calculator-section-card--pinned="window.__fishystuffCalculator.isPinnedSection($_calculator_ui.pinned_sections, 'overview')">
        <fieldset class="card card-border bg-base-100">
            __OVERVIEW_LEGEND__
            <div class="card-body gap-5 pt-0">
                <div id="calculator-fishing-timeline" class="rounded-box border border-base-300 bg-base-200 p-4">
                    <fishy-timeline-chart
                        id="fishing-timeline"
                        class="timeline-chart"
                        aria-label="__TIMELINE_ARIA__"
                        signal-path="_live.fishing_timeline_chart"></fishy-timeline-chart>
                </div>

                <div class="grid gap-4">
                    <div class="stats stats-vertical rounded-box border border-base-300 bg-base-100 xl:stats-horizontal">
                        <div class="stat fishy-explainable-stat" tabindex="0" data-attr:data-fishy-stat-breakdown="$_live.stat_breakdowns.total_time || ''" data-fishy-stat-color="var(--color-info)">
                            <div class="stat-title">__STAT_TOTAL_TIME__</div>
                            <div class="stat-value text-2xl" data-text="$_live.total_time"></div>
                            <div class="stat-desc">__STAT_SECONDS__</div>
                        </div>
                        <div class="stat fishy-explainable-stat" tabindex="0" data-attr:data-fishy-stat-breakdown="$_live.stat_breakdowns.bite_time || ''" data-fishy-stat-color="var(--color-info)">
                            <div class="stat-title">__STAT_BITE_TIME__</div>
                            <div class="stat-value text-2xl" data-text="$_live.bite_time"></div>
                            <div class="stat-desc">__STAT_SECONDS__</div>
                        </div>
                        <div class="stat fishy-explainable-stat" tabindex="0" data-attr:data-fishy-stat-breakdown="$_live.stat_breakdowns.auto_fish_time || ''" data-fishy-stat-color="var(--color-info)">
                            <div class="stat-title">__STAT_AUTO_FISHING_TIME_AFT__</div>
                            <div class="stat-value text-2xl" data-text="$_live.auto_fish_time"></div>
                            <div class="stat-desc">__STAT_SECONDS__</div>
                        </div>
                        <div class="stat fishy-explainable-stat" tabindex="0" data-attr:data-fishy-stat-breakdown="$_live.stat_breakdowns.auto_fish_time_reduction || ''" data-fishy-stat-color="var(--color-info)">
                            <div class="stat-title">__STAT_AUTO_FISHING_TIME_REDUCTION_AFR__</div>
                            <div class="stat-value text-2xl" data-text="$_live.auto_fish_time_reduction_text"></div>
                        </div>
                    </div>

                    <div class="stats stats-vertical rounded-box border border-base-300 bg-base-100 xl:stats-horizontal">
                        <div class="stat fishy-explainable-stat" tabindex="0" data-attr:data-fishy-stat-breakdown="$_live.stat_breakdowns.casts_average || ''" data-fishy-stat-color="var(--color-info)">
                            <div class="stat-title whitespace-normal leading-snug" data-text="$_live.casts_title"></div>
                            <div class="stat-value text-2xl" data-text="$_live.casts_average"></div>
                        </div>
                        <div class="stat fishy-explainable-stat" tabindex="0" data-attr:data-fishy-stat-breakdown="$_live.stat_breakdowns.item_drr || ''" data-fishy-stat-color="var(--color-warning)">
                            <div class="stat-title">__STAT_ITEM_DRR__</div>
                            <div class="stat-value text-2xl" data-text="$_live.item_drr_text"></div>
                        </div>
                        <div class="stat fishy-explainable-stat" tabindex="0" data-attr:data-fishy-stat-breakdown="$_live.stat_breakdowns.chance_to_consume_durability || ''" data-fishy-stat-color="var(--color-warning)">
                            <div class="stat-title">__STAT_CHANCE_TO_CONSUME_DURABILITY__</div>
                            <div class="stat-value text-2xl" data-text="$_live.chance_to_consume_durability_text"></div>
                        </div>
                        <div class="stat fishy-explainable-stat" tabindex="0" data-attr:data-fishy-stat-breakdown="$_live.stat_breakdowns.durability_loss_average || ''" data-fishy-stat-color="var(--color-warning)">
                            <div class="stat-title whitespace-normal leading-snug" data-text="$_live.durability_loss_title"></div>
                            <div class="stat-value text-2xl" data-text="$_live.durability_loss_average"></div>
                        </div>
                    </div>
                </div>
            </div>
        </fieldset>
    </div>

    <div data-show="window.__fishystuffCalculator.sectionVisible('zone', $_calculator_ui.top_level_tab, $_calculator_ui.pinned_sections)"
         class="grid gap-6 fishy-calculator-section-card"
         data-calculator-section-card
         data-calculator-section-id="zone"
         data-class:fishy-calculator-section-card--pinned="window.__fishystuffCalculator.isPinnedSection($_calculator_ui.pinned_sections, 'zone')">
        <fieldset class="card card-border bg-base-100">
            __ZONE_LEGEND__
            <div class="card-body gap-4 pt-0">
                <div class="grid gap-4">
                    <input id="calculator-zone-value" type="hidden" data-bind="zone" value="__ZONE_VALUE__">
                    __ZONE_SEARCH_DROPDOWN__
                    <div class="stats stats-horizontal rounded-box border border-base-300 bg-base-100 shadow-none">
                        <div class="stat px-4 py-3 fishy-explainable-stat" tabindex="0" data-attr:data-fishy-stat-breakdown="$_live.stat_breakdowns.zone_bite_min || ''" data-fishy-stat-color="var(--color-secondary)">
                            <div class="stat-title">__STAT_MIN__</div>
                            <div class="stat-value text-lg" data-text="$_live.zone_bite_min"></div>
                            <div class="stat-desc">__STAT_SECONDS__</div>
                        </div>
                        <div class="stat px-4 py-3 fishy-explainable-stat" tabindex="0" data-attr:data-fishy-stat-breakdown="$_live.stat_breakdowns.zone_bite_avg || ''" data-fishy-stat-color="var(--color-secondary)">
                            <div class="stat-title">__STAT_AVERAGE__</div>
                            <div class="stat-value text-lg" data-text="$_live.zone_bite_avg"></div>
                            <div class="stat-desc">__STAT_SECONDS__</div>
                        </div>
                        <div class="stat px-4 py-3 fishy-explainable-stat" tabindex="0" data-attr:data-fishy-stat-breakdown="$_live.stat_breakdowns.zone_bite_max || ''" data-fishy-stat-color="var(--color-secondary)">
                            <div class="stat-title">__STAT_MAX__</div>
                            <div class="stat-value text-lg" data-text="$_live.zone_bite_max"></div>
                            <div class="stat-desc">__STAT_SECONDS__</div>
                        </div>
                    </div>
                </div>
            </div>
        </fieldset>
    </div>

    <div data-show="window.__fishystuffCalculator.sectionVisible('bite_time', $_calculator_ui.top_level_tab, $_calculator_ui.pinned_sections)"
         class="grid gap-6 fishy-calculator-section-card"
         data-calculator-section-card
         data-calculator-section-id="bite_time"
         data-class:fishy-calculator-section-card--pinned="window.__fishystuffCalculator.isPinnedSection($_calculator_ui.pinned_sections, 'bite_time')">
        <fieldset class="card card-border bg-base-100">
            __BITE_TIME_LEGEND__
            <div class="card-body gap-4 pt-0">
                <div class="grid gap-4">
                    <fieldset class="fieldset">
                        <legend class="fieldset-legend">__FIELD_FISHING_LEVEL__</legend>
                        __LEVEL_SELECT__
                    </fieldset>
                    <fieldset class="fieldset">
                        <legend class="fieldset-legend">__FIELD_FISHING_RESOURCES__</legend>
                        <input data-bind="_resources" type="range" class="range-xs range-secondary w-full" min="0" max="100">
                        <span class="label text-sm font-medium" data-text="$_resources + '% (' + ($_live.abundance_label || __ABUNDANCE_FALLBACK_JS__) + ')'"></span>
                    </fieldset>
                    <div class="stats stats-horizontal rounded-box border border-base-300 bg-base-100 shadow-none">
                        <div class="stat px-4 py-3 fishy-explainable-stat" tabindex="0" data-attr:data-fishy-stat-breakdown="$_live.stat_breakdowns.effective_bite_min || ''" data-fishy-stat-color="var(--color-secondary)">
                            <div class="stat-title">__STAT_EFFECTIVE_MIN__</div>
                            <div class="stat-value text-lg" data-text="$_live.effective_bite_min"></div>
                            <div class="stat-desc">__STAT_SECONDS__</div>
                        </div>
                        <div class="stat px-4 py-3 fishy-explainable-stat" tabindex="0" data-attr:data-fishy-stat-breakdown="$_live.stat_breakdowns.effective_bite_avg || ''" data-fishy-stat-color="var(--color-secondary)">
                            <div class="stat-title">__STAT_EFFECTIVE_AVERAGE__</div>
                            <div class="stat-value text-lg" data-text="$_live.effective_bite_avg"></div>
                            <div class="stat-desc">__STAT_SECONDS__</div>
                        </div>
                        <div class="stat px-4 py-3 fishy-explainable-stat" tabindex="0" data-attr:data-fishy-stat-breakdown="$_live.stat_breakdowns.effective_bite_max || ''" data-fishy-stat-color="var(--color-secondary)">
                            <div class="stat-title">__STAT_EFFECTIVE_MAX__</div>
                            <div class="stat-value text-lg" data-text="$_live.effective_bite_max"></div>
                            <div class="stat-desc">__STAT_SECONDS__</div>
                        </div>
                    </div>
                </div>
            </div>
        </fieldset>
    </div>

    <div data-show="window.__fishystuffCalculator.sectionVisible('catch_time', $_calculator_ui.top_level_tab, $_calculator_ui.pinned_sections)"
         class="grid gap-6 fishy-calculator-section-card"
         data-calculator-section-card
         data-calculator-section-id="catch_time"
         data-class:fishy-calculator-section-card--pinned="window.__fishystuffCalculator.isPinnedSection($_calculator_ui.pinned_sections, 'catch_time')">
        <fieldset class="card card-border bg-base-100">
            __CATCH_TIME_LEGEND__
            <div class="card-body gap-4 pt-0">
                <div class="grid gap-3 sm:grid-cols-2">
                    <fieldset class="fieldset">
                        <legend class="fieldset-legend">__FIELD_ACTIVE__</legend>
                        <input type="number" min="0" step="any" class="input input-sm w-full" data-bind="catchTimeActive">
                        <span class="label text-xs">__STAT_SECONDS__</span>
                    </fieldset>
                    <fieldset class="fieldset">
                        <legend class="fieldset-legend">__FIELD_AFK__</legend>
                        <input type="number" min="0" step="any" class="input input-sm w-full" data-bind="catchTimeAfk">
                        <span class="label text-xs">__STAT_SECONDS__</span>
                    </fieldset>
                </div>
            </div>
        </fieldset>
    </div>

    <div data-show="window.__fishystuffCalculator.sectionVisible('session', $_calculator_ui.top_level_tab, $_calculator_ui.pinned_sections)"
         class="grid gap-6 fishy-calculator-section-card"
         data-calculator-section-card
         data-calculator-section-id="session"
         data-class:fishy-calculator-section-card--pinned="window.__fishystuffCalculator.isPinnedSection($_calculator_ui.pinned_sections, 'session')">
        <fieldset class="card card-border bg-base-100">
            __SESSION_LEGEND__
            <div class="card-body gap-3 pt-0">
                <div class="grid gap-3">
                    <div class="grid grid-cols-2 gap-3">
                        <fieldset class="fieldset">
                            <legend class="fieldset-legend">__FIELD_AMOUNT__</legend>
                            <input type="number" min="0" step="any" class="input input-sm w-full" id="timespan_amount" data-bind="timespanAmount" name="timespan_amount">
                        </fieldset>
                        <fieldset class="fieldset">
                            <legend class="fieldset-legend">__FIELD_UNIT__</legend>
                            __TIMESPAN_UNIT_SELECT__
                        </fieldset>
                    </div>

                    __SESSION_PRESETS__
                </div>
            </div>
        </fieldset>
    </div>

    <div data-show="window.__fishystuffCalculator.sectionVisible('distribution', $_calculator_ui.top_level_tab, $_calculator_ui.pinned_sections)"
         class="grid gap-6 fishy-calculator-section-card"
         data-calculator-section-card
         data-calculator-section-id="distribution"
         data-class:fishy-calculator-section-card--pinned="window.__fishystuffCalculator.isPinnedSection($_calculator_ui.pinned_sections, 'distribution')">
        __FISH_GROUP_WINDOW__
    </div>

    <div data-show="window.__fishystuffCalculator.sectionVisible('loot', $_calculator_ui.top_level_tab, $_calculator_ui.pinned_sections)"
         class="grid gap-6 fishy-calculator-section-card"
         data-calculator-section-card
         data-calculator-section-id="loot"
         data-class:fishy-calculator-section-card--pinned="window.__fishystuffCalculator.isPinnedSection($_calculator_ui.pinned_sections, 'loot')">
        __LOOT_WINDOW__
    </div>

    <div data-show="window.__fishystuffCalculator.sectionVisible('trade', $_calculator_ui.top_level_tab, $_calculator_ui.pinned_sections)"
         class="grid gap-6 fishy-calculator-section-card"
         data-calculator-section-card
         data-calculator-section-id="trade"
         data-class:fishy-calculator-section-card--pinned="window.__fishystuffCalculator.isPinnedSection($_calculator_ui.pinned_sections, 'trade')">
        __TRADE_WINDOW__
    </div>

    <div data-show="window.__fishystuffCalculator.sectionVisible('gear', $_calculator_ui.top_level_tab, $_calculator_ui.pinned_sections)"
         class="grid gap-6 fishy-calculator-section-card"
         data-calculator-section-card
         data-calculator-section-id="gear"
         data-class:fishy-calculator-section-card--pinned="window.__fishystuffCalculator.isPinnedSection($_calculator_ui.pinned_sections, 'gear')">
        <fieldset class="card card-border bg-base-100 xl:col-span-2">
            __GEAR_LEGEND__
            <div class="card-body pt-0">
                <div id="items" class="grid gap-4 md:grid-cols-2">
                    <fieldset class="fieldset">
                        <legend class="fieldset-legend">__FIELD_LIFESKILL_LEVEL__</legend>
                        __LIFESKILL_LEVEL_SELECT__
                    </fieldset>
                    <fieldset class="fieldset">
                        <legend class="fieldset-legend">__FIELD_FISHING_ROD__</legend>
                        __ROD_SELECT__
                    </fieldset>
                    <fieldset class="fieldset">
                        <legend class="fieldset-legend">__FIELD_BRAND__</legend>
                        <label class="label cursor-pointer justify-start gap-3 rounded-box border border-base-300 bg-base-200 px-3 py-3 font-medium">
                            <input data-bind="brand" type="checkbox" class="checkbox checkbox-primary">
                        </label>
                    </fieldset>
                    <fieldset class="fieldset">
                        <legend class="fieldset-legend">__FIELD_FLOAT__</legend>
                        __FLOAT_SELECT__
                    </fieldset>
                    <fieldset class="fieldset">
                        <legend class="fieldset-legend">__FIELD_CHAIR__</legend>
                        __CHAIR_SELECT__
                    </fieldset>
                    <fieldset class="fieldset">
                        <legend class="fieldset-legend">__FIELD_LIGHTSTONE_SET__</legend>
                        __LIGHTSTONE_SET_SELECT__
                    </fieldset>
                    <fieldset class="fieldset">
                        <legend class="fieldset-legend">__FIELD_BACKPACK__</legend>
                        __BACKPACK_SELECT__
                    </fieldset>
                    <fieldset class="fieldset rounded-box border border-base-300 bg-base-200 p-4 md:col-span-2">
                        <legend class="fieldset-legend">__FIELD_OUTFIT__</legend>
                        __OUTFITS__
                    </fieldset>
                </div>
            </div>
        </fieldset>
    </div>

    <div data-show="window.__fishystuffCalculator.sectionVisible('food', $_calculator_ui.top_level_tab, $_calculator_ui.pinned_sections)"
         class="grid gap-6 fishy-calculator-section-card"
         data-calculator-section-card
         data-calculator-section-id="food"
         data-class:fishy-calculator-section-card--pinned="window.__fishystuffCalculator.isPinnedSection($_calculator_ui.pinned_sections, 'food')">
        <fieldset class="card card-border bg-base-100 xl:col-span-2">
            __FOOD_LEGEND__
            <div class="card-body pt-0">
                __FOODS__
            </div>
        </fieldset>
    </div>

    <div data-show="window.__fishystuffCalculator.sectionVisible('buffs', $_calculator_ui.top_level_tab, $_calculator_ui.pinned_sections)"
         class="grid gap-6 fishy-calculator-section-card"
         data-calculator-section-card
         data-calculator-section-id="buffs"
         data-class:fishy-calculator-section-card--pinned="window.__fishystuffCalculator.isPinnedSection($_calculator_ui.pinned_sections, 'buffs')">
        <fieldset class="card card-border bg-base-100 xl:col-span-2">
            __BUFFS_LEGEND__
            <div class="card-body pt-0">
                __BUFFS__
            </div>
        </fieldset>
    </div>

    <div data-show="window.__fishystuffCalculator.sectionVisible('pets', $_calculator_ui.top_level_tab, $_calculator_ui.pinned_sections)"
         class="grid gap-6 fishy-calculator-section-card"
         data-calculator-section-card
         data-calculator-section-id="pets"
         data-class:fishy-calculator-section-card--pinned="window.__fishystuffCalculator.isPinnedSection($_calculator_ui.pinned_sections, 'pets')">
        <fieldset class="card card-border bg-base-100 xl:col-span-2">
            __PETS_LEGEND__
            <div class="card-body pt-0">
                __PETS__
            </div>
        </fieldset>
    </div>

    <div data-show="window.__fishystuffCalculator.sectionVisible('overlay', $_calculator_ui.top_level_tab, $_calculator_ui.pinned_sections)"
         class="grid gap-6 fishy-calculator-section-card"
         data-calculator-section-card
         data-calculator-section-id="overlay"
         data-class:fishy-calculator-section-card--pinned="window.__fishystuffCalculator.isPinnedSection($_calculator_ui.pinned_sections, 'overlay')">
        <fieldset class="card card-border bg-base-100">
            __OVERLAY_LEGEND__
            <div class="card-body pt-0">
                <fishy-calculator-overlay-panel></fishy-calculator-overlay-panel>
            </div>
        </fieldset>
    </div>

    <div data-show="window.__fishystuffCalculator.sectionVisible('debug', $_calculator_ui.top_level_tab, $_calculator_ui.pinned_sections)"
         class="grid gap-6 fishy-calculator-section-card"
         data-calculator-section-card
         data-calculator-section-id="debug"
         data-class:fishy-calculator-section-card--pinned="window.__fishystuffCalculator.isPinnedSection($_calculator_ui.pinned_sections, 'debug')">
        <fieldset class="card card-border bg-base-100">
            __DEBUG_LEGEND__
            <div class="card-body gap-4 pt-0">
                <code class="rounded-box border border-base-300 bg-base-200 p-4 text-sm">
                    <pre class="overflow-x-auto whitespace-pre-wrap break-all" data-text="$_calc.debug_json"></pre>
                </code>
            </div>
        </fieldset>
    </div>
    </fishy-calculator-section-stack>
</div>
"####
    .to_string();

    let replacements = [
        ("__ZONE_SEARCH_DROPDOWN__", zone_dropdown),
        ("__ZONE_VALUE__", escape_html(&signals.zone)),
        (
            "__UNPINNED_SLOT_HANDLE__",
            render_calculator_unpinned_slot_handle(data.lang),
        ),
        ("__MODE_WINDOW__", render_calculator_mode_window(data.lang)),
        (
            "__TEXT_COPY_URL__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.action.copy_url",
            )),
        ),
        (
            "__PIN_DROPZONE_TITLE__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.action.pin_dropzone_title",
            )),
        ),
        (
            "__PIN_DROPZONE_DETAIL__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.action.pin_dropzone_detail",
            )),
        ),
        (
            "__UNPINNED_DROPZONE_TITLE__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.action.unpinned_dropzone_title",
            )),
        ),
        (
            "__UNPINNED_DROPZONE_DETAIL__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.action.unpinned_dropzone_detail",
            )),
        ),
        (
            "__TEXT_COPY_SHARE__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.action.copy_share",
            )),
        ),
        (
            "__TEXT_RESET_LAYOUT__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.action.reset_layout",
            )),
        ),
        (
            "__TEXT_CLEAR__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.action.clear",
            )),
        ),
        (
            "__TOP_LEVEL_TABS_ARIA__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.chart.aria.top_level_tabs",
            )),
        ),
        (
            "__TAB_MODE__",
            render_calculator_tab_label(
                "mode",
                &calculator_route_text(data.lang, "calculator.server.section.mode"),
            ),
        ),
        (
            "__TAB_OVERVIEW__",
            render_calculator_tab_label(
                "overview",
                &calculator_route_text(data.lang, "calculator.server.tab.overview"),
            ),
        ),
        (
            "__OVERVIEW_LEGEND__",
            render_calculator_panel_legend(
                data.lang,
                "overview",
                &calculator_route_text(data.lang, "calculator.server.tab.overview"),
                None,
            ),
        ),
        (
            "__TAB_ZONE__",
            render_calculator_tab_label(
                "zone",
                &calculator_route_text(data.lang, "calculator.server.section.zone"),
            ),
        ),
        (
            "__ZONE_LEGEND__",
            render_calculator_panel_legend(
                data.lang,
                "zone",
                &calculator_route_text(data.lang, "calculator.server.section.zone"),
                None,
            ),
        ),
        (
            "__TAB_BITE_TIME__",
            render_calculator_tab_label(
                "bite_time",
                &calculator_route_text(data.lang, "calculator.server.section.bite_time"),
            ),
        ),
        (
            "__BITE_TIME_LEGEND__",
            render_calculator_panel_legend(
                data.lang,
                "bite_time",
                &calculator_route_text(data.lang, "calculator.server.section.bite_time"),
                None,
            ),
        ),
        (
            "__TAB_CATCH_TIME__",
            render_calculator_tab_label(
                "catch_time",
                &calculator_route_text(data.lang, "calculator.server.section.catch_time"),
            ),
        ),
        (
            "__CATCH_TIME_LEGEND__",
            render_calculator_panel_legend(
                data.lang,
                "catch_time",
                &calculator_route_text(data.lang, "calculator.server.section.catch_time"),
                None,
            ),
        ),
        (
            "__TAB_SESSION__",
            render_calculator_tab_label(
                "session",
                &calculator_route_text(data.lang, "calculator.server.section.session"),
            ),
        ),
        (
            "__SESSION_LEGEND__",
            render_calculator_panel_legend(
                data.lang,
                "session",
                &calculator_route_text(data.lang, "calculator.server.section.session"),
                None,
            ),
        ),
        (
            "__SECTION_DISTRIBUTION__",
            render_calculator_tab_label(
                "distribution",
                &calculator_route_text(data.lang, "calculator.server.section.distribution"),
            ),
        ),
        (
            "__SECTION_LOOT__",
            render_calculator_tab_label(
                "loot",
                &calculator_route_text(data.lang, "calculator.server.section.loot"),
            ),
        ),
        (
            "__SECTION_TRADE__",
            render_calculator_tab_label(
                "trade",
                &calculator_route_text(data.lang, "calculator.server.section.trade"),
            ),
        ),
        (
            "__TAB_OVERLAY__",
            render_calculator_tab_label(
                "overlay",
                &calculator_route_text(data.lang, "calculator.server.tab.overlay"),
            ),
        ),
        (
            "__TAB_DEBUG__",
            render_calculator_tab_label(
                "debug",
                &calculator_route_text(data.lang, "calculator.server.tab.debug"),
            ),
        ),
        (
            "__CALCULATOR_ICON_SPRITE_URL__",
            CALCULATOR_ICON_SPRITE_URL.to_string(),
        ),
        (
            "__TIMELINE_ARIA__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.chart.aria.timeline",
            )),
        ),
        (
            "__STAT_TOTAL_TIME__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.stat.average_total_fishing_time",
            )),
        ),
        (
            "__STAT_BITE_TIME__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.stat.average_bite_time",
            )),
        ),
        (
            "__STAT_SECONDS__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.stat.seconds",
            )),
        ),
        (
            "__STAT_AUTO_FISHING_TIME_AFT__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.stat.auto_fishing_time_aft",
            )),
        ),
        (
            "__STAT_AUTO_FISHING_TIME_REDUCTION_AFR__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.stat.auto_fishing_time_reduction_afr",
            )),
        ),
        (
            "__STAT_ITEM_DRR__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.stat.item_drr",
            )),
        ),
        (
            "__STAT_CHANCE_TO_CONSUME_DURABILITY__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.stat.chance_to_consume_durability",
            )),
        ),
        (
            "__SECTION_ZONE__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.section.zone",
            )),
        ),
        (
            "__STAT_MIN__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.stat.min",
            )),
        ),
        (
            "__STAT_AVERAGE__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.stat.average",
            )),
        ),
        (
            "__STAT_MAX__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.stat.max",
            )),
        ),
        (
            "__SECTION_BITE_TIME__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.section.bite_time",
            )),
        ),
        (
            "__FIELD_FISHING_LEVEL__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.field.fishing_level",
            )),
        ),
        (
            "__FIELD_FISHING_RESOURCES__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.field.fishing_resources",
            )),
        ),
        (
            "__ABUNDANCE_FALLBACK_JS__",
            escaped_js_string_literal(&calculator_route_text(
                data.lang,
                "calculator.resource.exhausted",
            )),
        ),
        (
            "__STAT_EFFECTIVE_MIN__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.stat.effective_min",
            )),
        ),
        (
            "__STAT_EFFECTIVE_AVERAGE__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.stat.effective_average",
            )),
        ),
        (
            "__STAT_EFFECTIVE_MAX__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.stat.effective_max",
            )),
        ),
        (
            "__SECTION_CATCH_TIME__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.section.catch_time",
            )),
        ),
        (
            "__FIELD_ACTIVE__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.field.active",
            )),
        ),
        (
            "__FIELD_AFK__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.field.afk",
            )),
        ),
        (
            "__SECTION_SESSION__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.section.session",
            )),
        ),
        (
            "__TIMESPAN_FALLBACK_JS__",
            escaped_js_string_literal(&derived.timespan_text),
        ),
        (
            "__FIELD_AMOUNT__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.field.amount",
            )),
        ),
        (
            "__FIELD_UNIT__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.field.unit",
            )),
        ),
        (
            "__SECTION_GEAR__",
            render_calculator_tab_label(
                "gear",
                &calculator_route_text(data.lang, "calculator.server.section.gear"),
            ),
        ),
        (
            "__SECTION_FOOD__",
            render_calculator_tab_label(
                "food",
                &calculator_route_text(data.lang, "calculator.server.field.food"),
            ),
        ),
        (
            "__SECTION_BUFFS__",
            render_calculator_tab_label(
                "buffs",
                &calculator_route_text(data.lang, "calculator.server.field.buffs"),
            ),
        ),
        (
            "__GEAR_LEGEND__",
            render_calculator_panel_legend(
                data.lang,
                "gear",
                &calculator_route_text(data.lang, "calculator.server.section.gear"),
                None,
            ),
        ),
        (
            "__FOOD_LEGEND__",
            render_calculator_panel_legend(
                data.lang,
                "food",
                &calculator_route_text(data.lang, "calculator.server.field.food"),
                None,
            ),
        ),
        (
            "__BUFFS_LEGEND__",
            render_calculator_panel_legend(
                data.lang,
                "buffs",
                &calculator_route_text(data.lang, "calculator.server.field.buffs"),
                None,
            ),
        ),
        (
            "__FIELD_LIFESKILL_LEVEL__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.field.lifeskill_level",
            )),
        ),
        (
            "__FIELD_FISHING_ROD__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.field.fishing_rod",
            )),
        ),
        (
            "__FIELD_BRAND__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.field.brand",
            )),
        ),
        (
            "__FIELD_FLOAT__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.field.float",
            )),
        ),
        (
            "__FIELD_CHAIR__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.field.chair",
            )),
        ),
        (
            "__FIELD_LIGHTSTONE_SET__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.field.lightstone_set",
            )),
        ),
        (
            "__FIELD_BACKPACK__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.field.backpack",
            )),
        ),
        (
            "__FIELD_OUTFIT__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.field.outfit",
            )),
        ),
        (
            "__FIELD_FOOD__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.field.food",
            )),
        ),
        (
            "__FIELD_BUFFS__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.field.buffs",
            )),
        ),
        (
            "__SECTION_PETS__",
            render_calculator_tab_label(
                "pets",
                &calculator_route_text(data.lang, "calculator.server.section.pets"),
            ),
        ),
        (
            "__PETS_LEGEND__",
            render_calculator_panel_legend(
                data.lang,
                "pets",
                &calculator_route_text(data.lang, "calculator.server.section.pets"),
                None,
            ),
        ),
        (
            "__SECTION_OVERLAY_PROPOSAL__",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.section.overlay_proposal",
            )),
        ),
        (
            "__OVERLAY_LEGEND__",
            render_calculator_panel_legend(
                data.lang,
                "overlay",
                &calculator_route_text(data.lang, "calculator.server.section.overlay_proposal"),
                Some("edit-4-fill"),
            ),
        ),
        (
            "__DEBUG_LEGEND__",
            render_calculator_panel_legend(
                data.lang,
                "debug",
                &calculator_route_text(data.lang, "calculator.server.tab.debug"),
                None,
            ),
        ),
        (
            "__LEVEL_SELECT__",
            render_searchable_select_control(
                data.cdn_base_url.as_str(),
                &data.api_lang,
                data.lang,
                "calculator-level-picker",
                "calculator-level-value",
                "level",
                CalculatorSearchableOptionKind::FishingLevel,
                &signals.level.to_string(),
                &fishing_levels,
                false,
                &calculator_route_text(data.lang, "calculator.server.search.fishing_levels"),
                false,
            ),
        ),
        (
            "__TIMESPAN_UNIT_SELECT__",
            render_searchable_select_control(
                data.cdn_base_url.as_str(),
                &data.api_lang,
                data.lang,
                "calculator-session-unit-picker",
                "calculator-session-unit-value",
                "timespanUnit",
                CalculatorSearchableOptionKind::SessionUnit,
                &signals.timespan_unit,
                &session_units,
                false,
                &calculator_route_text(data.lang, "calculator.server.search.session_units"),
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
                &data.api_lang,
                data.lang,
                "calculator-lifeskill-level-picker",
                "calculator-lifeskill-level-value",
                "lifeskill_level",
                CalculatorSearchableOptionKind::LifeskillLevel,
                &signals.lifeskill_level,
                &lifeskill_levels,
                false,
                &calculator_route_text(data.lang, "calculator.server.search.lifeskill_levels"),
                false,
            ),
        ),
        (
            "__FISH_GROUP_WINDOW__",
            render_fish_group_window(
                data,
                signals,
                &fish_group_chart,
                &loot_chart,
                signals.mastery,
                &target_fishes,
                &target_fish_summary,
            ),
        ),
        ("__LOOT_WINDOW__", render_loot_window(data, signals)),
        (
            "__TRADE_WINDOW__",
            render_trade_window(data, signals, &trade_levels),
        ),
        (
            "__ROD_SELECT__",
            render_searchable_select_control(
                data.cdn_base_url.as_str(),
                &data.api_lang,
                data.lang,
                "calculator-rod-picker",
                "calculator-rod-value",
                "rod",
                CalculatorSearchableOptionKind::Rod,
                &signals.rod,
                &rods,
                false,
                &calculator_route_text(data.lang, "calculator.server.search.rods"),
                false,
            ),
        ),
        (
            "__FLOAT_SELECT__",
            render_searchable_select_control(
                data.cdn_base_url.as_str(),
                &data.api_lang,
                data.lang,
                "calculator-float-picker",
                "calculator-float-value",
                "float",
                CalculatorSearchableOptionKind::Float,
                &signals.float,
                &floats,
                true,
                &calculator_route_text(data.lang, "calculator.server.search.floats"),
                false,
            ),
        ),
        (
            "__CHAIR_SELECT__",
            render_searchable_select_control(
                data.cdn_base_url.as_str(),
                &data.api_lang,
                data.lang,
                "calculator-chair-picker",
                "calculator-chair-value",
                "chair",
                CalculatorSearchableOptionKind::Chair,
                &signals.chair,
                &chairs,
                true,
                &calculator_route_text(data.lang, "calculator.server.search.chairs"),
                false,
            ),
        ),
        (
            "__LIGHTSTONE_SET_SELECT__",
            render_searchable_select_control(
                data.cdn_base_url.as_str(),
                &data.api_lang,
                data.lang,
                "calculator-lightstone-set-picker",
                "calculator-lightstone-set-value",
                "lightstone_set",
                CalculatorSearchableOptionKind::LightstoneSet,
                &signals.lightstone_set,
                &lightstone_sets,
                true,
                &calculator_route_text(data.lang, "calculator.server.search.lightstone_sets"),
                false,
            ),
        ),
        (
            "__BACKPACK_SELECT__",
            render_searchable_select_control(
                data.cdn_base_url.as_str(),
                &data.api_lang,
                data.lang,
                "calculator-backpack-picker",
                "calculator-backpack-value",
                "backpack",
                CalculatorSearchableOptionKind::Backpack,
                &signals.backpack,
                &backpacks,
                true,
                &calculator_route_text(data.lang, "calculator.server.search.backpacks"),
                false,
            ),
        ),
        (
            "__OUTFITS__",
            render_checkbox_group(
                data.lang,
                data.cdn_base_url.as_str(),
                "outfits",
                "_outfit_slots",
                &signals.outfit,
                &outfits,
                None,
                None,
                None,
            ),
        ),
        (
            "__FOODS__",
            render_searchable_multiselect_control(
                data.cdn_base_url.as_str(),
                &SearchableMultiselectConfig {
                    lang: data.lang,
                    root_id: "calculator-food-picker",
                    bind_key: "_food_slots",
                    search_placeholder: &calculator_route_text(
                        data.lang,
                        "calculator.server.search.foods",
                    ),
                    helper_text: Some(&calculator_route_text(
                        data.lang,
                        "calculator.server.helper.food_family",
                    )),
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
                    lang: data.lang,
                    root_id: "calculator-buff-picker",
                    bind_key: "_buff_slots",
                    search_placeholder: &calculator_route_text(
                        data.lang,
                        "calculator.server.search.buffs",
                    ),
                    helper_text: Some(&calculator_route_text(
                        data.lang,
                        "calculator.server.helper.buff_group",
                    )),
                },
                &signals.buff,
                &buffs,
            ),
        ),
        (
            "__PETS__",
            render_pet_cards(
                data.cdn_base_url.as_str(),
                &data.api_lang,
                data.lang,
                &data.catalog.pets,
                signals,
            ),
        ),
    ];

    for (token, replacement) in replacements {
        html = html.replace(token, &replacement);
    }
    html = html.replace(
        "__CANONICAL_SIGNAL_COMPUTEDS__",
        &canonical_signal_computeds,
    );
    Ok(html)
}

fn select_options_from_catalog(options: &[CalculatorOptionEntry]) -> Vec<SelectOption<'_>> {
    options
        .iter()
        .map(|option| SelectOption {
            value: option.key.as_str(),
            label: option.label.as_str(),
            icon: None,
            grade_tone: "unknown",
            pet_variant_talent: None,
            pet_variant_special: None,
            pet_skill: None,
            pet_effective_talent_effects: None,
            pet_skill_learn_chance: None,
            item: None,
            lifeskill_level: None,
            presentation: SelectOptionPresentation::Default,
            sort_priority: 1,
        })
        .collect()
}

fn select_options_from_pet_options(options: &[CalculatorPetOptionEntry]) -> Vec<SelectOption<'_>> {
    options
        .iter()
        .map(|option| SelectOption {
            value: option.key.as_str(),
            label: option.label.as_str(),
            icon: None,
            grade_tone: "unknown",
            pet_variant_talent: None,
            pet_variant_special: None,
            pet_skill: None,
            pet_effective_talent_effects: None,
            pet_skill_learn_chance: None,
            item: None,
            lifeskill_level: None,
            presentation: SelectOptionPresentation::Default,
            sort_priority: 1,
        })
        .collect()
}

fn pet_variant_talent_option<'a>(
    tier_entry: &'a CalculatorPetTierEntry,
    catalog: &'a CalculatorPetCatalog,
) -> Option<&'a CalculatorPetOptionEntry> {
    tier_entry
        .talents
        .iter()
        .find_map(|key| catalog.talents.iter().find(|option| option.key == *key))
}

fn pet_variant_special_option<'a>(
    tier_entry: &'a CalculatorPetTierEntry,
    catalog: &'a CalculatorPetCatalog,
) -> Option<&'a CalculatorPetOptionEntry> {
    tier_entry
        .specials
        .iter()
        .find_map(|key| catalog.specials.iter().find(|option| option.key == *key))
}

fn select_options_from_pet_entries_for_tier<'a>(
    catalog: &'a CalculatorPetCatalog,
    tier_key: &str,
    selected_pet_key: Option<&str>,
    pet_context: Option<&CalculatorPetSignals>,
) -> Vec<SelectOption<'a>> {
    let tier_key = tier_key.trim();
    let tier_grade_tone = pet_tier_grade_tone(tier_key);
    let selected_pet =
        selected_pet_key.and_then(|key| catalog.pets.iter().find(|entry| entry.key == key));
    let filtered = if tier_key.is_empty() {
        catalog.pets.iter().collect::<Vec<_>>()
    } else {
        catalog
            .pets
            .iter()
            .filter(|option| option.tiers.iter().any(|tier| tier.key == tier_key))
            .collect::<Vec<_>>()
    };
    filtered
        .into_iter()
        .map(|option| {
            let same_variant_group = selected_pet.is_some_and(|selected| {
                option.key != selected.key && pet_entries_share_variant_group(selected, option)
            });
            let pet_variant_talent = option
                .tiers
                .iter()
                .find(|tier| tier.key == tier_key)
                .and_then(|tier| pet_variant_talent_option(tier, catalog));
            let pet_effective_talent_effects = pet_context.and_then(|pet| {
                pet_variant_talent.map(|talent| {
                    let mut candidate_pet = pet.clone();
                    candidate_pet.pet = option.key.clone();
                    candidate_pet.tier = tier_key.to_string();
                    effective_pet_talent_effects(&candidate_pet, catalog, talent)
                })
            });
            SelectOption {
                value: option.key.as_str(),
                label: option.label.as_str(),
                icon: option.image_url.as_deref(),
                grade_tone: tier_grade_tone,
                pet_variant_talent,
                pet_variant_special: option
                    .tiers
                    .iter()
                    .find(|tier| tier.key == tier_key)
                    .and_then(|tier| pet_variant_special_option(tier, catalog)),
                pet_skill: None,
                pet_effective_talent_effects,
                pet_skill_learn_chance: None,
                item: None,
                lifeskill_level: None,
                presentation: SelectOptionPresentation::PetCard,
                sort_priority: if same_variant_group { 0 } else { 1 },
            }
        })
        .collect()
}

fn pet_tier_grade_tone(tier_key: &str) -> &'static str {
    match tier_key.trim() {
        "5" => "red",
        "4" => "yellow",
        "3" => "blue",
        "2" => "green",
        "1" => "white",
        _ => "unknown",
    }
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
    scored.into_iter().map(|(zone, _)| zone).collect()
}

fn render_searchable_dropdown_more_results_row(
    lang: CalculatorLocale,
    next_offset: usize,
) -> String {
    format!(
        "<li><button type=\"button\" class=\"justify-start gap-3 text-left text-base-content/70\" data-searchable-dropdown-more data-next-offset=\"{}\"><span>{}</span></button></li>",
        next_offset,
        escape_html(&calculator_route_text(
            lang,
            "calculator.server.result.more_available",
        )),
    )
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
        "<span class=\"badge badge-xs whitespace-nowrap border font-medium {class_name}\" title=\"{}\">{}</span>",
        escape_html(label),
        escape_html(label)
    )
}

fn render_wrapping_effect_badge(label: &str, class_name: &str) -> String {
    format!(
        "<span class=\"badge badge-xs h-auto min-h-5 max-w-full whitespace-normal border px-1.5 py-0.5 text-center font-medium leading-tight {class_name}\" title=\"{}\">{}</span>",
        escape_html(label),
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

fn render_pmf_chart(chart_id: &str, aria_label: &str, signal_path: &str) -> String {
    format!(
        "<fishy-pmf-chart id=\"{}\" class=\"distribution-chart\" aria-label=\"{}\" signal-path=\"{}\"></fishy-pmf-chart>",
        escape_html(chart_id),
        escape_html(aria_label),
        escape_html(signal_path),
    )
}

fn groups_distribution_segments(
    rows: &[FishGroupChartRow],
    total_catches_raw: f64,
    show_normalized_rates: bool,
    lang: CalculatorLocale,
) -> Vec<DistributionChartSegment> {
    let total_weight_pct = rows.iter().map(|row| row.weight_pct.max(0.0)).sum::<f64>();

    rows.iter()
        .map(|row| DistributionChartSegment {
            label: calculator_group_display_label(lang, &row.label),
            value_text: percent_value_text(if show_normalized_rates {
                row.current_share_pct
            } else {
                row.weight_pct
            }),
            // Expected catches are based on the normalized group share.
            // The toggle only changes how the rate itself is shown.
            detail_text: trim_float(total_catches_raw * (row.current_share_pct / 100.0)),
            width_pct: if show_normalized_rates {
                row.current_share_pct
            } else {
                row.weight_pct
            },
            fill_color: row.fill_color,
            stroke_color: row.stroke_color,
            text_color: row.text_color,
            connector_color: row.connector_color,
            breakdown: Some(fish_group_distribution_breakdown(
                row,
                total_catches_raw,
                total_weight_pct,
                show_normalized_rates,
                lang,
            )),
        })
        .collect()
}

fn group_silver_distribution_segments(
    loot_rows: &[LootChartRow],
    species_rows: &[LootSpeciesRow],
    lang: CalculatorLocale,
) -> Vec<DistributionChartSegment> {
    let total_profit_raw = loot_rows
        .iter()
        .map(|row| row.expected_profit_raw)
        .sum::<f64>();

    loot_rows
        .iter()
        .map(|row| DistributionChartSegment {
            label: calculator_group_display_label(lang, row.label),
            value_text: row.silver_share_text.clone(),
            detail_text: compact_silver_text(row.expected_profit_raw),
            width_pct: if total_profit_raw > 0.0 {
                (row.expected_profit_raw / total_profit_raw) * 100.0
            } else {
                0.0
            },
            fill_color: row.fill_color,
            stroke_color: row.stroke_color,
            text_color: row.text_color,
            connector_color: row.connector_color,
            breakdown: Some(group_silver_distribution_breakdown(
                row,
                species_rows,
                total_profit_raw,
                lang,
            )),
        })
        .collect()
}

fn timeline_chart_segment(
    label: &str,
    value_seconds: f64,
    width_pct: f64,
    fill_color: &'static str,
    stroke_color: &'static str,
    breakdown: Option<Value>,
) -> TimelineChartSegment {
    TimelineChartSegment {
        label: label.to_string(),
        value_text: format!("{}s", fmt2(value_seconds)),
        detail_text: percent_value_text(width_pct),
        width_pct: width_pct.max(0.0),
        fill_color,
        stroke_color,
        breakdown,
    }
}

fn fishing_timeline_chart(
    lang: CalculatorLocale,
    active: bool,
    bite_time_raw: f64,
    auto_fish_time_raw: f64,
    catch_time_active_raw: f64,
    catch_time_afk_raw: f64,
    total_time_raw: f64,
    zone_bite_avg_raw: f64,
    bite_time_breakdown: Option<Value>,
    auto_fish_time_breakdown: Option<Value>,
    catch_time_breakdown: Option<Value>,
    time_saved_breakdown: Option<Value>,
) -> TimelineChartSignal {
    let catch_time_raw = if active {
        catch_time_active_raw
    } else {
        catch_time_afk_raw
    };
    let unoptimized_time_raw = zone_bite_avg_raw
        + if active {
            catch_time_active_raw
        } else {
            catch_time_afk_raw + 180.0
        };
    let percent_bite = percentage_of_average_time(bite_time_raw, unoptimized_time_raw);
    let percent_af = if active {
        0.0
    } else {
        percentage_of_average_time(auto_fish_time_raw, unoptimized_time_raw)
    };
    let percent_catch = percentage_of_average_time(catch_time_raw, unoptimized_time_raw);
    let percent_saved =
        (100.0 - percentage_of_average_time(total_time_raw, unoptimized_time_raw)).max(0.0);
    let time_saved_raw = (unoptimized_time_raw - total_time_raw).max(0.0);

    let mut segments = vec![timeline_chart_segment(
        &calculator_route_text(lang, "calculator.timeline.bite_time"),
        bite_time_raw,
        percent_bite,
        "#46d2a7",
        "color-mix(in srgb, #46d2a7 72%, var(--color-base-content) 22%)",
        bite_time_breakdown,
    )];
    if !active {
        segments.push(timeline_chart_segment(
            &calculator_route_text(lang, "calculator.timeline.auto_fishing_time"),
            auto_fish_time_raw,
            percent_af,
            "#4e7296",
            "color-mix(in srgb, #4e7296 76%, var(--color-base-content) 24%)",
            auto_fish_time_breakdown,
        ));
    }
    segments.push(timeline_chart_segment(
        &calculator_route_text(lang, "calculator.timeline.catch_time"),
        catch_time_raw,
        percent_catch,
        "#d27746",
        "color-mix(in srgb, #d27746 74%, var(--color-base-content) 24%)",
        catch_time_breakdown,
    ));
    segments.push(timeline_chart_segment(
        &calculator_route_text(lang, "calculator.timeline.time_saved"),
        time_saved_raw,
        percent_saved,
        "color-mix(in oklab, var(--color-base-100) 55%, var(--color-base-content) 10%)",
        "color-mix(in oklab, var(--color-base-content) 16%, transparent)",
        time_saved_breakdown,
    ));

    TimelineChartSignal { segments }
}

fn target_fish_pmf_chart(summary: &TargetFishSummary) -> PmfChartSignal {
    PmfChartSignal {
        bars: summary
            .session_distribution
            .iter()
            .map(|bucket| PmfChartBar {
                label: bucket.label.clone(),
                value_text: bucket.probability_text.clone(),
                probability_pct: bucket.probability_pct,
                highlight: pmf_bucket_contains_target(&bucket.label, summary.target_amount),
            })
            .collect(),
        expected_value_text: summary.expected_count_text.clone(),
    }
}

fn pmf_bucket_contains_target(label: &str, target_amount: u32) -> bool {
    if let Some(tail_start) = label.strip_prefix('≥') {
        return tail_start
            .trim()
            .parse::<u32>()
            .map(|start| target_amount >= start)
            .unwrap_or(false);
    }

    if let Some((start, end)) = label.split_once('–') {
        return match (start.trim().parse::<u32>(), end.trim().parse::<u32>()) {
            (Ok(start), Ok(end)) => (start..=end).contains(&target_amount),
            _ => false,
        };
    }

    label
        .trim()
        .parse::<u32>()
        .map(|value| value == target_amount)
        .unwrap_or(false)
}

fn filtered_loot_flow_rows(
    loot_rows: &[LootChartRow],
    species_rows: &[LootSpeciesRow],
) -> Vec<LootChartRow> {
    let groups_with_species = species_rows
        .iter()
        .map(|row| row.group_label)
        .collect::<HashSet<_>>();

    loot_rows
        .iter()
        .filter(|row| row.current_share_pct > 0.0 && groups_with_species.contains(row.label))
        .cloned()
        .collect()
}

fn render_loot_sankey(lang: CalculatorLocale, chart: &LootChart) -> String {
    if chart.species_rows.is_empty() {
        return format!(
            "<div class=\"rounded-box border border-dashed border-base-300 bg-base-200 p-4 text-sm text-base-content/70\">{}</div>",
            escape_html(&calculator_route_text(
                lang,
                "calculator.server.chart.no_loot_rows",
            ))
        );
    }
    format!(
        "<div class=\"rounded-box border border-base-300 bg-base-200 p-4\"><div class=\"mb-3\"><div class=\"text-sm font-medium\">{}</div><div class=\"text-xs text-base-content/70\">{}</div></div><div class=\"overflow-x-auto loot-sankey-scroll\"><fishy-loot-sankey class=\"loot-sankey\" aria-label=\"{}\" signal-path=\"_calc.loot_sankey_chart\"></fishy-loot-sankey></div></div>",
        escape_html(&calculator_route_text(
            lang,
            "calculator.server.chart.loot_flow_title",
        )),
        escape_html(&calculator_route_text(
            lang,
            "calculator.server.chart.loot_flow_description",
        )),
        escape_html(&calculator_route_text(
            lang,
            "calculator.server.chart.aria.loot_flow",
        )),
    )
}

fn render_fish_group_chart(
    lang: CalculatorLocale,
    chart: &FishGroupChart,
    show_normalized_rates: bool,
) -> String {
    if !chart.available {
        return format!(
            "<div id=\"calculator-fish-group-chart\" class=\"rounded-box border border-dashed border-base-300 bg-base-200 p-4 text-sm text-base-content/70\">{}</div>",
            escape_html(&chart.note)
        );
    }

    format!(
        "<div id=\"calculator-fish-group-chart\"><div class=\"rounded-box border border-base-300 bg-base-200 p-4\"><div class=\"mb-3\"><div class=\"text-sm font-medium\">{}</div><div class=\"text-xs text-base-content/70\">{}</div></div>{}</div></div>",
        escape_html(&calculator_route_text(
            lang,
            "calculator.server.chart.group_distribution_title",
        )),
        escape_html(&calculator_route_text(
            lang,
            if show_normalized_rates {
                "calculator.server.chart.group_distribution_description.normalized"
            } else {
                "calculator.server.chart.group_distribution_description.raw"
            },
        )),
        render_distribution_chart(
            "fish-group-distribution-chart",
            &calculator_route_text(lang, "calculator.server.chart.aria.group_distribution"),
            "_calc.fish_group_distribution_chart",
        ),
    )
}

fn render_fish_group_silver_chart(lang: CalculatorLocale, chart: &LootChart) -> String {
    if !chart.available {
        return format!(
            "<div id=\"calculator-fish-group-silver-chart\" class=\"rounded-box border border-dashed border-base-300 bg-base-200 p-4 text-sm text-base-content/70\">{}</div>",
            escape_html(&chart.note)
        );
    }

    format!(
        "<div id=\"calculator-fish-group-silver-chart\"><div class=\"rounded-box border border-base-300 bg-base-200 p-4\"><div class=\"mb-3\"><div class=\"text-sm font-medium\">{}</div><div class=\"text-xs text-base-content/70\">{}</div></div>{}</div></div>",
        escape_html(&calculator_route_text(
            lang,
            "calculator.server.chart.group_silver_distribution_title",
        )),
        escape_html(&calculator_route_text(
            lang,
            "calculator.server.chart.group_silver_distribution_description",
        )),
        render_distribution_chart(
            "fish-group-silver-distribution-chart",
            &calculator_route_text(
                lang,
                "calculator.server.chart.aria.group_silver_distribution"
            ),
            "_calc.fish_group_silver_distribution_chart",
        ),
    )
}

fn render_loot_chart(lang: CalculatorLocale, chart: &LootChart) -> String {
    if !chart.available {
        return format!(
            "<div id=\"calculator-loot-chart\" class=\"rounded-box border border-dashed border-base-300 bg-base-200 p-4 text-sm text-base-content/70\">{}</div>",
            escape_html(&chart.note)
        );
    }

    format!(
        "<div id=\"calculator-loot-chart\" class=\"grid gap-4\">{}</div>",
        render_loot_sankey(lang, chart),
    )
}

fn render_target_fish_panel(
    data: &CalculatorData,
    signals: &CalculatorSignals,
    target_fish_options: &[SelectOption<'_>],
    target_fish_summary: &TargetFishSummary,
) -> String {
    if target_fish_options.is_empty() {
        return format!(
            "<div id=\"calculator-target-fish-panel\" class=\"rounded-box border border-dashed border-base-300 bg-base-200 p-4 text-sm text-base-content/70\">{}</div>",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.target.no_rows",
            ))
        );
    }

    let session_distribution_html = format!(
        "<div class=\"rounded-box border border-base-300 bg-base-200 p-4\" data-show=\"($_calc.target_fish_pmf_chart.bars || []).length > 0\">\
                <div class=\"mb-3 flex items-center justify-between gap-3\">\
                    <div>\
                        <div class=\"text-sm font-medium\">{}</div>\
                        <div class=\"text-xs text-base-content/70\">{}</div>\
                    </div>\
                    <div class=\"text-right text-xs text-base-content/70\">{}</div>\
                </div>\
                {}\
            </div>",
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.target.session_distribution_title",
            )),
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.target.session_distribution_description",
            )),
            escape_html(&calculator_route_text(
                data.lang,
                "calculator.server.target.count_bucket_probability",
            )),
            render_pmf_chart(
                "target-fish-pmf-chart",
                &calculator_route_text(
                    data.lang,
                    "calculator.server.chart.aria.target_distribution"
                ),
                "_calc.target_fish_pmf_chart",
            )
    );

    format!(
        "<div id=\"calculator-target-fish-panel\" class=\"grid gap-4\">\
            <div class=\"grid gap-3 md:grid-cols-[minmax(0,1fr)_10rem_10rem]\">\
                <fieldset class=\"fieldset\">\
                    <legend class=\"fieldset-legend\">{}</legend>\
                    <div id=\"calculator-target-fish-control\">{}</div>\
                </fieldset>\
                <fieldset class=\"fieldset\">\
                    <legend class=\"fieldset-legend\">{}</legend>\
                    <input type=\"number\" min=\"1\" step=\"1\" class=\"input input-sm w-full\" data-bind=\"targetFishAmount\">\
                    <span class=\"label text-xs\">{}</span>\
                </fieldset>\
                <fieldset class=\"fieldset\">\
                    <legend class=\"fieldset-legend\">{}</legend>\
                    <input type=\"number\" min=\"0\" step=\"1\" class=\"input input-sm w-full\" data-bind=\"targetFishPmfCount\">\
                    <span class=\"label text-xs\" data-text=\"$_calc.target_fish_pmf_count_hint\">{}</span>\
                </fieldset>\
            </div>\
            <div class=\"grid gap-3 lg:grid-cols-3\">\
                <div class=\"rounded-box border border-base-300 bg-base-200 px-4 py-3 fishy-explainable-stat\" tabindex=\"0\" data-attr:data-fishy-stat-breakdown=\"$_live.stat_breakdowns.target_expected_count || ''\" data-fishy-stat-color=\"var(--color-info)\">\
                    <div class=\"text-sm font-medium whitespace-normal leading-snug\" data-text=\"$_calc.target_fish_expected_title\">{}</div>\
                    <div class=\"mt-2 text-2xl font-semibold\" data-text=\"$_calc.target_fish_expected_count\">{}</div>\
                    <div class=\"mt-1 text-xs text-base-content/70\" data-text=\"$_calc.target_fish_status_text\">{}</div>\
                </div>\
                <div class=\"rounded-box border border-base-300 bg-base-200 px-4 py-3 fishy-explainable-stat\" tabindex=\"0\" data-attr:data-fishy-stat-breakdown=\"$_live.stat_breakdowns.target_time_to_target || ''\" data-fishy-stat-color=\"var(--color-info)\">\
                    <div class=\"text-sm font-medium\">{}</div>\
                    <div class=\"mt-2 text-2xl font-semibold\" data-text=\"$_calc.target_fish_time_to_target\">{}</div>\
                    <div class=\"mt-1 text-xs text-base-content/70\" data-text=\"$_calc.target_fish_time_to_target_helper\">{}</div>\
                </div>\
                <div class=\"rounded-box border border-base-300 bg-base-200 px-4 py-3 fishy-explainable-stat\" tabindex=\"0\" data-attr:data-fishy-stat-breakdown=\"$_live.stat_breakdowns.target_probability_at_least || ''\" data-fishy-stat-color=\"var(--color-info)\">\
                    <div class=\"text-sm font-medium\" data-text=\"$_calc.target_fish_probability_at_least_title\">{}</div>\
                    <div class=\"mt-2 text-2xl font-semibold\" data-text=\"$_calc.target_fish_probability_at_least\">{}</div>\
                    <div class=\"mt-1 text-xs text-base-content/70\">{}</div>\
                </div>\
            </div>\
            {}\
        </div>",
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.field.target_fish",
        )),
        render_target_fish_select_control(data, signals, target_fish_options),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.field.target_amount",
        )),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.helper.target_amount",
        )),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.field.pmf_max_count",
        )),
        escape_html(&target_fish_summary.pmf_count_hint_text),
        escape_html(&calculator_route_text_with_vars(
            data.lang,
            "calculator.server.target.expected",
            &[(
                "timespan",
                &timespan_text(data.lang, signals.timespan_amount, &signals.timespan_unit),
            )],
        )),
        escape_html(&target_fish_summary.expected_count_text),
        escape_html(&target_fish_summary.status_text),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.target.time_to_target",
        )),
        escape_html(&target_fish_summary.time_to_target_text),
        escape_html(&if target_fish_summary.selected_label.is_empty() {
            calculator_route_text(data.lang, "calculator.server.helper.select_target_fish")
        } else {
            calculator_route_text_with_vars(
                data.lang,
                "calculator.server.helper.target_status_per_day",
                &[
                    ("label", &target_fish_summary.selected_label),
                    ("per_day", &target_fish_summary.per_day_text),
                ],
            )
        }),
        escape_html(&calculator_route_text_with_vars(
            data.lang,
            "calculator.server.target.chance_at_least",
            &[("amount", &target_fish_summary.target_amount_text)],
        )),
        escape_html(&target_fish_summary.probability_at_least_text),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.helper.within_current_session_duration",
        )),
        session_distribution_html,
    )
}

fn render_fish_group_window(
    data: &CalculatorData,
    signals: &CalculatorSignals,
    fish_group_chart: &FishGroupChart,
    loot_chart: &LootChart,
    mastery: f64,
    target_fish_options: &[SelectOption<'_>],
    target_fish_summary: &TargetFishSummary,
) -> String {
    let title = calculator_route_text(data.lang, "calculator.server.section.distribution");
    format!(
        "<fieldset id=\"calculator-fish-group-window\" class=\"card card-border bg-base-100\">\
            {}\
            <div class=\"card-body gap-4 pt-0\">\
                {}\
                <div class=\"grid gap-4\">\
                    <div class=\"grid grid-cols-2 items-start gap-3\">\
                        <fieldset class=\"fieldset min-w-0\">\
                            <legend class=\"fieldset-legend\">{}</legend>\
                            <div class=\"grid gap-2\">\
                                <input type=\"number\" min=\"0\" max=\"3000\" step=\"50\" class=\"input input-sm w-full\" data-bind=\"mastery\" value=\"{}\">\
                                <input type=\"range\" min=\"0\" max=\"3000\" step=\"50\" class=\"range-xs range-secondary w-full\" data-bind=\"mastery\" value=\"{}\">\
                            </div>\
                            <span class=\"label text-xs\">{}</span>\
                        </fieldset>\
                        <div class=\"min-w-0 rounded-box border border-base-300 bg-base-100 px-3 py-3 fishy-explainable-stat\" tabindex=\"0\" data-attr:data-fishy-stat-breakdown=\"$_live.stat_breakdowns.raw_prize_rate || ''\" data-fishy-stat-color=\"var(--color-warning)\">\
                            <div class=\"text-sm font-medium\">{}</div>\
                            <div class=\"mt-1 text-xs text-base-content/70\">{} <span data-text=\"$_calc.raw_prize_mastery_text\">{}</span> {}</div>\
                            <div class=\"mt-3 text-2xl font-semibold\" data-text=\"$_calc.raw_prize_rate_text\">{}</div>\
                            <div class=\"text-xs text-base-content/70\">{}</div>\
                        </div>\
                    </div>\
                    <div class=\"grid gap-4\">\
                        <div class=\"grid gap-3 md:grid-cols-2\">\
                            <label class=\"label cursor-pointer justify-start gap-3 rounded-box border border-base-300 bg-base-100 px-3 py-3\">\
                                <input data-bind=\"showNormalizedSelectRates\" type=\"checkbox\" class=\"checkbox checkbox-primary checkbox-sm\"{}>\
                                <span class=\"text-sm font-medium\">{}</span>\
                            </label>\
                            <div class=\"rounded-box border border-base-300 bg-base-100 px-3 py-3\">\
                                <label class=\"mb-2 block text-sm font-medium\">{}</label>\
                                <select data-bind=\"discardGrade\" class=\"select select-sm w-full\">\
                                    <option value=\"none\">{}</option>\
                                    <option value=\"white\">{}</option>\
                                    <option value=\"green\">{}</option>\
                                    <option value=\"blue\">{}</option>\
                                    <option value=\"yellow\">{}</option>\
                                </select>\
                                <div class=\"mt-2 text-xs text-base-content/70\">{}</div>\
                            </div>\
                        </div>\
                        <div role=\"tablist\" class=\"tabs tabs-box bg-base-200/80 p-1\" aria-label=\"{}\">\
                            <button type=\"button\" class=\"tab\" data-class:tab-active=\"$_calculator_ui.distribution_tab === 'groups'\" data-attr:aria-selected=\"($_calculator_ui.distribution_tab === 'groups').toString()\" data-on:click=\"$_calculator_ui.distribution_tab = 'groups'\">{}</button>\
                            <button type=\"button\" class=\"tab\" data-class:tab-active=\"$_calculator_ui.distribution_tab === 'silver'\" data-attr:aria-selected=\"($_calculator_ui.distribution_tab === 'silver').toString()\" data-on:click=\"$_calculator_ui.distribution_tab = 'silver'\">{}</button>\
                            <button type=\"button\" class=\"tab\" data-class:tab-active=\"$_calculator_ui.distribution_tab === 'loot_flow'\" data-attr:aria-selected=\"($_calculator_ui.distribution_tab === 'loot_flow').toString()\" data-on:click=\"$_calculator_ui.distribution_tab = 'loot_flow'\">{}</button>\
                            <button type=\"button\" class=\"tab\" data-class:tab-active=\"$_calculator_ui.distribution_tab === 'target_fish'\" data-attr:aria-selected=\"($_calculator_ui.distribution_tab === 'target_fish').toString()\" data-on:click=\"$_calculator_ui.distribution_tab = 'target_fish'\">{}</button>\
                        </div>\
                        <div data-show=\"$_calculator_ui.distribution_tab === 'groups'\" class=\"grid gap-4\">{}\
                        </div>\
                        <div data-show=\"$_calculator_ui.distribution_tab === 'silver'\">{}\
                        </div>\
                        <div data-show=\"$_calculator_ui.distribution_tab === 'loot_flow'\">{}\
                        </div>\
                        <div data-show=\"$_calculator_ui.distribution_tab === 'target_fish'\">{}\
                        </div>\
                    </div>\
                </div>\
            </div>\
        </fieldset>",
        render_calculator_panel_legend(data.lang, "distribution", &title, None),
        render_calculator_data_disclaimer(data.lang),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.field.mastery",
        )),
        escape_html(&trim_float(mastery)),
        escape_html(&trim_float(mastery)),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.helper.mastery",
        )),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.stat.raw_prize_catch_rate",
        )),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.helper.mastery_formula_prefix",
        )),
        escape_html(&fish_group_chart.mastery_text),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.helper.mastery_formula_suffix",
        )),
        escape_html(&fish_group_chart.raw_prize_rate_text),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.helper.before_zone_group_normalization",
        )),
        if signals.show_normalized_select_rates {
            " checked"
        } else {
            ""
        },
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.helper.normalize_rates",
        )),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.field.discard_grade",
        )),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.discard.none"
        )),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.discard.white"
        )),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.discard.green"
        )),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.discard.blue"
        )),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.discard.yellow"
        )),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.helper.fish_only_notice",
        )),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.chart.aria.distribution_tabs",
        )),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.tab.groups"
        )),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.tab.silver"
        )),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.tab.loot_flow"
        )),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.tab.target_fish"
        )),
        render_fish_group_chart(
            data.lang,
            fish_group_chart,
            signals.show_normalized_select_rates
        ),
        render_fish_group_silver_chart(data.lang, loot_chart),
        render_loot_chart(data.lang, loot_chart),
        render_target_fish_panel(data, signals, target_fish_options, target_fish_summary),
    )
}

fn render_loot_window(data: &CalculatorData, signals: &CalculatorSignals) -> String {
    let title = calculator_route_text(data.lang, "calculator.server.section.loot");
    format!(
        "<fieldset id=\"calculator-loot-window\" class=\"card card-border bg-base-100 xl:col-span-2\">\
            {}\
            <div class=\"card-body gap-4 pt-0\">\
                {}\
                <div class=\"stats stats-vertical rounded-box border border-base-300 bg-base-100 shadow-none\">\
                    <div class=\"stat fishy-explainable-stat\" tabindex=\"0\" data-attr:data-fishy-stat-breakdown=\"$_live.stat_breakdowns.loot_total_catches || ''\" data-fishy-stat-color=\"var(--color-success)\">\
                        <div class=\"stat-title whitespace-normal leading-snug\">{} (<span data-text=\"$_live.timespan_text || {}\"></span>)</div>\
                        <div class=\"stat-value text-2xl\" data-text=\"$_live.loot_total_catches\"></div>\
                        <div class=\"stat-desc\">{} <span data-text=\"$_live.loot_fish_multiplier_text\"></span> {}</div>\
                    </div>\
                    <div class=\"stat fishy-explainable-stat\" tabindex=\"0\" data-attr:data-fishy-stat-breakdown=\"$_live.stat_breakdowns.loot_fish_per_hour || ''\" data-fishy-stat-color=\"var(--color-success)\">\
                        <div class=\"stat-title\">{}</div>\
                        <div class=\"stat-value text-2xl\" data-text=\"$_live.loot_fish_per_hour\"></div>\
                    </div>\
                    <div class=\"stat fishy-explainable-stat\" tabindex=\"0\" data-attr:data-fishy-stat-breakdown=\"$_live.stat_breakdowns.loot_total_profit || ''\" data-fishy-stat-color=\"var(--color-success)\">\
                        <div class=\"stat-title whitespace-normal leading-snug\">{} (<span data-text=\"$_live.timespan_text || {}\"></span>)</div>\
                        <div class=\"stat-value text-2xl\" data-text=\"$_live.loot_total_profit\"></div>\
                        <div class=\"stat-desc\">{} <span data-text=\"$_calc.trade_sale_multiplier_text\"></span></div>\
                    </div>\
                    <div class=\"stat fishy-explainable-stat\" tabindex=\"0\" data-attr:data-fishy-stat-breakdown=\"$_live.stat_breakdowns.loot_profit_per_hour || ''\" data-fishy-stat-color=\"var(--color-success)\">\
                        <div class=\"stat-title\">{}</div>\
                        <div class=\"stat-value text-2xl\" data-text=\"$_live.loot_profit_per_hour\"></div>\
                    </div>\
                </div>\
            </div>\
        </fieldset>",
        render_calculator_panel_legend(data.lang, "loot", &title, None),
        render_calculator_data_disclaimer(data.lang),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.stat.expected_catches",
        )),
        escaped_js_string_literal(&timespan_text(
            data.lang,
            signals.timespan_amount,
            &signals.timespan_unit,
        )),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.helper.using"
        )),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.helper.per_cast"
        )),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.stat.expected_catches_per_hour",
        )),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.stat.expected_profit",
        )),
        escaped_js_string_literal(&timespan_text(
            data.lang,
            signals.timespan_amount,
            &signals.timespan_unit,
        )),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.helper.sale"
        )),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.stat.profit_per_hour",
        )),
    )
}

fn render_trade_window(
    data: &CalculatorData,
    signals: &CalculatorSignals,
    trade_levels: &[SelectOption<'_>],
) -> String {
    let title = calculator_route_text(data.lang, "calculator.server.section.trade");
    format!(
        "<fieldset id=\"calculator-trade-window\" class=\"card card-border bg-base-100 xl:col-span-2\">\
            {}\
            <div class=\"card-body gap-4 pt-0\">\
                <div class=\"grid gap-3\">\
                    <fieldset class=\"fieldset\">\
                        <legend class=\"fieldset-legend\">{}</legend>\
                        {}\
                    </fieldset>\
                    <div class=\"grid gap-3 sm:grid-cols-2\">\
                        <fieldset class=\"fieldset\">\
                            <legend class=\"fieldset-legend\">{}</legend>\
                            <input type=\"number\" min=\"0\" step=\"any\" class=\"input input-sm w-full\" data-bind=\"tradeDistanceBonus\">\
                            <span class=\"label text-xs\">{}</span>\
                        </fieldset>\
                        <fieldset class=\"fieldset\">\
                            <legend class=\"fieldset-legend\">{}</legend>\
                            <input type=\"number\" min=\"0\" step=\"any\" class=\"input input-sm w-full\" data-bind=\"tradePriceCurve\">\
                            <span class=\"label text-xs\">{}</span>\
                        </fieldset>\
                    </div>\
                    <label class=\"label cursor-pointer justify-start gap-3 rounded-box border border-base-300 bg-base-100 px-3 py-3\">\
                        <input data-bind=\"applyTradeModifiers\" type=\"checkbox\" class=\"checkbox checkbox-primary checkbox-sm\">\
                        <span class=\"text-sm font-medium\">{}</span>\
                    </label>\
                    <div class=\"grid gap-3 sm:grid-cols-2\">\
                        <div class=\"rounded-box border border-base-300 bg-base-100 px-3 py-2 text-sm\">\
                            <span class=\"block text-xs text-base-content/70\">{}</span>\
                            <span class=\"font-medium\" data-text=\"$_calc.trade_bargain_bonus_text\"></span>\
                        </div>\
                        <div class=\"rounded-box border border-base-300 bg-base-100 px-3 py-2 text-sm\">\
                            <span class=\"block text-xs text-base-content/70\">{}</span>\
                            <span class=\"font-medium\" data-text=\"$_calc.trade_sale_multiplier_text\"></span>\
                        </div>\
                    </div>\
                </div>\
            </div>\
        </fieldset>",
        render_calculator_panel_legend(data.lang, "trade", &title, None),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.field.trade_level"
        )),
        render_searchable_select_control(
            data.cdn_base_url.as_str(),
            &data.api_lang,
            data.lang,
            "calculator-trade-level-picker",
            "calculator-trade-level-value",
            "trade_level",
            CalculatorSearchableOptionKind::TradeLevel,
            &signals.trade_level,
            trade_levels,
            false,
            &calculator_route_text(data.lang, "calculator.server.search.trade_levels"),
            false,
        ),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.field.distance_bonus",
        )),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.helper.distance_bonus",
        )),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.field.trade_price_curve",
        )),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.helper.trade_price_curve",
        )),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.helper.apply_trade_settings",
        )),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.stat.bargain_bonus",
        )),
        escape_html(&calculator_route_text(
            data.lang,
            "calculator.server.stat.sale_multiplier",
        )),
    )
}

fn render_calculator_data_disclaimer(lang: CalculatorLocale) -> String {
    format!(
        "<fishy-notice-disclosure class=\"not-prose\" title=\"Notice\" icon=\"alert-triangle\" settings-path=\"calculator.noticeOpen\" open>\
            <div class=\"rounded-box border px-4 py-4\" style=\"border-color: color-mix(in oklab, var(--color-warning, #c77d19) 56%, var(--color-base-300, #d4d4d8) 44%); background: color-mix(in oklab, var(--color-warning, #c77d19) 14%, var(--color-base-100, #ffffff) 86%);\">\
                <div class=\"flex items-start gap-3\">\
                    <div class=\"shrink-0 pt-0.5\" style=\"color: var(--color-warning, #f59e0b);\">\
                        <svg class=\"fishy-icon size-6\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><use width=\"100%\" height=\"100%\" href=\"{}#fishy-alert-fill\"></use></svg>\
                    </div>\
                    <div class=\"min-w-0\">\
                        <div class=\"text-sm font-semibold uppercase tracking-widest\" style=\"color: color-mix(in oklab, var(--color-warning, #c77d19) 78%, var(--color-base-content, #1f2937) 22%);\">{}</div>\
                        <div class=\"mt-2 space-y-2 text-sm leading-relaxed text-base-content/85\">\
                            <p>{}</p>\
                            <p>{}</p>\
                            <p>{}</p>\
                            <p>{}</p>\
                            <p>{}</p>\
                        </div>\
                    </div>\
                </div>\
            </div>\
        </fishy-notice-disclosure>",
        CALCULATOR_ICON_SPRITE_URL,
        escape_html(&calculator_route_text(
            lang,
            "calculator.server.disclaimer.title",
        )),
        escape_html(&calculator_route_text(
            lang,
            "calculator.server.disclaimer.p1"
        )),
        escape_html(&calculator_route_text(
            lang,
            "calculator.server.disclaimer.p2"
        )),
        escape_html(&calculator_route_text(
            lang,
            "calculator.server.disclaimer.p3"
        )),
        escape_html(&calculator_route_text(
            lang,
            "calculator.server.disclaimer.p4"
        )),
        escape_html(&calculator_route_text(
            lang,
            "calculator.server.disclaimer.p5"
        )),
    )
}

fn render_item_effect_badges(lang: CalculatorLocale, item: &CalculatorItemEntry) -> String {
    let text_with_vars =
        |key: &str, vars: &[(&str, &str)]| calculator_route_text_with_vars(lang, key, vars);
    let text = |key: &str| calculator_route_text(lang, key);
    let mut badges = Vec::new();
    if let Some(category_label) = buff_category_label(item) {
        badges.push(render_effect_badge(
            &category_label,
            "border-base-content/15 bg-base-300 text-base-content",
        ));
    }
    if let Some(afr) = item.afr.filter(|value| *value > 0.0) {
        badges.push(render_effect_badge(
            &text_with_vars(
                "calculator.server.badge.aft",
                &[("percent", &format_effect_percent(afr))],
            ),
            "border-blue-400 bg-blue-300 text-blue-950",
        ));
    }
    if let Some(bonus_rare) = item.bonus_rare.filter(|value| *value > 0.0) {
        badges.push(render_effect_badge(
            &text_with_vars(
                "calculator.server.badge.rare",
                &[("percent", &format_effect_percent(bonus_rare))],
            ),
            "border-yellow-400 bg-yellow-300 text-yellow-950",
        ));
    }
    if let Some(bonus_big) = item.bonus_big.filter(|value| *value > 0.0) {
        badges.push(render_effect_badge(
            &text_with_vars(
                "calculator.server.badge.hq",
                &[("percent", &format_effect_percent(bonus_big))],
            ),
            "border-blue-400 bg-blue-300 text-blue-950",
        ));
    }
    if let Some(item_drr) = item.item_drr.filter(|value| *value > 0.0) {
        badges.push(render_effect_badge(
            &text_with_vars(
                "calculator.server.badge.item_drr",
                &[("percent", &format_effect_percent(item_drr))],
            ),
            "border-amber-400 bg-amber-300 text-amber-950",
        ));
    }
    if let Some(fish_multiplier) = item
        .fish_multiplier
        .filter(|value| *value > 0.0 && (*value - 1.0).abs() > 0.0001)
    {
        badges.push(render_effect_badge(
            &text_with_vars(
                "calculator.server.badge.fish_multiplier",
                &[("multiplier", &trim_float(f64::from(fish_multiplier)))],
            ),
            "border-base-content/15 bg-base-300 text-base-content",
        ));
    }
    if let Some(exp_fish) = item.exp_fish.filter(|value| *value > 0.0) {
        badges.push(render_effect_badge(
            &text_with_vars(
                "calculator.server.badge.fish_exp",
                &[("percent", &format_effect_percent(exp_fish))],
            ),
            "border-cyan-400 bg-cyan-300 text-cyan-950",
        ));
    }
    if let Some(exp_life) = item.exp_life.filter(|value| *value > 0.0) {
        badges.push(render_effect_badge(
            &text_with_vars(
                "calculator.server.badge.life_exp",
                &[("percent", &format_effect_percent(exp_life))],
            ),
            "border-green-400 bg-green-300 text-green-950",
        ));
    }
    if badges.is_empty() && item.r#type == "outfit" {
        badges.push(render_effect_badge(
            &text("calculator.server.badge.set_effect"),
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

fn pet_skill_learn_chance_text(chance: f32) -> String {
    let percent = trim_float(f64::from(chance.max(0.0)) * 100.0);
    format!("{percent}%")
}

fn render_pet_skill_badges(lang: CalculatorLocale, skill: &CalculatorPetOptionEntry) -> String {
    let text_with_vars =
        |key: &str, vars: &[(&str, &str)]| calculator_route_text_with_vars(lang, key, vars);
    let mut badges = Vec::new();
    if let Some(exp_fish) = skill.fishing_exp.filter(|value| *value > 0.0) {
        badges.push(render_effect_badge(
            &text_with_vars(
                "calculator.server.badge.fish_exp",
                &[("percent", &format_effect_percent(exp_fish))],
            ),
            "border-cyan-400 bg-cyan-300 text-cyan-950",
        ));
    }
    if let Some(exp_life) = skill.life_exp.filter(|value| *value > 0.0) {
        badges.push(render_effect_badge(
            &text_with_vars(
                "calculator.server.badge.life_exp",
                &[("percent", &format_effect_percent(exp_life))],
            ),
            "border-green-400 bg-green-300 text-green-950",
        ));
    }
    if let Some(item_drr) = skill
        .durability_reduction_resistance
        .filter(|value| *value > 0.0)
    {
        badges.push(render_effect_badge(
            &text_with_vars(
                "calculator.server.badge.item_drr",
                &[("percent", &format_effect_percent(item_drr))],
            ),
            "border-amber-400 bg-amber-300 text-amber-950",
        ));
    }
    if badges.is_empty() {
        badges.push(render_effect_badge(
            &skill.label,
            "border-base-content/15 bg-base-300 text-base-content",
        ));
    }
    format!(
        "<span class=\"flex min-w-0 flex-wrap gap-1\">{}</span>",
        badges.join("")
    )
}

fn render_pet_skill_option_content_html(
    lang: CalculatorLocale,
    option: SelectOption<'_>,
) -> Option<String> {
    let chance = option.pet_skill_learn_chance?;
    let skill = option.pet_skill?;
    let chance_text = pet_skill_learn_chance_text(chance);
    Some(format!(
        "<span class=\"flex min-w-0 flex-1 items-center gap-2\"><span class=\"w-10 shrink-0 text-right font-medium tabular-nums text-base-content/70\">{}</span><span class=\"shrink-0 text-base-content/30\">|</span><span class=\"min-w-0 flex-1\">{}</span></span>",
        escape_html(&chance_text),
        render_pet_skill_badges(lang, skill),
    ))
}

fn render_pet_talent_badges_from_effects(
    lang: CalculatorLocale,
    talent: &CalculatorPetOptionEntry,
    effects: PetEffectiveTalentEffects,
    wrapper_class: &str,
    fallback_to_label: bool,
) -> String {
    let text_with_vars =
        |key: &str, vars: &[(&str, &str)]| calculator_route_text_with_vars(lang, key, vars);
    let mut badges = Vec::new();
    if let Some(item_drr) = effects.item_drr.filter(|value| *value > 0.0) {
        badges.push(render_effect_badge(
            &text_with_vars(
                "calculator.server.badge.item_drr",
                &[("percent", &trim_float(item_drr * 100.0))],
            ),
            "border-amber-400 bg-amber-300 text-amber-950",
        ));
    }
    if let Some(exp_life) = effects.life_exp.filter(|value| *value > 0.0) {
        badges.push(render_effect_badge(
            &text_with_vars(
                "calculator.server.badge.life_exp",
                &[("percent", &trim_float(exp_life * 100.0))],
            ),
            "border-green-400 bg-green-300 text-green-950",
        ));
    }
    if badges.is_empty() && fallback_to_label {
        badges.push(render_effect_badge(
            &talent.label,
            "border-base-content/15 bg-base-300 text-base-content",
        ));
    }
    if badges.is_empty() {
        return String::new();
    }
    format!(
        "<span class=\"{}\">{}</span>",
        escape_html(wrapper_class),
        badges.join("")
    )
}

fn raw_pet_talent_effects(talent: &CalculatorPetOptionEntry) -> PetEffectiveTalentEffects {
    PetEffectiveTalentEffects {
        item_drr: talent.durability_reduction_resistance.map(f64::from),
        life_exp: talent.life_exp.map(f64::from),
    }
}

fn effective_pet_talent_effects(
    pet: &CalculatorPetSignals,
    catalog: &CalculatorPetCatalog,
    talent: &CalculatorPetOptionEntry,
) -> PetEffectiveTalentEffects {
    let mut candidate_pet = pet.clone();
    candidate_pet.talent = talent.key.clone();
    PetEffectiveTalentEffects {
        item_drr: talent
            .durability_reduction_resistance
            .filter(|value| *value > 0.0)
            .map(|_| pet_drr(&candidate_pet, catalog)),
        life_exp: talent
            .life_exp
            .filter(|value| *value > 0.0)
            .map(|_| pet_life_exp(&candidate_pet, catalog)),
    }
}

fn render_pet_talent_badges(lang: CalculatorLocale, talent: &CalculatorPetOptionEntry) -> String {
    render_pet_talent_badges_from_effects(
        lang,
        talent,
        raw_pet_talent_effects(talent),
        "fishy-calculator-pet-option__badges",
        true,
    )
}

fn render_pet_effective_talent_badges(
    lang: CalculatorLocale,
    pet: &CalculatorPetSignals,
    catalog: &CalculatorPetCatalog,
    talent: &CalculatorPetOptionEntry,
) -> String {
    render_pet_talent_badges_from_effects(
        lang,
        talent,
        effective_pet_talent_effects(pet, catalog, talent),
        "flex min-w-0 flex-wrap gap-1",
        false,
    )
}

fn pet_special_badge_class(special: &CalculatorPetOptionEntry) -> &'static str {
    if special
        .auto_fishing_time_reduction
        .filter(|value| *value > 0.0)
        .is_some()
    {
        "fishy-calculator-pet-special-badge fishy-item-grade-blue"
    } else {
        "border-base-content/15 bg-base-300 text-base-content"
    }
}

fn render_pet_special_badge(special: &CalculatorPetOptionEntry) -> String {
    render_wrapping_effect_badge(&special.label, pet_special_badge_class(special))
}

fn render_pet_special_badges(special: &CalculatorPetOptionEntry) -> String {
    format!(
        "<span class=\"fishy-calculator-pet-option__badges\">{}</span>",
        render_pet_special_badge(special)
    )
}

fn render_pet_fixed_option_empty(lang: CalculatorLocale) -> String {
    format!(
        "<span class=\"block min-w-0 font-medium text-base-content/50\">{}</span>",
        escape_html(none_select_option(lang).label)
    )
}

fn render_pet_fixed_special_content(
    lang: CalculatorLocale,
    special: Option<&CalculatorPetOptionEntry>,
) -> String {
    special
        .map(render_pet_fixed_special_select_content)
        .unwrap_or_else(|| render_pet_fixed_option_empty(lang))
}

fn render_pet_fixed_talent_content(
    lang: CalculatorLocale,
    pet: &CalculatorPetSignals,
    catalog: &CalculatorPetCatalog,
    talent: Option<&CalculatorPetOptionEntry>,
) -> String {
    talent
        .map(|talent| render_pet_effective_talent_badges(lang, pet, catalog, talent))
        .unwrap_or_else(|| render_pet_fixed_option_empty(lang))
}

fn render_pet_fixed_special_select_content(special: &CalculatorPetOptionEntry) -> String {
    format!(
        "<span class=\"flex min-w-0 flex-wrap justify-start gap-1\">{}</span>",
        render_pet_special_badge(special)
    )
}

fn render_pet_fixed_talent_select_content(
    lang: CalculatorLocale,
    talent: &CalculatorPetOptionEntry,
    effective_effects: Option<PetEffectiveTalentEffects>,
) -> String {
    render_pet_talent_badges_from_effects(
        lang,
        talent,
        effective_effects.unwrap_or_else(|| raw_pet_talent_effects(talent)),
        "flex min-w-0 flex-wrap gap-1",
        true,
    )
}

fn render_pet_fixed_option_control(
    input_id: &str,
    bind_key: &str,
    selected_value: &str,
    content_html: &str,
) -> String {
    format!(
        "<input id=\"{}\" type=\"hidden\" data-bind=\"{}\" value=\"{}\">\
         {}",
        escape_html(input_id),
        escape_html(bind_key),
        escape_html(selected_value),
        render_pet_fixed_option_display(input_id, content_html),
    )
}

fn render_pet_fixed_option_content_id(input_id: &str) -> String {
    format!("{input_id}-content")
}

fn render_pet_fixed_option_display(input_id: &str, content_html: &str) -> String {
    format!(
        "<div id=\"{}\" class=\"min-h-10 rounded-box border border-base-300 bg-base-100 px-3 py-2 text-sm\" data-pet-fixed-option aria-live=\"polite\">{}</div>",
        escape_html(&render_pet_fixed_option_content_id(input_id)),
        content_html,
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

fn render_pet_dropdown_content_html(
    lang: CalculatorLocale,
    cdn_base_url: &str,
    option: SelectOption<'_>,
    selected: bool,
    include_fixed_badges: bool,
) -> String {
    let tone_class = format!("fishy-item-grade-{}", escape_html(option.grade_tone));
    let image_html = option
        .icon
        .map(|icon| {
            format!(
                "<img aria-hidden=\"true\" src=\"{}\" class=\"fishy-calculator-pet-option__image item-icon\" alt=\"{}\" data-fallback-label=\"{}\" loading=\"lazy\" decoding=\"async\">",
                escape_html(&absolute_public_asset_url(cdn_base_url, icon)),
                escape_html(option.label),
                escape_html(option.label),
            )
        })
        .unwrap_or_else(|| {
            format!(
                "<span class=\"fishy-calculator-pet-option__fallback fishy-item-icon-fallback {tone_class}\">{}</span>",
                escape_html(
                    &option
                        .label
                        .chars()
                        .next()
                        .map(|ch| ch.to_ascii_uppercase().to_string())
                        .unwrap_or_else(|| "?".to_string())
                )
            )
    });
    let mut badges_html = String::new();
    if include_fixed_badges {
        if let Some(special) = option.pet_variant_special {
            badges_html.push_str(&render_pet_special_badges(special));
        }
        if let Some(talent) = option.pet_variant_talent {
            let talent_badges = option
                .pet_effective_talent_effects
                .map(|effects| {
                    render_pet_talent_badges_from_effects(
                        lang,
                        talent,
                        effects,
                        "fishy-calculator-pet-option__badges",
                        true,
                    )
                })
                .unwrap_or_else(|| render_pet_talent_badges(lang, talent));
            badges_html.push_str(&talent_badges);
        }
    }
    format!(
        "<span class=\"fishy-calculator-pet-option{}\" data-pet-option-card><span class=\"fishy-calculator-pet-option__frame {}\">{}</span><span class=\"fishy-calculator-pet-option__label\">{}</span>{}</span>",
        if selected {
            " fishy-calculator-pet-option--selected"
        } else {
            ""
        },
        tone_class,
        image_html,
        escape_html(option.label),
        badges_html,
    )
}

fn render_pet_dropdown_option_content_html(
    lang: CalculatorLocale,
    cdn_base_url: &str,
    option: SelectOption<'_>,
) -> String {
    render_pet_dropdown_content_html(lang, cdn_base_url, option, false, true)
}

fn render_pet_dropdown_selected_content_html(
    lang: CalculatorLocale,
    cdn_base_url: &str,
    option: SelectOption<'_>,
) -> String {
    render_pet_dropdown_content_html(lang, cdn_base_url, option, true, false)
}

fn render_searchable_dropdown_option_content_html(
    lang: CalculatorLocale,
    cdn_base_url: &str,
    option: SelectOption<'_>,
) -> String {
    if matches!(option.presentation, SelectOptionPresentation::PetCard) {
        return render_pet_dropdown_option_content_html(lang, cdn_base_url, option);
    }
    if let Some(html) = render_pet_skill_option_content_html(lang, option) {
        return html;
    }
    if let Some(special) = option.pet_variant_special {
        return render_pet_fixed_special_select_content(special);
    }
    if let Some(talent) = option.pet_variant_talent {
        return render_pet_fixed_talent_select_content(
            lang,
            talent,
            option.pet_effective_talent_effects,
        );
    }

    let uses_item_presentation = option.icon.is_some();
    let grade_tone = escape_html(option.grade_tone);
    let tone_class = format!("fishy-item-grade-{grade_tone}");
    let mut html = String::new();
    if uses_item_presentation {
        write!(
            html,
            "<span class=\"fishy-item-row min-w-0 flex-1 {}\">",
            tone_class
        )
        .unwrap();
    } else {
        html.push_str("<span class=\"min-w-0 flex-1\">");
    }
    if let Some(icon) = option.icon {
        write!(
            html,
            "<span class=\"fishy-item-icon-frame is-md {tone_class}\"><img aria-hidden=\"true\" src=\"{}\" class=\"fishy-item-icon item-icon\" alt=\"{} icon\"/></span>",
            escape_html(&absolute_public_asset_url(cdn_base_url, icon)),
            escape_html(option.label)
        )
        .unwrap();
    }
    let badges = option
        .item
        .map(|item| render_item_effect_badges(lang, item))
        .or_else(|| {
            option.lifeskill_level.map(|level| {
                format!(
                    "<span class=\"mt-1 flex flex-wrap gap-1\">{}</span>",
                    render_effect_badge(
                        &calculator_route_text_with_vars(
                            lang,
                            "calculator.server.badge.level_drr",
                            &[("percent", &format_effect_percent(level.lifeskill_level_drr),)],
                        ),
                        "border-amber-400 bg-amber-300 text-amber-950",
                    )
                )
            })
        })
        .unwrap_or_default();
    if uses_item_presentation {
        write!(
            html,
            "<span class=\"min-w-0 flex-1\"><span class=\"fishy-item-label block truncate font-medium {tone_class}\">{}</span>{}</span>",
            escape_html(option.label),
            badges,
        )
        .unwrap();
    } else {
        write!(
            html,
            "<span class=\"min-w-0 flex-1\"><span class=\"block truncate font-medium text-base-content\">{}</span>{}</span>",
            escape_html(option.label),
            badges,
        )
        .unwrap();
    }
    html.push_str("</span>");
    html
}

fn render_searchable_dropdown_selected_content_html(
    lang: CalculatorLocale,
    cdn_base_url: &str,
    option: SelectOption<'_>,
) -> String {
    if matches!(option.presentation, SelectOptionPresentation::PetCard) {
        return render_pet_dropdown_selected_content_html(lang, cdn_base_url, option);
    }
    render_searchable_dropdown_option_content_html(lang, cdn_base_url, option)
}

fn render_select_option_search_text(option: SelectOption<'_>) -> String {
    let mut parts = vec![option.label.to_string()];
    if let Some(special) = option.pet_variant_special {
        let normalized = special.label.trim();
        if !normalized.is_empty() {
            parts.push(normalized.to_string());
        }
        if special
            .auto_fishing_time_reduction
            .filter(|value| *value > 0.0)
            .is_some()
        {
            parts.extend(
                [
                    "aft",
                    "afr",
                    "auto fishing",
                    "auto-fishing",
                    "auto fish time",
                ]
                .into_iter()
                .map(ToOwned::to_owned),
            );
        }
    }
    if let Some(talent) = option.pet_variant_talent {
        let normalized = talent.label.trim();
        if !normalized.is_empty() {
            parts.push(normalized.to_string());
        }
        if talent
            .durability_reduction_resistance
            .filter(|value| *value > 0.0)
            .is_some()
        {
            parts.extend(
                ["item drr", "durability reduction resistance", "durability"]
                    .into_iter()
                    .map(ToOwned::to_owned),
            );
        }
        if talent.life_exp.filter(|value| *value > 0.0).is_some() {
            parts.extend(
                ["life exp", "life experience"]
                    .into_iter()
                    .map(ToOwned::to_owned),
            );
        }
    }
    parts.join(" ")
}

fn with_optional_none<'a>(
    options: &[SelectOption<'a>],
    include_none: bool,
    lang: CalculatorLocale,
) -> Vec<SelectOption<'a>> {
    let mut values = Vec::with_capacity(options.len() + usize::from(include_none));
    if include_none {
        values.push(none_select_option(lang));
    }
    values.extend_from_slice(options);
    values
}

fn searchable_options_for_kind<'a>(
    data: &'a CalculatorData,
    kind: CalculatorSearchableOptionKind,
    pet_tier: Option<&str>,
    selected_value: Option<&str>,
    pet_context: Option<&CalculatorPetSignals>,
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
        CalculatorSearchableOptionKind::TargetFish => (target_fish_options(data), true),
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
        CalculatorSearchableOptionKind::Pet => (
            select_options_from_pet_entries_for_tier(
                &data.catalog.pets,
                pet_tier.unwrap_or(data.catalog.defaults.pet1.tier.as_str()),
                selected_value,
                pet_context,
            ),
            true,
        ),
        CalculatorSearchableOptionKind::PetTier => {
            (select_options_from_catalog(&data.catalog.pets.tiers), false)
        }
        CalculatorSearchableOptionKind::PetSpecial => (
            select_options_from_pet_options(&data.catalog.pets.specials),
            false,
        ),
        CalculatorSearchableOptionKind::PetTalent => (
            select_options_from_pet_options(&data.catalog.pets.talents),
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
        options.sort_by(|left, right| {
            (if left.value == current_value { 0 } else { 1 })
                .cmp(&(if right.value == current_value { 0 } else { 1 }))
                .then_with(|| left.sort_priority.cmp(&right.sort_priority))
                .then_with(|| compare_pet_skill_chance_desc(*left, *right))
                .then_with(|| left.label.cmp(right.label))
        });
        return options;
    }

    let matcher = SkimMatcherV2::default();
    let normalized_query = normalize_lookup_value(trimmed);
    let mut scored = options
        .into_iter()
        .filter_map(|option| {
            let search_text = render_select_option_search_text(option);
            matcher
                .fuzzy_match(&normalize_lookup_value(&search_text), &normalized_query)
                .map(|score| (option, score))
        })
        .collect::<Vec<_>>();
    scored.sort_by(|(left_option, left_score), (right_option, right_score)| {
        (if left_option.value == current_value {
            0
        } else {
            1
        })
        .cmp(
            &(if right_option.value == current_value {
                0
            } else {
                1
            }),
        )
        .then_with(|| right_score.cmp(left_score))
        .then_with(|| compare_pet_skill_chance_desc(*left_option, *right_option))
        .then_with(|| left_option.label.cmp(right_option.label))
    });
    scored.into_iter().map(|(option, _)| option).collect()
}

fn compare_pet_skill_chance_desc(left: SelectOption<'_>, right: SelectOption<'_>) -> Ordering {
    right
        .pet_skill_learn_chance
        .unwrap_or(-1.0)
        .partial_cmp(&left.pet_skill_learn_chance.unwrap_or(-1.0))
        .unwrap_or(Ordering::Equal)
}

fn render_searchable_dropdown_catalog_html(
    lang: CalculatorLocale,
    cdn_base_url: &str,
    options: &[SelectOption<'_>],
) -> String {
    let mut html = String::new();
    html.push_str("<div data-role=\"selected-content-catalog\" hidden>");
    for option in options {
        let selected_content_html =
            render_searchable_dropdown_selected_content_html(lang, cdn_base_url, *option);
        let option_content_html =
            render_searchable_dropdown_option_content_html(lang, cdn_base_url, *option);
        write!(
            html,
            "<template data-role=\"selected-content\" data-value=\"{}\" data-label=\"{}\" data-search-text=\"{}\">{}</template>",
            escape_html(option.value),
            escape_html(option.label),
            escape_html(&render_select_option_search_text(*option)),
            selected_content_html,
        )
        .unwrap();
        if selected_content_html != option_content_html {
            write!(
                html,
                "<template data-role=\"option-content\" data-value=\"{}\">{}</template>",
                escape_html(option.value),
                option_content_html,
            )
            .unwrap();
        }
    }
    html.push_str("</div>");
    html
}

fn render_searchable_select_results(
    lang: CalculatorLocale,
    cdn_base_url: &str,
    results_list_id: &str,
    options: &[SelectOption<'_>],
    current_value: &str,
    query: &str,
    offset: usize,
) -> String {
    let page = paginate_searchable_dropdown_items(
        fuzzy_select_matches(options, query, current_value),
        offset,
    );
    let next_offset_attr = page
        .next_offset
        .map(|value| format!(" data-next-offset=\"{}\"", value))
        .unwrap_or_default();
    let mut html = String::new();
    write!(
        html,
        "<ul id=\"{}\" tabindex=\"-1\" data-role=\"results\" class=\"menu menu-sm max-h-96 w-full gap-1 overflow-auto p-1\"{}>",
        escape_html(results_list_id),
        next_offset_attr,
    )
    .unwrap();
    if page.items.is_empty() {
        write!(
            html,
            "<li class=\"menu-disabled\"><span>{}</span></li>",
            escape_html(&calculator_route_text(
                lang,
                "calculator.server.result.no_matching_options",
            )),
        )
        .unwrap();
    } else {
        for option in page.items.iter().copied() {
            let is_selected = option.value == current_value;
            let selected_content_html =
                render_searchable_dropdown_selected_content_html(lang, cdn_base_url, option);
            let selected_marker = if is_selected {
                let selected_text =
                    calculator_route_text(lang, "calculator.server.result.selected");
                if option.pet_skill_learn_chance.is_some() {
                    format!(
                        "<span class=\"shrink-0 text-xs text-base-content/60\">({})</span>",
                        escape_html(&selected_text),
                    )
                } else {
                    format!(
                        "<span class=\"badge badge-soft badge-primary badge-xs\">{}</span>",
                        escape_html(&selected_text),
                    )
                }
            } else {
                String::new()
            };
            write!(
                html,
                "<li><button type=\"button\" class=\"justify-between gap-3 text-left{}\" data-searchable-dropdown-option data-value=\"{}\" data-label=\"{}\"><span data-role=\"option-content\" class=\"flex min-w-0 flex-1 items-center gap-3\">{}</span><template data-role=\"selected-content\">{}</template>{}</button></li>",
                if is_selected { " menu-active" } else { "" },
                escape_html(option.value),
                escape_html(option.label),
                render_searchable_dropdown_option_content_html(lang, cdn_base_url, option),
                selected_content_html,
                selected_marker
            )
            .unwrap();
        }
        if let Some(next_offset) = page.next_offset {
            html.push_str(&render_searchable_dropdown_more_results_row(
                lang,
                next_offset,
            ));
        }
    }
    html.push_str("</ul>");
    html
}

fn render_calculator_option_search_url(
    api_lang: &DataLang,
    lang: CalculatorLocale,
    kind: CalculatorSearchableOptionKind,
    results_id: &str,
) -> String {
    format!(
        "/api/v1/calculator/datastar/option-search?lang={}&locale={}&kind={}&results_id={}",
        lang_param(api_lang),
        locale_param(lang),
        kind.param(),
        results_id,
    )
}

fn render_calculator_pet_search_url(
    api_lang: &DataLang,
    lang: CalculatorLocale,
    results_id: &str,
    tier_key: &str,
    pack_leader: bool,
) -> String {
    format!(
        "/api/v1/calculator/datastar/option-search?lang={}&locale={}&kind=pet&results_id={}&tier={}&pack_leader={}",
        lang_param(api_lang),
        locale_param(lang),
        results_id,
        escape_html(tier_key),
        pack_leader,
    )
}

fn render_searchable_select_control(
    cdn_base_url: &str,
    api_lang: &DataLang,
    lang: CalculatorLocale,
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
    let options = with_optional_none(options, include_none, lang);
    let selected_option = options
        .iter()
        .copied()
        .find(|option| option.value == selected_value);
    let none_option = none_select_option(lang);
    let selected_label = selected_option
        .map(|option| option.label)
        .unwrap_or_else(|| {
            if selected_value.trim().is_empty() {
                none_option.label
            } else {
                selected_value
            }
        });
    let selected_content_html = selected_option
        .map(|option| render_searchable_dropdown_selected_content_html(lang, cdn_base_url, option))
        .unwrap_or_else(|| render_searchable_dropdown_text_content(selected_label));
    let catalog_html = render_searchable_dropdown_catalog_html(lang, cdn_base_url, &options);
    let results_html = render_searchable_select_results(
        lang,
        cdn_base_url,
        &results_id,
        &options,
        selected_value,
        "",
        0,
    );
    let search_url = render_calculator_option_search_url(api_lang, lang, kind, &results_id);
    let dropdown = render_searchable_dropdown(
        &SearchableDropdownConfig {
            catalog_html: Some(&catalog_html),
            compact,
            trigger_size: SearchableDropdownTriggerSize::Fill,
            trigger_width: None,
            trigger_min_height: None,
            panel_width: None,
            panel_placement: SearchableDropdownPanelPlacement::Adjacent,
            results_layout: SearchableDropdownResultsLayout::List,
            root_id,
            input_id,
            label: selected_label,
            selected_content_html: &selected_content_html,
            value: selected_value,
            search_url: &search_url,
            search_url_root: Some("api"),
            exclude_selected_inputs: None,
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

fn render_pet_select_control(
    cdn_base_url: &str,
    api_lang: &DataLang,
    lang: CalculatorLocale,
    root_id: &str,
    input_id: &str,
    bind_key: &str,
    selected_value: &str,
    tier_key: &str,
    pack_leader: bool,
    options: &[SelectOption<'_>],
    search_placeholder: &str,
) -> String {
    let results_id = format!("{root_id}-results");
    let options = with_optional_none(options, true, lang);
    let selected_option = options
        .iter()
        .copied()
        .find(|option| option.value == selected_value);
    let none_option = none_select_option(lang);
    let selected_label = selected_option
        .map(|option| option.label)
        .unwrap_or_else(|| {
            if selected_value.trim().is_empty() {
                none_option.label
            } else {
                selected_value
            }
        });
    let selected_content_html = selected_option
        .map(|option| render_searchable_dropdown_selected_content_html(lang, cdn_base_url, option))
        .unwrap_or_else(|| render_searchable_dropdown_text_content(selected_label));
    let results_html = render_searchable_select_results(
        lang,
        cdn_base_url,
        &results_id,
        &options,
        selected_value,
        "",
        0,
    );
    let search_url =
        render_calculator_pet_search_url(api_lang, lang, &results_id, tier_key, pack_leader);
    let dropdown = render_searchable_dropdown(
        &SearchableDropdownConfig {
            catalog_html: None,
            compact: false,
            trigger_size: SearchableDropdownTriggerSize::Content,
            trigger_width: None,
            trigger_min_height: None,
            panel_width: Some("60rem"),
            panel_placement: SearchableDropdownPanelPlacement::OverlayAnchor,
            results_layout: SearchableDropdownResultsLayout::Cards,
            root_id,
            input_id,
            label: selected_label,
            selected_content_html: &selected_content_html,
            value: selected_value,
            search_url: &search_url,
            search_url_root: Some("api"),
            exclude_selected_inputs: None,
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

fn pet_skill_slot_values(selected_values: &[String]) -> Vec<String> {
    (0..3)
        .map(|index| selected_values.get(index).cloned().unwrap_or_default())
        .collect()
}

fn pet_skill_options_for_slot<'a>(
    options: &[SelectOption<'a>],
    selected_values: &[String],
    slot_index: usize,
) -> Vec<SelectOption<'a>> {
    let own_value = selected_values
        .get(slot_index)
        .map(|value| value.as_str())
        .unwrap_or_default();
    let excluded = selected_values
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != slot_index)
        .map(|(_, value)| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .collect::<HashSet<_>>();

    options
        .iter()
        .copied()
        .filter(|option| option.value == own_value || !excluded.contains(option.value))
        .collect()
}

fn render_pet_skill_select_control(
    lang: CalculatorLocale,
    root_id: &str,
    input_id: &str,
    bind_key: &str,
    selected_value: &str,
    options: &[SelectOption<'_>],
    include_none: bool,
    input_group: &str,
    exclude_selected_inputs: &str,
    search_placeholder: &str,
) -> String {
    let results_id = format!("{root_id}-results");
    let options = with_optional_none(options, include_none, lang);
    let selected_option = options
        .iter()
        .copied()
        .find(|option| option.value == selected_value);
    let none_option = none_select_option(lang);
    let selected_label = selected_option
        .map(|option| option.label)
        .unwrap_or_else(|| {
            if selected_value.trim().is_empty() {
                none_option.label
            } else {
                selected_value
            }
        });
    let selected_content_html = selected_option
        .map(|option| render_searchable_dropdown_selected_content_html(lang, "", option))
        .unwrap_or_else(|| render_searchable_dropdown_text_content(selected_label));
    let catalog_html = render_searchable_dropdown_catalog_html(lang, "", &options);
    let results_html =
        render_searchable_select_results(lang, "", &results_id, &options, selected_value, "", 0);
    let dropdown = render_searchable_dropdown(
        &SearchableDropdownConfig {
            catalog_html: Some(&catalog_html),
            compact: true,
            trigger_size: SearchableDropdownTriggerSize::Fill,
            trigger_width: None,
            trigger_min_height: None,
            panel_width: Some("32rem"),
            panel_placement: SearchableDropdownPanelPlacement::OverlayAnchor,
            results_layout: SearchableDropdownResultsLayout::List,
            root_id,
            input_id,
            label: selected_label,
            selected_content_html: &selected_content_html,
            value: selected_value,
            search_url: "",
            search_url_root: None,
            exclude_selected_inputs: Some(exclude_selected_inputs),
            search_placeholder,
        },
        &results_html,
    );

    format!(
        "<input id=\"{}\" type=\"hidden\" data-bind=\"{}\" data-pet-skill-input data-pet-skill-input-group=\"{}\" value=\"{}\">{}",
        escape_html(input_id),
        escape_html(bind_key),
        escape_html(input_group),
        escape_html(selected_value),
        dropdown,
    )
}

fn render_pet_skill_selects(
    lang: CalculatorLocale,
    pet_slot: usize,
    skill_limit: usize,
    selected_values: &[String],
    options: &[SelectOption<'_>],
) -> String {
    let mut sorted_options = options.to_vec();
    sorted_options.sort_by(|left, right| {
        compare_pet_skill_chance_desc(*left, *right).then_with(|| left.label.cmp(right.label))
    });

    let selected_values = pet_skill_slot_values(selected_values);
    let group_key = format!("pet{pet_slot}");
    let exclude_selector = format!(r#"[data-pet-skill-input-group="{group_key}"]"#);
    let search_placeholder = calculator_route_text(lang, "calculator.server.search.pet_skills");
    let active_slots = skill_limit.clamp(1, 3);
    let mut html = String::new();
    write!(
        html,
        "<div id=\"pet{}_skills\" class=\"fishy-calculator-pet-skills-grid grid gap-2\">",
        pet_slot
    )
    .unwrap();
    for skill_slot in 1..=3 {
        let bind_key = format!("_pet{pet_slot}_skill_slot{skill_slot}");
        let input_id = format!("calculator-pet{pet_slot}-skill-slot{skill_slot}-value");
        if skill_slot > active_slots {
            write!(
                html,
                "<input id=\"{}\" type=\"hidden\" data-bind=\"{}\" data-pet-skill-input data-pet-skill-input-group=\"{}\" value=\"\">",
                escape_html(&input_id),
                escape_html(&bind_key),
                escape_html(&group_key),
            )
            .unwrap();
            continue;
        }

        let slot_index = skill_slot - 1;
        let slot_options =
            pet_skill_options_for_slot(&sorted_options, &selected_values, slot_index);
        html.push_str(&render_pet_skill_select_control(
            lang,
            &format!("calculator-pet{pet_slot}-skill-slot{skill_slot}-picker"),
            &input_id,
            &bind_key,
            selected_values
                .get(slot_index)
                .map(|value| value.as_str())
                .unwrap_or_default(),
            &slot_options,
            skill_slot > 1,
            &group_key,
            &exclude_selector,
            &search_placeholder,
        ));
    }
    html.push_str("</div>");
    html
}

fn pet_same_tier_variant_option_keys(
    catalog: &CalculatorPetCatalog,
    selected_pet: &CalculatorPetEntry,
    tier_key: &str,
    kind: PetFixedOptionKind,
) -> Vec<String> {
    let mut keys = Vec::new();
    for (_, tier) in pet_same_tier_variant_entries(catalog, selected_pet, tier_key) {
        match kind {
            PetFixedOptionKind::Special => keys.extend(tier.specials.iter().cloned()),
            PetFixedOptionKind::Talent => keys.extend(tier.talents.iter().cloned()),
        }
    }
    let mut seen = HashSet::new();
    keys.retain(|key| seen.insert(key.clone()));
    keys
}

fn select_pet_fixed_option_entries_by_keys<'a>(
    keys: &[String],
    options: &'a [CalculatorPetOptionEntry],
    kind: PetFixedOptionKind,
    pet_context: Option<&CalculatorPetSignals>,
    catalog: Option<&CalculatorPetCatalog>,
) -> Vec<SelectOption<'a>> {
    keys.iter()
        .filter_map(|key| options.iter().find(|option| option.key == *key))
        .map(|option| {
            let pet_effective_talent_effects = match (kind, pet_context, catalog) {
                (PetFixedOptionKind::Talent, Some(pet), Some(catalog)) => {
                    Some(effective_pet_talent_effects(pet, catalog, option))
                }
                _ => None,
            };
            SelectOption {
                value: option.key.as_str(),
                label: option.label.as_str(),
                icon: None,
                grade_tone: "unknown",
                pet_variant_talent: (kind == PetFixedOptionKind::Talent).then_some(option),
                pet_variant_special: (kind == PetFixedOptionKind::Special).then_some(option),
                pet_skill: None,
                pet_effective_talent_effects,
                pet_skill_learn_chance: None,
                item: None,
                lifeskill_level: None,
                presentation: SelectOptionPresentation::Default,
                sort_priority: 1,
            }
        })
        .collect()
}

fn render_pet_variant_fixed_option_select_control(
    lang: CalculatorLocale,
    root_id: &str,
    input_id: &str,
    bind_key: &str,
    selected_value: &str,
    options: &[SelectOption<'_>],
    selected_content_html: &str,
    search_placeholder: &str,
) -> String {
    let results_id = format!("{root_id}-results");
    let selected_label = options
        .iter()
        .copied()
        .find(|option| option.value == selected_value)
        .map(|option| option.label)
        .unwrap_or(selected_value);
    let catalog_html = render_searchable_dropdown_catalog_html(lang, "", options);
    let results_html =
        render_searchable_select_results(lang, "", &results_id, options, selected_value, "", 0);
    let dropdown = render_searchable_dropdown(
        &SearchableDropdownConfig {
            catalog_html: Some(&catalog_html),
            compact: true,
            trigger_size: SearchableDropdownTriggerSize::Fill,
            trigger_width: None,
            trigger_min_height: None,
            panel_width: Some("18rem"),
            panel_placement: SearchableDropdownPanelPlacement::OverlayAnchor,
            results_layout: SearchableDropdownResultsLayout::List,
            root_id,
            input_id,
            label: selected_label,
            selected_content_html,
            value: selected_value,
            search_url: "",
            search_url_root: None,
            exclude_selected_inputs: None,
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

fn render_target_fish_select_control(
    data: &CalculatorData,
    signals: &CalculatorSignals,
    options: &[SelectOption<'_>],
) -> String {
    let root_id = "calculator-target-fish-picker";
    let input_id = "calculator-target-fish-value";
    let results_id = format!("{root_id}-results");
    let options = with_optional_none(options, true, data.lang);
    let selected_option = options
        .iter()
        .copied()
        .find(|option| option.value == signals.target_fish);
    let selected_label = selected_option
        .map(|option| option.label)
        .unwrap_or_else(|| none_select_option(data.lang).label);
    let selected_content_html = selected_option
        .map(|option| {
            render_searchable_dropdown_selected_content_html(
                data.lang,
                data.cdn_base_url.as_str(),
                option,
            )
        })
        .unwrap_or_else(|| render_searchable_dropdown_text_content(selected_label));
    let catalog_html =
        render_searchable_dropdown_catalog_html(data.lang, data.cdn_base_url.as_str(), &options);
    let results_html = render_searchable_select_results(
        data.lang,
        data.cdn_base_url.as_str(),
        &results_id,
        &options,
        &signals.target_fish,
        "",
        0,
    );
    let search_url = format!(
        "/api/v1/calculator/datastar/option-search?lang={}&locale={}&kind=target_fish&results_id={}&zone={}",
        lang_param(&data.api_lang),
        locale_param(data.lang),
        escape_html(&results_id),
        escape_html(&signals.zone),
    );
    let dropdown = render_searchable_dropdown(
        &SearchableDropdownConfig {
            catalog_html: Some(&catalog_html),
            compact: false,
            trigger_size: SearchableDropdownTriggerSize::Fill,
            trigger_width: None,
            trigger_min_height: None,
            panel_width: Some("34rem"),
            panel_placement: SearchableDropdownPanelPlacement::Adjacent,
            results_layout: SearchableDropdownResultsLayout::List,
            root_id,
            input_id,
            label: selected_label,
            selected_content_html: &selected_content_html,
            value: &signals.target_fish,
            search_url: &search_url,
            search_url_root: Some("api"),
            exclude_selected_inputs: None,
            search_placeholder: &calculator_route_text(
                data.lang,
                "calculator.server.search.loot_rows",
            ),
        },
        &results_html,
    );

    format!(
        "<input id=\"{}\" type=\"hidden\" data-bind=\"targetFish\" value=\"{}\">{}",
        escape_html(input_id),
        escape_html(&signals.target_fish),
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
    lang: CalculatorLocale,
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
            render_searchable_dropdown_option_content_html(lang, cdn_base_url, *option),
        )
        .unwrap();
    }
    html.push_str("</div>");
    html
}

fn render_searchable_multiselect_bound_select_html(
    bind_key: &str,
    selected_values: &[String],
    options: &[SelectOption<'_>],
) -> String {
    let selected = selected_values
        .iter()
        .map(|value| value.as_str())
        .collect::<std::collections::HashSet<_>>();
    let mut html = format!(
        "<select data-role=\"bound-select\" data-bind=\"{}\" multiple>",
        escape_html(bind_key)
    );
    for option in options {
        let selected_attr = if selected.contains(option.value) {
            " selected"
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
            "<option data-role=\"bound-option\" data-label=\"{}\" value=\"{}\"{}{}>{}</option>",
            escape_html(option.label),
            escape_html(option.value),
            selected_attr,
            category_key_attr,
            escape_html(option.label),
        )
        .unwrap();
    }
    html.push_str("</select>");
    html
}

fn render_searchable_multiselect_selection_html(
    lang: CalculatorLocale,
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
            "<div class=\"join items-stretch rounded-box border border-base-300 bg-base-100 p-1 text-base-content shadow-sm\"><span class=\"inline-flex min-w-0 items-center px-2 py-1 text-sm\">{}</span><button type=\"button\" class=\"btn btn-ghost btn-xs btn-circle join-item h-7 min-h-0 w-7 border-0 text-base-content/70\" data-searchable-multiselect-remove data-value=\"{}\" aria-label=\"{}\">×</button></div>",
            render_searchable_dropdown_option_content_html(lang, cdn_base_url, option),
            escape_html(option.value),
            escape_html(&calculator_route_text_with_vars(
                lang,
                "calculator.server.action.remove",
                &[("label", option.label)],
            )),
        )
        .unwrap();
    }
    html
}

fn render_searchable_multiselect_results_html(
    lang: CalculatorLocale,
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
    matches.truncate(SEARCHABLE_MULTISELECT_RESULT_LIMIT);

    let mut html = String::new();
    html.push_str(
        "<ul data-role=\"results\" class=\"menu menu-sm max-h-96 w-full gap-1 overflow-auto p-1\">",
    );
    if matches.is_empty() {
        write!(
            html,
            "<li class=\"menu-disabled\"><span>{}</span></li>",
            escape_html(&calculator_route_text(
                lang,
                "calculator.server.result.no_matching_options",
            )),
        )
        .unwrap();
    } else {
        for option in matches {
            let is_selected = selected.contains(option.value);
            let selected_badge = if is_selected {
                format!(
                    "<span class=\"badge badge-soft badge-primary badge-xs\">{}</span>",
                    escape_html(&calculator_route_text(
                        lang,
                        "calculator.server.result.added",
                    )),
                )
            } else {
                String::new()
            };
            write!(
                html,
                "<li><button type=\"button\" class=\"justify-between gap-3 text-left{}\" data-searchable-multiselect-option data-selected=\"{}\" data-value=\"{}\" data-label=\"{}\"><span class=\"flex min-w-0 flex-1 items-center gap-3\">{}</span>{}</button></li>",
                if is_selected { " opacity-75" } else { "" },
                if is_selected { "true" } else { "false" },
                escape_html(option.value),
                escape_html(option.label),
                render_searchable_dropdown_option_content_html(lang, cdn_base_url, option),
                selected_badge
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
    let bound_inputs_id = format!("{}-bound-inputs", config.root_id);
    let selection_html = render_searchable_multiselect_selection_html(
        config.lang,
        cdn_base_url,
        selected_values,
        &options,
    );
    let results_html = render_searchable_multiselect_results_html(
        config.lang,
        cdn_base_url,
        &options,
        selected_values,
        "",
    );
    let catalog_html =
        render_searchable_multiselect_catalog_html(config.lang, cdn_base_url, &options);
    let bound_select_html =
        render_searchable_multiselect_bound_select_html(config.bind_key, selected_values, &options);
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
        r#"<div class="grid gap-2">
    <div id="{bound_inputs_id}" data-role="bound-inputs-root" hidden>{bound_select_html}</div>
    <fishy-searchable-multiselect id="{root_id}" class="relative block w-full" placeholder="{search_placeholder}" bound-select-id="{bound_inputs_id}">
    <div data-role="shell" class="flex min-h-11 w-full flex-wrap items-center gap-2 rounded-box border border-base-300 bg-base-100 px-3 py-2 shadow-sm">
        <div data-role="selection" class="flex flex-wrap gap-2"{selection_hidden_attr}>{selection_html}</div>
        <label class="flex min-w-[12rem] flex-1 items-center gap-2 text-sm">
        <svg class="fishy-icon size-4 opacity-60" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="{icon_sprite_url}#fishy-search-field"></use></svg>
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
</fishy-searchable-multiselect>
</div>"#,
        bound_inputs_id = escape_html(&bound_inputs_id),
        root_id = escape_html(config.root_id),
        panel_id = escape_html(&panel_id),
        search_input_id = escape_html(&search_input_id),
        selection_hidden_attr = selection_hidden_attr,
        selection_html = selection_html,
        search_placeholder = escape_html(config.search_placeholder),
        results_html = results_html,
        helper_text_html = helper_text_html,
        catalog_html = catalog_html,
        bound_select_html = bound_select_html,
        icon_sprite_url = CALCULATOR_ICON_SPRITE_URL,
    )
}

fn render_zone_search_results(
    lang: CalculatorLocale,
    results_list_id: &str,
    zones: &[ZoneEntry],
    current_zone: &str,
    query: &str,
    _offset: usize,
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
        write!(
            html,
            "<li class=\"menu-disabled\"><span>{}</span></li>",
            escape_html(&calculator_route_text(
                lang,
                "calculator.server.result.no_matching_zones",
            )),
        )
        .unwrap();
    } else {
        for zone in matches {
            let label = zone_name(zone);
            let is_selected = zone.rgb_key.0 == current_zone;
            let active_class = if is_selected { " menu-active" } else { "" };
            let option_content = render_searchable_dropdown_text_content(label);
            let selected_badge = if is_selected {
                format!(
                    "<span class=\"badge badge-soft badge-primary badge-xs\">{}</span>",
                    escape_html(&calculator_route_text(
                        lang,
                        "calculator.server.result.selected",
                    )),
                )
            } else {
                String::new()
            };
            write!(
                html,
                "<li><button type=\"button\" class=\"justify-between gap-3 text-left{}\" data-searchable-dropdown-option data-value=\"{}\" data-label=\"{}\"><span data-role=\"option-content\" class=\"flex min-w-0 flex-1 items-center gap-3\">{}</span>{}</button></li>",
                active_class,
                escape_html(&zone.rgb_key.0),
                escape_html(label),
                option_content,
                selected_badge
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
    let content_sized_trigger = config.trigger_size == SearchableDropdownTriggerSize::Content;
    let trigger_class = if content_sized_trigger {
        "inline-flex w-auto max-w-full items-center justify-between gap-2 overflow-hidden rounded-box border border-base-300 bg-base-100 p-0 text-left shadow-sm"
    } else if config.compact {
        "flex min-h-10 w-full items-center justify-between gap-3 rounded-box border border-base-300 bg-base-100 px-3 py-2 text-left text-sm shadow-sm"
    } else {
        "flex min-h-11 w-full items-center justify-between gap-3 rounded-box border border-base-300 bg-base-100 px-4 py-3 text-left shadow-sm"
    };
    let search_shell_class = if config.compact {
        "flex min-h-10 w-full min-w-full items-center gap-3 bg-base-100 px-3 py-2 text-sm"
    } else {
        "flex min-h-11 w-full min-w-full items-center gap-3 bg-base-100 px-4 py-3"
    };
    let selected_content_class = if content_sized_trigger {
        "flex min-w-0 items-center"
    } else if config.compact {
        "flex min-w-0 flex-1 items-center gap-3 text-sm"
    } else {
        "flex min-w-0 flex-1 items-center gap-3"
    };
    let trigger_size_attr = if content_sized_trigger {
        " trigger-size=\"content\""
    } else {
        ""
    };
    let mut trigger_style_parts = Vec::new();
    if let Some(width) = config.trigger_width {
        trigger_style_parts.push(format!(
            "--fishy-searchable-dropdown-trigger-width: {};",
            escape_html(width)
        ));
    }
    if let Some(min_height) = config.trigger_min_height {
        trigger_style_parts.push(format!(
            "--fishy-searchable-dropdown-trigger-min-height: {};",
            escape_html(min_height)
        ));
    }
    let trigger_style_attr = if trigger_style_parts.is_empty() {
        String::new()
    } else {
        format!(" style=\"{}\"", trigger_style_parts.join(" "))
    };
    let panel_attrs = config
        .panel_width
        .map(|width| {
            format!(
                " panel-mode=\"detached\" panel-min-width=\"panel\" panel-width=\"{}\"",
                escape_html(width)
            )
        })
        .unwrap_or_default();
    let panel_placement_attr =
        if config.panel_placement == SearchableDropdownPanelPlacement::OverlayAnchor {
            " panel-placement=\"overlay-anchor\""
        } else {
            ""
        };
    let results_layout_attr = if config.results_layout == SearchableDropdownResultsLayout::Cards {
        " results-layout=\"cards\""
    } else {
        ""
    };
    let panel_results_layout_attr =
        if config.results_layout == SearchableDropdownResultsLayout::Cards {
            " data-searchable-dropdown-results-layout=\"cards\""
        } else {
            ""
        };
    let results_wrapper_class = if config.results_layout == SearchableDropdownResultsLayout::Cards {
        "p-0"
    } else {
        "px-1 pb-1"
    };
    let search_url_root_attr = config
        .search_url_root
        .map(|value| format!(" search-url-root=\"{}\"", escape_html(value)))
        .unwrap_or_default();
    let exclude_selected_inputs_attr = config
        .exclude_selected_inputs
        .map(|value| format!(" exclude-selected-inputs=\"{}\"", escape_html(value)))
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
     {exclude_selected_inputs_attr}
     placeholder="{search_placeholder}"{trigger_size_attr}{panel_attrs}{panel_placement_attr}{results_layout_attr}{trigger_style_attr}>
    <button type="button"
            data-role="trigger"
            class="{trigger_class}"
            aria-haspopup="listbox"
            aria-expanded="false"
            aria-controls="{panel_id}">
        <span data-role="selected-content" class="{selected_content_class}">{selected_content_html}</span>
        <svg class="fishy-icon size-4 opacity-60" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="{icon_sprite_url}#fishy-caret-down"></use></svg>
    </button>

    <div id="{panel_id}" data-role="panel" class="absolute left-0 top-0 z-50 w-full min-w-full max-w-full"{panel_results_layout_attr} hidden>
        <div class="grid w-full min-w-full overflow-hidden rounded-box border border-base-300 bg-base-100 shadow-lg">
            <label class="{search_shell_class}">
                <svg class="fishy-icon size-4 opacity-60" viewBox="0 0 24 24" aria-hidden="true"><use width="100%" height="100%" href="{icon_sprite_url}#fishy-search-field"></use></svg>
                <input id="{search_input_id}"
                       data-role="search-input"
                       type="search"
                       class="w-full border-0 bg-transparent p-0 shadow-none outline-none"
                       style="outline: none; box-shadow: none;"
                       placeholder="{search_placeholder}"
                       autocomplete="off"
                       spellcheck="false">
            </label>
            <div class="{results_wrapper_class}">
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
        exclude_selected_inputs_attr = exclude_selected_inputs_attr,
        panel_id = escape_html(&panel_id),
        search_input_id = escape_html(&search_input_id),
        search_placeholder = escape_html(config.search_placeholder),
        trigger_size_attr = trigger_size_attr,
        trigger_style_attr = trigger_style_attr,
        panel_attrs = panel_attrs,
        panel_placement_attr = panel_placement_attr,
        results_layout_attr = results_layout_attr,
        panel_results_layout_attr = panel_results_layout_attr,
        results_wrapper_class = results_wrapper_class,
        results_html = results_html,
        trigger_class = trigger_class,
        icon_sprite_url = CALCULATOR_ICON_SPRITE_URL,
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
            grade_tone: "unknown",
            pet_variant_talent: None,
            pet_variant_special: None,
            pet_skill: None,
            pet_effective_talent_effects: None,
            pet_skill_learn_chance: None,
            item: None,
            lifeskill_level: Some(level),
            presentation: SelectOptionPresentation::Default,
            sort_priority: 1,
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
            grade_tone: item_grade_tone(item.grade.as_deref()),
            pet_variant_talent: None,
            pet_variant_special: None,
            pet_skill: None,
            pet_effective_talent_effects: None,
            pet_skill_learn_chance: None,
            item: Some(item),
            lifeskill_level: None,
            presentation: SelectOptionPresentation::Default,
            sort_priority: 1,
        })
        .collect()
}

fn target_fish_options<'a>(data: &'a CalculatorData) -> Vec<SelectOption<'a>> {
    let mut seen = HashSet::<String>::new();
    let mut options = data
        .zone_loot_entries
        .iter()
        .filter(|entry| {
            let key = normalize_lookup_value(&entry.name);
            !key.is_empty() && seen.insert(key)
        })
        .map(|entry| SelectOption {
            value: entry.name.as_str(),
            label: entry.name.as_str(),
            icon: entry.icon.as_deref(),
            grade_tone: item_grade_tone(entry.grade.as_deref()),
            pet_variant_talent: None,
            pet_variant_special: None,
            pet_skill: None,
            pet_effective_talent_effects: None,
            pet_skill_learn_chance: None,
            item: None,
            lifeskill_level: None,
            presentation: SelectOptionPresentation::Default,
            sort_priority: 1,
        })
        .collect::<Vec<_>>();
    options.sort_by(|left, right| left.label.cmp(right.label));
    options
}

fn render_checkbox_group(
    lang: CalculatorLocale,
    cdn_base_url: &str,
    id: &str,
    bind_key: &str,
    selected_values: &[String],
    options: &[SelectOption<'_>],
    change_attr: Option<&str>,
    option_grid_class: Option<&str>,
    max_selected: Option<usize>,
) -> String {
    let selected = selected_values
        .iter()
        .map(|value| value.as_str())
        .collect::<std::collections::HashSet<_>>();
    let mut html = String::new();
    let change_attr = change_attr.unwrap_or("");
    let max_selected_attr = max_selected
        .filter(|value| *value > 0)
        .map(|value| format!(" max-selected=\"{value}\""))
        .unwrap_or_default();
    let bound_inputs_id = format!("{id}-bound-inputs");
    write!(html, "<div id=\"{}\" class=\"block\">", escape_html(id)).unwrap();
    write!(
        html,
        "<div id=\"{}\" data-role=\"bound-inputs-root\" hidden>{}</div>",
        escape_html(&bound_inputs_id),
        render_searchable_multiselect_bound_select_html(bind_key, selected_values, options),
    )
    .unwrap();
    write!(
        html,
        "<fishy-checkbox-group class=\"block\" bound-select-id=\"{}\"{}>",
        escape_html(&bound_inputs_id),
        max_selected_attr,
    )
    .unwrap();
    write!(
        html,
        "<div class=\"{}\" {}>",
        escape_html(option_grid_class.unwrap_or("grid gap-2 sm:grid-cols-2")),
        change_attr,
    )
    .unwrap();
    for option in options.iter() {
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
            "<label class=\"label cursor-pointer items-start justify-start gap-3 rounded-box border border-base-300 bg-base-100 px-3 py-2 text-sm\"><input type=\"checkbox\" data-checkbox-group-option class=\"checkbox checkbox-primary checkbox-sm mt-0.5 shrink-0\" value=\"{}\"{}{}>",
            escape_html(option.value),
            checked,
            category_key_attr,
        )
        .unwrap();
        if let Some(skill_content) = render_pet_skill_option_content_html(lang, *option) {
            write!(html, "{}</label>", skill_content).unwrap();
            continue;
        }
        let uses_item_presentation = option.icon.is_some();
        let grade_tone = escape_html(option.grade_tone);
        let tone_class = format!("fishy-item-grade-{grade_tone}");
        let badges = option
            .item
            .map(|item| render_item_effect_badges(lang, item))
            .unwrap_or_default();
        if uses_item_presentation {
            write!(
                html,
                "<span class=\"fishy-item-row min-w-0 flex-1 {}\">",
                tone_class,
            )
            .unwrap();
        } else {
            html.push_str("<span class=\"min-w-0 flex-1\">");
        }
        if let Some(icon) = option.icon {
            write!(
                html,
                "<span class=\"fishy-item-icon-frame is-md {tone_class}\"><img aria-hidden=\"true\" src=\"{}\" class=\"fishy-item-icon item-icon\" alt=\"{} icon\"/></span>",
                escape_html(&absolute_public_asset_url(cdn_base_url, icon)),
                escape_html(option.label)
            )
            .unwrap();
        }
        if uses_item_presentation {
            write!(
                html,
                "<span class=\"min-w-0 flex-1\"><span class=\"fishy-item-label block font-medium {tone_class}\">{}</span>{}</span></span></label>",
                escape_html(option.label),
                badges,
            )
            .unwrap();
        } else {
            write!(
                html,
                "<span class=\"min-w-0 flex-1\"><span class=\"block font-medium text-base-content\">{}</span>{}</span></span></label>",
                escape_html(option.label),
                badges,
            )
            .unwrap();
        }
    }
    html.push_str("</div></fishy-checkbox-group></div>");
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
            "<button type=\"button\" class=\"btn btn-soft btn-sm join-item\" data-on:click=\"$timespanAmount = {}; $timespanUnit = '{}'\">{}</button>",
            trim_float(preset.amount),
            escape_html(&preset.unit),
            escape_html(&preset.label)
        )
        .unwrap();
    }
    html.push_str("</div>");
    html
}

fn select_pet_option_entries_by_keys<'a>(
    keys: &[String],
    options: &'a [CalculatorPetOptionEntry],
    skill_chances: &BTreeMap<String, f32>,
) -> Vec<SelectOption<'a>> {
    keys.iter()
        .filter_map(|key| options.iter().find(|option| option.key == *key))
        .map(|option| SelectOption {
            value: option.key.as_str(),
            label: option.label.as_str(),
            icon: None,
            grade_tone: "unknown",
            pet_variant_talent: None,
            pet_variant_special: None,
            pet_skill: Some(option),
            pet_effective_talent_effects: None,
            pet_skill_learn_chance: skill_chances.get(&option.key).copied(),
            item: None,
            lifeskill_level: None,
            presentation: SelectOptionPresentation::Default,
            sort_priority: 1,
        })
        .collect()
}

fn normalized_pet_tier_value(value: &str) -> &str {
    match value.trim() {
        "1" | "2" | "3" | "4" | "5" => value.trim(),
        _ => "1",
    }
}

fn pet_tier_step_expression(slot: usize, delta: i32) -> String {
    let signal = format!("$pet{slot}.tier");
    format!(
        "{signal} = String(Math.min(5, Math.max(1, (Number({signal} || '1') || 1) + ({delta}))))"
    )
}

fn render_pet_tier_header_control(
    slot: usize,
    selected_tier: &str,
    lang: CalculatorLocale,
) -> String {
    let input_id = format!("calculator-pet{slot}-tier-value");
    let bind_key = format!("pet{slot}.tier");
    let tier_value = normalized_pet_tier_value(selected_tier);
    let tier_signal = format!("$pet{slot}.tier || '1'");
    let increment_expression = escape_html(&pet_tier_step_expression(slot, 1));
    let decrement_expression = escape_html(&pet_tier_step_expression(slot, -1));
    let increment_disabled = escape_html(&format!("Number({tier_signal}) >= 5"));
    let decrement_disabled = escape_html(&format!("Number({tier_signal}) <= 1"));
    let tier_label = calculator_route_text(lang, "calculator.server.field.tier");
    let increment_label = format!("{tier_label} +1");
    let decrement_label = format!("{tier_label} -1");

    format!(
        "<input id=\"{input_id}\" type=\"hidden\" data-bind=\"{bind_key}\" value=\"{tier_value}\">\
         <div class=\"flex shrink-0 flex-col items-center\" data-pet-tier-control data-pet-tier-stack>\
            <button type=\"button\" class=\"btn btn-ghost btn-xs btn-square\" data-on:click=\"{increment_expression}\" data-class:btn-disabled=\"{increment_disabled}\" data-attr:aria-disabled=\"({increment_disabled}).toString()\" aria-label=\"{increment_label}\">\
                <svg class=\"fishy-icon size-5\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><use width=\"100%\" height=\"100%\" href=\"{icon_sprite_url}#fishy-up-small-fill\"></use></svg>\
            </button>\
            <kbd class=\"kbd kbd-xl h-12 min-h-12 w-12 text-2xl font-bold\" aria-live=\"polite\" data-text=\"{tier_signal}\">{tier_value}</kbd>\
            <button type=\"button\" class=\"btn btn-ghost btn-xs btn-square\" data-on:click=\"{decrement_expression}\" data-class:btn-disabled=\"{decrement_disabled}\" data-attr:aria-disabled=\"({decrement_disabled}).toString()\" aria-label=\"{decrement_label}\">\
                <svg class=\"fishy-icon size-5\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><use width=\"100%\" height=\"100%\" href=\"{icon_sprite_url}#fishy-down-small-fill\"></use></svg>\
            </button>\
         </div>",
        input_id = escape_html(&input_id),
        bind_key = escape_html(&bind_key),
        tier_value = escape_html(tier_value),
        tier_signal = escape_html(&tier_signal),
        increment_expression = increment_expression,
        increment_disabled = increment_disabled,
        increment_label = escape_html(&increment_label),
        icon_sprite_url = CALCULATOR_ICON_SPRITE_URL,
        decrement_expression = decrement_expression,
        decrement_disabled = decrement_disabled,
        decrement_label = escape_html(&decrement_label),
    )
}

fn render_pet_cards(
    cdn_base_url: &str,
    api_lang: &DataLang,
    lang: CalculatorLocale,
    catalog: &CalculatorPetCatalog,
    signals: &CalculatorSignals,
) -> String {
    let total_slots = catalog.slots.max(1);

    let mut html = String::new();
    html.push_str("<div id=\"pets\" class=\"fishy-calculator-pets\">");
    for slot in 1..=total_slots {
        let pet = match slot {
            1 => &signals.pet1,
            2 => &signals.pet2,
            3 => &signals.pet3,
            4 => &signals.pet4,
            _ => &signals.pet5,
        };
        let pet_options =
            select_options_from_pet_entries_for_tier(catalog, &pet.tier, Some(&pet.pet), Some(pet));
        let selected_pet = catalog.pets.iter().find(|entry| entry.key == pet.pet);
        let selected_tier_entry =
            selected_pet.and_then(|entry| entry.tiers.iter().find(|tier| tier.key == pet.tier));
        let selected_special = pet_option_by_key(&catalog.specials, &pet.special);
        let selected_talent = pet_option_by_key(&catalog.talents, &pet.talent);
        let selected_special_content = render_pet_fixed_special_content(lang, selected_special);
        let selected_talent_content =
            render_pet_fixed_talent_content(lang, pet, catalog, selected_talent);
        let special_variant_options = selected_pet
            .map(|entry| {
                select_pet_fixed_option_entries_by_keys(
                    &pet_same_tier_variant_option_keys(
                        catalog,
                        entry,
                        &pet.tier,
                        PetFixedOptionKind::Special,
                    ),
                    &catalog.specials,
                    PetFixedOptionKind::Special,
                    None,
                    None,
                )
            })
            .unwrap_or_default();
        let talent_variant_options = selected_pet
            .map(|entry| {
                select_pet_fixed_option_entries_by_keys(
                    &pet_same_tier_variant_option_keys(
                        catalog,
                        entry,
                        &pet.tier,
                        PetFixedOptionKind::Talent,
                    ),
                    &catalog.talents,
                    PetFixedOptionKind::Talent,
                    Some(pet),
                    Some(catalog),
                )
            })
            .unwrap_or_default();
        let skill_limit = selected_tier_entry
            .map(|tier| pet_skill_limit_for_tier_key(&tier.key))
            .unwrap_or(1);
        let skill_options = selected_tier_entry
            .map(|tier| {
                select_pet_option_entries_by_keys(
                    &tier.skills,
                    &catalog.skills,
                    &tier.skill_chances,
                )
            })
            .unwrap_or_default();
        let bind_prefix = format!("pet{slot}");
        let pack_leader_input_id = format!("calculator-pet{slot}-pack-leader-value");
        let pack_leader_checked = if pet.pack_leader { " checked" } else { "" };
        let show_pack_leader = selected_tier_entry.is_some_and(|tier| tier.key.trim() == "5");
        let tier_header_html = render_pet_tier_header_control(slot, &pet.tier, lang);
        write!(
            html,
            "<section class=\"card card-border bg-base-100\" data-pet-slot=\"{}\"><div class=\"card-body gap-4\">",
            slot
        )
        .unwrap();
        html.push_str("<div class=\"fishy-calculator-pet-card-layout\">");
        html.push_str("<div class=\"fishy-calculator-pet-tier-column\">");
        html.push_str(&tier_header_html);
        if show_pack_leader {
            write!(
                html,
                "<div class=\"fishy-calculator-pet-pack-leader\" style=\"display:flex;flex-direction:column;align-items:center;gap:0.55rem;margin-top:1.25rem;text-align:center\"><input id=\"{}\" type=\"checkbox\" class=\"checkbox checkbox-primary checkbox-sm\" data-bind=\"{}.packLeader\" data-on:change=\"window.__fishystuffCalculator.applyPackLeaderChange(el, {})\" data-pet-pack-leader data-pet-pack-leader-slot=\"{}\"{}><label class=\"fishy-calculator-pet-pack-leader__text\" for=\"{}\">{}</label></div>",
                escape_html(&pack_leader_input_id),
                escape_html(&bind_prefix),
                slot,
                slot,
                pack_leader_checked,
                escape_html(&pack_leader_input_id),
                escape_html(&calculator_pet_pack_leader_label(lang)),
            )
            .unwrap();
        } else {
            html.push_str(
                "<div class=\"fishy-calculator-pet-pack-leader fishy-calculator-pet-pack-leader--placeholder\" aria-hidden=\"true\"></div>",
            );
        }
        html.push_str("</div>");
        html.push_str(
            "<fieldset class=\"fieldset fishy-calculator-pet-select-field min-w-0 max-w-full shrink\">",
        );
        html.push_str(&render_pet_select_control(
            cdn_base_url,
            api_lang,
            lang,
            &format!("calculator-pet{slot}-pet-picker"),
            &format!("calculator-pet{slot}-pet-value"),
            &format!("{}.pet", bind_prefix),
            &pet.pet,
            &pet.tier,
            pet.pack_leader,
            &pet_options,
            &calculator_route_text(lang, "calculator.server.search.pets"),
        ));
        html.push_str("</fieldset>");
        html.push_str("<div class=\"fishy-calculator-pet-controls\">");
        html.push_str("<div class=\"fishy-calculator-pet-fixed-options\">");
        html.push_str(&format!(
            "<fieldset class=\"fieldset\"><legend class=\"fieldset-legend\">{}</legend>",
            escape_html(&calculator_route_text(
                lang,
                "calculator.server.field.special",
            ))
        ));
        if special_variant_options.len() > 1 {
            html.push_str(&render_pet_variant_fixed_option_select_control(
                lang,
                &format!("calculator-pet{slot}-special-picker"),
                &format!("calculator-pet{slot}-special-value"),
                &format!("{}.special", bind_prefix),
                &pet.special,
                &special_variant_options,
                &selected_special_content,
                &calculator_route_text(lang, "calculator.server.search.pet_specials"),
            ));
        } else {
            html.push_str(&render_pet_fixed_option_control(
                &format!("calculator-pet{slot}-special-value"),
                &format!("{}.special", bind_prefix),
                &pet.special,
                &selected_special_content,
            ));
        }
        html.push_str("</fieldset>");
        html.push_str(&format!(
            "<fieldset class=\"fieldset\"><legend class=\"fieldset-legend\">{}</legend>",
            escape_html(&calculator_route_text(
                lang,
                "calculator.server.field.talent",
            ))
        ));
        if talent_variant_options.len() > 1 {
            html.push_str(&render_pet_variant_fixed_option_select_control(
                lang,
                &format!("calculator-pet{slot}-talent-picker"),
                &format!("calculator-pet{slot}-talent-value"),
                &format!("{}.talent", bind_prefix),
                &pet.talent,
                &talent_variant_options,
                &selected_talent_content,
                &calculator_route_text(lang, "calculator.server.search.pet_talents"),
            ));
        } else {
            html.push_str(&render_pet_fixed_option_control(
                &format!("calculator-pet{slot}-talent-value"),
                &format!("{}.talent", bind_prefix),
                &pet.talent,
                &selected_talent_content,
            ));
        }
        html.push_str("</fieldset>");
        html.push_str("</div>");
        html.push_str(&format!(
            "<fieldset class=\"fieldset gap-2\"><legend class=\"fieldset-legend\">{}</legend>",
            escape_html(&calculator_route_text(
                lang,
                "calculator.server.field.skills",
            ))
        ));
        html.push_str(&render_pet_skill_selects(
            lang,
            slot,
            skill_limit,
            &pet.skills,
            &skill_options,
        ));
        html.push_str("</fieldset></div></div></div></section>");
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

fn escaped_js_string_literal(value: &str) -> String {
    escape_html(&serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string()))
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
        CalculatorPetEntry, CalculatorPetOptionEntry, CalculatorPetSignals, CalculatorPetTierEntry,
        CalculatorPriceOverrideSignals, CalculatorSignals, CalculatorZoneGroupRateEntry,
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
    use serde_json::{json, Value};

    use crate::config::{AppConfig, TelemetryConfig, ZoneStatusConfig};
    use crate::error::AppResult;
    use crate::state::{AppState, RequestId};
    use crate::store::{
        CalculatorZoneLootEntry, CalculatorZoneLootEvidence, CalculatorZoneLootRateContribution,
        DataLang, Store,
    };

    use super::{
        apply_calculator_condition_context_to_loot_entries, auto_target_fish_pmf_tail_count,
        base_price_for_species, buff_category_label, build_pet_value_aliases,
        build_zone_loot_summary_condition_options, default_reset_signals_patch_map,
        derive_fish_group_chart, derive_loot_chart, derive_target_fish_summary,
        derive_zone_loot_summary_response,
        derive_zone_loot_summary_response_with_condition_options, discard_grade_enabled,
        filtered_loot_flow_rows, get_calculator_datastar_init,
        get_calculator_datastar_option_search, get_calculator_datastar_zone_search,
        init_signals_patch_map, load_calculator_runtime_data, loot_species_evidence_text,
        loot_species_presence_source_kind, loot_species_presence_text,
        loot_species_presence_tooltip, mastery_prize_rate_for_bracket, normalize_lookup_value,
        normalize_named_array, normalize_pack_leader_selection, normalize_pet, normalize_signals,
        parse_calculator_signals_value, pet_drr, pet_skill_limit_for_tier_key,
        pmf_bucket_contains_target, poisson_probability_at_least, post_calculator_datastar_eval,
        render_pet_cards, render_pet_dropdown_option_content_html,
        render_pet_dropdown_selected_content_html, render_pet_effective_talent_badges,
        render_pet_talent_badges, render_searchable_select_results,
        render_select_option_search_text, render_target_fish_panel,
        select_options_from_pet_entries_for_tier, trade_sale_multiplier_for_species,
        CalculatorData, CalculatorDatastarQuery, CalculatorLocale, CalculatorQuery,
        CalculatorSearchableOptionQuery, CalculatorZoneSearchQuery, FishGroupChart,
        FishGroupChartRow, LootChartRow, LootSpeciesRow, SelectOption, SelectOptionPresentation,
        TargetFishSummary,
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
            _lang: DataLang,
            _ref_id: Option<String>,
        ) -> AppResult<FishListResponse> {
            panic!("unused in test")
        }

        async fn calculator_catalog(
            &self,
            _lang: DataLang,
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
                pets: CalculatorPetCatalog {
                    slots: 5,
                    pets: vec![CalculatorPetEntry {
                        key: "pet:3:1:pet_hawk_0014".to_string(),
                        label: "Hawk".to_string(),
                        skin_key: Some("2001".to_string()),
                        image_url: Some("/images/pets/pet_hawk_0014.webp".to_string()),
                        alias_keys: Vec::new(),
                        lineage_keys: Vec::new(),
                        variant_group_keys: Vec::new(),
                        tiers: vec![CalculatorPetTierEntry {
                            key: "5".to_string(),
                            label: "Tier 5".to_string(),
                            specials: vec!["auto_fishing_time_reduction".to_string()],
                            talents: vec!["durability_reduction_resistance".to_string()],
                            skills: vec!["fishing_exp".to_string()],
                            skill_chances: Default::default(),
                        }],
                    }],
                    tiers: (1..=5)
                        .map(|tier| CalculatorOptionEntry {
                            key: tier.to_string(),
                            label: format!("Tier {tier}"),
                        })
                        .collect(),
                    specials: vec![CalculatorPetOptionEntry {
                        key: "auto_fishing_time_reduction".to_string(),
                        label: "Auto-Fishing Time Reduction".to_string(),
                        auto_fishing_time_reduction: Some(0.1),
                        ..CalculatorPetOptionEntry::default()
                    }],
                    talents: vec![CalculatorPetOptionEntry {
                        key: "durability_reduction_resistance".to_string(),
                        label: "Durability Reduction Resistance".to_string(),
                        durability_reduction_resistance: Some(0.05),
                        ..CalculatorPetOptionEntry::default()
                    }],
                    skills: vec![CalculatorPetOptionEntry {
                        key: "fishing_exp".to_string(),
                        label: "Fishing EXP".to_string(),
                        fishing_exp: Some(0.07),
                        ..CalculatorPetOptionEntry::default()
                    }],
                },
                defaults: CalculatorSignals {
                    level: 5,
                    lifeskill_level: "100".to_string(),
                    mastery: 2500.0,
                    trade_level: "73".to_string(),
                    zone: "240,74,74".to_string(),
                    resources: 0.0,
                    fishing_mode: "rod".to_string(),
                    rod: "item:16162".to_string(),
                    float: String::new(),
                    chair: "item:705539".to_string(),
                    lightstone_set: "lightstone-set:30".to_string(),
                    backpack: "item:830150".to_string(),
                    target_fish: String::new(),
                    target_fish_amount: 1.0,
                    target_fish_pmf_count: 0.0,
                    outfit: vec![
                        "effect:8-piece-outfit-set-effect".to_string(),
                        "effect:mainhand-weapon-outfit".to_string(),
                        "effect:awakening-weapon-outfit".to_string(),
                        "item:14330".to_string(),
                    ],
                    food: vec!["item:9359".to_string()],
                    buff: vec!["".to_string(), "item:721092".to_string()],
                    pet1: CalculatorPetSignals {
                        pet: String::new(),
                        tier: "5".to_string(),
                        pack_leader: false,
                        special: "auto_fishing_time_reduction".to_string(),
                        talent: "durability_reduction_resistance".to_string(),
                        skills: vec!["fishing_exp".to_string()],
                    },
                    pet2: CalculatorPetSignals {
                        pet: String::new(),
                        tier: "4".to_string(),
                        pack_leader: false,
                        special: String::new(),
                        talent: "durability_reduction_resistance".to_string(),
                        skills: vec!["fishing_exp".to_string()],
                    },
                    pet3: CalculatorPetSignals {
                        pet: String::new(),
                        tier: "4".to_string(),
                        pack_leader: false,
                        special: String::new(),
                        talent: "durability_reduction_resistance".to_string(),
                        skills: vec!["fishing_exp".to_string()],
                    },
                    pet4: CalculatorPetSignals {
                        pet: String::new(),
                        tier: "4".to_string(),
                        pack_leader: false,
                        special: String::new(),
                        talent: "durability_reduction_resistance".to_string(),
                        skills: vec!["fishing_exp".to_string()],
                    },
                    pet5: CalculatorPetSignals {
                        pet: String::new(),
                        tier: "4".to_string(),
                        pack_leader: false,
                        special: String::new(),
                        talent: "durability_reduction_resistance".to_string(),
                        skills: vec!["fishing_exp".to_string()],
                    },
                    trade_distance_bonus: 134.15,
                    trade_price_curve: 120.0,
                    price_overrides: Default::default(),
                    overlay: Default::default(),
                    catch_time_active: 17.5,
                    catch_time_afk: 6.5,
                    timespan_amount: 8.0,
                    timespan_unit: "hours".to_string(),
                    apply_trade_modifiers: true,
                    show_silver_amounts: false,
                    show_normalized_select_rates: true,
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
            defaults: MetaDefaults::default(),
            status_cfg: ZoneStatusConfig::default(),
            cache_zone_stats_max: 4,
            cache_effort_max: 4,
            cache_log: false,
            request_timeout_secs: 5,
            telemetry: TelemetryConfig::default(),
        };
        AppState::for_tests(config, Arc::new(MockStore))
    }

    #[tokio::test]
    async fn init_returns_html_fragment_with_initial_signals() {
        let response = get_calculator_datastar_init(
            State(test_state()),
            Ok(Query(CalculatorDatastarQuery {
                lang: Some("en".to_string()),
                locale: Some("en-US".to_string()),
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
        assert!(text.contains("\"mastery\":2500.0"));
        assert!(text.contains("\"timespanAmount\":8.0"));
        assert!(text.contains("\"active\":false"));
        assert!(text.contains("\"fishingMode\":\"rod\""));
        assert!(text.contains("\"_resources\":0.0"));
        assert!(text.contains("\"unpinned_insert_index\":[0,0]"));
        assert!(text.contains("\"chair\":\"item:705539\""));
        assert!(text.contains("\"zone_name\":\"Velia Beach"));
        assert!(text.contains("event:datastar-patch-elements"));
        assert!(text.contains("data:selector #calculator-app"));
        assert!(text.contains("<div id=\"calculator-app\""));
        assert!(text.contains("placeholder=\"Search zones\""));
        assert!(text.contains("<fishy-searchable-dropdown"));
        assert!(text.contains("input-id=\"calculator-zone-value\""));
        assert!(text.contains("id=\"calculator-zone-picker\""));
        assert!(text
            .contains("panel-mode=\"detached\" panel-min-width=\"panel\" panel-width=\"32rem\""));
        assert!(text.contains(
            "search-url=\"/api/v1/calculator/datastar/zone-search?lang=en&amp;locale=en-US\""
        ));
        assert!(text.contains(
            "search-url=\"/api/v1/calculator/datastar/option-search?lang=en&amp;locale=en-US&amp;kind=rod"
        ));
        assert!(text.contains("search-url-root=\"api\""));
        assert!(text.contains("data-role=\"selected-content\""));
        assert!(text.contains("kind=rod"));
        assert!(text.contains("calculator-rod-picker"));
        assert!(text.contains("calculator-pet1-tier-value"));
        assert!(text.contains("data-bind=\"pet1.tier\""));
        assert!(text.contains("data-pet-tier-control"));
        assert!(text.contains("data-pet-tier-stack"));
        assert!(text.contains("flex shrink-0 flex-col items-center"));
        assert!(text.contains("btn btn-ghost btn-xs btn-square"));
        assert!(text.contains("kbd kbd-xl h-12 min-h-12 w-12 text-2xl font-bold"));
        assert!(text.contains("#fishy-up-small-fill"));
        assert!(text.contains("#fishy-down-small-fill"));
        assert!(text.contains("http://127.0.0.1:4040/images/pets/pet_hawk_0014.webp"));
        assert!(!text.contains("data-pet-pack-leader"));
        assert!(text.contains("<div id=\"pets\" class=\"fishy-calculator-pets\">"));
        assert!(text.contains("fishy-calculator-pet-card-layout"));
        assert!(text.contains("fishy-calculator-pet-tier-column"));
        assert!(text.contains("fishy-calculator-pet-pack-leader--placeholder"));
        assert!(!text.contains("id=\"calculator-pet1-pack-leader-value\""));
        assert!(text.contains("id=\"calculator-pet1-talent-value-content\""));
        assert!(text.contains("fishy-calculator-pet-select-field"));
        assert!(text.contains("fishy-calculator-pet-fixed-options"));
        assert!(text.contains("fishy-calculator-pet-skills-grid"));
        let tier_index = text.find("calculator-pet1-tier-value").unwrap();
        let pet_selector_index = text.find("calculator-pet1-pet-value").unwrap();
        let special_index = text.find("calculator-pet1-special-value").unwrap();
        let talent_index = text.find("calculator-pet1-talent-value").unwrap();
        let skills_index = text.find("id=\"pet1_skills\"").unwrap();
        assert!(tier_index < pet_selector_index);
        assert!(pet_selector_index < special_index);
        assert!(special_index < talent_index);
        assert!(talent_index < skills_index);
        assert!(text.contains("fishy-calculator-pet-option--selected"));
        assert!(text.contains("fishy-item-grade-red"));
        assert!(text.contains("fishy-calculator-pet-option__badges"));
        assert!(text.contains("+5% Item DRR"));
        assert!(!text.contains("calculator-pet1-tier-picker"));
        assert!(text.contains("calculator-pet1-special-value"));
        assert!(text.contains("data-bind=\"pet1.special\""));
        assert!(!text.contains("calculator-pet1-special-picker"));
        assert!(text.contains("calculator-pet1-talent-value"));
        assert!(text.contains("data-bind=\"pet1.talent\""));
        assert!(!text.contains("calculator-pet1-talent-picker"));
        assert!(text.contains("data-pet-fixed-option"));
        assert!(text.contains("<fishy-searchable-multiselect"));
        assert!(text.contains("calculator-food-picker"));
        assert!(text.contains("calculator-buff-picker"));
        assert!(text.contains("data-bind=\"_food_slots\""));
        assert!(text.contains("data-bind=\"_buff_slots\""));
        assert!(text.contains("bound-select-id=\"calculator-food-picker-bound-inputs\""));
        assert!(text.contains("bound-select-id=\"calculator-buff-picker-bound-inputs\""));
        assert!(text.contains("data-bind=\"_outfit_slots\""));
        assert!(text.contains("bound-select-id=\"outfits-bound-inputs\""));
        assert!(text.contains("$_calculator_actions.copyUrlToken = (($_calculator_actions && $_calculator_actions.copyUrlToken) || 0) + 1"));
        assert!(text.contains("$_calculator_actions.copyShareToken = (($_calculator_actions && $_calculator_actions.copyShareToken) || 0) + 1"));
        assert!(text.contains("$_calculator_actions.resetLayoutToken = (($_calculator_actions && $_calculator_actions.resetLayoutToken) || 0) + 1"));
        assert!(text.contains("$_calculator_actions.clearToken = (($_calculator_actions && $_calculator_actions.clearToken) || 0) + 1"));
        assert!(text.contains("<fishy-preset-manager"));
        assert!(text.contains("data-preset-collection=\"calculator-layouts\""));
        assert!(text.contains(
            "window.__fishystuffCalculator.blurActiveElement(); window.__fishystuffCalculator.togglePinnedSectionInPlace($_calculator_ui, 'overview')"
        ));
        assert!(text.contains("<fishy-calculator-section-stack"));
        assert!(text.contains("data-calculator-unpinned-slot-drag"));
        assert!(text.contains("data-calculator-unpinned-slot-projection"));
        assert!(text.contains("data-calculator-pin-dropzone"));
        assert!(text.contains("data-calculator-section-drag"));
        assert!(text.contains("fishy-calculator-panel-pin-slot"));
        assert!(text.contains("fishy-calculator-panel-control--pin"));
        assert!(text.contains("fishy-calculator-unpinned-slot-handle"));
        assert!(text.contains("fishy-calculator-tab--pinned"));
        assert!(text.contains("fishy-calculator-tab-label"));
        assert!(text.contains("fishy-calculator-tab-main"));
        assert!(text.contains("fishy-calculator-tab-pin"));
        assert!(text.contains(
            "data-class:fishy-calculator-tab--pinned=\"window.__fishystuffCalculator.isPinnedSection($_calculator_ui.pinned_sections, 'overview')\""
        ));
        assert!(text.contains("calculator.server.action.drag_section_generic"));
        assert!(text.contains("calculator.server.action.drag_unpinned_slot"));
        assert!(text.contains("calculator.server.action.unpinned_dropzone_title"));
        assert!(text.contains("calculator.server.action.unpinned_dropzone_detail"));
        assert!(text.contains("#fishy-pin"));
        assert!(text.contains("#fishy-arrow-to-down-fill"));
        assert!(text.contains("#fishy-drag-handle"));
        assert!(!text.contains("window.__fishystuffCalculator.persist("));
        assert!(!text.contains("window.__fishystuffCalculator.persistSignalPatchFilter()"));
        assert!(!text.contains("window.__fishystuffCalculator.presetUrl("));
        assert!(!text.contains("window.__fishystuffCalculator.shareText("));
        assert!(!text.contains("window.__fishystuffCalculator.clear("));
        assert!(!text.contains("window.__fishystuffCalculator.movePinnedSection($_calculator_ui.pinned_sections, 'overview', 1)"));
        assert!(text.contains(
            "data-computed:outfit=\"Array.isArray($_outfit_slots) ? $_outfit_slots : []\""
        ));
        assert!(
            text.contains("data-computed:food=\"Array.isArray($_food_slots) ? $_food_slots : []\"")
        );
        assert!(
            text.contains("data-computed:buff=\"Array.isArray($_buff_slots) ? $_buff_slots : []\"")
        );
        assert!(text.contains("Search foods by name or effect"));
        assert!(text.contains("class=\"grid grid-cols-2 items-start gap-3\""));
        assert_eq!(text.matches("data-bind=\"mastery\"").count(), 2);
        assert!(text.contains(
            "type=\"number\" min=\"0\" max=\"3000\" step=\"50\" class=\"input input-sm w-full\" data-bind=\"mastery\""
        ));
        assert!(text.contains(
            "type=\"range\" min=\"0\" max=\"3000\" step=\"50\" class=\"range-xs range-secondary w-full\" data-bind=\"mastery\""
        ));
        assert!(text.contains("Raw Prize Catch Rate"));
        assert!(text.contains("data-text=\"$_calc.raw_prize_mastery_text\""));
        assert!(text.contains("data-text=\"$_calc.raw_prize_rate_text\""));
        assert!(text.contains(
            "data-attr:data-fishy-stat-breakdown=\"$_live.stat_breakdowns.total_time || ''\""
        ));
        assert!(text.contains(
            "data-attr:data-fishy-stat-breakdown=\"$_live.stat_breakdowns.raw_prize_rate || ''\""
        ));
        assert!(text.contains(
            "data-attr:data-fishy-stat-breakdown=\"$_live.stat_breakdowns.loot_total_profit || ''\""
        ));
        assert!(text.contains("Target Fish"));
        assert!(text.contains("Loot Flow"));
        assert!(text.contains("Expected Catches / Hour"));
        assert!(text.contains("$_calculator_ui.top_level_tab === 'mode'"));
        assert!(text.contains("$_calculator_ui.top_level_tab === 'zone'"));
        assert!(text.contains("$_calculator_ui.top_level_tab === 'bite_time'"));
        assert!(text.contains("$_calculator_ui.top_level_tab === 'catch_time'"));
        assert!(text.contains("$_calculator_ui.top_level_tab === 'session'"));
        assert!(text.contains("$_calculator_ui.top_level_tab === 'trade'"));
        assert!(text.contains("$_calculator_ui.top_level_tab === 'food'"));
        assert!(text.contains("$_calculator_ui.top_level_tab === 'buffs'"));
        assert!(text.contains("data-calculator-section-id=\"mode\""));
        assert!(text.contains("data-calculator-section-id=\"zone\""));
        assert!(text.contains("data-calculator-section-id=\"bite_time\""));
        assert!(text.contains("data-calculator-section-id=\"catch_time\""));
        assert!(text.contains("data-calculator-section-id=\"session\""));
        assert!(text.contains("calculator-loot-window"));
        assert!(text.contains("calculator-trade-window"));
        assert!(text.contains("data-calculator-section-id=\"trade\""));
        assert!(text.contains("data-calculator-section-id=\"food\""));
        assert!(text.contains("data-calculator-section-id=\"buffs\""));
        assert!(text.contains("#fishy-fish-fill"));
        assert!(text.contains("#fishy-information-fill"));
        assert!(text.contains("#fishy-fullscreen-fill"));
        assert!(text.contains("#fishy-stopwatch-2-fill"));
        assert!(text.contains("#fishy-stopwatch-fill"));
        assert!(text.contains("#fishy-time-fill"));
        assert!(text.contains("#fishy-chart-pie-2-fill"));
        assert!(text.contains("#fishy-trending-up-fill"));
        assert!(text.contains("#fishy-wheel-fill"));
        assert!(text.contains("#fishy-dinner-fill"));
        assert!(text.contains("#fishy-arrows-up-fill"));
        assert!(text.contains("#fishy-paw-fill"));
        assert!(text.contains("#fishy-bug-fill"));
        assert!(text.contains("<fishy-distribution-chart"));
        assert!(text.contains("signal-path=\"_calc.fish_group_distribution_chart\""));
        assert!(text.contains("No source-backed loot rows are available for this zone yet."));
        assert!(text.contains("Normalize rates"));
        assert!(text.contains("data-category-key=\"buff-category:1\""));
        assert!(text.contains("Meal"));
        assert!(text.contains("value=\"effect:8-piece-outfit-set-effect\" checked"));
        assert!(text.contains("value=\"effect:mainhand-weapon-outfit\" checked"));
        assert!(text.contains("value=\"effect:awakening-weapon-outfit\" checked"));
        assert!(text.contains("value=\"item:14330\" checked"));
        assert!(text.contains("src=\"/img/calculator/fishing-mode-rod.png\""));
        assert!(text.contains("src=\"/img/calculator/fishing-mode-harpoon.png\""));
        assert!(text.contains("src=\"http://127.0.0.1:4040/images/items/00016162.webp\""));
    }

    #[tokio::test]
    async fn init_returns_korean_html_fragment_when_data_lang_is_kr() {
        let response = get_calculator_datastar_init(
            State(test_state()),
            Ok(Query(CalculatorDatastarQuery {
                lang: Some("kr".to_string()),
                locale: Some("ko-KR".to_string()),
                r#ref: None,
                datastar: Some("{}".to_string()),
            })),
            Extension(RequestId("req-test".to_string())),
        )
        .await
        .unwrap()
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body()).await.unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains(
            "search-url=\"/api/v1/calculator/datastar/zone-search?lang=kr&amp;locale=ko-KR\""
        ));
        assert!(text.contains("placeholder=\"지역 검색\""));
        assert!(text.contains("오버레이 제안"));
        assert!(text.contains("시간당 예상 횟수"));
        assert!(text.contains("선택됨"));
        assert!(text.contains("<fishy-calculator-overlay-panel>"));
    }

    #[tokio::test]
    async fn eval_normalizes_legacy_values_and_returns_calc_signals_sse() {
        let response = post_calculator_datastar_eval(
            State(test_state()),
            Ok(Query(CalculatorQuery {
                lang: Some("en".to_string()),
                locale: Some("en-US".to_string()),
                r#ref: None,
                pet_cards: None,
                target_fish_select: None,
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
        assert!(text.contains("data:selector #pets"));
        assert!(!text.contains("data:selector #calculator-fish-group-chart"));
        assert!(!text.contains("data:selector #calculator-fish-group-silver-chart"));
        assert!(!text.contains("data:selector #calculator-target-fish-panel"));
        assert!(!text.contains("data:selector #calculator-loot-chart"));
        assert!(text.contains("\"zone_name\":\"Velia Beach\""));
        assert!(text.contains("\"raw_prize_rate_text\":\""));
        assert!(text.contains("\"raw_prize_mastery_text\":\""));
        assert!(!text.contains("\"zone\":\"240,74,74\""));
        assert!(!text.contains("\"rod\":\"item:16162\""));
        assert!(!text.contains("\"_resources\":0.0"));
    }

    #[tokio::test]
    async fn eval_can_patch_pet_talents_without_replacing_pet_cards() {
        let response = post_calculator_datastar_eval(
            State(test_state()),
            Ok(Query(CalculatorQuery {
                lang: Some("en".to_string()),
                locale: Some("en-US".to_string()),
                r#ref: None,
                pet_cards: Some(false),
                target_fish_select: None,
            })),
            Extension(RequestId("req-test".to_string())),
            Bytes::from_static(
                br#"{"zone":"Velia Beach","pet1":{"pet":"pet:3:1:pet_hawk_0014","tier":"5","packLeader":true,"talent":"durability_reduction_resistance"}}"#,
            ),
        )
        .await
        .unwrap()
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body()).await.unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("event:datastar-patch-signals"));
        assert!(text.contains("data:selector #calculator-pet1-talent-value-content"));
        assert!(text.contains("+6% Item DRR"));
        assert!(!text.contains("data:selector #pets"));
        assert!(!text.contains("data:selector #calculator-fish-group-chart"));
        assert!(!text.contains("data:selector #calculator-target-fish-panel"));
        assert!(!text.contains("data:selector #calculator-loot-chart"));
    }

    #[tokio::test]
    async fn eval_can_patch_target_fish_control_without_replacing_distribution_panels() {
        let response = post_calculator_datastar_eval(
            State(test_state()),
            Ok(Query(CalculatorQuery {
                lang: Some("en".to_string()),
                locale: Some("en-US".to_string()),
                r#ref: None,
                pet_cards: Some(false),
                target_fish_select: Some(true),
            })),
            Extension(RequestId("req-test".to_string())),
            Bytes::from_static(br#"{"zone":"Velia Beach"}"#),
        )
        .await
        .unwrap()
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body()).await.unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("event:datastar-patch-signals"));
        assert!(text.contains("data:selector #calculator-target-fish-control"));
        assert!(!text.contains("data:selector #calculator-target-fish-panel"));
        assert!(!text.contains("data:selector #calculator-fish-group-chart"));
        assert!(!text.contains("data:selector #calculator-fish-group-silver-chart"));
        assert!(!text.contains("data:selector #calculator-loot-chart"));
    }

    #[test]
    fn render_pet_cards_shows_pack_leader_only_for_tier_five_selected_pets() {
        let catalog = CalculatorPetCatalog {
            slots: 2,
            pets: vec![CalculatorPetEntry {
                key: "pet:test".to_string(),
                label: "Test Pet".to_string(),
                tiers: vec![
                    CalculatorPetTierEntry {
                        key: "4".to_string(),
                        label: "Tier 4".to_string(),
                        ..CalculatorPetTierEntry::default()
                    },
                    CalculatorPetTierEntry {
                        key: "5".to_string(),
                        label: "Tier 5".to_string(),
                        ..CalculatorPetTierEntry::default()
                    },
                ],
                ..CalculatorPetEntry::default()
            }],
            tiers: vec![
                CalculatorOptionEntry {
                    key: "4".to_string(),
                    label: "Tier 4".to_string(),
                },
                CalculatorOptionEntry {
                    key: "5".to_string(),
                    label: "Tier 5".to_string(),
                },
            ],
            ..CalculatorPetCatalog::default()
        };
        let signals = CalculatorSignals {
            pet1: CalculatorPetSignals {
                pet: "pet:test".to_string(),
                tier: "4".to_string(),
                ..CalculatorPetSignals::default()
            },
            pet2: CalculatorPetSignals {
                pet: "pet:test".to_string(),
                tier: "5".to_string(),
                ..CalculatorPetSignals::default()
            },
            ..CalculatorSignals::default()
        };

        let html = render_pet_cards(
            "",
            &DataLang::En,
            CalculatorLocale::EnUs,
            &catalog,
            &signals,
        );

        assert!(!html.contains("calculator-pet1-pack-leader-value"));
        assert!(html.contains("fishy-calculator-pet-pack-leader--placeholder"));
        assert!(html.contains("calculator-pet2-pack-leader-value"));
        assert!(!html.contains("data-pet-pack-leader-slot=\"1\""));
        assert!(html.contains("data-pet-pack-leader-slot=\"2\""));
        assert!(html.contains("window.__fishystuffCalculator.applyPackLeaderChange(el, 2)"));
    }

    #[test]
    fn render_pet_cards_renders_searchable_skill_slots_and_shows_learn_chances() {
        let catalog = CalculatorPetCatalog {
            slots: 1,
            pets: vec![CalculatorPetEntry {
                key: "pet:test".to_string(),
                label: "Test Pet".to_string(),
                tiers: vec![CalculatorPetTierEntry {
                    key: "4".to_string(),
                    label: "Tier 4".to_string(),
                    skills: vec![
                        "skill_a".to_string(),
                        "skill_b".to_string(),
                        "skill_c".to_string(),
                    ],
                    skill_chances: [
                        ("skill_a".to_string(), 0.15_f32),
                        ("skill_b".to_string(), 0.03_f32),
                        ("skill_c".to_string(), 0.01_f32),
                    ]
                    .into_iter()
                    .collect(),
                    ..CalculatorPetTierEntry::default()
                }],
                ..CalculatorPetEntry::default()
            }],
            skills: vec![
                CalculatorPetOptionEntry {
                    key: "skill_a".to_string(),
                    label: "Fishing EXP +7%".to_string(),
                    fishing_exp: Some(0.07),
                    ..CalculatorPetOptionEntry::default()
                },
                CalculatorPetOptionEntry {
                    key: "skill_b".to_string(),
                    label: "Luck +1".to_string(),
                    ..CalculatorPetOptionEntry::default()
                },
                CalculatorPetOptionEntry {
                    key: "skill_c".to_string(),
                    label: "Fishing EXP +7%".to_string(),
                    ..CalculatorPetOptionEntry::default()
                },
            ],
            ..CalculatorPetCatalog::default()
        };
        let signals = CalculatorSignals {
            pet1: CalculatorPetSignals {
                pet: "pet:test".to_string(),
                tier: "4".to_string(),
                skills: vec!["skill_a".to_string(), "skill_b".to_string()],
                ..CalculatorPetSignals::default()
            },
            ..CalculatorSignals::default()
        };

        let html = render_pet_cards(
            "",
            &DataLang::En,
            CalculatorLocale::EnUs,
            &catalog,
            &signals,
        );

        assert!(html.contains("calculator-pet1-skill-slot1-picker"));
        assert!(html.contains("calculator-pet1-skill-slot2-picker"));
        assert!(html.contains("calculator-pet1-skill-slot3-picker"));
        assert!(html.contains("data-bind=\"_pet1_skill_slot1\""));
        assert!(html.contains("data-bind=\"_pet1_skill_slot2\""));
        assert!(html.contains("data-bind=\"_pet1_skill_slot3\""));
        assert!(html
            .contains("exclude-selected-inputs=\"[data-pet-skill-input-group=&quot;pet1&quot;]\""));
        assert!(html.contains("15%"));
        assert!(html.contains("3%"));
        assert!(html.contains("1%"));
        assert!(html.contains("15%</span><span class=\"shrink-0 text-base-content/30\">|</span><span class=\"min-w-0 flex-1\"><span class=\"flex min-w-0 flex-wrap gap-1\"><span class=\"badge"));
        assert!(html.contains("+7% Fish EXP"));
        assert!(html.contains("title=\"Luck +1\""));
        assert!(html.contains("(Selected)</span>"));
        assert!(!html.contains("border-emerald-400"));
        assert!(!html.contains("Choose 1 to 3 skills."));
    }

    #[test]
    fn pet_skill_options_sort_by_learn_chance_descending() {
        let high_skill = CalculatorPetOptionEntry {
            key: "skill_high".to_string(),
            label: "Z Fishing EXP +7%".to_string(),
            fishing_exp: Some(0.07),
            ..CalculatorPetOptionEntry::default()
        };
        let mid_skill = CalculatorPetOptionEntry {
            key: "skill_mid".to_string(),
            label: "A Luck +1".to_string(),
            ..CalculatorPetOptionEntry::default()
        };
        let low_skill = CalculatorPetOptionEntry {
            key: "skill_low".to_string(),
            label: "M Fishing EXP +5%".to_string(),
            fishing_exp: Some(0.05),
            ..CalculatorPetOptionEntry::default()
        };
        let options = vec![
            SelectOption {
                value: "skill_mid",
                label: &mid_skill.label,
                icon: None,
                grade_tone: "unknown",
                pet_variant_talent: None,
                pet_variant_special: None,
                pet_skill: Some(&mid_skill),
                pet_effective_talent_effects: None,
                pet_skill_learn_chance: Some(0.03),
                item: None,
                lifeskill_level: None,
                presentation: SelectOptionPresentation::Default,
                sort_priority: 1,
            },
            SelectOption {
                value: "skill_low",
                label: &low_skill.label,
                icon: None,
                grade_tone: "unknown",
                pet_variant_talent: None,
                pet_variant_special: None,
                pet_skill: Some(&low_skill),
                pet_effective_talent_effects: None,
                pet_skill_learn_chance: Some(0.01),
                item: None,
                lifeskill_level: None,
                presentation: SelectOptionPresentation::Default,
                sort_priority: 1,
            },
            SelectOption {
                value: "skill_high",
                label: &high_skill.label,
                icon: None,
                grade_tone: "unknown",
                pet_variant_talent: None,
                pet_variant_special: None,
                pet_skill: Some(&high_skill),
                pet_effective_talent_effects: None,
                pet_skill_learn_chance: Some(0.15),
                item: None,
                lifeskill_level: None,
                presentation: SelectOptionPresentation::Default,
                sort_priority: 1,
            },
        ];

        let html = render_searchable_select_results(
            CalculatorLocale::EnUs,
            "",
            "pet-skill-results",
            &options,
            "",
            "",
            0,
        );
        let high_index = html
            .find("data-value=\"skill_high\"")
            .expect("high chance skill should render");
        let mid_index = html
            .find("data-value=\"skill_mid\"")
            .expect("mid chance skill should render");
        let low_index = html
            .find("data-value=\"skill_low\"")
            .expect("low chance skill should render");

        assert!(high_index < mid_index);
        assert!(mid_index < low_index);
    }

    #[test]
    fn render_pet_cards_uses_dropdowns_for_same_tier_skin_variant_fixed_options() {
        let catalog = CalculatorPetCatalog {
            slots: 1,
            pets: vec![
                CalculatorPetEntry {
                    key: "pet:skin-a".to_string(),
                    label: "Pet Skin A".to_string(),
                    variant_group_keys: vec!["variant:pet".to_string()],
                    tiers: vec![CalculatorPetTierEntry {
                        key: "4".to_string(),
                        specials: vec!["special_a".to_string()],
                        talents: vec!["talent_a".to_string()],
                        ..CalculatorPetTierEntry::default()
                    }],
                    ..CalculatorPetEntry::default()
                },
                CalculatorPetEntry {
                    key: "pet:skin-b".to_string(),
                    label: "Pet Skin B".to_string(),
                    variant_group_keys: vec!["variant:pet".to_string()],
                    tiers: vec![CalculatorPetTierEntry {
                        key: "4".to_string(),
                        specials: vec!["special_b".to_string()],
                        talents: vec!["talent_b".to_string()],
                        ..CalculatorPetTierEntry::default()
                    }],
                    ..CalculatorPetEntry::default()
                },
                CalculatorPetEntry {
                    key: "pet:tier-neighbor".to_string(),
                    label: "Tier Neighbor".to_string(),
                    lineage_keys: vec!["variant:pet".to_string()],
                    tiers: vec![CalculatorPetTierEntry {
                        key: "4".to_string(),
                        specials: vec!["special_c".to_string()],
                        talents: vec!["talent_c".to_string()],
                        ..CalculatorPetTierEntry::default()
                    }],
                    ..CalculatorPetEntry::default()
                },
            ],
            specials: vec![
                CalculatorPetOptionEntry {
                    key: "special_a".to_string(),
                    label: "Special A".to_string(),
                    ..CalculatorPetOptionEntry::default()
                },
                CalculatorPetOptionEntry {
                    key: "special_b".to_string(),
                    label: "Special B".to_string(),
                    ..CalculatorPetOptionEntry::default()
                },
                CalculatorPetOptionEntry {
                    key: "special_c".to_string(),
                    label: "Special C".to_string(),
                    ..CalculatorPetOptionEntry::default()
                },
            ],
            talents: vec![
                CalculatorPetOptionEntry {
                    key: "talent_a".to_string(),
                    label: "Talent A".to_string(),
                    ..CalculatorPetOptionEntry::default()
                },
                CalculatorPetOptionEntry {
                    key: "talent_b".to_string(),
                    label: "Talent B".to_string(),
                    ..CalculatorPetOptionEntry::default()
                },
                CalculatorPetOptionEntry {
                    key: "talent_c".to_string(),
                    label: "Talent C".to_string(),
                    ..CalculatorPetOptionEntry::default()
                },
            ],
            ..CalculatorPetCatalog::default()
        };
        let signals = CalculatorSignals {
            pet1: CalculatorPetSignals {
                pet: "pet:skin-a".to_string(),
                tier: "4".to_string(),
                special: "special_a".to_string(),
                talent: "talent_a".to_string(),
                ..CalculatorPetSignals::default()
            },
            ..CalculatorSignals::default()
        };

        let html = render_pet_cards(
            "",
            &DataLang::En,
            CalculatorLocale::EnUs,
            &catalog,
            &signals,
        );

        assert!(html.contains("calculator-pet1-special-picker"));
        assert!(html.contains("calculator-pet1-talent-picker"));
        assert!(html.contains("data-bind=\"pet1.special\""));
        assert!(html.contains("data-bind=\"pet1.talent\""));
        assert!(html.contains("Special B"));
        assert!(html.contains("Talent B"));
        assert!(!html.contains("data-value=\"special_c\""));
        assert!(!html.contains("data-value=\"talent_c\""));
    }

    #[test]
    fn render_pet_cards_variant_talent_options_include_pack_leader_bonus_badges() {
        let catalog = CalculatorPetCatalog {
            slots: 1,
            pets: vec![
                CalculatorPetEntry {
                    key: "pet:skin-a".to_string(),
                    label: "Pet Skin A".to_string(),
                    variant_group_keys: vec!["variant:pet".to_string()],
                    tiers: vec![CalculatorPetTierEntry {
                        key: "5".to_string(),
                        talents: vec!["talent_a".to_string()],
                        ..CalculatorPetTierEntry::default()
                    }],
                    ..CalculatorPetEntry::default()
                },
                CalculatorPetEntry {
                    key: "pet:skin-b".to_string(),
                    label: "Pet Skin B".to_string(),
                    variant_group_keys: vec!["variant:pet".to_string()],
                    tiers: vec![CalculatorPetTierEntry {
                        key: "5".to_string(),
                        talents: vec!["talent_b".to_string()],
                        ..CalculatorPetTierEntry::default()
                    }],
                    ..CalculatorPetEntry::default()
                },
            ],
            talents: vec![
                CalculatorPetOptionEntry {
                    key: "talent_a".to_string(),
                    label: "Durability Reduction Resistance +4%".to_string(),
                    durability_reduction_resistance: Some(0.04),
                    ..CalculatorPetOptionEntry::default()
                },
                CalculatorPetOptionEntry {
                    key: "talent_b".to_string(),
                    label: "Durability Reduction Resistance +4%".to_string(),
                    durability_reduction_resistance: Some(0.04),
                    ..CalculatorPetOptionEntry::default()
                },
            ],
            ..CalculatorPetCatalog::default()
        };
        let signals = CalculatorSignals {
            pet1: CalculatorPetSignals {
                pet: "pet:skin-a".to_string(),
                tier: "5".to_string(),
                pack_leader: true,
                talent: "talent_a".to_string(),
                ..CalculatorPetSignals::default()
            },
            ..CalculatorSignals::default()
        };

        let html = render_pet_cards(
            "",
            &DataLang::En,
            CalculatorLocale::EnUs,
            &catalog,
            &signals,
        );

        assert!(html.contains("calculator-pet1-talent-picker"));
        assert!(html.contains("+5% Item DRR"));
        assert!(!html.contains("+4% Item DRR"));
        assert!(html.matches("+5% Item DRR").count() >= 2);
    }

    #[tokio::test]
    async fn derived_signals_include_generic_stat_breakdown_payloads() {
        let state = test_state();
        let defaults = MockStore
            .calculator_catalog(DataLang::En, None)
            .await
            .unwrap()
            .defaults;
        let (_, _, derived) = load_calculator_runtime_data(
            &state,
            DataLang::En,
            CalculatorLocale::EnUs,
            None,
            &RequestId("req-test".to_string()),
            defaults,
        )
        .await
        .unwrap();

        let total_time =
            serde_json::from_str::<Value>(&derived.stat_breakdowns.total_time).unwrap();
        assert_eq!(total_time["title"], "Average Total Fishing Time");
        assert!(!total_time["value_text"]
            .as_str()
            .unwrap_or_default()
            .is_empty());
        assert_eq!(total_time["formula_terms"][0]["label"], "Average total");
        assert_eq!(total_time["formula_terms"][1]["label"], "Average bite time");
        assert_eq!(
            total_time["sections"][0]["rows"][0]["label"],
            "Average bite time"
        );

        let auto_fish_reduction =
            serde_json::from_str::<Value>(&derived.stat_breakdowns.auto_fish_time_reduction)
                .unwrap();
        assert_eq!(
            auto_fish_reduction["formula_terms"][1]["label"],
            "highest pet AFR"
        );
        assert_eq!(
            auto_fish_reduction["formula_terms"][2]["label"],
            "additive item AFR"
        );
        assert_eq!(auto_fish_reduction["formula_terms"][0]["aliases"][0], "AFR");

        let item_drr = serde_json::from_str::<Value>(&derived.stat_breakdowns.item_drr).unwrap();
        assert_eq!(item_drr["title"], "Item DRR");
        assert_eq!(item_drr["sections"][0]["rows"][0]["kind"], "item");
        assert!(item_drr["sections"][0]["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["label"] == "Lil' Otter Fishing Carrier"));

        let raw_prize =
            serde_json::from_str::<Value>(&derived.stat_breakdowns.raw_prize_rate).unwrap();
        assert_eq!(raw_prize["title"], "Raw Prize Catch Rate");
        assert_eq!(
            raw_prize["sections"][1]["rows"][0]["label"],
            "Resolved curve rate"
        );

        let catch_time =
            serde_json::from_str::<Value>(&derived.stat_breakdowns.catch_time).unwrap();
        assert_eq!(catch_time["title"], "Catch Time");
        assert_eq!(catch_time["sections"][1]["rows"][0]["label"], "Catch time");

        let time_saved =
            serde_json::from_str::<Value>(&derived.stat_breakdowns.time_saved).unwrap();
        assert_eq!(time_saved["title"], "Time Saved");
        assert_eq!(time_saved["sections"][1]["rows"][1]["label"], "Saved share");

        assert_eq!(derived.fishing_timeline_chart.segments.len(), 4);
        assert_eq!(
            derived.fishing_timeline_chart.segments[0].label,
            "Bite Time"
        );
        assert_eq!(
            derived.fishing_timeline_chart.segments[1].label,
            "Auto-Fishing Time"
        );
        assert_eq!(
            derived.fishing_timeline_chart.segments[3].label,
            "Time Saved"
        );
        assert!(derived.fishing_timeline_chart.segments[0]
            .breakdown
            .is_some());

        let target_expected =
            serde_json::from_str::<Value>(&derived.stat_breakdowns.target_expected_count).unwrap();
        assert_eq!(target_expected["title"], "Expected (8 hours)");
        assert_eq!(
            target_expected["sections"][0]["rows"][0]["value_text"],
            "Unavailable"
        );
        assert_eq!(
            target_expected["formula_terms"][1]["label"],
            "Expected catches"
        );
        assert_eq!(target_expected["formula_terms"][2]["label"], "Group share");
    }

    #[tokio::test]
    async fn eval_keeps_passive_auto_fish_time_when_active_is_true() {
        let response = post_calculator_datastar_eval(
            State(test_state()),
            Ok(Query(CalculatorQuery {
                lang: Some("en".to_string()),
                locale: Some("en-US".to_string()),
                r#ref: None,
                pet_cards: None,
                target_fish_select: None,
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
                locale: Some("en-US".to_string()),
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
                locale: Some("en-US".to_string()),
                r#ref: None,
                q: Some("vlia bech".to_string()),
                offset: None,
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
        assert!(!text.contains("data-next-offset"));
        assert!(!text.contains("data-searchable-dropdown-more"));
    }

    #[tokio::test]
    async fn searchable_select_results_include_pagination_metadata_when_more_results_exist() {
        let options = (0..30)
            .map(|index| SelectOption {
                value: Box::leak(format!("value-{index}").into_boxed_str()),
                label: Box::leak(format!("Option {index}").into_boxed_str()),
                icon: None,
                grade_tone: "unknown",
                pet_variant_talent: None,
                pet_variant_special: None,
                pet_skill: None,
                pet_effective_talent_effects: None,
                pet_skill_learn_chance: None,
                item: None,
                lifeskill_level: None,
                presentation: SelectOptionPresentation::Default,
                sort_priority: 1,
            })
            .collect::<Vec<_>>();
        let text = render_searchable_select_results(
            CalculatorLocale::EnUs,
            "",
            "calculator-search-results",
            &options,
            "",
            "",
            0,
        );

        assert!(text.contains("data-next-offset=\"24\""));
        assert!(text.contains("data-searchable-dropdown-more"));
        assert!(text.contains("Load more results"));
    }

    #[test]
    fn pet_select_results_prioritize_same_tier_variants_before_pagination() {
        let tier = CalculatorPetTierEntry {
            key: "5".to_string(),
            ..CalculatorPetTierEntry::default()
        };
        let mut pets = vec![
            CalculatorPetEntry {
                key: "pet:selected".to_string(),
                label: "Selected Dragon".to_string(),
                variant_group_keys: vec!["variant:dragon".to_string()],
                tiers: vec![tier.clone()],
                ..CalculatorPetEntry::default()
            },
            CalculatorPetEntry {
                key: "pet:variant".to_string(),
                label: "Z Variant Dragon".to_string(),
                variant_group_keys: vec!["variant:dragon".to_string()],
                tiers: vec![tier.clone()],
                ..CalculatorPetEntry::default()
            },
        ];
        pets.extend((0..30).map(|index| CalculatorPetEntry {
            key: format!("pet:other-{index:02}"),
            label: format!("A Other Pet {index:02}"),
            tiers: vec![tier.clone()],
            ..CalculatorPetEntry::default()
        }));
        let catalog = CalculatorPetCatalog {
            pets,
            ..CalculatorPetCatalog::default()
        };
        let options =
            select_options_from_pet_entries_for_tier(&catalog, "5", Some("pet:selected"), None);

        let text = render_searchable_select_results(
            CalculatorLocale::EnUs,
            "",
            "calculator-pet-search-results",
            &options,
            "pet:selected",
            "",
            0,
        );

        let selected_index = text
            .find("data-value=\"pet:selected\"")
            .expect("selected pet should be on the first page");
        let variant_index = text
            .find("data-value=\"pet:variant\"")
            .expect("same-tier variant should be on the first page");
        let other_index = text
            .find("data-value=\"pet:other-00\"")
            .expect("other pets should still be listed");

        assert!(selected_index < variant_index);
        assert!(variant_index < other_index);
        assert!(text.contains("data-next-offset=\"24\""));
        assert!(!text.contains("data-value=\"pet:other-29\""));
    }

    #[test]
    fn pet_select_search_ranks_text_match_before_same_tier_variant_priority() {
        let options = vec![
            SelectOption {
                value: "pet:variant",
                label: "Dire Cat Variant",
                icon: None,
                grade_tone: "unknown",
                pet_variant_talent: None,
                pet_variant_special: None,
                pet_skill: None,
                pet_effective_talent_effects: None,
                pet_skill_learn_chance: None,
                item: None,
                lifeskill_level: None,
                presentation: SelectOptionPresentation::Default,
                sort_priority: 0,
            },
            SelectOption {
                value: "pet:direct",
                label: "Direct Match",
                icon: None,
                grade_tone: "unknown",
                pet_variant_talent: None,
                pet_variant_special: None,
                pet_skill: None,
                pet_effective_talent_effects: None,
                pet_skill_learn_chance: None,
                item: None,
                lifeskill_level: None,
                presentation: SelectOptionPresentation::Default,
                sort_priority: 1,
            },
        ];

        let matches = super::fuzzy_select_matches(&options, "direct", "");

        assert_eq!(matches[0].value, "pet:direct");
        assert_eq!(matches[1].value, "pet:variant");
    }

    #[tokio::test]
    async fn option_search_returns_fuzzy_item_results_with_rich_content() {
        let response = get_calculator_datastar_option_search(
            State(test_state()),
            Ok(Query(CalculatorSearchableOptionQuery {
                lang: Some("en".to_string()),
                locale: Some("en-US".to_string()),
                r#ref: None,
                kind: Some("rod".to_string()),
                q: Some("baleno".to_string()),
                offset: None,
                results_id: Some("calculator-rod-picker-results".to_string()),
                selected: Some("item:16162".to_string()),
                tier: None,
                zone: None,
                pack_leader: None,
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
                locale: Some("en-US".to_string()),
                r#ref: None,
                kind: Some("lightstone_set".to_string()),
                q: Some("blacksmith".to_string()),
                offset: None,
                results_id: Some("calculator-lightstone-picker-results".to_string()),
                selected: Some("lightstone-set:30".to_string()),
                tier: None,
                zone: None,
                pack_leader: None,
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
                locale: Some("en-US".to_string()),
                r#ref: None,
                kind: Some("lifeskill_level".to_string()),
                q: Some("guru".to_string()),
                offset: None,
                results_id: Some("calculator-lifeskill-level-picker-results".to_string()),
                selected: Some("100".to_string()),
                tier: None,
                zone: None,
                pack_leader: None,
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

    #[tokio::test]
    async fn option_search_returns_pet_cards_for_tier() {
        let response = get_calculator_datastar_option_search(
            State(test_state()),
            Ok(Query(CalculatorSearchableOptionQuery {
                lang: Some("en".to_string()),
                locale: Some("en-US".to_string()),
                r#ref: None,
                kind: Some("pet".to_string()),
                q: Some("hawk drr".to_string()),
                offset: None,
                results_id: Some("calculator-pet1-pet-picker-results".to_string()),
                selected: Some("pet:3:1:pet_hawk_0014".to_string()),
                tier: Some("5".to_string()),
                zone: None,
                pack_leader: None,
            })),
            Extension(RequestId("req-test".to_string())),
        )
        .await
        .unwrap()
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body()).await.unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("id=\"calculator-pet1-pet-picker-results\""));
        assert!(text.contains("data-searchable-dropdown-option"));
        assert!(text.contains("Hawk"));
        assert!(text.contains("fishy-calculator-pet-option__badges"));
        assert!(text.contains("+5% Item DRR"));
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
    fn normalize_named_array_keeps_all_empty_placeholders_as_cleared_selection() {
        let valid_keys = std::collections::HashSet::from(["item:1".to_string()]);
        let lookup = HashMap::from([(normalize_lookup_value("Item One"), "item:1".to_string())]);

        let normalized = normalize_named_array(
            &["".to_string(), "".to_string()],
            &valid_keys,
            &lookup,
            None,
            vec!["item:1".to_string()],
            None,
        );

        assert!(normalized.is_empty());
    }

    #[test]
    fn init_signals_patch_map_keeps_checkbox_group_transport_arrays_compact() {
        let _data = CalculatorData {
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
                    CalculatorItemEntry {
                        key: "item:9359".to_string(),
                        name: "Balacs Lunchbox".to_string(),
                        r#type: "food".to_string(),
                        ..CalculatorItemEntry::default()
                    },
                    CalculatorItemEntry {
                        key: "item:16716".to_string(),
                        name: "Seafood Cron Meal".to_string(),
                        r#type: "food".to_string(),
                        ..CalculatorItemEntry::default()
                    },
                    CalculatorItemEntry {
                        key: "item:721092".to_string(),
                        name: "Treant's Tear".to_string(),
                        r#type: "buff".to_string(),
                        ..CalculatorItemEntry::default()
                    },
                    CalculatorItemEntry {
                        key: "item:15229".to_string(),
                        name: "Life Crystal: Fishing".to_string(),
                        r#type: "buff".to_string(),
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
            lang: CalculatorLocale::EnUs,
            api_lang: DataLang::En,
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
            food: vec!["item:9359".to_string()],
            buff: vec!["item:721092".to_string()],
            ..CalculatorSignals::default()
        };

        let patch = init_signals_patch_map(&signals).unwrap();

        assert_eq!(
            patch.get("outfit"),
            Some(&json!([
                "effect:8-piece-outfit-set-effect",
                "effect:awakening-weapon-outfit",
                "effect:mainhand-weapon-outfit",
                "item:14330"
            ]))
        );
        assert_eq!(
            patch.get("_outfit_slots"),
            Some(&json!([
                "effect:8-piece-outfit-set-effect",
                "effect:awakening-weapon-outfit",
                "effect:mainhand-weapon-outfit",
                "item:14330"
            ]))
        );
        assert_eq!(patch.get("food"), Some(&json!(["item:9359"])));
        assert_eq!(patch.get("_food_slots"), Some(&json!(["item:9359"])));
        assert_eq!(patch.get("buff"), Some(&json!(["item:721092"])));
        assert_eq!(patch.get("_buff_slots"), Some(&json!(["item:721092"])));
        assert_eq!(patch.get("_calculator_ui"), None);
    }

    #[test]
    fn default_reset_signals_patch_map_includes_transport_arrays_and_groups_tab() {
        let defaults = CalculatorSignals {
            food: vec!["item:9359".to_string()],
            buff: vec!["item:721092".to_string()],
            outfit: vec!["effect:8-piece-outfit-set-effect".to_string()],
            ..CalculatorSignals::default()
        };

        let patch = default_reset_signals_patch_map(&defaults).unwrap();

        assert_eq!(patch.get("food"), Some(&json!(["item:9359"])));
        assert_eq!(patch.get("_food_slots"), Some(&json!(["item:9359"])));
        assert_eq!(patch.get("buff"), Some(&json!(["item:721092"])));
        assert_eq!(patch.get("_buff_slots"), Some(&json!(["item:721092"])));
        assert_eq!(
            patch.get("outfit"),
            Some(&json!(["effect:8-piece-outfit-set-effect"]))
        );
        assert_eq!(
            patch.get("_outfit_slots"),
            Some(&json!(["effect:8-piece-outfit-set-effect"]))
        );
        assert_eq!(
            patch.get("_calculator_ui"),
            Some(&json!({
                "top_level_tab": "mode",
                "distribution_tab": "groups",
                "pinned_layout": [[["overview"]], [["zone"], ["session"]], [["bite_time"], ["loot"]]],
                "pinned_sections": ["overview", "zone", "session", "bite_time", "loot"],
                "unpinned_insert_index": [0, 0],
            }))
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
    fn normalize_pet_clears_skin_when_selected_tier_has_no_matching_variant() {
        let catalog = CalculatorPetCatalog {
            slots: 5,
            pets: vec![CalculatorPetEntry {
                key: "pet:3:1:pet_hawk_0014".to_string(),
                label: "Hawk".to_string(),
                skin_key: Some("2001".to_string()),
                image_url: Some("/images/pets/pet_hawk_0014.webp".to_string()),
                alias_keys: Vec::new(),
                lineage_keys: Vec::new(),
                variant_group_keys: Vec::new(),
                tiers: vec![CalculatorPetTierEntry {
                    key: "5".to_string(),
                    label: "Tier 5".to_string(),
                    specials: vec!["auto_fishing_time_reduction".to_string()],
                    talents: vec!["durability_reduction_resistance".to_string()],
                    skills: vec!["fishing_exp".to_string()],
                    skill_chances: Default::default(),
                }],
            }],
            tiers: (1..=5)
                .map(|tier| CalculatorOptionEntry {
                    key: tier.to_string(),
                    label: format!("Tier {tier}"),
                })
                .collect(),
            specials: vec![CalculatorPetOptionEntry {
                key: "auto_fishing_time_reduction".to_string(),
                label: "Auto-Fishing Time Reduction".to_string(),
                auto_fishing_time_reduction: Some(0.1),
                ..CalculatorPetOptionEntry::default()
            }],
            talents: vec![CalculatorPetOptionEntry {
                key: "durability_reduction_resistance".to_string(),
                label: "Durability Reduction Resistance".to_string(),
                durability_reduction_resistance: Some(0.05),
                ..CalculatorPetOptionEntry::default()
            }],
            skills: vec![CalculatorPetOptionEntry {
                key: "fishing_exp".to_string(),
                label: "Fishing EXP".to_string(),
                fishing_exp: Some(0.07),
                ..CalculatorPetOptionEntry::default()
            }],
        };
        let aliases = build_pet_value_aliases(&catalog);
        let defaults = CalculatorPetSignals {
            tier: "5".to_string(),
            ..CalculatorPetSignals::default()
        };
        let mut pet = CalculatorPetSignals {
            pet: "pet:3:1:pet_hawk_0014".to_string(),
            tier: "3".to_string(),
            pack_leader: false,
            special: "auto_fishing_time_reduction".to_string(),
            talent: "durability_reduction_resistance".to_string(),
            skills: vec!["fishing_exp".to_string()],
        };

        normalize_pet(&mut pet, defaults, &catalog, &aliases);

        assert_eq!(pet.tier, "3");
        assert_eq!(pet.pet, "");
        assert_eq!(pet.special, "");
        assert_eq!(pet.talent, "");
        assert!(pet.skills.is_empty());
    }

    #[test]
    fn normalize_pet_keeps_selected_skin_when_target_tier_exists() {
        let catalog = CalculatorPetCatalog {
            slots: 5,
            pets: vec![CalculatorPetEntry {
                key: "pet:hawk".to_string(),
                label: "Hawk".to_string(),
                skin_key: Some("2001".to_string()),
                image_url: Some("/images/pets/pet_hawk_0014.webp".to_string()),
                alias_keys: Vec::new(),
                lineage_keys: Vec::new(),
                variant_group_keys: Vec::new(),
                tiers: vec![
                    CalculatorPetTierEntry {
                        key: "3".to_string(),
                        label: "Tier 3".to_string(),
                        specials: vec!["special_t3".to_string()],
                        talents: vec!["talent_t3".to_string()],
                        skills: vec!["skill_t3".to_string()],
                        skill_chances: Default::default(),
                    },
                    CalculatorPetTierEntry {
                        key: "4".to_string(),
                        label: "Tier 4".to_string(),
                        specials: vec!["special_t4".to_string()],
                        talents: vec!["talent_t4".to_string()],
                        skills: vec!["skill_t4".to_string()],
                        skill_chances: Default::default(),
                    },
                ],
            }],
            tiers: (1..=5)
                .map(|tier| CalculatorOptionEntry {
                    key: tier.to_string(),
                    label: format!("Tier {tier}"),
                })
                .collect(),
            specials: vec![
                CalculatorPetOptionEntry {
                    key: "special_t3".to_string(),
                    label: "Tier 3 Special".to_string(),
                    auto_fishing_time_reduction: Some(0.07),
                    ..CalculatorPetOptionEntry::default()
                },
                CalculatorPetOptionEntry {
                    key: "special_t4".to_string(),
                    label: "Tier 4 Special".to_string(),
                    auto_fishing_time_reduction: Some(0.10),
                    ..CalculatorPetOptionEntry::default()
                },
            ],
            talents: vec![
                CalculatorPetOptionEntry {
                    key: "talent_t3".to_string(),
                    label: "Tier 3 Talent".to_string(),
                    durability_reduction_resistance: Some(0.03),
                    ..CalculatorPetOptionEntry::default()
                },
                CalculatorPetOptionEntry {
                    key: "talent_t4".to_string(),
                    label: "Tier 4 Talent".to_string(),
                    durability_reduction_resistance: Some(0.04),
                    ..CalculatorPetOptionEntry::default()
                },
            ],
            skills: vec![
                CalculatorPetOptionEntry {
                    key: "skill_t3".to_string(),
                    label: "Tier 3 Skill".to_string(),
                    fishing_exp: Some(0.03),
                    ..CalculatorPetOptionEntry::default()
                },
                CalculatorPetOptionEntry {
                    key: "skill_t4".to_string(),
                    label: "Tier 4 Skill".to_string(),
                    fishing_exp: Some(0.04),
                    ..CalculatorPetOptionEntry::default()
                },
            ],
        };
        let aliases = build_pet_value_aliases(&catalog);
        let defaults = CalculatorPetSignals {
            tier: "4".to_string(),
            ..CalculatorPetSignals::default()
        };
        let mut pet = CalculatorPetSignals {
            pet: "pet:hawk".to_string(),
            tier: "4".to_string(),
            pack_leader: true,
            special: "special_t3".to_string(),
            talent: "talent_t3".to_string(),
            skills: vec!["skill_t3".to_string()],
        };

        normalize_pet(&mut pet, defaults, &catalog, &aliases);

        assert_eq!(pet.pet, "pet:hawk");
        assert_eq!(pet.tier, "4");
        assert_eq!(pet.special, "special_t4");
        assert_eq!(pet.talent, "talent_t4");
        assert_eq!(pet.skills, vec!["skill_t4".to_string()]);
        assert!(!pet.pack_leader);
    }

    #[test]
    fn normalize_pet_forces_fixed_special_and_talent_from_skin() {
        let catalog = CalculatorPetCatalog {
            slots: 5,
            pets: vec![CalculatorPetEntry {
                key: "pet:fixed".to_string(),
                label: "Fixed Pet".to_string(),
                tiers: vec![CalculatorPetTierEntry {
                    key: "5".to_string(),
                    label: "Tier 5".to_string(),
                    specials: vec!["skin_special".to_string(), "other_special".to_string()],
                    talents: vec!["skin_talent".to_string(), "other_talent".to_string()],
                    skills: vec!["skill_a".to_string()],
                    skill_chances: Default::default(),
                }],
                ..CalculatorPetEntry::default()
            }],
            tiers: vec![CalculatorOptionEntry {
                key: "5".to_string(),
                label: "Tier 5".to_string(),
            }],
            specials: vec![
                CalculatorPetOptionEntry {
                    key: "skin_special".to_string(),
                    label: "Skin Special".to_string(),
                    auto_fishing_time_reduction: Some(0.1),
                    ..CalculatorPetOptionEntry::default()
                },
                CalculatorPetOptionEntry {
                    key: "other_special".to_string(),
                    label: "Other Special".to_string(),
                    auto_fishing_time_reduction: Some(0.2),
                    ..CalculatorPetOptionEntry::default()
                },
            ],
            talents: vec![
                CalculatorPetOptionEntry {
                    key: "skin_talent".to_string(),
                    label: "Skin Talent".to_string(),
                    durability_reduction_resistance: Some(0.05),
                    ..CalculatorPetOptionEntry::default()
                },
                CalculatorPetOptionEntry {
                    key: "other_talent".to_string(),
                    label: "Other Talent".to_string(),
                    life_exp: Some(0.05),
                    ..CalculatorPetOptionEntry::default()
                },
            ],
            skills: vec![CalculatorPetOptionEntry {
                key: "skill_a".to_string(),
                label: "Skill A".to_string(),
                fishing_exp: Some(0.01),
                ..CalculatorPetOptionEntry::default()
            }],
        };
        let aliases = build_pet_value_aliases(&catalog);
        let mut pet = CalculatorPetSignals {
            pet: "pet:fixed".to_string(),
            tier: "5".to_string(),
            special: "other_special".to_string(),
            talent: "other_talent".to_string(),
            skills: vec!["skill_a".to_string()],
            ..CalculatorPetSignals::default()
        };

        normalize_pet(
            &mut pet,
            CalculatorPetSignals::default(),
            &catalog,
            &aliases,
        );

        assert_eq!(pet.special, "skin_special");
        assert_eq!(pet.talent, "skin_talent");
    }

    #[test]
    fn normalize_pet_switches_only_same_tier_skin_variant_for_fixed_options() {
        let catalog = CalculatorPetCatalog {
            pets: vec![
                CalculatorPetEntry {
                    key: "pet:skin-a".to_string(),
                    label: "Pet Skin A".to_string(),
                    lineage_keys: vec!["lineage:pet".to_string()],
                    variant_group_keys: vec!["variant:pet".to_string()],
                    tiers: vec![CalculatorPetTierEntry {
                        key: "4".to_string(),
                        specials: vec!["special_a".to_string()],
                        talents: vec!["talent_a".to_string()],
                        skills: vec!["skill_shared".to_string()],
                        skill_chances: Default::default(),
                        ..CalculatorPetTierEntry::default()
                    }],
                    ..CalculatorPetEntry::default()
                },
                CalculatorPetEntry {
                    key: "pet:skin-b".to_string(),
                    label: "Pet Skin B".to_string(),
                    lineage_keys: vec!["lineage:pet".to_string()],
                    variant_group_keys: vec!["variant:pet".to_string()],
                    tiers: vec![CalculatorPetTierEntry {
                        key: "4".to_string(),
                        specials: vec!["special_b".to_string()],
                        talents: vec!["talent_b".to_string()],
                        skills: vec!["skill_shared".to_string()],
                        skill_chances: Default::default(),
                        ..CalculatorPetTierEntry::default()
                    }],
                    ..CalculatorPetEntry::default()
                },
                CalculatorPetEntry {
                    key: "pet:tier-neighbor".to_string(),
                    label: "Tier Neighbor".to_string(),
                    lineage_keys: vec!["lineage:pet".to_string()],
                    tiers: vec![CalculatorPetTierEntry {
                        key: "4".to_string(),
                        specials: vec!["special_c".to_string()],
                        talents: vec!["talent_c".to_string()],
                        skills: vec!["skill_shared".to_string()],
                        skill_chances: Default::default(),
                        ..CalculatorPetTierEntry::default()
                    }],
                    ..CalculatorPetEntry::default()
                },
            ],
            specials: vec![
                CalculatorPetOptionEntry {
                    key: "special_a".to_string(),
                    label: "Special A".to_string(),
                    ..CalculatorPetOptionEntry::default()
                },
                CalculatorPetOptionEntry {
                    key: "special_b".to_string(),
                    label: "Special B".to_string(),
                    ..CalculatorPetOptionEntry::default()
                },
                CalculatorPetOptionEntry {
                    key: "special_c".to_string(),
                    label: "Special C".to_string(),
                    ..CalculatorPetOptionEntry::default()
                },
            ],
            talents: vec![
                CalculatorPetOptionEntry {
                    key: "talent_a".to_string(),
                    label: "Talent A".to_string(),
                    ..CalculatorPetOptionEntry::default()
                },
                CalculatorPetOptionEntry {
                    key: "talent_b".to_string(),
                    label: "Talent B".to_string(),
                    ..CalculatorPetOptionEntry::default()
                },
                CalculatorPetOptionEntry {
                    key: "talent_c".to_string(),
                    label: "Talent C".to_string(),
                    ..CalculatorPetOptionEntry::default()
                },
            ],
            skills: vec![CalculatorPetOptionEntry {
                key: "skill_shared".to_string(),
                label: "Shared Skill".to_string(),
                ..CalculatorPetOptionEntry::default()
            }],
            ..CalculatorPetCatalog::default()
        };
        let aliases = build_pet_value_aliases(&catalog);
        let defaults = CalculatorPetSignals {
            tier: "4".to_string(),
            ..CalculatorPetSignals::default()
        };

        let mut pet = CalculatorPetSignals {
            pet: "pet:skin-a".to_string(),
            tier: "4".to_string(),
            special: "special_a".to_string(),
            talent: "talent_b".to_string(),
            skills: vec!["skill_shared".to_string()],
            ..CalculatorPetSignals::default()
        };
        normalize_pet(&mut pet, defaults.clone(), &catalog, &aliases);
        assert_eq!(pet.pet, "pet:skin-b");
        assert_eq!(pet.special, "special_b");
        assert_eq!(pet.talent, "talent_b");
        assert_eq!(pet.skills, vec!["skill_shared".to_string()]);

        let mut lineage_only_pet = CalculatorPetSignals {
            pet: "pet:skin-a".to_string(),
            tier: "4".to_string(),
            special: "special_a".to_string(),
            talent: "talent_c".to_string(),
            skills: vec!["skill_shared".to_string()],
            ..CalculatorPetSignals::default()
        };
        normalize_pet(&mut lineage_only_pet, defaults, &catalog, &aliases);
        assert_eq!(lineage_only_pet.pet, "pet:skin-a");
        assert_eq!(lineage_only_pet.special, "special_a");
        assert_eq!(lineage_only_pet.talent, "talent_a");
    }

    #[test]
    fn normalize_pet_follows_lineage_to_target_tier_entry() {
        let lineage_key = "change-look:46001:46:1".to_string();
        let catalog = CalculatorPetCatalog {
            slots: 5,
            pets: vec![
                CalculatorPetEntry {
                    key: "pet:arctic:tier3".to_string(),
                    label: "Arctic Fox".to_string(),
                    skin_key: Some("46001".to_string()),
                    image_url: Some("/images/pets/pet_arcticfox_0001.webp".to_string()),
                    lineage_keys: vec![lineage_key.clone()],
                    tiers: vec![CalculatorPetTierEntry {
                        key: "3".to_string(),
                        label: "Tier 3".to_string(),
                        talents: vec!["talent_t3".to_string()],
                        skills: vec!["skill_t3".to_string()],
                        ..CalculatorPetTierEntry::default()
                    }],
                    ..CalculatorPetEntry::default()
                },
                CalculatorPetEntry {
                    key: "pet:arctic:tier45".to_string(),
                    label: "Arctic Fox".to_string(),
                    skin_key: Some("46001".to_string()),
                    image_url: Some("/images/pets/pet_arcticfox_0002.webp".to_string()),
                    lineage_keys: vec![lineage_key],
                    tiers: vec![
                        CalculatorPetTierEntry {
                            key: "4".to_string(),
                            label: "Tier 4".to_string(),
                            talents: vec!["talent_t4".to_string()],
                            skills: vec!["skill_t4".to_string()],
                            ..CalculatorPetTierEntry::default()
                        },
                        CalculatorPetTierEntry {
                            key: "5".to_string(),
                            label: "Tier 5".to_string(),
                            talents: vec!["talent_t4".to_string()],
                            skills: vec!["skill_t4".to_string()],
                            ..CalculatorPetTierEntry::default()
                        },
                    ],
                    ..CalculatorPetEntry::default()
                },
            ],
            tiers: (1..=5)
                .map(|tier| CalculatorOptionEntry {
                    key: tier.to_string(),
                    label: format!("Tier {tier}"),
                })
                .collect(),
            talents: vec![
                CalculatorPetOptionEntry {
                    key: "talent_t3".to_string(),
                    label: "Tier 3 Talent".to_string(),
                    durability_reduction_resistance: Some(0.03),
                    ..CalculatorPetOptionEntry::default()
                },
                CalculatorPetOptionEntry {
                    key: "talent_t4".to_string(),
                    label: "Tier 4 Talent".to_string(),
                    durability_reduction_resistance: Some(0.04),
                    ..CalculatorPetOptionEntry::default()
                },
            ],
            skills: vec![
                CalculatorPetOptionEntry {
                    key: "skill_t3".to_string(),
                    label: "Tier 3 Skill".to_string(),
                    fishing_exp: Some(0.03),
                    ..CalculatorPetOptionEntry::default()
                },
                CalculatorPetOptionEntry {
                    key: "skill_t4".to_string(),
                    label: "Tier 4 Skill".to_string(),
                    fishing_exp: Some(0.04),
                    ..CalculatorPetOptionEntry::default()
                },
            ],
            ..CalculatorPetCatalog::default()
        };
        let aliases = build_pet_value_aliases(&catalog);
        let defaults = CalculatorPetSignals::default();

        let mut descendant_to_ancestor = CalculatorPetSignals {
            pet: "pet:arctic:tier45".to_string(),
            tier: "3".to_string(),
            pack_leader: true,
            talent: "talent_t4".to_string(),
            skills: vec!["skill_t4".to_string()],
            ..CalculatorPetSignals::default()
        };
        normalize_pet(
            &mut descendant_to_ancestor,
            defaults.clone(),
            &catalog,
            &aliases,
        );
        assert_eq!(descendant_to_ancestor.pet, "pet:arctic:tier3");
        assert_eq!(descendant_to_ancestor.tier, "3");
        assert_eq!(descendant_to_ancestor.talent, "talent_t3");
        assert_eq!(descendant_to_ancestor.skills, vec!["skill_t3".to_string()]);
        assert!(!descendant_to_ancestor.pack_leader);

        let mut ancestor_to_descendant = CalculatorPetSignals {
            pet: "pet:arctic:tier3".to_string(),
            tier: "4".to_string(),
            talent: "talent_t3".to_string(),
            skills: vec!["skill_t3".to_string()],
            ..CalculatorPetSignals::default()
        };
        normalize_pet(&mut ancestor_to_descendant, defaults, &catalog, &aliases);
        assert_eq!(ancestor_to_descendant.pet, "pet:arctic:tier45");
        assert_eq!(ancestor_to_descendant.tier, "4");
        assert_eq!(ancestor_to_descendant.talent, "talent_t4");
        assert_eq!(ancestor_to_descendant.skills, vec!["skill_t4".to_string()]);
    }

    #[test]
    fn normalize_pet_defaults_to_first_skill_and_caps_skill_count() {
        let catalog = CalculatorPetCatalog {
            slots: 5,
            pets: vec![CalculatorPetEntry {
                key: "pet:test".to_string(),
                label: "Test Pet".to_string(),
                skin_key: None,
                image_url: None,
                alias_keys: Vec::new(),
                lineage_keys: Vec::new(),
                variant_group_keys: Vec::new(),
                tiers: vec![CalculatorPetTierEntry {
                    key: "5".to_string(),
                    label: "Tier 5".to_string(),
                    specials: Vec::new(),
                    talents: vec!["durability_reduction_resistance".to_string()],
                    skills: vec![
                        "skill_a".to_string(),
                        "skill_b".to_string(),
                        "skill_c".to_string(),
                        "skill_d".to_string(),
                    ],
                    skill_chances: Default::default(),
                }],
            }],
            tiers: vec![CalculatorOptionEntry {
                key: "5".to_string(),
                label: "Tier 5".to_string(),
            }],
            specials: Vec::new(),
            talents: vec![CalculatorPetOptionEntry {
                key: "durability_reduction_resistance".to_string(),
                label: "Durability Reduction Resistance".to_string(),
                durability_reduction_resistance: Some(0.05),
                ..CalculatorPetOptionEntry::default()
            }],
            skills: vec![
                CalculatorPetOptionEntry {
                    key: "skill_a".to_string(),
                    label: "Skill A".to_string(),
                    fishing_exp: Some(0.01),
                    ..CalculatorPetOptionEntry::default()
                },
                CalculatorPetOptionEntry {
                    key: "skill_b".to_string(),
                    label: "Skill B".to_string(),
                    fishing_exp: Some(0.02),
                    ..CalculatorPetOptionEntry::default()
                },
                CalculatorPetOptionEntry {
                    key: "skill_c".to_string(),
                    label: "Skill C".to_string(),
                    fishing_exp: Some(0.03),
                    ..CalculatorPetOptionEntry::default()
                },
                CalculatorPetOptionEntry {
                    key: "skill_d".to_string(),
                    label: "Skill D".to_string(),
                    fishing_exp: Some(0.04),
                    ..CalculatorPetOptionEntry::default()
                },
            ],
        };
        let aliases = build_pet_value_aliases(&catalog);
        let defaults = CalculatorPetSignals {
            tier: "5".to_string(),
            ..CalculatorPetSignals::default()
        };

        let mut pet = CalculatorPetSignals {
            pet: "pet:test".to_string(),
            tier: "5".to_string(),
            pack_leader: false,
            talent: "durability_reduction_resistance".to_string(),
            skills: vec![
                "skill_c".to_string(),
                "skill_a".to_string(),
                "skill_d".to_string(),
                "skill_b".to_string(),
            ],
            ..CalculatorPetSignals::default()
        };

        normalize_pet(&mut pet, defaults.clone(), &catalog, &aliases);
        assert_eq!(
            pet.skills,
            vec![
                "skill_c".to_string(),
                "skill_a".to_string(),
                "skill_d".to_string(),
            ]
        );

        let mut empty_skill_pet = CalculatorPetSignals {
            pet: "pet:test".to_string(),
            tier: "5".to_string(),
            pack_leader: false,
            talent: "durability_reduction_resistance".to_string(),
            ..CalculatorPetSignals::default()
        };
        normalize_pet(&mut empty_skill_pet, defaults, &catalog, &aliases);
        assert_eq!(empty_skill_pet.skills, vec!["skill_a".to_string()]);
    }

    #[test]
    fn pet_skill_limit_tracks_pet_tier() {
        assert_eq!(pet_skill_limit_for_tier_key("1"), 1);
        assert_eq!(pet_skill_limit_for_tier_key("2"), 1);
        assert_eq!(pet_skill_limit_for_tier_key("3"), 2);
        assert_eq!(pet_skill_limit_for_tier_key("4"), 3);
        assert_eq!(pet_skill_limit_for_tier_key("5"), 3);
    }

    #[test]
    fn normalize_pack_leader_selection_keeps_only_first_selected_pet() {
        let mut pet1 = CalculatorPetSignals {
            pet: "pet:1".to_string(),
            tier: "5".to_string(),
            pack_leader: true,
            ..CalculatorPetSignals::default()
        };
        let mut pet2 = CalculatorPetSignals {
            pet: "pet:2".to_string(),
            tier: "5".to_string(),
            pack_leader: true,
            ..CalculatorPetSignals::default()
        };
        let mut pet3 = CalculatorPetSignals {
            pet: String::new(),
            tier: "5".to_string(),
            pack_leader: true,
            ..CalculatorPetSignals::default()
        };
        let mut pet4 = CalculatorPetSignals {
            pet: "pet:4".to_string(),
            tier: "4".to_string(),
            pack_leader: true,
            ..CalculatorPetSignals::default()
        };
        let mut pet5 = CalculatorPetSignals::default();

        normalize_pack_leader_selection([&mut pet1, &mut pet2, &mut pet3, &mut pet4, &mut pet5]);

        assert!(pet1.pack_leader);
        assert!(!pet2.pack_leader);
        assert!(!pet3.pack_leader);
        assert!(!pet4.pack_leader);
    }

    #[test]
    fn pet_talent_effect_uses_catalog_value_and_applies_pack_leader_bonus() {
        let catalog = CalculatorPetCatalog {
            talents: vec![CalculatorPetOptionEntry {
                key: "durability_reduction_resistance".to_string(),
                label: "Durability Reduction Resistance".to_string(),
                durability_reduction_resistance: Some(0.04),
                ..CalculatorPetOptionEntry::default()
            }],
            ..CalculatorPetCatalog::default()
        };

        let pet = CalculatorPetSignals {
            pet: "pet:1".to_string(),
            tier: "4".to_string(),
            talent: "durability_reduction_resistance".to_string(),
            ..CalculatorPetSignals::default()
        };
        let tier_four_pack_leader_pet = CalculatorPetSignals {
            pet: "pet:1".to_string(),
            tier: "4".to_string(),
            pack_leader: true,
            talent: "durability_reduction_resistance".to_string(),
            ..CalculatorPetSignals::default()
        };
        let pack_leader_pet = CalculatorPetSignals {
            pet: "pet:1".to_string(),
            tier: "5".to_string(),
            pack_leader: true,
            talent: "durability_reduction_resistance".to_string(),
            ..CalculatorPetSignals::default()
        };

        assert!((pet_drr(&pet, &catalog) - 0.04).abs() < 0.0001);
        assert!((pet_drr(&tier_four_pack_leader_pet, &catalog) - 0.04).abs() < 0.0001);
        assert!((pet_drr(&pack_leader_pet, &catalog) - 0.05).abs() < 0.0001);
    }

    #[test]
    fn render_pet_effective_talent_badges_includes_pack_leader_bonus() {
        let talent = CalculatorPetOptionEntry {
            key: "durability_reduction_resistance".to_string(),
            label: "Durability Reduction Resistance".to_string(),
            durability_reduction_resistance: Some(0.04),
            ..CalculatorPetOptionEntry::default()
        };
        let catalog = CalculatorPetCatalog {
            talents: vec![talent.clone()],
            ..CalculatorPetCatalog::default()
        };
        let pet = CalculatorPetSignals {
            pet: "pet:1".to_string(),
            tier: "5".to_string(),
            pack_leader: true,
            talent: "durability_reduction_resistance".to_string(),
            ..CalculatorPetSignals::default()
        };

        let html =
            render_pet_effective_talent_badges(CalculatorLocale::EnUs, &pet, &catalog, &talent);

        assert!(html.contains("+5% Item DRR"));
        assert!(!html.contains("+16% Item DRR"));
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
            pets: vec![CalculatorPetEntry {
                key: "pet:azure:38".to_string(),
                label: "Young Azure Dragon".to_string(),
                alias_keys: vec!["pet:azure:43".to_string()],
                ..CalculatorPetEntry::default()
            }],
            specials: vec![CalculatorPetOptionEntry {
                key: "auto_fishing_time_reduction".to_string(),
                label: "자동 낚시 시간 감소".to_string(),
                ..CalculatorPetOptionEntry::default()
            }],
            talents: vec![CalculatorPetOptionEntry {
                key: "life_exp".to_string(),
                label: "생활 경험치".to_string(),
                life_exp: Some(0.01),
                ..CalculatorPetOptionEntry::default()
            }],
            skills: vec![CalculatorPetOptionEntry {
                key: "fishing_exp".to_string(),
                label: "낚시 경험치".to_string(),
                fishing_exp: Some(0.01),
                ..CalculatorPetOptionEntry::default()
            }],
            ..CalculatorPetCatalog::default()
        });

        assert_eq!(
            aliases
                .options
                .get(&normalize_lookup_value("자동 낚시 시간 감소")),
            Some(&"auto_fishing_time_reduction".to_string())
        );
        assert_eq!(
            aliases.options.get(&normalize_lookup_value("life exp")),
            Some(&"life_exp".to_string())
        );
        assert_eq!(
            aliases.options.get(&normalize_lookup_value("Fishing EXP")),
            Some(&"fishing_exp".to_string())
        );
        assert_eq!(
            aliases.pets.get(&normalize_lookup_value("pet:azure:43")),
            Some(&"pet:azure:38".to_string())
        );
        assert_eq!(
            aliases
                .pets
                .get(&normalize_lookup_value("Young Azure Dragon")),
            Some(&"pet:azure:38".to_string())
        );
    }

    #[test]
    fn render_select_option_search_text_includes_pet_talent_label() {
        let option = SelectOption {
            value: "pet:azure:life",
            label: "Young Azure Dragon",
            icon: Some("/images/pets/pet_blue_dragon_0001.webp"),
            grade_tone: "red",
            pet_variant_talent: Some(&CalculatorPetOptionEntry {
                key: "life_exp".to_string(),
                label: "Life EXP +4%".to_string(),
                life_exp: Some(0.04),
                ..CalculatorPetOptionEntry::default()
            }),
            pet_variant_special: None,
            pet_skill: None,
            pet_effective_talent_effects: None,
            pet_skill_learn_chance: None,
            item: None,
            lifeskill_level: None,
            presentation: SelectOptionPresentation::PetCard,
            sort_priority: 1,
        };

        let text = render_select_option_search_text(option);
        assert!(text.contains("Young Azure Dragon"));
        assert!(text.contains("Life EXP +4%"));
        assert!(text.contains("life exp"));
        assert!(text.contains("life experience"));
    }

    #[test]
    fn render_select_option_search_text_includes_pet_special_label() {
        let option = SelectOption {
            value: "pet:penguin",
            label: "Penguin",
            icon: Some("/images/pets/pet_penguin_0001.webp"),
            grade_tone: "red",
            pet_variant_talent: None,
            pet_variant_special: Some(&CalculatorPetOptionEntry {
                key: "pet-special:37".to_string(),
                label: "Special: Auto-Fishing Time Reduction -30%".to_string(),
                auto_fishing_time_reduction: Some(0.30),
                ..CalculatorPetOptionEntry::default()
            }),
            pet_skill: None,
            pet_effective_talent_effects: None,
            pet_skill_learn_chance: None,
            item: None,
            lifeskill_level: None,
            presentation: SelectOptionPresentation::PetCard,
            sort_priority: 1,
        };

        let text = render_select_option_search_text(option);
        assert!(text.contains("Penguin"));
        assert!(text.contains("Special: Auto-Fishing Time Reduction -30%"));
        assert!(text.contains("auto fishing"));
        assert!(text.contains("auto-fishing"));
        assert!(text.contains("afr"));
    }

    #[test]
    fn render_pet_dropdown_selected_content_hides_redundant_fixed_badges() {
        let special = CalculatorPetOptionEntry {
            key: "pet-special:37".to_string(),
            label: "Special: Auto-Fishing Time Reduction -30%".to_string(),
            auto_fishing_time_reduction: Some(0.30),
            ..CalculatorPetOptionEntry::default()
        };
        let talent = CalculatorPetOptionEntry {
            key: "durability_reduction_resistance".to_string(),
            label: "Durability Reduction Resistance +5%".to_string(),
            durability_reduction_resistance: Some(0.05),
            ..CalculatorPetOptionEntry::default()
        };
        let option = SelectOption {
            value: "pet:hawk",
            label: "Hawk",
            icon: Some("/images/pets/pet_hawk_0014.webp"),
            grade_tone: "red",
            pet_variant_talent: Some(&talent),
            pet_variant_special: Some(&special),
            pet_skill: None,
            pet_effective_talent_effects: None,
            pet_skill_learn_chance: None,
            item: None,
            lifeskill_level: None,
            presentation: SelectOptionPresentation::PetCard,
            sort_priority: 1,
        };

        let selected_html =
            render_pet_dropdown_selected_content_html(CalculatorLocale::EnUs, "", option);
        let result_html =
            render_pet_dropdown_option_content_html(CalculatorLocale::EnUs, "", option);

        assert!(selected_html.contains("fishy-calculator-pet-option--selected"));
        assert!(!selected_html.contains("fishy-calculator-pet-option__badges"));
        assert!(!selected_html.contains("+5% Item DRR"));
        assert!(!selected_html.contains("Special: Auto-Fishing Time Reduction"));
        assert!(result_html.contains("fishy-calculator-pet-option__badges"));
        assert!(result_html.contains("+5% Item DRR"));
        assert!(result_html.contains("Special: Auto-Fishing Time Reduction"));
    }

    #[test]
    fn render_pet_talent_badges_uses_abbreviated_item_drr_badge() {
        let html = render_pet_talent_badges(
            CalculatorLocale::EnUs,
            &CalculatorPetOptionEntry {
                key: "durability_reduction_resistance".to_string(),
                label: "Durability Reduction Resistance +5%".to_string(),
                durability_reduction_resistance: Some(0.05),
                ..CalculatorPetOptionEntry::default()
            },
        );

        assert!(html.contains("+5% Item DRR"));
        assert!(html.contains("title=\"+5% Item DRR\""));
        assert!(!html.contains("Durability Reduction Resistance +5%"));
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
                    "packLeader": "true",
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
        assert!(parsed.pet1.pack_leader);
    }

    #[test]
    fn normalize_signals_keeps_cleared_food_and_buff_arrays_empty() {
        let mut parsed = parse_calculator_signals_value(
            serde_json::json!({
                "food": {
                    "0": ""
                },
                "buff": {
                    "0": ""
                }
            }),
            &CalculatorSignals::default(),
            &RequestId("req-test".to_string()),
        )
        .expect("cleared food and buff arrays should stay empty");

        let data = CalculatorData {
            catalog: CalculatorCatalogResponse {
                items: vec![
                    CalculatorItemEntry {
                        key: "item:9359".to_string(),
                        name: "Balacs Lunchbox".to_string(),
                        r#type: "food".to_string(),
                        buff_category_key: Some("buff-category:1".to_string()),
                        ..CalculatorItemEntry::default()
                    },
                    CalculatorItemEntry {
                        key: "item:721092".to_string(),
                        name: "Treant's Tear".to_string(),
                        r#type: "buff".to_string(),
                        buff_category_key: Some("buff-category:6".to_string()),
                        ..CalculatorItemEntry::default()
                    },
                ],
                defaults: CalculatorSignals {
                    food: vec!["item:9359".to_string()],
                    buff: vec!["".to_string(), "item:721092".to_string()],
                    ..CalculatorSignals::default()
                },
                ..CalculatorCatalogResponse::default()
            },
            cdn_base_url: "http://127.0.0.1:4040".to_string(),
            lang: CalculatorLocale::EnUs,
            api_lang: DataLang::En,
            zones: Vec::new(),
            zone_group_rates: HashMap::new(),
            zone_loot_entries: Vec::new(),
        };

        normalize_signals(&mut parsed, &data);

        assert!(parsed.food.is_empty());
        assert!(parsed.buff.is_empty());
    }

    #[test]
    fn normalize_signals_clamps_mastery_to_slider_range() {
        let mut parsed = parse_calculator_signals_value(
            serde_json::json!({
                "mastery": 3200
            }),
            &CalculatorSignals::default(),
            &RequestId("req-test".to_string()),
        )
        .expect("mastery should parse");

        let data = CalculatorData {
            catalog: CalculatorCatalogResponse::default(),
            cdn_base_url: "http://127.0.0.1:4040".to_string(),
            lang: CalculatorLocale::EnUs,
            api_lang: DataLang::En,
            zones: Vec::new(),
            zone_group_rates: HashMap::new(),
            zone_loot_entries: Vec::new(),
        };

        normalize_signals(&mut parsed, &data);

        assert_eq!(parsed.mastery, 3000.0);
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
    fn loot_species_evidence_text_includes_db_guess_and_presence() {
        let normalized_signals = CalculatorSignals {
            show_normalized_select_rates: true,
            ..CalculatorSignals::default()
        };
        let raw_signals = CalculatorSignals {
            show_normalized_select_rates: false,
            ..CalculatorSignals::default()
        };
        let entry = CalculatorZoneLootEntry {
            within_group_rate: 0.5,
            evidence: vec![
                CalculatorZoneLootEvidence {
                    source_family: "database".to_string(),
                    claim_kind: "in_group_rate".to_string(),
                    scope: "group".to_string(),
                    rate: Some(0.3),
                    normalized_rate: Some(0.25),
                    status: Some("best_effort".to_string()),
                    claim_count: None,
                    ..CalculatorZoneLootEvidence::default()
                },
                CalculatorZoneLootEvidence {
                    source_family: "community".to_string(),
                    claim_kind: "guessed_in_group_rate".to_string(),
                    scope: "group".to_string(),
                    rate: Some(0.02),
                    normalized_rate: Some(0.05),
                    status: Some("guessed".to_string()),
                    claim_count: None,
                    ..CalculatorZoneLootEvidence::default()
                },
                CalculatorZoneLootEvidence {
                    source_family: "community".to_string(),
                    claim_kind: "presence".to_string(),
                    scope: "zone".to_string(),
                    rate: None,
                    normalized_rate: None,
                    status: Some("confirmed".to_string()),
                    claim_count: Some(1),
                    ..CalculatorZoneLootEvidence::default()
                },
            ],
            ..CalculatorZoneLootEntry::default()
        };

        assert_eq!(
            loot_species_evidence_text(&normalized_signals, &entry, CalculatorLocale::EnUs),
            "DB 25% · Community guess 5% · Community confirmed×1 · zone-only"
        );
        assert_eq!(
            loot_species_evidence_text(&raw_signals, &entry, CalculatorLocale::EnUs),
            "DB 30% · Community guess 2% · Community confirmed×1 · zone-only"
        );
    }

    #[test]
    fn loot_species_evidence_text_handles_community_guess_without_db_rate() {
        let normalized_signals = CalculatorSignals {
            show_normalized_select_rates: true,
            ..CalculatorSignals::default()
        };
        let raw_signals = CalculatorSignals {
            show_normalized_select_rates: false,
            ..CalculatorSignals::default()
        };
        let entry = CalculatorZoneLootEntry {
            within_group_rate: 0.02,
            evidence: vec![CalculatorZoneLootEvidence {
                source_family: "community".to_string(),
                claim_kind: "guessed_in_group_rate".to_string(),
                scope: "group".to_string(),
                rate: Some(0.02),
                normalized_rate: Some(0.04651),
                status: Some("guessed".to_string()),
                claim_count: None,
                ..CalculatorZoneLootEvidence::default()
            }],
            ..CalculatorZoneLootEntry::default()
        };

        assert_eq!(
            loot_species_evidence_text(&normalized_signals, &entry, CalculatorLocale::EnUs),
            "Community guess 4.65%"
        );
        assert_eq!(
            loot_species_evidence_text(&raw_signals, &entry, CalculatorLocale::EnUs),
            "Community guess 2%"
        );
    }

    #[test]
    fn loot_species_evidence_text_shows_explicit_subgroup_scope_when_present() {
        let raw_signals = CalculatorSignals {
            show_normalized_select_rates: false,
            ..CalculatorSignals::default()
        };
        let entry = CalculatorZoneLootEntry {
            within_group_rate: 0.02,
            evidence: vec![CalculatorZoneLootEvidence {
                source_family: "community".to_string(),
                claim_kind: "presence".to_string(),
                scope: "subgroup".to_string(),
                rate: None,
                normalized_rate: None,
                status: Some("confirmed".to_string()),
                claim_count: Some(2),
                source_id: Some("community_presence_sheet".to_string()),
                slot_idx: Some(1),
                subgroup_key: Some(11054),
                ..CalculatorZoneLootEvidence::default()
            }],
            ..CalculatorZoneLootEntry::default()
        };

        assert_eq!(
            loot_species_evidence_text(&raw_signals, &entry, CalculatorLocale::EnUs),
            "Community confirmed×2 · Prize subgroup 11054 · source community_presence_sheet"
        );
        assert_eq!(
            loot_species_presence_text(&entry, CalculatorLocale::EnUs),
            Some("Community confirmed×2 · Prize subgroup".to_string())
        );
    }

    #[test]
    fn loot_species_presence_prefers_full_ranking_ring_and_marks_mixed_sources() {
        let entry = CalculatorZoneLootEntry {
            within_group_rate: 0.02,
            evidence: vec![
                CalculatorZoneLootEvidence {
                    source_family: "community".to_string(),
                    claim_kind: "presence".to_string(),
                    scope: "group".to_string(),
                    status: Some("confirmed".to_string()),
                    claim_count: Some(2),
                    source_id: Some("community_zone_fish_support".to_string()),
                    slot_idx: Some(1),
                    item_main_group_key: Some(11056),
                    ..CalculatorZoneLootEvidence::default()
                },
                CalculatorZoneLootEvidence {
                    source_family: "ranking".to_string(),
                    claim_kind: "presence".to_string(),
                    scope: "ring_partial".to_string(),
                    status: Some("observed".to_string()),
                    claim_count: Some(3),
                    source_id: Some("layer_revision:v1".to_string()),
                    ..CalculatorZoneLootEvidence::default()
                },
                CalculatorZoneLootEvidence {
                    source_family: "ranking".to_string(),
                    claim_kind: "presence".to_string(),
                    scope: "ring_full".to_string(),
                    status: Some("observed".to_string()),
                    claim_count: Some(8),
                    source_id: Some("layer_revision:v1".to_string()),
                    ..CalculatorZoneLootEvidence::default()
                },
            ],
            ..CalculatorZoneLootEntry::default()
        };

        assert_eq!(
            loot_species_presence_text(&entry, CalculatorLocale::EnUs),
            Some("Ranking ring fully inside zone ×8".to_string())
        );
        assert_eq!(loot_species_presence_source_kind(&entry), "mixed");
        assert_eq!(
            loot_species_presence_tooltip(&entry, CalculatorLocale::EnUs),
            Some(
                "Ranking ring fully inside zone ×8 · source layer_revision:v1 | Community confirmed×2 · Prize group 11056 · source community_zone_fish_support | Ranking ring overlaps zone edge ×3 · source layer_revision:v1".to_string()
            )
        );
    }

    #[test]
    fn loot_species_evidence_text_preserves_tiny_non_zero_rates() {
        let raw_signals = CalculatorSignals {
            show_normalized_select_rates: false,
            ..CalculatorSignals::default()
        };
        let entry = CalculatorZoneLootEntry {
            within_group_rate: 0.0000005,
            evidence: vec![CalculatorZoneLootEvidence {
                source_family: "database".to_string(),
                claim_kind: "in_group_rate".to_string(),
                scope: "group".to_string(),
                rate: Some(0.0000005),
                normalized_rate: Some(0.0000005),
                status: Some("best_effort".to_string()),
                claim_count: None,
                ..CalculatorZoneLootEvidence::default()
            }],
            ..CalculatorZoneLootEntry::default()
        };

        assert_eq!(
            loot_species_evidence_text(&raw_signals, &entry, CalculatorLocale::EnUs),
            "DB 0.00005%"
        );
    }

    #[test]
    fn overlay_editor_signal_preserves_tiny_non_zero_percent_text() {
        let signals = CalculatorSignals {
            zone: "240,74,74".to_string(),
            ..CalculatorSignals::default()
        };
        let fish_group_chart = FishGroupChart {
            available: true,
            note: String::new(),
            raw_prize_rate_text: "0%".to_string(),
            mastery_text: "0".to_string(),
            rows: vec![FishGroupChartRow {
                label: "Prize",
                fill_color: "pink",
                stroke_color: "red",
                text_color: "black",
                connector_color: "rgba(0,0,0,0.2)",
                bonus_text: String::new(),
                base_share_pct: 100.0,
                default_weight_pct: 100.0,
                weight_pct: 100.0,
                current_share_pct: 100.0,
                drop_rate_source_kind: String::new(),
                drop_rate_tooltip: String::new(),
                rate_inputs: Vec::new(),
            }],
        };
        let data = CalculatorData {
            catalog: CalculatorCatalogResponse::default(),
            cdn_base_url: "http://127.0.0.1:4040".to_string(),
            lang: CalculatorLocale::EnUs,
            api_lang: DataLang::En,
            zones: vec![ZoneEntry {
                rgb_key: fishystuff_api::ids::RgbKey("240,74,74".to_string()),
                name: Some("Velia Beach".to_string()),
                ..ZoneEntry::default()
            }],
            zone_group_rates: HashMap::new(),
            zone_loot_entries: vec![CalculatorZoneLootEntry {
                slot_idx: 1,
                item_id: 820001,
                name: "Tiny Fish".to_string(),
                within_group_rate: 0.0000005,
                evidence: vec![CalculatorZoneLootEvidence {
                    source_family: "database".to_string(),
                    claim_kind: "in_group_rate".to_string(),
                    scope: "group".to_string(),
                    rate: Some(0.0000005),
                    normalized_rate: Some(0.0000005),
                    ..CalculatorZoneLootEvidence::default()
                }],
                ..CalculatorZoneLootEntry::default()
            }],
        };

        let editor = super::build_overlay_editor_signal(&signals, &data, &fish_group_chart);

        assert_eq!(editor.zone_name, "Velia Beach");
        assert_eq!(editor.items.len(), 1);
        assert!((editor.items[0].default_raw_rate_pct - 0.00005).abs() < 1e-12);
        assert_eq!(editor.items[0].default_raw_rate_text, "0.00005%");
    }

    #[test]
    fn overlay_editor_signal_exposes_bonus_and_normalized_breakdowns() {
        let mut signals = CalculatorSignals {
            zone: "overlay_group_zone".to_string(),
            ..CalculatorSignals::default()
        };
        signals.overlay.zones.insert(
            "overlay_group_zone".to_string(),
            fishystuff_api::models::calculator::CalculatorZoneOverlaySignals {
                groups: std::collections::BTreeMap::from([(
                    "2".to_string(),
                    fishystuff_api::models::calculator::CalculatorZoneGroupOverlaySignals {
                        present: Some(true),
                        raw_rate_percent: Some(12.0),
                    },
                )]),
                items: std::collections::BTreeMap::new(),
            },
        );
        let fish_group_chart = FishGroupChart {
            available: true,
            note: String::new(),
            raw_prize_rate_text: "0%".to_string(),
            mastery_text: "0".to_string(),
            rows: vec![
                FishGroupChartRow {
                    label: "Prize",
                    fill_color: "pink",
                    stroke_color: "red",
                    text_color: "black",
                    connector_color: "rgba(0,0,0,0.2)",
                    bonus_text: String::new(),
                    base_share_pct: 0.0,
                    default_weight_pct: 0.0,
                    weight_pct: 0.0,
                    current_share_pct: 0.0,
                    drop_rate_source_kind: String::new(),
                    drop_rate_tooltip: String::new(),
                    rate_inputs: Vec::new(),
                },
                FishGroupChartRow {
                    label: "Rare",
                    fill_color: "yellow",
                    stroke_color: "gold",
                    text_color: "black",
                    connector_color: "rgba(0,0,0,0.2)",
                    bonus_text: "+3% Rare".to_string(),
                    base_share_pct: 10.0,
                    default_weight_pct: 13.0,
                    weight_pct: 15.0,
                    current_share_pct: 30.0,
                    drop_rate_source_kind: "overlay".to_string(),
                    drop_rate_tooltip: "Overlay-adjusted rare rate".to_string(),
                    rate_inputs: vec![
                        super::computed_stat_breakdown_row(
                            "Personal overlay raw base rate",
                            "12%",
                            "Editable raw base group rate before bonuses.",
                        ),
                        super::computed_stat_breakdown_row(
                            "Rare bonus sources",
                            "3%",
                            "Accrued bonus from active effects.",
                        ),
                    ],
                },
            ],
        };
        let data = CalculatorData {
            catalog: CalculatorCatalogResponse::default(),
            cdn_base_url: "http://127.0.0.1:4040".to_string(),
            lang: CalculatorLocale::EnUs,
            api_lang: DataLang::En,
            zones: vec![ZoneEntry {
                rgb_key: fishystuff_api::ids::RgbKey("overlay_group_zone".to_string()),
                name: Some("Overlay Bay".to_string()),
                ..ZoneEntry::default()
            }],
            zone_group_rates: HashMap::new(),
            zone_loot_entries: Vec::new(),
        };

        let editor = super::build_overlay_editor_signal(&signals, &data, &fish_group_chart);
        let rare_row = &editor.groups[1];

        assert_eq!(rare_row.current_raw_rate_pct, 12.0);
        assert_eq!(rare_row.current_raw_rate_text, "12%");
        assert_eq!(rare_row.bonus_rate_pct, 3.0);
        assert_eq!(rare_row.bonus_rate_text, "3%");
        assert_eq!(rare_row.effective_raw_weight_pct, 15.0);
        assert_eq!(rare_row.effective_raw_weight_text, "15%");
        assert_eq!(rare_row.normalized_share_pct, 30.0);
        assert_eq!(rare_row.normalized_share_text, "30%");

        let bonus_breakdown: Value =
            serde_json::from_str(&rare_row.bonus_rate_breakdown).expect("bonus breakdown json");
        assert_eq!(bonus_breakdown["title"], "Rare accrued bonus");
        assert_eq!(bonus_breakdown["value_text"], "3%");
        assert_eq!(
            bonus_breakdown["formula_text"],
            "Accrued bonus = Effective raw weight - Current raw base rate."
        );

        let normalized_breakdown: Value =
            serde_json::from_str(&rare_row.normalized_share_breakdown)
                .expect("normalized breakdown json");
        assert_eq!(normalized_breakdown["title"], "Rare normalized share");
        assert_eq!(normalized_breakdown["value_text"], "30%");
        assert_eq!(
            normalized_breakdown["formula_text"],
            "Normalized share = Effective raw weight / All effective raw weights."
        );
    }

    #[test]
    fn loot_flow_filter_excludes_groups_without_share_or_derived_species_rows() {
        let signals = CalculatorSignals::default();
        let fish_group_chart = FishGroupChart {
            available: true,
            note: String::new(),
            raw_prize_rate_text: "5%".to_string(),
            mastery_text: "3000".to_string(),
            rows: vec![
                FishGroupChartRow {
                    label: "Prize",
                    fill_color: "pink",
                    stroke_color: "red",
                    text_color: "black",
                    connector_color: "rgba(0,0,0,0.2)",
                    bonus_text: String::new(),
                    base_share_pct: 0.0,
                    default_weight_pct: 0.0,
                    weight_pct: 0.0,
                    current_share_pct: 10.0,
                    drop_rate_source_kind: String::new(),
                    drop_rate_tooltip: String::new(),
                    rate_inputs: Vec::new(),
                },
                FishGroupChartRow {
                    label: "Rare",
                    fill_color: "yellow",
                    stroke_color: "gold",
                    text_color: "black",
                    connector_color: "rgba(0,0,0,0.2)",
                    bonus_text: String::new(),
                    base_share_pct: 0.0,
                    default_weight_pct: 0.0,
                    weight_pct: 0.0,
                    current_share_pct: 0.0,
                    drop_rate_source_kind: String::new(),
                    drop_rate_tooltip: String::new(),
                    rate_inputs: Vec::new(),
                },
                FishGroupChartRow {
                    label: "General",
                    fill_color: "green",
                    stroke_color: "lime",
                    text_color: "black",
                    connector_color: "rgba(0,0,0,0.2)",
                    bonus_text: String::new(),
                    base_share_pct: 0.0,
                    default_weight_pct: 0.0,
                    weight_pct: 0.0,
                    current_share_pct: 90.0,
                    drop_rate_source_kind: String::new(),
                    drop_rate_tooltip: String::new(),
                    rate_inputs: Vec::new(),
                },
            ],
        };
        let data = CalculatorData {
            catalog: CalculatorCatalogResponse::default(),
            cdn_base_url: "http://127.0.0.1:4040".to_string(),
            lang: CalculatorLocale::EnUs,
            api_lang: DataLang::En,
            zones: Vec::new(),
            zone_group_rates: HashMap::new(),
            zone_loot_entries: vec![CalculatorZoneLootEntry {
                slot_idx: 1,
                item_id: 820985,
                name: "Silver Beltfish".to_string(),
                vendor_price: Some(80_000_000),
                within_group_rate: 1.0,
                evidence: vec![CalculatorZoneLootEvidence {
                    source_family: "community".to_string(),
                    claim_kind: "guessed_in_group_rate".to_string(),
                    scope: "group".to_string(),
                    rate: Some(0.02),
                    normalized_rate: Some(0.04651),
                    status: Some("guessed".to_string()),
                    claim_count: None,
                    ..CalculatorZoneLootEvidence::default()
                }],
                ..CalculatorZoneLootEntry::default()
            }],
        };

        let loot_chart = derive_loot_chart(&signals, &data, &fish_group_chart, 100.0, 1.0);
        let loot_flow_rows = filtered_loot_flow_rows(&loot_chart.rows, &loot_chart.species_rows);

        assert_eq!(loot_chart.rows.len(), 3);
        assert_eq!(loot_chart.species_rows.len(), 1);
        assert_eq!(loot_chart.species_rows[0].group_label, "Prize");
        assert_eq!(loot_chart.species_rows[0].label, "Silver Beltfish");
        assert_eq!(loot_flow_rows.len(), 1);
        assert_eq!(loot_flow_rows[0].label, "Prize");
    }

    #[test]
    fn derive_loot_chart_skips_presence_only_rows() {
        let signals = CalculatorSignals::default();
        let fish_group_chart = FishGroupChart {
            available: true,
            note: String::new(),
            raw_prize_rate_text: "0%".to_string(),
            mastery_text: "0".to_string(),
            rows: vec![FishGroupChartRow {
                label: "General",
                fill_color: "green",
                stroke_color: "lime",
                text_color: "black",
                connector_color: "rgba(0,0,0,0.2)",
                bonus_text: String::new(),
                base_share_pct: 0.0,
                default_weight_pct: 0.0,
                weight_pct: 0.0,
                current_share_pct: 100.0,
                drop_rate_source_kind: String::new(),
                drop_rate_tooltip: String::new(),
                rate_inputs: Vec::new(),
            }],
        };
        let data = CalculatorData {
            catalog: CalculatorCatalogResponse::default(),
            cdn_base_url: "http://127.0.0.1:4040".to_string(),
            lang: CalculatorLocale::EnUs,
            api_lang: DataLang::En,
            zones: Vec::new(),
            zone_group_rates: HashMap::new(),
            zone_loot_entries: vec![
                CalculatorZoneLootEntry {
                    slot_idx: 1,
                    item_id: 820001,
                    name: "Weighted Fish".to_string(),
                    within_group_rate: 1.0,
                    ..CalculatorZoneLootEntry::default()
                },
                CalculatorZoneLootEntry {
                    slot_idx: 1,
                    item_id: 820002,
                    name: "Presence Only Fish".to_string(),
                    within_group_rate: 0.0,
                    evidence: vec![CalculatorZoneLootEvidence {
                        source_family: "community".to_string(),
                        claim_kind: "presence".to_string(),
                        scope: "group".to_string(),
                        status: Some("confirmed".to_string()),
                        claim_count: Some(1),
                        ..CalculatorZoneLootEvidence::default()
                    }],
                    ..CalculatorZoneLootEntry::default()
                },
            ],
        };

        let loot_chart = derive_loot_chart(&signals, &data, &fish_group_chart, 100.0, 1.0);

        assert_eq!(loot_chart.species_rows.len(), 1);
        assert_eq!(loot_chart.species_rows[0].label, "Weighted Fish");
    }

    #[test]
    fn zone_loot_summary_keeps_unassigned_presence_only_rows_visible() {
        let signals = CalculatorSignals {
            zone: "lake_flondor".to_string(),
            ..CalculatorSignals::default()
        };
        let data = CalculatorData {
            catalog: CalculatorCatalogResponse::default(),
            cdn_base_url: "http://127.0.0.1:4040".to_string(),
            lang: CalculatorLocale::EnUs,
            api_lang: DataLang::En,
            zones: Vec::new(),
            zone_group_rates: HashMap::new(),
            zone_loot_entries: vec![CalculatorZoneLootEntry {
                slot_idx: 0,
                item_id: 820986,
                name: "Pink Dolphin".to_string(),
                evidence: vec![CalculatorZoneLootEvidence {
                    source_family: "community".to_string(),
                    claim_kind: "presence".to_string(),
                    scope: "zone".to_string(),
                    status: Some("confirmed".to_string()),
                    claim_count: Some(1),
                    source_id: Some("manual_community_zone_fish_presence".to_string()),
                    ..CalculatorZoneLootEvidence::default()
                }],
                ..CalculatorZoneLootEntry::default()
            }],
        };
        let zone = ZoneEntry {
            name: Some("Lake Flondor".to_string()),
            ..ZoneEntry::default()
        };

        let summary = derive_zone_loot_summary_response(&signals, &data, &zone);

        assert!(summary.available);
        assert_eq!(summary.groups.len(), 1);
        assert_eq!(summary.groups[0].slot_idx, 0);
        assert_eq!(summary.groups[0].label, "Unassigned");
        assert_eq!(summary.groups[0].drop_rate_text, "");
        assert_eq!(summary.groups[0].drop_rate_source_kind, "");
        assert_eq!(summary.groups[0].condition_text, "");
        assert_eq!(summary.species_rows.len(), 1);
        assert_eq!(summary.species_rows[0].group_label, "Unassigned");
        assert_eq!(summary.species_rows[0].drop_rate_text, "");
        assert_eq!(
            summary.species_rows[0].catch_methods,
            vec!["rod".to_string()]
        );
        assert!(summary.species_rows[0].presence_text.is_some());
        assert!(summary
            .data_quality_note
            .contains("Expected loot uses average session casts"));
        assert!(summary.note.contains("Unassigned"));
    }

    #[test]
    fn zone_loot_summary_keeps_group_presence_only_rows_visible() {
        let signals = CalculatorSignals {
            zone: "edania_longing_lake".to_string(),
            ..CalculatorSignals::default()
        };
        let data = CalculatorData {
            catalog: CalculatorCatalogResponse::default(),
            cdn_base_url: "http://127.0.0.1:4040".to_string(),
            lang: CalculatorLocale::EnUs,
            api_lang: DataLang::En,
            zones: Vec::new(),
            zone_group_rates: HashMap::from([(
                "edania_longing_lake".to_string(),
                CalculatorZoneGroupRateEntry {
                    zone_rgb_key: "0,0,0".to_string(),
                    prize_main_group_key: None,
                    rare_rate_raw: 0,
                    high_quality_rate_raw: 0,
                    general_rate_raw: 1_000_000,
                    trash_rate_raw: 0,
                },
            )]),
            zone_loot_entries: vec![CalculatorZoneLootEntry {
                slot_idx: 4,
                item_id: 800123,
                name: "Leaffish".to_string(),
                evidence: vec![CalculatorZoneLootEvidence {
                    source_family: "community".to_string(),
                    claim_kind: "presence".to_string(),
                    scope: "group".to_string(),
                    status: Some("confirmed".to_string()),
                    claim_count: Some(1),
                    slot_idx: Some(4),
                    item_main_group_key: Some(9001),
                    ..CalculatorZoneLootEvidence::default()
                }],
                ..CalculatorZoneLootEntry::default()
            }],
        };
        let zone = ZoneEntry {
            name: Some("Edania - Longing Lake".to_string()),
            ..ZoneEntry::default()
        };

        let summary = derive_zone_loot_summary_response(&signals, &data, &zone);

        assert!(summary.available);
        assert_eq!(summary.groups.len(), 1);
        assert_eq!(summary.groups[0].slot_idx, 4);
        assert_eq!(summary.groups[0].label, "General");
        assert_eq!(summary.groups[0].drop_rate_text, "100%");
        assert_eq!(summary.groups[0].drop_rate_source_kind, "database");
        assert_eq!(summary.groups[0].condition_text, "Zone base rate 100%");
        assert_eq!(summary.species_rows.len(), 1);
        assert_eq!(summary.species_rows[0].group_label, "General");
        assert_eq!(summary.species_rows[0].drop_rate_text, "");
        assert_eq!(
            summary.species_rows[0].catch_methods,
            vec!["rod".to_string()]
        );
        assert!(summary.note.contains("stay visible"));
    }

    #[test]
    fn zone_loot_summary_preserves_harpoon_methods_for_species_rows() {
        let signals = CalculatorSignals {
            zone: "valencia_depth_5".to_string(),
            ..CalculatorSignals::default()
        };
        let data = CalculatorData {
            catalog: CalculatorCatalogResponse::default(),
            cdn_base_url: "http://127.0.0.1:4040".to_string(),
            lang: CalculatorLocale::EnUs,
            api_lang: DataLang::En,
            zones: Vec::new(),
            zone_group_rates: HashMap::from([(
                "valencia_depth_5".to_string(),
                CalculatorZoneGroupRateEntry {
                    zone_rgb_key: "0,0,0".to_string(),
                    prize_main_group_key: None,
                    rare_rate_raw: 0,
                    high_quality_rate_raw: 0,
                    general_rate_raw: 1_000_000,
                    trash_rate_raw: 0,
                },
            )]),
            zone_loot_entries: vec![CalculatorZoneLootEntry {
                slot_idx: 4,
                item_id: 820115,
                name: "Mako Shark".to_string(),
                catch_methods: vec!["harpoon".to_string()],
                evidence: vec![CalculatorZoneLootEvidence {
                    source_family: "community".to_string(),
                    claim_kind: "presence".to_string(),
                    scope: "group".to_string(),
                    status: Some("confirmed".to_string()),
                    claim_count: Some(1),
                    slot_idx: Some(4),
                    ..CalculatorZoneLootEvidence::default()
                }],
                ..CalculatorZoneLootEntry::default()
            }],
        };
        let zone = ZoneEntry {
            name: Some("Valencia Sea - Depth 5".to_string()),
            ..ZoneEntry::default()
        };

        let summary = derive_zone_loot_summary_response(&signals, &data, &zone);

        assert_eq!(
            summary.species_rows[0].catch_methods,
            vec!["harpoon".to_string()]
        );
    }

    #[test]
    fn zone_loot_summary_humanizes_database_group_conditions() {
        let signals = CalculatorSignals {
            zone: "margoria_harpoon".to_string(),
            ..CalculatorSignals::default()
        };
        let data = CalculatorData {
            catalog: CalculatorCatalogResponse {
                lifeskill_levels: vec![
                    CalculatorLifeskillLevelEntry {
                        key: "35".to_string(),
                        name: "Professional 5".to_string(),
                        index: 35,
                        order: 35,
                        lifeskill_level_drr: 0.0,
                    },
                    CalculatorLifeskillLevelEntry {
                        key: "81".to_string(),
                        name: "Guru 1".to_string(),
                        index: 81,
                        order: 81,
                        lifeskill_level_drr: 0.0,
                    },
                ],
                ..CalculatorCatalogResponse::default()
            },
            cdn_base_url: "http://127.0.0.1:4040".to_string(),
            lang: CalculatorLocale::EnUs,
            api_lang: DataLang::En,
            zones: Vec::new(),
            zone_group_rates: HashMap::new(),
            zone_loot_entries: vec![CalculatorZoneLootEntry {
                slot_idx: 6,
                item_id: 820115,
                name: "Mako Shark".to_string(),
                catch_methods: vec!["harpoon".to_string()],
                group_conditions_raw: vec![
                    "lifestat(1,1)>199;lifestat(1,1)<700;".to_string(),
                    "lifestat(1,1)>699;lifestat(1,1)<1200;".to_string(),
                    "lifestat(1,1)>1199;".to_string(),
                    "getLifeLevel(1)>80;".to_string(),
                ],
                evidence: vec![CalculatorZoneLootEvidence {
                    source_family: "community".to_string(),
                    claim_kind: "presence".to_string(),
                    scope: "group".to_string(),
                    status: Some("confirmed".to_string()),
                    claim_count: Some(1),
                    slot_idx: Some(6),
                    ..CalculatorZoneLootEvidence::default()
                }],
                ..CalculatorZoneLootEntry::default()
            }],
        };
        let zone = ZoneEntry {
            name: Some("Margoria Harpoon".to_string()),
            ..ZoneEntry::default()
        };

        let summary = derive_zone_loot_summary_response(&signals, &data, &zone);

        assert_eq!(summary.groups.len(), 1);
        assert_eq!(summary.groups[0].label, "Harpoon");
        assert_eq!(
            summary.groups[0].condition_text,
            "Mastery 200-699 · Mastery 700-1199 · Mastery 1200+ · Fishing Level Guru 1+"
        );
        assert_eq!(summary.groups[0].catch_methods, vec!["harpoon".to_string()]);
    }

    #[test]
    fn loot_condition_context_filters_guru_locked_rate_contributions() {
        let base_entries = vec![
            CalculatorZoneLootEntry {
                slot_idx: 2,
                item_id: 8201,
                name: "Regular rare".to_string(),
                within_group_rate: 0.99995,
                evidence: vec![CalculatorZoneLootEvidence {
                    source_family: "database".to_string(),
                    claim_kind: "in_group_rate".to_string(),
                    scope: "group".to_string(),
                    ..CalculatorZoneLootEvidence::default()
                }],
                rate_contributions: vec![CalculatorZoneLootRateContribution {
                    source_family: "database".to_string(),
                    subgroup_key: Some(12001),
                    weight: 999_950_000_000.0,
                    ..CalculatorZoneLootRateContribution::default()
                }],
                ..CalculatorZoneLootEntry::default()
            },
            CalculatorZoneLootEntry {
                slot_idx: 2,
                item_id: 820985,
                name: "Mystical fish".to_string(),
                within_group_rate: 0.00005,
                evidence: vec![CalculatorZoneLootEvidence {
                    source_family: "database".to_string(),
                    claim_kind: "in_group_rate".to_string(),
                    scope: "group".to_string(),
                    ..CalculatorZoneLootEvidence::default()
                }],
                rate_contributions: vec![CalculatorZoneLootRateContribution {
                    source_family: "database".to_string(),
                    subgroup_key: Some(12002),
                    group_conditions_raw: vec!["getLifeLevel(1)>80;".to_string()],
                    weight: 50_000_000.0,
                    ..CalculatorZoneLootRateContribution::default()
                }],
                ..CalculatorZoneLootEntry::default()
            },
        ];

        let below_guru = CalculatorSignals {
            lifeskill_level: "80".to_string(),
            ..CalculatorSignals::default()
        };
        let below_entries =
            apply_calculator_condition_context_to_loot_entries(&below_guru, &base_entries);

        assert_eq!(below_entries.len(), 1);
        assert_eq!(below_entries[0].item_id, 8201);
        assert!((below_entries[0].within_group_rate - 1.0).abs() < 1e-12);

        let guru_one = CalculatorSignals {
            lifeskill_level: "81".to_string(),
            ..CalculatorSignals::default()
        };
        let guru_entries =
            apply_calculator_condition_context_to_loot_entries(&guru_one, &base_entries);
        let mystical = guru_entries
            .iter()
            .find(|entry| entry.item_id == 820985)
            .expect("Guru 1 should activate mystical fish contribution");

        assert!((mystical.within_group_rate - 0.00005).abs() < 1e-12);
        assert_eq!(
            mystical.group_conditions_raw,
            vec!["getLifeLevel(1)>80;".to_string()]
        );
        assert_eq!(
            mystical.evidence[0].rate,
            Some(0.00005),
            "raw DB rate remains non-normalized"
        );
    }

    #[test]
    fn loot_condition_context_treats_main_group_options_as_user_state_branches() {
        let base_entries = vec![
            CalculatorZoneLootEntry {
                slot_idx: 2,
                item_id: 8261,
                name: "Grunt".to_string(),
                evidence: vec![CalculatorZoneLootEvidence {
                    source_family: "database".to_string(),
                    claim_kind: "in_group_rate".to_string(),
                    scope: "group".to_string(),
                    ..CalculatorZoneLootEvidence::default()
                }],
                rate_contributions: vec![
                    CalculatorZoneLootRateContribution {
                        source_family: "database".to_string(),
                        item_main_group_key: Some(10990),
                        option_idx: Some(0),
                        subgroup_key: Some(11152),
                        group_conditions_raw: vec!["getLifeLevel(1)>80;".to_string()],
                        weight: 473_500_000_000.0,
                    },
                    CalculatorZoneLootRateContribution {
                        source_family: "database".to_string(),
                        item_main_group_key: Some(10990),
                        option_idx: Some(1),
                        subgroup_key: Some(10990),
                        weight: 473_500_000_000.0,
                        ..CalculatorZoneLootRateContribution::default()
                    },
                ],
                ..CalculatorZoneLootEntry::default()
            },
            CalculatorZoneLootEntry {
                slot_idx: 2,
                item_id: 42281,
                name: "Mystical fish".to_string(),
                evidence: vec![CalculatorZoneLootEvidence {
                    source_family: "database".to_string(),
                    claim_kind: "in_group_rate".to_string(),
                    scope: "group".to_string(),
                    ..CalculatorZoneLootEvidence::default()
                }],
                rate_contributions: vec![CalculatorZoneLootRateContribution {
                    source_family: "database".to_string(),
                    item_main_group_key: Some(10990),
                    option_idx: Some(0),
                    subgroup_key: Some(11152),
                    group_conditions_raw: vec!["getLifeLevel(1)>80;".to_string()],
                    weight: 50_000_000.0,
                }],
                ..CalculatorZoneLootEntry::default()
            },
        ];

        let guru_one = CalculatorSignals {
            lifeskill_level: "81".to_string(),
            ..CalculatorSignals::default()
        };
        let guru_entries =
            apply_calculator_condition_context_to_loot_entries(&guru_one, &base_entries);
        let grunt = guru_entries
            .iter()
            .find(|entry| entry.item_id == 8261)
            .expect("Guru branch should include Grunt once");
        let mystical = guru_entries
            .iter()
            .find(|entry| entry.item_id == 42281)
            .expect("Guru branch should include Mystical fish");

        assert!((grunt.within_group_rate - 0.999894414).abs() < 1e-9);
        assert!((mystical.within_group_rate - 0.000105585).abs() < 1e-9);
        assert_eq!(grunt.rate_contributions.len(), 1);
        assert_eq!(grunt.rate_contributions[0].subgroup_key, Some(11152));
        assert_eq!(grunt.evidence[0].rate, Some(0.4735));
        assert_eq!(mystical.evidence[0].rate, Some(0.00005));

        let below_guru = CalculatorSignals {
            lifeskill_level: "80".to_string(),
            ..CalculatorSignals::default()
        };
        let below_entries =
            apply_calculator_condition_context_to_loot_entries(&below_guru, &base_entries);
        assert_eq!(below_entries.len(), 1);
        assert_eq!(below_entries[0].item_id, 8261);
        assert_eq!(
            below_entries[0].rate_contributions[0].subgroup_key,
            Some(10990)
        );
        assert!((below_entries[0].within_group_rate - 1.0).abs() < 1e-12);
    }

    #[test]
    fn zone_loot_summary_condition_options_include_inactive_scalar_branches() {
        let base_entries = vec![
            CalculatorZoneLootEntry {
                slot_idx: 2,
                item_id: 8261,
                name: "Grunt".to_string(),
                evidence: vec![CalculatorZoneLootEvidence {
                    source_family: "database".to_string(),
                    claim_kind: "in_group_rate".to_string(),
                    scope: "group".to_string(),
                    ..CalculatorZoneLootEvidence::default()
                }],
                rate_contributions: vec![
                    CalculatorZoneLootRateContribution {
                        source_family: "database".to_string(),
                        item_main_group_key: Some(10990),
                        option_idx: Some(0),
                        subgroup_key: Some(11152),
                        group_conditions_raw: vec!["getLifeLevel(1)>80;".to_string()],
                        weight: 473_500_000_000.0,
                    },
                    CalculatorZoneLootRateContribution {
                        source_family: "database".to_string(),
                        item_main_group_key: Some(10990),
                        option_idx: Some(1),
                        subgroup_key: Some(10990),
                        weight: 473_500_000_000.0,
                        ..CalculatorZoneLootRateContribution::default()
                    },
                ],
                ..CalculatorZoneLootEntry::default()
            },
            CalculatorZoneLootEntry {
                slot_idx: 2,
                item_id: 42281,
                name: "Mystical fish".to_string(),
                evidence: vec![CalculatorZoneLootEvidence {
                    source_family: "database".to_string(),
                    claim_kind: "in_group_rate".to_string(),
                    scope: "group".to_string(),
                    ..CalculatorZoneLootEvidence::default()
                }],
                rate_contributions: vec![CalculatorZoneLootRateContribution {
                    source_family: "database".to_string(),
                    item_main_group_key: Some(10990),
                    option_idx: Some(0),
                    subgroup_key: Some(11152),
                    group_conditions_raw: vec!["getLifeLevel(1)>80;".to_string()],
                    weight: 50_000_000.0,
                }],
                ..CalculatorZoneLootEntry::default()
            },
        ];
        let signals = CalculatorSignals {
            zone: "velia_event".to_string(),
            lifeskill_level: "80".to_string(),
            ..CalculatorSignals::default()
        };
        let active_entries =
            apply_calculator_condition_context_to_loot_entries(&signals, &base_entries);
        let data = CalculatorData {
            catalog: CalculatorCatalogResponse {
                lifeskill_levels: vec![CalculatorLifeskillLevelEntry {
                    key: "81".to_string(),
                    name: "Guru 1".to_string(),
                    index: 81,
                    order: 81,
                    lifeskill_level_drr: 0.0,
                }],
                ..CalculatorCatalogResponse::default()
            },
            cdn_base_url: "http://127.0.0.1:4040".to_string(),
            lang: CalculatorLocale::EnUs,
            api_lang: DataLang::En,
            zones: Vec::new(),
            zone_group_rates: HashMap::from([(
                "velia_event".to_string(),
                CalculatorZoneGroupRateEntry {
                    zone_rgb_key: "0,0,0".to_string(),
                    prize_main_group_key: None,
                    rare_rate_raw: 1_000_000,
                    high_quality_rate_raw: 0,
                    general_rate_raw: 0,
                    trash_rate_raw: 0,
                },
            )]),
            zone_loot_entries: active_entries,
        };
        let zone = ZoneEntry {
            name: Some("Velia Event".to_string()),
            ..ZoneEntry::default()
        };
        let condition_options_by_slot =
            build_zone_loot_summary_condition_options(&signals, &data, &base_entries);
        let summary = derive_zone_loot_summary_response_with_condition_options(
            &signals,
            &data,
            &zone,
            &condition_options_by_slot,
        );
        let rare_group = summary
            .groups
            .iter()
            .find(|group| group.slot_idx == 2)
            .expect("rare group should be visible");

        assert_eq!(rare_group.condition_options.len(), 2);
        let active_option = rare_group
            .condition_options
            .iter()
            .find(|option| option.active)
            .expect("default branch should be active below Guru 1");
        assert_eq!(active_option.condition_text, "Default");
        assert_eq!(active_option.species_rows.len(), 1);
        assert_eq!(active_option.species_rows[0].label, "Grunt");
        let guru_option = rare_group
            .condition_options
            .iter()
            .find(|option| option.condition_text == "Fishing Level Guru 1+")
            .expect("Guru branch should remain selectable");
        assert_eq!(
            guru_option
                .species_rows
                .iter()
                .map(|row| row.label.as_str())
                .collect::<Vec<_>>(),
            vec!["Grunt", "Mystical fish"]
        );
    }

    #[test]
    fn loot_condition_context_applies_harpoon_mastery_brackets() {
        let base_entries = vec![
            CalculatorZoneLootEntry {
                slot_idx: 6,
                item_id: 820115,
                name: "Lower bracket harpoon fish".to_string(),
                rate_contributions: vec![CalculatorZoneLootRateContribution {
                    source_family: "database".to_string(),
                    item_main_group_key: Some(10901),
                    option_idx: Some(0),
                    subgroup_key: Some(10916),
                    group_conditions_raw: vec!["lifestat(1,1)>199;lifestat(1,1)<700;".to_string()],
                    weight: 1_000_000_000_000.0,
                }],
                ..CalculatorZoneLootEntry::default()
            },
            CalculatorZoneLootEntry {
                slot_idx: 6,
                item_id: 820116,
                name: "Middle bracket harpoon fish".to_string(),
                rate_contributions: vec![CalculatorZoneLootRateContribution {
                    source_family: "database".to_string(),
                    item_main_group_key: Some(10901),
                    option_idx: Some(1),
                    subgroup_key: Some(10917),
                    group_conditions_raw: vec!["lifestat(1,1)>699;lifestat(1,1)<1200;".to_string()],
                    weight: 1_000_000_000_000.0,
                }],
                ..CalculatorZoneLootEntry::default()
            },
            CalculatorZoneLootEntry {
                slot_idx: 6,
                item_id: 820117,
                name: "Lifeskill fallback harpoon fish".to_string(),
                rate_contributions: vec![CalculatorZoneLootRateContribution {
                    source_family: "database".to_string(),
                    item_main_group_key: Some(10901),
                    option_idx: Some(3),
                    subgroup_key: Some(10901),
                    group_conditions_raw: vec!["getLifeLevel(1)>34;".to_string()],
                    weight: 1_000_000_000_000.0,
                }],
                ..CalculatorZoneLootEntry::default()
            },
        ];

        let mastery_699 = CalculatorSignals {
            mastery: 699.0,
            lifeskill_level: "100".to_string(),
            ..CalculatorSignals::default()
        };
        let mastery_699_entries =
            apply_calculator_condition_context_to_loot_entries(&mastery_699, &base_entries);
        assert_eq!(
            mastery_699_entries
                .iter()
                .map(|entry| entry.item_id)
                .collect::<Vec<_>>(),
            vec![820115]
        );

        let mastery_700 = CalculatorSignals {
            mastery: 700.0,
            lifeskill_level: "100".to_string(),
            ..CalculatorSignals::default()
        };
        let mastery_700_entries =
            apply_calculator_condition_context_to_loot_entries(&mastery_700, &base_entries);
        assert_eq!(
            mastery_700_entries
                .iter()
                .map(|entry| entry.item_id)
                .collect::<Vec<_>>(),
            vec![820116]
        );

        let below_mastery = CalculatorSignals {
            mastery: 0.0,
            lifeskill_level: "35".to_string(),
            ..CalculatorSignals::default()
        };
        let below_mastery_entries =
            apply_calculator_condition_context_to_loot_entries(&below_mastery, &base_entries);
        assert_eq!(
            below_mastery_entries
                .iter()
                .map(|entry| entry.item_id)
                .collect::<Vec<_>>(),
            vec![820117]
        );
    }

    #[test]
    fn render_calculator_data_disclaimer_uses_persistent_notice_disclosure() {
        let html = super::render_calculator_data_disclaimer(CalculatorLocale::EnUs);

        assert!(html.contains("<fishy-notice-disclosure"));
        assert!(html.contains("title=\"Notice\""));
        assert!(html.contains("settings-path=\"calculator.noticeOpen\""));
        assert!(html.contains("open"));
        assert!(html.contains(">Data Quality Warning<"));
    }

    #[test]
    fn groups_distribution_segments_can_use_raw_group_rates() {
        let rows = vec![
            FishGroupChartRow {
                label: "Prize",
                fill_color: "pink",
                stroke_color: "red",
                text_color: "black",
                connector_color: "rgba(0,0,0,0.2)",
                bonus_text: String::new(),
                base_share_pct: 0.0,
                default_weight_pct: 0.0,
                weight_pct: 6.25,
                current_share_pct: 5.81,
                drop_rate_source_kind: String::new(),
                drop_rate_tooltip: String::new(),
                rate_inputs: Vec::new(),
            },
            FishGroupChartRow {
                label: "Trash",
                fill_color: "gray",
                stroke_color: "black",
                text_color: "black",
                connector_color: "rgba(0,0,0,0.2)",
                bonus_text: String::new(),
                base_share_pct: 6.25,
                default_weight_pct: 6.25,
                weight_pct: 6.25,
                current_share_pct: 5.81,
                drop_rate_source_kind: String::new(),
                drop_rate_tooltip: String::new(),
                rate_inputs: Vec::new(),
            },
        ];

        let normalized =
            super::groups_distribution_segments(&rows, 52.0, true, CalculatorLocale::EnUs);
        let raw = super::groups_distribution_segments(&rows, 52.0, false, CalculatorLocale::EnUs);

        assert_eq!(normalized[0].value_text, "5.81%");
        assert_eq!(normalized[0].detail_text, "3.02");
        assert_eq!(normalized[0].width_pct, 5.81);
        assert_eq!(
            normalized[0]
                .breakdown
                .as_ref()
                .expect("normalized group chart should expose a breakdown")
                .sections[1]
                .rows[0]
                .label,
            "Raw group weight"
        );

        assert_eq!(raw[0].value_text, "6.25%");
        assert_eq!(raw[0].detail_text, "3.02");
        assert_eq!(raw[0].width_pct, 6.25);
        assert!(raw[0]
            .breakdown
            .as_ref()
            .expect("raw group chart should expose a breakdown")
            .summary_text
            .contains("Raw group weight before normalization"));
        assert_eq!(raw[1].value_text, "6.25%");
        assert_eq!(raw[1].detail_text, "3.02");
    }

    #[test]
    fn group_silver_distribution_segments_expose_breakdowns() {
        let rows = vec![
            LootChartRow {
                label: "Prize",
                fill_color: "pink",
                stroke_color: "red",
                text_color: "black",
                connector_color: "rgba(0,0,0,0.2)",
                drop_rate_source_kind: "derived".to_string(),
                drop_rate_tooltip: "Derived from group share".to_string(),
                condition_text: String::new(),
                condition_tooltip: String::new(),
                expected_count_raw: 3.02,
                expected_profit_raw: 120_000.0,
                expected_count_text: "3.02".to_string(),
                expected_profit_text: "120,000".to_string(),
                current_share_pct: 5.81,
                count_share_text: "5.81%".to_string(),
                silver_share_text: "24.00%".to_string(),
                count_breakdown: String::new(),
                silver_breakdown: String::new(),
            },
            LootChartRow {
                label: "General",
                fill_color: "green",
                stroke_color: "lime",
                text_color: "black",
                connector_color: "rgba(0,0,0,0.2)",
                drop_rate_source_kind: "derived".to_string(),
                drop_rate_tooltip: "Derived from group share".to_string(),
                condition_text: String::new(),
                condition_tooltip: String::new(),
                expected_count_raw: 48.98,
                expected_profit_raw: 380_000.0,
                expected_count_text: "48.98".to_string(),
                expected_profit_text: "380,000".to_string(),
                current_share_pct: 94.19,
                count_share_text: "94.19%".to_string(),
                silver_share_text: "76.00%".to_string(),
                count_breakdown: String::new(),
                silver_breakdown: String::new(),
            },
        ];
        let species_rows = vec![
            LootSpeciesRow {
                slot_idx: 1,
                item_id: 820001,
                group_label: "Prize",
                label: "Golden Coelacanth".to_string(),
                icon_url: Some("http://127.0.0.1:4040/items/golden-coelacanth.webp".to_string()),
                icon_grade_tone: "yellow".to_string(),
                fill_color: "pink",
                stroke_color: "red",
                text_color: "black",
                connector_color: "rgba(0,0,0,0.2)",
                expected_count_raw: 2.0,
                expected_profit_raw: 100_000.0,
                expected_count_text: "2".to_string(),
                expected_profit_text: "100,000".to_string(),
                silver_share_text: "20.00%".to_string(),
                rate_text: "60%".to_string(),
                rate_source_kind: "database".to_string(),
                rate_tooltip: "Database in-group rate".to_string(),
                drop_rate_text: "60%".to_string(),
                drop_rate_source_kind: "database".to_string(),
                drop_rate_tooltip: "Database in-group rate".to_string(),
                presence_text: None,
                presence_source_kind: "database".to_string(),
                presence_tooltip: None,
                evidence_text: String::new(),
                catch_methods: vec!["rod".to_string()],
                count_breakdown: String::new(),
                silver_breakdown: String::new(),
                within_group_rate_raw: 0.60,
                base_price_raw: 50_000.0,
                sale_multiplier_raw: 1.0,
                discarded: false,
            },
            LootSpeciesRow {
                slot_idx: 1,
                item_id: 820002,
                group_label: "Prize",
                label: "Silver Pomfret".to_string(),
                icon_url: Some("http://127.0.0.1:4040/items/silver-pomfret.webp".to_string()),
                icon_grade_tone: "blue".to_string(),
                fill_color: "pink",
                stroke_color: "red",
                text_color: "black",
                connector_color: "rgba(0,0,0,0.2)",
                expected_count_raw: 1.02,
                expected_profit_raw: 20_000.0,
                expected_count_text: "1.02".to_string(),
                expected_profit_text: "20,000".to_string(),
                silver_share_text: "4.00%".to_string(),
                rate_text: "40%".to_string(),
                rate_source_kind: "database".to_string(),
                rate_tooltip: "Database in-group rate".to_string(),
                drop_rate_text: "40%".to_string(),
                drop_rate_source_kind: "database".to_string(),
                drop_rate_tooltip: "Database in-group rate".to_string(),
                presence_text: None,
                presence_source_kind: "database".to_string(),
                presence_tooltip: None,
                evidence_text: String::new(),
                catch_methods: vec!["rod".to_string()],
                count_breakdown: String::new(),
                silver_breakdown: String::new(),
                within_group_rate_raw: 0.40,
                base_price_raw: 19_607.843137254902,
                sale_multiplier_raw: 1.0,
                discarded: false,
            },
            LootSpeciesRow {
                slot_idx: 4,
                item_id: 820003,
                group_label: "General",
                label: "Trout".to_string(),
                icon_url: None,
                icon_grade_tone: "green".to_string(),
                fill_color: "green",
                stroke_color: "lime",
                text_color: "black",
                connector_color: "rgba(0,0,0,0.2)",
                expected_count_raw: 48.98,
                expected_profit_raw: 380_000.0,
                expected_count_text: "48.98".to_string(),
                expected_profit_text: "380,000".to_string(),
                silver_share_text: "76.00%".to_string(),
                rate_text: "100%".to_string(),
                rate_source_kind: "database".to_string(),
                rate_tooltip: "Database in-group rate".to_string(),
                drop_rate_text: "100%".to_string(),
                drop_rate_source_kind: "database".to_string(),
                drop_rate_tooltip: "Database in-group rate".to_string(),
                presence_text: None,
                presence_source_kind: "database".to_string(),
                presence_tooltip: None,
                evidence_text: String::new(),
                catch_methods: vec!["rod".to_string()],
                count_breakdown: String::new(),
                silver_breakdown: String::new(),
                within_group_rate_raw: 1.0,
                base_price_raw: 7_758.268681094324,
                sale_multiplier_raw: 1.0,
                discarded: false,
            },
        ];

        let segments =
            super::group_silver_distribution_segments(&rows, &species_rows, CalculatorLocale::EnUs);
        let breakdown = segments[0]
            .breakdown
            .as_ref()
            .expect("silver group chart should expose a breakdown");

        assert_eq!(segments[0].label, "Prize");
        assert_eq!(segments[0].value_text, "24.00%");
        assert_eq!(segments[0].detail_text, "120K");
        assert_eq!(breakdown.title, "Prize silver share");
        assert!(breakdown.summary_text.contains("Expected silver share"));
        assert_eq!(breakdown.sections[0].label, "Inputs");
        assert_eq!(breakdown.sections[0].rows[0].label, "Golden Coelacanth");
        assert_eq!(breakdown.sections[0].rows[0].value_text, "100,000");
        assert_eq!(breakdown.sections[0].rows[0].kind, Some("item"));
        assert_eq!(
            breakdown.sections[0].rows[0].icon_url.as_deref(),
            Some("http://127.0.0.1:4040/items/golden-coelacanth.webp")
        );
        assert_eq!(
            breakdown.sections[1].rows[0].label,
            "Normalized group share"
        );
        assert_eq!(breakdown.sections[1].rows[2].value_text, "120,000");
        assert_eq!(breakdown.sections[1].rows[4].value_text, "24.00%");
    }

    #[test]
    fn derive_loot_chart_exposes_loot_flow_breakdowns() {
        let signals = CalculatorSignals::default();
        let fish_group_chart = FishGroupChart {
            available: true,
            note: String::new(),
            raw_prize_rate_text: "0%".to_string(),
            mastery_text: "0".to_string(),
            rows: vec![FishGroupChartRow {
                label: "General",
                fill_color: "green",
                stroke_color: "lime",
                text_color: "black",
                connector_color: "rgba(0,0,0,0.2)",
                bonus_text: String::new(),
                base_share_pct: 100.0,
                default_weight_pct: 100.0,
                weight_pct: 100.0,
                current_share_pct: 100.0,
                drop_rate_source_kind: String::new(),
                drop_rate_tooltip: String::new(),
                rate_inputs: Vec::new(),
            }],
        };
        let data = CalculatorData {
            catalog: CalculatorCatalogResponse {
                lifeskill_levels: vec![CalculatorLifeskillLevelEntry {
                    key: "81".to_string(),
                    name: "Guru 1".to_string(),
                    index: 81,
                    order: 81,
                    lifeskill_level_drr: 0.0,
                }],
                ..CalculatorCatalogResponse::default()
            },
            cdn_base_url: "http://127.0.0.1:4040".to_string(),
            lang: CalculatorLocale::EnUs,
            api_lang: DataLang::En,
            zones: Vec::new(),
            zone_group_rates: HashMap::new(),
            zone_loot_entries: vec![CalculatorZoneLootEntry {
                slot_idx: 1,
                item_id: 820001,
                name: "Weighted Fish".to_string(),
                vendor_price: Some(50_000),
                within_group_rate: 0.25,
                icon: Some("/items/weighted-fish.webp".to_string()),
                group_conditions_raw: vec!["getLifeLevel(1)>80;".to_string()],
                ..CalculatorZoneLootEntry::default()
            }],
        };

        let loot_chart = derive_loot_chart(&signals, &data, &fish_group_chart, 40.0, 1.0);
        let flow_rows = filtered_loot_flow_rows(&loot_chart.rows, &loot_chart.species_rows);
        let group_payload = serde_json::from_str::<Value>(&flow_rows[0].count_breakdown)
            .expect("group breakdown should be valid json");
        let group_silver_payload = serde_json::from_str::<Value>(&flow_rows[0].silver_breakdown)
            .expect("group silver breakdown should be valid json");
        let species_payload =
            serde_json::from_str::<Value>(&loot_chart.species_rows[0].count_breakdown)
                .expect("species count breakdown should be valid json");
        let species_silver_payload =
            serde_json::from_str::<Value>(&loot_chart.species_rows[0].silver_breakdown)
                .expect("species silver breakdown should be valid json");

        assert_eq!(group_payload["title"], "General group");
        assert_eq!(flow_rows[0].condition_text, "Fishing Level Guru 1+");
        assert_eq!(group_silver_payload["title"], "General silver share");
        assert_eq!(species_payload["title"], "Weighted Fish expected catches");
        assert_eq!(species_payload["formula_terms"][1]["label"], "Group share");
        assert_eq!(
            species_silver_payload["title"],
            "Weighted Fish silver share"
        );
        assert_eq!(
            species_silver_payload["formula_terms"][0]["label"],
            "Item expected silver"
        );
        assert_eq!(
            species_silver_payload["formula_terms"][4]["label"],
            "Silver share"
        );
    }

    #[test]
    fn derive_target_fish_summary_aggregates_matching_rows_across_groups() {
        let signals = CalculatorSignals {
            target_fish: "Laila's Petal".to_string(),
            target_fish_amount: 1.0,
            target_fish_pmf_count: 1.0,
            ..CalculatorSignals::default()
        };
        let fish_group_chart = FishGroupChart {
            available: true,
            note: String::new(),
            raw_prize_rate_text: "5%".to_string(),
            mastery_text: "0".to_string(),
            rows: vec![
                FishGroupChartRow {
                    label: "Prize",
                    fill_color: "pink",
                    stroke_color: "red",
                    text_color: "black",
                    connector_color: "rgba(0,0,0,0.2)",
                    bonus_text: String::new(),
                    base_share_pct: 0.0,
                    default_weight_pct: 0.0,
                    weight_pct: 0.0,
                    current_share_pct: 25.0,
                    drop_rate_source_kind: String::new(),
                    drop_rate_tooltip: String::new(),
                    rate_inputs: Vec::new(),
                },
                FishGroupChartRow {
                    label: "Rare",
                    fill_color: "yellow",
                    stroke_color: "gold",
                    text_color: "black",
                    connector_color: "rgba(0,0,0,0.2)",
                    bonus_text: String::new(),
                    base_share_pct: 0.0,
                    default_weight_pct: 0.0,
                    weight_pct: 0.0,
                    current_share_pct: 0.0,
                    drop_rate_source_kind: String::new(),
                    drop_rate_tooltip: String::new(),
                    rate_inputs: Vec::new(),
                },
                FishGroupChartRow {
                    label: "High-Quality",
                    fill_color: "blue",
                    stroke_color: "navy",
                    text_color: "black",
                    connector_color: "rgba(0,0,0,0.2)",
                    bonus_text: String::new(),
                    base_share_pct: 0.0,
                    default_weight_pct: 0.0,
                    weight_pct: 0.0,
                    current_share_pct: 0.0,
                    drop_rate_source_kind: String::new(),
                    drop_rate_tooltip: String::new(),
                    rate_inputs: Vec::new(),
                },
                FishGroupChartRow {
                    label: "General",
                    fill_color: "green",
                    stroke_color: "lime",
                    text_color: "black",
                    connector_color: "rgba(0,0,0,0.2)",
                    bonus_text: String::new(),
                    base_share_pct: 0.0,
                    default_weight_pct: 0.0,
                    weight_pct: 0.0,
                    current_share_pct: 75.0,
                    drop_rate_source_kind: String::new(),
                    drop_rate_tooltip: String::new(),
                    rate_inputs: Vec::new(),
                },
            ],
        };
        let data = CalculatorData {
            catalog: CalculatorCatalogResponse::default(),
            cdn_base_url: "http://127.0.0.1:4040".to_string(),
            lang: CalculatorLocale::EnUs,
            api_lang: DataLang::En,
            zones: Vec::new(),
            zone_group_rates: HashMap::new(),
            zone_loot_entries: vec![
                CalculatorZoneLootEntry {
                    slot_idx: 1,
                    item_id: 54031,
                    name: "Laila's Petal".to_string(),
                    within_group_rate: 0.1,
                    ..CalculatorZoneLootEntry::default()
                },
                CalculatorZoneLootEntry {
                    slot_idx: 4,
                    item_id: 54031,
                    name: "Laila's Petal".to_string(),
                    within_group_rate: 0.02,
                    ..CalculatorZoneLootEntry::default()
                },
            ],
        };

        let summary = derive_target_fish_summary(&signals, &data, &fish_group_chart, 100.0, 7200.0);

        assert_eq!(summary.selected_label, "Laila's Petal");
        assert_eq!(summary.target_amount_text, "1");
        assert_eq!(summary.expected_count_text, "4");
        assert_eq!(summary.per_day_text, "48");
        assert_eq!(summary.time_to_target_text, "30m");
        assert_eq!(summary.probability_at_least_text, "98.17%");
        assert_eq!(summary.session_distribution.len(), 2);
        assert_eq!(summary.session_distribution[0].label, "0");
        assert_eq!(summary.session_distribution[0].probability_text, "1.83%");
        assert_eq!(summary.session_distribution[1].label, "≥1");
        assert_eq!(summary.session_distribution[1].probability_text, "98.17%");
    }

    #[test]
    fn render_target_fish_panel_places_picker_before_target_amount_field() {
        let data = CalculatorData {
            catalog: CalculatorCatalogResponse::default(),
            cdn_base_url: "http://127.0.0.1:4040".to_string(),
            lang: CalculatorLocale::EnUs,
            api_lang: DataLang::En,
            zones: Vec::new(),
            zone_group_rates: HashMap::new(),
            zone_loot_entries: Vec::new(),
        };
        let signals = CalculatorSignals::default();
        let target_fish_options = vec![SelectOption {
            value: "item:1",
            label: "Test Fish",
            icon: None,
            grade_tone: "unknown",
            pet_variant_talent: None,
            pet_variant_special: None,
            pet_skill: None,
            pet_effective_talent_effects: None,
            pet_skill_learn_chance: None,
            item: None,
            lifeskill_level: None,
            presentation: SelectOptionPresentation::Default,
            sort_priority: 1,
        }];
        let target_fish_summary = TargetFishSummary {
            selected_label: String::new(),
            target_amount: 1,
            target_amount_text: "1".to_string(),
            pmf_count_hint_text: "0 = auto".to_string(),
            expected_count_text: "—".to_string(),
            per_day_text: "—".to_string(),
            time_to_target_text: "—".to_string(),
            probability_at_least_text: "—".to_string(),
            session_distribution: Vec::new(),
            status_text: "Select a target fish.".to_string(),
        };

        let html =
            render_target_fish_panel(&data, &signals, &target_fish_options, &target_fish_summary);

        let target_fish_picker = html
            .find("calculator-target-fish-picker")
            .expect("target fish picker should render");
        let target_amount_input = html
            .find("data-bind=\"targetFishAmount\"")
            .expect("target amount input should render");
        assert!(target_fish_picker < target_amount_input);
        assert!(html
            .contains("panel-mode=\"detached\" panel-min-width=\"panel\" panel-width=\"34rem\""));
        assert!(html.contains("id=\"target-fish-pmf-chart\""));
        assert!(html.contains("data-show=\"($_calc.target_fish_pmf_chart.bars || []).length > 0\""));
    }

    #[test]
    fn derive_target_fish_summary_compresses_distribution_for_larger_targets() {
        let signals = CalculatorSignals {
            target_fish: "Laila's Petal".to_string(),
            target_fish_amount: 8.0,
            target_fish_pmf_count: 8.0,
            ..CalculatorSignals::default()
        };
        let fish_group_chart = FishGroupChart {
            available: true,
            note: String::new(),
            raw_prize_rate_text: "5%".to_string(),
            mastery_text: "0".to_string(),
            rows: vec![FishGroupChartRow {
                label: "General",
                fill_color: "green",
                stroke_color: "lime",
                text_color: "black",
                connector_color: "rgba(0,0,0,0.2)",
                bonus_text: String::new(),
                base_share_pct: 0.0,
                default_weight_pct: 0.0,
                weight_pct: 0.0,
                current_share_pct: 100.0,
                drop_rate_source_kind: String::new(),
                drop_rate_tooltip: String::new(),
                rate_inputs: Vec::new(),
            }],
        };
        let data = CalculatorData {
            catalog: CalculatorCatalogResponse::default(),
            cdn_base_url: "http://127.0.0.1:4040".to_string(),
            lang: CalculatorLocale::EnUs,
            api_lang: DataLang::En,
            zones: Vec::new(),
            zone_group_rates: HashMap::new(),
            zone_loot_entries: vec![CalculatorZoneLootEntry {
                slot_idx: 1,
                item_id: 54031,
                name: "Laila's Petal".to_string(),
                within_group_rate: 0.04,
                ..CalculatorZoneLootEntry::default()
            }],
        };

        let summary = derive_target_fish_summary(&signals, &data, &fish_group_chart, 100.0, 7200.0);

        assert_eq!(summary.target_amount_text, "8");
        assert_eq!(
            summary
                .session_distribution
                .iter()
                .map(|bucket| bucket.label.as_str())
                .collect::<Vec<_>>(),
            vec!["0", "1", "2", "3", "4", "5", "6", "7", "≥8"]
        );
    }

    #[test]
    fn derive_target_fish_summary_uses_narrower_ranges_for_large_targets() {
        let signals = CalculatorSignals {
            target_fish: "Laila's Petal".to_string(),
            target_fish_amount: 20.0,
            target_fish_pmf_count: 20.0,
            ..CalculatorSignals::default()
        };
        let fish_group_chart = FishGroupChart {
            available: true,
            note: String::new(),
            raw_prize_rate_text: "5%".to_string(),
            mastery_text: "0".to_string(),
            rows: vec![FishGroupChartRow {
                label: "General",
                fill_color: "green",
                stroke_color: "lime",
                text_color: "black",
                connector_color: "rgba(0,0,0,0.2)",
                bonus_text: String::new(),
                base_share_pct: 0.0,
                default_weight_pct: 0.0,
                weight_pct: 0.0,
                current_share_pct: 100.0,
                drop_rate_source_kind: String::new(),
                drop_rate_tooltip: String::new(),
                rate_inputs: Vec::new(),
            }],
        };
        let data = CalculatorData {
            catalog: CalculatorCatalogResponse::default(),
            cdn_base_url: "http://127.0.0.1:4040".to_string(),
            lang: CalculatorLocale::EnUs,
            api_lang: DataLang::En,
            zones: Vec::new(),
            zone_group_rates: HashMap::new(),
            zone_loot_entries: vec![CalculatorZoneLootEntry {
                slot_idx: 1,
                item_id: 54031,
                name: "Laila's Petal".to_string(),
                within_group_rate: 0.1118,
                ..CalculatorZoneLootEntry::default()
            }],
        };

        let summary = derive_target_fish_summary(&signals, &data, &fish_group_chart, 100.0, 3600.0);
        let labels = summary
            .session_distribution
            .iter()
            .map(|bucket| bucket.label.as_str())
            .collect::<Vec<_>>();

        assert_eq!(labels.last().copied(), Some("≥20"));
        assert!(labels.iter().any(|label| label.contains('–')));
        assert!(labels.len() <= 11);
        assert!(!labels.iter().any(|label| *label == "6–19"));
    }

    #[test]
    fn derive_target_fish_summary_auto_pmf_cutoff_uses_tail_probability_threshold() {
        let signals = CalculatorSignals {
            target_fish: "Laila's Petal".to_string(),
            target_fish_amount: 1.0,
            target_fish_pmf_count: 0.0,
            ..CalculatorSignals::default()
        };
        let fish_group_chart = FishGroupChart {
            available: true,
            note: String::new(),
            raw_prize_rate_text: "5%".to_string(),
            mastery_text: "0".to_string(),
            rows: vec![FishGroupChartRow {
                label: "General",
                fill_color: "green",
                stroke_color: "lime",
                text_color: "black",
                connector_color: "rgba(0,0,0,0.2)",
                bonus_text: String::new(),
                base_share_pct: 0.0,
                default_weight_pct: 0.0,
                weight_pct: 0.0,
                current_share_pct: 100.0,
                drop_rate_source_kind: String::new(),
                drop_rate_tooltip: String::new(),
                rate_inputs: Vec::new(),
            }],
        };
        let data = CalculatorData {
            catalog: CalculatorCatalogResponse::default(),
            cdn_base_url: "http://127.0.0.1:4040".to_string(),
            lang: CalculatorLocale::EnUs,
            api_lang: DataLang::En,
            zones: Vec::new(),
            zone_group_rates: HashMap::new(),
            zone_loot_entries: vec![CalculatorZoneLootEntry {
                slot_idx: 1,
                item_id: 54031,
                name: "Laila's Petal".to_string(),
                within_group_rate: 0.1118,
                ..CalculatorZoneLootEntry::default()
            }],
        };

        let summary = derive_target_fish_summary(&signals, &data, &fish_group_chart, 100.0, 3600.0);

        let effective = auto_target_fish_pmf_tail_count(11.18);
        assert!(poisson_probability_at_least(11.18, effective) * 100.0 <= 0.5);
        assert!(effective <= 1 || poisson_probability_at_least(11.18, effective - 1) * 100.0 > 0.5);
        assert!(summary.pmf_count_hint_text.contains("0.5% tail cutoff"));
    }

    #[test]
    fn pmf_bucket_contains_target_matches_exact_ranges_and_tail() {
        assert!(pmf_bucket_contains_target("1", 1));
        assert!(!pmf_bucket_contains_target("1", 2));
        assert!(pmf_bucket_contains_target("6–7", 6));
        assert!(pmf_bucket_contains_target("6–7", 7));
        assert!(!pmf_bucket_contains_target("6–7", 8));
        assert!(pmf_bucket_contains_target("≥20", 20));
        assert!(pmf_bucket_contains_target("≥20", 27));
        assert!(!pmf_bucket_contains_target("≥20", 19));
    }

    #[test]
    fn derive_fish_group_chart_preserves_prize_group_when_zone_supports_mastery_bonus() {
        let signals = CalculatorSignals {
            zone: "240,74,74".to_string(),
            mastery: 1000.0,
            ..CalculatorSignals::default()
        };
        let data = CalculatorData {
            catalog: CalculatorCatalogResponse {
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
                ],
                ..CalculatorCatalogResponse::default()
            },
            cdn_base_url: "http://127.0.0.1:4040".to_string(),
            lang: CalculatorLocale::EnUs,
            api_lang: DataLang::En,
            zones: Vec::new(),
            zone_group_rates: HashMap::from([(
                "240,74,74".to_string(),
                CalculatorZoneGroupRateEntry {
                    zone_rgb_key: "240,74,74".to_string(),
                    prize_main_group_key: Some(11054),
                    rare_rate_raw: 100_000,
                    high_quality_rate_raw: 217_500,
                    general_rate_raw: 620_000,
                    trash_rate_raw: 62_500,
                },
            )]),
            zone_loot_entries: vec![CalculatorZoneLootEntry {
                slot_idx: 4,
                item_id: 8201,
                name: "Mudskipper".to_string(),
                within_group_rate: 1.0,
                ..CalculatorZoneLootEntry::default()
            }],
        };

        let fish_group_chart = derive_fish_group_chart(&signals, &data, &HashMap::new());

        assert_eq!(fish_group_chart.rows.len(), 5);
        assert_eq!(fish_group_chart.rows[0].label, "Prize");
        assert!((fish_group_chart.rows[0].base_share_pct - 0.0).abs() < 1e-9);
        assert!((fish_group_chart.rows[0].weight_pct - 2.5).abs() < 1e-9);
        assert!((fish_group_chart.rows[0].current_share_pct - 3.875968992248062).abs() < 1e-9);
        assert_eq!(fish_group_chart.rows[1].label, "Rare");
        assert_eq!(fish_group_chart.rows[1].current_share_pct, 0.0);
        assert_eq!(fish_group_chart.rows[2].label, "High-Quality");
        assert_eq!(fish_group_chart.rows[2].current_share_pct, 0.0);
        assert_eq!(fish_group_chart.rows[3].label, "General");
        assert!((fish_group_chart.rows[3].current_share_pct - 96.12403100775194).abs() < 1e-9);
        assert_eq!(fish_group_chart.rows[4].label, "Trash");
        assert_eq!(fish_group_chart.rows[4].current_share_pct, 0.0);
    }

    #[test]
    fn derive_fish_group_chart_includes_bonus_sources_in_rate_inputs() {
        let signals = CalculatorSignals {
            zone: "bonus_zone".to_string(),
            float: "item:rare-float".to_string(),
            chair: "item:hq-chair".to_string(),
            ..CalculatorSignals::default()
        };
        let data = CalculatorData {
            catalog: CalculatorCatalogResponse {
                items: vec![
                    CalculatorItemEntry {
                        key: "item:rare-float".to_string(),
                        name: "Rare Float".to_string(),
                        bonus_rare: Some(0.10),
                        grade: Some("Rare".to_string()),
                        icon: Some("/items/rare-float.webp".to_string()),
                        ..CalculatorItemEntry::default()
                    },
                    CalculatorItemEntry {
                        key: "item:hq-chair".to_string(),
                        name: "HQ Chair".to_string(),
                        bonus_big: Some(0.11),
                        grade: Some("HighQuality".to_string()),
                        icon: Some("/items/hq-chair.webp".to_string()),
                        ..CalculatorItemEntry::default()
                    },
                ],
                ..CalculatorCatalogResponse::default()
            },
            cdn_base_url: "http://127.0.0.1:4040".to_string(),
            lang: CalculatorLocale::EnUs,
            api_lang: DataLang::En,
            zones: Vec::new(),
            zone_group_rates: HashMap::from([(
                "bonus_zone".to_string(),
                CalculatorZoneGroupRateEntry {
                    zone_rgb_key: "bonus_zone".to_string(),
                    prize_main_group_key: None,
                    rare_rate_raw: 100_000,
                    high_quality_rate_raw: 200_000,
                    general_rate_raw: 700_000,
                    trash_rate_raw: 0,
                },
            )]),
            zone_loot_entries: vec![
                CalculatorZoneLootEntry {
                    slot_idx: 2,
                    item_id: 820010,
                    name: "Rare Fish".to_string(),
                    within_group_rate: 1.0,
                    ..CalculatorZoneLootEntry::default()
                },
                CalculatorZoneLootEntry {
                    slot_idx: 3,
                    item_id: 820020,
                    name: "HQ Fish".to_string(),
                    within_group_rate: 1.0,
                    ..CalculatorZoneLootEntry::default()
                },
                CalculatorZoneLootEntry {
                    slot_idx: 4,
                    item_id: 820030,
                    name: "General Fish".to_string(),
                    within_group_rate: 1.0,
                    ..CalculatorZoneLootEntry::default()
                },
            ],
        };
        let items_by_key = data
            .catalog
            .items
            .iter()
            .map(|item| (item.key.as_str(), item))
            .collect::<HashMap<_, _>>();

        let fish_group_chart = derive_fish_group_chart(&signals, &data, &items_by_key);
        let tolerance = 1e-6;

        assert_eq!(fish_group_chart.rows[1].label, "Rare");
        assert_eq!(fish_group_chart.rows[1].bonus_text, "+10% Rare");
        assert!((fish_group_chart.rows[1].base_share_pct - 10.0).abs() < tolerance);
        assert!((fish_group_chart.rows[1].weight_pct - 20.0).abs() < tolerance);
        assert!(
            (fish_group_chart.rows[1].current_share_pct - 16.528925619834713).abs() < tolerance
        );
        assert_eq!(
            fish_group_chart.rows[1].rate_inputs[0].label,
            "Zone base rate"
        );
        assert_eq!(fish_group_chart.rows[1].rate_inputs[0].value_text, "10%");
        assert_eq!(fish_group_chart.rows[1].rate_inputs[1].label, "Rare Float");
        assert_eq!(fish_group_chart.rows[1].rate_inputs[1].value_text, "+10%");
        assert_eq!(fish_group_chart.rows[1].rate_inputs[1].kind, Some("item"));
        assert_eq!(
            fish_group_chart.rows[1].rate_inputs[1].icon_url.as_deref(),
            Some("http://127.0.0.1:4040/items/rare-float.webp")
        );
        assert_eq!(
            fish_group_chart.rows[1].rate_inputs[1]
                .grade_tone
                .as_deref(),
            Some("yellow")
        );

        assert_eq!(fish_group_chart.rows[2].label, "High-Quality");
        assert_eq!(fish_group_chart.rows[2].bonus_text, "+11% HQ");
        assert!((fish_group_chart.rows[2].base_share_pct - 20.0).abs() < tolerance);
        assert!((fish_group_chart.rows[2].weight_pct - 31.0).abs() < tolerance);
        assert!((fish_group_chart.rows[2].current_share_pct - 25.6198347107438).abs() < tolerance);
        assert_eq!(
            fish_group_chart.rows[2].rate_inputs[0].label,
            "Zone base rate"
        );
        assert_eq!(fish_group_chart.rows[2].rate_inputs[0].value_text, "20%");
        assert_eq!(fish_group_chart.rows[2].rate_inputs[1].label, "HQ Chair");
        assert_eq!(fish_group_chart.rows[2].rate_inputs[1].value_text, "+11%");
        assert_eq!(fish_group_chart.rows[2].rate_inputs[1].kind, Some("item"));
        assert_eq!(
            fish_group_chart.rows[2].rate_inputs[1].icon_url.as_deref(),
            Some("http://127.0.0.1:4040/items/hq-chair.webp")
        );
        assert_eq!(
            fish_group_chart.rows[2].rate_inputs[1]
                .grade_tone
                .as_deref(),
            Some("blue")
        );

        assert_eq!(fish_group_chart.rows[3].label, "General");
        assert!((fish_group_chart.rows[3].weight_pct - 70.0).abs() < tolerance);
        assert!((fish_group_chart.rows[3].current_share_pct - 57.85123966942149).abs() < tolerance);
    }

    #[test]
    fn derive_fish_group_chart_normalizes_raw_overlay_weights() {
        let mut signals = CalculatorSignals {
            zone: "overlay_zone".to_string(),
            ..CalculatorSignals::default()
        };
        signals.overlay.zones.insert(
            "overlay_zone".to_string(),
            fishystuff_api::models::calculator::CalculatorZoneOverlaySignals {
                groups: std::collections::BTreeMap::from([(
                    "2".to_string(),
                    fishystuff_api::models::calculator::CalculatorZoneGroupOverlaySignals {
                        present: None,
                        raw_rate_percent: Some(40.0),
                    },
                )]),
                items: std::collections::BTreeMap::new(),
            },
        );
        let data = CalculatorData {
            catalog: CalculatorCatalogResponse::default(),
            cdn_base_url: "http://127.0.0.1:4040".to_string(),
            lang: CalculatorLocale::EnUs,
            api_lang: DataLang::En,
            zones: Vec::new(),
            zone_group_rates: HashMap::from([(
                "overlay_zone".to_string(),
                CalculatorZoneGroupRateEntry {
                    zone_rgb_key: "overlay_zone".to_string(),
                    prize_main_group_key: None,
                    rare_rate_raw: 100_000,
                    high_quality_rate_raw: 200_000,
                    general_rate_raw: 700_000,
                    trash_rate_raw: 0,
                },
            )]),
            zone_loot_entries: vec![
                CalculatorZoneLootEntry {
                    slot_idx: 2,
                    item_id: 820010,
                    name: "Rare Fish".to_string(),
                    within_group_rate: 1.0,
                    ..CalculatorZoneLootEntry::default()
                },
                CalculatorZoneLootEntry {
                    slot_idx: 3,
                    item_id: 820020,
                    name: "HQ Fish".to_string(),
                    within_group_rate: 1.0,
                    ..CalculatorZoneLootEntry::default()
                },
                CalculatorZoneLootEntry {
                    slot_idx: 4,
                    item_id: 820030,
                    name: "General Fish".to_string(),
                    within_group_rate: 1.0,
                    ..CalculatorZoneLootEntry::default()
                },
            ],
        };

        let fish_group_chart = derive_fish_group_chart(&signals, &data, &HashMap::new());
        let tolerance = 1e-9;

        assert!((fish_group_chart.rows[1].default_weight_pct - 10.0).abs() < tolerance);
        assert!((fish_group_chart.rows[1].weight_pct - 40.0).abs() < tolerance);
        assert!((fish_group_chart.rows[1].current_share_pct - 30.76923076923077).abs() < tolerance);
        assert_eq!(fish_group_chart.rows[1].drop_rate_source_kind, "overlay");
        assert!(fish_group_chart.rows[1]
            .drop_rate_tooltip
            .contains("raw group base rate 40%"));
    }

    #[test]
    fn derive_fish_group_chart_keeps_mastery_bonus_when_prize_base_override_is_zero() {
        let mut signals = CalculatorSignals {
            zone: "prize_overlay_zone".to_string(),
            mastery: 1000.0,
            ..CalculatorSignals::default()
        };
        signals.overlay.zones.insert(
            "prize_overlay_zone".to_string(),
            fishystuff_api::models::calculator::CalculatorZoneOverlaySignals {
                groups: std::collections::BTreeMap::from([(
                    "1".to_string(),
                    fishystuff_api::models::calculator::CalculatorZoneGroupOverlaySignals {
                        present: None,
                        raw_rate_percent: Some(0.0),
                    },
                )]),
                items: std::collections::BTreeMap::new(),
            },
        );
        let data = CalculatorData {
            catalog: CalculatorCatalogResponse {
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
                ],
                ..CalculatorCatalogResponse::default()
            },
            cdn_base_url: "http://127.0.0.1:4040".to_string(),
            lang: CalculatorLocale::EnUs,
            api_lang: DataLang::En,
            zones: Vec::new(),
            zone_group_rates: HashMap::from([(
                "prize_overlay_zone".to_string(),
                CalculatorZoneGroupRateEntry {
                    zone_rgb_key: "prize_overlay_zone".to_string(),
                    prize_main_group_key: Some(11054),
                    rare_rate_raw: 0,
                    high_quality_rate_raw: 0,
                    general_rate_raw: 1_000_000,
                    trash_rate_raw: 0,
                },
            )]),
            zone_loot_entries: vec![CalculatorZoneLootEntry {
                slot_idx: 4,
                item_id: 820001,
                name: "General Fish".to_string(),
                within_group_rate: 1.0,
                ..CalculatorZoneLootEntry::default()
            }],
        };

        let fish_group_chart = derive_fish_group_chart(&signals, &data, &HashMap::new());

        assert!((fish_group_chart.rows[0].base_share_pct - 0.0).abs() < 1e-9);
        assert!((fish_group_chart.rows[0].weight_pct - 2.5).abs() < 1e-9);
        assert!((fish_group_chart.rows[0].current_share_pct - 2.4390243902439024).abs() < 1e-9);
        assert_eq!(fish_group_chart.rows[0].drop_rate_source_kind, "overlay");
        assert!(fish_group_chart.rows[0]
            .drop_rate_tooltip
            .contains("base rate 0% plus active group bonus 2.5%"));
        assert_eq!(fish_group_chart.rows[0].bonus_text, "Base 0% + bonus 2.5%");
    }

    #[test]
    fn overlay_editor_prize_group_stays_present_with_zero_base_rate() {
        let signals = CalculatorSignals {
            zone: "prize_editor_zone".to_string(),
            mastery: 0.0,
            ..CalculatorSignals::default()
        };
        let data = CalculatorData {
            catalog: CalculatorCatalogResponse::default(),
            cdn_base_url: "http://127.0.0.1:4040".to_string(),
            lang: CalculatorLocale::EnUs,
            api_lang: DataLang::En,
            zones: vec![ZoneEntry {
                rgb_key: fishystuff_api::ids::RgbKey("prize_editor_zone".to_string()),
                name: Some("Prize Coast".to_string()),
                ..ZoneEntry::default()
            }],
            zone_group_rates: HashMap::from([(
                "prize_editor_zone".to_string(),
                CalculatorZoneGroupRateEntry {
                    zone_rgb_key: "prize_editor_zone".to_string(),
                    prize_main_group_key: Some(11054),
                    rare_rate_raw: 0,
                    high_quality_rate_raw: 0,
                    general_rate_raw: 1_000_000,
                    trash_rate_raw: 0,
                },
            )]),
            zone_loot_entries: vec![CalculatorZoneLootEntry {
                slot_idx: 4,
                item_id: 820001,
                name: "General Fish".to_string(),
                within_group_rate: 1.0,
                ..CalculatorZoneLootEntry::default()
            }],
        };

        let fish_group_chart = derive_fish_group_chart(&signals, &data, &HashMap::new());
        let editor = super::build_overlay_editor_signal(&signals, &data, &fish_group_chart);

        assert_eq!(editor.zone_name, "Prize Coast");
        assert!(editor.groups[0].default_present);
        assert_eq!(editor.groups[0].default_raw_rate_pct, 0.0);
        assert_eq!(editor.groups[0].default_raw_rate_text, "0%");
    }

    #[test]
    fn apply_zone_overlay_to_loot_entries_normalizes_raw_item_overrides() {
        let mut signals = CalculatorSignals {
            zone: "overlay_zone".to_string(),
            ..CalculatorSignals::default()
        };
        signals.overlay.zones.insert(
            "overlay_zone".to_string(),
            fishystuff_api::models::calculator::CalculatorZoneOverlaySignals {
                groups: std::collections::BTreeMap::new(),
                items: std::collections::BTreeMap::from([(
                    "820002".to_string(),
                    fishystuff_api::models::calculator::CalculatorZoneLootOverlaySignals {
                        present: None,
                        slot_idx: None,
                        raw_rate_percent: Some(10.0),
                        name: None,
                        grade: None,
                        is_fish: None,
                    },
                )]),
            },
        );
        let base_entries = vec![
            CalculatorZoneLootEntry {
                slot_idx: 4,
                item_id: 820001,
                name: "Fish A".to_string(),
                within_group_rate: 0.5,
                evidence: vec![CalculatorZoneLootEvidence {
                    source_family: "database".to_string(),
                    claim_kind: "in_group_rate".to_string(),
                    scope: "group".to_string(),
                    rate: Some(0.30),
                    normalized_rate: Some(0.5),
                    ..CalculatorZoneLootEvidence::default()
                }],
                ..CalculatorZoneLootEntry::default()
            },
            CalculatorZoneLootEntry {
                slot_idx: 4,
                item_id: 820002,
                name: "Fish B".to_string(),
                within_group_rate: 0.3333333333333333,
                evidence: vec![CalculatorZoneLootEvidence {
                    source_family: "database".to_string(),
                    claim_kind: "in_group_rate".to_string(),
                    scope: "group".to_string(),
                    rate: Some(0.20),
                    normalized_rate: Some(0.3333333333333333),
                    ..CalculatorZoneLootEvidence::default()
                }],
                ..CalculatorZoneLootEntry::default()
            },
            CalculatorZoneLootEntry {
                slot_idx: 4,
                item_id: 820003,
                name: "Fish C".to_string(),
                within_group_rate: 0.16666666666666666,
                evidence: vec![CalculatorZoneLootEvidence {
                    source_family: "database".to_string(),
                    claim_kind: "in_group_rate".to_string(),
                    scope: "group".to_string(),
                    rate: Some(0.10),
                    normalized_rate: Some(0.16666666666666666),
                    ..CalculatorZoneLootEvidence::default()
                }],
                ..CalculatorZoneLootEntry::default()
            },
        ];

        let entries =
            super::apply_zone_overlay_to_loot_entries(&signals, "overlay_zone", &base_entries);
        let by_item_id = entries
            .iter()
            .map(|entry| (entry.item_id, entry.within_group_rate))
            .collect::<HashMap<_, _>>();

        assert!((by_item_id.get(&820001).copied().unwrap_or_default() - 0.6).abs() < 1e-9);
        assert!((by_item_id.get(&820002).copied().unwrap_or_default() - 0.2).abs() < 1e-9);
        assert!((by_item_id.get(&820003).copied().unwrap_or_default() - 0.2).abs() < 1e-9);
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
