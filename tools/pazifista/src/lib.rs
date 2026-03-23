mod archive;
mod compression;
mod gcdata;
mod ice;
mod pabr;
mod wildcard;

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use archive::{ArchiveIndex, ExtractOptions};
use clap::{ArgAction, CommandFactory, Parser, Subcommand};
use gcdata::{
    compare_region_sources, inspect_arraywaypoint_bin, inspect_pabr_table,
    inspect_regionclientdata, inspect_regiongroupinfo_bss, inspect_regioninfo_bss,
    inspect_stringtable_bss, inspect_waypoint_xml,
};
use pabr::{PabrMap, RegionGroupMapping, DEFAULT_ROW_SHIFT};

const VERSION_HEADER: &str =
    "pazifista - tool for extracting Black Desert Online archives and decoding PABR region maps.";

#[derive(Debug, Parser)]
#[command(
    name = "pazifista",
    about = "Extract Black Desert Online archives and inspect or render PABR region maps"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    #[command(flatten)]
    extract: ExtractCli,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(about = "Inspect archive entries and raw payloads")]
    Archive(ArchiveCli),
    #[command(about = "Inspect or render .rid/.bkd PABR region-map pairs")]
    Pabr(PabrCli),
    #[command(about = "Inspect original gamecommondata metadata related to regions")]
    Gcdata(GcdataCli),
}

#[derive(Debug, Parser, Default)]
struct ExtractCli {
    #[arg(value_name = "input file")]
    input_file: Option<PathBuf>,
    #[arg(
        short = 'f',
        value_name = "mask",
        action = ArgAction::Append,
        help = "Filter mask; repeat -f to match multiple masks"
    )]
    filters: Vec<String>,
    #[arg(short = 'o', value_name = "path")]
    output: Option<PathBuf>,
    #[arg(short = 'l')]
    list: bool,
    #[arg(short = 'n')]
    no_folders: bool,
    #[arg(short = 'y')]
    yes_to_all: bool,
    #[arg(short = 'q')]
    quiet: bool,
}

#[derive(Debug, Parser)]
struct PabrCli {
    #[command(subcommand)]
    command: PabrCommand,
}

#[derive(Debug, Parser)]
struct ArchiveCli {
    #[command(subcommand)]
    command: ArchiveCommand,
}

#[derive(Debug, Parser)]
struct GcdataCli {
    #[command(subcommand)]
    command: GcdataCommand,
}

#[derive(Debug, Subcommand)]
enum ArchiveCommand {
    #[command(
        about = "Inspect matching archive entries and optionally dump their raw on-disk payload"
    )]
    Inspect(ArchiveInspectCli),
}

#[derive(Debug, Subcommand)]
enum PabrCommand {
    #[command(about = "Print structural metadata for a PABR .rid/.bkd pair")]
    Inspect(PabrInputCli),
    #[command(about = "Render a deterministic BMP visualization from a PABR .rid/.bkd pair")]
    Render(PabrRenderCli),
    #[command(
        about = "Export unsmoothed regions GeoJSON derived directly from a PABR .rid/.bkd pair"
    )]
    ExportRegionsGeojson(PabrExportRegionsGeojsonCli),
    #[command(
        about = "Export unsmoothed region-groups GeoJSON derived directly from a PABR .rid/.bkd pair"
    )]
    ExportRegionGroupsGeojson(PabrExportRegionGroupsGeojsonCli),
    #[command(about = "Match PABR region IDs against the current regions GeoJSON by overlap")]
    MatchRegions(PabrMatchRegionsCli),
}

#[derive(Debug, Subcommand)]
enum GcdataCommand {
    #[command(
        about = "Compare region IDs across PABR RID, current GeoJSON, current regioninfo.json, regioninfo.bss, and regionclientdata XMLs"
    )]
    CompareRegionSources(GcdataCompareRegionSourcesCli),
    #[command(about = "List the region IDs present in one or more regionclientdata_*.xml files")]
    InspectRegionclientdata(GcdataInspectRegionclientdataCli),
    #[command(about = "Print the PABR entry count header for a gamecommondata .bss table")]
    InspectPabrTable(GcdataInspectPabrTableCli),
    #[command(
        about = "Decode the validated regioninfo.bss row family and compare it against the current external regioninfo.json"
    )]
    InspectRegioninfoBss(GcdataInspectRegioninfoBssCli),
    #[command(
        about = "Decode regiongroupinfo.bss rows and compare them against the current external deck_rg_graphs.json"
    )]
    InspectRegiongroupinfoBss(GcdataInspectRegiongroupinfoBssCli),
    #[command(
        about = "Decode the structure of mapdata_arraywaypoint.bin and optionally sample current waypoint positions against it"
    )]
    InspectArraywaypointBin(GcdataInspectArraywaypointBinCli),
    #[command(
        about = "Decode stringtable.bss index sections and trailing text entries, then inspect focus string IDs"
    )]
    InspectStringtableBss(GcdataInspectStringtableBssCli),
    #[command(
        about = "Inspect original gamecommondata waypoint XML rows and links, including mapdata_realexplore*.xml"
    )]
    InspectWaypointXml(GcdataInspectWaypointXmlCli),
}

#[derive(Debug, Parser)]
struct PabrInputCli {
    #[arg(value_name = "input file", help = "Either the .rid or .bkd file")]
    input_file: PathBuf,
}

#[derive(Debug, Parser)]
struct PabrRenderCli {
    #[arg(value_name = "input file", help = "Either the .rid or .bkd file")]
    input_file: PathBuf,
    #[arg(short = 'o', long = "output", value_name = "path")]
    output: PathBuf,
    #[arg(long = "width", value_name = "pixels")]
    width: Option<u32>,
    #[arg(long = "height", value_name = "pixels")]
    height: Option<u32>,
    #[arg(long = "scale", value_name = "factor")]
    scale: Option<f32>,
    #[arg(long = "row-shift", value_name = "value", default_value_t = DEFAULT_ROW_SHIFT)]
    row_shift: u32,
    #[arg(short = 'q')]
    quiet: bool,
}

#[derive(Debug, Parser)]
struct PabrExportRegionsGeojsonCli {
    #[arg(value_name = "input file", help = "Either the .rid or .bkd file")]
    input_file: PathBuf,
    #[arg(short = 'o', long = "output", value_name = "path")]
    output: PathBuf,
    #[arg(long = "row-shift", value_name = "value", default_value_t = DEFAULT_ROW_SHIFT)]
    row_shift: u32,
    #[arg(short = 'q')]
    quiet: bool,
}

#[derive(Debug, Parser)]
struct PabrExportRegionGroupsGeojsonCli {
    #[arg(value_name = "input file", help = "Either the .rid or .bkd file")]
    input_file: PathBuf,
    #[arg(long = "regioninfo", value_name = "path")]
    regioninfo: PathBuf,
    #[arg(short = 'o', long = "output", value_name = "path")]
    output: PathBuf,
    #[arg(long = "row-shift", value_name = "value", default_value_t = DEFAULT_ROW_SHIFT)]
    row_shift: u32,
    #[arg(short = 'q')]
    quiet: bool,
}

#[derive(Debug, Parser)]
struct PabrMatchRegionsCli {
    #[arg(value_name = "input file", help = "Either the .rid or .bkd file")]
    input_file: PathBuf,
    #[arg(long = "current-regions", value_name = "path")]
    current_regions: PathBuf,
    #[arg(short = 'o', long = "output", value_name = "path")]
    output: PathBuf,
    #[arg(long = "row-shift", value_name = "value", default_value_t = DEFAULT_ROW_SHIFT)]
    row_shift: u32,
    #[arg(long = "top", value_name = "count", default_value_t = 3)]
    top: usize,
    #[arg(short = 'q')]
    quiet: bool,
}

#[derive(Debug, Parser)]
struct GcdataCompareRegionSourcesCli {
    #[arg(long = "rid", value_name = "path")]
    rid: PathBuf,
    #[arg(long = "current-regions", value_name = "path")]
    current_regions: PathBuf,
    #[arg(long = "row-shift", value_name = "value", default_value_t = DEFAULT_ROW_SHIFT)]
    row_shift: u32,
    #[arg(long = "current-regioninfo", value_name = "path")]
    current_regioninfo: Option<PathBuf>,
    #[arg(long = "regioninfo-bss", value_name = "path")]
    regioninfo_bss: Option<PathBuf>,
    #[arg(
        long = "regionclientdata",
        value_name = "path",
        action = ArgAction::Append
    )]
    regionclientdata: Vec<PathBuf>,
    #[arg(short = 'o', long = "output", value_name = "path")]
    output: PathBuf,
    #[arg(short = 'q')]
    quiet: bool,
}

#[derive(Debug, Parser)]
struct GcdataInspectRegionclientdataCli {
    #[arg(value_name = "regionclientdata xml", required = true)]
    input_files: Vec<PathBuf>,
    #[arg(long = "id", value_name = "region-id", action = ArgAction::Append)]
    focus_region_ids: Vec<u32>,
}

#[derive(Debug, Parser)]
struct GcdataInspectPabrTableCli {
    #[arg(value_name = "bss file")]
    input_file: PathBuf,
}

#[derive(Debug, Parser)]
struct GcdataInspectRegioninfoBssCli {
    #[arg(value_name = "regioninfo.bss file")]
    input_file: PathBuf,
    #[arg(long = "loc", value_name = "path")]
    loc: Option<PathBuf>,
    #[arg(long = "current-regioninfo", value_name = "path")]
    current_regioninfo: Option<PathBuf>,
    #[arg(long = "id", value_name = "region-id", action = ArgAction::Append)]
    focus_region_ids: Vec<u32>,
    #[arg(short = 'o', long = "output", value_name = "path")]
    output: Option<PathBuf>,
}

#[derive(Debug, Parser)]
struct GcdataInspectRegiongroupinfoBssCli {
    #[arg(value_name = "regiongroupinfo.bss file")]
    input_file: PathBuf,
    #[arg(long = "current-deck-rg-graphs", value_name = "path")]
    current_deck_rg_graphs: Option<PathBuf>,
    #[arg(long = "id", value_name = "group-id", action = ArgAction::Append)]
    focus_group_ids: Vec<u32>,
    #[arg(short = 'o', long = "output", value_name = "path")]
    output: Option<PathBuf>,
}

#[derive(Debug, Parser)]
struct GcdataInspectArraywaypointBinCli {
    #[arg(value_name = "mapdata_arraywaypoint.bin file")]
    input_file: PathBuf,
    #[arg(long = "waypoints", value_name = "path")]
    waypoints: Option<PathBuf>,
    #[arg(long = "id", value_name = "waypoint-id", action = ArgAction::Append)]
    focus_waypoint_ids: Vec<u32>,
    #[arg(short = 'o', long = "output", value_name = "path")]
    output: Option<PathBuf>,
    #[arg(long = "preview-bmp", value_name = "path")]
    preview_bmp: Option<PathBuf>,
}

#[derive(Debug, Parser)]
struct GcdataInspectStringtableBssCli {
    #[arg(value_name = "stringtable.bss file")]
    input_file: PathBuf,
    #[arg(long = "id", value_name = "string-id", action = ArgAction::Append)]
    focus_string_ids: Vec<u32>,
    #[arg(short = 'o', long = "output", value_name = "path")]
    output: Option<PathBuf>,
}

#[derive(Debug, Parser)]
struct GcdataInspectWaypointXmlCli {
    #[arg(value_name = "waypoint xml file")]
    input_file: PathBuf,
    #[arg(long = "id", value_name = "waypoint-id", action = ArgAction::Append)]
    focus_waypoint_ids: Vec<u32>,
    #[arg(short = 'o', long = "output", value_name = "path")]
    output: Option<PathBuf>,
}

#[derive(Debug, Parser)]
struct ArchiveInspectCli {
    #[arg(value_name = "input file")]
    input_file: PathBuf,
    #[arg(
        short = 'f',
        value_name = "mask",
        action = ArgAction::Append,
        help = "Filter mask; repeat -f to inspect multiple matches"
    )]
    filters: Vec<String>,
    #[arg(
        long = "all",
        help = "Print every matching entry instead of just the first one"
    )]
    all: bool,
    #[arg(
        short = 'o',
        long = "raw-output",
        value_name = "path",
        help = "Write the raw on-disk payload for a single matching entry"
    )]
    raw_output: Option<PathBuf>,
    #[arg(
        long = "raw-preview-bytes",
        value_name = "count",
        default_value_t = 32,
        help = "Number of raw bytes to preview in hex for a single matching entry"
    )]
    raw_preview_bytes: usize,
    #[arg(short = 'q')]
    quiet: bool,
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    if cli.command.is_none() && cli.extract.input_file.is_none() {
        let mut command = Cli::command();
        command.print_help()?;
        println!();
        return Ok(());
    }

    match cli.command {
        Some(Command::Archive(archive_cli)) => run_archive(archive_cli),
        Some(Command::Pabr(pabr_cli)) => run_pabr(pabr_cli),
        Some(Command::Gcdata(gcdata_cli)) => run_gcdata(gcdata_cli),
        None => run_extract(cli.extract),
    }
}

fn run_extract(cli: ExtractCli) -> Result<()> {
    let input_file = cli
        .input_file
        .context("missing input file; use `pazifista -h` for help")?;
    let source_path = canonical_existing_path(&input_file)?;
    let target_path = match &cli.output {
        Some(path) => normalize_output_path(path)?,
        None => std::env::current_dir().context("failed to read current directory")?,
    };

    if !cli.quiet {
        println!("{VERSION_HEADER}");
        println!("Type pazifista -h for help.");
        println!();
        println!("Source file: {}", source_path.display());
        println!("Target path: {}", target_path.display());
        println!("Filter masks: {}", format_filters(&cli.filters));
    }

    let extension = source_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default();

    let mut archive = if extension.eq_ignore_ascii_case("meta") {
        ArchiveIndex::from_meta(&source_path, cli.quiet)?
    } else if extension.eq_ignore_ascii_case("paz") {
        ArchiveIndex::from_paz(&source_path, cli.quiet)?
    } else {
        bail!(
            "input file must have extension .meta or .paz; got .{}",
            extension
        );
    };

    if cli.list {
        archive.list(&cli.filters, cli.quiet);
    } else {
        archive.extract(
            &cli.filters,
            &target_path,
            ExtractOptions {
                quiet: cli.quiet,
                no_folders: cli.no_folders,
                yes_to_all: cli.yes_to_all,
            },
        )?;
    }

    Ok(())
}

fn run_archive(cli: ArchiveCli) -> Result<()> {
    match cli.command {
        ArchiveCommand::Inspect(args) => {
            if args.filters.is_empty() {
                bail!("at least one -f/--filter mask is required");
            }

            let input_path = canonical_existing_path(&args.input_file)?;
            let extension = input_path
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or_default();
            let mut archive = if extension.eq_ignore_ascii_case("meta") {
                ArchiveIndex::from_meta(&input_path, true)?
            } else if extension.eq_ignore_ascii_case("paz") {
                ArchiveIndex::from_paz(&input_path, true)?
            } else {
                bail!(
                    "input file must have extension .meta or .paz; got .{}",
                    extension
                );
            };

            let matches = archive.matching_entries(&args.filters);
            if matches.is_empty() {
                bail!(
                    "no archive entries matched {}",
                    format_filters(&args.filters)
                );
            }

            let print_all = args.all || matches.len() == 1;
            let printed_entries = if print_all {
                matches.clone()
            } else {
                vec![matches[0].clone()]
            };

            if let Some(raw_output) = args.raw_output.as_ref() {
                if matches.len() != 1 {
                    bail!(
                        "--raw-output requires exactly one matching entry, got {}",
                        matches.len()
                    );
                }
                let raw_output = normalize_output_path(raw_output)?;
                let raw = archive.read_raw_payload(&matches[0])?;
                std::fs::write(&raw_output, &raw)
                    .with_context(|| format!("failed to write {}", raw_output.display()))?;
                if !args.quiet {
                    println!("Raw payload: {}", raw_output.display());
                }
            }

            let raw_preview = if matches.len() == 1 && args.raw_preview_bytes > 0 {
                Some(archive.read_raw_payload(&matches[0])?)
            } else {
                None
            };

            if !args.quiet {
                println!("{VERSION_HEADER}");
                println!("Archive: {}", input_path.display());
                println!("Filter masks: {}", format_filters(&args.filters));
                println!("Mobile archive mode: {}", archive.is_mobile());
                println!("Matching entries: {}", matches.len());
                if !print_all {
                    println!("Showing first match; pass --all to print the rest.");
                }
            }

            for entry in printed_entries {
                println!("Path: {}", entry.file_path);
                println!("PAZ: pad{:05}.paz", entry.paz_num);
                println!("Offset: {}", entry.offset);
                println!("Compressed size: {}", entry.compressed_size);
                println!("Original size: {}", entry.original_size);
                println!(
                    "ICE block aligned: {}",
                    archive.is_mobile() || entry.compressed_size % 8 == 0
                );
                println!();
            }

            if let Some(raw_preview) = raw_preview.as_ref() {
                let preview_len = args.raw_preview_bytes.min(raw_preview.len());
                println!(
                    "Raw preview ({} bytes): {}",
                    preview_len,
                    format_hex_bytes(&raw_preview[..preview_len])
                );
            }

            Ok(())
        }
    }
}

fn run_pabr(cli: PabrCli) -> Result<()> {
    match cli.command {
        PabrCommand::Inspect(args) => {
            let map = load_pabr_map(&args.input_file)?;
            let inspect = map.inspect()?;

            println!("{VERSION_HEADER}");
            println!("RID file: {}", map.rid_path.display());
            println!("BKD file: {}", map.bkd_path.display());
            println!(
                "Native size: {}x{}",
                inspect.native_width, inspect.native_height
            );
            println!("Wrapped bands: {}", inspect.wrapped_bands);
            println!("Dictionary entries: {}", inspect.dictionary_entries);
            println!("Scanline rows: {}", inspect.scanline_rows);
            println!(
                "Used dictionary entries: {}",
                inspect.used_dictionary_entries
            );
            println!("Used region IDs: {}", inspect.used_region_ids);
            println!(
                "Transparent breakpoints: {}",
                inspect.transparent_breakpoints
            );
            println!("Max source x: {}", inspect.max_source_x);
            println!(
                "RID trailer prefix length: {} bytes",
                inspect.rid_trailer_prefix_len
            );
            println!(
                "BKD trailer words: [{}, {}, {}]",
                inspect.bkd_trailer_words[0],
                inspect.bkd_trailer_words[1],
                inspect.bkd_trailer_words[2]
            );
            Ok(())
        }
        PabrCommand::Render(args) => {
            let map = load_pabr_map(&args.input_file)?;
            let output_path = normalize_output_path(&args.output)?;
            let dimensions = map.resolve_output_dimensions(args.width, args.height, args.scale)?;
            let summary = map.render_bmp(&output_path, dimensions, args.row_shift)?;

            if !args.quiet {
                println!("{VERSION_HEADER}");
                println!("RID file: {}", map.rid_path.display());
                println!("BKD file: {}", map.bkd_path.display());
                println!("Row shift: {}", args.row_shift);
                println!(
                    "Rendered BMP: {} ({}x{})",
                    summary.output_path.display(),
                    summary.dimensions.width,
                    summary.dimensions.height
                );
            }
            Ok(())
        }
        PabrCommand::ExportRegionsGeojson(args) => {
            let map = load_pabr_map(&args.input_file)?;
            let output_path = normalize_output_path(&args.output)?;
            let summary = map.export_regions_geojson(&output_path, args.row_shift)?;

            if !args.quiet {
                println!("{VERSION_HEADER}");
                println!("RID file: {}", map.rid_path.display());
                println!("BKD file: {}", map.bkd_path.display());
                println!("Row shift: {}", args.row_shift);
                println!(
                    "Exported regions GeoJSON: {} (features={}, rectangles={})",
                    summary.output_path.display(),
                    summary.feature_count,
                    summary.rectangle_count
                );
            }
            Ok(())
        }
        PabrCommand::ExportRegionGroupsGeojson(args) => {
            let map = load_pabr_map(&args.input_file)?;
            let output_path = normalize_output_path(&args.output)?;
            let regioninfo_path = canonical_existing_path(&args.regioninfo)?;
            let mapping = RegionGroupMapping::from_regioninfo_path(&regioninfo_path)?;
            let summary =
                map.export_region_groups_geojson(&output_path, args.row_shift, &mapping)?;

            if !args.quiet {
                println!("{VERSION_HEADER}");
                println!("RID file: {}", map.rid_path.display());
                println!("BKD file: {}", map.bkd_path.display());
                println!("Region info: {}", regioninfo_path.display());
                println!("Row shift: {}", args.row_shift);
                println!(
                    "Exported region-groups GeoJSON: {} (features={}, rectangles={})",
                    summary.output_path.display(),
                    summary.feature_count,
                    summary.rectangle_count
                );
            }
            Ok(())
        }
        PabrCommand::MatchRegions(args) => {
            let map = load_pabr_map(&args.input_file)?;
            let output_path = normalize_output_path(&args.output)?;
            let current_regions_path = canonical_existing_path(&args.current_regions)?;
            let summary = map.match_regions_geojson(
                &current_regions_path,
                &output_path,
                args.row_shift,
                args.top,
            )?;

            if !args.quiet {
                println!("{VERSION_HEADER}");
                println!("RID file: {}", map.rid_path.display());
                println!("BKD file: {}", map.bkd_path.display());
                println!("Current regions: {}", current_regions_path.display());
                println!("Row shift: {}", args.row_shift);
                println!("Top candidates per region: {}", args.top);
                println!(
                    "Matched regions report: {} (pabr={}, current={}, overlap_pairs={}, pabr_only={}, current_only={}, mutual_best={})",
                    summary.output_path.display(),
                    summary.pabr_region_count,
                    summary.current_region_count,
                    summary.overlap_pair_count,
                    summary.pabr_only_count,
                    summary.current_only_count,
                    summary.mutual_best_match_count,
                );
            }
            Ok(())
        }
    }
}

fn run_gcdata(cli: GcdataCli) -> Result<()> {
    match cli.command {
        GcdataCommand::CompareRegionSources(args) => {
            let rid_path = canonical_existing_path(&args.rid)?;
            let current_regions_path = canonical_existing_path(&args.current_regions)?;
            let current_regioninfo_path = args
                .current_regioninfo
                .as_ref()
                .map(canonical_existing_path)
                .transpose()?;
            let regioninfo_bss_path = args
                .regioninfo_bss
                .as_ref()
                .map(canonical_existing_path)
                .transpose()?;
            let regionclientdata_paths = args
                .regionclientdata
                .iter()
                .map(canonical_existing_path)
                .collect::<Result<Vec<_>>>()?;
            let output_path = normalize_output_path(&args.output)?;
            let summary = compare_region_sources(
                &rid_path,
                &current_regions_path,
                args.row_shift,
                current_regioninfo_path.as_deref(),
                regioninfo_bss_path.as_deref(),
                &regionclientdata_paths,
                &output_path,
            )?;

            if !args.quiet {
                println!("{VERSION_HEADER}");
                println!("RID file: {}", rid_path.display());
                println!("Current regions: {}", current_regions_path.display());
                println!("Row shift: {}", args.row_shift);
                if let Some(path) = current_regioninfo_path.as_ref() {
                    println!("Current regioninfo: {}", path.display());
                }
                if let Some(path) = regioninfo_bss_path.as_ref() {
                    println!("Original regioninfo.bss: {}", path.display());
                }
                println!(
                    "Regionclientdata variants: {}",
                    regionclientdata_paths
                        .iter()
                        .map(|path| path.display().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                println!(
                    "Compared region sources: {} (rid_dictionary={}, bkd_referenced={}, active_pabr={}, current={}, current_regioninfo={}, unresolved={})",
                    summary.output_path.display(),
                    summary.rid_dictionary_region_count,
                    summary.pabr_used_region_count,
                    summary.pabr_active_region_count,
                    summary.current_region_count,
                    summary
                        .current_regioninfo_count
                        .map(|count| count.to_string())
                        .unwrap_or_else(|| "n/a".to_string()),
                    summary.unresolved_region_count,
                );
            }
            Ok(())
        }
        GcdataCommand::InspectRegionclientdata(args) => {
            let input_files = args
                .input_files
                .iter()
                .map(canonical_existing_path)
                .collect::<Result<Vec<_>>>()?;
            let (variants, summary) =
                inspect_regionclientdata(&input_files, &args.focus_region_ids)?;

            println!("{VERSION_HEADER}");
            println!("Regionclientdata variants: {}", summary.variant_count);
            println!(
                "Unique region IDs across variants: {}",
                summary.total_unique_region_ids
            );
            if !args.focus_region_ids.is_empty() {
                println!(
                    "Focus region IDs present across variants: {}",
                    summary.focus_region_count
                );
            }
            for variant in variants {
                println!(
                    "{}: {} ids{}",
                    variant.variant,
                    variant.region_count,
                    format_focus_ids(&variant.focus_region_ids)
                );
            }
            Ok(())
        }
        GcdataCommand::InspectPabrTable(args) => {
            let input_path = canonical_existing_path(&args.input_file)?;
            let summary = inspect_pabr_table(&input_path)?;

            println!("{VERSION_HEADER}");
            println!("PABR table: {}", summary.path.display());
            println!("Entry count: {}", summary.entry_count);
            println!("File size: {}", summary.file_size);
            Ok(())
        }
        GcdataCommand::InspectRegioninfoBss(args) => {
            let input_path = canonical_existing_path(&args.input_file)?;
            let loc_path = args.loc.as_ref().map(canonical_existing_path).transpose()?;
            let current_regioninfo_path = args
                .current_regioninfo
                .as_ref()
                .map(canonical_existing_path)
                .transpose()?;
            let output_path = args
                .output
                .as_ref()
                .map(normalize_output_path)
                .transpose()?;
            let summary = inspect_regioninfo_bss(
                &input_path,
                loc_path.as_deref(),
                current_regioninfo_path.as_deref(),
                &args.focus_region_ids,
                output_path.as_deref(),
            )?;

            println!("{VERSION_HEADER}");
            println!("regioninfo.bss: {}", input_path.display());
            if let Some(path) = loc_path.as_ref() {
                println!("Localization: {}", path.display());
            }
            if let Some(path) = current_regioninfo_path.as_ref() {
                println!("Current regioninfo: {}", path.display());
            }
            println!("Header entry count: {}", summary.header_entry_count);
            println!(
                "Decoded signature-family rows: {}",
                summary.decoded_signature_row_count
            );
            println!("Focus rows decoded: {}", summary.focus_row_count);
            println!("Missing focus rows: {}", summary.missing_focus_row_count);
            if let Some(path) = summary.output_path.as_ref() {
                println!("Report: {}", path.display());
            }
            Ok(())
        }
        GcdataCommand::InspectRegiongroupinfoBss(args) => {
            let input_path = canonical_existing_path(&args.input_file)?;
            let current_deck_rg_graphs_path = args
                .current_deck_rg_graphs
                .as_ref()
                .map(canonical_existing_path)
                .transpose()?;
            let output_path = args
                .output
                .as_ref()
                .map(normalize_output_path)
                .transpose()?;
            let summary = inspect_regiongroupinfo_bss(
                &input_path,
                current_deck_rg_graphs_path.as_deref(),
                &args.focus_group_ids,
                output_path.as_deref(),
            )?;

            println!("{VERSION_HEADER}");
            println!("regiongroupinfo.bss: {}", input_path.display());
            if let Some(path) = current_deck_rg_graphs_path.as_ref() {
                println!("Current deck_rg_graphs: {}", path.display());
            }
            println!("Header entry count: {}", summary.header_entry_count);
            println!(
                "Decoded nonzero group rows: {}",
                summary.decoded_group_row_count
            );
            println!(
                "Blank placeholder rows: {}",
                summary.blank_placeholder_row_count
            );
            println!("Focus rows decoded: {}", summary.focus_row_count);
            println!("Missing focus rows: {}", summary.missing_focus_row_count);
            if let Some(current_group_count) = summary.current_group_count {
                println!("Current external group rows: {}", current_group_count);
            }
            if let Some(original_only_group_count) = summary.original_only_group_count {
                println!("Original-only group IDs: {}", original_only_group_count);
            }
            if let Some(current_only_group_count) = summary.current_only_group_count {
                println!("Current-only group IDs: {}", current_only_group_count);
            }
            if let Some(path) = summary.output_path.as_ref() {
                println!("Report: {}", path.display());
            }
            Ok(())
        }
        GcdataCommand::InspectArraywaypointBin(args) => {
            let input_path = canonical_existing_path(&args.input_file)?;
            let waypoints_path = args
                .waypoints
                .as_ref()
                .map(canonical_existing_path)
                .transpose()?;
            let output_path = args
                .output
                .as_ref()
                .map(normalize_output_path)
                .transpose()?;
            let preview_bmp_path = args
                .preview_bmp
                .as_ref()
                .map(normalize_output_path)
                .transpose()?;
            let summary = inspect_arraywaypoint_bin(
                &input_path,
                waypoints_path.as_deref(),
                &args.focus_waypoint_ids,
                output_path.as_deref(),
                preview_bmp_path.as_deref(),
            )?;

            println!("{VERSION_HEADER}");
            println!("mapdata_arraywaypoint.bin: {}", input_path.display());
            if let Some(path) = waypoints_path.as_ref() {
                println!("Current waypoints: {}", path.display());
            }
            println!(
                "Sector bounds: x=[{}, {}), y=[{}, {}), z=[{}, {})",
                summary.min_x_sector,
                summary.max_x_sector,
                summary.min_y_sector,
                summary.max_y_sector,
                summary.min_z_sector,
                summary.max_z_sector,
            );
            println!(
                "Grid size: {}x{} microcells ({}x{} sectors)",
                summary.grid_width,
                summary.grid_height,
                summary.sector_width,
                summary.sector_height,
            );
            println!("Unique values: {}", summary.unique_value_count);
            println!(
                "Uniform macro blocks: {} / {}",
                summary.uniform_block_count, summary.block_count
            );
            if let Some(total_waypoints) = summary.total_waypoint_count {
                println!(
                    "Waypoints inside decoded bounds: {} / {}",
                    summary.waypoints_inside_bounds, total_waypoints
                );
            }
            println!(
                "Focus waypoint samples: {}",
                summary.focus_waypoint_sample_count
            );
            if let Some(path) = summary.output_path.as_ref() {
                println!("Report: {}", path.display());
            }
            if let Some(path) = summary.preview_bmp_path.as_ref() {
                println!("Preview BMP: {}", path.display());
            }
            Ok(())
        }
        GcdataCommand::InspectStringtableBss(args) => {
            let input_path = canonical_existing_path(&args.input_file)?;
            let output_path = args
                .output
                .as_ref()
                .map(normalize_output_path)
                .transpose()?;
            let summary = inspect_stringtable_bss(
                &input_path,
                &args.focus_string_ids,
                output_path.as_deref(),
            )?;

            println!("{VERSION_HEADER}");
            println!("stringtable.bss: {}", input_path.display());
            println!("Header section count: {}", summary.header_section_count);
            println!("Total index rows: {}", summary.total_index_row_count);
            println!("Decoded text entries: {}", summary.text_entry_count);
            println!("Focus entries decoded: {}", summary.focus_entry_count);
            println!(
                "Missing focus entries: {}",
                summary.missing_focus_entry_count
            );
            if let Some(path) = summary.output_path.as_ref() {
                println!("Report: {}", path.display());
            }
            Ok(())
        }
        GcdataCommand::InspectWaypointXml(args) => {
            let input_path = canonical_existing_path(&args.input_file)?;
            let output_path = args
                .output
                .as_ref()
                .map(normalize_output_path)
                .transpose()?;
            let summary = inspect_waypoint_xml(
                &input_path,
                &args.focus_waypoint_ids,
                output_path.as_deref(),
            )?;

            println!("{VERSION_HEADER}");
            println!("Waypoint XML: {}", input_path.display());
            println!("Waypoints: {}", summary.waypoint_count);
            println!("Links: {}", summary.link_count);
            println!("Focus waypoints decoded: {}", summary.focus_waypoint_count);
            println!(
                "Missing focus waypoints: {}",
                summary.missing_focus_waypoint_count
            );
            if let Some(path) = summary.output_path.as_ref() {
                println!("Report: {}", path.display());
            }
            Ok(())
        }
    }
}

fn load_pabr_map(input_file: &PathBuf) -> Result<PabrMap> {
    let input_path = canonical_existing_path(input_file)?;
    let (rid_path, bkd_path) = PabrMap::paired_paths(&input_path)?;
    let rid_path = canonical_existing_path(&rid_path)?;
    let bkd_path = canonical_existing_path(&bkd_path)?;
    PabrMap::from_paths(&rid_path, &bkd_path)
}

fn format_filters(filters: &[String]) -> String {
    if filters.is_empty() {
        String::new()
    } else {
        filters.join(", ")
    }
}

fn format_focus_ids(region_ids: &[u32]) -> String {
    if region_ids.is_empty() {
        String::new()
    } else {
        format!(
            " [{}]",
            region_ids
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

fn format_hex_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn canonical_existing_path(path: &PathBuf) -> Result<PathBuf> {
    let absolute = normalize_output_path(path)?;
    if !absolute.exists() {
        bail!("{} doesn't exist", absolute.display());
    }
    absolute
        .canonicalize()
        .with_context(|| format!("failed to canonicalize {}", absolute.display()))
}

fn normalize_output_path(path: &PathBuf) -> Result<PathBuf> {
    if path.exists() {
        path.canonicalize()
            .with_context(|| format!("failed to canonicalize {}", path.display()))
    } else if path.is_absolute() {
        Ok(path.clone())
    } else {
        Ok(std::env::current_dir()
            .context("failed to read current directory")?
            .join(path))
    }
}
