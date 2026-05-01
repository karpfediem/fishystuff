mod effect_table_headers;
mod item_table_headers;

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use calamine::{open_workbook_auto, Data, Range, Reader};
use clap::{Parser, Subcommand, ValueEnum};
use csv::{QuoteStyle, Writer, WriterBuilder};
use fishystuff_api::models::trade::TradeNpcCatalogResponse;
use fishystuff_core::calculator_effects::{
    normalized_effect_lines, parse_unique_calculator_effect_text, CalculatorEffectValues,
};
use fishystuff_core::loc::scan_loc_records;
use sha2::{Digest, Sha256};

use effect_table_headers::{
    BUFF_TABLE_HEADERS, COMMON_STAT_DATA_HEADERS, ENCHANT_WORKBOOK_HEADERS,
    FISHING_STAT_DATA_HEADERS, LIGHTSTONE_SET_OPTION_HEADERS, PET_BASE_SKILL_TABLE_HEADERS,
    PET_EQUIPSKILL_AQUIRE_TABLE_HEADERS, PET_EQUIPSKILL_TABLE_HEADERS, PET_EXP_TABLE_HEADERS,
    PET_GRADE_TABLE_HEADERS, PET_SETSTATS_TABLE_HEADERS, PET_SKILL_TABLE_HEADERS,
    PET_TABLE_HEADERS, PRODUCTTOOL_PROPERTY_HEADERS, SKILLTYPE_TABLE_NEW_HEADERS,
    SKILL_TABLE_NEW_HEADERS, TOOLTIP_TABLE_HEADERS, TRANSLATE_STAT_HEADERS,
    UPGRADEPET_LOOTING_PERCENT_HEADERS,
};
use item_table_headers::ITEM_TABLE_HEADERS;
const FISHING_HEADERS: [&str; 18] = [
    "R",
    "G",
    "B",
    "DropID",
    "DropIDHarpoon",
    "DropIDNet",
    "DropRate1",
    "DropID1",
    "DropRate2",
    "DropID2",
    "DropRate3",
    "DropID3",
    "DropRate4",
    "DropID4",
    "DropRate5",
    "DropID5",
    "MinWaitTime",
    "MaxWaitTime",
];

const TRADE_NPC_CATALOG_META_TABLE: &str = "trade_npc_catalog_meta";
const TRADE_NPC_CATALOG_SOURCES_TABLE: &str = "trade_npc_catalog_sources";
const TRADE_ORIGIN_REGIONS_TABLE: &str = "trade_origin_regions";
const TRADE_ZONE_ORIGIN_REGIONS_TABLE: &str = "trade_zone_origin_regions";
const TRADE_NPC_DESTINATIONS_TABLE: &str = "trade_npc_destinations";
const TRADE_NPC_EXCLUDED_TABLE: &str = "trade_npc_excluded";

const MAIN_GROUP_HEADERS: [&str; 17] = [
    "ItemMainGroupKey",
    "DoSelectOnlyOne",
    "RefreshStartHour",
    "RefreshInterval",
    "PlantCraftResultCount",
    "SelectRate0",
    "Condition0",
    "ItemSubGroupKey0",
    "SelectRate1",
    "Condition1",
    "ItemSubGroupKey1",
    "SelectRate2",
    "Condition2",
    "ItemSubGroupKey2",
    "SelectRate3",
    "Condition3",
    "ItemSubGroupKey3",
];

const SUB_GROUP_HEADERS: [&str; 19] = [
    "ItemSubGroupKey",
    "ItemKey",
    "EnchantLevel",
    "DoPetAddDrop",
    "DoSechiAddDrop",
    "SelectRate_0",
    "MinCount_0",
    "MaxCount_0",
    "SelectRate_1",
    "MinCount_1",
    "MaxCount_1",
    "SelectRate_2",
    "MinCount_2",
    "MaxCount_2",
    "IntimacyVariation",
    "ExplorationPoint",
    "ApplyRandomPrice",
    "RentTime",
    "PriceOption",
];

const LANGUAGEDATA_SOURCE_HEADERS: [&str; 4] = ["id", "unk", "text", "format"];
const LANGUAGEDATA_HEADERS: [&str; 5] = ["lang", "id", "category", "text", "format"];
const LANGUAGEDATA_TABLE: &str = "languagedata";
const LANGUAGEDATA_IMPORT_TABLE: &str = "languagedata_import";
const CALCULATOR_CONSUMABLE_SOURCE_ITEM_EFFECT_EVIDENCE_TABLE: &str =
    "calculator_consumable_source_item_effect_evidence";
const CALCULATOR_CONSUMABLE_SOURCE_ITEM_EFFECT_EVIDENCE_HEADERS: [&str; 13] = [
    "source_key",
    "item_id",
    "item_type",
    "buff_category_key",
    "buff_category_id",
    "buff_category_level",
    "source_text_ko",
    "source_text_afr",
    "source_text_bonus_rare",
    "source_text_bonus_big",
    "source_text_item_drr",
    "source_text_exp_fish",
    "source_text_exp_life",
];
const FISH_TABLE_HEADERS: [&str; 5] = [
    "encyclopedia_key",
    "item_key",
    "name",
    "icon",
    "encyclopedia_icon",
];
const PATCHES_HEADERS: [&str; 11] = [
    "patch_id",
    "start_date",
    "start_ts_utc",
    "patch_name",
    "category",
    "sub_category",
    "key_values",
    "change_description",
    "impact",
    "region",
    "source_url",
];

const COMMUNITY_ZONE_FISH_SUPPORT_HEADERS: [&str; 14] = [
    "source_id",
    "source_label",
    "source_sha256",
    "zone_rgb",
    "zone_r",
    "zone_g",
    "zone_b",
    "region_name",
    "zone_name",
    "item_id",
    "fish_name",
    "support_status",
    "claim_count",
    "notes",
];

const FLOCKFISH_ZONE_GROUP_SLOT_HEADERS: [&str; 13] = [
    "source_id",
    "source_label",
    "source_sha256",
    "zone_rgb",
    "zone_r",
    "zone_g",
    "zone_b",
    "zone_name",
    "source_drop_label",
    "slot_idx",
    "item_main_group_key",
    "resolution_status",
    "resolution_value_raw",
];

const FISHING_MG_COLS: [usize; 8] = [3, 4, 5, 7, 9, 11, 13, 15];
const MAIN_GROUP_KEY_COL: usize = 0;
const MAIN_GROUP_SG_COLS: [usize; 4] = [7, 10, 13, 16];
const SUB_GROUP_KEY_COL: usize = 0;
const COMMUNITY_PRIZE_GUESS_SOURCE_ID: &str = "community_prize_fish_guesses_workbook";
const COMMUNITY_PRIZE_GUESS_SOURCE_LABEL: &str = "Updated Fishing Setup guessed prize-fish rates";
const MANUAL_COMMUNITY_PRESENCE_SOURCE_ID: &str = "manual_community_zone_fish_presence";
const MANUAL_COMMUNITY_PRESENCE_SOURCE_LABEL: &str = "Manual community zone fish presence";
const MANUAL_COMMUNITY_GUESS_SOURCE_ID: &str = "manual_community_zone_fish_guess";
const MANUAL_COMMUNITY_GUESS_SOURCE_LABEL: &str = "Manual community zone fish rate guess";
const FLOCKFISH_SOURCE_ID: &str = "flockfish_workbook";
const FLOCKFISH_ZONE_GROUP_SOURCE_LABEL: &str = "Flockfish final combined zone group table";
const COMMUNITY_SUBGROUP_OVERLAY_TABLE: &str = "community_item_sub_group_overlay";
const COMMUNITY_SUBGROUP_OVERLAY_IMPORT_TABLE: &str = "community_item_sub_group_overlay_import";
const COMMUNITY_SUBGROUP_UNRESOLVED_TABLE: &str = "community_item_sub_group_unresolved_overlay";
const COMMUNITY_SUBGROUP_UNRESOLVED_IMPORT_TABLE: &str =
    "community_item_sub_group_unresolved_overlay_import";
const COMMUNITY_ACTIVE_OVERLAYS_TABLE: &str = "community_active_overlays";
const COMMUNITY_SUBGROUP_OVERLAY_KIND: &str = "item_sub_group";
const DEFAULT_COMMUNITY_SUBGROUP_SOURCE_ID: &str = "community_subgroups_no_formulas_workbook";
const DEFAULT_COMMUNITY_SUBGROUP_SOURCE_LABEL: &str = "Community Subgroups(no formulas) workbook";
const SETUP_SPOT_NAME_COL: usize = 0;
const SETUP_SPOT_R_COL: usize = 1;
const SETUP_SPOT_G_COL: usize = 2;
const SETUP_SPOT_B_COL: usize = 3;
const SETUP_SPOT_PRIZE_SUBGROUP_COL: usize = 4;
const SETUP_NEW_PRIZE_ID_COL: usize = 0;
const SETUP_NEW_PRIZE_TITLE_COL: usize = 1;
const SETUP_NEW_PRIZE_ZONE_COL: usize = 4;
const SETUP_NEW_PRIZE_ITEM_KEY_COL: usize = 5;
const SETUP_NEW_PRIZE_FISH_COL: usize = 6;
const SETUP_NEW_PRIZE_CHANCE_COL: usize = 7;
const FLOCKFISH_JALLO_FINAL_R_COL: usize = 14;
const FLOCKFISH_JALLO_FINAL_G_COL: usize = 15;
const FLOCKFISH_JALLO_FINAL_B_COL: usize = 16;
const FLOCKFISH_JALLO_FINAL_ZONE_NAME_COL: usize = 17;
const FLOCKFISH_JALLO_FINAL_DROP_LABEL_COL: usize = 18;
const FLOCKFISH_JALLO_FINAL_GROUP_VALUE_COL: usize = 19;
const COMMUNITY_SUBGROUP_KEY_COL: usize = 0;
const COMMUNITY_SUBGROUP_ITEM_COL: usize = 1;
const COMMUNITY_SUBGROUP_SPOTTED_COL: usize = 2;
const COMMUNITY_SUBGROUP_COMMENT_COL: usize = 3;
const COMMUNITY_SUBGROUP_TABLE_COL: usize = 4;
const COMMUNITY_SUBGROUP_GRADE_COL: usize = 5;
const COMMUNITY_SUBGROUP_ITEM_NAME_COL: usize = 6;
const COMMUNITY_SUBGROUP_REMOVED_COL: usize = 8;
const COMMUNITY_SUBGROUP_ADDED_COL: usize = 9;
const COMMUNITY_SUBGROUP_ENCHANT_COL: usize = 10;
const COMMUNITY_SUBGROUP_DO_PET_COL: usize = 11;
const COMMUNITY_SUBGROUP_DO_SECHI_COL: usize = 12;
const COMMUNITY_SUBGROUP_FOR_HUMANS_COL: usize = 13;
const COMMUNITY_SUBGROUP_SELECT_RATE_0_COL: usize = 14;
const COMMUNITY_SUBGROUP_MIN_COUNT_0_COL: usize = 15;
const COMMUNITY_SUBGROUP_MAX_COUNT_0_COL: usize = 16;
const COMMUNITY_SUBGROUP_SELECT_RATE_1_COL: usize = 17;
const COMMUNITY_SUBGROUP_MIN_COUNT_1_COL: usize = 18;
const COMMUNITY_SUBGROUP_MAX_COUNT_1_COL: usize = 19;
const COMMUNITY_SUBGROUP_SELECT_RATE_2_COL: usize = 20;
const COMMUNITY_SUBGROUP_MIN_COUNT_2_COL: usize = 21;
const COMMUNITY_SUBGROUP_MAX_COUNT_2_COL: usize = 22;
const COMMUNITY_SUBGROUP_INTIMACY_VARIATION_COL: usize = 23;
const COMMUNITY_SUBGROUP_EXPLORATION_POINT_COL: usize = 24;
const COMMUNITY_SUBGROUP_APPLY_RANDOM_PRICE_COL: usize = 25;
const COMMUNITY_SUBGROUP_RENT_TIME_COL: usize = 26;
const COMMUNITY_SUBGROUP_PRICE_OPTION_COL: usize = 27;

const COMMUNITY_SUBGROUP_OVERLAY_HEADERS: [&str; 32] = [
    "source_id",
    "source_label",
    "source_sha256",
    "source_sheet",
    "source_row",
    "source_spotted_auto",
    "source_comment",
    "source_table",
    "source_grade",
    "source_item_name",
    "source_for_humans",
    "source_removed",
    "source_added",
    "ItemSubGroupKey",
    "ItemKey",
    "EnchantLevel",
    "DoPetAddDrop",
    "DoSechiAddDrop",
    "SelectRate_0",
    "MinCount_0",
    "MaxCount_0",
    "SelectRate_1",
    "MinCount_1",
    "MaxCount_1",
    "SelectRate_2",
    "MinCount_2",
    "MaxCount_2",
    "IntimacyVariation",
    "ExplorationPoint",
    "ApplyRandomPrice",
    "RentTime",
    "PriceOption",
];

const COMMUNITY_SUBGROUP_UNRESOLVED_HEADERS: [&str; 33] = [
    "source_id",
    "source_label",
    "source_sha256",
    "source_sheet",
    "source_row",
    "source_reason",
    "source_item_sub_group_key_raw",
    "source_item_key_raw",
    "source_enchant_level_raw",
    "source_spotted_auto",
    "source_comment",
    "source_table",
    "source_grade",
    "source_item_name",
    "source_for_humans",
    "source_removed",
    "source_added",
    "DoPetAddDrop_raw",
    "DoSechiAddDrop_raw",
    "SelectRate_0_raw",
    "MinCount_0_raw",
    "MaxCount_0_raw",
    "SelectRate_1_raw",
    "MinCount_1_raw",
    "MaxCount_1_raw",
    "SelectRate_2_raw",
    "MinCount_2_raw",
    "MaxCount_2_raw",
    "IntimacyVariation_raw",
    "ExplorationPoint_raw",
    "ApplyRandomPrice_raw",
    "RentTime_raw",
    "PriceOption_raw",
];

#[derive(Parser)]
#[command(name = "fishystuff_dolt_import")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
#[command(rename_all = "kebab-case")]
enum Commands {
    ImportFishingXlsx {
        #[arg(long)]
        dolt_repo: PathBuf,
        #[arg(long)]
        fishing_xlsx: PathBuf,
        #[arg(long)]
        main_group_xlsx: PathBuf,
        #[arg(long)]
        sub_group_xlsx: PathBuf,
        #[arg(long)]
        item_table_xlsx: Option<PathBuf>,
        #[arg(long)]
        fish_table_csv: Option<PathBuf>,
        #[arg(long)]
        patches_csv: Option<PathBuf>,
        #[arg(long)]
        languagedata_en_csv: Option<PathBuf>,
        #[arg(long = "languagedata-csv", value_parser = parse_languagedata_csv_arg)]
        languagedata_csvs: Vec<LanguageDataCsvArg>,
        #[arg(long)]
        output_dir: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = SubsetMode::FishingOnly)]
        subset: SubsetMode,
        #[arg(long, default_value_t = false)]
        commit: bool,
        #[arg(long)]
        commit_msg: Option<String>,
    },
    ImportLanguagedataLoc {
        #[arg(long)]
        dolt_repo: PathBuf,
        #[arg(long = "loc", value_parser = parse_languagedata_loc_arg)]
        locs: Vec<LanguageDataLocArg>,
        #[arg(long)]
        output_dir: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        commit: bool,
        #[arg(long)]
        commit_msg: Option<String>,
    },
    ImportCommunityPrizeFishXlsx {
        #[arg(long)]
        dolt_repo: PathBuf,
        #[arg(long)]
        guessed_rates_workbook_xlsx: Option<PathBuf>,
        #[arg(long)]
        output_dir: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        commit: bool,
        #[arg(long)]
        commit_msg: Option<String>,
    },
    ImportCalculatorEffectsXlsx {
        #[arg(long)]
        dolt_repo: PathBuf,
        #[arg(long)]
        excel_dir: PathBuf,
        #[arg(long)]
        output_dir: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        commit: bool,
        #[arg(long)]
        commit_msg: Option<String>,
    },
    RefreshCalculatorConsumableSourceItems {
        #[arg(long)]
        dolt_repo: PathBuf,
        #[arg(long)]
        output_dir: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        commit: bool,
        #[arg(long)]
        commit_msg: Option<String>,
    },
    ImportCalculatorProgressionXlsx {
        #[arg(long)]
        dolt_repo: PathBuf,
        #[arg(long)]
        excel_dir: PathBuf,
        #[arg(long)]
        output_dir: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        commit: bool,
        #[arg(long)]
        commit_msg: Option<String>,
    },
    ImportFlockfishSubgroupsXlsx {
        #[arg(long)]
        dolt_repo: PathBuf,
        #[arg(long)]
        workbook_xlsx: PathBuf,
        #[arg(long)]
        output_dir: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        commit: bool,
        #[arg(long)]
        commit_msg: Option<String>,
    },
    ImportTradeNpcCatalog {
        #[arg(long)]
        dolt_repo: PathBuf,
        #[arg(long)]
        catalog_json: PathBuf,
        #[arg(long)]
        output_dir: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        commit: bool,
        #[arg(long)]
        commit_msg: Option<String>,
    },
    ImportCommunitySubgroupOverlayXlsx {
        #[arg(long)]
        dolt_repo: PathBuf,
        #[arg(long)]
        subgroups_xlsx: PathBuf,
        #[arg(long, default_value = "no formulas")]
        sheet: String,
        #[arg(long, default_value = DEFAULT_COMMUNITY_SUBGROUP_SOURCE_ID)]
        source_id: String,
        #[arg(long, default_value = DEFAULT_COMMUNITY_SUBGROUP_SOURCE_LABEL)]
        source_label: String,
        #[arg(long)]
        output_dir: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        emit_only: bool,
        #[arg(long, default_value_t = false)]
        activate: bool,
        #[arg(long, default_value_t = false)]
        commit: bool,
        #[arg(long)]
        commit_msg: Option<String>,
    },
    UpsertCommunityZoneFishPresence {
        #[arg(long)]
        dolt_repo: Option<PathBuf>,
        #[arg(long)]
        zone_name: String,
        #[arg(long)]
        fish_name: Option<String>,
        #[arg(long)]
        item_id: Option<i64>,
        #[arg(long, value_enum, default_value_t = ManualCommunityPresenceStatus::Confirmed)]
        support_status: ManualCommunityPresenceStatus,
        #[arg(long, default_value_t = 1)]
        claim_count: u32,
        #[arg(long)]
        slot_idx: Option<u8>,
        #[arg(long, value_enum)]
        group: Option<CommunityFishGroup>,
        #[arg(long)]
        subgroup_key: Option<i64>,
        #[arg(long, default_value_t = false)]
        commit: bool,
        #[arg(long)]
        commit_msg: Option<String>,
    },
    UpsertCommunityZoneFishGuess {
        #[arg(long)]
        dolt_repo: Option<PathBuf>,
        #[arg(long)]
        zone_name: String,
        #[arg(long)]
        fish_name: Option<String>,
        #[arg(long)]
        item_id: Option<i64>,
        #[arg(long)]
        guessed_rate_pct: f64,
        #[arg(long)]
        slot_idx: Option<u8>,
        #[arg(long, value_enum)]
        group: Option<CommunityFishGroup>,
        #[arg(long)]
        subgroup_key: Option<i64>,
        #[arg(long, default_value_t = false)]
        commit: bool,
        #[arg(long)]
        commit_msg: Option<String>,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
#[clap(rename_all = "kebab-case")]
enum SubsetMode {
    FishingOnly,
    All,
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
#[clap(rename_all = "kebab-case")]
enum ManualCommunityPresenceStatus {
    Confirmed,
    Unconfirmed,
    DataIncomplete,
}

impl ManualCommunityPresenceStatus {
    fn as_db_value(self) -> &'static str {
        match self {
            Self::Confirmed => "confirmed",
            Self::Unconfirmed => "unconfirmed",
            Self::DataIncomplete => "data_incomplete",
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
#[clap(rename_all = "kebab-case")]
enum CommunityFishGroup {
    Prize,
    Rare,
    HighQuality,
    General,
    Trash,
}

impl CommunityFishGroup {
    fn slot_idx(self) -> u8 {
        match self {
            Self::Prize => 1,
            Self::Rare => 2,
            Self::HighQuality => 3,
            Self::General => 4,
            Self::Trash => 5,
        }
    }
}

struct FishingImport {
    row_count: usize,
    mg_keys: BTreeSet<i64>,
}

struct MainGroupImport {
    row_count: usize,
    sg_keys: BTreeSet<i64>,
    matched_mg: BTreeSet<i64>,
}

struct SubGroupImport {
    row_count: usize,
    matched_sg: BTreeSet<i64>,
}

struct ItemTableImport {
    row_count: usize,
}

struct LanguageDataImport {
    row_count: usize,
}

#[derive(Debug, Clone)]
struct LanguageDataCsvArg {
    lang: String,
    path: PathBuf,
}

#[derive(Debug, Clone)]
struct LanguageDataLocArg {
    lang: String,
    path: PathBuf,
}

struct FishTableImport {
    row_count: usize,
}

struct PatchesImport {
    row_count: usize,
}

struct ImportCommand {
    dolt_repo: PathBuf,
    fishing_xlsx: PathBuf,
    main_group_xlsx: PathBuf,
    sub_group_xlsx: PathBuf,
    item_table_xlsx: Option<PathBuf>,
    fish_table_csv: Option<PathBuf>,
    patches_csv: Option<PathBuf>,
    languagedata_en_csv: Option<PathBuf>,
    languagedata_csvs: Vec<LanguageDataCsvArg>,
    output_dir: Option<PathBuf>,
    subset: SubsetMode,
    commit: bool,
    commit_msg: Option<String>,
}

struct ImportLanguageDataLocCommand {
    dolt_repo: PathBuf,
    locs: Vec<LanguageDataLocArg>,
    output_dir: Option<PathBuf>,
    commit: bool,
    commit_msg: Option<String>,
}

struct ImportDigests {
    fishing_sha: String,
    main_group_sha: String,
    sub_group_sha: String,
    item_table_sha: Option<String>,
    fish_table_sha: Option<String>,
    patches_sha: Option<String>,
    languagedata_shas: BTreeMap<String, String>,
}

struct ImportOutputs {
    fishing_csv: PathBuf,
    main_group_csv: PathBuf,
    sub_group_csv: PathBuf,
    item_table_csv: PathBuf,
    fish_table_csv: PathBuf,
    patches_csv: PathBuf,
    languagedata_csvs: BTreeMap<String, PathBuf>,
}

struct CommunityPrizeGuessImport {
    emitted_rows: usize,
    resolved_item_keys: usize,
    matched_names: usize,
    unresolved_names: usize,
    unresolved_zones: usize,
    subgroup_mapped_rows: usize,
}

struct CommunityPrizeImportCommand {
    dolt_repo: PathBuf,
    guessed_rates_workbook_xlsx: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    commit: bool,
    commit_msg: Option<String>,
}

struct CommunityPrizeOutputs {
    community_csv: PathBuf,
}

struct ManualCommunityPresenceCommand {
    dolt_repo: Option<PathBuf>,
    zone_name: String,
    fish_name: Option<String>,
    item_id: Option<i64>,
    support_status: ManualCommunityPresenceStatus,
    claim_count: u32,
    slot_idx: Option<u8>,
    group: Option<CommunityFishGroup>,
    subgroup_key: Option<i64>,
    commit: bool,
    commit_msg: Option<String>,
}

struct ManualCommunityGuessCommand {
    dolt_repo: Option<PathBuf>,
    zone_name: String,
    fish_name: Option<String>,
    item_id: Option<i64>,
    guessed_rate_pct: f64,
    slot_idx: Option<u8>,
    group: Option<CommunityFishGroup>,
    subgroup_key: Option<i64>,
    commit: bool,
    commit_msg: Option<String>,
}

#[derive(Debug, Clone)]
struct ResolvedZone {
    zone_rgb: u32,
    zone_r: u8,
    zone_g: u8,
    zone_b: u8,
    region_name: String,
    zone_name: String,
}

#[derive(Debug, Clone)]
struct ResolvedFish {
    item_id: i64,
    fish_name: String,
}

#[derive(Debug, Clone)]
struct ResolvedZoneSlot {
    slot_idx: u8,
    item_main_group_key: i64,
    subgroup_keys: Vec<i64>,
}

struct RawTableImport {
    row_count: usize,
}

struct CalculatorConsumableSourceItemEffectEvidenceImport {
    row_count: usize,
}

#[derive(Debug, Clone)]
struct CalculatorConsumableSkillRow {
    skill_no: String,
    buff_ids: Vec<String>,
}

#[derive(Debug, Clone)]
struct CalculatorConsumableItemSourceRow {
    item_id: i64,
    item_classify: Option<String>,
    skill_no: Option<String>,
    sub_skill_no: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct CalculatorConsumableBuffCategory {
    category_id: Option<i32>,
    category_level: Option<i32>,
}

#[derive(Debug, Clone)]
struct CalculatorConsumableBuffText {
    text: String,
    has_description: bool,
}

struct FlockfishSubgroupImportCommand {
    dolt_repo: PathBuf,
    workbook_xlsx: PathBuf,
    output_dir: Option<PathBuf>,
    commit: bool,
    commit_msg: Option<String>,
}

struct TradeNpcCatalogImportCommand {
    dolt_repo: PathBuf,
    catalog_json: PathBuf,
    output_dir: Option<PathBuf>,
    commit: bool,
    commit_msg: Option<String>,
}

struct FlockfishTableImportStats {
    row_count: usize,
}

struct FlockfishGroupsImport {
    main_group: FlockfishTableImportStats,
    sub_group: FlockfishTableImportStats,
    zone_group_slots: FlockfishZoneGroupSlotsImport,
}

struct FlockfishSubgroupOutputs {
    main_group_csv: PathBuf,
    sub_group_csv: PathBuf,
    zone_group_slots_csv: PathBuf,
}

struct FlockfishZoneGroupSlotsImport {
    row_count: usize,
    numeric_rows: usize,
    unresolved_rows: usize,
}

struct TradeNpcCatalogOutputs {
    meta_csv: PathBuf,
    sources_csv: PathBuf,
    origin_regions_csv: PathBuf,
    zone_origin_regions_csv: PathBuf,
    destinations_csv: PathBuf,
    excluded_csv: PathBuf,
}

struct CommunitySubgroupOverlayImportCommand {
    dolt_repo: PathBuf,
    subgroups_xlsx: PathBuf,
    sheet: String,
    source_id: String,
    source_label: String,
    output_dir: Option<PathBuf>,
    emit_only: bool,
    activate: bool,
    commit: bool,
    commit_msg: Option<String>,
}

struct CommunitySubgroupOverlayOutputs {
    overlay_csv: PathBuf,
    unresolved_csv: PathBuf,
}

struct CommunitySubgroupOverlayImport {
    row_count: usize,
    active_rows: usize,
    removed_rows: usize,
    added_rows: usize,
    note_rows: usize,
    unresolved_rows: usize,
    unresolved_symbolic_key_rows: usize,
    unresolved_missing_item_key_rows: usize,
}

struct CalculatorEffectsImportCommand {
    dolt_repo: PathBuf,
    excel_dir: PathBuf,
    output_dir: Option<PathBuf>,
    commit: bool,
    commit_msg: Option<String>,
}

struct RefreshCalculatorConsumableSourceItemsCommand {
    dolt_repo: PathBuf,
    output_dir: Option<PathBuf>,
    commit: bool,
    commit_msg: Option<String>,
}

struct CalculatorEffectsWorkbookSet {
    buff_table_xlsx: PathBuf,
    common_stat_data_xlsx: PathBuf,
    fishing_stat_data_xlsx: PathBuf,
    skill_table_new_xlsx: PathBuf,
    skilltype_table_new_xlsx: PathBuf,
    lightstone_set_option_xlsx: PathBuf,
    translate_stat_xlsx: PathBuf,
    enchant_cash_xlsx: PathBuf,
    enchant_equipment_xlsx: PathBuf,
    enchant_lifeequipment_xlsx: PathBuf,
    tooltip_table_xlsx: PathBuf,
    producttool_property_xlsx: PathBuf,
    pet_table_xlsx: PathBuf,
    pet_skill_table_xlsx: PathBuf,
    pet_base_skill_table_xlsx: PathBuf,
    pet_setstats_table_xlsx: PathBuf,
    pet_equipskill_table_xlsx: PathBuf,
    pet_equipskill_aquire_table_xlsx: PathBuf,
    pet_grade_table_xlsx: PathBuf,
    pet_exp_table_xlsx: PathBuf,
    upgradepet_looting_percent_xlsx: PathBuf,
}

struct CalculatorEffectsOutputs {
    buff_table_csv: PathBuf,
    common_stat_data_csv: PathBuf,
    fishing_stat_data_csv: PathBuf,
    skill_table_new_csv: PathBuf,
    skilltype_table_new_csv: PathBuf,
    lightstone_set_option_csv: PathBuf,
    translate_stat_csv: PathBuf,
    enchant_cash_csv: PathBuf,
    enchant_equipment_csv: PathBuf,
    enchant_lifeequipment_csv: PathBuf,
    tooltip_table_csv: PathBuf,
    producttool_property_csv: PathBuf,
    pet_table_csv: PathBuf,
    pet_skill_table_csv: PathBuf,
    pet_base_skill_table_csv: PathBuf,
    pet_setstats_table_csv: PathBuf,
    pet_equipskill_table_csv: PathBuf,
    pet_equipskill_aquire_table_csv: PathBuf,
    pet_grade_table_csv: PathBuf,
    pet_exp_table_csv: PathBuf,
    upgradepet_looting_percent_csv: PathBuf,
    consumable_source_item_effect_evidence_csv: PathBuf,
}

struct CalculatorEffectsDigests {
    buff_table_sha: String,
    common_stat_data_sha: String,
    fishing_stat_data_sha: String,
    skill_table_new_sha: String,
    skilltype_table_new_sha: String,
    lightstone_set_option_sha: String,
    translate_stat_sha: String,
    enchant_cash_sha: String,
    enchant_equipment_sha: String,
    enchant_lifeequipment_sha: String,
    tooltip_table_sha: String,
    producttool_property_sha: String,
    pet_table_sha: String,
    pet_skill_table_sha: String,
    pet_base_skill_table_sha: String,
    pet_setstats_table_sha: String,
    pet_equipskill_table_sha: String,
    pet_equipskill_aquire_table_sha: String,
    pet_grade_table_sha: String,
    pet_exp_table_sha: String,
    upgradepet_looting_percent_sha: String,
}

struct CalculatorProgressionImportCommand {
    dolt_repo: PathBuf,
    excel_dir: PathBuf,
    output_dir: Option<PathBuf>,
    commit: bool,
    commit_msg: Option<String>,
}

struct CalculatorProgressionWorkbookSet {
    common_stat_data_xlsx: PathBuf,
    fishing_stat_data_xlsx: PathBuf,
    translate_stat_xlsx: PathBuf,
}

struct CalculatorProgressionOutputs {
    common_stat_data_csv: PathBuf,
    fishing_stat_data_csv: PathBuf,
    translate_stat_csv: PathBuf,
}

struct CalculatorProgressionDigests {
    common_stat_data_sha: String,
    fishing_stat_data_sha: String,
    translate_stat_sha: String,
}

struct ImportReport<'a> {
    subset: SubsetMode,
    fishing: &'a FishingImport,
    main_group: &'a MainGroupImport,
    sub_group: &'a SubGroupImport,
    item_table: Option<&'a ItemTableImport>,
    fish_table: Option<&'a FishTableImport>,
    patches: Option<&'a PatchesImport>,
    languagedata: &'a BTreeMap<String, LanguageDataImport>,
    outputs: &'a ImportOutputs,
}

fn parse_languagedata_csv_arg(value: &str) -> std::result::Result<LanguageDataCsvArg, String> {
    let (raw_lang, raw_path) = value
        .split_once('=')
        .or_else(|| value.split_once(':'))
        .map(|(lang, path)| (Some(lang), path))
        .unwrap_or((None, value));
    let path = PathBuf::from(raw_path);
    let lang = match raw_lang {
        Some(lang) => normalize_languagedata_lang(lang)?,
        None => infer_languagedata_lang(&path)?,
    };
    Ok(LanguageDataCsvArg { lang, path })
}

fn parse_languagedata_loc_arg(value: &str) -> std::result::Result<LanguageDataLocArg, String> {
    let (raw_lang, raw_path) = value
        .split_once('=')
        .or_else(|| value.split_once(':'))
        .map(|(lang, path)| (Some(lang), path))
        .unwrap_or((None, value));
    let path = PathBuf::from(raw_path);
    let lang = match raw_lang {
        Some(lang) => normalize_languagedata_lang(lang)?,
        None => infer_languagedata_lang_with_extension(&path, "loc")?,
    };
    Ok(LanguageDataLocArg { lang, path })
}

fn infer_languagedata_lang(path: &Path) -> std::result::Result<String, String> {
    infer_languagedata_lang_with_extension(path, "csv")
}

fn infer_languagedata_lang_with_extension(
    path: &Path,
    extension: &str,
) -> std::result::Result<String, String> {
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("cannot infer language from {}", path.display()))?;
    if path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| !value.eq_ignore_ascii_case(extension))
    {
        return Err(format!(
            "expected .{extension} file for languagedata input: {}",
            path.display()
        ));
    }
    let lang = stem
        .strip_prefix("languagedata_")
        .ok_or_else(|| format!("expected filename like languagedata_<lang>.{extension}: {stem}"))?;
    normalize_languagedata_lang(lang)
}

fn normalize_languagedata_lang(value: &str) -> std::result::Result<String, String> {
    let lang = value.trim().to_string();
    if lang.is_empty() {
        return Err("language code cannot be empty".to_string());
    }
    if !lang
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
    {
        return Err(format!("unsupported language code: {value}"));
    }
    Ok(lang)
}

fn languagedata_output_file_name(lang: &str) -> String {
    format!("languagedata_{lang}.csv")
}

fn collect_languagedata_inputs(
    languagedata_en_csv: Option<PathBuf>,
    languagedata_csvs: Vec<LanguageDataCsvArg>,
) -> Result<BTreeMap<String, PathBuf>> {
    let mut inputs = BTreeMap::<String, PathBuf>::new();
    if let Some(path) = languagedata_en_csv {
        inputs.insert("en".to_string(), path);
    }
    for input in languagedata_csvs {
        if inputs
            .insert(input.lang.clone(), input.path.clone())
            .is_some()
        {
            bail!("duplicate languagedata CSV for language {}", input.lang);
        }
    }
    Ok(inputs)
}

fn collect_languagedata_loc_inputs(
    locs: Vec<LanguageDataLocArg>,
) -> Result<BTreeMap<String, PathBuf>> {
    let mut inputs = BTreeMap::<String, PathBuf>::new();
    for input in locs {
        if inputs
            .insert(input.lang.clone(), input.path.clone())
            .is_some()
        {
            bail!("duplicate languagedata LOC for language {}", input.lang);
        }
    }
    Ok(inputs)
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::ImportFishingXlsx {
            dolt_repo,
            fishing_xlsx,
            main_group_xlsx,
            sub_group_xlsx,
            item_table_xlsx,
            fish_table_csv,
            patches_csv,
            languagedata_en_csv,
            languagedata_csvs,
            output_dir,
            subset,
            commit,
            commit_msg,
        } => run_import(ImportCommand {
            dolt_repo,
            fishing_xlsx,
            main_group_xlsx,
            sub_group_xlsx,
            item_table_xlsx,
            fish_table_csv,
            patches_csv,
            languagedata_en_csv,
            languagedata_csvs,
            output_dir,
            subset,
            commit,
            commit_msg,
        }),
        Commands::ImportLanguagedataLoc {
            dolt_repo,
            locs,
            output_dir,
            commit,
            commit_msg,
        } => run_languagedata_loc_import(ImportLanguageDataLocCommand {
            dolt_repo,
            locs,
            output_dir,
            commit,
            commit_msg,
        }),
        Commands::ImportCommunityPrizeFishXlsx {
            dolt_repo,
            guessed_rates_workbook_xlsx,
            output_dir,
            commit,
            commit_msg,
        } => run_community_prize_import(CommunityPrizeImportCommand {
            dolt_repo,
            guessed_rates_workbook_xlsx,
            output_dir,
            commit,
            commit_msg,
        }),
        Commands::ImportCalculatorEffectsXlsx {
            dolt_repo,
            excel_dir,
            output_dir,
            commit,
            commit_msg,
        } => run_calculator_effects_import(CalculatorEffectsImportCommand {
            dolt_repo,
            excel_dir,
            output_dir,
            commit,
            commit_msg,
        }),
        Commands::RefreshCalculatorConsumableSourceItems {
            dolt_repo,
            output_dir,
            commit,
            commit_msg,
        } => run_refresh_calculator_consumable_source_items(
            RefreshCalculatorConsumableSourceItemsCommand {
                dolt_repo,
                output_dir,
                commit,
                commit_msg,
            },
        ),
        Commands::ImportCalculatorProgressionXlsx {
            dolt_repo,
            excel_dir,
            output_dir,
            commit,
            commit_msg,
        } => run_calculator_progression_import(CalculatorProgressionImportCommand {
            dolt_repo,
            excel_dir,
            output_dir,
            commit,
            commit_msg,
        }),
        Commands::ImportFlockfishSubgroupsXlsx {
            dolt_repo,
            workbook_xlsx,
            output_dir,
            commit,
            commit_msg,
        } => run_flockfish_subgroup_import(FlockfishSubgroupImportCommand {
            dolt_repo,
            workbook_xlsx,
            output_dir,
            commit,
            commit_msg,
        }),
        Commands::ImportTradeNpcCatalog {
            dolt_repo,
            catalog_json,
            output_dir,
            commit,
            commit_msg,
        } => run_trade_npc_catalog_import(TradeNpcCatalogImportCommand {
            dolt_repo,
            catalog_json,
            output_dir,
            commit,
            commit_msg,
        }),
        Commands::ImportCommunitySubgroupOverlayXlsx {
            dolt_repo,
            subgroups_xlsx,
            sheet,
            source_id,
            source_label,
            output_dir,
            emit_only,
            activate,
            commit,
            commit_msg,
        } => run_community_subgroup_overlay_import(CommunitySubgroupOverlayImportCommand {
            dolt_repo,
            subgroups_xlsx,
            sheet,
            source_id,
            source_label,
            output_dir,
            emit_only,
            activate,
            commit,
            commit_msg,
        }),
        Commands::UpsertCommunityZoneFishPresence {
            dolt_repo,
            zone_name,
            fish_name,
            item_id,
            support_status,
            claim_count,
            slot_idx,
            group,
            subgroup_key,
            commit,
            commit_msg,
        } => run_manual_community_presence_upsert(ManualCommunityPresenceCommand {
            dolt_repo,
            zone_name,
            fish_name,
            item_id,
            support_status,
            claim_count,
            slot_idx,
            group,
            subgroup_key,
            commit,
            commit_msg,
        }),
        Commands::UpsertCommunityZoneFishGuess {
            dolt_repo,
            zone_name,
            fish_name,
            item_id,
            guessed_rate_pct,
            slot_idx,
            group,
            subgroup_key,
            commit,
            commit_msg,
        } => run_manual_community_guess_upsert(ManualCommunityGuessCommand {
            dolt_repo,
            zone_name,
            fish_name,
            item_id,
            guessed_rate_pct,
            slot_idx,
            group,
            subgroup_key,
            commit,
            commit_msg,
        }),
    }
}

fn run_import(command: ImportCommand) -> Result<()> {
    let ImportCommand {
        dolt_repo,
        fishing_xlsx,
        main_group_xlsx,
        sub_group_xlsx,
        item_table_xlsx,
        fish_table_csv,
        patches_csv,
        languagedata_en_csv,
        languagedata_csvs,
        output_dir,
        subset,
        commit,
        commit_msg,
    } = command;
    let languagedata_inputs = collect_languagedata_inputs(languagedata_en_csv, languagedata_csvs)?;

    let output_dir = match output_dir {
        Some(path) => path,
        None => default_output_dir()?,
    };
    fs::create_dir_all(&output_dir)
        .with_context(|| format!("create output dir: {}", output_dir.display()))?;

    let digests = ImportDigests {
        fishing_sha: sha256_file(&fishing_xlsx)?,
        main_group_sha: sha256_file(&main_group_xlsx)?,
        sub_group_sha: sha256_file(&sub_group_xlsx)?,
        item_table_sha: match item_table_xlsx.as_ref() {
            Some(path) => Some(sha256_file(path)?),
            None => None,
        },
        fish_table_sha: match fish_table_csv.as_ref() {
            Some(path) => Some(sha256_file(path)?),
            None => None,
        },
        patches_sha: match patches_csv.as_ref() {
            Some(path) => Some(sha256_file(path)?),
            None => None,
        },
        languagedata_shas: languagedata_inputs
            .iter()
            .map(|(lang, path)| Ok((lang.clone(), sha256_file(path)?)))
            .collect::<Result<BTreeMap<_, _>>>()?,
    };

    let languagedata_csvs = languagedata_inputs
        .keys()
        .map(|lang| {
            (
                lang.clone(),
                output_dir.join(languagedata_output_file_name(lang)),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let outputs = ImportOutputs {
        fishing_csv: output_dir.join("fishing_table.csv"),
        main_group_csv: output_dir.join("item_main_group_table.csv"),
        sub_group_csv: output_dir.join("item_sub_group_table.csv"),
        item_table_csv: output_dir.join("item_table.csv"),
        fish_table_csv: output_dir.join("fish_table.csv"),
        patches_csv: output_dir.join("patches.csv"),
        languagedata_csvs,
    };

    let fishing_stats = import_fishing_table(&fishing_xlsx, &outputs.fishing_csv)?;
    let main_group_stats = import_main_group_table(
        &main_group_xlsx,
        &outputs.main_group_csv,
        subset,
        &fishing_stats.mg_keys,
    )?;
    let sub_group_stats = import_sub_group_table(
        &sub_group_xlsx,
        &outputs.sub_group_csv,
        subset,
        &main_group_stats.sg_keys,
    )?;
    let item_table_stats = match item_table_xlsx.as_ref() {
        Some(path) => Some(import_item_table(path, &outputs.item_table_csv)?),
        None => None,
    };
    let fish_table_stats = match fish_table_csv.as_ref() {
        Some(path) => Some(import_fish_table_csv(path, &outputs.fish_table_csv)?),
        None => None,
    };
    let patches_stats = match patches_csv.as_ref() {
        Some(path) => Some(import_patches_csv(path, &outputs.patches_csv)?),
        None => None,
    };
    let mut languagedata_stats = BTreeMap::<String, LanguageDataImport>::new();
    for (lang, input_path) in &languagedata_inputs {
        let output_path = outputs
            .languagedata_csvs
            .get(lang)
            .expect("languagedata output path should exist for input");
        languagedata_stats.insert(
            lang.clone(),
            import_languagedata_csv(input_path, output_path, lang)?,
        );
    }

    run_dolt_table_import(&dolt_repo, "fishing_table", &outputs.fishing_csv)?;
    run_dolt_table_import_or_sql_server(
        &dolt_repo,
        "item_main_group_table",
        &outputs.main_group_csv,
    )?;
    run_dolt_table_import_or_sql_server(
        &dolt_repo,
        "item_sub_group_table",
        &outputs.sub_group_csv,
    )?;
    if item_table_stats.is_some() {
        run_dolt_table_import(&dolt_repo, "item_table", &outputs.item_table_csv)?;
    }
    if fish_table_stats.is_some() {
        run_dolt_table_import(&dolt_repo, "fish_table", &outputs.fish_table_csv)?;
    }
    if patches_stats.is_some() {
        run_dolt_table_import(&dolt_repo, "patches", &outputs.patches_csv)?;
    }
    replace_languagedata_languages(&dolt_repo, &outputs.languagedata_csvs)?;
    ensure_calculator_lookup_indexes(&dolt_repo)?;

    if commit {
        let msg = build_commit_message(commit_msg, &digests);
        run_dolt_commit(&dolt_repo, &msg)?;
    }

    report_import(ImportReport {
        subset,
        fishing: &fishing_stats,
        main_group: &main_group_stats,
        sub_group: &sub_group_stats,
        item_table: item_table_stats.as_ref(),
        fish_table: fish_table_stats.as_ref(),
        patches: patches_stats.as_ref(),
        languagedata: &languagedata_stats,
        outputs: &outputs,
    });

    Ok(())
}

fn run_languagedata_loc_import(command: ImportLanguageDataLocCommand) -> Result<()> {
    let ImportLanguageDataLocCommand {
        dolt_repo,
        locs,
        output_dir,
        commit,
        commit_msg,
    } = command;
    let loc_inputs = collect_languagedata_loc_inputs(locs)?;
    if loc_inputs.is_empty() {
        bail!("at least one --loc input is required");
    }

    let output_dir = match output_dir {
        Some(path) => path,
        None => default_output_dir()?,
    };
    fs::create_dir_all(&output_dir)
        .with_context(|| format!("create output dir: {}", output_dir.display()))?;

    let mut loc_shas = BTreeMap::<String, String>::new();
    let mut stats = BTreeMap::<String, LanguageDataImport>::new();
    let mut outputs = BTreeMap::<String, PathBuf>::new();
    for (lang, input_path) in &loc_inputs {
        loc_shas.insert(lang.clone(), sha256_file(input_path)?);
        let output_path = output_dir.join(languagedata_output_file_name(lang));
        stats.insert(
            lang.clone(),
            import_languagedata_loc(input_path, &output_path, lang)?,
        );
        outputs.insert(lang.clone(), output_path);
    }

    replace_languagedata_languages(&dolt_repo, &outputs)?;
    ensure_calculator_lookup_indexes(&dolt_repo)?;

    if commit {
        let message = build_languagedata_loc_commit_message(commit_msg, &loc_shas);
        run_dolt_commit(&dolt_repo, &message)?;
    }

    for (lang, stat) in &stats {
        println!("languagedata_{lang} rows emitted: {}", stat.row_count);
    }
    for (lang, output) in &outputs {
        println!("output languagedata_{lang} csv: {}", output.display());
    }

    Ok(())
}

fn run_community_prize_import(command: CommunityPrizeImportCommand) -> Result<()> {
    let CommunityPrizeImportCommand {
        dolt_repo,
        guessed_rates_workbook_xlsx,
        output_dir,
        commit,
        commit_msg,
    } = command;

    let output_dir = match output_dir {
        Some(path) => path,
        None => default_output_dir()?,
    };
    fs::create_dir_all(&output_dir)
        .with_context(|| format!("create output dir: {}", output_dir.display()))?;

    let outputs = CommunityPrizeOutputs {
        community_csv: output_dir.join("community_zone_fish_support.csv"),
    };
    let mut writer = build_csv_writer(&outputs.community_csv)?;
    writer.write_record(COMMUNITY_ZONE_FISH_SUPPORT_HEADERS)?;
    writer.flush()?;
    let guessed_sha = match guessed_rates_workbook_xlsx.as_ref() {
        Some(path) => Some(sha256_file(path)?),
        None => None,
    };
    let guess_stats = match guessed_rates_workbook_xlsx.as_ref() {
        Some(path) => Some(append_community_prize_guess_rows(
            &dolt_repo,
            path,
            guessed_sha.as_deref().unwrap_or_default(),
            &outputs.community_csv,
        )?),
        None => None,
    };
    run_dolt_sql_table_import_or_remote(
        &dolt_repo,
        "community_zone_fish_support",
        &outputs.community_csv,
    )?;

    if commit {
        let msg = match commit_msg {
            Some(msg) => build_community_prize_commit_message(&msg, guessed_sha.as_deref()),
            None => build_community_prize_commit_message(
                "Import community zone fish support",
                guessed_sha.as_deref(),
            ),
        };
        run_dolt_commit(&dolt_repo, &msg)?;
    }

    if let Some(stats) = guess_stats.as_ref() {
        println!(
            "community guessed prize rows emitted: {}",
            stats.emitted_rows
        );
        println!(
            "community guessed prize direct item keys resolved: {}",
            stats.resolved_item_keys
        );
        println!(
            "community guessed prize name lookups resolved: {}",
            stats.matched_names
        );
        println!(
            "community guessed prize unresolved names skipped: {}",
            stats.unresolved_names
        );
        println!(
            "community guessed prize unresolved zones skipped: {}",
            stats.unresolved_zones
        );
        println!(
            "community guessed prize rows with subgroup mapping: {}",
            stats.subgroup_mapped_rows
        );
    }
    println!("output community csv: {}", outputs.community_csv.display());

    Ok(())
}

fn build_community_prize_commit_message(prefix: &str, guessed_sha: Option<&str>) -> String {
    match guessed_sha {
        Some(guessed_sha) => format!("{prefix} (FishingSetupWorkbook={guessed_sha})"),
        None => prefix.to_string(),
    }
}

fn run_manual_community_presence_upsert(command: ManualCommunityPresenceCommand) -> Result<()> {
    let ManualCommunityPresenceCommand {
        dolt_repo,
        zone_name,
        fish_name,
        item_id,
        support_status,
        claim_count,
        slot_idx,
        group,
        subgroup_key,
        commit,
        commit_msg,
    } = command;
    let dolt_repo = resolve_dolt_repo_path(dolt_repo)?;

    validate_manual_fish_reference(fish_name.as_deref(), item_id)?;
    let resolved_zone = resolve_zone_by_name(&dolt_repo, &zone_name)?;
    let resolved_fish = resolve_fish_reference(&dolt_repo, fish_name.as_deref(), item_id)?;
    let resolved_slot_idx = resolve_requested_slot_idx(slot_idx, group, None)?;
    let resolved_slot = match resolved_slot_idx {
        Some(slot_idx) => Some(resolve_zone_slot(
            &dolt_repo,
            resolved_zone.zone_rgb,
            slot_idx,
        )?),
        None => None,
    };
    if subgroup_key.is_some() && resolved_slot.is_none() {
        bail!("--subgroup-key requires --slot-idx or --group so the zone slot lineage can be verified");
    }
    if let (Some(subgroup_key), Some(resolved_slot)) = (subgroup_key, resolved_slot.as_ref()) {
        if !resolved_slot.subgroup_keys.contains(&subgroup_key) {
            bail!(
                "subgroup_key {} does not belong to zone '{}' slot {}",
                subgroup_key,
                resolved_zone.zone_name,
                resolved_slot.slot_idx
            );
        }
    }

    ensure_community_zone_fish_support_table(&dolt_repo)?;

    let notes = format_manual_community_notes(
        resolved_slot.as_ref().map(|slot| slot.slot_idx),
        None,
        resolved_slot.as_ref().map(|slot| slot.item_main_group_key),
        subgroup_key,
    );
    let query = build_community_zone_fish_support_upsert_query(
        MANUAL_COMMUNITY_PRESENCE_SOURCE_ID,
        MANUAL_COMMUNITY_PRESENCE_SOURCE_LABEL,
        &resolved_zone,
        &resolved_fish,
        support_status.as_db_value(),
        claim_count,
        notes.as_deref(),
    );
    run_dolt_sql_query_or_remote(&dolt_repo, &query, "upsert manual community presence row")?;

    if commit {
        let message = commit_msg.unwrap_or_else(|| {
            format!(
                "Upsert manual community presence for {} in {}",
                resolved_fish.fish_name, resolved_zone.zone_name
            )
        });
        run_dolt_commit(&dolt_repo, &message)?;
    }

    println!(
        "upserted presence row: zone='{}' item_id={} fish='{}' status={} claim_count={}",
        resolved_zone.zone_name,
        resolved_fish.item_id,
        resolved_fish.fish_name,
        support_status.as_db_value(),
        claim_count
    );

    Ok(())
}

fn run_manual_community_guess_upsert(command: ManualCommunityGuessCommand) -> Result<()> {
    let ManualCommunityGuessCommand {
        dolt_repo,
        zone_name,
        fish_name,
        item_id,
        guessed_rate_pct,
        slot_idx,
        group,
        subgroup_key,
        commit,
        commit_msg,
    } = command;
    let dolt_repo = resolve_dolt_repo_path(dolt_repo)?;

    validate_manual_fish_reference(fish_name.as_deref(), item_id)?;
    if !(guessed_rate_pct.is_finite() && guessed_rate_pct > 0.0) {
        bail!("--guessed-rate-pct must be a positive finite number");
    }

    let resolved_zone = resolve_zone_by_name(&dolt_repo, &zone_name)?;
    let resolved_fish = resolve_fish_reference(&dolt_repo, fish_name.as_deref(), item_id)?;
    let resolved_slot_idx = resolve_requested_slot_idx(slot_idx, group, Some(1))?
        .ok_or_else(|| anyhow::anyhow!("manual community guess requires a slot"))?;
    let resolved_slot = resolve_zone_slot(&dolt_repo, resolved_zone.zone_rgb, resolved_slot_idx)?;
    let resolved_subgroup_key = match subgroup_key {
        Some(subgroup_key) => {
            if !resolved_slot.subgroup_keys.contains(&subgroup_key) {
                bail!(
                    "subgroup_key {} does not belong to zone '{}' slot {}",
                    subgroup_key,
                    resolved_zone.zone_name,
                    resolved_slot.slot_idx
                );
            }
            Some(subgroup_key)
        }
        None if resolved_slot.subgroup_keys.len() == 1 => resolved_slot.subgroup_keys.first().copied(),
        None if resolved_slot.subgroup_keys.is_empty() => None,
        None => bail!(
            "zone '{}' slot {} has multiple subgroup options ({}); pass --subgroup-key to disambiguate",
            resolved_zone.zone_name,
            resolved_slot.slot_idx,
            resolved_slot
                .subgroup_keys
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    };

    ensure_community_zone_fish_support_table(&dolt_repo)?;

    let guessed_rate = guessed_rate_pct / 100.0;
    let notes = format_manual_community_notes(
        Some(resolved_slot.slot_idx),
        Some(guessed_rate),
        Some(resolved_slot.item_main_group_key),
        resolved_subgroup_key,
    );
    let query = build_community_zone_fish_support_upsert_query(
        MANUAL_COMMUNITY_GUESS_SOURCE_ID,
        MANUAL_COMMUNITY_GUESS_SOURCE_LABEL,
        &resolved_zone,
        &resolved_fish,
        "guessed",
        0,
        notes.as_deref(),
    );
    run_dolt_sql_query_or_remote(&dolt_repo, &query, "upsert manual community guess row")?;

    if commit {
        let message = commit_msg.unwrap_or_else(|| {
            format!(
                "Upsert manual community guess for {} in {}",
                resolved_fish.fish_name, resolved_zone.zone_name
            )
        });
        run_dolt_commit(&dolt_repo, &message)?;
    }

    println!(
        "upserted guess row: zone='{}' item_id={} fish='{}' guessed_rate_pct={} slot_idx={} subgroup_key={}",
        resolved_zone.zone_name,
        resolved_fish.item_id,
        resolved_fish.fish_name,
        format_float(guessed_rate_pct),
        resolved_slot.slot_idx,
        resolved_subgroup_key
            .map(|value| value.to_string())
            .unwrap_or_else(|| "none".to_string())
    );

    Ok(())
}

fn validate_manual_fish_reference(fish_name: Option<&str>, item_id: Option<i64>) -> Result<()> {
    let has_fish_name = fish_name.is_some_and(|value| !value.trim().is_empty());
    let has_item_id = item_id.is_some();
    if has_fish_name || has_item_id {
        return Ok(());
    }
    bail!("provide either --fish-name or --item-id");
}

fn validate_slot_idx(slot_idx: u8) -> Result<()> {
    if (1..=5).contains(&slot_idx) {
        Ok(())
    } else {
        bail!("slot_idx must be between 1 and 5")
    }
}

fn resolve_requested_slot_idx(
    slot_idx: Option<u8>,
    group: Option<CommunityFishGroup>,
    default_slot_idx: Option<u8>,
) -> Result<Option<u8>> {
    if let Some(slot_idx) = slot_idx {
        validate_slot_idx(slot_idx)?;
    }

    let group_slot_idx = group.map(CommunityFishGroup::slot_idx);
    match (slot_idx, group_slot_idx) {
        (Some(slot_idx), Some(group_slot_idx)) if slot_idx != group_slot_idx => bail!(
            "--slot-idx {} conflicts with --group slot {}",
            slot_idx,
            group_slot_idx
        ),
        (Some(slot_idx), _) => Ok(Some(slot_idx)),
        (None, Some(group_slot_idx)) => Ok(Some(group_slot_idx)),
        (None, None) => Ok(default_slot_idx),
    }
}

fn resolve_dolt_repo_path(dolt_repo: Option<PathBuf>) -> Result<PathBuf> {
    match dolt_repo {
        Some(path) => find_dolt_repo_root(&path).ok_or_else(|| {
            anyhow::anyhow!(
                "could not find a Dolt repo at or above '{}'",
                path.display()
            )
        }),
        None => {
            let cwd = std::env::current_dir().context("read current working directory")?;
            find_dolt_repo_root(&cwd).ok_or_else(|| {
                anyhow::anyhow!(
                    "could not find a Dolt repo from current directory '{}'; pass --dolt-repo",
                    cwd.display()
                )
            })
        }
    }
}

fn find_dolt_repo_root(start: &Path) -> Option<PathBuf> {
    let start = if start.is_file() {
        start.parent()?
    } else {
        start
    };
    start
        .ancestors()
        .find(|path| path.join(".dolt").is_dir())
        .map(Path::to_path_buf)
}

fn resolve_zone_by_name(repo_path: &Path, zone_name: &str) -> Result<ResolvedZone> {
    let zone_name = zone_name.trim();
    if zone_name.is_empty() {
        bail!("zone name cannot be empty");
    }

    let fields = "name, CAST(R AS UNSIGNED) AS zone_r, CAST(G AS UNSIGNED) AS zone_g, CAST(B AS UNSIGNED) AS zone_b";
    let exact_rows = run_dolt_select_named_rows(
        repo_path,
        &format!(
            "SELECT {fields} FROM zones_merged WHERE LOWER(name) = LOWER({}) ORDER BY name",
            sql_value(zone_name)
        ),
        "resolve zone exact name",
    )?;
    if let Some(zone) = try_single_zone_match(zone_name, &exact_rows)? {
        return Ok(zone);
    }

    let like_rows = run_dolt_select_named_rows(
        repo_path,
        &format!(
            "SELECT {fields} FROM zones_merged WHERE LOWER(name) LIKE LOWER({}) ORDER BY name",
            sql_value(&format!("%{zone_name}%"))
        ),
        "resolve zone fuzzy name",
    )?;
    try_single_zone_match(zone_name, &like_rows)?.ok_or_else(|| {
        anyhow::anyhow!("zone '{}' did not match any row in zones_merged", zone_name)
    })
}

fn try_single_zone_match(
    original_zone_name: &str,
    rows: &[BTreeMap<String, String>],
) -> Result<Option<ResolvedZone>> {
    match rows {
        [] => Ok(None),
        [row] => Ok(Some(parse_zone_row(row)?)),
        _ => bail!(
            "zone '{}' is ambiguous; matches: {}",
            original_zone_name,
            rows.iter()
                .filter_map(|row| row.get("name").cloned())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn parse_zone_row(row: &BTreeMap<String, String>) -> Result<ResolvedZone> {
    let zone_name = csv_required(row, "name")?;
    let zone_r = csv_required(row, "zone_r")?
        .parse::<u8>()
        .with_context(|| format!("parse zone_r for {zone_name}"))?;
    let zone_g = csv_required(row, "zone_g")?
        .parse::<u8>()
        .with_context(|| format!("parse zone_g for {zone_name}"))?;
    let zone_b = csv_required(row, "zone_b")?
        .parse::<u8>()
        .with_context(|| format!("parse zone_b for {zone_name}"))?;
    Ok(ResolvedZone {
        zone_rgb: (u32::from(zone_r) << 16) | (u32::from(zone_g) << 8) | u32::from(zone_b),
        zone_r,
        zone_g,
        zone_b,
        region_name: derive_region_name_from_zone_name(&zone_name),
        zone_name,
    })
}

fn resolve_fish_reference(
    repo_path: &Path,
    fish_name: Option<&str>,
    item_id: Option<i64>,
) -> Result<ResolvedFish> {
    match (
        fish_name.map(str::trim).filter(|value| !value.is_empty()),
        item_id,
    ) {
        (Some(fish_name), Some(item_id)) => {
            let by_id = resolve_fish_by_item_id(repo_path, item_id)?;
            let by_name = resolve_fish_by_name(repo_path, fish_name)?;
            if by_id.item_id != by_name.item_id {
                bail!(
                    "fish reference mismatch: item_id {} resolves to '{}' but fish name '{}' resolves to item_id {}",
                    by_id.item_id,
                    by_id.fish_name,
                    fish_name,
                    by_name.item_id
                );
            }
            Ok(by_id)
        }
        (Some(fish_name), None) => resolve_fish_by_name(repo_path, fish_name),
        (None, Some(item_id)) => resolve_fish_by_item_id(repo_path, item_id),
        (None, None) => bail!("provide either --fish-name or --item-id"),
    }
}

fn resolve_fish_by_name(repo_path: &Path, fish_name: &str) -> Result<ResolvedFish> {
    let fish_name = fish_name.trim();
    let exact_rows = run_dolt_select_named_rows(
        repo_path,
        &format!(
            "SELECT CAST(item_key AS SIGNED) AS item_id, name AS fish_name \
             FROM fish_table WHERE LOWER(name) = LOWER({}) ORDER BY name",
            sql_value(fish_name)
        ),
        "resolve fish exact name",
    )?;
    if let Some(fish) = try_single_fish_match(fish_name, &exact_rows)? {
        return Ok(fish);
    }

    let like_rows = run_dolt_select_named_rows(
        repo_path,
        &format!(
            "SELECT CAST(item_key AS SIGNED) AS item_id, name AS fish_name \
             FROM fish_table WHERE LOWER(name) LIKE LOWER({}) ORDER BY name",
            sql_value(&format!("%{fish_name}%"))
        ),
        "resolve fish fuzzy name",
    )?;
    if let Some(fish) = try_single_fish_match(fish_name, &like_rows)? {
        return Ok(fish);
    }

    let item_rows = run_dolt_select_named_rows(
        repo_path,
        &format!(
            "SELECT CAST(`Index` AS SIGNED) AS item_id, `ItemName` AS fish_name \
             FROM item_table WHERE LOWER(`ItemName`) = LOWER({}) \
                OR LOWER(`ItemName`) LIKE LOWER({}) ORDER BY `ItemName`",
            sql_value(fish_name),
            sql_value(&format!("%{fish_name}%"))
        ),
        "resolve fish via item_table name",
    )?;
    try_single_fish_match(fish_name, &item_rows)?.ok_or_else(|| {
        anyhow::anyhow!(
            "fish '{}' did not match any row in fish_table or item_table",
            fish_name
        )
    })
}

fn try_single_fish_match(
    original_fish_name: &str,
    rows: &[BTreeMap<String, String>],
) -> Result<Option<ResolvedFish>> {
    match rows {
        [] => Ok(None),
        [row] => Ok(Some(parse_fish_row(row)?)),
        _ => bail!(
            "fish '{}' is ambiguous; matches: {}",
            original_fish_name,
            rows.iter()
                .filter_map(|row| row.get("fish_name").cloned())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn resolve_fish_by_item_id(repo_path: &Path, item_id: i64) -> Result<ResolvedFish> {
    if item_id <= 0 {
        bail!("item_id must be positive");
    }

    let fish_rows = run_dolt_select_named_rows(
        repo_path,
        &format!(
            "SELECT CAST(item_key AS SIGNED) AS item_id, name AS fish_name \
             FROM fish_table WHERE item_key = {item_id}"
        ),
        "resolve fish by item_id from fish_table",
    )?;
    if let Some(fish) = fish_rows.first() {
        return parse_fish_row(fish);
    }

    let item_rows = run_dolt_select_named_rows(
        repo_path,
        &format!(
            "SELECT CAST(`Index` AS SIGNED) AS item_id, `ItemName` AS fish_name \
             FROM item_table WHERE `Index` = {item_id}"
        ),
        "resolve fish by item_id from item_table",
    )?;
    if let Some(fish) = item_rows.first() {
        return parse_fish_row(fish);
    }

    bail!(
        "item_id {} did not match any row in fish_table or item_table",
        item_id
    )
}

fn parse_fish_row(row: &BTreeMap<String, String>) -> Result<ResolvedFish> {
    let item_id = csv_required(row, "item_id")?
        .parse::<i64>()
        .context("parse item_id")?;
    let fish_name = csv_required(row, "fish_name")?;
    Ok(ResolvedFish { item_id, fish_name })
}

fn resolve_zone_slot(repo_path: &Path, zone_rgb: u32, slot_idx: u8) -> Result<ResolvedZoneSlot> {
    validate_slot_idx(slot_idx)?;

    let slot_rows = run_dolt_select_named_rows(
        repo_path,
        &format!(
            "SELECT CAST(item_main_group_key AS SIGNED) AS item_main_group_key \
             FROM flockfish_zone_group_slots \
             WHERE zone_rgb = {zone_rgb} \
               AND slot_idx = {slot_idx} \
               AND resolution_status = 'numeric'"
        ),
        "resolve zone slot main group",
    )?;
    let slot_row = slot_rows
        .first()
        .ok_or_else(|| anyhow::anyhow!("zone_rgb {} has no numeric slot {}", zone_rgb, slot_idx))?;
    let item_main_group_key = csv_required(slot_row, "item_main_group_key")?
        .parse::<i64>()
        .context("parse item_main_group_key")?;
    if item_main_group_key <= 0 {
        bail!(
            "zone_rgb {} slot {} does not have a positive item_main_group_key",
            zone_rgb,
            slot_idx
        );
    }

    let main_group_rows = run_dolt_select_named_rows(
        repo_path,
        &format!(
            "SELECT \
                CAST(ItemSubGroupKey0 AS SIGNED) AS subgroup0, \
                CAST(ItemSubGroupKey1 AS SIGNED) AS subgroup1, \
                CAST(ItemSubGroupKey2 AS SIGNED) AS subgroup2, \
                CAST(ItemSubGroupKey3 AS SIGNED) AS subgroup3 \
             FROM item_main_group_table \
             WHERE ItemMainGroupKey = {item_main_group_key}"
        ),
        "resolve main group subgroup options",
    )?;
    let subgroup_keys = main_group_rows
        .first()
        .map(|row| {
            ["subgroup0", "subgroup1", "subgroup2", "subgroup3"]
                .into_iter()
                .filter_map(|key| row.get(key))
                .map(String::as_str)
                .filter_map(|value| value.parse::<i64>().ok())
                .filter(|value| *value > 0)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(ResolvedZoneSlot {
        slot_idx,
        item_main_group_key,
        subgroup_keys,
    })
}

fn format_manual_community_notes(
    slot_idx: Option<u8>,
    guessed_rate: Option<f64>,
    item_main_group_key: Option<i64>,
    subgroup_key: Option<i64>,
) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(slot_idx) = slot_idx {
        parts.push(format!("slot_idx={slot_idx}"));
    }
    if let Some(guessed_rate) = guessed_rate {
        parts.push(format!("guessed_rate={}", format_float(guessed_rate)));
    }
    if let Some(item_main_group_key) = item_main_group_key {
        parts.push(format!("item_main_group_key={item_main_group_key}"));
    }
    if let Some(subgroup_key) = subgroup_key {
        parts.push(format!("subgroup_key={subgroup_key}"));
    }
    (!parts.is_empty()).then_some(parts.join(";"))
}

fn build_community_zone_fish_support_upsert_query(
    source_id: &str,
    source_label: &str,
    zone: &ResolvedZone,
    fish: &ResolvedFish,
    support_status: &str,
    claim_count: u32,
    notes: Option<&str>,
) -> String {
    let values = [
        sql_value(source_id),
        sql_value(source_label),
        "NULL".to_string(),
        zone.zone_rgb.to_string(),
        zone.zone_r.to_string(),
        zone.zone_g.to_string(),
        zone.zone_b.to_string(),
        sql_value(&zone.region_name),
        sql_value(&zone.zone_name),
        fish.item_id.to_string(),
        sql_value(&fish.fish_name),
        sql_value(support_status),
        claim_count.to_string(),
        notes.map(sql_value).unwrap_or_else(|| "NULL".to_string()),
    ]
    .join(", ");

    format!(
        "INSERT INTO `community_zone_fish_support` \
            (`source_id`, `source_label`, `source_sha256`, `zone_rgb`, `zone_r`, `zone_g`, `zone_b`, `region_name`, `zone_name`, `item_id`, `fish_name`, `support_status`, `claim_count`, `notes`) \
         VALUES ({values}) \
         ON DUPLICATE KEY UPDATE \
            `source_label` = VALUES(`source_label`), \
            `source_sha256` = VALUES(`source_sha256`), \
            `zone_r` = VALUES(`zone_r`), \
            `zone_g` = VALUES(`zone_g`), \
            `zone_b` = VALUES(`zone_b`), \
            `region_name` = VALUES(`region_name`), \
            `zone_name` = VALUES(`zone_name`), \
            `fish_name` = VALUES(`fish_name`), \
            `support_status` = VALUES(`support_status`), \
            `claim_count` = VALUES(`claim_count`), \
            `notes` = VALUES(`notes`);"
    )
}

fn ensure_community_zone_fish_support_table(repo_path: &Path) -> Result<()> {
    run_dolt_sql_query_or_remote(
        repo_path,
        "CREATE TABLE IF NOT EXISTS `community_zone_fish_support` (\
            `source_id` varchar(64) NOT NULL,\
            `source_label` varchar(128) NOT NULL,\
            `source_sha256` char(64),\
            `zone_rgb` int unsigned NOT NULL,\
            `zone_r` tinyint unsigned NOT NULL,\
            `zone_g` tinyint unsigned NOT NULL,\
            `zone_b` tinyint unsigned NOT NULL,\
            `region_name` text,\
            `zone_name` text,\
            `item_id` bigint NOT NULL,\
            `fish_name` text,\
            `support_status` varchar(32) NOT NULL,\
            `claim_count` int NOT NULL DEFAULT '1',\
            `notes` text,\
            PRIMARY KEY (`source_id`,`zone_rgb`,`item_id`),\
            KEY `idx_community_zone_fish_support_item` (`item_id`),\
            KEY `idx_community_zone_fish_support_rgb` (`zone_rgb`),\
            KEY `idx_community_zone_fish_support_status` (`support_status`)\
         ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;",
        "ensure community_zone_fish_support table",
    )
}

fn ensure_languagedata_table(repo_path: &Path) -> Result<()> {
    let query = format!(
        "CREATE TABLE IF NOT EXISTS {} (\
            `lang` VARCHAR(16) NOT NULL,\
            `id` BIGINT NOT NULL,\
            `category` VARCHAR(64) NOT NULL,\
            `text` LONGTEXT,\
            `format` VARCHAR(8) NOT NULL,\
            PRIMARY KEY (`lang`, `format`, `category`, `id`),\
            KEY `idx_languagedata_lang_id_format_category` (`lang`, `id`, `format`, `category`)\
         ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;",
        sql_ident(LANGUAGEDATA_TABLE)
    );
    run_dolt_sql_query_or_remote(repo_path, &query, "ensure languagedata table")?;
    ensure_dolt_index(
        repo_path,
        LANGUAGEDATA_TABLE,
        "idx_languagedata_lang_id_format_category",
        "CREATE INDEX `idx_languagedata_lang_id_format_category` \
         ON `languagedata` (`lang`, `id`, `format`, `category`);",
    )
}

fn ensure_languagedata_import_table(repo_path: &Path) -> Result<()> {
    let query = format!(
        "CREATE TABLE IF NOT EXISTS {} (\
            `lang` VARCHAR(16) NOT NULL,\
            `id` BIGINT NOT NULL,\
            `category` VARCHAR(64) NOT NULL,\
            `text` LONGTEXT,\
            `format` VARCHAR(8) NOT NULL\
         ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;",
        sql_ident(LANGUAGEDATA_IMPORT_TABLE)
    );
    run_dolt_sql_query_or_remote(repo_path, &query, "ensure languagedata import table")
}

fn replace_languagedata_languages(
    repo_path: &Path,
    outputs: &BTreeMap<String, PathBuf>,
) -> Result<()> {
    ensure_languagedata_table(repo_path)?;
    for (lang, output_path) in outputs {
        ensure_languagedata_import_table(repo_path)?;
        run_dolt_table_replace(repo_path, LANGUAGEDATA_IMPORT_TABLE, output_path)?;
        let query = format!(
            "DELETE FROM {table} WHERE `lang` = {lang};\
             INSERT INTO {table} (`lang`, `id`, `category`, `text`, `format`) \
             SELECT `lang`, `id`, COALESCE(`category`, ''), `text`, `format` \
             FROM {staging} \
             WHERE `lang` = {lang};\
             DROP TABLE {staging};",
            table = sql_ident(LANGUAGEDATA_TABLE),
            staging = sql_ident(LANGUAGEDATA_IMPORT_TABLE),
            lang = sql_value(lang),
        );
        run_dolt_sql_query_or_remote(
            repo_path,
            &query,
            &format!("replace languagedata rows for {lang}"),
        )?;
    }
    Ok(())
}

fn ensure_calculator_lookup_indexes(repo_path: &Path) -> Result<()> {
    ensure_languagedata_table(repo_path)?;
    ensure_dolt_index(
        repo_path,
        "item_table",
        "idx_item_table_item_name",
        "CREATE INDEX `idx_item_table_item_name` \
         ON `item_table` (`ItemName`(191));",
    )?;
    Ok(())
}

fn dolt_table_exists(repo_path: &Path, table_name: &str) -> Result<bool> {
    let query = format!(
        "SELECT 1 AS present \
         FROM information_schema.tables \
         WHERE table_schema = DATABASE() \
           AND table_name = {} \
         LIMIT 1",
        sql_value(table_name)
    );
    Ok(
        !run_dolt_select_named_rows(repo_path, &query, &format!("check {table_name} table"))?
            .is_empty(),
    )
}

fn dolt_index_exists(repo_path: &Path, table_name: &str, index_name: &str) -> Result<bool> {
    let query = format!(
        "SELECT 1 AS present \
         FROM information_schema.statistics \
         WHERE table_schema = DATABASE() \
           AND table_name = {} \
           AND index_name = {} \
         LIMIT 1",
        sql_value(table_name),
        sql_value(index_name)
    );
    Ok(!run_dolt_select_named_rows(
        repo_path,
        &query,
        &format!("check {table_name}.{index_name} index"),
    )?
    .is_empty())
}

fn ensure_dolt_index(
    repo_path: &Path,
    table_name: &str,
    index_name: &str,
    create_sql: &str,
) -> Result<()> {
    if !dolt_table_exists(repo_path, table_name)? {
        return Ok(());
    }
    if dolt_index_exists(repo_path, table_name, index_name)? {
        return Ok(());
    }
    run_dolt_sql_query_or_remote(
        repo_path,
        create_sql,
        &format!("create {table_name}.{index_name} index"),
    )
}

fn csv_required(row: &BTreeMap<String, String>, key: &str) -> Result<String> {
    let value = row
        .get(key)
        .map(String::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    if value.is_empty() {
        bail!("missing expected CSV field '{}'", key);
    }
    Ok(value)
}

fn run_calculator_effects_import(command: CalculatorEffectsImportCommand) -> Result<()> {
    let CalculatorEffectsImportCommand {
        dolt_repo,
        excel_dir,
        output_dir,
        commit,
        commit_msg,
    } = command;

    let workbook_set = resolve_calculator_effect_workbooks(&excel_dir)?;
    let output_dir = match output_dir {
        Some(path) => path,
        None => default_output_dir()?,
    };
    fs::create_dir_all(&output_dir)
        .with_context(|| format!("create output dir: {}", output_dir.display()))?;

    let digests = CalculatorEffectsDigests {
        buff_table_sha: sha256_file(&workbook_set.buff_table_xlsx)?,
        common_stat_data_sha: sha256_file(&workbook_set.common_stat_data_xlsx)?,
        fishing_stat_data_sha: sha256_file(&workbook_set.fishing_stat_data_xlsx)?,
        skill_table_new_sha: sha256_file(&workbook_set.skill_table_new_xlsx)?,
        skilltype_table_new_sha: sha256_file(&workbook_set.skilltype_table_new_xlsx)?,
        lightstone_set_option_sha: sha256_file(&workbook_set.lightstone_set_option_xlsx)?,
        translate_stat_sha: sha256_file(&workbook_set.translate_stat_xlsx)?,
        enchant_cash_sha: sha256_file(&workbook_set.enchant_cash_xlsx)?,
        enchant_equipment_sha: sha256_file(&workbook_set.enchant_equipment_xlsx)?,
        enchant_lifeequipment_sha: sha256_file(&workbook_set.enchant_lifeequipment_xlsx)?,
        tooltip_table_sha: sha256_file(&workbook_set.tooltip_table_xlsx)?,
        producttool_property_sha: sha256_file(&workbook_set.producttool_property_xlsx)?,
        pet_table_sha: sha256_file(&workbook_set.pet_table_xlsx)?,
        pet_skill_table_sha: sha256_file(&workbook_set.pet_skill_table_xlsx)?,
        pet_base_skill_table_sha: sha256_file(&workbook_set.pet_base_skill_table_xlsx)?,
        pet_setstats_table_sha: sha256_file(&workbook_set.pet_setstats_table_xlsx)?,
        pet_equipskill_table_sha: sha256_file(&workbook_set.pet_equipskill_table_xlsx)?,
        pet_equipskill_aquire_table_sha: sha256_file(
            &workbook_set.pet_equipskill_aquire_table_xlsx,
        )?,
        pet_grade_table_sha: sha256_file(&workbook_set.pet_grade_table_xlsx)?,
        pet_exp_table_sha: sha256_file(&workbook_set.pet_exp_table_xlsx)?,
        upgradepet_looting_percent_sha: sha256_file(&workbook_set.upgradepet_looting_percent_xlsx)?,
    };

    let outputs = CalculatorEffectsOutputs {
        buff_table_csv: output_dir.join("buff_table.csv"),
        common_stat_data_csv: output_dir.join("common_stat_data.csv"),
        fishing_stat_data_csv: output_dir.join("fishing_stat_data.csv"),
        skill_table_new_csv: output_dir.join("skill_table_new.csv"),
        skilltype_table_new_csv: output_dir.join("skilltype_table_new.csv"),
        lightstone_set_option_csv: output_dir.join("lightstone_set_option.csv"),
        translate_stat_csv: output_dir.join("translate_stat.csv"),
        enchant_cash_csv: output_dir.join("enchant_cash.csv"),
        enchant_equipment_csv: output_dir.join("enchant_equipment.csv"),
        enchant_lifeequipment_csv: output_dir.join("enchant_lifeequipment.csv"),
        tooltip_table_csv: output_dir.join("tooltip_table.csv"),
        producttool_property_csv: output_dir.join("producttool_property.csv"),
        pet_table_csv: output_dir.join("pet_table.csv"),
        pet_skill_table_csv: output_dir.join("pet_skill_table.csv"),
        pet_base_skill_table_csv: output_dir.join("pet_base_skill_table.csv"),
        pet_setstats_table_csv: output_dir.join("pet_setstats_table.csv"),
        pet_equipskill_table_csv: output_dir.join("pet_equipskill_table.csv"),
        pet_equipskill_aquire_table_csv: output_dir.join("pet_equipskill_aquire_table.csv"),
        pet_grade_table_csv: output_dir.join("pet_grade_table.csv"),
        pet_exp_table_csv: output_dir.join("pet_exp_table.csv"),
        upgradepet_looting_percent_csv: output_dir.join("upgradepet_looting_percent.csv"),
        consumable_source_item_effect_evidence_csv: output_dir
            .join("calculator_consumable_source_item_effect_evidence.csv"),
    };

    let buff_table_stats = import_workbook_sheet(
        &workbook_set.buff_table_xlsx,
        "Buff_Table",
        &BUFF_TABLE_HEADERS,
        &outputs.buff_table_csv,
    )?;
    let common_stat_data_stats = import_workbook_sheet(
        &workbook_set.common_stat_data_xlsx,
        "CommonStatData",
        &COMMON_STAT_DATA_HEADERS,
        &outputs.common_stat_data_csv,
    )?;
    let fishing_stat_data_stats = import_workbook_sheet(
        &workbook_set.fishing_stat_data_xlsx,
        "FishingStatData",
        &FISHING_STAT_DATA_HEADERS,
        &outputs.fishing_stat_data_csv,
    )?;
    let skill_table_new_stats = import_workbook_sheet(
        &workbook_set.skill_table_new_xlsx,
        "Skill_Table_New",
        &SKILL_TABLE_NEW_HEADERS,
        &outputs.skill_table_new_csv,
    )?;
    let skilltype_table_new_stats = import_workbook_sheet(
        &workbook_set.skilltype_table_new_xlsx,
        "SkillType_Table_New",
        &SKILLTYPE_TABLE_NEW_HEADERS,
        &outputs.skilltype_table_new_csv,
    )?;
    let lightstone_set_option_stats = import_workbook_sheet(
        &workbook_set.lightstone_set_option_xlsx,
        "LightStoneSetOption",
        &LIGHTSTONE_SET_OPTION_HEADERS,
        &outputs.lightstone_set_option_csv,
    )?;
    let translate_stat_stats = import_workbook_sheet(
        &workbook_set.translate_stat_xlsx,
        "TranslateStat",
        &TRANSLATE_STAT_HEADERS,
        &outputs.translate_stat_csv,
    )?;
    let enchant_cash_stats = import_workbook_sheet(
        &workbook_set.enchant_cash_xlsx,
        "Enchant_Cash",
        &ENCHANT_WORKBOOK_HEADERS,
        &outputs.enchant_cash_csv,
    )?;
    let enchant_equipment_stats = import_workbook_sheet(
        &workbook_set.enchant_equipment_xlsx,
        "Enchant_Equipment",
        &ENCHANT_WORKBOOK_HEADERS,
        &outputs.enchant_equipment_csv,
    )?;
    let enchant_lifeequipment_stats = import_workbook_sheet(
        &workbook_set.enchant_lifeequipment_xlsx,
        "Enchant_LifeEquipment",
        &ENCHANT_WORKBOOK_HEADERS,
        &outputs.enchant_lifeequipment_csv,
    )?;
    let tooltip_table_stats = import_workbook_sheet(
        &workbook_set.tooltip_table_xlsx,
        "Tooltip_Table",
        &TOOLTIP_TABLE_HEADERS,
        &outputs.tooltip_table_csv,
    )?;
    let producttool_property_stats = import_workbook_sheet(
        &workbook_set.producttool_property_xlsx,
        "ProductTool_Property",
        &PRODUCTTOOL_PROPERTY_HEADERS,
        &outputs.producttool_property_csv,
    )?;
    let pet_table_stats = import_workbook_sheet(
        &workbook_set.pet_table_xlsx,
        "Pet_Table",
        &PET_TABLE_HEADERS,
        &outputs.pet_table_csv,
    )?;
    let pet_skill_table_stats = import_workbook_sheet(
        &workbook_set.pet_skill_table_xlsx,
        "Pet_Skill_Table",
        &PET_SKILL_TABLE_HEADERS,
        &outputs.pet_skill_table_csv,
    )?;
    let pet_base_skill_table_stats = import_workbook_sheet(
        &workbook_set.pet_base_skill_table_xlsx,
        "Pet_BaseSkill_Table",
        &PET_BASE_SKILL_TABLE_HEADERS,
        &outputs.pet_base_skill_table_csv,
    )?;
    let pet_setstats_table_stats = import_workbook_sheet(
        &workbook_set.pet_setstats_table_xlsx,
        "Pet_SetStats_Table",
        &PET_SETSTATS_TABLE_HEADERS,
        &outputs.pet_setstats_table_csv,
    )?;
    let pet_equipskill_table_stats = import_workbook_sheet(
        &workbook_set.pet_equipskill_table_xlsx,
        "Pet_EquipSkill_Table",
        &PET_EQUIPSKILL_TABLE_HEADERS,
        &outputs.pet_equipskill_table_csv,
    )?;
    let pet_equipskill_aquire_table_stats = import_workbook_sheet(
        &workbook_set.pet_equipskill_aquire_table_xlsx,
        "Pet_EquipSkill_Aquire_Table",
        &PET_EQUIPSKILL_AQUIRE_TABLE_HEADERS,
        &outputs.pet_equipskill_aquire_table_csv,
    )?;
    let pet_grade_table_stats = import_workbook_sheet(
        &workbook_set.pet_grade_table_xlsx,
        "Pet_Grade_Table",
        &PET_GRADE_TABLE_HEADERS,
        &outputs.pet_grade_table_csv,
    )?;
    let pet_exp_table_stats = import_workbook_sheet(
        &workbook_set.pet_exp_table_xlsx,
        "Pet_Exp_Table",
        &PET_EXP_TABLE_HEADERS,
        &outputs.pet_exp_table_csv,
    )?;
    let upgradepet_looting_percent_stats = import_workbook_sheet(
        &workbook_set.upgradepet_looting_percent_xlsx,
        "UpgradePet_Looting_Percent",
        &UPGRADEPET_LOOTING_PERCENT_HEADERS,
        &outputs.upgradepet_looting_percent_csv,
    )?;

    run_dolt_sql_table_import(&dolt_repo, "buff_table", &outputs.buff_table_csv)?;
    run_dolt_sql_table_import(
        &dolt_repo,
        "common_stat_data",
        &outputs.common_stat_data_csv,
    )?;
    run_dolt_sql_table_import(
        &dolt_repo,
        "fishing_stat_data",
        &outputs.fishing_stat_data_csv,
    )?;
    run_dolt_sql_table_import(&dolt_repo, "skill_table_new", &outputs.skill_table_new_csv)?;
    run_dolt_sql_table_import(
        &dolt_repo,
        "skilltype_table_new",
        &outputs.skilltype_table_new_csv,
    )?;
    run_dolt_sql_table_import(
        &dolt_repo,
        "lightstone_set_option",
        &outputs.lightstone_set_option_csv,
    )?;
    run_dolt_sql_table_import(&dolt_repo, "translate_stat", &outputs.translate_stat_csv)?;
    run_dolt_sql_table_import(&dolt_repo, "enchant_cash", &outputs.enchant_cash_csv)?;
    run_dolt_sql_table_import(
        &dolt_repo,
        "enchant_equipment",
        &outputs.enchant_equipment_csv,
    )?;
    run_dolt_sql_table_import(
        &dolt_repo,
        "enchant_lifeequipment",
        &outputs.enchant_lifeequipment_csv,
    )?;
    run_dolt_sql_table_import(&dolt_repo, "tooltip_table", &outputs.tooltip_table_csv)?;
    run_dolt_sql_table_import(
        &dolt_repo,
        "producttool_property",
        &outputs.producttool_property_csv,
    )?;
    run_dolt_sql_table_import(&dolt_repo, "pet_table", &outputs.pet_table_csv)?;
    run_dolt_sql_table_import(&dolt_repo, "pet_skill_table", &outputs.pet_skill_table_csv)?;
    run_dolt_sql_table_import(
        &dolt_repo,
        "pet_base_skill_table",
        &outputs.pet_base_skill_table_csv,
    )?;
    run_dolt_sql_table_import(
        &dolt_repo,
        "pet_setstats_table",
        &outputs.pet_setstats_table_csv,
    )?;
    run_dolt_sql_table_import(
        &dolt_repo,
        "pet_equipskill_table",
        &outputs.pet_equipskill_table_csv,
    )?;
    run_dolt_sql_table_import(
        &dolt_repo,
        "pet_equipskill_aquire_table",
        &outputs.pet_equipskill_aquire_table_csv,
    )?;
    run_dolt_sql_table_import(&dolt_repo, "pet_grade_table", &outputs.pet_grade_table_csv)?;
    run_dolt_sql_table_import(&dolt_repo, "pet_exp_table", &outputs.pet_exp_table_csv)?;
    run_dolt_sql_table_import(
        &dolt_repo,
        "upgradepet_looting_percent",
        &outputs.upgradepet_looting_percent_csv,
    )?;
    let consumable_source_items_stats = refresh_calculator_consumable_source_item_effect_evidence(
        &dolt_repo,
        &outputs.consumable_source_item_effect_evidence_csv,
    )?;

    if commit {
        let msg = build_calculator_effects_commit_message(commit_msg, &digests);
        run_dolt_commit(&dolt_repo, &msg)?;
    }

    println!("buff_table rows imported: {}", buff_table_stats.row_count);
    println!(
        "common_stat_data rows imported: {}",
        common_stat_data_stats.row_count
    );
    println!(
        "fishing_stat_data rows imported: {}",
        fishing_stat_data_stats.row_count
    );
    println!(
        "skill_table_new rows imported: {}",
        skill_table_new_stats.row_count
    );
    println!(
        "skilltype_table_new rows imported: {}",
        skilltype_table_new_stats.row_count
    );
    println!(
        "lightstone_set_option rows imported: {}",
        lightstone_set_option_stats.row_count
    );
    println!(
        "translate_stat rows imported: {}",
        translate_stat_stats.row_count
    );
    println!(
        "enchant_cash rows imported: {}",
        enchant_cash_stats.row_count
    );
    println!(
        "enchant_equipment rows imported: {}",
        enchant_equipment_stats.row_count
    );
    println!(
        "enchant_lifeequipment rows imported: {}",
        enchant_lifeequipment_stats.row_count
    );
    println!(
        "tooltip_table rows imported: {}",
        tooltip_table_stats.row_count
    );
    println!(
        "producttool_property rows imported: {}",
        producttool_property_stats.row_count
    );
    println!("pet_table rows imported: {}", pet_table_stats.row_count);
    println!(
        "pet_skill_table rows imported: {}",
        pet_skill_table_stats.row_count
    );
    println!(
        "pet_base_skill_table rows imported: {}",
        pet_base_skill_table_stats.row_count
    );
    println!(
        "pet_setstats_table rows imported: {}",
        pet_setstats_table_stats.row_count
    );
    println!(
        "pet_equipskill_table rows imported: {}",
        pet_equipskill_table_stats.row_count
    );
    println!(
        "pet_equipskill_aquire_table rows imported: {}",
        pet_equipskill_aquire_table_stats.row_count
    );
    println!(
        "pet_grade_table rows imported: {}",
        pet_grade_table_stats.row_count
    );
    println!(
        "pet_exp_table rows imported: {}",
        pet_exp_table_stats.row_count
    );
    println!(
        "upgradepet_looting_percent rows imported: {}",
        upgradepet_looting_percent_stats.row_count
    );
    println!(
        "calculator consumable source item evidence rows emitted: {}",
        consumable_source_items_stats.row_count
    );

    Ok(())
}

fn run_refresh_calculator_consumable_source_items(
    command: RefreshCalculatorConsumableSourceItemsCommand,
) -> Result<()> {
    let RefreshCalculatorConsumableSourceItemsCommand {
        dolt_repo,
        output_dir,
        commit,
        commit_msg,
    } = command;

    let output_dir = match output_dir {
        Some(path) => path,
        None => default_output_dir()?,
    };
    fs::create_dir_all(&output_dir)
        .with_context(|| format!("create output dir: {}", output_dir.display()))?;
    let output_csv = output_dir.join("calculator_consumable_source_item_effect_evidence.csv");
    let stats = refresh_calculator_consumable_source_item_effect_evidence(&dolt_repo, &output_csv)?;

    if commit {
        let msg = commit_msg.unwrap_or_else(|| {
            "Refresh calculator consumable source item effect evidence".to_string()
        });
        run_dolt_commit(&dolt_repo, &msg)?;
    }

    println!(
        "calculator consumable source item evidence rows emitted: {}",
        stats.row_count
    );
    Ok(())
}

fn refresh_calculator_consumable_source_item_effect_evidence(
    repo_path: &Path,
    output_csv: &Path,
) -> Result<CalculatorConsumableSourceItemEffectEvidenceImport> {
    let rows = build_calculator_consumable_source_item_effect_evidence_rows(repo_path)?;
    let mut writer = build_csv_writer(output_csv)?;
    writer.write_record(CALCULATOR_CONSUMABLE_SOURCE_ITEM_EFFECT_EVIDENCE_HEADERS)?;
    for row in &rows {
        writer.write_record(row)?;
    }
    writer.flush()?;

    run_dolt_sql_table_import_or_remote(
        repo_path,
        CALCULATOR_CONSUMABLE_SOURCE_ITEM_EFFECT_EVIDENCE_TABLE,
        output_csv,
    )?;

    Ok(CalculatorConsumableSourceItemEffectEvidenceImport {
        row_count: rows.len(),
    })
}

fn build_calculator_consumable_source_item_effect_evidence_rows(
    repo_path: &Path,
) -> Result<Vec<Vec<String>>> {
    let skill_descriptions = load_calculator_consumable_skill_descriptions(repo_path)?;
    let buff_texts = load_calculator_consumable_buff_texts(repo_path)?;
    if skill_descriptions.is_empty() && buff_texts.is_empty() {
        return Ok(Vec::new());
    }

    let relevant_skill_rows = load_calculator_consumable_relevant_skill_rows(
        repo_path,
        &skill_descriptions,
        &buff_texts,
    )?;
    if relevant_skill_rows.is_empty() {
        return Ok(Vec::new());
    }
    let mut relevant_skill_ids = relevant_skill_rows
        .iter()
        .map(|row| row.skill_no.clone())
        .collect::<Vec<_>>();
    relevant_skill_ids.sort_unstable();
    relevant_skill_ids.dedup();

    let item_rows = load_calculator_consumable_item_source_rows(repo_path, &relevant_skill_ids)?;
    if item_rows.is_empty() {
        return Ok(Vec::new());
    }

    let primary_skill_counts = item_rows
        .iter()
        .filter_map(|row| row.skill_no.clone())
        .fold(HashMap::<String, usize>::new(), |mut counts, skill_id| {
            *counts.entry(skill_id).or_default() += 1;
            counts
        });
    let skill_ids = item_rows
        .iter()
        .flat_map(|row| [row.skill_no.clone(), row.sub_skill_no.clone()])
        .flatten()
        .collect::<Vec<_>>();
    let buff_categories_by_skill =
        load_calculator_consumable_skill_buff_categories(repo_path, &skill_ids)?;
    let skill_buffs = relevant_skill_rows
        .into_iter()
        .map(|row| (row.skill_no, row.buff_ids))
        .collect::<HashMap<_, _>>();

    let mut out = Vec::<Vec<String>>::new();
    let mut item_rows = item_rows;
    item_rows.sort_by_key(|row| row.item_id);
    for row in item_rows {
        let Ok(item_id) = i32::try_from(row.item_id) else {
            continue;
        };
        let mut effect_lines = Vec::<String>::new();
        for candidate_skill in [row.skill_no.as_deref(), row.sub_skill_no.as_deref()]
            .into_iter()
            .flatten()
        {
            let selected_texts = skill_buffs
                .get(candidate_skill)
                .map(|buff_ids| {
                    select_consumable_effect_texts(
                        candidate_skill,
                        buff_ids,
                        &buff_texts,
                        &skill_descriptions,
                    )
                })
                .filter(|texts| !texts.is_empty())
                .unwrap_or_else(|| {
                    skill_descriptions
                        .get(candidate_skill)
                        .cloned()
                        .into_iter()
                        .collect()
                });
            for text in selected_texts {
                for line in normalized_effect_lines(&text) {
                    if !effect_lines.iter().any(|existing| existing == &line) {
                        effect_lines.push(line);
                    }
                }
            }
        }
        if effect_lines.is_empty() {
            continue;
        }

        let category_metadata = select_consumable_category_metadata(
            row.skill_no.as_deref(),
            row.sub_skill_no.as_deref(),
            &buff_categories_by_skill,
        );
        let buff_category_key = buff_category_key(category_metadata.category_id).or_else(|| {
            fallback_consumable_family_key(row.skill_no.as_deref(), &primary_skill_counts)
        });
        let item_type = match (category_metadata.category_id, row.item_classify.as_deref()) {
            (Some(1), _) | (None, Some("8")) => "food",
            _ => "buff",
        };
        let source_text_ko = effect_lines.join("\n");
        let mut source_text_values = CalculatorEffectValues::default();
        parse_unique_calculator_effect_text(&mut source_text_values, &source_text_ko);
        out.push(vec![
            format!("item:{item_id}"),
            item_id.to_string(),
            item_type.to_string(),
            buff_category_key.unwrap_or_default(),
            optional_i32_to_string(category_metadata.category_id),
            optional_i32_to_string(category_metadata.category_level),
            source_text_ko,
            optional_f32_to_string(source_text_values.afr),
            optional_f32_to_string(source_text_values.bonus_rare),
            optional_f32_to_string(source_text_values.bonus_big),
            optional_f32_to_string(source_text_values.item_drr),
            optional_f32_to_string(source_text_values.exp_fish),
            optional_f32_to_string(source_text_values.exp_life),
        ]);
    }

    Ok(out)
}

fn load_calculator_consumable_skill_descriptions(
    repo_path: &Path,
) -> Result<HashMap<String, String>> {
    let query = format!(
        "SELECT `SkillNo` AS skill_no, `Desc` AS description \
         FROM skilltype_table_new \
         WHERE ({})",
        calculator_effect_keyword_predicate("`Desc`")
    );
    let rows = run_dolt_select_named_rows(repo_path, &query, "calculator skill descriptions")?;
    Ok(rows
        .into_iter()
        .filter_map(|row| {
            Some((
                normalized_db_field(&row, "skill_no")?,
                normalized_db_field(&row, "description")?,
            ))
        })
        .collect())
}

fn load_calculator_consumable_buff_texts(
    repo_path: &Path,
) -> Result<HashMap<String, CalculatorConsumableBuffText>> {
    let query = format!(
        "SELECT `Index` AS buff_id, `BuffName` AS buff_name, `Description` AS description \
         FROM buff_table \
         WHERE ({}) OR ({})",
        calculator_effect_keyword_predicate("`Description`"),
        calculator_effect_keyword_predicate("`BuffName`")
    );
    let rows = run_dolt_select_named_rows(repo_path, &query, "calculator buff texts")?;
    Ok(rows
        .into_iter()
        .filter_map(|row| {
            let buff_id = normalized_db_field(&row, "buff_id")?;
            let description = normalized_db_field(&row, "description");
            let buff_name = normalized_db_field(&row, "buff_name");
            Some((
                buff_id,
                CalculatorConsumableBuffText {
                    text: description.clone().or(buff_name)?,
                    has_description: description.is_some(),
                },
            ))
        })
        .collect())
}

fn load_calculator_consumable_relevant_skill_rows(
    repo_path: &Path,
    skill_descriptions: &HashMap<String, String>,
    buff_texts: &HashMap<String, CalculatorConsumableBuffText>,
) -> Result<Vec<CalculatorConsumableSkillRow>> {
    let skill_ids = skill_descriptions.keys().cloned().collect::<Vec<_>>();
    let buff_ids = buff_texts.keys().cloned().collect::<Vec<_>>();
    let skill_filter = if skill_ids.is_empty() {
        String::from("FALSE")
    } else {
        format!("`SkillNo` IN ({})", quote_sql_string_list(&skill_ids))
    };
    let buff_filter = if buff_ids.is_empty() {
        String::from("FALSE")
    } else {
        format!(
            "`Buff0` IN ({ids}) \
             OR `Buff1` IN ({ids}) \
             OR `Buff2` IN ({ids}) \
             OR `Buff3` IN ({ids}) \
             OR `Buff4` IN ({ids}) \
             OR `Buff5` IN ({ids}) \
             OR `Buff6` IN ({ids}) \
             OR `Buff7` IN ({ids}) \
             OR `Buff8` IN ({ids}) \
             OR `Buff9` IN ({ids})",
            ids = quote_sql_string_list(&buff_ids)
        )
    };
    let query = format!(
        "SELECT `SkillNo` AS skill_no, \
                `Buff0` AS buff0, \
                `Buff1` AS buff1, \
                `Buff2` AS buff2, \
                `Buff3` AS buff3, \
                `Buff4` AS buff4, \
                `Buff5` AS buff5, \
                `Buff6` AS buff6, \
                `Buff7` AS buff7, \
                `Buff8` AS buff8, \
                `Buff9` AS buff9 \
         FROM skill_table_new \
         WHERE ({skill_filter}) OR ({buff_filter})"
    );
    load_calculator_consumable_skill_rows_from_query(
        repo_path,
        &query,
        "calculator relevant skills",
    )
}

fn load_calculator_consumable_item_source_rows(
    repo_path: &Path,
    skill_ids: &[String],
) -> Result<Vec<CalculatorConsumableItemSourceRow>> {
    if skill_ids.is_empty() {
        return Ok(Vec::new());
    }
    let query = format!(
        "SELECT CAST(`Index` AS SIGNED) AS item_id, \
                `ItemClassify` AS item_classify, \
                `SkillNo` AS skill_no, \
                `SubSkillNo` AS sub_skill_no \
         FROM item_table \
         WHERE `SkillNo` IN ({skill_ids}) \
            OR `SubSkillNo` IN ({skill_ids})",
        skill_ids = quote_sql_string_list(skill_ids)
    );
    let rows = run_dolt_select_named_rows(repo_path, &query, "calculator consumable item sources")?;
    Ok(rows
        .into_iter()
        .filter_map(|row| {
            let item_id = normalized_db_field(&row, "item_id")?.parse::<i64>().ok()?;
            Some(CalculatorConsumableItemSourceRow {
                item_id,
                item_classify: normalized_db_field(&row, "item_classify"),
                skill_no: normalized_db_field(&row, "skill_no"),
                sub_skill_no: normalized_db_field(&row, "sub_skill_no"),
            })
        })
        .collect())
}

fn load_calculator_consumable_skill_buff_categories(
    repo_path: &Path,
    skill_ids: &[String],
) -> Result<HashMap<String, CalculatorConsumableBuffCategory>> {
    let mut skill_ids = skill_ids
        .iter()
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    skill_ids.sort_unstable();
    skill_ids.dedup();
    if skill_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let query = format!(
        "SELECT `SkillNo` AS skill_no, \
                `Buff0` AS buff0, \
                `Buff1` AS buff1, \
                `Buff2` AS buff2, \
                `Buff3` AS buff3, \
                `Buff4` AS buff4, \
                `Buff5` AS buff5, \
                `Buff6` AS buff6, \
                `Buff7` AS buff7, \
                `Buff8` AS buff8, \
                `Buff9` AS buff9 \
         FROM skill_table_new \
         WHERE `SkillNo` IN ({})",
        quote_sql_string_list(&skill_ids)
    );
    let skill_rows = load_calculator_consumable_skill_rows_from_query(
        repo_path,
        &query,
        "calculator category skills",
    )?;

    let mut buff_ids = skill_rows
        .iter()
        .flat_map(|row| row.buff_ids.iter().cloned())
        .collect::<Vec<_>>();
    buff_ids.sort_unstable();
    buff_ids.dedup();
    if buff_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let buff_query = format!(
        "SELECT `Index` AS buff_id, `Category` AS category, `CategoryLevel` AS category_level \
         FROM buff_table \
         WHERE `Index` IN ({})",
        quote_sql_string_list(&buff_ids)
    );
    let buff_rows =
        run_dolt_select_named_rows(repo_path, &buff_query, "calculator category buffs")?;
    let buff_metadata = buff_rows
        .into_iter()
        .filter_map(|row| {
            Some((
                normalized_db_field(&row, "buff_id")?,
                CalculatorConsumableBuffCategory {
                    category_id: parse_optional_i32(normalized_db_field(&row, "category")),
                    category_level: parse_optional_i32(normalized_db_field(&row, "category_level")),
                },
            ))
        })
        .collect::<HashMap<_, _>>();

    let mut out = HashMap::new();
    for skill_row in skill_rows {
        let categories = skill_row
            .buff_ids
            .iter()
            .filter_map(|buff_id| buff_metadata.get(buff_id))
            .filter_map(|metadata| {
                metadata
                    .category_id
                    .filter(|category_id| *category_id > 0)
                    .map(|category_id| (category_id, metadata.category_level))
            })
            .collect::<Vec<_>>();
        let Some((category_id, occurrences)) = categories
            .iter()
            .fold(
                HashMap::<i32, usize>::new(),
                |mut counts, (category_id, _)| {
                    *counts.entry(*category_id).or_default() += 1;
                    counts
                },
            )
            .into_iter()
            .max_by_key(|(category_id, count)| (*count, -(*category_id)))
        else {
            continue;
        };
        let _ = occurrences;
        let category_level = categories
            .iter()
            .filter(|(candidate_id, _)| *candidate_id == category_id)
            .filter_map(|(_, category_level)| *category_level)
            .max();
        out.insert(
            skill_row.skill_no,
            CalculatorConsumableBuffCategory {
                category_id: Some(category_id),
                category_level,
            },
        );
    }
    Ok(out)
}

fn load_calculator_consumable_skill_rows_from_query(
    repo_path: &Path,
    query: &str,
    label: &str,
) -> Result<Vec<CalculatorConsumableSkillRow>> {
    let rows = run_dolt_select_named_rows(repo_path, query, label)?;
    Ok(rows
        .into_iter()
        .filter_map(|row| {
            let skill_no = normalized_db_field(&row, "skill_no")?;
            let mut buff_ids = Vec::new();
            for key in [
                "buff0", "buff1", "buff2", "buff3", "buff4", "buff5", "buff6", "buff7", "buff8",
                "buff9",
            ] {
                let Some(buff_id) = normalized_db_field(&row, key) else {
                    continue;
                };
                if !buff_ids.iter().any(|existing| existing == &buff_id) {
                    buff_ids.push(buff_id);
                }
            }
            Some(CalculatorConsumableSkillRow { skill_no, buff_ids })
        })
        .collect())
}

fn select_consumable_effect_texts(
    skill_id: &str,
    buff_ids: &[String],
    buff_text_rows: &HashMap<String, CalculatorConsumableBuffText>,
    skill_descriptions: &HashMap<String, String>,
) -> Vec<String> {
    let buff_rows = buff_ids
        .iter()
        .filter_map(|buff_id| buff_text_rows.get(buff_id))
        .collect::<Vec<_>>();
    let composite_rows = buff_rows
        .iter()
        .filter(|row| row.has_description && normalized_effect_lines(&row.text).len() > 1)
        .map(|row| row.text.clone())
        .collect::<Vec<_>>();
    if !composite_rows.is_empty() {
        return composite_rows;
    }
    let leaf_rows = buff_rows
        .iter()
        .map(|row| row.text.clone())
        .collect::<Vec<_>>();
    if !leaf_rows.is_empty() {
        return leaf_rows;
    }
    skill_descriptions
        .get(skill_id)
        .cloned()
        .into_iter()
        .collect()
}

fn select_consumable_category_metadata(
    primary_skill_id: Option<&str>,
    fallback_skill_id: Option<&str>,
    buff_categories_by_skill: &HashMap<String, CalculatorConsumableBuffCategory>,
) -> CalculatorConsumableBuffCategory {
    if let Some(primary) = primary_skill_id
        .and_then(|skill_id| buff_categories_by_skill.get(skill_id))
        .filter(|metadata| metadata.category_id.is_some())
    {
        return primary.clone();
    }
    if let Some(fallback) = fallback_skill_id
        .and_then(|skill_id| buff_categories_by_skill.get(skill_id))
        .filter(|metadata| metadata.category_id.is_some())
    {
        return fallback.clone();
    }
    let categories = [primary_skill_id, fallback_skill_id]
        .into_iter()
        .flatten()
        .filter_map(|skill_id| buff_categories_by_skill.get(skill_id))
        .filter_map(|metadata| {
            metadata
                .category_id
                .map(|category_id| (category_id, metadata.category_level))
        })
        .collect::<Vec<_>>();
    let Some((category_id, category_level)) = categories
        .iter()
        .max_by_key(|(category_id, category_level)| (*category_level, -*category_id))
        .copied()
    else {
        return CalculatorConsumableBuffCategory::default();
    };
    CalculatorConsumableBuffCategory {
        category_id: Some(category_id),
        category_level,
    }
}

fn fallback_consumable_family_key(
    primary_skill_id: Option<&str>,
    primary_skill_counts: &HashMap<String, usize>,
) -> Option<String> {
    let skill_id = primary_skill_id?;
    (primary_skill_counts.get(skill_id).copied().unwrap_or(0) > 1)
        .then(|| format!("skill-family:{skill_id}"))
}

fn buff_category_key(category_id: Option<i32>) -> Option<String> {
    category_id.map(|category_id| format!("buff-category:{category_id}"))
}

fn calculator_effect_keyword_predicate(column: &str) -> String {
    [
        "낚시",
        "희귀 어종",
        "대형 어종",
        "생활 경험치",
        "생활 숙련도",
        "내구도 소모 감소 저항",
    ]
    .into_iter()
    .map(|keyword| format!("COALESCE({column}, '') LIKE '%{keyword}%'"))
    .collect::<Vec<_>>()
    .join(" OR ")
}

fn normalized_db_field(row: &BTreeMap<String, String>, key: &str) -> Option<String> {
    row.get(key)
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .filter(|value| !is_null_marker(value))
        .map(str::to_string)
}

fn parse_optional_i32(value: Option<String>) -> Option<i32> {
    value.and_then(|value| value.parse::<i32>().ok())
}

fn optional_i32_to_string(value: Option<i32>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

fn optional_f32_to_string(value: Option<f32>) -> String {
    value
        .map(|value| format_float(f64::from(value)))
        .unwrap_or_default()
}

fn quote_sql_string_list(values: &[String]) -> String {
    values
        .iter()
        .map(|value| sql_value(value))
        .collect::<Vec<_>>()
        .join(",")
}

fn run_calculator_progression_import(command: CalculatorProgressionImportCommand) -> Result<()> {
    let CalculatorProgressionImportCommand {
        dolt_repo,
        excel_dir,
        output_dir,
        commit,
        commit_msg,
    } = command;

    let workbook_set = resolve_calculator_progression_workbooks(&excel_dir)?;
    let output_dir = match output_dir {
        Some(path) => path,
        None => default_output_dir()?,
    };
    fs::create_dir_all(&output_dir)
        .with_context(|| format!("create output dir: {}", output_dir.display()))?;

    let digests = CalculatorProgressionDigests {
        common_stat_data_sha: sha256_file(&workbook_set.common_stat_data_xlsx)?,
        fishing_stat_data_sha: sha256_file(&workbook_set.fishing_stat_data_xlsx)?,
        translate_stat_sha: sha256_file(&workbook_set.translate_stat_xlsx)?,
    };

    let outputs = CalculatorProgressionOutputs {
        common_stat_data_csv: output_dir.join("common_stat_data.csv"),
        fishing_stat_data_csv: output_dir.join("fishing_stat_data.csv"),
        translate_stat_csv: output_dir.join("translate_stat.csv"),
    };

    let common_stat_data_stats = import_workbook_sheet(
        &workbook_set.common_stat_data_xlsx,
        "CommonStatData",
        &COMMON_STAT_DATA_HEADERS,
        &outputs.common_stat_data_csv,
    )?;
    let fishing_stat_data_stats = import_workbook_sheet(
        &workbook_set.fishing_stat_data_xlsx,
        "FishingStatData",
        &FISHING_STAT_DATA_HEADERS,
        &outputs.fishing_stat_data_csv,
    )?;
    let translate_stat_stats = import_workbook_sheet(
        &workbook_set.translate_stat_xlsx,
        "TranslateStat",
        &TRANSLATE_STAT_HEADERS,
        &outputs.translate_stat_csv,
    )?;

    run_dolt_sql_table_import(
        &dolt_repo,
        "common_stat_data",
        &outputs.common_stat_data_csv,
    )?;
    run_dolt_sql_table_import(
        &dolt_repo,
        "fishing_stat_data",
        &outputs.fishing_stat_data_csv,
    )?;
    run_dolt_sql_table_import(&dolt_repo, "translate_stat", &outputs.translate_stat_csv)?;

    if commit {
        let msg = build_calculator_progression_commit_message(commit_msg, &digests);
        run_dolt_commit(&dolt_repo, &msg)?;
    }

    println!(
        "common_stat_data rows imported: {}",
        common_stat_data_stats.row_count
    );
    println!(
        "fishing_stat_data rows imported: {}",
        fishing_stat_data_stats.row_count
    );
    println!(
        "translate_stat rows imported: {}",
        translate_stat_stats.row_count
    );

    Ok(())
}

fn run_flockfish_subgroup_import(command: FlockfishSubgroupImportCommand) -> Result<()> {
    let FlockfishSubgroupImportCommand {
        dolt_repo,
        workbook_xlsx,
        output_dir,
        commit,
        commit_msg,
    } = command;

    let output_dir = match output_dir {
        Some(path) => path,
        None => default_output_dir()?,
    };
    fs::create_dir_all(&output_dir)
        .with_context(|| format!("create output dir: {}", output_dir.display()))?;

    let workbook_sha = sha256_file(&workbook_xlsx)?;
    let outputs = FlockfishSubgroupOutputs {
        main_group_csv: output_dir.join("item_main_group_table.csv"),
        sub_group_csv: output_dir.join("item_sub_group_table.csv"),
        zone_group_slots_csv: output_dir.join("flockfish_zone_group_slots.csv"),
    };
    let stats = import_flockfish_group_tables(
        &workbook_xlsx,
        &workbook_sha,
        &outputs.main_group_csv,
        &outputs.sub_group_csv,
        &outputs.zone_group_slots_csv,
    )?;

    run_dolt_sql_table_import_or_remote(
        &dolt_repo,
        "item_main_group_table",
        &outputs.main_group_csv,
    )?;
    run_dolt_sql_table_import_or_remote(
        &dolt_repo,
        "item_sub_group_table",
        &outputs.sub_group_csv,
    )?;
    ensure_flockfish_zone_group_slots_table(&dolt_repo)?;
    run_dolt_sql_table_import(
        &dolt_repo,
        "flockfish_zone_group_slots",
        &outputs.zone_group_slots_csv,
    )?;

    if commit {
        let msg = match commit_msg {
            Some(msg) => format!("{msg} (FlockfishWorkbook={workbook_sha})"),
            None => {
                format!("Import flockfish fishing group tables (FlockfishWorkbook={workbook_sha})")
            }
        };
        run_dolt_commit(&dolt_repo, &msg)?;
    }

    println!(
        "flockfish main-group rows emitted: {}",
        stats.main_group.row_count
    );
    println!(
        "output main-group csv: {}",
        outputs.main_group_csv.display()
    );
    println!(
        "flockfish subgroup rows emitted: {}",
        stats.sub_group.row_count
    );
    println!("output subgroup csv: {}", outputs.sub_group_csv.display());
    println!(
        "flockfish resolved zone-group rows emitted: {}",
        stats.zone_group_slots.row_count
    );
    println!(
        "flockfish resolved numeric zone-group rows: {}",
        stats.zone_group_slots.numeric_rows
    );
    println!(
        "flockfish unresolved zone-group rows: {}",
        stats.zone_group_slots.unresolved_rows
    );
    println!(
        "output resolved zone-group csv: {}",
        outputs.zone_group_slots_csv.display()
    );

    Ok(())
}

fn run_trade_npc_catalog_import(command: TradeNpcCatalogImportCommand) -> Result<()> {
    let TradeNpcCatalogImportCommand {
        dolt_repo,
        catalog_json,
        output_dir,
        commit,
        commit_msg,
    } = command;

    let output_dir = match output_dir {
        Some(path) => path,
        None => default_output_dir()?,
    };
    fs::create_dir_all(&output_dir)
        .with_context(|| format!("create output dir: {}", output_dir.display()))?;

    let catalog_sha = sha256_file(&catalog_json)?;
    let catalog_file = File::open(&catalog_json)
        .with_context(|| format!("open trade NPC catalog json: {}", catalog_json.display()))?;
    let catalog: TradeNpcCatalogResponse = serde_json::from_reader(catalog_file)
        .with_context(|| format!("parse trade NPC catalog json: {}", catalog_json.display()))?;

    let outputs = TradeNpcCatalogOutputs {
        meta_csv: output_dir.join("trade_npc_catalog_meta.csv"),
        sources_csv: output_dir.join("trade_npc_catalog_sources.csv"),
        origin_regions_csv: output_dir.join("trade_origin_regions.csv"),
        zone_origin_regions_csv: output_dir.join("trade_zone_origin_regions.csv"),
        destinations_csv: output_dir.join("trade_npc_destinations.csv"),
        excluded_csv: output_dir.join("trade_npc_excluded.csv"),
    };
    write_trade_npc_catalog_csvs(&catalog, &outputs)?;

    ensure_trade_npc_catalog_tables(&dolt_repo)?;
    run_dolt_sql_table_import_or_remote(
        &dolt_repo,
        TRADE_NPC_CATALOG_META_TABLE,
        &outputs.meta_csv,
    )?;
    run_dolt_sql_table_import_or_remote(
        &dolt_repo,
        TRADE_NPC_CATALOG_SOURCES_TABLE,
        &outputs.sources_csv,
    )?;
    run_dolt_sql_table_import_or_remote(
        &dolt_repo,
        TRADE_ORIGIN_REGIONS_TABLE,
        &outputs.origin_regions_csv,
    )?;
    run_dolt_sql_table_import_or_remote(
        &dolt_repo,
        TRADE_ZONE_ORIGIN_REGIONS_TABLE,
        &outputs.zone_origin_regions_csv,
    )?;
    run_dolt_sql_table_import_or_remote(
        &dolt_repo,
        TRADE_NPC_DESTINATIONS_TABLE,
        &outputs.destinations_csv,
    )?;
    run_dolt_sql_table_import_or_remote(
        &dolt_repo,
        TRADE_NPC_EXCLUDED_TABLE,
        &outputs.excluded_csv,
    )?;

    if commit {
        let msg = match commit_msg {
            Some(msg) => format!("{msg} (TradeNpcCatalog={catalog_sha})"),
            None => format!("Import trade NPC catalog (TradeNpcCatalog={catalog_sha})"),
        };
        run_dolt_commit(&dolt_repo, &msg)?;
    }

    println!(
        "trade origin rows emitted: {}",
        catalog.origin_regions.len()
    );
    println!(
        "trade zone-origin rows emitted: {}",
        catalog
            .zone_origin_regions
            .iter()
            .map(|entry| entry.origins.len())
            .sum::<usize>()
    );
    println!(
        "trade destination rows emitted: {}",
        catalog.destinations.len()
    );
    println!("trade excluded rows emitted: {}", catalog.excluded.len());
    println!("output trade meta csv: {}", outputs.meta_csv.display());
    println!(
        "output trade destinations csv: {}",
        outputs.destinations_csv.display()
    );

    Ok(())
}

fn run_community_subgroup_overlay_import(
    command: CommunitySubgroupOverlayImportCommand,
) -> Result<()> {
    let CommunitySubgroupOverlayImportCommand {
        dolt_repo,
        subgroups_xlsx,
        sheet,
        source_id,
        source_label,
        output_dir,
        emit_only,
        activate,
        commit,
        commit_msg,
    } = command;

    validate_import_source_id(&source_id)?;
    if source_label.trim().is_empty() {
        bail!("--source-label cannot be empty");
    }
    if emit_only && activate {
        bail!("--emit-only cannot be combined with --activate");
    }
    if emit_only && commit {
        bail!("--emit-only cannot be combined with --commit");
    }

    let output_dir = match output_dir {
        Some(path) => path,
        None => default_output_dir()?,
    };
    fs::create_dir_all(&output_dir)
        .with_context(|| format!("create output dir: {}", output_dir.display()))?;

    let workbook_sha = sha256_file(&subgroups_xlsx)?;
    let outputs = CommunitySubgroupOverlayOutputs {
        overlay_csv: output_dir.join("community_item_sub_group_overlay.csv"),
        unresolved_csv: output_dir.join("community_item_sub_group_unresolved_overlay.csv"),
    };
    let stats = import_community_subgroup_overlay_sheet(
        &subgroups_xlsx,
        &workbook_sha,
        &sheet,
        &source_id,
        &source_label,
        &outputs.overlay_csv,
        &outputs.unresolved_csv,
    )?;

    if !emit_only {
        replace_community_subgroup_overlay_source(
            &dolt_repo,
            &source_id,
            &outputs.overlay_csv,
            &outputs.unresolved_csv,
        )?;
        if activate {
            activate_community_subgroup_overlay_source(
                &dolt_repo,
                &source_id,
                &source_label,
                &workbook_sha,
            )?;
        }

        if commit {
            let msg = match commit_msg {
                Some(msg) => format!("{msg} (CommunitySubgroups={workbook_sha})"),
                None => {
                    format!("Import community subgroup overlay (CommunitySubgroups={workbook_sha})")
                }
            };
            run_dolt_commit(&dolt_repo, &msg)?;
        }
    }

    println!(
        "community subgroup overlay rows emitted: {}",
        stats.row_count
    );
    println!("community subgroup active rows: {}", stats.active_rows);
    println!("community subgroup removed rows: {}", stats.removed_rows);
    println!("community subgroup added rows: {}", stats.added_rows);
    println!("community subgroup note rows skipped: {}", stats.note_rows);
    println!(
        "community subgroup unresolved rows preserved: {}",
        stats.unresolved_rows
    );
    println!(
        "community subgroup unresolved symbolic key rows: {}",
        stats.unresolved_symbolic_key_rows
    );
    println!(
        "community subgroup unresolved missing item-key rows: {}",
        stats.unresolved_missing_item_key_rows
    );
    println!("community subgroup source id: {source_id}");
    println!("community subgroup workbook sha256: {workbook_sha}");
    println!("output overlay csv: {}", outputs.overlay_csv.display());
    println!(
        "output unresolved overlay csv: {}",
        outputs.unresolved_csv.display()
    );
    if emit_only {
        println!("emit only: Dolt import skipped");
    }
    if activate {
        println!("activated community subgroup overlay source: {source_id}");
    }

    Ok(())
}

fn import_flockfish_group_tables(
    workbook_xlsx: &Path,
    workbook_sha: &str,
    main_group_csv: &Path,
    sub_group_csv: &Path,
    zone_group_slots_csv: &Path,
) -> Result<FlockfishGroupsImport> {
    let main_group_rows = load_flockfish_main_group_rows(workbook_xlsx)?;
    let main_group_stats = FlockfishTableImportStats {
        row_count: main_group_rows.len(),
    };
    write_group_rows_csv(main_group_csv, &MAIN_GROUP_HEADERS, main_group_rows)?;

    let sub_group_rows = load_flockfish_sub_group_rows(workbook_xlsx)?;
    let sub_group_stats = FlockfishTableImportStats {
        row_count: sub_group_rows.len(),
    };
    write_group_rows_csv(sub_group_csv, &SUB_GROUP_HEADERS, sub_group_rows)?;

    let zone_group_slots =
        import_flockfish_zone_group_slots(workbook_xlsx, workbook_sha, zone_group_slots_csv)?;

    Ok(FlockfishGroupsImport {
        main_group: main_group_stats,
        sub_group: sub_group_stats,
        zone_group_slots,
    })
}

fn write_group_rows_csv<I>(output_csv: &Path, headers: &[&str], rows: I) -> Result<()>
where
    I: IntoIterator<Item = Vec<String>>,
{
    let mut writer = build_csv_writer(output_csv)?;
    writer.write_record(headers)?;
    for row in rows {
        writer.write_record(row)?;
    }
    writer.flush()?;
    Ok(())
}

fn load_flockfish_main_group_rows(workbook_xlsx: &Path) -> Result<Vec<Vec<String>>> {
    load_flockfish_sheet_rows(workbook_xlsx, "Maingroup", &MAIN_GROUP_HEADERS)
}

fn load_flockfish_sub_group_rows(workbook_xlsx: &Path) -> Result<Vec<Vec<String>>> {
    load_flockfish_sheet_rows(workbook_xlsx, "Subgroup", &SUB_GROUP_HEADERS).map(|rows| {
        rows.into_iter()
            .filter(|row| !is_removed_flockfish_subgroup_outlier(row))
            .collect()
    })
}

fn is_removed_flockfish_subgroup_outlier(row: &[String]) -> bool {
    matches!(
        (
            row.get(SUB_GROUP_KEY_COL).map(String::as_str),
            row.get(1).map(String::as_str),
        ),
        (Some("10956"), Some("43871"))
    )
}

fn load_flockfish_sheet_rows(
    workbook_xlsx: &Path,
    sheet_name: &str,
    expected_headers: &[&str],
) -> Result<Vec<Vec<String>>> {
    let range = read_sheet(workbook_xlsx, sheet_name)?;
    let headers = read_headers(&range)?;
    validate_headers_normalized(
        &headers,
        expected_headers,
        &format!("{}:{sheet_name}", workbook_xlsx.display()),
    )?;

    let mut rows_out = Vec::new();
    for row in range.rows().skip(1) {
        if row_is_empty(row) {
            continue;
        }
        let Some(first_cell) = cell_to_string_opt(row.get(0))? else {
            continue;
        };
        if first_cell
            .parse::<i64>()
            .ok()
            .filter(|value| *value > 0)
            .is_none()
        {
            continue;
        }
        let mut record = build_record(row, expected_headers.len())?;
        for value in &mut record {
            *value = normalize_flockfish_numeric_literal(value);
        }
        rows_out.push(record);
    }
    Ok(rows_out)
}

fn import_flockfish_zone_group_slots(
    workbook_xlsx: &Path,
    workbook_sha: &str,
    output_csv: &Path,
) -> Result<FlockfishZoneGroupSlotsImport> {
    let range = read_sheet(workbook_xlsx, "Jallo - New Fish Work Sheet")?;
    let mut by_key = BTreeMap::<(u32, u8), Vec<String>>::new();
    let mut numeric_rows = 0usize;
    let mut unresolved_rows = 0usize;

    for row in range.rows().skip(2) {
        if row_is_empty(row) {
            continue;
        }

        let Some(zone_r_raw) = cell_to_string_opt(row.get(FLOCKFISH_JALLO_FINAL_R_COL))? else {
            continue;
        };
        let Some(zone_g_raw) = cell_to_string_opt(row.get(FLOCKFISH_JALLO_FINAL_G_COL))? else {
            continue;
        };
        let Some(zone_b_raw) = cell_to_string_opt(row.get(FLOCKFISH_JALLO_FINAL_B_COL))? else {
            continue;
        };
        let Ok(zone_r_i64) = zone_r_raw.parse::<i64>() else {
            continue;
        };
        let Ok(zone_g_i64) = zone_g_raw.parse::<i64>() else {
            continue;
        };
        let Ok(zone_b_i64) = zone_b_raw.parse::<i64>() else {
            continue;
        };
        let Some(zone_name) = cell_to_string_opt(row.get(FLOCKFISH_JALLO_FINAL_ZONE_NAME_COL))?
        else {
            continue;
        };
        let Some(source_drop_label) =
            cell_to_string_opt(row.get(FLOCKFISH_JALLO_FINAL_DROP_LABEL_COL))?
        else {
            continue;
        };
        let Some(slot_idx) = flockfish_drop_label_to_slot_idx(&source_drop_label) else {
            continue;
        };

        let zone_r = u8::try_from(zone_r_i64)
            .with_context(|| format!("zone R out of range: {zone_r_i64}"))?;
        let zone_g = u8::try_from(zone_g_i64)
            .with_context(|| format!("zone G out of range: {zone_g_i64}"))?;
        let zone_b = u8::try_from(zone_b_i64)
            .with_context(|| format!("zone B out of range: {zone_b_i64}"))?;
        let zone_rgb = (u32::from(zone_r) << 16) | (u32::from(zone_g) << 8) | u32::from(zone_b);

        let resolution_value_raw =
            cell_to_string_opt(row.get(FLOCKFISH_JALLO_FINAL_GROUP_VALUE_COL))?.unwrap_or_default();
        let (item_main_group_key, resolution_status) =
            parse_flockfish_zone_group_value(&resolution_value_raw);
        if item_main_group_key.is_some() {
            numeric_rows += 1;
        } else {
            unresolved_rows += 1;
        }

        let record = vec![
            FLOCKFISH_SOURCE_ID.to_string(),
            FLOCKFISH_ZONE_GROUP_SOURCE_LABEL.to_string(),
            workbook_sha.to_string(),
            zone_rgb.to_string(),
            zone_r.to_string(),
            zone_g.to_string(),
            zone_b.to_string(),
            zone_name,
            source_drop_label,
            slot_idx.to_string(),
            item_main_group_key
                .map(|value| value.to_string())
                .unwrap_or_default(),
            resolution_status.to_string(),
            resolution_value_raw,
        ];

        let key = (zone_rgb, slot_idx);
        if let Some(existing) = by_key.insert(key, record.clone()) {
            if existing != record {
                bail!(
                    "conflicting flockfish zone-group rows for rgb={} slot_idx={slot_idx}",
                    zone_rgb
                );
            }
        }
    }

    let row_count = by_key.len();
    write_group_rows_csv(
        output_csv,
        &FLOCKFISH_ZONE_GROUP_SLOT_HEADERS,
        by_key.into_values(),
    )?;

    Ok(FlockfishZoneGroupSlotsImport {
        row_count,
        numeric_rows,
        unresolved_rows,
    })
}

fn ensure_flockfish_zone_group_slots_table(dolt_repo: &Path) -> Result<()> {
    run_dolt_sql_query(
        dolt_repo,
        "CREATE TABLE IF NOT EXISTS `flockfish_zone_group_slots` (\
            `source_id` VARCHAR(64) NOT NULL,\
            `source_label` VARCHAR(255) NOT NULL,\
            `source_sha256` CHAR(64) NOT NULL,\
            `zone_rgb` INT UNSIGNED NOT NULL,\
            `zone_r` TINYINT UNSIGNED NOT NULL,\
            `zone_g` TINYINT UNSIGNED NOT NULL,\
            `zone_b` TINYINT UNSIGNED NOT NULL,\
            `zone_name` VARCHAR(255) NOT NULL,\
            `source_drop_label` VARCHAR(64) NOT NULL,\
            `slot_idx` TINYINT UNSIGNED NOT NULL,\
            `item_main_group_key` BIGINT NULL,\
            `resolution_status` VARCHAR(32) NOT NULL,\
            `resolution_value_raw` VARCHAR(255) NULL,\
            PRIMARY KEY (`source_id`, `zone_rgb`, `slot_idx`),\
            KEY `idx_zone_rgb_slot` (`zone_rgb`, `slot_idx`),\
            KEY `idx_resolution_status` (`resolution_status`)\
        );",
        "ensure flockfish_zone_group_slots table",
    )
}

fn write_trade_npc_catalog_csvs(
    catalog: &TradeNpcCatalogResponse,
    outputs: &TradeNpcCatalogOutputs,
) -> Result<()> {
    {
        let mut writer = build_csv_writer(&outputs.meta_csv)?;
        writer.write_record([
            "catalog_schema",
            "version",
            "coordinate_space",
            "character_function_trade_rows",
            "character_function_barter_rows",
            "character_function_trade_barter_overlap_rows",
            "selling_to_npc_rows",
            "title_trade_manager_rows",
            "candidate_npcs",
            "origin_regions",
            "zone_origin_regions",
            "destinations",
            "excluded_missing_spawn",
            "excluded_missing_trade_origin",
        ])?;
        writer.write_record([
            catalog.schema.clone(),
            catalog.version.to_string(),
            catalog.coordinate_space.clone(),
            catalog.summary.character_function_trade_rows.to_string(),
            catalog.summary.character_function_barter_rows.to_string(),
            catalog
                .summary
                .character_function_trade_barter_overlap_rows
                .to_string(),
            catalog.summary.selling_to_npc_rows.to_string(),
            catalog.summary.title_trade_manager_rows.to_string(),
            catalog.summary.candidate_npcs.to_string(),
            catalog.summary.origin_regions.to_string(),
            catalog.summary.zone_origin_regions.to_string(),
            catalog.summary.destinations.to_string(),
            catalog.summary.excluded_missing_spawn.to_string(),
            catalog.summary.excluded_missing_trade_origin.to_string(),
        ])?;
        writer.flush()?;
    }

    {
        let mut writer = build_csv_writer(&outputs.sources_csv)?;
        writer.write_record(["source_id", "file", "role"])?;
        for source in &catalog.sources {
            writer.write_record([
                source.id.as_str(),
                source.file.as_str(),
                source.role.as_str(),
            ])?;
        }
        writer.flush()?;
    }

    {
        let mut writer = build_csv_writer(&outputs.origin_regions_csv)?;
        writer.write_record([
            "region_id",
            "region_name",
            "waypoint_id",
            "waypoint_name",
            "world_x",
            "world_z",
        ])?;
        for origin in &catalog.origin_regions {
            writer.write_record([
                origin.region_id.to_string(),
                optional_string_csv(origin.region_name.as_deref()),
                optional_u32_csv(origin.waypoint_id),
                optional_string_csv(origin.waypoint_name.as_deref()),
                origin.world_x.to_string(),
                origin.world_z.to_string(),
            ])?;
        }
        writer.flush()?;
    }

    {
        let mut writer = build_csv_writer(&outputs.zone_origin_regions_csv)?;
        writer.write_record([
            "zone_rgb_key",
            "zone_rgb_u32",
            "origin_region_id",
            "pixel_count",
        ])?;
        for zone in &catalog.zone_origin_regions {
            for origin in &zone.origins {
                writer.write_record([
                    zone.zone_rgb_key.clone(),
                    zone.zone_rgb_u32.to_string(),
                    origin.region_id.to_string(),
                    origin.pixel_count.to_string(),
                ])?;
            }
        }
        writer.flush()?;
    }

    {
        let mut writer = build_csv_writer(&outputs.destinations_csv)?;
        writer.write_record([
            "destination_id",
            "npc_key",
            "npc_name",
            "role_source",
            "source_tags_json",
            "item_main_group_key",
            "trade_group_type",
            "npc_spawn_region_id",
            "npc_spawn_region_name",
            "npc_spawn_world_x",
            "npc_spawn_world_y",
            "npc_spawn_world_z",
            "assigned_region_id",
            "assigned_region_name",
            "assigned_waypoint_id",
            "assigned_waypoint_name",
            "assigned_world_x",
            "assigned_world_z",
            "sell_origin_region_id",
            "sell_origin_region_name",
            "sell_origin_waypoint_id",
            "sell_origin_waypoint_name",
            "sell_origin_world_x",
            "sell_origin_world_z",
        ])?;
        for destination in &catalog.destinations {
            writer.write_record([
                destination.id.clone(),
                destination.npc_key.to_string(),
                destination.npc_name.clone(),
                destination.role_source.clone(),
                serde_json::to_string(&destination.source_tags)?,
                optional_u32_csv(destination.trade.item_main_group_key),
                optional_string_csv(destination.trade.trade_group_type.as_deref()),
                destination.npc_spawn.region_id.to_string(),
                optional_string_csv(destination.npc_spawn.region_name.as_deref()),
                destination.npc_spawn.world_x.to_string(),
                destination.npc_spawn.world_y.to_string(),
                destination.npc_spawn.world_z.to_string(),
                optional_u32_csv(destination.assigned_region.region_id),
                optional_string_csv(destination.assigned_region.region_name.as_deref()),
                optional_u32_csv(destination.assigned_region.waypoint_id),
                optional_string_csv(destination.assigned_region.waypoint_name.as_deref()),
                optional_f64_csv(destination.assigned_region.world_x),
                optional_f64_csv(destination.assigned_region.world_z),
                optional_u32_csv(destination.sell_destination_trade_origin.region_id),
                optional_string_csv(
                    destination
                        .sell_destination_trade_origin
                        .region_name
                        .as_deref(),
                ),
                optional_u32_csv(destination.sell_destination_trade_origin.waypoint_id),
                optional_string_csv(
                    destination
                        .sell_destination_trade_origin
                        .waypoint_name
                        .as_deref(),
                ),
                optional_f64_csv(destination.sell_destination_trade_origin.world_x),
                optional_f64_csv(destination.sell_destination_trade_origin.world_z),
            ])?;
        }
        writer.flush()?;
    }

    {
        let mut writer = build_csv_writer(&outputs.excluded_csv)?;
        writer.write_record(["npc_key", "npc_name", "reason", "source_tags_json"])?;
        for excluded in &catalog.excluded {
            writer.write_record([
                excluded.npc_key.to_string(),
                excluded.npc_name.clone(),
                excluded.reason.clone(),
                serde_json::to_string(&excluded.source_tags)?,
            ])?;
        }
        writer.flush()?;
    }

    Ok(())
}

fn optional_string_csv(value: Option<&str>) -> String {
    value.unwrap_or_default().to_string()
}

fn optional_u32_csv(value: Option<u32>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

fn optional_f64_csv(value: Option<f64>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

fn ensure_trade_npc_catalog_tables(dolt_repo: &Path) -> Result<()> {
    run_dolt_sql_query_or_remote(
        dolt_repo,
        "CREATE TABLE IF NOT EXISTS `trade_npc_catalog_meta` (\
            `catalog_schema` VARCHAR(128) NOT NULL PRIMARY KEY,\
            `version` INT UNSIGNED NOT NULL,\
            `coordinate_space` VARCHAR(64) NOT NULL,\
            `character_function_trade_rows` BIGINT UNSIGNED NOT NULL,\
            `character_function_barter_rows` BIGINT UNSIGNED NOT NULL,\
            `character_function_trade_barter_overlap_rows` BIGINT UNSIGNED NOT NULL,\
            `selling_to_npc_rows` BIGINT UNSIGNED NOT NULL,\
            `title_trade_manager_rows` BIGINT UNSIGNED NOT NULL,\
            `candidate_npcs` BIGINT UNSIGNED NOT NULL,\
            `origin_regions` BIGINT UNSIGNED NOT NULL,\
            `zone_origin_regions` BIGINT UNSIGNED NOT NULL,\
            `destinations` BIGINT UNSIGNED NOT NULL,\
            `excluded_missing_spawn` BIGINT UNSIGNED NOT NULL,\
            `excluded_missing_trade_origin` BIGINT UNSIGNED NOT NULL\
        );\
        CREATE TABLE IF NOT EXISTS `trade_npc_catalog_sources` (\
            `source_id` VARCHAR(64) NOT NULL PRIMARY KEY,\
            `file` VARCHAR(255) NOT NULL,\
            `role` TEXT NOT NULL\
        );\
        CREATE TABLE IF NOT EXISTS `trade_origin_regions` (\
            `region_id` INT UNSIGNED NOT NULL PRIMARY KEY,\
            `region_name` VARCHAR(255) NULL,\
            `waypoint_id` INT UNSIGNED NULL,\
            `waypoint_name` VARCHAR(255) NULL,\
            `world_x` DOUBLE NOT NULL,\
            `world_z` DOUBLE NOT NULL,\
            KEY `idx_waypoint_id` (`waypoint_id`)\
        );\
        CREATE TABLE IF NOT EXISTS `trade_zone_origin_regions` (\
            `zone_rgb_key` VARCHAR(32) NOT NULL,\
            `zone_rgb_u32` INT UNSIGNED NOT NULL,\
            `origin_region_id` INT UNSIGNED NOT NULL,\
            `pixel_count` INT UNSIGNED NOT NULL,\
            PRIMARY KEY (`zone_rgb_u32`, `origin_region_id`),\
            KEY `idx_origin_region_id` (`origin_region_id`)\
        );\
        CREATE TABLE IF NOT EXISTS `trade_npc_destinations` (\
            `destination_id` VARCHAR(64) NOT NULL PRIMARY KEY,\
            `npc_key` INT UNSIGNED NOT NULL,\
            `npc_name` VARCHAR(255) NOT NULL,\
            `role_source` VARCHAR(64) NOT NULL,\
            `source_tags_json` LONGTEXT NULL,\
            `item_main_group_key` INT UNSIGNED NULL,\
            `trade_group_type` VARCHAR(64) NULL,\
            `npc_spawn_region_id` INT UNSIGNED NOT NULL,\
            `npc_spawn_region_name` VARCHAR(255) NULL,\
            `npc_spawn_world_x` DOUBLE NOT NULL,\
            `npc_spawn_world_y` DOUBLE NOT NULL,\
            `npc_spawn_world_z` DOUBLE NOT NULL,\
            `assigned_region_id` INT UNSIGNED NULL,\
            `assigned_region_name` VARCHAR(255) NULL,\
            `assigned_waypoint_id` INT UNSIGNED NULL,\
            `assigned_waypoint_name` VARCHAR(255) NULL,\
            `assigned_world_x` DOUBLE NULL,\
            `assigned_world_z` DOUBLE NULL,\
            `sell_origin_region_id` INT UNSIGNED NULL,\
            `sell_origin_region_name` VARCHAR(255) NULL,\
            `sell_origin_waypoint_id` INT UNSIGNED NULL,\
            `sell_origin_waypoint_name` VARCHAR(255) NULL,\
            `sell_origin_world_x` DOUBLE NULL,\
            `sell_origin_world_z` DOUBLE NULL,\
            KEY `idx_npc_key` (`npc_key`),\
            KEY `idx_spawn_region` (`npc_spawn_region_id`),\
            KEY `idx_sell_origin_region` (`sell_origin_region_id`)\
        );\
        CREATE TABLE IF NOT EXISTS `trade_npc_excluded` (\
            `npc_key` INT UNSIGNED NOT NULL,\
            `npc_name` VARCHAR(255) NOT NULL,\
            `reason` VARCHAR(128) NOT NULL,\
            `source_tags_json` LONGTEXT NULL,\
            PRIMARY KEY (`npc_key`, `reason`)\
        );",
        "ensure trade NPC catalog tables",
    )
}

fn import_community_subgroup_overlay_sheet(
    workbook_xlsx: &Path,
    workbook_sha: &str,
    sheet_name: &str,
    source_id: &str,
    source_label: &str,
    output_csv: &Path,
    unresolved_csv: &Path,
) -> Result<CommunitySubgroupOverlayImport> {
    let range = read_sheet(workbook_xlsx, sheet_name)?;
    let rows = range.rows().collect::<Vec<_>>();
    if rows.is_empty() {
        bail!("{}:{sheet_name} has no rows", workbook_xlsx.display());
    }
    validate_community_subgroup_overlay_headers(rows[0], workbook_xlsx, sheet_name)?;

    let mut writer = build_csv_writer(output_csv)?;
    writer.write_record(COMMUNITY_SUBGROUP_OVERLAY_HEADERS)?;
    let mut unresolved_writer = build_csv_writer(unresolved_csv)?;
    unresolved_writer.write_record(COMMUNITY_SUBGROUP_UNRESOLVED_HEADERS)?;

    let mut seen_keys = BTreeSet::<(i64, i64, i64)>::new();
    let mut row_count = 0usize;
    let mut active_rows = 0usize;
    let mut removed_rows = 0usize;
    let mut added_rows = 0usize;
    let mut note_rows = 0usize;
    let mut unresolved_rows = 0usize;
    let mut unresolved_symbolic_key_rows = 0usize;
    let mut unresolved_missing_item_key_rows = 0usize;

    for (idx, row) in rows.into_iter().enumerate().skip(1) {
        if row_is_empty(row) {
            continue;
        }
        let first_cell = cell_to_source_string_opt(row.get(COMMUNITY_SUBGROUP_KEY_COL));
        if first_cell.as_deref() == Some("<Note>") {
            note_rows += 1;
            continue;
        }
        let item_sub_group_key = cell_to_i64_import_key_opt(row.get(COMMUNITY_SUBGROUP_KEY_COL))?;
        let Some(item_sub_group_key) = item_sub_group_key.filter(|value| *value > 0) else {
            if row_has_community_subgroup_unresolved_payload(row) {
                let source_item_key =
                    cell_to_i64_import_key_opt(row.get(COMMUNITY_SUBGROUP_ITEM_COL))?;
                let reason = if source_item_key.filter(|value| *value > 0).is_some() {
                    unresolved_symbolic_key_rows += 1;
                    "symbolic_subgroup_key"
                } else {
                    "unresolved_subgroup_key"
                };
                let source_removed = cell_to_bool_flag(row.get(COMMUNITY_SUBGROUP_REMOVED_COL))?;
                let source_added = cell_to_bool_flag(row.get(COMMUNITY_SUBGROUP_ADDED_COL))?;
                unresolved_writer.write_record(build_community_subgroup_unresolved_record(
                    row,
                    source_id,
                    source_label,
                    workbook_sha,
                    sheet_name,
                    idx + 1,
                    reason,
                    source_removed,
                    source_added,
                )?)?;
                unresolved_rows += 1;
            }
            continue;
        };
        let Some(item_key) = cell_to_i64_import_key_opt(row.get(COMMUNITY_SUBGROUP_ITEM_COL))?
            .filter(|value| *value > 0)
        else {
            if row_has_community_subgroup_unresolved_payload(row) {
                unresolved_missing_item_key_rows += 1;
                let source_removed = cell_to_bool_flag(row.get(COMMUNITY_SUBGROUP_REMOVED_COL))?;
                let source_added = cell_to_bool_flag(row.get(COMMUNITY_SUBGROUP_ADDED_COL))?;
                unresolved_writer.write_record(build_community_subgroup_unresolved_record(
                    row,
                    source_id,
                    source_label,
                    workbook_sha,
                    sheet_name,
                    idx + 1,
                    "missing_item_key",
                    source_removed,
                    source_added,
                )?)?;
                unresolved_rows += 1;
            }
            continue;
        };
        let enchant_level = cell_to_i64_opt(row.get(COMMUNITY_SUBGROUP_ENCHANT_COL))?.unwrap_or(0);
        let key = (item_sub_group_key, item_key, enchant_level);
        if !seen_keys.insert(key) {
            bail!(
                "duplicate community subgroup overlay key in {}:{sheet_name}: ItemSubGroupKey={} ItemKey={} EnchantLevel={}",
                workbook_xlsx.display(),
                item_sub_group_key,
                item_key,
                enchant_level
            );
        }

        let source_removed = cell_to_bool_flag(row.get(COMMUNITY_SUBGROUP_REMOVED_COL))?;
        let source_added = cell_to_bool_flag(row.get(COMMUNITY_SUBGROUP_ADDED_COL))?;
        let record = build_community_subgroup_overlay_record(
            row,
            source_id,
            source_label,
            workbook_sha,
            sheet_name,
            idx + 1,
            source_removed,
            source_added,
        )?;
        writer.write_record(record)?;
        row_count += 1;
        if source_removed {
            removed_rows += 1;
        } else {
            active_rows += 1;
        }
        if source_added {
            added_rows += 1;
        }
    }

    writer.flush()?;
    unresolved_writer.flush()?;
    Ok(CommunitySubgroupOverlayImport {
        row_count,
        active_rows,
        removed_rows,
        added_rows,
        note_rows,
        unresolved_rows,
        unresolved_symbolic_key_rows,
        unresolved_missing_item_key_rows,
    })
}

fn build_community_subgroup_overlay_record(
    row: &[Data],
    source_id: &str,
    source_label: &str,
    workbook_sha: &str,
    sheet_name: &str,
    source_row: usize,
    source_removed: bool,
    source_added: bool,
) -> Result<Vec<String>> {
    let core_cols = [
        COMMUNITY_SUBGROUP_KEY_COL,
        COMMUNITY_SUBGROUP_ITEM_COL,
        COMMUNITY_SUBGROUP_ENCHANT_COL,
        COMMUNITY_SUBGROUP_DO_PET_COL,
        COMMUNITY_SUBGROUP_DO_SECHI_COL,
        COMMUNITY_SUBGROUP_SELECT_RATE_0_COL,
        COMMUNITY_SUBGROUP_MIN_COUNT_0_COL,
        COMMUNITY_SUBGROUP_MAX_COUNT_0_COL,
        COMMUNITY_SUBGROUP_SELECT_RATE_1_COL,
        COMMUNITY_SUBGROUP_MIN_COUNT_1_COL,
        COMMUNITY_SUBGROUP_MAX_COUNT_1_COL,
        COMMUNITY_SUBGROUP_SELECT_RATE_2_COL,
        COMMUNITY_SUBGROUP_MIN_COUNT_2_COL,
        COMMUNITY_SUBGROUP_MAX_COUNT_2_COL,
        COMMUNITY_SUBGROUP_INTIMACY_VARIATION_COL,
        COMMUNITY_SUBGROUP_EXPLORATION_POINT_COL,
        COMMUNITY_SUBGROUP_APPLY_RANDOM_PRICE_COL,
        COMMUNITY_SUBGROUP_RENT_TIME_COL,
        COMMUNITY_SUBGROUP_PRICE_OPTION_COL,
    ];

    let mut record = vec![
        source_id.to_string(),
        source_label.to_string(),
        workbook_sha.to_string(),
        sheet_name.to_string(),
        source_row.to_string(),
        normalized_optional_source_cell(row.get(COMMUNITY_SUBGROUP_SPOTTED_COL))?,
        normalized_optional_source_cell(row.get(COMMUNITY_SUBGROUP_COMMENT_COL))?,
        normalized_optional_source_cell(row.get(COMMUNITY_SUBGROUP_TABLE_COL))?,
        normalized_optional_source_cell(row.get(COMMUNITY_SUBGROUP_GRADE_COL))?,
        normalized_optional_source_cell(row.get(COMMUNITY_SUBGROUP_ITEM_NAME_COL))?,
        normalized_optional_source_cell(row.get(COMMUNITY_SUBGROUP_FOR_HUMANS_COL))?,
        bool_flag_to_string(source_removed).to_string(),
        bool_flag_to_string(source_added).to_string(),
    ];
    for col in core_cols {
        let value = normalized_optional_cell(row.get(col))?;
        record.push(normalize_flockfish_numeric_literal(&value));
    }
    Ok(record)
}

fn build_community_subgroup_unresolved_record(
    row: &[Data],
    source_id: &str,
    source_label: &str,
    workbook_sha: &str,
    sheet_name: &str,
    source_row: usize,
    source_reason: &str,
    source_removed: bool,
    source_added: bool,
) -> Result<Vec<String>> {
    let raw_cols = [
        COMMUNITY_SUBGROUP_DO_PET_COL,
        COMMUNITY_SUBGROUP_DO_SECHI_COL,
        COMMUNITY_SUBGROUP_SELECT_RATE_0_COL,
        COMMUNITY_SUBGROUP_MIN_COUNT_0_COL,
        COMMUNITY_SUBGROUP_MAX_COUNT_0_COL,
        COMMUNITY_SUBGROUP_SELECT_RATE_1_COL,
        COMMUNITY_SUBGROUP_MIN_COUNT_1_COL,
        COMMUNITY_SUBGROUP_MAX_COUNT_1_COL,
        COMMUNITY_SUBGROUP_SELECT_RATE_2_COL,
        COMMUNITY_SUBGROUP_MIN_COUNT_2_COL,
        COMMUNITY_SUBGROUP_MAX_COUNT_2_COL,
        COMMUNITY_SUBGROUP_INTIMACY_VARIATION_COL,
        COMMUNITY_SUBGROUP_EXPLORATION_POINT_COL,
        COMMUNITY_SUBGROUP_APPLY_RANDOM_PRICE_COL,
        COMMUNITY_SUBGROUP_RENT_TIME_COL,
        COMMUNITY_SUBGROUP_PRICE_OPTION_COL,
    ];

    let mut record = vec![
        source_id.to_string(),
        source_label.to_string(),
        workbook_sha.to_string(),
        sheet_name.to_string(),
        source_row.to_string(),
        source_reason.to_string(),
        normalized_optional_source_cell(row.get(COMMUNITY_SUBGROUP_KEY_COL))?,
        normalized_optional_source_cell(row.get(COMMUNITY_SUBGROUP_ITEM_COL))?,
        normalized_optional_source_cell(row.get(COMMUNITY_SUBGROUP_ENCHANT_COL))?,
        normalized_optional_source_cell(row.get(COMMUNITY_SUBGROUP_SPOTTED_COL))?,
        normalized_optional_source_cell(row.get(COMMUNITY_SUBGROUP_COMMENT_COL))?,
        normalized_optional_source_cell(row.get(COMMUNITY_SUBGROUP_TABLE_COL))?,
        normalized_optional_source_cell(row.get(COMMUNITY_SUBGROUP_GRADE_COL))?,
        normalized_optional_source_cell(row.get(COMMUNITY_SUBGROUP_ITEM_NAME_COL))?,
        normalized_optional_source_cell(row.get(COMMUNITY_SUBGROUP_FOR_HUMANS_COL))?,
        bool_flag_to_string(source_removed).to_string(),
        bool_flag_to_string(source_added).to_string(),
    ];
    for col in raw_cols {
        record.push(normalized_optional_source_cell(row.get(col))?);
    }
    Ok(record)
}

fn row_has_community_subgroup_unresolved_payload(row: &[Data]) -> bool {
    [
        COMMUNITY_SUBGROUP_KEY_COL,
        COMMUNITY_SUBGROUP_ITEM_COL,
        COMMUNITY_SUBGROUP_GRADE_COL,
        COMMUNITY_SUBGROUP_ITEM_NAME_COL,
        COMMUNITY_SUBGROUP_SELECT_RATE_0_COL,
        COMMUNITY_SUBGROUP_SELECT_RATE_1_COL,
        COMMUNITY_SUBGROUP_SELECT_RATE_2_COL,
    ]
    .iter()
    .any(|col| cell_to_source_string_opt(row.get(*col)).is_some())
}

fn validate_community_subgroup_overlay_headers(
    row: &[Data],
    workbook_xlsx: &Path,
    sheet_name: &str,
) -> Result<()> {
    let headers: Vec<String> = row.iter().map(header_cell_to_string).collect();
    let expected = [
        (COMMUNITY_SUBGROUP_KEY_COL, "ItemSubGroupKey"),
        (COMMUNITY_SUBGROUP_ITEM_COL, "%ItemKey"),
        (COMMUNITY_SUBGROUP_SPOTTED_COL, "spotted(auto)"),
        (COMMUNITY_SUBGROUP_COMMENT_COL, "comment"),
        (COMMUNITY_SUBGROUP_TABLE_COL, "table"),
        (COMMUNITY_SUBGROUP_REMOVED_COL, "removed"),
        (COMMUNITY_SUBGROUP_ADDED_COL, "added"),
        (COMMUNITY_SUBGROUP_ENCHANT_COL, "%EnchantLevel"),
        (COMMUNITY_SUBGROUP_DO_PET_COL, "DoPetAddDrop"),
        (COMMUNITY_SUBGROUP_DO_SECHI_COL, "DoSechiAddDrop"),
        (COMMUNITY_SUBGROUP_FOR_HUMANS_COL, "for humans"),
        (COMMUNITY_SUBGROUP_SELECT_RATE_0_COL, "%SelectRate_0"),
        (COMMUNITY_SUBGROUP_MIN_COUNT_0_COL, "%MinCount_0"),
        (COMMUNITY_SUBGROUP_MAX_COUNT_0_COL, "%MaxCount_0"),
        (COMMUNITY_SUBGROUP_SELECT_RATE_1_COL, "%SelectRate_1"),
        (COMMUNITY_SUBGROUP_MIN_COUNT_1_COL, "%MinCount_1"),
        (COMMUNITY_SUBGROUP_MAX_COUNT_1_COL, "%MaxCount_1"),
        (COMMUNITY_SUBGROUP_SELECT_RATE_2_COL, "%SelectRate_2"),
        (COMMUNITY_SUBGROUP_MIN_COUNT_2_COL, "%MinCount_2"),
        (COMMUNITY_SUBGROUP_MAX_COUNT_2_COL, "%MaxCount_2"),
        (
            COMMUNITY_SUBGROUP_INTIMACY_VARIATION_COL,
            "IntimacyVariation",
        ),
        (COMMUNITY_SUBGROUP_EXPLORATION_POINT_COL, "ExplorationPoint"),
        (
            COMMUNITY_SUBGROUP_APPLY_RANDOM_PRICE_COL,
            "ApplyRandomPrice",
        ),
        (COMMUNITY_SUBGROUP_RENT_TIME_COL, "RentTime"),
        (COMMUNITY_SUBGROUP_PRICE_OPTION_COL, "PriceOption"),
    ];
    for (idx, expected_value) in expected {
        let actual = headers.get(idx).map(|value| value.trim()).unwrap_or("");
        if actual != expected_value {
            bail!(
                "unexpected community subgroup overlay workbook header in {}:{sheet_name} at column {}. expected '{}' got '{}'",
                workbook_xlsx.display(),
                idx,
                expected_value,
                actual
            );
        }
    }
    Ok(())
}

fn validate_import_source_id(source_id: &str) -> Result<()> {
    if source_id.is_empty() {
        bail!("--source-id cannot be empty");
    }
    if source_id.len() > 64 {
        bail!("--source-id must be at most 64 bytes");
    }
    if !source_id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'))
    {
        bail!("--source-id may only contain ASCII letters, numbers, '_' and '-'");
    }
    Ok(())
}

fn normalized_optional_cell(cell: Option<&Data>) -> Result<String> {
    Ok(cell_to_string_opt(cell)?.unwrap_or_default())
}

fn normalized_optional_source_cell(cell: Option<&Data>) -> Result<String> {
    Ok(cell_to_source_string_opt(cell).unwrap_or_default())
}

fn cell_to_source_string_opt(cell: Option<&Data>) -> Option<String> {
    let cell = cell?;
    match cell {
        Data::Empty => None,
        Data::String(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() || is_null_marker(trimmed) {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        Data::DateTimeIso(value) | Data::DurationIso(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() || is_null_marker(trimmed) {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        Data::Float(value) => Some(format_float(*value)),
        Data::Int(value) => Some(value.to_string()),
        Data::Bool(value) => Some(if *value { "1" } else { "0" }.to_string()),
        Data::DateTime(value) => Some(format_float(value.as_f64())),
        Data::Error(err) => Some(err.to_string()),
    }
}

fn cell_to_i64_import_key_opt(cell: Option<&Data>) -> Result<Option<i64>> {
    match cell {
        Some(Data::String(value)) => {
            let trimmed = value.trim();
            if trimmed.is_empty() || is_null_marker(trimmed) {
                return Ok(None);
            }
            Ok(trimmed.parse::<i64>().ok())
        }
        Some(Data::Error(_)) => Ok(None),
        _ => cell_to_i64_opt(cell),
    }
}

fn cell_to_bool_flag(cell: Option<&Data>) -> Result<bool> {
    let Some(value) = cell_to_string_opt(cell)? else {
        return Ok(false);
    };
    match value.trim().to_ascii_lowercase().as_str() {
        "" | "0" | "false" | "no" => Ok(false),
        "1" | "true" | "yes" => Ok(true),
        other => bail!("expected boolean flag, got {other}"),
    }
}

fn bool_flag_to_string(value: bool) -> &'static str {
    if value {
        "1"
    } else {
        "0"
    }
}

fn replace_community_subgroup_overlay_source(
    dolt_repo: &Path,
    source_id: &str,
    overlay_csv: &Path,
    unresolved_csv: &Path,
) -> Result<()> {
    ensure_community_subgroup_overlay_table(dolt_repo)?;
    ensure_community_subgroup_unresolved_table(dolt_repo)?;
    prepare_community_subgroup_overlay_import_table(dolt_repo)?;
    run_dolt_sql_table_import_or_remote(
        dolt_repo,
        COMMUNITY_SUBGROUP_OVERLAY_IMPORT_TABLE,
        overlay_csv,
    )?;
    prepare_community_subgroup_unresolved_import_table(dolt_repo)?;
    run_dolt_sql_table_import_or_remote(
        dolt_repo,
        COMMUNITY_SUBGROUP_UNRESOLVED_IMPORT_TABLE,
        unresolved_csv,
    )?;

    let columns = COMMUNITY_SUBGROUP_OVERLAY_HEADERS
        .iter()
        .map(|header| sql_ident(header))
        .collect::<Vec<_>>()
        .join(", ");
    let select_columns = community_subgroup_overlay_staging_select_columns().join(", ");
    let query = format!(
        "DELETE FROM {target} WHERE `source_id` = {source_id};\
         INSERT INTO {target} ({columns}) SELECT {select_columns} FROM {staging};\
         DROP TABLE {staging};",
        target = sql_ident(COMMUNITY_SUBGROUP_OVERLAY_TABLE),
        staging = sql_ident(COMMUNITY_SUBGROUP_OVERLAY_IMPORT_TABLE),
        source_id = sql_value(source_id),
        columns = columns,
        select_columns = select_columns,
    );
    run_dolt_sql_query_or_remote(
        dolt_repo,
        &query,
        "replace community item subgroup overlay source",
    )?;

    let unresolved_columns = COMMUNITY_SUBGROUP_UNRESOLVED_HEADERS
        .iter()
        .map(|header| sql_ident(header))
        .collect::<Vec<_>>()
        .join(", ");
    let unresolved_select_columns =
        community_subgroup_unresolved_staging_select_columns().join(", ");
    let unresolved_query = format!(
        "DELETE FROM {target} WHERE `source_id` = {source_id};\
         INSERT INTO {target} ({columns}) SELECT {select_columns} FROM {staging};\
         DROP TABLE {staging};",
        target = sql_ident(COMMUNITY_SUBGROUP_UNRESOLVED_TABLE),
        staging = sql_ident(COMMUNITY_SUBGROUP_UNRESOLVED_IMPORT_TABLE),
        source_id = sql_value(source_id),
        columns = unresolved_columns,
        select_columns = unresolved_select_columns,
    );
    run_dolt_sql_query_or_remote(
        dolt_repo,
        &unresolved_query,
        "replace community item subgroup unresolved overlay source",
    )
}

fn community_subgroup_overlay_staging_select_columns() -> Vec<&'static str> {
    vec![
        "`source_id`",
        "`source_label`",
        "`source_sha256`",
        "`source_sheet`",
        "CAST(`source_row` AS UNSIGNED)",
        "NULLIF(`source_spotted_auto`, '')",
        "NULLIF(`source_comment`, '')",
        "NULLIF(`source_table`, '')",
        "NULLIF(`source_grade`, '')",
        "NULLIF(`source_item_name`, '')",
        "NULLIF(`source_for_humans`, '')",
        "CAST(`source_removed` AS UNSIGNED)",
        "CAST(`source_added` AS UNSIGNED)",
        "CAST(`ItemSubGroupKey` AS SIGNED)",
        "CAST(`ItemKey` AS SIGNED)",
        "CAST(`EnchantLevel` AS SIGNED)",
        "CAST(NULLIF(`DoPetAddDrop`, '') AS SIGNED)",
        "CAST(NULLIF(`DoSechiAddDrop`, '') AS SIGNED)",
        "CAST(NULLIF(`SelectRate_0`, '') AS SIGNED)",
        "CAST(NULLIF(`MinCount_0`, '') AS SIGNED)",
        "CAST(NULLIF(`MaxCount_0`, '') AS SIGNED)",
        "CAST(NULLIF(`SelectRate_1`, '') AS SIGNED)",
        "CAST(NULLIF(`MinCount_1`, '') AS SIGNED)",
        "CAST(NULLIF(`MaxCount_1`, '') AS SIGNED)",
        "CAST(NULLIF(`SelectRate_2`, '') AS SIGNED)",
        "CAST(NULLIF(`MinCount_2`, '') AS SIGNED)",
        "CAST(NULLIF(`MaxCount_2`, '') AS SIGNED)",
        "CAST(NULLIF(`IntimacyVariation`, '') AS SIGNED)",
        "CAST(NULLIF(`ExplorationPoint`, '') AS SIGNED)",
        "CAST(NULLIF(`ApplyRandomPrice`, '') AS SIGNED)",
        "CAST(NULLIF(`RentTime`, '') AS SIGNED)",
        "CAST(NULLIF(`PriceOption`, '') AS SIGNED)",
    ]
}

fn community_subgroup_unresolved_staging_select_columns() -> Vec<&'static str> {
    vec![
        "`source_id`",
        "`source_label`",
        "`source_sha256`",
        "`source_sheet`",
        "CAST(`source_row` AS UNSIGNED)",
        "`source_reason`",
        "NULLIF(`source_item_sub_group_key_raw`, '')",
        "NULLIF(`source_item_key_raw`, '')",
        "NULLIF(`source_enchant_level_raw`, '')",
        "NULLIF(`source_spotted_auto`, '')",
        "NULLIF(`source_comment`, '')",
        "NULLIF(`source_table`, '')",
        "NULLIF(`source_grade`, '')",
        "NULLIF(`source_item_name`, '')",
        "NULLIF(`source_for_humans`, '')",
        "CAST(`source_removed` AS UNSIGNED)",
        "CAST(`source_added` AS UNSIGNED)",
        "NULLIF(`DoPetAddDrop_raw`, '')",
        "NULLIF(`DoSechiAddDrop_raw`, '')",
        "NULLIF(`SelectRate_0_raw`, '')",
        "NULLIF(`MinCount_0_raw`, '')",
        "NULLIF(`MaxCount_0_raw`, '')",
        "NULLIF(`SelectRate_1_raw`, '')",
        "NULLIF(`MinCount_1_raw`, '')",
        "NULLIF(`MaxCount_1_raw`, '')",
        "NULLIF(`SelectRate_2_raw`, '')",
        "NULLIF(`MinCount_2_raw`, '')",
        "NULLIF(`MaxCount_2_raw`, '')",
        "NULLIF(`IntimacyVariation_raw`, '')",
        "NULLIF(`ExplorationPoint_raw`, '')",
        "NULLIF(`ApplyRandomPrice_raw`, '')",
        "NULLIF(`RentTime_raw`, '')",
        "NULLIF(`PriceOption_raw`, '')",
    ]
}

fn prepare_community_subgroup_overlay_import_table(dolt_repo: &Path) -> Result<()> {
    run_dolt_sql_query_or_remote(
        dolt_repo,
        "DROP TABLE IF EXISTS `community_item_sub_group_overlay_import`;\
         CREATE TABLE `community_item_sub_group_overlay_import` (\
            `source_id` VARCHAR(64) NULL,\
            `source_label` VARCHAR(255) NULL,\
            `source_sha256` CHAR(64) NULL,\
            `source_sheet` VARCHAR(128) NULL,\
            `source_row` VARCHAR(32) NULL,\
            `source_spotted_auto` VARCHAR(255) NULL,\
            `source_comment` TEXT NULL,\
            `source_table` VARCHAR(64) NULL,\
            `source_grade` VARCHAR(16) NULL,\
            `source_item_name` VARCHAR(255) NULL,\
            `source_for_humans` VARCHAR(255) NULL,\
            `source_removed` VARCHAR(8) NULL,\
            `source_added` VARCHAR(8) NULL,\
            `ItemSubGroupKey` VARCHAR(32) NULL,\
            `ItemKey` VARCHAR(32) NULL,\
            `EnchantLevel` VARCHAR(32) NULL,\
            `DoPetAddDrop` VARCHAR(32) NULL,\
            `DoSechiAddDrop` VARCHAR(32) NULL,\
            `SelectRate_0` VARCHAR(32) NULL,\
            `MinCount_0` VARCHAR(32) NULL,\
            `MaxCount_0` VARCHAR(32) NULL,\
            `SelectRate_1` VARCHAR(32) NULL,\
            `MinCount_1` VARCHAR(32) NULL,\
            `MaxCount_1` VARCHAR(32) NULL,\
            `SelectRate_2` VARCHAR(32) NULL,\
            `MinCount_2` VARCHAR(32) NULL,\
            `MaxCount_2` VARCHAR(32) NULL,\
            `IntimacyVariation` VARCHAR(32) NULL,\
            `ExplorationPoint` VARCHAR(32) NULL,\
            `ApplyRandomPrice` VARCHAR(32) NULL,\
            `RentTime` VARCHAR(32) NULL,\
            `PriceOption` VARCHAR(32) NULL\
         ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;",
        "prepare community item subgroup overlay import table",
    )
}

fn prepare_community_subgroup_unresolved_import_table(dolt_repo: &Path) -> Result<()> {
    run_dolt_sql_query_or_remote(
        dolt_repo,
        "DROP TABLE IF EXISTS `community_item_sub_group_unresolved_overlay_import`;\
         CREATE TABLE `community_item_sub_group_unresolved_overlay_import` (\
            `source_id` VARCHAR(64) NULL,\
            `source_label` VARCHAR(255) NULL,\
            `source_sha256` CHAR(64) NULL,\
            `source_sheet` VARCHAR(128) NULL,\
            `source_row` VARCHAR(32) NULL,\
            `source_reason` VARCHAR(64) NULL,\
            `source_item_sub_group_key_raw` VARCHAR(255) NULL,\
            `source_item_key_raw` VARCHAR(255) NULL,\
            `source_enchant_level_raw` VARCHAR(255) NULL,\
            `source_spotted_auto` VARCHAR(255) NULL,\
            `source_comment` TEXT NULL,\
            `source_table` VARCHAR(64) NULL,\
            `source_grade` VARCHAR(16) NULL,\
            `source_item_name` VARCHAR(255) NULL,\
            `source_for_humans` VARCHAR(255) NULL,\
            `source_removed` VARCHAR(8) NULL,\
            `source_added` VARCHAR(8) NULL,\
            `DoPetAddDrop_raw` VARCHAR(255) NULL,\
            `DoSechiAddDrop_raw` VARCHAR(255) NULL,\
            `SelectRate_0_raw` VARCHAR(255) NULL,\
            `MinCount_0_raw` VARCHAR(255) NULL,\
            `MaxCount_0_raw` VARCHAR(255) NULL,\
            `SelectRate_1_raw` VARCHAR(255) NULL,\
            `MinCount_1_raw` VARCHAR(255) NULL,\
            `MaxCount_1_raw` VARCHAR(255) NULL,\
            `SelectRate_2_raw` VARCHAR(255) NULL,\
            `MinCount_2_raw` VARCHAR(255) NULL,\
            `MaxCount_2_raw` VARCHAR(255) NULL,\
            `IntimacyVariation_raw` VARCHAR(255) NULL,\
            `ExplorationPoint_raw` VARCHAR(255) NULL,\
            `ApplyRandomPrice_raw` VARCHAR(255) NULL,\
            `RentTime_raw` VARCHAR(255) NULL,\
            `PriceOption_raw` VARCHAR(255) NULL\
         ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;",
        "prepare community item subgroup unresolved overlay import table",
    )
}

fn ensure_community_subgroup_overlay_table(dolt_repo: &Path) -> Result<()> {
    run_dolt_sql_query_or_remote(
        dolt_repo,
        "CREATE TABLE IF NOT EXISTS `community_item_sub_group_overlay` (\
            `source_id` VARCHAR(64) NOT NULL,\
            `source_label` VARCHAR(255) NOT NULL,\
            `source_sha256` CHAR(64) NOT NULL,\
            `source_sheet` VARCHAR(128) NOT NULL,\
            `source_row` INT UNSIGNED NOT NULL,\
            `source_spotted_auto` VARCHAR(255) NULL,\
            `source_comment` TEXT NULL,\
            `source_table` VARCHAR(64) NULL,\
            `source_grade` VARCHAR(16) NULL,\
            `source_item_name` VARCHAR(255) NULL,\
            `source_for_humans` VARCHAR(255) NULL,\
            `source_removed` TINYINT NOT NULL DEFAULT 0,\
            `source_added` TINYINT NOT NULL DEFAULT 0,\
            `ItemSubGroupKey` BIGINT NOT NULL,\
            `ItemKey` BIGINT NOT NULL,\
            `EnchantLevel` INT NOT NULL,\
            `DoPetAddDrop` TINYINT NULL,\
            `DoSechiAddDrop` TINYINT NULL,\
            `SelectRate_0` BIGINT NULL,\
            `MinCount_0` INT NULL,\
            `MaxCount_0` INT NULL,\
            `SelectRate_1` BIGINT NULL,\
            `MinCount_1` INT NULL,\
            `MaxCount_1` INT NULL,\
            `SelectRate_2` BIGINT NULL,\
            `MinCount_2` INT NULL,\
            `MaxCount_2` INT NULL,\
            `IntimacyVariation` INT NULL,\
            `ExplorationPoint` INT NULL,\
            `ApplyRandomPrice` TINYINT NULL,\
            `RentTime` INT NULL,\
            `PriceOption` INT NULL,\
            PRIMARY KEY (`source_id`, `ItemSubGroupKey`, `ItemKey`, `EnchantLevel`),\
            KEY `idx_community_subgroup_overlay_key` (`ItemSubGroupKey`, `ItemKey`, `EnchantLevel`),\
            KEY `idx_community_subgroup_overlay_source_flags` (`source_id`, `source_removed`, `source_added`)\
         ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;",
        "ensure community item subgroup overlay table",
    )
}

fn ensure_community_subgroup_unresolved_table(dolt_repo: &Path) -> Result<()> {
    run_dolt_sql_query_or_remote(
        dolt_repo,
        "CREATE TABLE IF NOT EXISTS `community_item_sub_group_unresolved_overlay` (\
            `source_id` VARCHAR(64) NOT NULL,\
            `source_label` VARCHAR(255) NOT NULL,\
            `source_sha256` CHAR(64) NOT NULL,\
            `source_sheet` VARCHAR(128) NOT NULL,\
            `source_row` INT UNSIGNED NOT NULL,\
            `source_reason` VARCHAR(64) NOT NULL,\
            `source_item_sub_group_key_raw` VARCHAR(255) NULL,\
            `source_item_key_raw` VARCHAR(255) NULL,\
            `source_enchant_level_raw` VARCHAR(255) NULL,\
            `source_spotted_auto` VARCHAR(255) NULL,\
            `source_comment` TEXT NULL,\
            `source_table` VARCHAR(64) NULL,\
            `source_grade` VARCHAR(16) NULL,\
            `source_item_name` VARCHAR(255) NULL,\
            `source_for_humans` VARCHAR(255) NULL,\
            `source_removed` TINYINT NOT NULL DEFAULT 0,\
            `source_added` TINYINT NOT NULL DEFAULT 0,\
            `DoPetAddDrop_raw` VARCHAR(255) NULL,\
            `DoSechiAddDrop_raw` VARCHAR(255) NULL,\
            `SelectRate_0_raw` VARCHAR(255) NULL,\
            `MinCount_0_raw` VARCHAR(255) NULL,\
            `MaxCount_0_raw` VARCHAR(255) NULL,\
            `SelectRate_1_raw` VARCHAR(255) NULL,\
            `MinCount_1_raw` VARCHAR(255) NULL,\
            `MaxCount_1_raw` VARCHAR(255) NULL,\
            `SelectRate_2_raw` VARCHAR(255) NULL,\
            `MinCount_2_raw` VARCHAR(255) NULL,\
            `MaxCount_2_raw` VARCHAR(255) NULL,\
            `IntimacyVariation_raw` VARCHAR(255) NULL,\
            `ExplorationPoint_raw` VARCHAR(255) NULL,\
            `ApplyRandomPrice_raw` VARCHAR(255) NULL,\
            `RentTime_raw` VARCHAR(255) NULL,\
            `PriceOption_raw` VARCHAR(255) NULL,\
            PRIMARY KEY (`source_id`, `source_sheet`, `source_row`),\
            KEY `idx_community_subgroup_unresolved_reason` (`source_id`, `source_reason`)\
         ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;",
        "ensure community item subgroup unresolved overlay table",
    )
}

fn activate_community_subgroup_overlay_source(
    dolt_repo: &Path,
    source_id: &str,
    source_label: &str,
    source_sha256: &str,
) -> Result<()> {
    ensure_community_subgroup_overlay_table(dolt_repo)?;
    ensure_community_active_overlays_table(dolt_repo)?;
    let query = format!(
        "DELETE FROM {table} WHERE `overlay_kind` = {overlay_kind};\
         INSERT INTO {table} (`overlay_kind`, `source_id`, `source_label`, `source_sha256`) \
         VALUES ({overlay_kind}, {source_id}, {source_label}, {source_sha256});",
        table = sql_ident(COMMUNITY_ACTIVE_OVERLAYS_TABLE),
        overlay_kind = sql_value(COMMUNITY_SUBGROUP_OVERLAY_KIND),
        source_id = sql_value(source_id),
        source_label = sql_value(source_label),
        source_sha256 = sql_value(source_sha256),
    );
    run_dolt_sql_query_or_remote(dolt_repo, &query, "activate community subgroup overlay")?;
    replace_item_sub_group_effective_views(dolt_repo)
}

fn ensure_community_active_overlays_table(dolt_repo: &Path) -> Result<()> {
    run_dolt_sql_query_or_remote(
        dolt_repo,
        "CREATE TABLE IF NOT EXISTS `community_active_overlays` (\
            `overlay_kind` VARCHAR(64) NOT NULL,\
            `source_id` VARCHAR(64) NOT NULL,\
            `source_label` VARCHAR(255) NOT NULL,\
            `source_sha256` CHAR(64) NOT NULL,\
            PRIMARY KEY (`overlay_kind`)\
         ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;",
        "ensure community active overlays table",
    )
}

fn replace_item_sub_group_effective_views(dolt_repo: &Path) -> Result<()> {
    run_dolt_sql_query_or_remote(
        dolt_repo,
        &item_sub_group_effective_views_query(),
        "replace item subgroup effective views",
    )
}

fn item_sub_group_effective_views_query() -> String {
    "DROP VIEW IF EXISTS `item_sub_group_item_variants`;\
     DROP VIEW IF EXISTS `item_sub_group_effective_table`;\
     CREATE VIEW `item_sub_group_effective_table` AS \
     SELECT \
       base.`ItemSubGroupKey`, base.`ItemKey`, base.`EnchantLevel`, \
       base.`DoPetAddDrop`, base.`DoSechiAddDrop`, \
       base.`SelectRate_0`, base.`MinCount_0`, base.`MaxCount_0`, \
       base.`SelectRate_1`, base.`MinCount_1`, base.`MaxCount_1`, \
       base.`SelectRate_2`, base.`MinCount_2`, base.`MaxCount_2`, \
       base.`IntimacyVariation`, base.`ExplorationPoint`, \
       base.`ApplyRandomPrice`, base.`RentTime`, base.`PriceOption` \
     FROM `item_sub_group_table` base \
     LEFT JOIN `community_active_overlays` active \
       ON active.`overlay_kind` = 'item_sub_group' \
     LEFT JOIN `community_item_sub_group_overlay` overlay \
       ON overlay.`source_id` = active.`source_id` \
      AND overlay.`ItemSubGroupKey` = base.`ItemSubGroupKey` \
      AND overlay.`ItemKey` = base.`ItemKey` \
      AND overlay.`EnchantLevel` = base.`EnchantLevel` \
     WHERE overlay.`source_id` IS NULL \
     UNION ALL \
     SELECT \
       overlay.`ItemSubGroupKey`, overlay.`ItemKey`, overlay.`EnchantLevel`, \
       overlay.`DoPetAddDrop`, overlay.`DoSechiAddDrop`, \
       overlay.`SelectRate_0`, overlay.`MinCount_0`, overlay.`MaxCount_0`, \
       overlay.`SelectRate_1`, overlay.`MinCount_1`, overlay.`MaxCount_1`, \
       overlay.`SelectRate_2`, overlay.`MinCount_2`, overlay.`MaxCount_2`, \
       overlay.`IntimacyVariation`, overlay.`ExplorationPoint`, \
       overlay.`ApplyRandomPrice`, overlay.`RentTime`, overlay.`PriceOption` \
     FROM `community_item_sub_group_overlay` overlay \
     JOIN `community_active_overlays` active \
       ON active.`overlay_kind` = 'item_sub_group' \
      AND active.`source_id` = overlay.`source_id` \
     WHERE overlay.`source_removed` = 0;\
     CREATE VIEW `item_sub_group_item_variants` AS \
     SELECT ItemSubGroupKey AS item_sub_group_key, ItemKey AS item_key, EnchantLevel AS enchant_level, 0 AS variant_idx, \
            SelectRate_0 AS select_rate, MinCount_0 AS min_count, MaxCount_0 AS max_count \
     FROM item_sub_group_effective_table WHERE SelectRate_0 IS NOT NULL AND SelectRate_0 > 0 \
     UNION ALL \
     SELECT ItemSubGroupKey, ItemKey, EnchantLevel, 1, SelectRate_1, MinCount_1, MaxCount_1 \
     FROM item_sub_group_effective_table WHERE SelectRate_1 IS NOT NULL AND SelectRate_1 > 0 \
     UNION ALL \
     SELECT ItemSubGroupKey, ItemKey, EnchantLevel, 2, SelectRate_2, MinCount_2, MaxCount_2 \
     FROM item_sub_group_effective_table WHERE SelectRate_2 IS NOT NULL AND SelectRate_2 > 0;"
        .to_string()
}

fn flockfish_drop_label_to_slot_idx(value: &str) -> Option<u8> {
    match value.trim() {
        "DropID PRIZE CATCH" => Some(1),
        "DropID RARE" => Some(2),
        "DropID LARGE" => Some(3),
        "DropID GENERAL" => Some(4),
        "DropID TREASURE" => Some(5),
        _ => None,
    }
}

fn parse_flockfish_zone_group_value(value: &str) -> (Option<i64>, &'static str) {
    let normalized = normalize_import_string(value);
    if normalized.is_empty() {
        return (None, "blank");
    }
    if let Ok(parsed) = normalized.parse::<i64>() {
        if parsed > 0 {
            return (Some(parsed), "numeric");
        }
    }
    if normalized.starts_with("DUMMY") {
        return (None, "dummy");
    }
    (None, "other")
}

fn validate_headers_normalized(actual: &[String], expected: &[&str], label: &str) -> Result<()> {
    let normalized_actual = actual
        .iter()
        .map(|header| normalize_import_header(header))
        .collect::<Vec<_>>();
    let normalized_expected = expected
        .iter()
        .map(|header| normalize_import_header(header))
        .collect::<Vec<_>>();
    if normalized_actual != normalized_expected {
        bail!(
            "unexpected normalized headers in {label}. expected: [{}], got: [{}]",
            normalized_expected.join(", "),
            normalized_actual.join(", ")
        );
    }
    Ok(())
}

fn normalize_import_header(value: &str) -> String {
    value.trim().trim_start_matches('%').to_string()
}

fn normalize_import_string(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() || is_null_marker(trimmed) {
        String::new()
    } else {
        trimmed.to_string()
    }
}

fn normalize_flockfish_numeric_literal(value: &str) -> String {
    let trimmed = normalize_import_string(value);
    if trimmed.is_empty() {
        return trimmed;
    }
    if trimmed
        .chars()
        .all(|ch| ch.is_ascii_digit() || matches!(ch, '_' | '-' | '.'))
    {
        trimmed.replace('_', "")
    } else {
        trimmed
    }
}

fn resolve_calculator_effect_workbooks(excel_dir: &Path) -> Result<CalculatorEffectsWorkbookSet> {
    Ok(CalculatorEffectsWorkbookSet {
        buff_table_xlsx: resolve_required_workbook(excel_dir, "Buff_Table.xlsx")?,
        common_stat_data_xlsx: resolve_required_workbook(excel_dir, "CommonStatData.xlsx")?,
        fishing_stat_data_xlsx: resolve_required_workbook(excel_dir, "FishingStatData.xlsx")?,
        skill_table_new_xlsx: resolve_required_workbook(excel_dir, "Skill_Table_New.xlsx")?,
        skilltype_table_new_xlsx: resolve_required_workbook(excel_dir, "SkillType_Table_New.xlsx")?,
        lightstone_set_option_xlsx: resolve_required_workbook(
            excel_dir,
            "LightStoneSetOption.xlsx",
        )?,
        translate_stat_xlsx: resolve_required_workbook(excel_dir, "TranslateStat.xlsx")?,
        enchant_cash_xlsx: resolve_required_workbook(excel_dir, "Enchant_Cash.xlsx")?,
        enchant_equipment_xlsx: resolve_required_workbook(excel_dir, "Enchant_Equipment.xlsx")?,
        enchant_lifeequipment_xlsx: resolve_required_workbook(
            excel_dir,
            "Enchant_LifeEquipment.xlsx",
        )?,
        tooltip_table_xlsx: resolve_required_workbook(excel_dir, "Tooltip_Table.xlsx")?,
        producttool_property_xlsx: resolve_required_workbook(
            excel_dir,
            "ProductTool_Property.xlsx",
        )?,
        pet_table_xlsx: resolve_required_workbook(excel_dir, "Pet_Table.xlsx")?,
        pet_skill_table_xlsx: resolve_required_workbook(excel_dir, "Pet_Skill_Table.xlsx")?,
        pet_base_skill_table_xlsx: resolve_required_workbook(
            excel_dir,
            "Pet_BaseSkill_Table.xlsx",
        )?,
        pet_setstats_table_xlsx: resolve_required_workbook(excel_dir, "Pet_SetStats_Table.xlsx")?,
        pet_equipskill_table_xlsx: resolve_required_workbook(
            excel_dir,
            "Pet_EquipSkill_Table.xlsx",
        )?,
        pet_equipskill_aquire_table_xlsx: resolve_required_workbook(
            excel_dir,
            "Pet_EquipSkill_Aquire_Table.xlsx",
        )?,
        pet_grade_table_xlsx: resolve_required_workbook(excel_dir, "Pet_Grade_Table.xlsx")?,
        pet_exp_table_xlsx: resolve_required_workbook(excel_dir, "Pet_Exp_Table.xlsx")?,
        upgradepet_looting_percent_xlsx: resolve_required_workbook(
            excel_dir,
            "UpgradePet_Looting_Percent.xlsx",
        )?,
    })
}

fn resolve_calculator_progression_workbooks(
    excel_dir: &Path,
) -> Result<CalculatorProgressionWorkbookSet> {
    Ok(CalculatorProgressionWorkbookSet {
        common_stat_data_xlsx: resolve_required_workbook(excel_dir, "CommonStatData.xlsx")?,
        fishing_stat_data_xlsx: resolve_required_workbook(excel_dir, "FishingStatData.xlsx")?,
        translate_stat_xlsx: resolve_required_workbook(excel_dir, "TranslateStat.xlsx")?,
    })
}

fn resolve_required_workbook(base_dir: &Path, filename: &str) -> Result<PathBuf> {
    let path = base_dir.join(filename);
    if path.is_file() {
        Ok(path)
    } else {
        bail!("required workbook missing: {}", path.display());
    }
}

fn default_output_dir() -> Result<PathBuf> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system time before unix epoch")?
        .as_secs();
    Ok(std::env::temp_dir().join(format!("fishystuff-import-{seconds}")))
}

fn import_fishing_table(path: &Path, output_csv: &Path) -> Result<FishingImport> {
    let range = read_sheet(path, "Fishing_Table")?;
    let headers = read_headers(&range)?;
    validate_headers(
        &headers,
        &FISHING_HEADERS,
        &format!("{}:Fishing_Table", path.display()),
    )?;

    let mut writer = build_csv_writer(output_csv)?;
    writer.write_record(FISHING_HEADERS)?;

    let mut mg_keys = BTreeSet::new();
    let row_count = process_fishing_rows(range.rows().skip(1), &mut writer, &mut mg_keys)?;

    writer.flush()?;
    Ok(FishingImport { row_count, mg_keys })
}

fn import_main_group_table(
    path: &Path,
    output_csv: &Path,
    subset: SubsetMode,
    mg_keys: &BTreeSet<i64>,
) -> Result<MainGroupImport> {
    let range = read_sheet(path, "ItemMainGroup_Table")?;
    let headers = read_headers(&range)?;
    validate_headers(
        &headers,
        &MAIN_GROUP_HEADERS,
        &format!("{}:ItemMainGroup_Table", path.display()),
    )?;

    let mut writer = build_csv_writer(output_csv)?;
    writer.write_record(MAIN_GROUP_HEADERS)?;

    let mut sg_keys = BTreeSet::new();
    let mut matched_mg = BTreeSet::new();
    let row_count = process_main_group_rows(
        range.rows().skip(1),
        &mut writer,
        subset,
        mg_keys,
        &mut sg_keys,
        &mut matched_mg,
    )?;

    writer.flush()?;
    Ok(MainGroupImport {
        row_count,
        sg_keys,
        matched_mg,
    })
}

fn import_sub_group_table(
    path: &Path,
    output_csv: &Path,
    subset: SubsetMode,
    sg_keys: &BTreeSet<i64>,
) -> Result<SubGroupImport> {
    let range = read_sheet(path, "ItemSubGroup_Table")?;
    let headers = read_headers(&range)?;
    validate_headers(
        &headers,
        &SUB_GROUP_HEADERS,
        &format!("{}:ItemSubGroup_Table", path.display()),
    )?;

    let mut writer = build_csv_writer(output_csv)?;
    writer.write_record(SUB_GROUP_HEADERS)?;

    let mut matched_sg = BTreeSet::new();
    let row_count = process_sub_group_rows(
        range.rows().skip(1),
        &mut writer,
        subset,
        sg_keys,
        &mut matched_sg,
    )?;

    writer.flush()?;
    Ok(SubGroupImport {
        row_count,
        matched_sg,
    })
}

fn import_item_table(path: &Path, output_csv: &Path) -> Result<ItemTableImport> {
    let range = read_sheet(path, "Item_Table")?;
    let headers = read_headers(&range)?;
    validate_headers(
        &headers,
        &ITEM_TABLE_HEADERS,
        &format!("{}:Item_Table", path.display()),
    )?;

    let mut writer = build_csv_writer(output_csv)?;
    writer.write_record(ITEM_TABLE_HEADERS)?;

    let mut row_count = 0;
    for row in range.rows().skip(1) {
        if row_is_empty(row) {
            continue;
        }
        let record = build_record(row, ITEM_TABLE_HEADERS.len())?;
        writer.write_record(&record)?;
        row_count += 1;
    }

    writer.flush()?;
    Ok(ItemTableImport { row_count })
}

fn import_workbook_sheet(
    path: &Path,
    sheet_name: &str,
    headers: &[&str],
    output_csv: &Path,
) -> Result<RawTableImport> {
    let range = read_sheet(path, sheet_name)?;
    let actual_headers = read_headers(&range)?;
    validate_headers(
        &actual_headers,
        headers,
        &format!("{}:{sheet_name}", path.display()),
    )?;

    let mut writer = build_csv_writer(output_csv)?;
    writer.write_record(headers)?;

    let mut row_count = 0usize;
    for row in range.rows().skip(1) {
        if row_is_empty(row) {
            continue;
        }
        let record = build_record(row, headers.len())?;
        writer.write_record(&record)?;
        row_count += 1;
    }

    writer.flush()?;
    Ok(RawTableImport { row_count })
}

fn import_fish_table_csv(path: &Path, output_csv: &Path) -> Result<FishTableImport> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_path(path)
        .with_context(|| format!("open fish table csv: {}", path.display()))?;
    let mut writer = build_csv_writer(output_csv)?;
    writer.write_record(FISH_TABLE_HEADERS)?;
    let mut row_count = 0usize;
    for row in reader.records() {
        let record = row.context("read fish table csv row")?;
        if record.iter().all(|v| v.trim().is_empty()) {
            continue;
        }
        let mut cols = Vec::with_capacity(FISH_TABLE_HEADERS.len());
        for idx in 0..FISH_TABLE_HEADERS.len() {
            cols.push(record.get(idx).unwrap_or("").trim().to_string());
        }
        writer.write_record(&cols)?;
        row_count += 1;
    }
    writer.flush()?;
    Ok(FishTableImport { row_count })
}

fn import_patches_csv(path: &Path, output_csv: &Path) -> Result<PatchesImport> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(path)
        .with_context(|| format!("open patches csv: {}", path.display()))?;
    let headers = reader
        .headers()
        .context("read patches csv headers")?
        .clone();
    validate_headers(
        &headers.iter().map(|h| h.to_string()).collect::<Vec<_>>(),
        &PATCHES_HEADERS,
        &format!("{}:patches", path.display()),
    )?;

    let mut writer = build_csv_writer(output_csv)?;
    writer.write_record(PATCHES_HEADERS)?;

    let mut row_count = 0;
    for row in reader.records() {
        let record = row.context("read patches csv row")?;
        let mut out = Vec::with_capacity(PATCHES_HEADERS.len());
        for i in 0..PATCHES_HEADERS.len() {
            let raw = record.get(i).unwrap_or("").trim();
            if raw.is_empty() || is_null_marker(raw) {
                out.push(String::new());
            } else {
                out.push(raw.to_string());
            }
        }
        writer.write_record(&out)?;
        row_count += 1;
    }

    writer.flush()?;
    Ok(PatchesImport { row_count })
}

fn import_languagedata_csv(
    path: &Path,
    output_csv: &Path,
    lang: &str,
) -> Result<LanguageDataImport> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(path)
        .with_context(|| format!("open languagedata csv: {}", path.display()))?;
    let headers = reader
        .headers()
        .context("read languagedata csv headers")?
        .clone();
    validate_headers(
        &headers.iter().map(|h| h.to_string()).collect::<Vec<_>>(),
        &LANGUAGEDATA_SOURCE_HEADERS,
        &format!("{}:languagedata_{lang}", path.display()),
    )?;

    let mut writer = build_csv_writer(output_csv)?;
    writer.write_record(LANGUAGEDATA_HEADERS)?;

    let mut row_count = 0;
    for row in reader.records() {
        let record = row.context("read languagedata csv row")?;
        let mut source = Vec::with_capacity(LANGUAGEDATA_SOURCE_HEADERS.len());
        for i in 0..LANGUAGEDATA_SOURCE_HEADERS.len() {
            let raw = record.get(i).unwrap_or("").trim();
            if raw.is_empty() || is_null_marker(raw) {
                source.push(String::new());
            } else {
                source.push(raw.to_string());
            }
        }
        writer.write_record([
            lang,
            source.first().map(String::as_str).unwrap_or_default(),
            source.get(1).map(String::as_str).unwrap_or_default(),
            source.get(2).map(String::as_str).unwrap_or_default(),
            source.get(3).map(String::as_str).unwrap_or_default(),
        ])?;
        row_count += 1;
    }

    writer.flush()?;
    Ok(LanguageDataImport { row_count })
}

fn import_languagedata_loc(
    path: &Path,
    output_csv: &Path,
    lang: &str,
) -> Result<LanguageDataImport> {
    let mut writer = build_csv_writer(output_csv)?;
    writer.write_record(LANGUAGEDATA_HEADERS)?;

    let mut row_count = 0usize;
    scan_loc_records(path, 100_000, |record| {
        let id = record.key.to_string();
        let unk = record
            .namespace
            .map(|namespace| namespace.to_string())
            .unwrap_or_default();
        writer.write_record([
            lang,
            id.as_str(),
            unk.as_str(),
            record.text.as_str(),
            record.format.as_str(),
        ])?;
        row_count += 1;
        Ok(())
    })
    .with_context(|| format!("decode languagedata loc: {}", path.display()))?;

    writer.flush()?;
    Ok(LanguageDataImport { row_count })
}

fn append_community_prize_guess_rows(
    _dolt_repo: &Path,
    workbook_xlsx: &Path,
    workbook_sha: &str,
    output_csv: &Path,
) -> Result<CommunityPrizeGuessImport> {
    let spot_lookup = load_setup_spot_lookup(workbook_xlsx)?;
    let range = read_sheet(workbook_xlsx, "New Prize Fish Info")?;
    let rows = range.rows().collect::<Vec<_>>();
    if rows.is_empty() {
        bail!(
            "{}:New Prize Fish Info has no rows",
            workbook_xlsx.display()
        );
    }
    validate_community_prize_guess_headers(rows[0], workbook_xlsx)?;

    let mut aggregate = BTreeMap::<(u32, i64), Vec<String>>::new();
    let mut resolved_item_keys = 0usize;
    let matched_names = 0usize;
    let mut unresolved_names = 0usize;
    let mut unresolved_zones = 0usize;
    let mut subgroup_mapped_rows = 0usize;

    for row in rows.into_iter().skip(1) {
        if row_is_empty(row) {
            continue;
        }

        let Some(zone_name) = cell_to_string_opt(row.get(SETUP_NEW_PRIZE_ZONE_COL))? else {
            continue;
        };
        let Some(guessed_rate) = cell_to_f64_opt(row.get(SETUP_NEW_PRIZE_CHANCE_COL))? else {
            continue;
        };
        if guessed_rate <= 0.0 {
            continue;
        }

        let Some((zone_rgb, zone_r, zone_g, zone_b, subgroup_key)) = spot_lookup.get(&zone_name)
        else {
            unresolved_zones += 1;
            continue;
        };

        let preferred_name = cell_to_string_opt(row.get(SETUP_NEW_PRIZE_TITLE_COL))?
            .or(cell_to_string_opt(row.get(SETUP_NEW_PRIZE_FISH_COL))?)
            .unwrap_or_default();
        if preferred_name.is_empty() {
            unresolved_names += 1;
            continue;
        }

        let Some(item_id) = cell_to_i64_opt(row.get(SETUP_NEW_PRIZE_ITEM_KEY_COL))?
            .or(cell_to_i64_opt(row.get(SETUP_NEW_PRIZE_ID_COL))?)
            .filter(|value| *value > 0)
        else {
            unresolved_names += 1;
            continue;
        };
        resolved_item_keys += 1;

        if subgroup_key.is_some() {
            subgroup_mapped_rows += 1;
        }

        let fish_name = preferred_name;
        let notes = format_community_prize_guess_notes(1, guessed_rate, *subgroup_key);
        let record = vec![
            COMMUNITY_PRIZE_GUESS_SOURCE_ID.to_string(),
            COMMUNITY_PRIZE_GUESS_SOURCE_LABEL.to_string(),
            workbook_sha.to_string(),
            zone_rgb.to_string(),
            zone_r.to_string(),
            zone_g.to_string(),
            zone_b.to_string(),
            derive_region_name_from_zone_name(&zone_name),
            zone_name.clone(),
            item_id.to_string(),
            fish_name,
            "guessed".to_string(),
            "0".to_string(),
            notes,
        ];

        let key = (*zone_rgb, item_id);
        if let Some(existing) = aggregate.insert(key, record.clone()) {
            if existing != record {
                bail!(
                    "conflicting guessed prize rows for zone_rgb={} item_id={item_id}",
                    zone_rgb
                );
            }
        }
    }

    let output = OpenOptions::new()
        .append(true)
        .open(output_csv)
        .with_context(|| format!("append community csv: {}", output_csv.display()))?;
    let mut writer = WriterBuilder::new()
        .has_headers(false)
        .quote_style(QuoteStyle::Necessary)
        .from_writer(output);
    for row in aggregate.values() {
        writer.write_record(row)?;
    }
    writer.flush()?;

    Ok(CommunityPrizeGuessImport {
        emitted_rows: aggregate.len(),
        resolved_item_keys,
        matched_names,
        unresolved_names,
        unresolved_zones,
        subgroup_mapped_rows,
    })
}

fn load_setup_spot_lookup(
    workbook_xlsx: &Path,
) -> Result<HashMap<String, (u32, u8, u8, u8, Option<i64>)>> {
    let range = read_sheet(workbook_xlsx, "Spot Info")?;
    let rows = range.rows().collect::<Vec<_>>();
    if rows.is_empty() {
        bail!("{}:Spot Info has no rows", workbook_xlsx.display());
    }

    let mut lookup = HashMap::new();
    for row in rows.into_iter().skip(1) {
        if row_is_empty(row) {
            continue;
        }
        let Some(zone_name) = cell_to_string_opt(row.get(SETUP_SPOT_NAME_COL))? else {
            continue;
        };
        let Some(zone_r_i64) = cell_to_i64_opt(row.get(SETUP_SPOT_R_COL))? else {
            continue;
        };
        let Some(zone_g_i64) = cell_to_i64_opt(row.get(SETUP_SPOT_G_COL))? else {
            continue;
        };
        let Some(zone_b_i64) = cell_to_i64_opt(row.get(SETUP_SPOT_B_COL))? else {
            continue;
        };
        let zone_r = u8::try_from(zone_r_i64)
            .with_context(|| format!("zone R out of range: {zone_r_i64}"))?;
        let zone_g = u8::try_from(zone_g_i64)
            .with_context(|| format!("zone G out of range: {zone_g_i64}"))?;
        let zone_b = u8::try_from(zone_b_i64)
            .with_context(|| format!("zone B out of range: {zone_b_i64}"))?;
        let zone_rgb = (u32::from(zone_r) << 16) | (u32::from(zone_g) << 8) | u32::from(zone_b);
        let prize_subgroup = cell_to_i64_opt(row.get(SETUP_SPOT_PRIZE_SUBGROUP_COL))?;

        lookup.insert(
            zone_name,
            (zone_rgb, zone_r, zone_g, zone_b, prize_subgroup),
        );
    }
    Ok(lookup)
}

fn validate_community_prize_guess_headers(row: &[Data], workbook_xlsx: &Path) -> Result<()> {
    let headers: Vec<String> = row.iter().map(header_cell_to_string).collect();
    let expected = [
        (SETUP_NEW_PRIZE_TITLE_COL, "Title"),
        (SETUP_NEW_PRIZE_ZONE_COL, "Fishing Zone"),
        (SETUP_NEW_PRIZE_ITEM_KEY_COL, "%ItemKey"),
        (SETUP_NEW_PRIZE_FISH_COL, "Fish"),
        (SETUP_NEW_PRIZE_CHANCE_COL, "Chance Guess"),
    ];
    for (idx, expected_value) in expected {
        let actual = headers.get(idx).map(|value| value.trim()).unwrap_or("");
        if actual != expected_value {
            bail!(
                "unexpected community guessed-rate workbook headers in {}:New Prize Fish Info at column {}. expected '{}' got '{}'",
                workbook_xlsx.display(),
                idx,
                expected_value,
                actual
            );
        }
    }
    Ok(())
}

fn derive_region_name_from_zone_name(zone_name: &str) -> String {
    zone_name
        .split(" - ")
        .next()
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn format_community_prize_guess_notes(
    slot_idx: u8,
    guessed_rate: f64,
    subgroup_key: Option<i64>,
) -> String {
    match subgroup_key {
        Some(subgroup_key) => format!(
            "slot_idx={slot_idx};guessed_rate={};subgroup_key={subgroup_key}",
            format_float(guessed_rate)
        ),
        None => format!(
            "slot_idx={slot_idx};guessed_rate={}",
            format_float(guessed_rate)
        ),
    }
}

fn process_fishing_rows<'a, W, I>(
    rows: I,
    writer: &mut Writer<W>,
    mg_keys: &mut BTreeSet<i64>,
) -> Result<usize>
where
    W: Write,
    I: Iterator<Item = &'a [Data]>,
{
    let mut row_count = 0;
    for row in rows {
        if row_is_empty(row) {
            continue;
        }
        let record = build_record(row, FISHING_HEADERS.len())?;
        writer.write_record(&record)?;
        row_count += 1;

        for &idx in &FISHING_MG_COLS {
            if let Some(value) = cell_to_i64_opt(row.get(idx))? {
                if value > 0 {
                    mg_keys.insert(value);
                }
            }
        }
    }
    Ok(row_count)
}

fn process_main_group_rows<'a, W, I>(
    rows: I,
    writer: &mut Writer<W>,
    subset: SubsetMode,
    mg_keys: &BTreeSet<i64>,
    sg_keys: &mut BTreeSet<i64>,
    matched_mg: &mut BTreeSet<i64>,
) -> Result<usize>
where
    W: Write,
    I: Iterator<Item = &'a [Data]>,
{
    let mut row_count = 0;
    for row in rows {
        if row_is_empty(row) {
            continue;
        }
        let key = match cell_to_i64_opt(row.get(MAIN_GROUP_KEY_COL))? {
            Some(value) if value > 0 => value,
            _ => continue,
        };

        if mg_keys.contains(&key) {
            matched_mg.insert(key);
        }

        if subset == SubsetMode::FishingOnly && !mg_keys.contains(&key) {
            continue;
        }

        let record = build_record(row, MAIN_GROUP_HEADERS.len())?;
        writer.write_record(&record)?;
        row_count += 1;

        for &idx in &MAIN_GROUP_SG_COLS {
            if let Some(value) = cell_to_i64_opt(row.get(idx))? {
                if value > 0 {
                    sg_keys.insert(value);
                }
            }
        }
    }
    Ok(row_count)
}

fn process_sub_group_rows<'a, W, I>(
    rows: I,
    writer: &mut Writer<W>,
    subset: SubsetMode,
    sg_keys: &BTreeSet<i64>,
    matched_sg: &mut BTreeSet<i64>,
) -> Result<usize>
where
    W: Write,
    I: Iterator<Item = &'a [Data]>,
{
    let mut row_count = 0;
    for row in rows {
        if row_is_empty(row) {
            continue;
        }
        let key = match cell_to_i64_opt(row.get(SUB_GROUP_KEY_COL))? {
            Some(value) if value > 0 => value,
            _ => continue,
        };

        if sg_keys.contains(&key) {
            matched_sg.insert(key);
        }

        if subset == SubsetMode::FishingOnly && !sg_keys.contains(&key) {
            continue;
        }

        let record = build_record(row, SUB_GROUP_HEADERS.len())?;
        writer.write_record(&record)?;
        row_count += 1;
    }
    Ok(row_count)
}

fn build_record(row: &[Data], expected_len: usize) -> Result<Vec<String>> {
    let mut record = Vec::with_capacity(expected_len);
    for idx in 0..expected_len {
        let value = cell_to_string_opt(row.get(idx))?.unwrap_or_default();
        record.push(value);
    }
    Ok(record)
}

fn read_sheet(path: &Path, sheet_name: &str) -> Result<Range<Data>> {
    let mut workbook =
        open_workbook_auto(path).with_context(|| format!("open workbook: {}", path.display()))?;
    let range = workbook
        .worksheet_range(sheet_name)
        .with_context(|| format!("read sheet '{}' in {}", sheet_name, path.display()))?;
    Ok(range)
}

fn read_headers(range: &Range<Data>) -> Result<Vec<String>> {
    let mut rows = range.rows();
    let Some(row) = rows.next() else {
        bail!("sheet has no rows");
    };
    let mut headers: Vec<String> = row.iter().map(header_cell_to_string).collect();
    while headers.last().map(|h| h.trim().is_empty()).unwrap_or(false) {
        headers.pop();
    }
    Ok(headers)
}

fn validate_headers(actual: &[String], expected: &[&str], label: &str) -> Result<()> {
    let trimmed: Vec<String> = actual.iter().map(|h| h.trim().to_string()).collect();
    let expected_vec: Vec<String> = expected.iter().map(|h| h.to_string()).collect();
    if trimmed != expected_vec {
        bail!(
            "unexpected headers in {label}. expected: [{}], got: [{}]",
            expected_vec.join(", "),
            trimmed.join(", ")
        );
    }
    Ok(())
}

fn header_cell_to_string(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(value) => value.trim().to_string(),
        _ => cell.to_string().trim().to_string(),
    }
}

fn row_is_empty(row: &[Data]) -> bool {
    row.iter().all(|cell| match cell {
        Data::Empty => true,
        Data::String(value) => {
            let trimmed = value.trim();
            trimmed.is_empty() || is_null_marker(trimmed)
        }
        Data::DateTimeIso(value) | Data::DurationIso(value) => {
            let trimmed = value.trim();
            trimmed.is_empty() || is_null_marker(trimmed)
        }
        _ => false,
    })
}

fn cell_to_string_opt(cell: Option<&Data>) -> Result<Option<String>> {
    match cell {
        Some(cell) => cell_to_string(cell),
        None => Ok(None),
    }
}

fn cell_to_string(cell: &Data) -> Result<Option<String>> {
    match cell {
        Data::Empty => Ok(None),
        Data::String(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() || is_null_marker(trimmed) {
                Ok(None)
            } else {
                Ok(Some(trimmed.to_string()))
            }
        }
        Data::DateTimeIso(value) | Data::DurationIso(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() || is_null_marker(trimmed) {
                Ok(None)
            } else {
                Ok(Some(trimmed.to_string()))
            }
        }
        Data::Float(value) => Ok(Some(format_float(*value))),
        Data::Int(value) => Ok(Some(value.to_string())),
        Data::Bool(value) => Ok(Some(if *value { "1" } else { "0" }.to_string())),
        Data::DateTime(value) => Ok(Some(format_float(value.as_f64()))),
        Data::Error(err) => bail!("cell error: {err:?}"),
    }
}

fn cell_to_i64_opt(cell: Option<&Data>) -> Result<Option<i64>> {
    match cell {
        Some(cell) => cell_to_i64(cell),
        None => Ok(None),
    }
}

fn cell_to_f64_opt(cell: Option<&Data>) -> Result<Option<f64>> {
    match cell {
        Some(cell) => cell_to_f64(cell),
        None => Ok(None),
    }
}

fn cell_to_i64(cell: &Data) -> Result<Option<i64>> {
    match cell {
        Data::Empty => Ok(None),
        Data::String(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() || is_null_marker(trimmed) {
                return Ok(None);
            }
            let parsed = trimmed
                .parse::<i64>()
                .with_context(|| format!("parse int: {trimmed}"))?;
            Ok(Some(parsed))
        }
        Data::DateTimeIso(value) | Data::DurationIso(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() || is_null_marker(trimmed) {
                return Ok(None);
            }
            let parsed = trimmed
                .parse::<i64>()
                .with_context(|| format!("parse int: {trimmed}"))?;
            Ok(Some(parsed))
        }
        Data::Float(value) => {
            if value.fract() == 0.0 {
                Ok(Some(*value as i64))
            } else {
                Ok(None)
            }
        }
        Data::Int(value) => Ok(Some(*value)),
        Data::Bool(value) => Ok(Some(if *value { 1 } else { 0 })),
        Data::DateTime(value) => {
            let raw = value.as_f64();
            if raw.fract() == 0.0 {
                Ok(Some(raw as i64))
            } else {
                Ok(None)
            }
        }
        Data::Error(err) => bail!("cell error: {err:?}"),
    }
}

fn cell_to_f64(cell: &Data) -> Result<Option<f64>> {
    match cell {
        Data::Empty => Ok(None),
        Data::String(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() || is_null_marker(trimmed) {
                return Ok(None);
            }
            let parsed = trimmed
                .parse::<f64>()
                .with_context(|| format!("parse float: {trimmed}"))?;
            Ok(Some(parsed))
        }
        Data::DateTimeIso(value) | Data::DurationIso(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() || is_null_marker(trimmed) {
                return Ok(None);
            }
            let parsed = trimmed
                .parse::<f64>()
                .with_context(|| format!("parse float: {trimmed}"))?;
            Ok(Some(parsed))
        }
        Data::Float(value) => Ok(Some(*value)),
        Data::Int(value) => Ok(Some(*value as f64)),
        Data::Bool(value) => Ok(Some(if *value { 1.0 } else { 0.0 })),
        Data::DateTime(value) => Ok(Some(value.as_f64())),
        Data::Error(err) => bail!("cell error: {err:?}"),
    }
}

fn is_null_marker(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower == "null" || lower == "<null>"
}

fn format_float(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        let mut s = format!("{value:.10}");
        while s.contains('.') && s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
        s
    }
}

fn build_csv_writer(path: &Path) -> Result<Writer<File>> {
    let file = File::create(path).with_context(|| format!("create csv: {}", path.display()))?;
    Ok(WriterBuilder::new()
        .quote_style(QuoteStyle::Necessary)
        .terminator(csv::Terminator::Any(b'\n'))
        .from_writer(file))
}

fn run_dolt_table_import(repo_path: &Path, table: &str, csv_path: &Path) -> Result<()> {
    let output = Command::new("dolt")
        .current_dir(repo_path)
        .args([
            "table",
            "import",
            "-u",
            table,
            csv_path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("invalid csv path"))?,
        ])
        .output()
        .with_context(|| format!("run dolt table import for {table}"))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    bail!("dolt table import failed for {table}: {stderr}");
}

fn run_dolt_table_replace(repo_path: &Path, table: &str, csv_path: &Path) -> Result<()> {
    let output = Command::new("dolt")
        .current_dir(repo_path)
        .args([
            "table",
            "import",
            "-r",
            table,
            csv_path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("invalid csv path"))?,
        ])
        .output()
        .with_context(|| format!("run dolt table replace import for {table}"))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    bail!("dolt table replace import failed for {table}: {stderr}");
}

fn run_dolt_table_import_or_sql_server(
    repo_path: &Path,
    table: &str,
    csv_path: &Path,
) -> Result<()> {
    match run_dolt_table_import(repo_path, table, csv_path) {
        Ok(()) => Ok(()),
        Err(err) => {
            let err_text = err.to_string();
            if !err_text.contains("database is read only") {
                return Err(err);
            }
            eprintln!(
                "local dolt table import for {table} is read-only; falling back to sql-server import"
            );
            run_dolt_remote_sql_table_import(table, csv_path)
        }
    }
}

fn run_dolt_sql_table_import(repo_path: &Path, table: &str, csv_path: &Path) -> Result<()> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(csv_path)
        .with_context(|| format!("open generated csv for {table}: {}", csv_path.display()))?;
    let headers = reader
        .headers()
        .with_context(|| format!("read generated csv headers for {table}"))?
        .iter()
        .map(|header| header.to_string())
        .collect::<Vec<_>>();

    let ensure_table_query = build_import_table_ensure_query(table, &headers)?;
    run_dolt_sql_query(
        repo_path,
        &format!("{ensure_table_query}\nDELETE FROM {};", sql_ident(table)),
        &format!("ensure and truncate {table} via delete"),
    )?;

    let mut batch = Vec::new();
    for record in reader.records() {
        let record = record.with_context(|| format!("read generated csv row for {table}"))?;
        batch.push(
            record
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>(),
        );
        if batch.len() >= 200 {
            run_dolt_insert_batch(repo_path, table, &headers, &batch)?;
            batch.clear();
        }
    }

    if !batch.is_empty() {
        run_dolt_insert_batch(repo_path, table, &headers, &batch)?;
    }

    Ok(())
}

fn run_dolt_sql_table_import_or_remote(
    repo_path: &Path,
    table: &str,
    csv_path: &Path,
) -> Result<()> {
    match run_dolt_sql_table_import(repo_path, table, csv_path) {
        Ok(()) => Ok(()),
        Err(err) => {
            let err_text = err.to_string();
            if !err_text.contains("database is read only") {
                return Err(err);
            }
            eprintln!(
                "local dolt sql import for {table} is read-only; falling back to sql-server import"
            );
            run_dolt_remote_sql_table_import(table, csv_path)
        }
    }
}

fn run_dolt_remote_sql_table_import(table: &str, csv_path: &Path) -> Result<()> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(csv_path)
        .with_context(|| format!("open generated csv for {table}: {}", csv_path.display()))?;
    let headers = reader
        .headers()
        .with_context(|| format!("read generated csv headers for {table}"))?
        .iter()
        .map(|header| header.to_string())
        .collect::<Vec<_>>();

    let ensure_table_query = build_import_table_ensure_query(table, &headers)?;
    run_dolt_remote_sql_query(
        &format!(
            "USE {};\n{}\nDELETE FROM {};",
            sql_ident(&remote_dolt_database_name()),
            ensure_table_query,
            sql_ident(table)
        ),
        &format!("ensure and truncate {table} via delete on sql-server"),
    )?;

    let mut batch = Vec::new();
    for record in reader.records() {
        let record = record.with_context(|| format!("read generated csv row for {table}"))?;
        batch.push(
            record
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>(),
        );
        if batch.len() >= 200 {
            run_dolt_remote_insert_batch(table, &headers, &batch)?;
            batch.clear();
        }
    }

    if !batch.is_empty() {
        run_dolt_remote_insert_batch(table, &headers, &batch)?;
    }

    Ok(())
}

fn build_import_table_ensure_query(table: &str, headers: &[String]) -> Result<String> {
    if headers.is_empty() {
        bail!("generated csv for {table} has no headers");
    }
    let columns = headers
        .iter()
        .map(|header| format!("{} LONGTEXT", sql_ident(header)))
        .collect::<Vec<_>>()
        .join(", ");
    Ok(format!(
        "CREATE TABLE IF NOT EXISTS {} ({}) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;",
        sql_ident(table),
        columns
    ))
}

fn run_dolt_insert_batch(
    repo_path: &Path,
    table: &str,
    headers: &[String],
    rows: &[Vec<String>],
) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }

    let columns = headers
        .iter()
        .map(|header| sql_ident(header))
        .collect::<Vec<_>>()
        .join(", ");
    let values = rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|value| sql_value(value))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .map(|joined| format!("({joined})"))
        .collect::<Vec<_>>()
        .join(",\n");
    let query = format!(
        "INSERT INTO {} ({columns}) VALUES\n{values};",
        sql_ident(table)
    );
    run_dolt_sql_query(repo_path, &query, &format!("insert batch into {table}"))
}

fn run_dolt_remote_insert_batch(
    table: &str,
    headers: &[String],
    rows: &[Vec<String>],
) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }

    let columns = headers
        .iter()
        .map(|header| sql_ident(header))
        .collect::<Vec<_>>()
        .join(", ");
    let values = rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|value| sql_value(value))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .map(|joined| format!("({joined})"))
        .collect::<Vec<_>>()
        .join(",\n");
    let query = format!(
        "USE {};\nINSERT INTO {} ({columns}) VALUES\n{values};",
        sql_ident(&remote_dolt_database_name()),
        sql_ident(table)
    );
    run_dolt_remote_sql_query(&query, &format!("insert batch into {table} on sql-server"))
}

fn run_dolt_select_named_rows(
    repo_path: &Path,
    query: &str,
    label: &str,
) -> Result<Vec<BTreeMap<String, String>>> {
    let output = Command::new("dolt")
        .current_dir(repo_path)
        .args(["sql", "-r", "csv", "-q", query])
        .output()
        .with_context(|| format!("run dolt sql select for {label}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("dolt sql select failed during {label}: {stderr}");
    }

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(output.stdout.as_slice());
    let headers = reader
        .headers()
        .with_context(|| format!("read CSV headers for {label}"))?
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>();
    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record.with_context(|| format!("read CSV row for {label}"))?;
        let row = headers
            .iter()
            .cloned()
            .zip(record.iter().map(|value| value.to_string()))
            .collect::<BTreeMap<_, _>>();
        rows.push(row);
    }
    Ok(rows)
}

fn run_dolt_sql_query_or_remote(repo_path: &Path, query: &str, label: &str) -> Result<()> {
    match run_dolt_sql_query(repo_path, query, label) {
        Ok(()) => Ok(()),
        Err(err) => {
            let err_text = err.to_string();
            if !err_text.contains("database is read only") {
                return Err(err);
            }
            eprintln!("local dolt sql for {label} is read-only; falling back to sql-server");
            let remote_query = format!("USE {};\n{query}", sql_ident(&remote_dolt_database_name()));
            run_dolt_remote_sql_query(&remote_query, &format!("{label} on sql-server"))
        }
    }
}

fn run_dolt_sql_query(repo_path: &Path, query: &str, label: &str) -> Result<()> {
    let mut child = Command::new("dolt")
        .current_dir(repo_path)
        .arg("sql")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("spawn dolt sql for {label}"))?;

    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("missing dolt sql stdin for {label}"))?;
        stdin
            .write_all(query.as_bytes())
            .with_context(|| format!("write dolt sql query for {label}"))?;
    }

    let output = child
        .wait_with_output()
        .with_context(|| format!("wait for dolt sql during {label}"))?;
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    bail!("dolt sql failed during {label}: {stderr}");
}

fn run_dolt_remote_sql_query(query: &str, label: &str) -> Result<()> {
    let mut child = Command::new("dolt")
        .args([
            "--host",
            &remote_dolt_host(),
            "--port",
            &remote_dolt_port(),
            "--no-tls",
            "sql",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("spawn remote dolt sql for {label}"))?;

    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("missing remote dolt sql stdin for {label}"))?;
        stdin
            .write_all(query.as_bytes())
            .with_context(|| format!("write remote dolt sql query for {label}"))?;
    }

    let output = child
        .wait_with_output()
        .with_context(|| format!("wait for remote dolt sql during {label}"))?;
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    bail!("remote dolt sql failed during {label}: {stderr}");
}

fn remote_dolt_host() -> String {
    std::env::var("DOLT_SQL_HOST").unwrap_or_else(|_| "127.0.0.1".to_string())
}

fn remote_dolt_port() -> String {
    std::env::var("DOLT_SQL_PORT").unwrap_or_else(|_| "3306".to_string())
}

fn remote_dolt_database_name() -> String {
    std::env::var("DOLT_DATABASE_NAME").unwrap_or_else(|_| "fishystuff".to_string())
}

fn sql_ident(value: &str) -> String {
    format!("`{}`", value.replace('`', "``"))
}

fn sql_value(value: &str) -> String {
    if value.is_empty() {
        return "NULL".to_string();
    }

    let mut out = String::with_capacity(value.len() + 2);
    out.push('\'');
    for ch in value.chars() {
        match ch {
            '\'' => out.push_str("''"),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\0' => out.push_str("\\0"),
            _ => out.push(ch),
        }
    }
    out.push('\'');
    out
}

fn run_dolt_commit(repo_path: &Path, message: &str) -> Result<()> {
    let output = Command::new("dolt")
        .current_dir(repo_path)
        .args(["add", "-A"])
        .output()
        .context("run dolt add")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("dolt add failed: {stderr}");
    }

    let output = Command::new("dolt")
        .current_dir(repo_path)
        .args(["commit", "-m", message])
        .output()
        .context("run dolt commit")?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    bail!("dolt commit failed: {stderr}");
}

fn build_commit_message(base: Option<String>, digests: &ImportDigests) -> String {
    let mut parts = vec![
        format!("Fishing_Table={}", digests.fishing_sha),
        format!("ItemMainGroup={}", digests.main_group_sha),
        format!("ItemSubGroup={}", digests.sub_group_sha),
    ];
    if let Some(item_table_sha) = digests.item_table_sha.as_deref() {
        parts.push(format!("Item_Table={item_table_sha}"));
    }
    if let Some(fish_table_sha) = digests.fish_table_sha.as_deref() {
        parts.push(format!("Fish_Table={fish_table_sha}"));
    }
    if let Some(patches_sha) = digests.patches_sha.as_deref() {
        parts.push(format!("Patches={patches_sha}"));
    }
    for (lang, languagedata_sha) in &digests.languagedata_shas {
        parts.push(format!(
            "LanguageData_{}={languagedata_sha}",
            lang.to_uppercase()
        ));
    }
    let suffix = format!("({})", parts.join(", "));
    match base {
        Some(msg) => format!("{msg} {suffix}"),
        None => format!("Import fishing-related groups from community XLSX snapshot {suffix}"),
    }
}

fn build_languagedata_loc_commit_message(
    base: Option<String>,
    loc_shas: &BTreeMap<String, String>,
) -> String {
    let parts = loc_shas
        .iter()
        .map(|(lang, sha)| format!("LanguageData_{}={sha}", lang.to_uppercase()))
        .collect::<Vec<_>>();
    let suffix = format!("({})", parts.join(", "));
    match base {
        Some(msg) => format!("{msg} {suffix}"),
        None => format!("Import languagedata loc files {suffix}"),
    }
}

fn build_calculator_effects_commit_message(
    base: Option<String>,
    digests: &CalculatorEffectsDigests,
) -> String {
    let suffix = format!(
        "(Buff_Table={}, CommonStatData={}, FishingStatData={}, Skill_Table_New={}, SkillType_Table_New={}, LightStoneSetOption={}, TranslateStat={}, Enchant_Cash={}, Enchant_Equipment={}, Enchant_LifeEquipment={}, Tooltip_Table={}, ProductTool_Property={}, Pet_Table={}, Pet_Skill_Table={}, Pet_BaseSkill_Table={}, Pet_SetStats_Table={}, Pet_EquipSkill_Table={}, Pet_EquipSkill_Aquire_Table={}, Pet_Grade_Table={}, Pet_Exp_Table={}, UpgradePet_Looting_Percent={})",
        digests.buff_table_sha,
        digests.common_stat_data_sha,
        digests.fishing_stat_data_sha,
        digests.skill_table_new_sha,
        digests.skilltype_table_new_sha,
        digests.lightstone_set_option_sha,
        digests.translate_stat_sha,
        digests.enchant_cash_sha,
        digests.enchant_equipment_sha,
        digests.enchant_lifeequipment_sha,
        digests.tooltip_table_sha,
        digests.producttool_property_sha,
        digests.pet_table_sha,
        digests.pet_skill_table_sha,
        digests.pet_base_skill_table_sha,
        digests.pet_setstats_table_sha,
        digests.pet_equipskill_table_sha,
        digests.pet_equipskill_aquire_table_sha,
        digests.pet_grade_table_sha,
        digests.pet_exp_table_sha,
        digests.upgradepet_looting_percent_sha,
    );
    match base {
        Some(msg) => format!("{msg} {suffix}"),
        None => format!("Import calculator effect workbooks {suffix}"),
    }
}

fn build_calculator_progression_commit_message(
    base: Option<String>,
    digests: &CalculatorProgressionDigests,
) -> String {
    let suffix = format!(
        "(CommonStatData={}, FishingStatData={}, TranslateStat={})",
        digests.common_stat_data_sha, digests.fishing_stat_data_sha, digests.translate_stat_sha,
    );
    match base {
        Some(msg) => format!("{msg} {suffix}"),
        None => format!("Import calculator progression workbooks {suffix}"),
    }
}

fn report_import(report: ImportReport<'_>) {
    let ImportReport {
        subset,
        fishing,
        main_group,
        sub_group,
        item_table,
        fish_table,
        patches,
        languagedata,
        outputs,
    } = report;
    let missing_mg: BTreeSet<i64> = fishing
        .mg_keys
        .difference(&main_group.matched_mg)
        .copied()
        .collect();
    let missing_sg: BTreeSet<i64> = main_group
        .sg_keys
        .difference(&sub_group.matched_sg)
        .copied()
        .collect();

    println!("fishing rows: {}", fishing.row_count);
    println!("main group keys referenced: {}", fishing.mg_keys.len());
    println!("main group rows emitted: {}", main_group.row_count);
    if subset == SubsetMode::FishingOnly && !missing_mg.is_empty() {
        println!(
            "missing main group keys: {} -> {:?}",
            missing_mg.len(),
            missing_mg
        );
    }
    println!("sub group keys referenced: {}", main_group.sg_keys.len());
    println!("sub group rows emitted: {}", sub_group.row_count);
    if subset == SubsetMode::FishingOnly && !missing_sg.is_empty() {
        println!(
            "missing sub group keys: {} -> {:?}",
            missing_sg.len(),
            missing_sg
        );
    }
    if let Some(item_table) = item_table {
        println!("item table rows emitted: {}", item_table.row_count);
    }
    if let Some(fish_table) = fish_table {
        println!("fish table rows emitted: {}", fish_table.row_count);
    }
    if let Some(patches) = patches {
        println!("patches rows emitted: {}", patches.row_count);
    }
    for (lang, languagedata) in languagedata {
        println!(
            "languagedata_{lang} rows emitted: {}",
            languagedata.row_count
        );
    }
    println!("output fishing csv: {}", outputs.fishing_csv.display());
    println!(
        "output main group csv: {}",
        outputs.main_group_csv.display()
    );
    println!("output sub group csv: {}", outputs.sub_group_csv.display());
    if item_table.is_some() {
        println!(
            "output item table csv: {}",
            outputs.item_table_csv.display()
        );
    }
    if fish_table.is_some() {
        println!(
            "output fish table csv: {}",
            outputs.fish_table_csv.display()
        );
    }
    if patches.is_some() {
        println!("output patches csv: {}", outputs.patches_csv.display());
    }
    for (lang, output) in &outputs.languagedata_csvs {
        println!("output languagedata_{lang} csv: {}", output.display());
    }
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut file = File::open(path).with_context(|| format!("open file: {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 1024 * 64];
    loop {
        let n = std::io::Read::read(&mut file, &mut buf)
            .with_context(|| format!("read file: {}", path.display()))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let digest = hasher.finalize();
    Ok(bytes_to_hex(&digest))
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(char::from_digit((b >> 4) as u32, 16).unwrap());
        out.push(char::from_digit((b & 0x0f) as u32, 16).unwrap());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use calamine::CellErrorType;

    fn empty_row(len: usize) -> Vec<Data> {
        vec![Data::Empty; len]
    }

    #[test]
    fn validate_headers_matches() {
        let actual = vec!["R".to_string(), "G".to_string(), "B".to_string()];
        validate_headers(&actual, &["R", "G", "B"], "test").unwrap();
    }

    #[test]
    fn validate_headers_mismatch() {
        let actual = vec!["R".to_string(), "G".to_string(), "X".to_string()];
        let err = validate_headers(&actual, &["R", "G", "B"], "test").unwrap_err();
        assert!(err.to_string().contains("unexpected headers"));
    }

    #[test]
    fn parse_languagedata_csv_arg_accepts_explicit_language() {
        let parsed = parse_languagedata_csv_arg("de=/tmp/languagedata_de.csv").unwrap();
        assert_eq!(parsed.lang, "de");
        assert_eq!(parsed.path, PathBuf::from("/tmp/languagedata_de.csv"));
    }

    #[test]
    fn parse_languagedata_csv_arg_infers_language_from_filename() {
        let parsed = parse_languagedata_csv_arg("/tmp/languagedata_fr.csv").unwrap();
        assert_eq!(parsed.lang, "fr");
        assert_eq!(parsed.path, PathBuf::from("/tmp/languagedata_fr.csv"));
    }

    #[test]
    fn parse_languagedata_loc_arg_infers_language_from_filename() {
        let parsed = parse_languagedata_loc_arg("/tmp/languagedata_sp.loc").unwrap();
        assert_eq!(parsed.lang, "sp");
        assert_eq!(parsed.path, PathBuf::from("/tmp/languagedata_sp.loc"));
    }

    #[test]
    fn parse_languagedata_loc_arg_accepts_explicit_language() {
        let parsed = parse_languagedata_loc_arg("fr=/tmp/source.loc").unwrap();
        assert_eq!(parsed.lang, "fr");
        assert_eq!(parsed.path, PathBuf::from("/tmp/source.loc"));
    }

    #[test]
    fn parse_languagedata_csv_arg_rejects_locale_aliases() {
        let err = parse_languagedata_csv_arg("pt-BR=/tmp/languagedata_pt_br.csv").unwrap_err();
        assert!(err.contains("unsupported language code"));
        let err = parse_languagedata_csv_arg("DE=/tmp/languagedata_de.csv").unwrap_err();
        assert!(err.contains("unsupported language code"));
        let err = parse_languagedata_loc_arg("de-DE=/tmp/languagedata_de.loc").unwrap_err();
        assert!(err.contains("unsupported language code"));
    }

    #[test]
    fn collect_languagedata_inputs_rejects_duplicate_languages() {
        let err = collect_languagedata_inputs(
            Some(PathBuf::from("/tmp/languagedata_en.csv")),
            vec![LanguageDataCsvArg {
                lang: "en".to_string(),
                path: PathBuf::from("/tmp/other_en.csv"),
            }],
        )
        .unwrap_err();
        assert!(err
            .to_string()
            .contains("duplicate languagedata CSV for language en"));
    }

    #[test]
    fn fishing_only_filters_main_group_rows() {
        let mut row1 = empty_row(MAIN_GROUP_HEADERS.len());
        row1[MAIN_GROUP_KEY_COL] = Data::Int(100);
        row1[MAIN_GROUP_SG_COLS[0]] = Data::Int(500);

        let mut row2 = empty_row(MAIN_GROUP_HEADERS.len());
        row2[MAIN_GROUP_KEY_COL] = Data::Int(200);
        row2[MAIN_GROUP_SG_COLS[0]] = Data::Int(600);

        let rows = vec![row1, row2];
        let mut writer = WriterBuilder::new().from_writer(vec![]);

        let mut sg_keys = BTreeSet::new();
        let mut matched = BTreeSet::new();
        let mg_filter = BTreeSet::from([100]);

        let count = process_main_group_rows(
            rows.iter().map(|r| r.as_slice()),
            &mut writer,
            SubsetMode::FishingOnly,
            &mg_filter,
            &mut sg_keys,
            &mut matched,
        )
        .unwrap();

        assert_eq!(count, 1);
        assert!(matched.contains(&100));
        assert!(!matched.contains(&200));
        assert!(sg_keys.contains(&500));
        assert!(!sg_keys.contains(&600));
    }

    #[test]
    fn fishing_only_filters_sub_group_rows() {
        let mut row1 = empty_row(SUB_GROUP_HEADERS.len());
        row1[SUB_GROUP_KEY_COL] = Data::Int(900);
        let mut row2 = empty_row(SUB_GROUP_HEADERS.len());
        row2[SUB_GROUP_KEY_COL] = Data::Int(901);

        let rows = vec![row1, row2];
        let mut writer = WriterBuilder::new().from_writer(vec![]);
        let sg_filter = BTreeSet::from([901]);
        let mut matched = BTreeSet::new();

        let count = process_sub_group_rows(
            rows.iter().map(|r| r.as_slice()),
            &mut writer,
            SubsetMode::FishingOnly,
            &sg_filter,
            &mut matched,
        )
        .unwrap();

        assert_eq!(count, 1);
        assert!(matched.contains(&901));
        assert!(!matched.contains(&900));
    }

    #[test]
    fn validate_headers_normalized_accepts_prefixed_flockfish_headers() {
        let actual = vec![
            "ItemSubGroupKey".to_string(),
            "%ItemKey".to_string(),
            "%EnchantLevel".to_string(),
        ];
        validate_headers_normalized(
            &actual,
            &["ItemSubGroupKey", "ItemKey", "EnchantLevel"],
            "test",
        )
        .unwrap();
    }

    fn community_subgroup_header_row() -> Vec<Data> {
        let mut row = empty_row(COMMUNITY_SUBGROUP_PRICE_OPTION_COL + 1);
        let headers = [
            (COMMUNITY_SUBGROUP_KEY_COL, "ItemSubGroupKey"),
            (COMMUNITY_SUBGROUP_ITEM_COL, "%ItemKey"),
            (COMMUNITY_SUBGROUP_SPOTTED_COL, "spotted(auto)"),
            (COMMUNITY_SUBGROUP_COMMENT_COL, "comment"),
            (COMMUNITY_SUBGROUP_TABLE_COL, "table"),
            (COMMUNITY_SUBGROUP_REMOVED_COL, "removed"),
            (COMMUNITY_SUBGROUP_ADDED_COL, "added"),
            (COMMUNITY_SUBGROUP_ENCHANT_COL, "%EnchantLevel"),
            (COMMUNITY_SUBGROUP_DO_PET_COL, "DoPetAddDrop"),
            (COMMUNITY_SUBGROUP_DO_SECHI_COL, "DoSechiAddDrop"),
            (COMMUNITY_SUBGROUP_FOR_HUMANS_COL, "for humans"),
            (COMMUNITY_SUBGROUP_SELECT_RATE_0_COL, "%SelectRate_0"),
            (COMMUNITY_SUBGROUP_MIN_COUNT_0_COL, "%MinCount_0"),
            (COMMUNITY_SUBGROUP_MAX_COUNT_0_COL, "%MaxCount_0"),
            (COMMUNITY_SUBGROUP_SELECT_RATE_1_COL, "%SelectRate_1"),
            (COMMUNITY_SUBGROUP_MIN_COUNT_1_COL, "%MinCount_1"),
            (COMMUNITY_SUBGROUP_MAX_COUNT_1_COL, "%MaxCount_1"),
            (COMMUNITY_SUBGROUP_SELECT_RATE_2_COL, "%SelectRate_2"),
            (COMMUNITY_SUBGROUP_MIN_COUNT_2_COL, "%MinCount_2"),
            (COMMUNITY_SUBGROUP_MAX_COUNT_2_COL, "%MaxCount_2"),
            (
                COMMUNITY_SUBGROUP_INTIMACY_VARIATION_COL,
                "IntimacyVariation",
            ),
            (COMMUNITY_SUBGROUP_EXPLORATION_POINT_COL, "ExplorationPoint"),
            (
                COMMUNITY_SUBGROUP_APPLY_RANDOM_PRICE_COL,
                "ApplyRandomPrice",
            ),
            (COMMUNITY_SUBGROUP_RENT_TIME_COL, "RentTime"),
            (COMMUNITY_SUBGROUP_PRICE_OPTION_COL, "PriceOption"),
        ];
        for (idx, value) in headers {
            row[idx] = Data::String(value.to_string());
        }
        row
    }

    #[test]
    fn validate_community_subgroup_overlay_headers_accepts_no_formulas_layout() {
        let row = community_subgroup_header_row();
        validate_community_subgroup_overlay_headers(
            &row,
            Path::new("Subgroups(no formulas).xlsx"),
            "no formulas",
        )
        .unwrap();
    }

    #[test]
    fn community_subgroup_overlay_record_keeps_source_flags_and_core_columns() {
        let mut row = empty_row(COMMUNITY_SUBGROUP_PRICE_OPTION_COL + 1);
        row[COMMUNITY_SUBGROUP_KEY_COL] = Data::Float(10952.0);
        row[COMMUNITY_SUBGROUP_ITEM_COL] = Data::Float(8245.0);
        row[COMMUNITY_SUBGROUP_GRADE_COL] = Data::String("g".to_string());
        row[COMMUNITY_SUBGROUP_ITEM_NAME_COL] = Data::String("Bass".to_string());
        row[COMMUNITY_SUBGROUP_ADDED_COL] = Data::Bool(true);
        row[COMMUNITY_SUBGROUP_ENCHANT_COL] = Data::Float(0.0);
        row[COMMUNITY_SUBGROUP_DO_PET_COL] = Data::Float(0.0);
        row[COMMUNITY_SUBGROUP_DO_SECHI_COL] = Data::Float(0.0);
        row[COMMUNITY_SUBGROUP_SELECT_RATE_0_COL] = Data::String("350_000".to_string());
        row[COMMUNITY_SUBGROUP_MIN_COUNT_0_COL] = Data::Float(1.0);
        row[COMMUNITY_SUBGROUP_MAX_COUNT_0_COL] = Data::Float(1.0);
        row[COMMUNITY_SUBGROUP_SELECT_RATE_1_COL] = Data::String("350_000".to_string());
        row[COMMUNITY_SUBGROUP_MIN_COUNT_1_COL] = Data::Float(1.0);
        row[COMMUNITY_SUBGROUP_MAX_COUNT_1_COL] = Data::Float(1.0);
        row[COMMUNITY_SUBGROUP_SELECT_RATE_2_COL] = Data::String("350_000".to_string());
        row[COMMUNITY_SUBGROUP_MIN_COUNT_2_COL] = Data::Float(1.0);
        row[COMMUNITY_SUBGROUP_MAX_COUNT_2_COL] = Data::Float(1.0);
        row[COMMUNITY_SUBGROUP_APPLY_RANDOM_PRICE_COL] = Data::Float(0.0);
        row[COMMUNITY_SUBGROUP_PRICE_OPTION_COL] = Data::String("1_000_000".to_string());

        let record = build_community_subgroup_overlay_record(
            &row,
            "community_subgroups_no_formulas_workbook",
            "Community workbook",
            "abc123",
            "no formulas",
            42,
            false,
            true,
        )
        .unwrap();

        assert_eq!(record[0], "community_subgroups_no_formulas_workbook");
        assert_eq!(record[4], "42");
        assert_eq!(record[8], "g");
        assert_eq!(record[9], "Bass");
        assert_eq!(record[11], "0");
        assert_eq!(record[12], "1");
        assert_eq!(record[13], "10952");
        assert_eq!(record[14], "8245");
        assert_eq!(record[18], "350000");
        assert_eq!(record[31], "1000000");
    }

    #[test]
    fn community_subgroup_source_cells_preserve_excel_errors() {
        let value = normalized_optional_source_cell(Some(&Data::Error(CellErrorType::NA))).unwrap();
        assert_eq!(value, "#N/A");
    }

    #[test]
    fn community_subgroup_unresolved_record_keeps_raw_symbolic_row() {
        let mut row = empty_row(COMMUNITY_SUBGROUP_PRICE_OPTION_COL + 1);
        row[COMMUNITY_SUBGROUP_KEY_COL] = Data::String("New_Margoria_South".to_string());
        row[COMMUNITY_SUBGROUP_ITEM_COL] = Data::Float(40218.0);
        row[COMMUNITY_SUBGROUP_GRADE_COL] = Data::String("g".to_string());
        row[COMMUNITY_SUBGROUP_ITEM_NAME_COL] =
            Data::String("Ancient Relic Crystal Shard".to_string());
        row[COMMUNITY_SUBGROUP_SELECT_RATE_0_COL] = Data::String("920_000".to_string());
        row[COMMUNITY_SUBGROUP_SELECT_RATE_1_COL] = Data::String("920_000".to_string());
        row[COMMUNITY_SUBGROUP_SELECT_RATE_2_COL] = Data::String("920_000".to_string());

        let record = build_community_subgroup_unresolved_record(
            &row,
            "community_subgroups_no_formulas_workbook",
            "Community workbook",
            "abc123",
            "no formulas",
            2218,
            "symbolic_subgroup_key",
            false,
            false,
        )
        .unwrap();

        assert_eq!(record[4], "2218");
        assert_eq!(record[5], "symbolic_subgroup_key");
        assert_eq!(record[6], "New_Margoria_South");
        assert_eq!(record[7], "40218");
        assert_eq!(record[12], "g");
        assert_eq!(record[13], "Ancient Relic Crystal Shard");
        assert_eq!(record[19], "920_000");
    }

    #[test]
    fn item_sub_group_effective_view_uses_active_overlay_and_removed_rows() {
        let query = item_sub_group_effective_views_query();
        assert!(query.contains("community_active_overlays"));
        assert!(query.contains("overlay.`source_removed` = 0"));
        assert!(query.contains("DROP VIEW IF EXISTS `item_sub_group_item_variants`"));
        assert!(query.contains("CREATE VIEW `item_sub_group_item_variants`"));
    }

    #[test]
    fn flockfish_drop_label_to_slot_idx_maps_final_combined_labels() {
        assert_eq!(
            flockfish_drop_label_to_slot_idx("DropID PRIZE CATCH"),
            Some(1)
        );
        assert_eq!(flockfish_drop_label_to_slot_idx("DropID RARE"), Some(2));
        assert_eq!(flockfish_drop_label_to_slot_idx("DropID LARGE"), Some(3));
        assert_eq!(flockfish_drop_label_to_slot_idx("DropID GENERAL"), Some(4));
        assert_eq!(flockfish_drop_label_to_slot_idx("DropID TREASURE"), Some(5));
        assert_eq!(flockfish_drop_label_to_slot_idx("DropIDHarpoon"), None);
    }

    #[test]
    fn parse_flockfish_zone_group_value_preserves_unresolved_rows() {
        assert_eq!(
            parse_flockfish_zone_group_value("11023"),
            (Some(11023), "numeric")
        );
        assert_eq!(parse_flockfish_zone_group_value("DUMMY1"), (None, "dummy"));
        assert_eq!(parse_flockfish_zone_group_value(""), (None, "blank"));
        assert_eq!(parse_flockfish_zone_group_value("UNKNOWN"), (None, "other"));
    }

    #[test]
    fn normalize_flockfish_numeric_literal_strips_visual_underscores() {
        assert_eq!(normalize_flockfish_numeric_literal("292_200"), "292200");
        assert_eq!(normalize_flockfish_numeric_literal("1_000_000"), "1000000");
        assert_eq!(
            normalize_flockfish_numeric_literal("getLifeLevel(1)>34;"),
            "getLifeLevel(1)>34;"
        );
    }

    #[test]
    fn flockfish_subgroup_outlier_filter_drops_velia_bottle() {
        let row = vec![
            "10956".to_string(),
            "43871".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
        ];
        assert!(is_removed_flockfish_subgroup_outlier(&row));

        let keep = vec![
            "10956".to_string(),
            "54031".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
        ];
        assert!(!is_removed_flockfish_subgroup_outlier(&keep));
    }

    #[test]
    fn consumable_effect_texts_prefer_composite_buff_rows_over_leaf_rows() {
        let mut buff_text_rows = HashMap::new();
        buff_text_rows.insert(
            "55426".to_string(),
            CalculatorConsumableBuffText {
                text: "엔트의 눈물\n생활 경험치 획득량 +30%\n낚시 속도 잠재력 +2단계".to_string(),
                has_description: true,
            },
        );
        buff_text_rows.insert(
            "55427".to_string(),
            CalculatorConsumableBuffText {
                text: "생활 경험치 획득량 +30%".to_string(),
                has_description: false,
            },
        );

        let selected = select_consumable_effect_texts(
            "59335",
            &["55426".to_string(), "55427".to_string()],
            &buff_text_rows,
            &HashMap::new(),
        );

        assert_eq!(
            selected,
            vec!["엔트의 눈물\n생활 경험치 획득량 +30%\n낚시 속도 잠재력 +2단계".to_string()]
        );
    }

    #[test]
    fn consumable_effect_texts_fall_back_to_leaf_rows_without_composite_text() {
        let mut buff_text_rows = HashMap::new();
        buff_text_rows.insert(
            "55948".to_string(),
            CalculatorConsumableBuffText {
                text: "낚시 경험치 획득량 +10%".to_string(),
                has_description: true,
            },
        );
        buff_text_rows.insert(
            "55942".to_string(),
            CalculatorConsumableBuffText {
                text: "자동 낚시 시간 감소 7%".to_string(),
                has_description: true,
            },
        );

        let selected = select_consumable_effect_texts(
            "55570",
            &["55948".to_string(), "55942".to_string()],
            &buff_text_rows,
            &HashMap::new(),
        );

        assert_eq!(
            selected,
            vec![
                "낚시 경험치 획득량 +10%".to_string(),
                "자동 낚시 시간 감소 7%".to_string(),
            ]
        );
    }

    #[test]
    fn consumable_category_prefers_primary_skill_over_buff_removal_subskill() {
        let by_skill = HashMap::from([
            (
                "55595".to_string(),
                CalculatorConsumableBuffCategory {
                    category_id: Some(1),
                    category_level: Some(1),
                },
            ),
            (
                "51349".to_string(),
                CalculatorConsumableBuffCategory {
                    category_id: Some(10),
                    category_level: Some(1),
                },
            ),
        ]);

        let selected = select_consumable_category_metadata(Some("55595"), Some("51349"), &by_skill);

        assert_eq!(selected.category_id, Some(1));
        assert_eq!(selected.category_level, Some(1));
    }

    #[test]
    fn fallback_consumable_family_uses_skill_family_for_duplicate_skills() {
        let counts = HashMap::from([("12345".to_string(), 3usize)]);

        let key = fallback_consumable_family_key(Some("12345"), &counts);

        assert_eq!(key.as_deref(), Some("skill-family:12345"));
    }

    #[test]
    fn fallback_consumable_family_is_none_for_unique_skills_without_category() {
        let counts = HashMap::from([("59778".to_string(), 1usize)]);

        let key = fallback_consumable_family_key(Some("59778"), &counts);

        assert_eq!(key, None);
    }

    #[test]
    fn format_manual_community_notes_keeps_structural_provenance() {
        assert_eq!(
            format_manual_community_notes(Some(1), Some(0.01), Some(11057), Some(11057)).as_deref(),
            Some("slot_idx=1;guessed_rate=0.01;item_main_group_key=11057;subgroup_key=11057")
        );
        assert_eq!(
            format_manual_community_notes(Some(4), None, Some(11021), None).as_deref(),
            Some("slot_idx=4;item_main_group_key=11021")
        );
        assert_eq!(format_manual_community_notes(None, None, None, None), None);
    }

    #[test]
    fn manual_presence_status_uses_expected_db_values() {
        assert_eq!(
            ManualCommunityPresenceStatus::Confirmed.as_db_value(),
            "confirmed"
        );
        assert_eq!(
            ManualCommunityPresenceStatus::Unconfirmed.as_db_value(),
            "unconfirmed"
        );
        assert_eq!(
            ManualCommunityPresenceStatus::DataIncomplete.as_db_value(),
            "data_incomplete"
        );
    }

    #[test]
    fn community_fish_group_uses_expected_slot_idx() {
        assert_eq!(CommunityFishGroup::Prize.slot_idx(), 1);
        assert_eq!(CommunityFishGroup::Rare.slot_idx(), 2);
        assert_eq!(CommunityFishGroup::HighQuality.slot_idx(), 3);
        assert_eq!(CommunityFishGroup::General.slot_idx(), 4);
        assert_eq!(CommunityFishGroup::Trash.slot_idx(), 5);
    }

    #[test]
    fn resolve_requested_slot_idx_uses_group_and_default() {
        assert_eq!(
            resolve_requested_slot_idx(None, Some(CommunityFishGroup::General), None).unwrap(),
            Some(4)
        );
        assert_eq!(
            resolve_requested_slot_idx(None, None, Some(1)).unwrap(),
            Some(1)
        );
        assert_eq!(resolve_requested_slot_idx(None, None, None).unwrap(), None);
    }

    #[test]
    fn resolve_requested_slot_idx_rejects_conflicting_inputs() {
        let err = resolve_requested_slot_idx(Some(1), Some(CommunityFishGroup::General), None)
            .unwrap_err();
        assert!(err.to_string().contains("conflicts with"));
    }

    #[test]
    fn find_dolt_repo_root_walks_up_to_repo_root() {
        let unique = format!(
            "fishystuff-dolt-import-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time before unix epoch")
                .as_nanos()
        );
        let root = std::env::temp_dir().join(unique);
        let nested = root.join("a/b/c");
        fs::create_dir_all(root.join(".dolt")).expect("create .dolt");
        fs::create_dir_all(&nested).expect("create nested path");

        let resolved = find_dolt_repo_root(&nested).expect("resolve repo root");
        assert_eq!(resolved, root);

        fs::remove_dir_all(&root).expect("remove temp repo");
    }
}
