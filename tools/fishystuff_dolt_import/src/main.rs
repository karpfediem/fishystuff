mod item_table_headers;

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use calamine::{open_workbook_auto, Data, Range, Reader};
use clap::{Parser, Subcommand, ValueEnum};
use csv::{QuoteStyle, Writer, WriterBuilder};
use sha2::{Digest, Sha256};

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

const FISHING_MG_COLS: [usize; 8] = [3, 4, 5, 7, 9, 11, 13, 15];
const MAIN_GROUP_KEY_COL: usize = 0;
const MAIN_GROUP_SG_COLS: [usize; 4] = [7, 10, 13, 16];
const SUB_GROUP_KEY_COL: usize = 0;

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
        #[arg(long)]
        output_dir: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = SubsetMode::FishingOnly)]
        subset: SubsetMode,
        #[arg(long)]
        apply_schema: Option<PathBuf>,
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
    output_dir: Option<PathBuf>,
    subset: SubsetMode,
    apply_schema: Option<PathBuf>,
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
    languagedata_sha: Option<String>,
}

struct ImportOutputs {
    fishing_csv: PathBuf,
    main_group_csv: PathBuf,
    sub_group_csv: PathBuf,
    item_table_csv: PathBuf,
    fish_table_csv: PathBuf,
    patches_csv: PathBuf,
    languagedata_csv: PathBuf,
}

struct ImportReport<'a> {
    subset: SubsetMode,
    fishing: &'a FishingImport,
    main_group: &'a MainGroupImport,
    sub_group: &'a SubGroupImport,
    item_table: Option<&'a ItemTableImport>,
    fish_table: Option<&'a FishTableImport>,
    patches: Option<&'a PatchesImport>,
    languagedata: Option<&'a LanguageDataImport>,
    outputs: &'a ImportOutputs,
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
            output_dir,
            subset,
            apply_schema,
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
            output_dir,
            subset,
            apply_schema,
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
        output_dir,
        subset,
        apply_schema,
        commit,
        commit_msg,
    } = command;

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
        languagedata_sha: match languagedata_en_csv.as_ref() {
            Some(path) => Some(sha256_file(path)?),
            None => None,
        },
    };

    if let Some(schema_path) = apply_schema {
        apply_schema_sql(&dolt_repo, &schema_path)?;
    }

    let outputs = ImportOutputs {
        fishing_csv: output_dir.join("fishing_table.csv"),
        main_group_csv: output_dir.join("item_main_group_table.csv"),
        sub_group_csv: output_dir.join("item_sub_group_table.csv"),
        item_table_csv: output_dir.join("item_table.csv"),
        fish_table_csv: output_dir.join("fish_table.csv"),
        patches_csv: output_dir.join("patches.csv"),
        languagedata_csv: output_dir.join("languagedata_en.csv"),
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
    let languagedata_stats = match languagedata_en_csv.as_ref() {
        Some(path) => Some(import_languagedata_en_csv(path, &outputs.languagedata_csv)?),
        None => None,
    };

    run_dolt_table_import(&dolt_repo, "fishing_table", &outputs.fishing_csv)?;
    run_dolt_table_import(&dolt_repo, "item_main_group_table", &outputs.main_group_csv)?;
    run_dolt_table_import(&dolt_repo, "item_sub_group_table", &outputs.sub_group_csv)?;
    if item_table_stats.is_some() {
        run_dolt_table_import(&dolt_repo, "item_table", &outputs.item_table_csv)?;
    }
    if fish_table_stats.is_some() {
        run_dolt_table_import(&dolt_repo, "fish_table", &outputs.fish_table_csv)?;
    }
    if patches_stats.is_some() {
        run_dolt_table_import(&dolt_repo, "patches", &outputs.patches_csv)?;
    }
    if languagedata_stats.is_some() {
        run_dolt_table_import(&dolt_repo, "languagedata_en", &outputs.languagedata_csv)?;
    }

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
        languagedata: languagedata_stats.as_ref(),
        outputs: &outputs,
    });

    Ok(())
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

fn import_languagedata_en_csv(path: &Path, output_csv: &Path) -> Result<LanguageDataImport> {
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
        &format!("{}:languagedata_en", path.display()),
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

fn apply_schema_sql(repo_path: &Path, schema_path: &Path) -> Result<()> {
    let schema_file = File::open(schema_path)
        .with_context(|| format!("open schema file: {}", schema_path.display()))?;
    let output = Command::new("dolt")
        .current_dir(repo_path)
        .arg("sql")
        .stdin(Stdio::from(schema_file))
        .output()
        .context("run dolt sql for schema")?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    bail!("dolt sql failed: {stderr}");
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
    if let Some(languagedata_sha) = digests.languagedata_sha.as_deref() {
        parts.push(format!("LanguageData_EN={languagedata_sha}"));
    }
    let suffix = format!("({})", parts.join(", "));
    match base {
        Some(msg) => format!("{msg} {suffix}"),
        None => format!("Import fishing-related groups from community XLSX snapshot {suffix}"),
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
    if let Some(languagedata) = languagedata {
        println!("languagedata_en rows emitted: {}", languagedata.row_count);
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
    if languagedata.is_some() {
        println!(
            "output languagedata_en csv: {}",
            outputs.languagedata_csv.display()
        );
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
}
