use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

#[test]
fn gitops_check_served_accepts_consistent_rollback_ready_documents() -> Result<()> {
    let root = TestRoot::new("fishystuff-deploy-gitops-served")?;
    let status = root.path().join("status.json");
    let active = root.path().join("active.json");
    let rollback_set = root.path().join("rollback-set.json");
    let rollback = root.path().join("rollback.json");

    write_served_documents(&status, &active, &rollback_set, &rollback)?;

    let output = run_helper_raw([
        "gitops",
        "check-served",
        "--status",
        path_str(&status)?,
        "--active",
        path_str(&active)?,
        "--rollback-set",
        path_str(&rollback_set)?,
        "--rollback",
        path_str(&rollback)?,
        "--environment",
        "local-test",
        "--host",
        "vm-single-host",
        "--release-id",
        "active-release",
    ])?;
    if !output.status.success() {
        return bail_command("fishystuff_deploy gitops check-served", output);
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("served rollback-ready: environment=local-test"));
    assert!(stdout.contains("release_id=active-release"));
    assert!(stdout.contains("rollback_primary=previous-release"));

    Ok(())
}

#[test]
fn gitops_summary_served_prints_active_and_retained_releases() -> Result<()> {
    let root = TestRoot::new("fishystuff-deploy-gitops-summary")?;
    let status = root.path().join("status.json");
    let active = root.path().join("active.json");
    let rollback_set = root.path().join("rollback-set.json");
    let rollback = root.path().join("rollback.json");

    write_served_documents(&status, &active, &rollback_set, &rollback)?;

    let output = run_helper_raw([
        "gitops",
        "summary-served",
        "--status",
        path_str(&status)?,
        "--active",
        path_str(&active)?,
        "--rollback-set",
        path_str(&rollback_set)?,
        "--rollback",
        path_str(&rollback)?,
        "--environment",
        "local-test",
    ])?;
    if !output.status.success() {
        return bail_command("fishystuff_deploy gitops summary-served", output);
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("environment: local-test"));
    assert!(stdout.contains("host: vm-single-host"));
    assert!(stdout.contains("generation: 42"));
    assert!(stdout.contains("served_release: active-release"));
    assert!(stdout.contains("rollback_primary: previous-release"));
    assert!(stdout.contains("rollback_primary_identity: release=previous-release;api=example"));
    assert!(stdout.contains("retained_rollback_releases: previous-release"));
    assert!(stdout.contains("retained_count: 1"));

    Ok(())
}

#[test]
fn gitops_check_served_rejects_missing_rollback_readiness() -> Result<()> {
    let root = TestRoot::new("fishystuff-deploy-gitops-not-ready")?;
    let status = root.path().join("status.json");
    let active = root.path().join("active.json");
    let rollback_set = root.path().join("rollback-set.json");
    let rollback = root.path().join("rollback.json");

    write_served_documents(&status, &active, &rollback_set, &rollback)?;
    let mut stale_status = read_json(&status)?;
    stale_status["rollback_available"] = Value::Bool(false);
    write_json(&status, stale_status)?;

    assert_helper_failure_contains(
        [
            "gitops",
            "check-served",
            "--status",
            path_str(&status)?,
            "--active",
            path_str(&active)?,
            "--rollback-set",
            path_str(&rollback_set)?,
            "--rollback",
            path_str(&rollback)?,
        ],
        "status rollback_available must be true",
    )
}

#[test]
fn gitops_check_served_rejects_cross_document_release_mismatch() -> Result<()> {
    let root = TestRoot::new("fishystuff-deploy-gitops-mismatch")?;
    let status = root.path().join("status.json");
    let active = root.path().join("active.json");
    let rollback_set = root.path().join("rollback-set.json");
    let rollback = root.path().join("rollback.json");

    write_served_documents(&status, &active, &rollback_set, &rollback)?;
    let mut stale_active = read_json(&active)?;
    stale_active["release_id"] = Value::String("other-release".to_string());
    write_json(&active, stale_active)?;

    assert_helper_failure_contains(
        [
            "gitops",
            "check-served",
            "--status",
            path_str(&status)?,
            "--active",
            path_str(&active)?,
            "--rollback-set",
            path_str(&rollback_set)?,
            "--rollback",
            path_str(&rollback)?,
        ],
        "active release_id was other-release, expected active-release",
    )
}

#[test]
fn gitops_check_served_rejects_stale_rollback_primary() -> Result<()> {
    let root = TestRoot::new("fishystuff-deploy-gitops-stale-rollback-primary")?;
    let status = root.path().join("status.json");
    let active = root.path().join("active.json");
    let rollback_set = root.path().join("rollback-set.json");
    let rollback = root.path().join("rollback.json");

    write_served_documents(&status, &active, &rollback_set, &rollback)?;
    let mut stale_status = read_json(&status)?;
    stale_status["rollback_primary_release_id"] = Value::String("older-release".to_string());
    write_json(&status, stale_status)?;

    assert_helper_failure_contains(
        [
            "gitops",
            "check-served",
            "--status",
            path_str(&status)?,
            "--active",
            path_str(&active)?,
            "--rollback-set",
            path_str(&rollback_set)?,
            "--rollback",
            path_str(&rollback)?,
        ],
        "status rollback_primary_release_id was older-release, expected previous-release",
    )
}

#[test]
fn gitops_check_served_rejects_stale_rollback_readiness() -> Result<()> {
    let root = TestRoot::new("fishystuff-deploy-gitops-stale-rollback-readiness")?;
    let status = root.path().join("status.json");
    let active = root.path().join("active.json");
    let rollback_set = root.path().join("rollback-set.json");
    let rollback = root.path().join("rollback.json");

    write_served_documents(&status, &active, &rollback_set, &rollback)?;
    let mut stale_rollback = read_json(&rollback)?;
    stale_rollback["rollback_release_id"] = Value::String("older-release".to_string());
    write_json(&rollback, stale_rollback)?;

    assert_helper_failure_contains(
        [
            "gitops",
            "check-served",
            "--status",
            path_str(&status)?,
            "--active",
            path_str(&active)?,
            "--rollback-set",
            path_str(&rollback_set)?,
            "--rollback",
            path_str(&rollback)?,
        ],
        "rollback rollback_release_id was older-release, expected previous-release",
    )
}

#[test]
fn gitops_check_served_rejects_stale_active_retained_list() -> Result<()> {
    let root = TestRoot::new("fishystuff-deploy-gitops-stale-active-retained")?;
    let status = root.path().join("status.json");
    let active = root.path().join("active.json");
    let rollback_set = root.path().join("rollback-set.json");
    let rollback = root.path().join("rollback.json");

    write_served_documents(&status, &active, &rollback_set, &rollback)?;
    let mut stale_active = read_json(&active)?;
    stale_active["retained_release_ids"] = json!(["older-release"]);
    write_json(&active, stale_active)?;

    assert_helper_failure_contains(
        [
            "gitops",
            "check-served",
            "--status",
            path_str(&status)?,
            "--active",
            path_str(&active)?,
            "--rollback-set",
            path_str(&rollback_set)?,
            "--rollback",
            path_str(&rollback)?,
        ],
        "active retained_release_ids was [\"older-release\"], expected [\"previous-release\"]",
    )
}

#[test]
fn gitops_check_served_rejects_stale_rollback_set_retained_list() -> Result<()> {
    let root = TestRoot::new("fishystuff-deploy-gitops-stale-rollback-set")?;
    let status = root.path().join("status.json");
    let active = root.path().join("active.json");
    let rollback_set = root.path().join("rollback-set.json");
    let rollback = root.path().join("rollback.json");

    write_served_documents(&status, &active, &rollback_set, &rollback)?;
    let mut stale_rollback_set = read_json(&rollback_set)?;
    stale_rollback_set["retained_release_ids"] = json!(["older-release"]);
    write_json(&rollback_set, stale_rollback_set)?;

    assert_helper_failure_contains(
        [
            "gitops",
            "check-served",
            "--status",
            path_str(&status)?,
            "--active",
            path_str(&active)?,
            "--rollback-set",
            path_str(&rollback_set)?,
            "--rollback",
            path_str(&rollback)?,
        ],
        "rollback-set retained_release_ids was [\"older-release\"], expected [\"previous-release\"]",
    )
}

fn write_served_documents(
    status: &Path,
    active: &Path,
    rollback_set: &Path,
    rollback: &Path,
) -> Result<()> {
    write_json(
        status,
        json!({
            "desired_generation": 42,
            "release_id": "active-release",
            "release_identity": "release=active-release;api=example",
            "environment": "local-test",
            "host": "vm-single-host",
            "phase": "served",
            "transition_kind": "activate",
            "rollback_from_release": "",
            "rollback_to_release": "",
            "rollback_reason": "",
            "admission_state": "passed_fixture",
            "dolt_commit": "example",
            "dolt_materialization": "metadata_only",
            "dolt_cache_dir": "",
            "dolt_release_ref": "",
            "retained_release_ids": ["previous-release"],
            "retained_dolt_status_paths": [],
            "rollback_available": true,
            "rollback_primary_release_id": "previous-release",
            "rollback_retained_count": 1,
            "served": true,
            "failure_reason": "",
        }),
    )?;
    write_json(
        active,
        json!({
            "desired_generation": 42,
            "environment": "local-test",
            "host": "vm-single-host",
            "release_id": "active-release",
            "release_identity": "release=active-release;api=example",
            "instance_name": "local-test-active-release",
            "site_content": "/nix/store/example-site",
            "cdn_runtime_content": "/nix/store/example-cdn",
            "api_upstream": "http://127.0.0.1:18082",
            "site_link": "/var/lib/fishystuff/gitops/served/local-test/site",
            "cdn_link": "/var/lib/fishystuff/gitops/served/local-test/cdn",
            "retained_release_ids": ["previous-release"],
            "retained_dolt_status_paths": [],
            "transition_kind": "activate",
            "rollback_from_release": "",
            "rollback_to_release": "",
            "rollback_reason": "",
            "admission_state": "passed_fixture",
            "served": true,
            "route_state": "selected_local_symlinks",
        }),
    )?;
    write_json(
        rollback_set,
        json!({
            "desired_generation": 42,
            "environment": "local-test",
            "host": "vm-single-host",
            "current_release_id": "active-release",
            "current_release_identity": "release=active-release;api=example",
            "retained_release_count": 1,
            "retained_release_ids": ["previous-release"],
            "retained_release_document_paths": ["/var/lib/fishystuff/gitops/rollback-set/local-test/previous-release.json"],
            "rollback_set_available": true,
            "rollback_set_state": "retained_hot_release_set",
        }),
    )?;
    write_json(
        rollback,
        json!({
            "desired_generation": 42,
            "environment": "local-test",
            "host": "vm-single-host",
            "current_release_id": "active-release",
            "current_release_identity": "release=active-release;api=example",
            "rollback_release_id": "previous-release",
            "rollback_release_identity": "release=previous-release;api=example",
            "rollback_api_bundle": "/nix/store/example-previous-api",
            "rollback_dolt_service_bundle": "/nix/store/example-previous-dolt-service",
            "rollback_site_content": "/nix/store/example-previous-site",
            "rollback_cdn_runtime_content": "/nix/store/example-previous-cdn",
            "rollback_dolt_commit": "example-previous",
            "rollback_dolt_materialization": "metadata_only",
            "rollback_dolt_cache_dir": "",
            "rollback_dolt_release_ref": "",
            "rollback_available": true,
            "rollback_state": "retained_hot_release",
        }),
    )
}

struct TestRoot {
    path: PathBuf,
}

impl TestRoot {
    fn new(prefix: &str) -> Result<Self> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("system clock is before UNIX_EPOCH")?
            .as_nanos();
        let path = std::env::temp_dir().join(format!("{prefix}-{nonce}"));
        fs::create_dir_all(&path)
            .with_context(|| format!("creating test root {}", path.display()))?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn assert_helper_failure_contains<'a, I>(args: I, expected: &str) -> Result<()>
where
    I: IntoIterator<Item = &'a str>,
{
    let output = run_helper_raw(args)?;
    if output.status.success() {
        return bail_command("fishystuff_deploy expected failure", output);
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.contains(expected) {
        bail!(
            "helper failure did not contain {expected:?}\nstderr:\n{}",
            stderr
        );
    }
    Ok(())
}

fn run_helper_raw<'a, I>(args: I) -> Result<Output>
where
    I: IntoIterator<Item = &'a str>,
{
    let helper = env!("CARGO_BIN_EXE_fishystuff_deploy");
    let args: Vec<&str> = args.into_iter().collect();
    Command::new(helper)
        .args(&args)
        .env("NO_COLOR", "1")
        .output()
        .with_context(|| format!("running {} {}", helper, args.join(" ")))
}

fn bail_command<T>(command: &str, output: Output) -> Result<T> {
    bail!(
        "command failed: {command}\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn path_str(path: &Path) -> Result<&str> {
    path.to_str().context("path is not UTF-8")
}

fn write_json(path: &Path, value: Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file =
        fs::File::create(path).with_context(|| format!("creating {}", path.display()))?;
    serde_json::to_writer_pretty(&mut file, &value)
        .with_context(|| format!("writing {}", path.display()))?;
    file.write_all(b"\n")
        .with_context(|| format!("finalizing {}", path.display()))
}

fn read_json(path: &Path) -> Result<Value> {
    let file = fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;
    serde_json::from_reader(file).with_context(|| format!("decoding {}", path.display()))
}
