mod archive;
mod compression;
mod ice;
mod pabr;
mod wildcard;

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use archive::{ArchiveIndex, ExtractOptions};
use clap::{ArgAction, CommandFactory, Parser, Subcommand};
use pabr::PabrMap;

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
    #[command(about = "Inspect or render .rid/.bkd PABR region-map pairs")]
    Pabr(PabrCli),
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

#[derive(Debug, Subcommand)]
enum PabrCommand {
    #[command(about = "Print structural metadata for a PABR .rid/.bkd pair")]
    Inspect(PabrInputCli),
    #[command(about = "Render a deterministic BMP visualization from a PABR .rid/.bkd pair")]
    Render(PabrRenderCli),
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
        Some(Command::Pabr(pabr_cli)) => run_pabr(pabr_cli),
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
            let summary = map.render_bmp(&output_path, dimensions)?;

            if !args.quiet {
                println!("{VERSION_HEADER}");
                println!("RID file: {}", map.rid_path.display());
                println!("BKD file: {}", map.bkd_path.display());
                println!(
                    "Rendered BMP: {} ({}x{})",
                    summary.output_path.display(),
                    summary.dimensions.width,
                    summary.dimensions.height
                );
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
