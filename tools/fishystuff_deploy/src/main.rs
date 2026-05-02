mod dolt;
mod http_probe;

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
    Http {
        #[command(subcommand)]
        command: HttpCommands,
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

#[derive(Subcommand)]
enum HttpCommands {
    ProbeStatus {
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        status: PathBuf,
    },
    NeedsProbeStatus {
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        status: PathBuf,
    },
    ProbeJsonScalar {
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        status: PathBuf,
    },
    NeedsProbeJsonScalar {
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        status: PathBuf,
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
        Commands::Http { command } => match command {
            HttpCommands::ProbeStatus { request, status } => {
                http_probe::probe_status(&request, &status)?;
                Ok(ExitCode::SUCCESS)
            }
            HttpCommands::NeedsProbeStatus { request, status } => Ok(needs_exit_code(
                http_probe::needs_probe_status(&request, &status),
            )),
            HttpCommands::ProbeJsonScalar { request, status } => {
                http_probe::probe_json_scalar(&request, &status)?;
                Ok(ExitCode::SUCCESS)
            }
            HttpCommands::NeedsProbeJsonScalar { request, status } => Ok(needs_exit_code(
                http_probe::needs_probe_json_scalar(&request, &status),
            )),
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
