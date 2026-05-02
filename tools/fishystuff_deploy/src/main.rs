mod dolt;

use std::path::PathBuf;
use std::process::ExitCode;

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
    NeedsFetchPin {
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        status: PathBuf,
        #[arg(long, default_value = "dolt")]
        dolt_bin: PathBuf,
    },
    ProbeSqlScalar {
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        status: PathBuf,
        #[arg(long, default_value = "dolt")]
        dolt_bin: PathBuf,
    },
    NeedsProbeSqlScalar {
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        status: PathBuf,
        #[arg(long, default_value = "dolt")]
        dolt_bin: PathBuf,
    },
}

fn main() -> Result<ExitCode> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Dolt { command } => match command {
            DoltCommands::FetchPin {
                request,
                status,
                dolt_bin,
            } => {
                dolt::fetch_pin(&request, &status, &dolt_bin)?;
                Ok(ExitCode::SUCCESS)
            }
            DoltCommands::NeedsFetchPin {
                request,
                status,
                dolt_bin,
            } => Ok(needs_exit_code(dolt::needs_fetch_pin(
                &request, &status, &dolt_bin,
            ))),
            DoltCommands::ProbeSqlScalar {
                request,
                status,
                dolt_bin,
            } => {
                dolt::probe_sql_scalar(&request, &status, &dolt_bin)?;
                Ok(ExitCode::SUCCESS)
            }
            DoltCommands::NeedsProbeSqlScalar {
                request,
                status,
                dolt_bin,
            } => Ok(needs_exit_code(dolt::needs_probe_sql_scalar(
                &request, &status, &dolt_bin,
            ))),
        },
    }
}

fn needs_exit_code(needs_run: bool) -> ExitCode {
    if needs_run {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
