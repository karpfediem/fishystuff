use std::ffi::OsString;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct FetchPinRequest {
    environment: String,
    host: String,
    release_id: String,
    release_identity: String,
    repository: String,
    remote_url: String,
    branch_context: String,
    commit: String,
    access_mode: String,
    materialization: String,
    cache_dir: PathBuf,
    release_ref: String,
}

#[derive(Debug, Serialize)]
struct FetchPinStatus<'a> {
    environment: &'a str,
    host: &'a str,
    release_id: &'a str,
    release_identity: &'a str,
    repository: &'a str,
    remote_url: &'a str,
    branch_context: &'a str,
    requested_commit: &'a str,
    verified_commit: &'a str,
    access_mode: &'a str,
    materialization: &'a str,
    cache_dir: &'a Path,
    release_ref: &'a str,
    state: &'static str,
}

pub fn fetch_pin(request_path: &Path, status_path: &Path, dolt_bin: &Path) -> Result<()> {
    let request = read_request(request_path)?;
    validate_request(&request)?;

    ensure_parent_dir(&request.cache_dir)?;
    ensure_parent_dir(status_path)?;

    if !request.cache_dir.join(".dolt").is_dir() {
        clone_cache(&request, dolt_bin)?;
    }

    run_dolt(
        dolt_bin,
        Some(&request.cache_dir),
        ["fetch", "origin", request.branch_context.as_str()],
    )?;
    run_dolt(
        dolt_bin,
        Some(&request.cache_dir),
        [
            "branch",
            "-f",
            request.release_ref.as_str(),
            request.commit.as_str(),
        ],
    )?;

    let verified_commit = verify_ref(&request, dolt_bin)?;
    if verified_commit != request.commit {
        bail!(
            "expected Dolt commit {}, got {} for ref {}",
            request.commit,
            verified_commit,
            request.release_ref
        );
    }

    write_status(status_path, &request, &verified_commit)?;
    Ok(())
}

fn read_request(request_path: &Path) -> Result<FetchPinRequest> {
    let file = File::open(request_path)
        .with_context(|| format!("opening fetch-pin request {}", request_path.display()))?;
    serde_json::from_reader(file)
        .with_context(|| format!("decoding fetch-pin request {}", request_path.display()))
}

fn validate_request(request: &FetchPinRequest) -> Result<()> {
    require_non_empty("environment", &request.environment)?;
    require_non_empty("host", &request.host)?;
    require_non_empty("release_id", &request.release_id)?;
    require_non_empty("release_identity", &request.release_identity)?;
    require_non_empty("repository", &request.repository)?;
    require_non_empty("remote_url", &request.remote_url)?;
    require_non_empty("branch_context", &request.branch_context)?;
    require_non_empty("commit", &request.commit)?;
    require_non_empty("access_mode", &request.access_mode)?;
    require_non_empty("materialization", &request.materialization)?;
    require_non_empty("release_ref", &request.release_ref)?;

    if request.materialization != "fetch_pin" {
        bail!(
            "unsupported Dolt materialization {}; expected fetch_pin",
            request.materialization
        );
    }

    if request.cache_dir.as_os_str().is_empty() {
        bail!("fetch-pin request field cache_dir must not be empty");
    }

    Ok(())
}

fn require_non_empty(field: &str, value: &str) -> Result<()> {
    if value.is_empty() {
        bail!("fetch-pin request field {field} must not be empty");
    }
    Ok(())
}

fn ensure_parent_dir(path: &Path) -> Result<()> {
    let parent = path
        .parent()
        .with_context(|| format!("{} has no parent directory", path.display()))?;
    fs::create_dir_all(parent)
        .with_context(|| format!("creating parent directory {}", parent.display()))
}

fn clone_cache(request: &FetchPinRequest, dolt_bin: &Path) -> Result<()> {
    let tmp_cache = temporary_cache_path(&request.cache_dir);
    if tmp_cache.exists() {
        fs::remove_dir_all(&tmp_cache)
            .with_context(|| format!("removing stale temp Dolt cache {}", tmp_cache.display()))?;
    }

    let clone_result = run_dolt_os(
        dolt_bin,
        None,
        [
            OsString::from("clone"),
            OsString::from("--branch"),
            OsString::from(&request.branch_context),
            OsString::from("--single-branch"),
            OsString::from(&request.remote_url),
            tmp_cache.as_os_str().to_os_string(),
        ],
    );
    if clone_result.is_err() {
        let _ = fs::remove_dir_all(&tmp_cache);
    }
    clone_result?;

    if request.cache_dir.exists() {
        fs::remove_dir_all(&request.cache_dir).with_context(|| {
            format!(
                "removing non-Dolt cache path before replacement {}",
                request.cache_dir.display()
            )
        })?;
    }
    fs::rename(&tmp_cache, &request.cache_dir).with_context(|| {
        format!(
            "moving temp Dolt cache {} to {}",
            tmp_cache.display(),
            request.cache_dir.display()
        )
    })?;

    Ok(())
}

fn temporary_cache_path(cache_dir: &Path) -> PathBuf {
    let file_name = cache_dir
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| "cache".into());
    cache_dir.with_file_name(format!("{file_name}.tmp.{}", std::process::id()))
}

fn verify_ref(request: &FetchPinRequest, dolt_bin: &Path) -> Result<String> {
    let sql = format!(
        "select dolt_hashof('{}') as hash",
        sql_quote(&request.release_ref)
    );
    let stdout = run_dolt(
        dolt_bin,
        Some(&request.cache_dir),
        ["sql", "-r", "csv", "-q", sql.as_str()],
    )?;
    parse_single_hash_csv(&stdout)
}

fn write_status(
    status_path: &Path,
    request: &FetchPinRequest,
    verified_commit: &str,
) -> Result<()> {
    let status = FetchPinStatus {
        environment: &request.environment,
        host: &request.host,
        release_id: &request.release_id,
        release_identity: &request.release_identity,
        repository: &request.repository,
        remote_url: &request.remote_url,
        branch_context: &request.branch_context,
        requested_commit: &request.commit,
        verified_commit,
        access_mode: &request.access_mode,
        materialization: &request.materialization,
        cache_dir: &request.cache_dir,
        release_ref: &request.release_ref,
        state: "pinned",
    };

    let tmp_status = temporary_status_path(status_path);
    let mut file = File::create(&tmp_status)
        .with_context(|| format!("creating temp status {}", tmp_status.display()))?;
    serde_json::to_writer_pretty(&mut file, &status)
        .with_context(|| format!("writing temp status {}", tmp_status.display()))?;
    file.write_all(b"\n")
        .with_context(|| format!("finalizing temp status {}", tmp_status.display()))?;
    file.sync_all()
        .with_context(|| format!("syncing temp status {}", tmp_status.display()))?;
    drop(file);

    fs::rename(&tmp_status, status_path).with_context(|| {
        format!(
            "moving temp status {} to {}",
            tmp_status.display(),
            status_path.display()
        )
    })?;
    Ok(())
}

fn temporary_status_path(status_path: &Path) -> PathBuf {
    let file_name = status_path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| "status.json".into());
    status_path.with_file_name(format!(".{file_name}.tmp.{}", std::process::id()))
}

fn run_dolt<'a, I>(dolt_bin: &Path, cwd: Option<&Path>, args: I) -> Result<String>
where
    I: IntoIterator<Item = &'a str>,
{
    run_dolt_os(dolt_bin, cwd, args.into_iter().map(OsString::from))
}

fn run_dolt_os<I>(dolt_bin: &Path, cwd: Option<&Path>, args: I) -> Result<String>
where
    I: IntoIterator<Item = OsString>,
{
    let args: Vec<OsString> = args.into_iter().collect();
    let mut command = Command::new(dolt_bin);
    command.args(&args);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    command.stdin(Stdio::null());

    let output = command
        .output()
        .with_context(|| format!("running {} {}", dolt_bin.display(), display_args(&args)))?;
    if !output.status.success() {
        bail!(
            "Dolt command failed with status {}\ncommand: {} {}\nstdout:\n{}\nstderr:\n{}",
            output.status,
            dolt_bin.display(),
            display_args(&args),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    String::from_utf8(output.stdout).context("Dolt command wrote non-UTF-8 stdout")
}

fn display_args(args: &[OsString]) -> String {
    args.iter()
        .map(|arg| arg.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ")
}

fn sql_quote(value: &str) -> String {
    value.replace('\'', "''")
}

fn parse_single_hash_csv(stdout: &str) -> Result<String> {
    let mut lines = stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty());
    let header = lines.next().context("Dolt hash CSV output was empty")?;
    if header != "hash" {
        bail!("expected Dolt hash CSV header `hash`, got `{header}`");
    }

    let hash = lines.next().context("Dolt hash CSV output had no row")?;
    if lines.next().is_some() {
        bail!("Dolt hash CSV output had more than one row");
    }
    Ok(hash.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{parse_single_hash_csv, sql_quote};

    #[test]
    fn parses_dolt_hash_csv() {
        let parsed = parse_single_hash_csv("hash\nabc123\n").unwrap();
        assert_eq!(parsed, "abc123");
    }

    #[test]
    fn rejects_unexpected_hash_csv_shape() {
        assert!(parse_single_hash_csv("hash\nabc123\ndef456\n").is_err());
        assert!(parse_single_hash_csv("other\nabc123\n").is_err());
        assert!(parse_single_hash_csv("hash\n").is_err());
    }

    #[test]
    fn quotes_sql_string_literals() {
        assert_eq!(
            sql_quote("fishystuff/gitops/example"),
            "fishystuff/gitops/example"
        );
        assert_eq!(sql_quote("fishy'branch"), "fishy''branch");
    }
}
