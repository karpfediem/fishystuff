mod archive;
mod compression;
mod ice;
mod wildcard;

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use archive::{ArchiveIndex, ExtractOptions};
use clap::ArgAction;
use clap::Parser;

const VERSION_HEADER: &str = "pazifista - tool for extracting Black Desert Online archives.";

#[derive(Debug, Parser)]
#[command(
    name = "pazifista",
    about = "Extract Black Desert Online .meta and .paz archives",
    arg_required_else_help = true
)]
struct Cli {
    #[arg(value_name = "input file")]
    input_file: PathBuf,
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

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let source_path = canonical_existing_path(&cli.input_file)?;
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
