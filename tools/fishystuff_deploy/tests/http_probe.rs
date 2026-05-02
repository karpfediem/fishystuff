use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::thread::{self, JoinHandle};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

#[test]
fn http_status_probe_records_status_and_needs_state() -> Result<()> {
    let root = TestRoot::new("fishystuff-deploy-http-status")?;
    let (url, server) = spawn_server(TestResponse::plain(204, ""))?;
    let request = root.path().join("requests/readyz.json");
    let status = root.path().join("status/readyz.json");

    write_json(
        &request,
        json!({
            "environment": "local-test",
            "host": "vm-single-host",
            "release_id": "example-release",
            "release_identity": "release=example-release;api=example",
            "probe_name": "api-readyz",
            "url": format!("{url}/readyz"),
            "expected_status": 204,
            "timeout_ms": 2000,
        }),
    )?;

    run_helper([
        "http",
        "probe-status",
        "--request",
        request.to_str().context("request path is not UTF-8")?,
        "--status",
        status.to_str().context("status path is not UTF-8")?,
    ])?;
    server.join().expect("server thread panicked")?;

    let document = read_json(&status)?;
    assert_eq!(document["probe"], "http-status");
    assert_eq!(document["probe_name"], "api-readyz");
    assert_eq!(document["observed_status"], 204);
    assert_eq!(document["admission_state"], "passed_fixture");

    assert_helper_current([
        "http",
        "needs-probe-status",
        "--request",
        request.to_str().context("request path is not UTF-8")?,
        "--status",
        status.to_str().context("status path is not UTF-8")?,
    ])?;

    let stale_status = root.path().join("status/readyz-stale.json");
    let mut stale_document = document;
    stale_document["observed_status"] = Value::from(500);
    write_json(&stale_status, stale_document)?;
    assert_helper_needs([
        "http",
        "needs-probe-status",
        "--request",
        request.to_str().context("request path is not UTF-8")?,
        "--status",
        stale_status
            .to_str()
            .context("stale status path is not UTF-8")?,
    ])
}

#[test]
fn http_json_scalar_probe_records_exact_value() -> Result<()> {
    let root = TestRoot::new("fishystuff-deploy-http-json-scalar")?;
    let (url, server) = spawn_server(TestResponse::json(
        200,
        br#"{"meta":{"git_rev":"example","ready":true}}"#,
    ))?;
    let request = root.path().join("requests/meta.json");
    let status = root.path().join("status/meta.json");

    write_json(
        &request,
        json!({
            "environment": "local-test",
            "host": "vm-single-host",
            "release_id": "example-release",
            "release_identity": "release=example-release;api=example",
            "probe_name": "api-meta-git-rev",
            "url": format!("{url}/api/v1/meta"),
            "expected_status": 200,
            "timeout_ms": 2000,
            "json_pointer": "/meta/git_rev",
            "expected_scalar": "example",
        }),
    )?;

    run_helper([
        "http",
        "probe-json-scalar",
        "--request",
        request.to_str().context("request path is not UTF-8")?,
        "--status",
        status.to_str().context("status path is not UTF-8")?,
    ])?;
    server.join().expect("server thread panicked")?;

    let document = read_json(&status)?;
    assert_eq!(document["probe"], "http-json-scalar");
    assert_eq!(document["probe_name"], "api-meta-git-rev");
    assert_eq!(document["observed_status"], 200);
    assert_eq!(document["json_pointer"], "/meta/git_rev");
    assert_eq!(document["scalar"], "example");
    assert_eq!(document["expected_scalar"], "example");
    assert_eq!(document["admission_state"], "passed_fixture");

    assert_helper_current([
        "http",
        "needs-probe-json-scalar",
        "--request",
        request.to_str().context("request path is not UTF-8")?,
        "--status",
        status.to_str().context("status path is not UTF-8")?,
    ])
}

#[test]
fn http_json_scalar_probe_rejects_wrong_value() -> Result<()> {
    let root = TestRoot::new("fishystuff-deploy-http-json-wrong")?;
    let (url, server) = spawn_server(TestResponse::json(200, br#"{"meta":{"git_rev":"actual"}}"#))?;
    let request = root.path().join("requests/meta.json");
    let status = root.path().join("status/meta.json");

    write_json(
        &request,
        json!({
            "environment": "local-test",
            "host": "vm-single-host",
            "release_id": "example-release",
            "release_identity": "release=example-release;api=example",
            "probe_name": "api-meta-git-rev",
            "url": format!("{url}/api/v1/meta"),
            "expected_status": 200,
            "json_pointer": "/meta/git_rev",
            "expected_scalar": "expected",
        }),
    )?;

    assert_helper_failure_contains(
        [
            "http",
            "probe-json-scalar",
            "--request",
            request.to_str().context("request path is not UTF-8")?,
            "--status",
            status.to_str().context("status path is not UTF-8")?,
        ],
        "expected scalar",
    )?;
    server.join().expect("server thread panicked")
}

#[test]
fn http_probe_rejects_remote_or_credential_bearing_url() -> Result<()> {
    let root = TestRoot::new("fishystuff-deploy-http-reject-url")?;
    let request = root.path().join("requests/remote.json");
    let status = root.path().join("status/remote.json");

    write_json(
        &request,
        json!({
            "environment": "local-test",
            "host": "vm-single-host",
            "release_id": "example-release",
            "release_identity": "release=example-release;api=example",
            "probe_name": "api-readyz",
            "url": "http://example.invalid/readyz",
            "expected_status": 200,
        }),
    )?;
    assert_helper_failure_contains(
        [
            "http",
            "probe-status",
            "--request",
            request.to_str().context("request path is not UTF-8")?,
            "--status",
            status.to_str().context("status path is not UTF-8")?,
        ],
        "host must be localhost",
    )?;

    write_json(
        &request,
        json!({
            "environment": "local-test",
            "host": "vm-single-host",
            "release_id": "example-release",
            "release_identity": "release=example-release;api=example",
            "probe_name": "api-readyz",
            "url": "http://token@127.0.0.1/readyz",
            "expected_status": 200,
        }),
    )?;
    assert_helper_failure_contains(
        [
            "http",
            "probe-status",
            "--request",
            request.to_str().context("request path is not UTF-8")?,
            "--status",
            status.to_str().context("status path is not UTF-8")?,
        ],
        "must not contain credentials",
    )
}

struct TestResponse {
    status: u16,
    content_type: &'static str,
    body: Vec<u8>,
}

impl TestResponse {
    fn plain(status: u16, body: &str) -> Self {
        Self {
            status,
            content_type: "text/plain",
            body: body.as_bytes().to_vec(),
        }
    }

    fn json(status: u16, body: &[u8]) -> Self {
        Self {
            status,
            content_type: "application/json",
            body: body.to_vec(),
        }
    }
}

fn spawn_server(response: TestResponse) -> Result<(String, JoinHandle<Result<()>>)> {
    let listener = TcpListener::bind("127.0.0.1:0").context("binding local test HTTP server")?;
    let addr = listener
        .local_addr()
        .context("reading local HTTP server addr")?;
    let handle = thread::spawn(move || {
        let (stream, _) = listener.accept().context("accepting local HTTP request")?;
        serve_response(stream, response)
    });
    Ok((format!("http://{addr}"), handle))
}

fn serve_response(mut stream: TcpStream, response: TestResponse) -> Result<()> {
    let mut request_bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let read = stream
            .read(&mut buffer)
            .context("reading test HTTP request")?;
        if read == 0 {
            break;
        }
        request_bytes.extend_from_slice(&buffer[..read]);
        if request_bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    let response_bytes = format!(
        "HTTP/1.1 {} OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        response.status,
        response.content_type,
        response.body.len()
    );
    stream
        .write_all(response_bytes.as_bytes())
        .context("writing test HTTP response headers")?;
    stream
        .write_all(&response.body)
        .context("writing test HTTP response body")
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

fn run_helper<'a, I>(args: I) -> Result<()>
where
    I: IntoIterator<Item = &'a str>,
{
    let output = run_helper_raw(args)?;
    if !output.status.success() {
        return bail_command("fishystuff_deploy helper", output);
    }
    Ok(())
}

fn assert_helper_needs<'a, I>(args: I) -> Result<()>
where
    I: IntoIterator<Item = &'a str>,
{
    let output = run_helper_raw(args)?;
    if !output.status.success() {
        return bail_command("fishystuff_deploy needs helper", output);
    }
    Ok(())
}

fn assert_helper_current<'a, I>(args: I) -> Result<()>
where
    I: IntoIterator<Item = &'a str>,
{
    let output = run_helper_raw(args)?;
    if output.status.code() != Some(1) {
        return bail_command("fishystuff_deploy needs helper", output);
    }
    Ok(())
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
