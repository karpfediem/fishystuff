mod dolt;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "fishystuff_deploy")]
#[command(about = "FishyStuff deployment helper tools")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Dolt {
        #[command(subcommand)]
        command: DoltCommands,
    },
}

#[derive(Subcommand)]
enum DoltCommands {
    FetchPin {
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        status: PathBuf,
        #[arg(long, default_value = "dolt")]
        dolt_bin: PathBuf,
    },
    ProbeSqlFixture {
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        status: PathBuf,
        #[arg(long, default_value = "dolt")]
        dolt_bin: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Dolt { command } => match command {
            DoltCommands::FetchPin {
                request,
                status,
                dolt_bin,
            } => dolt::fetch_pin(&request, &status, &dolt_bin),
            DoltCommands::ProbeSqlFixture {
                request,
                status,
                dolt_bin,
            } => dolt::probe_sql_fixture(&request, &status, &dolt_bin),
        },
    }
}
