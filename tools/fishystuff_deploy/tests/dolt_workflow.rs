use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

#[test]
fn dolt_fetch_pin_and_sql_scalar_admission_follows_exact_commit() -> Result<()> {
    let Some(dolt_bin) = find_dolt()? else {
        eprintln!("skipping Dolt workflow test because dolt is not available in PATH");
        return Ok(());
    };

    let root = TestRoot::new("fishystuff-deploy-dolt-workflow")?;
    let source = root.path().join("source");
    let remote = root.path().join("remote");
    let alternate_remote = root.path().join("alternate-remote");
    let home = root.path().join("home");
    let cache = root.path().join("cache/fishystuff");
    let release_ref = "fishystuff/gitops/example-release";

    fs::create_dir_all(&source)?;
    fs::create_dir_all(&remote)?;
    fs::create_dir_all(&alternate_remote)?;
    fs::create_dir_all(&home)?;

    run(
        &dolt_bin,
        &home,
        None,
        [
            "config",
            "--global",
            "--add",
            "versioncheck.disabled",
            "true",
        ],
    )?;
    run(
        &dolt_bin,
        &home,
        None,
        ["config", "--global", "--add", "metrics.disabled", "true"],
    )?;
    run(
        &dolt_bin,
        &home,
        Some(&source),
        [
            "init",
            "--name",
            "FishyStuff GitOps Test",
            "--email",
            "fishystuff-gitops@example.invalid",
        ],
    )?;
    run(
        &dolt_bin,
        &home,
        Some(&source),
        [
            "sql",
            "-q",
            "create table t (pk int primary key, v varchar(20)); insert into t values (1, 'one');",
        ],
    )?;
    run(&dolt_bin, &home, Some(&source), ["add", "t"])?;
    run(
        &dolt_bin,
        &home,
        Some(&source),
        ["commit", "-m", "commit-one"],
    )?;
    run(
        &dolt_bin,
        &home,
        Some(&source),
        [
            "remote",
            "add",
            "origin",
            &format!("file://{}", remote.display()),
        ],
    )?;
    run(&dolt_bin, &home, Some(&source), ["push", "origin", "main"])?;
    run(
        &dolt_bin,
        &home,
        Some(&source),
        [
            "remote",
            "add",
            "alternate",
            &format!("file://{}", alternate_remote.display()),
        ],
    )?;
    run(
        &dolt_bin,
        &home,
        Some(&source),
        ["push", "alternate", "main"],
    )?;
    let commit1 = dolt_hash_of(&dolt_bin, &home, &source, "main")?;

    let fetch_status = root.path().join("status/fetch.json");
    let admission_status = root.path().join("status/admission.json");
    fetch_pin(
        &root,
        &home,
        &dolt_bin,
        &cache,
        release_ref,
        &commit1,
        &fetch_status,
    )?;
    let fetch_request = root.path().join("requests/fetch.json");
    assert_helper_current(
        &home,
        [
            "dolt",
            "needs-fetch-pin",
            "--request",
            fetch_request
                .to_str()
                .context("fetch request path is not UTF-8")?,
            "--status",
            fetch_status
                .to_str()
                .context("fetch status path is not UTF-8")?,
            "--dolt-bin",
            dolt_bin.to_str().context("dolt path is not UTF-8")?,
        ],
    )?;
    let stale_fetch_status = root.path().join("status/fetch-stale-identity.json");
    let mut stale_fetch = read_json(&fetch_status)?;
    stale_fetch["release_identity"] = Value::String("wrong-release-identity".to_owned());
    write_json(&stale_fetch_status, stale_fetch)?;
    assert_helper_needs(
        &home,
        [
            "dolt",
            "needs-fetch-pin",
            "--request",
            fetch_request
                .to_str()
                .context("fetch request path is not UTF-8")?,
            "--status",
            stale_fetch_status
                .to_str()
                .context("stale fetch status path is not UTF-8")?,
            "--dolt-bin",
            dolt_bin.to_str().context("dolt path is not UTF-8")?,
        ],
    )?;
    fetch_pin_from_remote(
        &root,
        &home,
        &dolt_bin,
        &cache,
        &format!("file://{}", alternate_remote.display()),
        release_ref,
        &commit1,
        &fetch_status,
    )?;
    assert_eq!(
        origin_remote_url(&dolt_bin, &home, &cache)?,
        format!("file://{}", alternate_remote.display())
    );
    run(
        &dolt_bin,
        &home,
        Some(&cache),
        ["remote", "remove", "origin"],
    )?;
    run(
        &dolt_bin,
        &home,
        Some(&cache),
        [
            "remote",
            "add",
            "origin",
            &format!("file://{}", remote.display()),
        ],
    )?;
    assert_helper_needs(
        &home,
        [
            "dolt",
            "needs-fetch-pin",
            "--request",
            fetch_request
                .to_str()
                .context("fetch request path is not UTF-8")?,
            "--status",
            fetch_status
                .to_str()
                .context("fetch status path is not UTF-8")?,
            "--dolt-bin",
            dolt_bin.to_str().context("dolt path is not UTF-8")?,
        ],
    )?;
    fetch_pin_from_remote(
        &root,
        &home,
        &dolt_bin,
        &cache,
        &format!("file://{}", alternate_remote.display()),
        release_ref,
        &commit1,
        &fetch_status,
    )?;
    probe_sql_scalar(
        &root,
        &home,
        &dolt_bin,
        &cache,
        release_ref,
        &commit1,
        &fetch_status,
        "select v from t as of 'fishystuff/gitops/example-release' where pk = 1",
        "one",
        &admission_status,
    )?;

    let admission: Value = read_json(&admission_status)?;
    assert_eq!(admission["admission_state"], "passed_fixture");
    assert_eq!(admission["verified_commit"], commit1);
    assert_eq!(admission["scalar"], "one");
    let admission_request = root.path().join("requests/admission.json");
    assert_helper_current(
        &home,
        [
            "dolt",
            "needs-probe-sql-scalar",
            "--request",
            admission_request
                .to_str()
                .context("admission request path is not UTF-8")?,
            "--status",
            admission_status
                .to_str()
                .context("admission status path is not UTF-8")?,
            "--dolt-bin",
            dolt_bin.to_str().context("dolt path is not UTF-8")?,
        ],
    )?;
    let stale_admission_status = root.path().join("status/admission-stale-query.json");
    let mut stale_admission = admission.clone();
    stale_admission["query"] = Value::String("select 'wrong'".to_owned());
    write_json(&stale_admission_status, stale_admission)?;
    assert_helper_needs(
        &home,
        [
            "dolt",
            "needs-probe-sql-scalar",
            "--request",
            admission_request
                .to_str()
                .context("admission request path is not UTF-8")?,
            "--status",
            stale_admission_status
                .to_str()
                .context("stale admission status path is not UTF-8")?,
            "--dolt-bin",
            dolt_bin.to_str().context("dolt path is not UTF-8")?,
        ],
    )?;

    assert_probe_failure_contains(
        probe_sql_scalar(
            &root,
            &home,
            &dolt_bin,
            &cache,
            release_ref,
            &commit1,
            &fetch_status,
            "select v from t as of 'fishystuff/gitops/example-release' where pk = 1",
            "wrong",
            &root.path().join("status/admission-wrong-scalar.json"),
        ),
        "expected scalar",
    );

    let stale_commit_status = root.path().join("status/fetch-stale-commit.json");
    write_fetch_status(
        &stale_commit_status,
        "not-the-requested-commit",
        &cache,
        release_ref,
    )?;
    assert_probe_failure_contains(
        probe_sql_scalar(
            &root,
            &home,
            &dolt_bin,
            &cache,
            release_ref,
            &commit1,
            &stale_commit_status,
            "select v from t as of 'fishystuff/gitops/example-release' where pk = 1",
            "one",
            &root.path().join("status/admission-stale-commit.json"),
        ),
        "does not match requested",
    );

    let wrong_cache_status = root.path().join("status/fetch-wrong-cache.json");
    write_fetch_status(
        &wrong_cache_status,
        &commit1,
        &root.path().join("other-cache"),
        release_ref,
    )?;
    assert_probe_failure_contains(
        probe_sql_scalar(
            &root,
            &home,
            &dolt_bin,
            &cache,
            release_ref,
            &commit1,
            &wrong_cache_status,
            "select v from t as of 'fishystuff/gitops/example-release' where pk = 1",
            "one",
            &root.path().join("status/admission-wrong-cache.json"),
        ),
        "does not match admission cache",
    );

    let wrong_ref_status = root.path().join("status/fetch-wrong-ref.json");
    write_fetch_status(
        &wrong_ref_status,
        &commit1,
        &cache,
        "fishystuff/gitops/other",
    )?;
    assert_probe_failure_contains(
        probe_sql_scalar(
            &root,
            &home,
            &dolt_bin,
            &cache,
            release_ref,
            &commit1,
            &wrong_ref_status,
            "select v from t as of 'fishystuff/gitops/example-release' where pk = 1",
            "one",
            &root.path().join("status/admission-wrong-ref.json"),
        ),
        "does not match admission release ref",
    );

    fs::write(cache.join("cache-survives-fetch"), "yes")?;
    run(
        &dolt_bin,
        &home,
        Some(&source),
        ["sql", "-q", "insert into t values (2, 'two');"],
    )?;
    run(&dolt_bin, &home, Some(&source), ["add", "t"])?;
    run(
        &dolt_bin,
        &home,
        Some(&source),
        ["commit", "-m", "commit-two"],
    )?;
    run(&dolt_bin, &home, Some(&source), ["push", "origin", "main"])?;
    let commit2 = dolt_hash_of(&dolt_bin, &home, &source, "main")?;

    let fake_commit2_status = root.path().join("status/fetch-fake-commit2.json");
    write_fetch_status(&fake_commit2_status, &commit2, &cache, release_ref)?;
    assert_probe_failure_contains(
        probe_sql_scalar(
            &root,
            &home,
            &dolt_bin,
            &cache,
            release_ref,
            &commit2,
            &fake_commit2_status,
            "select v from t as of 'fishystuff/gitops/example-release' where pk = 2",
            "two",
            &root.path().join("status/admission-fake-commit2.json"),
        ),
        "expected Dolt commit",
    );

    fetch_pin(
        &root,
        &home,
        &dolt_bin,
        &cache,
        release_ref,
        &commit2,
        &fetch_status,
    )?;
    probe_sql_scalar(
        &root,
        &home,
        &dolt_bin,
        &cache,
        release_ref,
        &commit2,
        &fetch_status,
        "select v from t as of 'fishystuff/gitops/example-release' where pk = 2",
        "two",
        &admission_status,
    )?;

    let fetch: Value = read_json(&fetch_status)?;
    let admission: Value = read_json(&admission_status)?;
    assert_eq!(fetch["state"], "pinned");
    assert_eq!(fetch["verified_commit"], commit2);
    assert_eq!(admission["admission_state"], "passed_fixture");
    assert_eq!(admission["verified_commit"], commit2);
    assert_eq!(admission["scalar"], "two");
    assert!(cache.join("cache-survives-fetch").is_file());

    Ok(())
}

fn assert_probe_failure_contains(result: Result<()>, expected: &str) {
    let error = result.expect_err("probe unexpectedly succeeded");
    let message = format!("{error:#}");
    assert!(
        message.contains(expected),
        "expected error containing {expected:?}, got:\n{message}"
    );
}

fn find_dolt() -> Result<Option<PathBuf>> {
    let output = Command::new("dolt").arg("version").output();
    match output {
        Ok(output) if output.status.success() => Ok(Some(PathBuf::from("dolt"))),
        Ok(output) => bail_command("dolt version", output),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).context("running dolt version"),
    }
}

fn fetch_pin(
    root: &TestRoot,
    home: &Path,
    dolt_bin: &Path,
    cache: &Path,
    release_ref: &str,
    commit: &str,
    status: &Path,
) -> Result<()> {
    fetch_pin_from_remote(
        root,
        home,
        dolt_bin,
        cache,
        &format!("file://{}", root.path().join("remote").display()),
        release_ref,
        commit,
        status,
    )
}

fn fetch_pin_from_remote(
    root: &TestRoot,
    home: &Path,
    dolt_bin: &Path,
    cache: &Path,
    remote_url: &str,
    release_ref: &str,
    commit: &str,
    status: &Path,
) -> Result<()> {
    let request = root.path().join("requests/fetch.json");
    write_json(
        &request,
        json!({
            "environment": "local-test",
            "host": "vm-single-host",
            "release_id": "example-release",
            "release_identity": format!("release=example-release;dolt_commit={commit}"),
            "repository": "fishystuff/fishystuff",
            "remote_url": remote_url,
            "branch_context": "main",
            "commit": commit,
            "access_mode": "read_only",
            "materialization": "fetch_pin",
            "cache_dir": cache,
            "release_ref": release_ref,
        }),
    )?;
    run_helper(
        home,
        [
            "dolt",
            "fetch-pin",
            "--request",
            request.to_str().context("request path is not UTF-8")?,
            "--status",
            status.to_str().context("status path is not UTF-8")?,
            "--dolt-bin",
            dolt_bin.to_str().context("dolt path is not UTF-8")?,
        ],
    )
}

fn origin_remote_url(dolt_bin: &Path, home: &Path, cache: &Path) -> Result<String> {
    let output = run(dolt_bin, home, Some(cache), ["remote", "-v"])?;
    output
        .lines()
        .find_map(|line| {
            let mut fields = line.split_whitespace();
            if fields.next() == Some("origin") {
                fields.next().map(str::to_owned)
            } else {
                None
            }
        })
        .context("origin remote was not configured")
}

#[allow(clippy::too_many_arguments)]
fn probe_sql_scalar(
    root: &TestRoot,
    home: &Path,
    dolt_bin: &Path,
    cache: &Path,
    release_ref: &str,
    commit: &str,
    dolt_status: &Path,
    query: &str,
    expected_scalar: &str,
    status: &Path,
) -> Result<()> {
    let request = root.path().join("requests/admission.json");
    write_json(
        &request,
        json!({
            "environment": "local-test",
            "host": "vm-single-host",
            "release_id": "example-release",
            "release_identity": format!("release=example-release;dolt_commit={commit}"),
            "expected_commit": commit,
            "materialization": "fetch_pin",
            "cache_dir": cache,
            "pinned_ref": release_ref,
            "materialization_status_path": dolt_status,
            "query": query,
            "expected_scalar": expected_scalar,
        }),
    )?;
    run_helper(
        home,
        [
            "dolt",
            "probe-sql-scalar",
            "--request",
            request.to_str().context("request path is not UTF-8")?,
            "--status",
            status.to_str().context("status path is not UTF-8")?,
            "--dolt-bin",
            dolt_bin.to_str().context("dolt path is not UTF-8")?,
        ],
    )
}

fn dolt_hash_of(dolt_bin: &Path, home: &Path, cwd: &Path, revision: &str) -> Result<String> {
    let output = run(
        dolt_bin,
        home,
        Some(cwd),
        [
            "sql",
            "-r",
            "csv",
            "-q",
            &format!("select dolt_hashof('{revision}') as hash"),
        ],
    )?;
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .nth(1)
        .map(str::to_owned)
        .context("Dolt hash query returned no hash row")
}

fn write_fetch_status(
    path: &Path,
    verified_commit: &str,
    cache: &Path,
    release_ref: &str,
) -> Result<()> {
    write_json(
        path,
        json!({
            "verified_commit": verified_commit,
            "cache_dir": cache,
            "release_ref": release_ref,
            "state": "pinned",
        }),
    )
}

fn run_helper<'a, I>(home: &Path, args: I) -> Result<()>
where
    I: IntoIterator<Item = &'a str>,
{
    let helper = env!("CARGO_BIN_EXE_fishystuff_deploy");
    run(Path::new(helper), home, None, args).map(|_| ())
}

fn assert_helper_needs<'a, I>(home: &Path, args: I) -> Result<()>
where
    I: IntoIterator<Item = &'a str>,
{
    let output = run_helper_raw(home, args)?;
    if !output.status.success() {
        return bail_command("fishystuff_deploy needs helper", output);
    }
    Ok(())
}

fn assert_helper_current<'a, I>(home: &Path, args: I) -> Result<()>
where
    I: IntoIterator<Item = &'a str>,
{
    let output = run_helper_raw(home, args)?;
    if output.status.code() != Some(1) {
        return bail_command("fishystuff_deploy needs helper", output);
    }
    Ok(())
}

fn run_helper_raw<'a, I>(home: &Path, args: I) -> Result<Output>
where
    I: IntoIterator<Item = &'a str>,
{
    let helper = env!("CARGO_BIN_EXE_fishystuff_deploy");
    let args: Vec<&str> = args.into_iter().collect();
    Command::new(helper)
        .args(&args)
        .env("HOME", home)
        .env("NO_COLOR", "1")
        .output()
        .with_context(|| format!("running {} {}", helper, args.join(" ")))
}

fn run<'a, I>(program: &Path, home: &Path, cwd: Option<&Path>, args: I) -> Result<String>
where
    I: IntoIterator<Item = &'a str>,
{
    let args: Vec<&str> = args.into_iter().collect();
    let mut command = Command::new(program);
    command.args(&args).env("HOME", home).env("NO_COLOR", "1");
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }

    let output = command
        .output()
        .with_context(|| format!("running {} {}", program.display(), args.join(" ")))?;
    if !output.status.success() {
        return bail_command(&format!("{} {}", program.display(), args.join(" ")), output);
    }
    String::from_utf8(output.stdout).context("command wrote non-UTF-8 stdout")
}

fn bail_command<T>(command: &str, output: Output) -> Result<T> {
    bail!(
        "command failed: {command}\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn write_json(path: &Path, value: Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(&value)?)?;
    Ok(())
}

fn read_json(path: &Path) -> Result<Value> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("decoding {}", path.display()))
}

struct TestRoot {
    path: PathBuf,
}

impl TestRoot {
    fn new(prefix: &str) -> Result<Self> {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("system time is before UNIX_EPOCH")?
            .as_millis();
        let path = std::env::temp_dir().join(format!("{prefix}-{}-{millis}", std::process::id()));
        fs::create_dir_all(&path)?;
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
