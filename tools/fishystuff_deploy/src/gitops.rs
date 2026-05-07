use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread::sleep;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct StatusDocument {
    desired_generation: u64,
    release_id: String,
    release_identity: String,
    environment: String,
    host: String,
    phase: String,
    admission_state: String,
    retained_release_ids: Vec<String>,
    rollback_available: bool,
    rollback_primary_release_id: String,
    rollback_retained_count: usize,
    served: bool,
}

#[derive(Debug, Deserialize)]
struct ActiveDocument {
    desired_generation: u64,
    release_id: String,
    release_identity: String,
    environment: String,
    host: String,
    retained_release_ids: Vec<String>,
    admission_state: String,
    served: bool,
}

#[derive(Debug, Deserialize)]
struct RollbackSetDocument {
    desired_generation: u64,
    environment: String,
    host: String,
    current_release_id: String,
    current_release_identity: String,
    retained_release_count: usize,
    retained_release_ids: Vec<String>,
    rollback_set_available: bool,
}

#[derive(Debug, Deserialize)]
struct RollbackReadinessDocument {
    desired_generation: u64,
    environment: String,
    host: String,
    current_release_id: String,
    current_release_identity: String,
    rollback_release_id: String,
    rollback_release_identity: String,
    rollback_available: bool,
}

#[derive(Debug, Deserialize)]
struct AdmissionDocument {
    environment: String,
    host: String,
    release_id: String,
    release_identity: String,
    admission_state: String,
    probe: String,
    #[serde(default)]
    probe_name: String,
    #[serde(default)]
    url: String,
}

#[derive(Debug, Deserialize)]
struct RouteDocument {
    desired_generation: u64,
    environment: String,
    host: String,
    release_id: String,
    release_identity: String,
    site_root: String,
    cdn_root: String,
    #[serde(default)]
    api_upstream: String,
    served: bool,
    state: String,
}

#[derive(Debug, Deserialize)]
struct RootsReadyRequest {
    desired_generation: u64,
    environment: String,
    host: String,
    release_id: String,
    release_identity: String,
    roots: Vec<RootRequest>,
    #[serde(default = "default_true")]
    require_nix_gcroot: bool,
    #[serde(default = "default_roots_timeout_ms")]
    timeout_ms: u64,
}

#[derive(Debug, Deserialize)]
struct RootRequest {
    name: String,
    root_path: PathBuf,
    store_path: PathBuf,
}

#[derive(Debug, Deserialize, Serialize)]
struct RootsReadyStatus {
    desired_generation: u64,
    environment: String,
    host: String,
    release_id: String,
    release_identity: String,
    root_count: usize,
    require_nix_gcroot: bool,
    roots_ready: bool,
    state: String,
    roots: Vec<RootStatus>,
}

#[derive(Debug, Deserialize, Serialize)]
struct RootStatus {
    name: String,
    root_path: String,
    store_path: String,
    observed_target: String,
    symlink_ready: bool,
    nix_gcroot_ready: bool,
}

pub struct ServedSummary {
    desired_generation: u64,
    environment: String,
    host: String,
    release_id: String,
    release_identity: String,
    rollback_primary_release_id: String,
    rollback_primary_release_identity: String,
    rollback_retained_count: usize,
    retained_release_ids: Vec<String>,
}

pub struct ServedInspection {
    summary: ServedSummary,
    admission: Option<AdmissionDocument>,
    route: Option<RouteDocument>,
    roots: Vec<RootsReadyStatus>,
}

impl ServedSummary {
    pub fn summary_line(&self) -> String {
        format!(
            "served rollback-ready: environment={} host={} release_id={} rollback_primary={} retained_count={}",
            self.environment,
            self.host,
            self.release_id,
            self.rollback_primary_release_id,
            self.rollback_retained_count
        )
    }

    pub fn operator_summary(&self) -> String {
        let retained = self.retained_release_ids.join(", ");
        format!(
            "environment: {}\nhost: {}\ngeneration: {}\nserved_release: {}\nrelease_identity: {}\nrollback_primary: {}\nrollback_primary_identity: {}\nretained_rollback_releases: {}\nretained_count: {}\n",
            self.environment,
            self.host,
            self.desired_generation,
            self.release_id,
            self.release_identity,
            self.rollback_primary_release_id,
            self.rollback_primary_release_identity,
            retained,
            self.rollback_retained_count
        )
    }
}

impl ServedInspection {
    pub fn operator_summary(&self) -> String {
        let mut output = self.summary.operator_summary();
        if let Some(admission) = &self.admission {
            output.push_str(&format!(
                "admission_state: {}\nadmission_probe: {}\n",
                admission.admission_state, admission.probe
            ));
            if !admission.probe_name.is_empty() {
                output.push_str(&format!("admission_probe_name: {}\n", admission.probe_name));
            }
            if !admission.url.is_empty() {
                output.push_str(&format!("admission_url: {}\n", admission.url));
            }
        }
        if let Some(route) = &self.route {
            output.push_str(&format!(
                "route_state: {}\nroute_site_root: {}\nroute_cdn_root: {}\n",
                route.state, route.site_root, route.cdn_root
            ));
            if !route.api_upstream.is_empty() {
                output.push_str(&format!("route_api_upstream: {}\n", route.api_upstream));
            }
        }
        if !self.roots.is_empty() {
            let releases = self
                .roots
                .iter()
                .map(|status| status.release_id.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            output.push_str(&format!(
                "roots_ready_releases: {}\nroots_ready_count: {}\n",
                releases,
                self.roots.len()
            ));
        }
        output
    }
}

pub fn check_served(
    status_path: &Path,
    active_path: &Path,
    rollback_set_path: &Path,
    rollback_path: &Path,
    expected_environment: Option<&str>,
    expected_host: Option<&str>,
    expected_release_id: Option<&str>,
) -> Result<ServedSummary> {
    let status: StatusDocument = read_json(status_path, "GitOps status")?;
    let active: ActiveDocument = read_json(active_path, "GitOps active selection")?;
    let rollback_set: RollbackSetDocument = read_json(rollback_set_path, "GitOps rollback set")?;
    let rollback: RollbackReadinessDocument =
        read_json(rollback_path, "GitOps rollback readiness")?;

    if let Some(expected) = expected_environment {
        require_eq("status environment", status.environment.as_str(), expected)?;
    }
    if let Some(expected) = expected_host {
        require_eq("status host", status.host.as_str(), expected)?;
    }
    if let Some(expected) = expected_release_id {
        require_eq("status release_id", status.release_id.as_str(), expected)?;
    }

    require_eq("status phase", status.phase.as_str(), "served")?;
    require_eq(
        "status admission_state",
        status.admission_state.as_str(),
        "passed_fixture",
    )?;
    require_true("status served", status.served)?;
    require_true("status rollback_available", status.rollback_available)?;
    if status.rollback_retained_count == 0 {
        bail!("status rollback_retained_count must be greater than zero");
    }
    if status.retained_release_ids.is_empty() {
        bail!("status retained_release_ids must not be empty for served rollback-ready state");
    }
    if status.rollback_retained_count != status.retained_release_ids.len() {
        bail!(
            "status rollback_retained_count {} does not match retained_release_ids length {}",
            status.rollback_retained_count,
            status.retained_release_ids.len()
        );
    }
    require_eq(
        "status rollback_primary_release_id",
        &status.rollback_primary_release_id,
        &status.retained_release_ids[0],
    )?;

    require_eq(
        "active desired_generation",
        active.desired_generation,
        status.desired_generation,
    )?;
    require_eq(
        "active environment",
        &active.environment,
        &status.environment,
    )?;
    require_eq("active host", &active.host, &status.host)?;
    require_eq("active release_id", &active.release_id, &status.release_id)?;
    require_eq(
        "active release_identity",
        &active.release_identity,
        &status.release_identity,
    )?;
    require_eq(
        "active admission_state",
        &active.admission_state,
        &status.admission_state,
    )?;
    require_true("active served", active.served)?;
    require_same_list(
        "active retained_release_ids",
        &active.retained_release_ids,
        &status.retained_release_ids,
    )?;

    require_eq(
        "rollback-set desired_generation",
        rollback_set.desired_generation,
        status.desired_generation,
    )?;
    require_eq(
        "rollback-set environment",
        &rollback_set.environment,
        &status.environment,
    )?;
    require_eq("rollback-set host", &rollback_set.host, &status.host)?;
    require_eq(
        "rollback-set current_release_id",
        &rollback_set.current_release_id,
        &status.release_id,
    )?;
    require_eq(
        "rollback-set current_release_identity",
        &rollback_set.current_release_identity,
        &status.release_identity,
    )?;
    require_true(
        "rollback-set rollback_set_available",
        rollback_set.rollback_set_available,
    )?;
    require_eq(
        "rollback-set retained_release_count",
        rollback_set.retained_release_count,
        status.rollback_retained_count,
    )?;
    require_same_list(
        "rollback-set retained_release_ids",
        &rollback_set.retained_release_ids,
        &status.retained_release_ids,
    )?;

    require_eq(
        "rollback desired_generation",
        rollback.desired_generation,
        status.desired_generation,
    )?;
    require_eq(
        "rollback environment",
        &rollback.environment,
        &status.environment,
    )?;
    require_eq("rollback host", &rollback.host, &status.host)?;
    require_eq(
        "rollback current_release_id",
        &rollback.current_release_id,
        &status.release_id,
    )?;
    require_eq(
        "rollback current_release_identity",
        &rollback.current_release_identity,
        &status.release_identity,
    )?;
    require_eq(
        "rollback rollback_release_id",
        &rollback.rollback_release_id,
        &status.rollback_primary_release_id,
    )?;
    if rollback.rollback_release_identity.is_empty() {
        bail!("rollback rollback_release_identity must not be empty");
    }
    require_true("rollback rollback_available", rollback.rollback_available)?;

    Ok(ServedSummary {
        desired_generation: status.desired_generation,
        environment: status.environment,
        host: status.host,
        release_id: status.release_id,
        release_identity: status.release_identity,
        rollback_primary_release_id: status.rollback_primary_release_id,
        rollback_primary_release_identity: rollback.rollback_release_identity,
        rollback_retained_count: status.rollback_retained_count,
        retained_release_ids: status.retained_release_ids,
    })
}

pub fn inspect_served(
    status_path: &Path,
    active_path: &Path,
    rollback_set_path: &Path,
    rollback_path: &Path,
    admission_path: Option<&Path>,
    route_path: Option<&Path>,
    roots_dir: Option<&Path>,
    expected_environment: Option<&str>,
    expected_host: Option<&str>,
    expected_release_id: Option<&str>,
) -> Result<ServedInspection> {
    let summary = check_served(
        status_path,
        active_path,
        rollback_set_path,
        rollback_path,
        expected_environment,
        expected_host,
        expected_release_id,
    )?;

    let admission = if let Some(path) = admission_path {
        let admission: AdmissionDocument = read_json(path, "GitOps admission status")?;
        validate_admission(&summary, &admission)?;
        Some(admission)
    } else {
        None
    };

    let route = if let Some(path) = route_path {
        let route: RouteDocument = read_json(path, "GitOps route selection")?;
        validate_route(&summary, &route)?;
        Some(route)
    } else {
        None
    };

    let roots = roots_dir
        .map(|dir| read_roots_statuses(dir, &summary))
        .transpose()?
        .unwrap_or_default();

    Ok(ServedInspection {
        summary,
        admission,
        route,
        roots,
    })
}

pub fn roots_ready(request_path: &Path, status_path: &Path) -> Result<()> {
    let request: RootsReadyRequest = read_json(request_path, "GitOps roots-ready request")?;
    validate_roots_request(&request)?;

    let timeout = Duration::from_millis(request.timeout_ms);
    let started = Instant::now();
    loop {
        match inspect_roots(&request) {
            Ok(roots) => {
                let status = roots_status(&request, roots);
                write_json(status_path, &status, "GitOps roots-ready status")?;
                return Ok(());
            }
            Err(error) => {
                if started.elapsed() >= timeout {
                    return Err(error).with_context(|| {
                        format!(
                            "waiting for roots for release {} in environment {}",
                            request.release_id, request.environment
                        )
                    });
                }
                sleep(Duration::from_millis(250));
            }
        }
    }
}

pub fn needs_roots_ready(request_path: &Path, status_path: &Path) -> bool {
    let Ok(request) = read_json::<RootsReadyRequest>(request_path, "GitOps roots-ready request")
    else {
        return true;
    };
    if validate_roots_request(&request).is_err() {
        return true;
    }
    let Ok(actual_roots) = inspect_roots(&request) else {
        return true;
    };
    let Ok(status) = read_json::<RootsReadyStatus>(status_path, "GitOps roots-ready status") else {
        return true;
    };

    status_matches_roots(&request, &actual_roots, &status).is_err()
}

fn validate_admission(summary: &ServedSummary, admission: &AdmissionDocument) -> Result<()> {
    require_eq(
        "admission environment",
        admission.environment.as_str(),
        summary.environment.as_str(),
    )?;
    require_eq(
        "admission host",
        admission.host.as_str(),
        summary.host.as_str(),
    )?;
    require_eq(
        "admission release_id",
        admission.release_id.as_str(),
        summary.release_id.as_str(),
    )?;
    require_eq(
        "admission release_identity",
        admission.release_identity.as_str(),
        summary.release_identity.as_str(),
    )?;
    require_eq(
        "admission admission_state",
        admission.admission_state.as_str(),
        "passed_fixture",
    )?;
    if admission.probe.is_empty() {
        bail!("admission probe must not be empty");
    }
    Ok(())
}

fn validate_route(summary: &ServedSummary, route: &RouteDocument) -> Result<()> {
    require_eq(
        "route desired_generation",
        route.desired_generation,
        summary.desired_generation,
    )?;
    require_eq(
        "route environment",
        route.environment.as_str(),
        summary.environment.as_str(),
    )?;
    require_eq("route host", route.host.as_str(), summary.host.as_str())?;
    require_eq(
        "route release_id",
        route.release_id.as_str(),
        summary.release_id.as_str(),
    )?;
    require_eq(
        "route release_identity",
        route.release_identity.as_str(),
        summary.release_identity.as_str(),
    )?;
    require_true("route served", route.served)?;
    require_eq("route state", route.state.as_str(), "selected_local_route")?;
    if route.site_root.is_empty() {
        bail!("route site_root must not be empty");
    }
    if route.cdn_root.is_empty() {
        bail!("route cdn_root must not be empty");
    }
    Ok(())
}

fn read_roots_statuses(roots_dir: &Path, summary: &ServedSummary) -> Result<Vec<RootsReadyStatus>> {
    let mut release_ids = Vec::with_capacity(1 + summary.retained_release_ids.len());
    release_ids.push(summary.release_id.clone());
    release_ids.extend(summary.retained_release_ids.iter().cloned());

    let mut statuses = Vec::with_capacity(release_ids.len());
    for release_id in release_ids {
        let path = roots_dir.join(format!("{}-{}.json", summary.environment, release_id));
        let status: RootsReadyStatus = read_json(&path, "GitOps roots-ready status")?;
        validate_roots_status_for_summary(summary, &status, &release_id)?;
        statuses.push(status);
    }
    Ok(statuses)
}

fn validate_roots_status_for_summary(
    summary: &ServedSummary,
    status: &RootsReadyStatus,
    release_id: &str,
) -> Result<()> {
    require_eq(
        "roots status desired_generation",
        status.desired_generation,
        summary.desired_generation,
    )?;
    require_eq(
        "roots status environment",
        status.environment.as_str(),
        summary.environment.as_str(),
    )?;
    require_eq(
        "roots status host",
        status.host.as_str(),
        summary.host.as_str(),
    )?;
    require_eq(
        "roots status release_id",
        status.release_id.as_str(),
        release_id,
    )?;
    if release_id == summary.release_id {
        require_eq(
            "roots status active release_identity",
            status.release_identity.as_str(),
            summary.release_identity.as_str(),
        )?;
    } else if status.release_identity.is_empty() {
        bail!("roots status retained release_identity must not be empty");
    }
    require_true("roots status roots_ready", status.roots_ready)?;
    require_eq("roots status state", status.state.as_str(), "roots_ready")?;
    if status.root_count == 0 {
        bail!("roots status root_count must be greater than zero");
    }
    require_eq(
        "roots status root_count",
        status.root_count,
        status.roots.len(),
    )?;
    for root in &status.roots {
        if root.name.is_empty() {
            bail!("roots status root name must not be empty");
        }
        if root.root_path.is_empty() {
            bail!("roots status root_path must not be empty");
        }
        if root.store_path.is_empty() {
            bail!("roots status store_path must not be empty");
        }
        require_true("roots status root symlink_ready", root.symlink_ready)?;
        if status.require_nix_gcroot {
            require_true("roots status root nix_gcroot_ready", root.nix_gcroot_ready)?;
        }
    }
    Ok(())
}

fn validate_roots_request(request: &RootsReadyRequest) -> Result<()> {
    if request.desired_generation == 0 {
        bail!("roots-ready request desired_generation must be non-zero");
    }
    if request.environment.is_empty() {
        bail!("roots-ready request environment must not be empty");
    }
    if request.host.is_empty() {
        bail!("roots-ready request host must not be empty");
    }
    if request.release_id.is_empty() {
        bail!("roots-ready request release_id must not be empty");
    }
    if request.release_identity.is_empty() {
        bail!("roots-ready request release_identity must not be empty");
    }
    if request.roots.is_empty() {
        bail!("roots-ready request roots must not be empty");
    }
    if request.timeout_ms == 0 || request.timeout_ms > 900_000 {
        bail!("roots-ready request timeout_ms must be between 1 and 900000");
    }
    for root in &request.roots {
        if root.name.is_empty() {
            bail!("roots-ready request root name must not be empty");
        }
        if !root.root_path.is_absolute() {
            bail!(
                "roots-ready request root {} path must be absolute",
                root.name
            );
        }
        if !root.store_path.is_absolute() {
            bail!(
                "roots-ready request root {} store path must be absolute",
                root.name
            );
        }
        if request.require_nix_gcroot && !path_starts_with_str(&root.store_path, "/nix/store/") {
            bail!(
                "roots-ready request root {} store path must be under /nix/store",
                root.name
            );
        }
    }
    Ok(())
}

fn inspect_roots(request: &RootsReadyRequest) -> Result<Vec<RootStatus>> {
    let nix_roots = if request.require_nix_gcroot {
        Some(nix_gc_roots()?)
    } else {
        None
    };

    request
        .roots
        .iter()
        .map(|root| inspect_root(root, request.require_nix_gcroot, nix_roots.as_deref()))
        .collect()
}

fn inspect_root(
    root: &RootRequest,
    require_nix_gcroot: bool,
    nix_roots: Option<&str>,
) -> Result<RootStatus> {
    let metadata = std::fs::symlink_metadata(&root.root_path)
        .with_context(|| format!("reading root {}", root.root_path.display()))?;
    if !metadata.file_type().is_symlink() {
        bail!("root {} is not a symlink", root.root_path.display());
    }

    let observed_target = std::fs::read_link(&root.root_path)
        .with_context(|| format!("reading root target {}", root.root_path.display()))?;
    if observed_target != root.store_path {
        bail!(
            "root {} points to {}, expected {}",
            root.root_path.display(),
            observed_target.display(),
            root.store_path.display()
        );
    }
    if !root.store_path.exists() {
        bail!("store path {} does not exist", root.store_path.display());
    }

    let nix_gcroot_ready = if require_nix_gcroot {
        let root_path = path_to_string(&root.root_path)?;
        nix_roots
            .context("missing nix roots output")?
            .lines()
            .any(|line| line.contains(&root_path))
    } else {
        false
    };
    if require_nix_gcroot && !nix_gcroot_ready {
        bail!(
            "root {} is not reported by nix-store --gc --print-roots",
            root.root_path.display()
        );
    }

    Ok(RootStatus {
        name: root.name.clone(),
        root_path: path_to_string(&root.root_path)?,
        store_path: path_to_string(&root.store_path)?,
        observed_target: path_to_string(&observed_target)?,
        symlink_ready: true,
        nix_gcroot_ready,
    })
}

fn roots_status(request: &RootsReadyRequest, roots: Vec<RootStatus>) -> RootsReadyStatus {
    RootsReadyStatus {
        desired_generation: request.desired_generation,
        environment: request.environment.clone(),
        host: request.host.clone(),
        release_id: request.release_id.clone(),
        release_identity: request.release_identity.clone(),
        root_count: roots.len(),
        require_nix_gcroot: request.require_nix_gcroot,
        roots_ready: true,
        state: "roots_ready".to_string(),
        roots,
    }
}

fn status_matches_roots(
    request: &RootsReadyRequest,
    actual_roots: &[RootStatus],
    status: &RootsReadyStatus,
) -> Result<()> {
    require_eq(
        "roots status desired_generation",
        status.desired_generation,
        request.desired_generation,
    )?;
    require_eq(
        "roots status environment",
        status.environment.as_str(),
        request.environment.as_str(),
    )?;
    require_eq(
        "roots status host",
        status.host.as_str(),
        request.host.as_str(),
    )?;
    require_eq(
        "roots status release_id",
        status.release_id.as_str(),
        request.release_id.as_str(),
    )?;
    require_eq(
        "roots status release_identity",
        status.release_identity.as_str(),
        request.release_identity.as_str(),
    )?;
    require_eq(
        "roots status require_nix_gcroot",
        status.require_nix_gcroot,
        request.require_nix_gcroot,
    )?;
    require_true("roots status roots_ready", status.roots_ready)?;
    require_eq("roots status state", status.state.as_str(), "roots_ready")?;
    require_eq(
        "roots status root_count",
        status.root_count,
        request.roots.len(),
    )?;
    require_eq(
        "roots status roots length",
        status.roots.len(),
        actual_roots.len(),
    )?;
    for (index, (status_root, actual_root)) in status.roots.iter().zip(actual_roots).enumerate() {
        require_eq(
            &format!("roots status root {index} name"),
            status_root.name.as_str(),
            actual_root.name.as_str(),
        )?;
        require_eq(
            &format!("roots status root {index} root_path"),
            status_root.root_path.as_str(),
            actual_root.root_path.as_str(),
        )?;
        require_eq(
            &format!("roots status root {index} store_path"),
            status_root.store_path.as_str(),
            actual_root.store_path.as_str(),
        )?;
        require_eq(
            &format!("roots status root {index} observed_target"),
            status_root.observed_target.as_str(),
            actual_root.observed_target.as_str(),
        )?;
        require_true(
            &format!("roots status root {index} symlink_ready"),
            status_root.symlink_ready,
        )?;
        require_eq(
            &format!("roots status root {index} nix_gcroot_ready"),
            status_root.nix_gcroot_ready,
            actual_root.nix_gcroot_ready,
        )?;
    }
    Ok(())
}

fn nix_gc_roots() -> Result<String> {
    let output = Command::new("nix-store")
        .args(["--gc", "--print-roots"])
        .output()
        .context("running nix-store --gc --print-roots")?;
    if !output.status.success() {
        bail!(
            "nix-store --gc --print-roots failed\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    String::from_utf8(output.stdout).context("decoding nix-store --gc --print-roots output")
}

fn read_json<T>(path: &Path, label: &str) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let file = File::open(path).with_context(|| format!("opening {label} {}", path.display()))?;
    serde_json::from_reader(file).with_context(|| format!("decoding {label} {}", path.display()))
}

fn write_json<T>(path: &Path, value: &T, label: &str) -> Result<()>
where
    T: Serialize,
{
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {label} parent {}", parent.display()))?;
    }
    let mut file =
        File::create(path).with_context(|| format!("creating {label} {}", path.display()))?;
    serde_json::to_writer_pretty(&mut file, value)
        .with_context(|| format!("writing {label} {}", path.display()))?;
    file.write_all(b"\n")
        .with_context(|| format!("finalizing {label} {}", path.display()))
}

fn require_true(label: &str, actual: bool) -> Result<()> {
    if !actual {
        bail!("{label} must be true");
    }
    Ok(())
}

fn require_eq<T>(label: &str, actual: T, expected: T) -> Result<()>
where
    T: std::fmt::Display + PartialEq,
{
    if actual != expected {
        bail!("{label} was {actual}, expected {expected}");
    }
    Ok(())
}

fn require_same_list(label: &str, actual: &[String], expected: &[String]) -> Result<()> {
    if actual != expected {
        bail!("{label} was {actual:?}, expected {expected:?}");
    }
    Ok(())
}

fn path_to_string(path: &Path) -> Result<String> {
    path.to_str()
        .map(ToOwned::to_owned)
        .with_context(|| format!("path is not UTF-8: {}", path.display()))
}

fn path_starts_with_str(path: &Path, prefix: &str) -> bool {
    path.to_str()
        .map(|value| value.starts_with(prefix))
        .unwrap_or(false)
}

fn default_true() -> bool {
    true
}

fn default_roots_timeout_ms() -> u64 {
    300_000
}
