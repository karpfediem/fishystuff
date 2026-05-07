mod dolt;
mod gitops;
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
    Gitops {
        #[command(subcommand)]
        command: GitopsCommands,
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
    ProbeJsonScalars {
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        status: PathBuf,
    },
    NeedsProbeJsonScalars {
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        status: PathBuf,
    },
}

#[derive(Subcommand)]
enum GitopsCommands {
    CheckServed {
        #[arg(long)]
        status: PathBuf,
        #[arg(long)]
        active: PathBuf,
        #[arg(long)]
        rollback_set: PathBuf,
        #[arg(long)]
        rollback: PathBuf,
        #[arg(long)]
        environment: Option<String>,
        #[arg(long)]
        host: Option<String>,
        #[arg(long)]
        release_id: Option<String>,
    },
    SummaryServed {
        #[arg(long)]
        status: PathBuf,
        #[arg(long)]
        active: PathBuf,
        #[arg(long)]
        rollback_set: PathBuf,
        #[arg(long)]
        rollback: PathBuf,
        #[arg(long)]
        environment: Option<String>,
        #[arg(long)]
        host: Option<String>,
        #[arg(long)]
        release_id: Option<String>,
    },
    InspectServed {
        #[arg(long)]
        status: PathBuf,
        #[arg(long)]
        active: PathBuf,
        #[arg(long)]
        rollback_set: PathBuf,
        #[arg(long)]
        rollback: PathBuf,
        #[arg(long)]
        admission: Option<PathBuf>,
        #[arg(long)]
        route: Option<PathBuf>,
        #[arg(long)]
        roots_dir: Option<PathBuf>,
        #[arg(long)]
        environment: Option<String>,
        #[arg(long)]
        host: Option<String>,
        #[arg(long)]
        release_id: Option<String>,
    },
    RootsReady {
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        status: PathBuf,
    },
    NeedsRootsReady {
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
            HttpCommands::ProbeJsonScalars { request, status } => {
                http_probe::probe_json_scalars(&request, &status)?;
                Ok(ExitCode::SUCCESS)
            }
            HttpCommands::NeedsProbeJsonScalars { request, status } => Ok(needs_exit_code(
                http_probe::needs_probe_json_scalars(&request, &status),
            )),
        },
        Commands::Gitops { command } => match command {
            GitopsCommands::CheckServed {
                status,
                active,
                rollback_set,
                rollback,
                environment,
                host,
                release_id,
            } => {
                let summary = gitops::check_served(
                    &status,
                    &active,
                    &rollback_set,
                    &rollback,
                    environment.as_deref(),
                    host.as_deref(),
                    release_id.as_deref(),
                )?;
                println!("{}", summary.summary_line());
                Ok(ExitCode::SUCCESS)
            }
            GitopsCommands::SummaryServed {
                status,
                active,
                rollback_set,
                rollback,
                environment,
                host,
                release_id,
            } => {
                let summary = gitops::check_served(
                    &status,
                    &active,
                    &rollback_set,
                    &rollback,
                    environment.as_deref(),
                    host.as_deref(),
                    release_id.as_deref(),
                )?;
                print!("{}", summary.operator_summary());
                Ok(ExitCode::SUCCESS)
            }
            GitopsCommands::InspectServed {
                status,
                active,
                rollback_set,
                rollback,
                admission,
                route,
                roots_dir,
                environment,
                host,
                release_id,
            } => {
                let inspection = gitops::inspect_served(
                    &status,
                    &active,
                    &rollback_set,
                    &rollback,
                    admission.as_deref(),
                    route.as_deref(),
                    roots_dir.as_deref(),
                    environment.as_deref(),
                    host.as_deref(),
                    release_id.as_deref(),
                )?;
                print!("{}", inspection.operator_summary());
                Ok(ExitCode::SUCCESS)
            }
            GitopsCommands::RootsReady { request, status } => {
                gitops::roots_ready(&request, &status)?;
                Ok(ExitCode::SUCCESS)
            }
            GitopsCommands::NeedsRootsReady { request, status } => Ok(needs_exit_code(
                gitops::needs_roots_ready(&request, &status),
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
