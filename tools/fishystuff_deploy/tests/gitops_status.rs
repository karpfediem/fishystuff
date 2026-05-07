use std::fs;
use std::io::Write;
use std::os::unix::fs as unix_fs;
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
fn gitops_inspect_served_prints_admission_route_and_roots() -> Result<()> {
    let root = TestRoot::new("fishystuff-deploy-gitops-inspect")?;
    let status = root.path().join("status.json");
    let active = root.path().join("active.json");
    let rollback_set = root.path().join("rollback-set.json");
    let rollback = root.path().join("rollback.json");
    let admission = root.path().join("run/admission/local-test.json");
    let route = root.path().join("run/routes/local-test.json");
    let roots_dir = root.path().join("run/roots");

    write_served_documents(&status, &active, &rollback_set, &rollback)?;
    write_admission_document(&admission)?;
    write_route_document(&route)?;
    write_roots_status(
        &roots_dir.join("local-test-active-release.json"),
        "active-release",
        "release=active-release;api=example",
    )?;
    write_roots_status(
        &roots_dir.join("local-test-previous-release.json"),
        "previous-release",
        "release=previous-release;api=example",
    )?;

    let output = run_helper_raw([
        "gitops",
        "inspect-served",
        "--status",
        path_str(&status)?,
        "--active",
        path_str(&active)?,
        "--rollback-set",
        path_str(&rollback_set)?,
        "--rollback",
        path_str(&rollback)?,
        "--admission",
        path_str(&admission)?,
        "--route",
        path_str(&route)?,
        "--roots-dir",
        path_str(&roots_dir)?,
        "--environment",
        "local-test",
        "--host",
        "vm-single-host",
        "--release-id",
        "active-release",
    ])?;
    if !output.status.success() {
        return bail_command("fishystuff_deploy gitops inspect-served", output);
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("served_release: active-release"));
    assert!(stdout.contains("admission_state: passed_fixture"));
    assert!(stdout.contains("admission_probe: local-fixture"));
    assert!(stdout.contains("route_state: selected_local_route"));
    assert!(stdout.contains("route_site_root: /var/lib/fishystuff/gitops/served/local-test/site"));
    assert!(stdout.contains("route_cdn_root: /var/lib/fishystuff/gitops/served/local-test/cdn"));
    assert!(stdout.contains("roots_ready_releases: active-release, previous-release"));
    assert!(stdout.contains("roots_ready_count: 2"));

    Ok(())
}

#[test]
fn gitops_inspect_served_rejects_missing_retained_roots() -> Result<()> {
    let root = TestRoot::new("fishystuff-deploy-gitops-inspect-missing-roots")?;
    let status = root.path().join("status.json");
    let active = root.path().join("active.json");
    let rollback_set = root.path().join("rollback-set.json");
    let rollback = root.path().join("rollback.json");
    let roots_dir = root.path().join("run/roots");

    write_served_documents(&status, &active, &rollback_set, &rollback)?;
    write_roots_status(
        &roots_dir.join("local-test-active-release.json"),
        "active-release",
        "release=active-release;api=example",
    )?;

    assert_helper_failure_contains(
        [
            "gitops",
            "inspect-served",
            "--status",
            path_str(&status)?,
            "--active",
            path_str(&active)?,
            "--rollback-set",
            path_str(&rollback_set)?,
            "--rollback",
            path_str(&rollback)?,
            "--roots-dir",
            path_str(&roots_dir)?,
        ],
        "local-test-previous-release.json",
    )
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

#[test]
fn gitops_retained_releases_json_converts_member_documents() -> Result<()> {
    let root = TestRoot::new("fishystuff-deploy-gitops-retained-json")?;
    let member = root
        .path()
        .join("rollback-set/local-test/previous-release.json");
    write_rollback_member_document(
        &member,
        "previous-release",
        "release=previous-release;generation=7;git_rev=previous-git;dolt_commit=previous-dolt;dolt_repository=fishystuff/fishystuff;dolt_branch_context=main;dolt_mode=read_only;api=/nix/store/example-previous-api;site=/nix/store/example-previous-site;cdn_runtime=/nix/store/example-previous-cdn;dolt_service=/nix/store/example-previous-dolt-service",
    )?;

    let output = run_helper_raw([
        "gitops",
        "retained-releases-json",
        "--rollback-member",
        path_str(&member)?,
    ])?;
    if !output.status.success() {
        return bail_command("fishystuff_deploy gitops retained-releases-json", output);
    }

    let retained: Value =
        serde_json::from_slice(&output.stdout).context("decoding retained releases stdout")?;
    assert_eq!(retained.as_array().map(Vec::len), Some(1));
    assert_eq!(retained[0]["release_id"], "previous-release");
    assert_eq!(retained[0]["generation"], 7);
    assert_eq!(retained[0]["git_rev"], "previous-git");
    assert_eq!(retained[0]["dolt_commit"], "previous-dolt");
    assert_eq!(
        retained[0]["api_closure"],
        "/nix/store/example-previous-api"
    );
    assert_eq!(
        retained[0]["site_closure"],
        "/nix/store/example-previous-site"
    );
    assert_eq!(
        retained[0]["cdn_runtime_closure"],
        "/nix/store/example-previous-cdn"
    );
    assert_eq!(
        retained[0]["dolt_service_closure"],
        "/nix/store/example-previous-dolt-service"
    );
    assert_eq!(retained[0]["dolt_materialization"], "fetch_pin");
    assert_eq!(
        retained[0]["dolt_cache_dir"],
        "/var/lib/fishystuff/gitops/dolt-cache/fishystuff"
    );
    assert_eq!(
        retained[0]["dolt_release_ref"],
        "fishystuff/gitops/previous-release"
    );

    Ok(())
}

#[test]
fn gitops_retained_releases_json_reads_rollback_set_index() -> Result<()> {
    let root = TestRoot::new("fishystuff-deploy-gitops-retained-json-index")?;
    let rollback_set = root.path().join("rollback-set/local-test.json");
    let member = root
        .path()
        .join("rollback-set/local-test/previous-release.json");
    write_rollback_member_document(
        &member,
        "previous-release",
        "release=previous-release;generation=7;git_rev=previous-git;dolt_commit=previous-dolt;dolt_repository=fishystuff/fishystuff;dolt_branch_context=main;dolt_mode=read_only;api=/nix/store/example-previous-api;site=/nix/store/example-previous-site;cdn_runtime=/nix/store/example-previous-cdn;dolt_service=/nix/store/example-previous-dolt-service",
    )?;
    write_json(
        &rollback_set,
        json!({
            "desired_generation": 42,
            "environment": "local-test",
            "host": "vm-single-host",
            "current_release_id": "active-release",
            "current_release_identity": "release=active-release;api=example",
            "retained_release_count": 1,
            "retained_release_ids": ["previous-release"],
            "retained_release_document_paths": [path_str(&member)?],
            "rollback_set_available": true,
            "rollback_set_state": "retained_hot_release_set",
        }),
    )?;

    let output = run_helper_raw([
        "gitops",
        "retained-releases-json",
        "--rollback-set",
        path_str(&rollback_set)?,
    ])?;
    if !output.status.success() {
        return bail_command("fishystuff_deploy gitops retained-releases-json", output);
    }

    let retained: Value =
        serde_json::from_slice(&output.stdout).context("decoding retained releases stdout")?;
    assert_eq!(retained.as_array().map(Vec::len), Some(1));
    assert_eq!(retained[0]["release_id"], "previous-release");
    assert_eq!(retained[0]["generation"], 7);

    Ok(())
}

#[test]
fn gitops_retained_releases_json_rejects_identity_path_mismatch() -> Result<()> {
    let root = TestRoot::new("fishystuff-deploy-gitops-retained-json-mismatch")?;
    let member = root
        .path()
        .join("rollback-set/local-test/previous-release.json");
    write_rollback_member_document(
        &member,
        "previous-release",
        "release=previous-release;generation=7;git_rev=previous-git;dolt_commit=previous-dolt;dolt_repository=fishystuff/fishystuff;dolt_branch_context=main;dolt_mode=read_only;api=/nix/store/example-previous-api;site=/nix/store/example-other-site;cdn_runtime=/nix/store/example-previous-cdn;dolt_service=/nix/store/example-previous-dolt-service",
    )?;

    assert_helper_failure_contains(
        [
            "gitops",
            "retained-releases-json",
            "--rollback-member",
            path_str(&member)?,
        ],
        "release identity site was /nix/store/example-other-site, expected /nix/store/example-previous-site",
    )
}

#[test]
fn gitops_check_desired_serving_accepts_retained_production_tuple() -> Result<()> {
    let root = TestRoot::new("fishystuff-deploy-gitops-desired-serving")?;
    let state = root.path().join("production.desired.json");
    write_json(&state, production_desired_state())?;

    let output = run_helper_raw([
        "gitops",
        "check-desired-serving",
        "--state",
        path_str(&state)?,
        "--environment",
        "production",
    ])?;
    if !output.status.success() {
        return bail_command("fishystuff_deploy gitops check-desired-serving", output);
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("desired serving-ready: cluster=production"));
    assert!(stdout.contains("environment=production"));
    assert!(stdout.contains("active_release=active-release"));
    assert!(stdout.contains("retained_count=1"));
    assert!(stdout.contains("serve_requested=false"));

    Ok(())
}

#[test]
fn gitops_check_desired_serving_rejects_missing_retained_release() -> Result<()> {
    let root = TestRoot::new("fishystuff-deploy-gitops-desired-serving-no-retained")?;
    let state = root.path().join("production.desired.json");
    let mut desired = production_desired_state();
    desired["environments"]["production"]["retained_releases"] = json!([]);
    write_json(&state, desired)?;

    assert_helper_failure_contains(
        [
            "gitops",
            "check-desired-serving",
            "--state",
            path_str(&state)?,
            "--environment",
            "production",
        ],
        "desired environment retained_releases must not be empty for serving readiness",
    )
}

#[test]
fn gitops_check_desired_serving_rejects_incomplete_retained_artifact() -> Result<()> {
    let root = TestRoot::new("fishystuff-deploy-gitops-desired-serving-missing-artifact")?;
    let state = root.path().join("production.desired.json");
    let mut desired = production_desired_state();
    desired["releases"]["previous-release"]["closures"]["cdn_runtime"]["store_path"] =
        Value::String(String::new());
    write_json(&state, desired)?;

    assert_helper_failure_contains(
        [
            "gitops",
            "check-desired-serving",
            "--state",
            path_str(&state)?,
            "--environment",
            "production",
        ],
        "desired release previous-release closure cdn_runtime store_path must not be empty",
    )
}

#[test]
fn gitops_check_desired_serving_rejects_production_metadata_only_dolt() -> Result<()> {
    let root = TestRoot::new("fishystuff-deploy-gitops-desired-serving-metadata-only")?;
    let state = root.path().join("production.desired.json");
    let mut desired = production_desired_state();
    desired["releases"]["active-release"]["dolt"]["materialization"] =
        Value::String("metadata_only".to_string());
    write_json(&state, desired)?;

    assert_helper_failure_contains(
        [
            "gitops",
            "check-desired-serving",
            "--state",
            path_str(&state)?,
            "--environment",
            "production",
        ],
        "desired release active-release dolt materialization must be fetch_pin for production serving readiness",
    )
}

#[test]
fn gitops_roots_ready_writes_status_for_matching_symlinks() -> Result<()> {
    let root = TestRoot::new("fishystuff-deploy-gitops-roots-ready")?;
    let request = root.path().join("request.json");
    let status = root.path().join("status.json");
    let store_dir = root.path().join("store");
    let target = store_dir.join("api");
    let gcroot_dir = root.path().join("gcroots");
    let gcroot = gcroot_dir.join("api");

    fs::create_dir_all(&store_dir)?;
    fs::create_dir_all(&gcroot_dir)?;
    fs::write(&target, b"api\n")?;
    unix_fs::symlink(&target, &gcroot)?;
    write_roots_request(&request, &gcroot, &target, false)?;

    let output = run_helper_raw([
        "gitops",
        "roots-ready",
        "--request",
        path_str(&request)?,
        "--status",
        path_str(&status)?,
    ])?;
    if !output.status.success() {
        return bail_command("fishystuff_deploy gitops roots-ready", output);
    }

    let status_json = read_json(&status)?;
    assert_eq!(status_json["environment"], "local-test");
    assert_eq!(status_json["host"], "vm-single-host");
    assert_eq!(status_json["release_id"], "active-release");
    assert_eq!(status_json["root_count"], 1);
    assert_eq!(status_json["require_nix_gcroot"], false);
    assert_eq!(status_json["roots_ready"], true);
    assert_eq!(status_json["state"], "roots_ready");
    assert_eq!(status_json["roots"][0]["name"], "api");
    assert_eq!(status_json["roots"][0]["root_path"], path_str(&gcroot)?);
    assert_eq!(status_json["roots"][0]["store_path"], path_str(&target)?);
    assert_eq!(
        status_json["roots"][0]["observed_target"],
        path_str(&target)?
    );
    assert_eq!(status_json["roots"][0]["symlink_ready"], true);
    assert_eq!(status_json["roots"][0]["nix_gcroot_ready"], false);

    let output = run_helper_raw([
        "gitops",
        "needs-roots-ready",
        "--request",
        path_str(&request)?,
        "--status",
        path_str(&status)?,
    ])?;
    assert!(
        !output.status.success(),
        "needs-roots-ready should return 1 when status is current"
    );

    Ok(())
}

#[test]
fn gitops_roots_ready_rejects_wrong_symlink_target() -> Result<()> {
    let root = TestRoot::new("fishystuff-deploy-gitops-roots-wrong")?;
    let request = root.path().join("request.json");
    let status = root.path().join("status.json");
    let store_dir = root.path().join("store");
    let expected = store_dir.join("api");
    let observed = store_dir.join("other-api");
    let gcroot_dir = root.path().join("gcroots");
    let gcroot = gcroot_dir.join("api");

    fs::create_dir_all(&store_dir)?;
    fs::create_dir_all(&gcroot_dir)?;
    fs::write(&expected, b"api\n")?;
    fs::write(&observed, b"other api\n")?;
    unix_fs::symlink(&observed, &gcroot)?;
    write_roots_request(&request, &gcroot, &expected, false)?;

    assert_helper_failure_contains(
        [
            "gitops",
            "roots-ready",
            "--request",
            path_str(&request)?,
            "--status",
            path_str(&status)?,
        ],
        "points to",
    )
}

#[test]
fn gitops_needs_roots_ready_detects_stale_status() -> Result<()> {
    let root = TestRoot::new("fishystuff-deploy-gitops-roots-stale")?;
    let request = root.path().join("request.json");
    let status = root.path().join("status.json");
    let store_dir = root.path().join("store");
    let first_target = store_dir.join("api");
    let second_target = store_dir.join("second-api");
    let gcroot_dir = root.path().join("gcroots");
    let gcroot = gcroot_dir.join("api");

    fs::create_dir_all(&store_dir)?;
    fs::create_dir_all(&gcroot_dir)?;
    fs::write(&first_target, b"api\n")?;
    fs::write(&second_target, b"second api\n")?;
    unix_fs::symlink(&first_target, &gcroot)?;
    write_roots_request(&request, &gcroot, &first_target, false)?;

    let output = run_helper_raw([
        "gitops",
        "roots-ready",
        "--request",
        path_str(&request)?,
        "--status",
        path_str(&status)?,
    ])?;
    if !output.status.success() {
        return bail_command("fishystuff_deploy gitops roots-ready", output);
    }

    fs::remove_file(&gcroot)?;
    unix_fs::symlink(&second_target, &gcroot)?;
    write_roots_request(&request, &gcroot, &second_target, false)?;

    let output = run_helper_raw([
        "gitops",
        "needs-roots-ready",
        "--request",
        path_str(&request)?,
        "--status",
        path_str(&status)?,
    ])?;
    assert!(
        output.status.success(),
        "needs-roots-ready should return 0 when status is stale"
    );

    Ok(())
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

fn write_admission_document(admission: &Path) -> Result<()> {
    write_json(
        admission,
        json!({
            "environment": "local-test",
            "host": "vm-single-host",
            "release_id": "active-release",
            "release_identity": "release=active-release;api=example",
            "site_content": "/nix/store/example-site",
            "cdn_runtime_content": "/nix/store/example-cdn",
            "dolt_commit": "example",
            "dolt_materialization": "metadata_only",
            "dolt_release_ref": "",
            "cdn_runtime_module": "fishystuff_ui_bevy.example.js",
            "cdn_runtime_wasm": "fishystuff_ui_bevy_bg.example.wasm",
            "cdn_serving_current_root": "/nix/store/example-cdn/current",
            "cdn_serving_retained_root_count": 1,
            "retained_release_count": 1,
            "cdn_serving_manifest_lists_runtime_manifest": true,
            "cdn_serving_manifest_lists_module": true,
            "cdn_serving_manifest_lists_wasm": true,
            "serving_artifacts_checked": true,
            "admission_state": "passed_fixture",
            "probe": "local-fixture",
        }),
    )
}

fn write_rollback_member_document(
    path: &Path,
    release_id: &str,
    release_identity: &str,
) -> Result<()> {
    write_json(
        path,
        json!({
            "desired_generation": 42,
            "environment": "local-test",
            "host": "vm-single-host",
            "current_release_id": "active-release",
            "release_id": release_id,
            "release_identity": release_identity,
            "api_bundle": "/nix/store/example-previous-api",
            "dolt_service_bundle": "/nix/store/example-previous-dolt-service",
            "site_content": "/nix/store/example-previous-site",
            "cdn_runtime_content": "/nix/store/example-previous-cdn",
            "dolt_commit": "previous-dolt",
            "dolt_materialization": "fetch_pin",
            "dolt_cache_dir": "/var/lib/fishystuff/gitops/dolt-cache/fishystuff",
            "dolt_release_ref": format!("fishystuff/gitops/{release_id}"),
            "dolt_status_path": format!("/run/fishystuff/gitops/dolt/{release_id}.json"),
            "rollback_member_state": "retained_hot_release",
        }),
    )
}

fn production_desired_state() -> Value {
    json!({
        "cluster": "production",
        "generation": 9,
        "mode": "validate",
        "hosts": {
            "production-single-host": {
                "enabled": true,
                "role": "single-site",
                "hostname": "production-single-host"
            }
        },
        "releases": {
            "active-release": desired_release_json(
                9,
                "active-git",
                "active-dolt",
                "active-release"
            ),
            "previous-release": desired_release_json(
                8,
                "previous-git",
                "previous-dolt",
                "previous-release"
            )
        },
        "environments": {
            "production": {
                "enabled": true,
                "strategy": "single_active",
                "host": "production-single-host",
                "active_release": "active-release",
                "retained_releases": ["previous-release"],
                "serve": false
            }
        }
    })
}

fn desired_release_json(
    generation: u64,
    git_rev: &str,
    dolt_commit: &str,
    release_id: &str,
) -> Value {
    json!({
        "generation": generation,
        "git_rev": git_rev,
        "dolt_commit": dolt_commit,
        "closures": {
            "api": desired_closure_json(release_id, "api", "api"),
            "site": desired_closure_json(release_id, "site", "site"),
            "cdn_runtime": desired_closure_json(release_id, "cdn-runtime", "cdn_runtime"),
            "dolt_service": desired_closure_json(release_id, "dolt-service", "dolt_service")
        },
        "dolt": {
            "repository": "fishystuff/fishystuff",
            "commit": dolt_commit,
            "branch_context": "main",
            "mode": "read_only",
            "materialization": "fetch_pin",
            "remote_url": "https://doltremoteapi.dolthub.com/fishystuff/fishystuff",
            "cache_dir": "/var/lib/fishystuff/gitops/dolt-cache/fishystuff",
            "release_ref": format!("fishystuff/gitops/{release_id}")
        }
    })
}

fn desired_closure_json(release_id: &str, gcroot_name: &str, store_suffix: &str) -> Value {
    json!({
        "enabled": true,
        "store_path": format!("/nix/store/example-{release_id}-{store_suffix}"),
        "gcroot_path": format!("/nix/var/nix/gcroots/fishystuff/gitops/{release_id}/{gcroot_name}")
    })
}

fn write_route_document(route: &Path) -> Result<()> {
    write_json(
        route,
        json!({
            "desired_generation": 42,
            "environment": "local-test",
            "host": "vm-single-host",
            "release_id": "active-release",
            "release_identity": "release=active-release;api=example",
            "active_path": "/var/lib/fishystuff/gitops/active/local-test.json",
            "site_root": "/var/lib/fishystuff/gitops/served/local-test/site",
            "cdn_root": "/var/lib/fishystuff/gitops/served/local-test/cdn",
            "api_upstream": "http://127.0.0.1:18082",
            "served": true,
            "state": "selected_local_route",
        }),
    )
}

fn write_roots_status(path: &Path, release_id: &str, release_identity: &str) -> Result<()> {
    write_json(
        path,
        json!({
            "desired_generation": 42,
            "environment": "local-test",
            "host": "vm-single-host",
            "release_id": release_id,
            "release_identity": release_identity,
            "root_count": 1,
            "require_nix_gcroot": true,
            "roots_ready": true,
            "state": "roots_ready",
            "roots": [
                {
                    "name": "api",
                    "root_path": format!("/nix/var/nix/gcroots/fishystuff/gitops/{release_id}/api"),
                    "store_path": format!("/nix/store/{release_id}-api"),
                    "observed_target": format!("/nix/store/{release_id}-api"),
                    "symlink_ready": true,
                    "nix_gcroot_ready": true,
                }
            ],
        }),
    )
}

fn write_roots_request(
    request: &Path,
    gcroot: &Path,
    store_path: &Path,
    require_nix_gcroot: bool,
) -> Result<()> {
    write_json(
        request,
        json!({
            "desired_generation": 42,
            "environment": "local-test",
            "host": "vm-single-host",
            "release_id": "active-release",
            "release_identity": "release=active-release;api=example",
            "timeout_ms": 100,
            "require_nix_gcroot": require_nix_gcroot,
            "roots": [
                {
                    "name": "api",
                    "root_path": gcroot,
                    "store_path": store_path,
                }
            ],
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
