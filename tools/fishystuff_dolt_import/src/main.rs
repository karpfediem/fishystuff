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

const LANGUAGEDATA_HEADERS: [&str; 4] = ["id", "unk", "text", "format"];
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

struct FlockfishSubgroupImportCommand {
    dolt_repo: PathBuf,
    workbook_xlsx: PathBuf,
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

struct CalculatorEffectsImportCommand {
    dolt_repo: PathBuf,
    excel_dir: PathBuf,
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

fn infer_languagedata_lang(path: &Path) -> std::result::Result<String, String> {
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("cannot infer language from {}", path.display()))?;
    let lang = stem
        .strip_prefix("languagedata_")
        .ok_or_else(|| format!("expected filename like languagedata_<lang>.csv: {stem}"))?;
    normalize_languagedata_lang(lang)
}

fn normalize_languagedata_lang(value: &str) -> std::result::Result<String, String> {
    let lang = value.trim().to_ascii_lowercase().replace('-', "_");
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

fn languagedata_table_name(lang: &str) -> String {
    format!("languagedata_{lang}")
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
                output_dir.join(format!("{}.csv", languagedata_table_name(lang))),
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
    for (lang, output_path) in &outputs.languagedata_csvs {
        run_dolt_table_import(&dolt_repo, &languagedata_table_name(lang), output_path)?;
    }
    ensure_calculator_lookup_indexes(&dolt_repo)?;
    refresh_calculator_item_name_tables(&dolt_repo)?;

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

fn ensure_calculator_lookup_indexes(repo_path: &Path) -> Result<()> {
    for source_table in list_languagedata_tables(repo_path)?.values() {
        ensure_dolt_index(
            repo_path,
            source_table,
            &format!("idx_{source_table}_id_format_unk"),
            &format!(
                "CREATE INDEX {} ON {} (`id`, `format`(1), `unk`(8));",
                sql_ident(&format!("idx_{source_table}_id_format_unk")),
                sql_ident(source_table)
            ),
        )?;
        ensure_dolt_index(
            repo_path,
            source_table,
            &format!("idx_{source_table}_format_unk_id"),
            &format!(
                "CREATE INDEX {} ON {} (`format`(1), `unk`(8), `id`);",
                sql_ident(&format!("idx_{source_table}_format_unk_id")),
                sql_ident(source_table)
            ),
        )?;
    }
    ensure_dolt_index(
        repo_path,
        "item_table",
        "idx_item_table_item_name",
        "CREATE INDEX `idx_item_table_item_name` \
         ON `item_table` (`ItemName`(191));",
    )?;
    Ok(())
}

fn refresh_calculator_item_name_tables(repo_path: &Path) -> Result<()> {
    for (lang, source_table) in list_languagedata_tables(repo_path)? {
        refresh_calculator_item_names(repo_path, &lang, &source_table)?;
        refresh_calculator_lightstone_set_names(repo_path, &lang, &source_table)?;
    }
    Ok(())
}

fn refresh_calculator_item_names(repo_path: &Path, lang: &str, source_table: &str) -> Result<()> {
    if !dolt_table_exists(repo_path, source_table)? {
        return Ok(());
    }
    let query = format!(
        "CREATE TABLE IF NOT EXISTS `calculator_item_names` (\
            `lang` VARCHAR(16) NOT NULL,\
            `item_id` BIGINT NOT NULL,\
            `name` LONGTEXT,\
            PRIMARY KEY (`lang`, `item_id`),\
            KEY `idx_calculator_item_names_item_id` (`item_id`)\
         ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;\
         DELETE FROM `calculator_item_names` WHERE `lang` = {lang};\
         INSERT INTO `calculator_item_names` (`lang`, `item_id`, `name`) \
         SELECT {lang} AS lang, \
                CAST(`id` AS SIGNED) AS item_id, \
                MAX(NULLIF(TRIM(`text`), '')) AS name \
         FROM {source_table} \
         WHERE `format` = 'A' \
           AND `unk` IS NULL \
         GROUP BY CAST(`id` AS SIGNED);",
        lang = sql_value(lang),
        source_table = sql_ident(source_table),
    );
    run_dolt_sql_query_or_remote(
        repo_path,
        &query,
        &format!("refresh calculator item names for {lang}"),
    )
}

fn refresh_calculator_lightstone_set_names(
    repo_path: &Path,
    lang: &str,
    source_table: &str,
) -> Result<()> {
    if !dolt_table_exists(repo_path, source_table)? {
        return Ok(());
    }
    let query = format!(
        "CREATE TABLE IF NOT EXISTS `calculator_lightstone_set_names` (\
            `lang` VARCHAR(16) NOT NULL,\
            `lightstone_set_id` BIGINT NOT NULL,\
            `name` LONGTEXT,\
            PRIMARY KEY (`lang`, `lightstone_set_id`),\
            KEY `idx_calculator_lightstone_set_names_set_id` (`lightstone_set_id`)\
         ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_bin;\
         DELETE FROM `calculator_lightstone_set_names` WHERE `lang` = {lang};\
         INSERT INTO `calculator_lightstone_set_names` (`lang`, `lightstone_set_id`, `name`) \
         SELECT {lang} AS lang, \
                CAST(`id` AS SIGNED) AS lightstone_set_id, \
                MAX(NULLIF(TRIM(TRAILING ']' FROM SUBSTRING_INDEX(SUBSTRING_INDEX(NULLIF(TRIM(`text`), ''), ']', 1), '[', -1)), '')) AS name \
         FROM {source_table} \
         WHERE `format` = 'B' \
           AND `unk` = '113' \
           AND NULLIF(TRIM(`text`), '') IS NOT NULL \
           AND `text` LIKE '%[%' \
           AND `text` LIKE '%]%' \
         GROUP BY CAST(`id` AS SIGNED);",
        lang = sql_value(lang),
        source_table = sql_ident(source_table),
    );
    run_dolt_sql_query_or_remote(
        repo_path,
        &query,
        &format!("refresh calculator lightstone set names for {lang}"),
    )
}

fn list_languagedata_tables(repo_path: &Path) -> Result<BTreeMap<String, String>> {
    let rows = run_dolt_select_named_rows(
        repo_path,
        "SELECT table_name \
         FROM information_schema.tables \
         WHERE table_schema = DATABASE() \
           AND table_name LIKE 'languagedata_%'",
        "list languagedata tables",
    )?;
    let mut tables = BTreeMap::<String, String>::new();
    for row in rows {
        let Some(table_name) = row.get("table_name").map(String::as_str) else {
            continue;
        };
        let Some(lang) = table_name
            .strip_prefix("languagedata_")
            .and_then(|lang| normalize_languagedata_lang(lang).ok())
        else {
            continue;
        };
        tables.insert(lang, table_name.to_string());
    }
    Ok(tables)
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

    Ok(())
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
        &LANGUAGEDATA_HEADERS,
        &format!("{}:{}", path.display(), languagedata_table_name(lang)),
    )?;

    let mut writer = build_csv_writer(output_csv)?;
    writer.write_record(LANGUAGEDATA_HEADERS)?;

    let mut row_count = 0;
    for row in reader.records() {
        let record = row.context("read languagedata csv row")?;
        let mut out = Vec::with_capacity(LANGUAGEDATA_HEADERS.len());
        for i in 0..LANGUAGEDATA_HEADERS.len() {
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
