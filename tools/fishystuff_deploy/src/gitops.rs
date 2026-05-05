use std::fs::File;
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::Deserialize;

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

pub struct ServedSummary {
    desired_generation: u64,
    environment: String,
    host: String,
    release_id: String,
    release_identity: String,
    rollback_primary_release_id: String,
    rollback_retained_count: usize,
    retained_release_ids: Vec<String>,
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
            "environment: {}\nhost: {}\ngeneration: {}\nserved_release: {}\nrelease_identity: {}\nrollback_primary: {}\nretained_rollback_releases: {}\nretained_count: {}\n",
            self.environment,
            self.host,
            self.desired_generation,
            self.release_id,
            self.release_identity,
            self.rollback_primary_release_id,
            retained,
            self.rollback_retained_count
        )
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
        rollback_retained_count: status.rollback_retained_count,
        retained_release_ids: status.retained_release_ids,
    })
}

fn read_json<T>(path: &Path, label: &str) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let file = File::open(path).with_context(|| format!("opening {label} {}", path.display()))?;
    serde_json::from_reader(file).with_context(|| format!("decoding {label} {}", path.display()))
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
