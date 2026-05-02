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

#[derive(Debug, Deserialize)]
struct ProbeSqlScalarRequest {
    environment: String,
    host: String,
    release_id: String,
    release_identity: String,
    expected_commit: String,
    materialization: String,
    cache_dir: PathBuf,
    pinned_ref: String,
    materialization_status_path: PathBuf,
    query: String,
    expected_scalar: String,
}

#[derive(Debug, Deserialize)]
struct FetchPinMaterializationStatus {
    verified_commit: String,
    cache_dir: PathBuf,
    release_ref: String,
    state: String,
}

#[derive(Debug, Serialize)]
struct ProbeSqlScalarStatus<'a> {
    environment: &'a str,
    host: &'a str,
    release_id: &'a str,
    release_identity: &'a str,
    expected_commit: &'a str,
    materialization: &'a str,
    cache_dir: &'a Path,
    pinned_ref: &'a str,
    materialization_status_path: &'a Path,
    verified_commit: &'a str,
    query: &'a str,
    scalar: &'a str,
    expected_scalar: &'a str,
    admission_state: &'static str,
    probe: &'static str,
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

pub fn probe_sql_scalar(request_path: &Path, status_path: &Path, dolt_bin: &Path) -> Result<()> {
    let request = read_probe_sql_scalar_request(request_path)?;
    validate_probe_sql_scalar_request(&request)?;
    ensure_parent_dir(status_path)?;

    let materialization_status = read_materialization_status(&request.materialization_status_path)?;
    validate_materialization_status(&request, &materialization_status)?;

    let verified_commit = verify_probe_ref(&request, dolt_bin)?;
    if verified_commit != request.expected_commit {
        bail!(
            "expected Dolt commit {}, got {} for ref {}",
            request.expected_commit,
            verified_commit,
            request.pinned_ref
        );
    }

    let scalar_value = run_probe_sql(&request, dolt_bin)?;
    if scalar_value != request.expected_scalar {
        bail!(
            "Dolt SQL scalar probe expected scalar {:?}, got {:?}",
            request.expected_scalar,
            scalar_value
        );
    }

    write_probe_sql_scalar_status(status_path, &request, &verified_commit, &scalar_value)
}

fn read_request(request_path: &Path) -> Result<FetchPinRequest> {
    let file = File::open(request_path)
        .with_context(|| format!("opening fetch-pin request {}", request_path.display()))?;
    serde_json::from_reader(file)
        .with_context(|| format!("decoding fetch-pin request {}", request_path.display()))
}

fn read_probe_sql_scalar_request(request_path: &Path) -> Result<ProbeSqlScalarRequest> {
    let file = File::open(request_path).with_context(|| {
        format!(
            "opening Dolt SQL scalar admission request {}",
            request_path.display()
        )
    })?;
    serde_json::from_reader(file).with_context(|| {
        format!(
            "decoding Dolt SQL scalar admission request {}",
            request_path.display()
        )
    })
}

fn read_materialization_status(status_path: &Path) -> Result<FetchPinMaterializationStatus> {
    let file = File::open(status_path).with_context(|| {
        format!(
            "opening Dolt materialization status {}",
            status_path.display()
        )
    })?;
    serde_json::from_reader(file).with_context(|| {
        format!(
            "decoding Dolt materialization status {}",
            status_path.display()
        )
    })
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

fn validate_probe_sql_scalar_request(request: &ProbeSqlScalarRequest) -> Result<()> {
    require_non_empty("environment", &request.environment)?;
    require_non_empty("host", &request.host)?;
    require_non_empty("release_id", &request.release_id)?;
    require_non_empty("release_identity", &request.release_identity)?;
    require_non_empty("expected_commit", &request.expected_commit)?;
    require_non_empty("materialization", &request.materialization)?;
    require_non_empty("pinned_ref", &request.pinned_ref)?;
    require_non_empty("query", &request.query)?;

    if request.materialization != "fetch_pin" {
        bail!(
            "Dolt SQL scalar admission requires fetch_pin materialization, got {}",
            request.materialization
        );
    }
    if request.cache_dir.as_os_str().is_empty() {
        bail!("Dolt SQL scalar request field cache_dir must not be empty");
    }
    if request.materialization_status_path.as_os_str().is_empty() {
        bail!("Dolt SQL scalar request field materialization_status_path must not be empty");
    }

    Ok(())
}

fn validate_materialization_status(
    request: &ProbeSqlScalarRequest,
    status: &FetchPinMaterializationStatus,
) -> Result<()> {
    if status.state != "pinned" {
        bail!(
            "Dolt materialization status is {}, expected pinned",
            status.state
        );
    }
    if status.verified_commit != request.expected_commit {
        bail!(
            "Dolt materialization status verified commit {} does not match requested {}",
            status.verified_commit,
            request.expected_commit
        );
    }
    if status.cache_dir != request.cache_dir {
        bail!(
            "Dolt materialization cache {} does not match admission cache {}",
            status.cache_dir.display(),
            request.cache_dir.display()
        );
    }
    if status.release_ref != request.pinned_ref {
        bail!(
            "Dolt materialization release ref {} does not match admission release ref {}",
            status.release_ref,
            request.pinned_ref
        );
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

fn verify_probe_ref(request: &ProbeSqlScalarRequest, dolt_bin: &Path) -> Result<String> {
    let sql = format!(
        "select dolt_hashof('{}') as hash",
        sql_quote(&request.pinned_ref)
    );
    let stdout = run_dolt(
        dolt_bin,
        Some(&request.cache_dir),
        ["sql", "-r", "csv", "-q", sql.as_str()],
    )?;
    parse_single_hash_csv(&stdout)
}

fn run_probe_sql(request: &ProbeSqlScalarRequest, dolt_bin: &Path) -> Result<String> {
    let stdout = run_dolt(
        dolt_bin,
        Some(&request.cache_dir),
        ["sql", "-r", "csv", "-q", request.query.as_str()],
    )?;
    parse_single_scalar_csv(&stdout)
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

fn write_probe_sql_scalar_status(
    status_path: &Path,
    request: &ProbeSqlScalarRequest,
    verified_commit: &str,
    scalar_value: &str,
) -> Result<()> {
    let status = ProbeSqlScalarStatus {
        environment: &request.environment,
        host: &request.host,
        release_id: &request.release_id,
        release_identity: &request.release_identity,
        expected_commit: &request.expected_commit,
        materialization: &request.materialization,
        cache_dir: &request.cache_dir,
        pinned_ref: &request.pinned_ref,
        materialization_status_path: &request.materialization_status_path,
        verified_commit,
        query: &request.query,
        scalar: scalar_value,
        expected_scalar: &request.expected_scalar,
        admission_state: "passed_fixture",
        probe: "dolt-sql-scalar",
    };

    write_json_status(status_path, &status)
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

    write_json_status(status_path, &status)
}

fn write_json_status<T>(status_path: &Path, status: &T) -> Result<()>
where
    T: Serialize,
{
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

fn parse_single_scalar_csv(stdout: &str) -> Result<String> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(stdout.as_bytes());
    let headers = reader
        .headers()
        .context("Dolt SQL CSV output had no header")?;
    if headers.len() != 1 {
        bail!(
            "Dolt SQL scalar probe expected one result column, got {}",
            headers.len()
        );
    }

    let mut records = reader.records();
    let record = records
        .next()
        .context("Dolt SQL CSV output had no row")?
        .context("reading Dolt SQL CSV row")?;
    if record.len() != 1 {
        bail!(
            "Dolt SQL scalar probe expected one scalar field, got {}",
            record.len()
        );
    }
    if records.next().is_some() {
        bail!("Dolt SQL scalar probe returned more than one row");
    }
    Ok(record.get(0).unwrap_or_default().to_owned())
}

#[cfg(test)]
mod tests {
    use super::{parse_single_hash_csv, parse_single_scalar_csv, sql_quote};

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

    #[test]
    fn parses_single_scalar_csv() {
        let parsed = parse_single_scalar_csv("value\none\n").unwrap();
        assert_eq!(parsed, "one");
    }

    #[test]
    fn rejects_non_scalar_csv() {
        assert!(parse_single_scalar_csv("a,b\none,two\n").is_err());
        assert!(parse_single_scalar_csv("value\none\ntwo\n").is_err());
        assert!(parse_single_scalar_csv("value\n").is_err());
    }
}
