use std::fs::{self, File};
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::Path;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const DEFAULT_TIMEOUT_MS: u64 = 2_000;
const MAX_BODY_BYTES: usize = 1024 * 1024;

#[derive(Debug, Deserialize)]
struct HttpProbeCommonRequest {
    environment: String,
    host: String,
    release_id: String,
    release_identity: String,
    probe_name: String,
    url: String,
    expected_status: u16,
    #[serde(default = "default_timeout_ms")]
    timeout_ms: u64,
}

#[derive(Debug, Deserialize)]
struct HttpStatusRequest {
    #[serde(flatten)]
    common: HttpProbeCommonRequest,
}

#[derive(Debug, Deserialize)]
struct HttpJsonScalarRequest {
    #[serde(flatten)]
    common: HttpProbeCommonRequest,
    json_pointer: String,
    expected_scalar: Value,
}

#[derive(Debug, Deserialize)]
struct HttpStatusStatusDocument {
    environment: String,
    host: String,
    release_id: String,
    release_identity: String,
    probe_name: String,
    url: String,
    expected_status: u16,
    observed_status: u16,
    admission_state: String,
    probe: String,
}

#[derive(Debug, Serialize)]
struct HttpStatusStatus<'a> {
    environment: &'a str,
    host: &'a str,
    release_id: &'a str,
    release_identity: &'a str,
    probe_name: &'a str,
    url: &'a str,
    expected_status: u16,
    observed_status: u16,
    admission_state: &'static str,
    probe: &'static str,
}

#[derive(Debug, Deserialize)]
struct HttpJsonScalarStatusDocument {
    environment: String,
    host: String,
    release_id: String,
    release_identity: String,
    probe_name: String,
    url: String,
    expected_status: u16,
    observed_status: u16,
    json_pointer: String,
    expected_scalar: Value,
    scalar: Value,
    admission_state: String,
    probe: String,
}

#[derive(Debug, Serialize)]
struct HttpJsonScalarStatus<'a> {
    environment: &'a str,
    host: &'a str,
    release_id: &'a str,
    release_identity: &'a str,
    probe_name: &'a str,
    url: &'a str,
    expected_status: u16,
    observed_status: u16,
    json_pointer: &'a str,
    expected_scalar: &'a Value,
    scalar: &'a Value,
    admission_state: &'static str,
    probe: &'static str,
}

struct ParsedHttpUrl {
    host: String,
    port: u16,
    path: String,
    host_header: String,
}

struct HttpResponse {
    status: u16,
    body: Vec<u8>,
}

pub fn probe_status(request_path: &Path, status_path: &Path) -> Result<()> {
    let request = read_http_status_request(request_path)?;
    validate_common_request(&request.common)?;

    let response = get_loopback_url(&request.common)?;
    if response.status != request.common.expected_status {
        bail!(
            "HTTP status probe {} expected status {}, got {}",
            request.common.probe_name,
            request.common.expected_status,
            response.status
        );
    }

    write_http_status(status_path, &request, response.status)
}

pub fn needs_probe_status(request_path: &Path, status_path: &Path) -> bool {
    let Ok(request) = read_http_status_request(request_path).and_then(|request| {
        validate_common_request(&request.common)?;
        Ok(request)
    }) else {
        return true;
    };
    let Ok(status) = read_http_status_status(status_path) else {
        return true;
    };
    !http_status_matches_request(&request, &status)
}

pub fn probe_json_scalar(request_path: &Path, status_path: &Path) -> Result<()> {
    let request = read_http_json_scalar_request(request_path)?;
    validate_common_request(&request.common)?;
    validate_json_scalar_request(&request)?;

    let response = get_loopback_url(&request.common)?;
    if response.status != request.common.expected_status {
        bail!(
            "HTTP JSON scalar probe {} expected status {}, got {}",
            request.common.probe_name,
            request.common.expected_status,
            response.status
        );
    }

    let body: Value = serde_json::from_slice(&response.body).with_context(|| {
        format!(
            "HTTP JSON scalar probe {} response body was not JSON",
            request.common.probe_name
        )
    })?;
    let scalar = body
        .pointer(&request.json_pointer)
        .with_context(|| {
            format!(
                "HTTP JSON scalar probe {} did not find JSON pointer {}",
                request.common.probe_name, request.json_pointer
            )
        })?
        .clone();
    if !is_json_scalar(&scalar) {
        bail!(
            "HTTP JSON scalar probe {} JSON pointer {} resolved to a non-scalar value",
            request.common.probe_name,
            request.json_pointer
        );
    }
    if scalar != request.expected_scalar {
        bail!(
            "HTTP JSON scalar probe {} expected scalar {}, got {}",
            request.common.probe_name,
            json_value_for_message(&request.expected_scalar),
            json_value_for_message(&scalar)
        );
    }

    write_http_json_scalar(status_path, &request, response.status, &scalar)
}

pub fn needs_probe_json_scalar(request_path: &Path, status_path: &Path) -> bool {
    let Ok(request) = read_http_json_scalar_request(request_path).and_then(|request| {
        validate_common_request(&request.common)?;
        validate_json_scalar_request(&request)?;
        Ok(request)
    }) else {
        return true;
    };
    let Ok(status) = read_http_json_scalar_status(status_path) else {
        return true;
    };
    !http_json_scalar_matches_request(&request, &status)
}

fn read_http_status_request(request_path: &Path) -> Result<HttpStatusRequest> {
    let file = File::open(request_path).with_context(|| {
        format!(
            "opening HTTP status probe request {}",
            request_path.display()
        )
    })?;
    serde_json::from_reader(file).with_context(|| {
        format!(
            "decoding HTTP status probe request {}",
            request_path.display()
        )
    })
}

fn read_http_json_scalar_request(request_path: &Path) -> Result<HttpJsonScalarRequest> {
    let file = File::open(request_path).with_context(|| {
        format!(
            "opening HTTP JSON scalar probe request {}",
            request_path.display()
        )
    })?;
    serde_json::from_reader(file).with_context(|| {
        format!(
            "decoding HTTP JSON scalar probe request {}",
            request_path.display()
        )
    })
}

fn read_http_status_status(status_path: &Path) -> Result<HttpStatusStatusDocument> {
    let file = File::open(status_path)
        .with_context(|| format!("opening HTTP status probe status {}", status_path.display()))?;
    serde_json::from_reader(file).with_context(|| {
        format!(
            "decoding HTTP status probe status {}",
            status_path.display()
        )
    })
}

fn read_http_json_scalar_status(status_path: &Path) -> Result<HttpJsonScalarStatusDocument> {
    let file = File::open(status_path).with_context(|| {
        format!(
            "opening HTTP JSON scalar probe status {}",
            status_path.display()
        )
    })?;
    serde_json::from_reader(file).with_context(|| {
        format!(
            "decoding HTTP JSON scalar probe status {}",
            status_path.display()
        )
    })
}

fn validate_common_request(request: &HttpProbeCommonRequest) -> Result<()> {
    require_non_empty("environment", &request.environment)?;
    require_non_empty("host", &request.host)?;
    require_non_empty("release_id", &request.release_id)?;
    require_non_empty("release_identity", &request.release_identity)?;
    require_non_empty("probe_name", &request.probe_name)?;
    require_non_empty("url", &request.url)?;

    if !(100..=599).contains(&request.expected_status) {
        bail!(
            "HTTP probe {} expected_status must be between 100 and 599",
            request.probe_name
        );
    }
    if !(1..=30_000).contains(&request.timeout_ms) {
        bail!(
            "HTTP probe {} timeout_ms must be between 1 and 30000",
            request.probe_name
        );
    }
    parse_loopback_http_url(&request.url)?;
    Ok(())
}

fn validate_json_scalar_request(request: &HttpJsonScalarRequest) -> Result<()> {
    if !request.json_pointer.is_empty() && !request.json_pointer.starts_with('/') {
        bail!(
            "HTTP JSON scalar probe {} json_pointer must be empty or start with /",
            request.common.probe_name
        );
    }
    if !is_json_scalar(&request.expected_scalar) {
        bail!(
            "HTTP JSON scalar probe {} expected_scalar must be a JSON scalar",
            request.common.probe_name
        );
    }
    Ok(())
}

fn http_status_matches_request(
    request: &HttpStatusRequest,
    status: &HttpStatusStatusDocument,
) -> bool {
    status.environment == request.common.environment
        && status.host == request.common.host
        && status.release_id == request.common.release_id
        && status.release_identity == request.common.release_identity
        && status.probe_name == request.common.probe_name
        && status.url == request.common.url
        && status.expected_status == request.common.expected_status
        && status.observed_status == request.common.expected_status
        && status.admission_state == "passed_fixture"
        && status.probe == "http-status"
}

fn http_json_scalar_matches_request(
    request: &HttpJsonScalarRequest,
    status: &HttpJsonScalarStatusDocument,
) -> bool {
    status.environment == request.common.environment
        && status.host == request.common.host
        && status.release_id == request.common.release_id
        && status.release_identity == request.common.release_identity
        && status.probe_name == request.common.probe_name
        && status.url == request.common.url
        && status.expected_status == request.common.expected_status
        && status.observed_status == request.common.expected_status
        && status.json_pointer == request.json_pointer
        && status.expected_scalar == request.expected_scalar
        && status.scalar == request.expected_scalar
        && status.admission_state == "passed_fixture"
        && status.probe == "http-json-scalar"
}

fn write_http_status(
    status_path: &Path,
    request: &HttpStatusRequest,
    observed_status: u16,
) -> Result<()> {
    ensure_parent_dir(status_path)?;
    let status = HttpStatusStatus {
        environment: &request.common.environment,
        host: &request.common.host,
        release_id: &request.common.release_id,
        release_identity: &request.common.release_identity,
        probe_name: &request.common.probe_name,
        url: &request.common.url,
        expected_status: request.common.expected_status,
        observed_status,
        admission_state: "passed_fixture",
        probe: "http-status",
    };
    let mut file = File::create(status_path).with_context(|| {
        format!(
            "creating HTTP status probe status {}",
            status_path.display()
        )
    })?;
    serde_json::to_writer_pretty(&mut file, &status)
        .with_context(|| format!("writing HTTP status probe status {}", status_path.display()))?;
    file.write_all(b"\n").with_context(|| {
        format!(
            "finalizing HTTP status probe status {}",
            status_path.display()
        )
    })
}

fn write_http_json_scalar(
    status_path: &Path,
    request: &HttpJsonScalarRequest,
    observed_status: u16,
    scalar: &Value,
) -> Result<()> {
    ensure_parent_dir(status_path)?;
    let status = HttpJsonScalarStatus {
        environment: &request.common.environment,
        host: &request.common.host,
        release_id: &request.common.release_id,
        release_identity: &request.common.release_identity,
        probe_name: &request.common.probe_name,
        url: &request.common.url,
        expected_status: request.common.expected_status,
        observed_status,
        json_pointer: &request.json_pointer,
        expected_scalar: &request.expected_scalar,
        scalar,
        admission_state: "passed_fixture",
        probe: "http-json-scalar",
    };
    let mut file = File::create(status_path).with_context(|| {
        format!(
            "creating HTTP JSON scalar probe status {}",
            status_path.display()
        )
    })?;
    serde_json::to_writer_pretty(&mut file, &status).with_context(|| {
        format!(
            "writing HTTP JSON scalar probe status {}",
            status_path.display()
        )
    })?;
    file.write_all(b"\n").with_context(|| {
        format!(
            "finalizing HTTP JSON scalar probe status {}",
            status_path.display()
        )
    })
}

fn get_loopback_url(request: &HttpProbeCommonRequest) -> Result<HttpResponse> {
    let url = parse_loopback_http_url(&request.url)?;
    let timeout = Duration::from_millis(request.timeout_ms);
    let mut stream = connect_loopback(&url, timeout)?;
    stream
        .set_read_timeout(Some(timeout))
        .context("setting HTTP probe read timeout")?;
    stream
        .set_write_timeout(Some(timeout))
        .context("setting HTTP probe write timeout")?;

    let request_bytes = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: fishystuff_deploy/0.1\r\nAccept: application/json\r\nConnection: close\r\n\r\n",
        url.path, url.host_header
    );
    stream
        .write_all(request_bytes.as_bytes())
        .with_context(|| format!("writing HTTP probe request to {}", request.url))?;

    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let read = stream
            .read(&mut buffer)
            .with_context(|| format!("reading HTTP probe response from {}", request.url))?;
        if read == 0 {
            break;
        }
        if bytes.len() + read > MAX_BODY_BYTES {
            bail!(
                "HTTP probe {} response exceeded {} bytes",
                request.probe_name,
                MAX_BODY_BYTES
            );
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    parse_http_response(&bytes)
}

fn connect_loopback(url: &ParsedHttpUrl, timeout: Duration) -> Result<TcpStream> {
    let addrs = (url.host.as_str(), url.port)
        .to_socket_addrs()
        .with_context(|| format!("resolving loopback HTTP probe host {}", url.host))?;
    let mut last_error = None;
    for addr in addrs {
        if !addr.ip().is_loopback() {
            bail!("HTTP probe resolved non-loopback address {}", addr);
        }
        match TcpStream::connect_timeout(&addr, timeout) {
            Ok(stream) => return Ok(stream),
            Err(error) => last_error = Some(error),
        }
    }
    match last_error {
        Some(error) => Err(error)
            .with_context(|| format!("connecting to loopback HTTP probe target {}", url.host)),
        None => bail!("HTTP probe host {} resolved to no addresses", url.host),
    }
}

fn parse_http_response(bytes: &[u8]) -> Result<HttpResponse> {
    let header_end =
        find_subslice(bytes, b"\r\n\r\n").context("HTTP response had no header end")?;
    let header_bytes = &bytes[..header_end];
    let body_start = header_end + 4;
    let headers =
        std::str::from_utf8(header_bytes).context("HTTP response headers were not UTF-8")?;
    let mut lines = headers.split("\r\n");
    let status_line = lines.next().context("HTTP response had no status line")?;
    let mut status_fields = status_line.split_whitespace();
    let protocol = status_fields
        .next()
        .context("HTTP response status line had no protocol")?;
    if !protocol.starts_with("HTTP/") {
        bail!("HTTP response status line did not start with HTTP/");
    }
    let status = status_fields
        .next()
        .context("HTTP response status line had no status code")?
        .parse::<u16>()
        .context("HTTP response status code was not numeric")?;

    let mut chunked = false;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.eq_ignore_ascii_case("transfer-encoding")
            && value
                .split(',')
                .any(|encoding| encoding.trim().eq_ignore_ascii_case("chunked"))
        {
            chunked = true;
        }
    }

    let body = if chunked {
        decode_chunked_body(&bytes[body_start..])?
    } else {
        bytes[body_start..].to_vec()
    };
    Ok(HttpResponse { status, body })
}

fn parse_loopback_http_url(url: &str) -> Result<ParsedHttpUrl> {
    if url.chars().any(char::is_whitespace) {
        bail!("HTTP probe URL must not contain whitespace");
    }
    if url.contains('#') {
        bail!("HTTP probe URL must not contain a fragment");
    }
    let rest = url
        .strip_prefix("http://")
        .context("HTTP probe URL must use http://")?;
    let split_index = rest.find(['/', '?']).unwrap_or(rest.len());
    let authority = &rest[..split_index];
    let suffix = &rest[split_index..];
    if authority.is_empty() {
        bail!("HTTP probe URL must include a host");
    }
    if authority.contains('@') {
        bail!("HTTP probe URL must not contain credentials");
    }

    let (host, port) = parse_authority(authority)?;
    if !matches!(host.as_str(), "localhost" | "127.0.0.1" | "::1") {
        bail!("HTTP probe URL host must be localhost, 127.0.0.1, or ::1");
    }
    let path = if suffix.is_empty() {
        "/".to_owned()
    } else if suffix.starts_with('?') {
        format!("/{suffix}")
    } else {
        suffix.to_owned()
    };
    let host_header = if host == "::1" {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    };

    Ok(ParsedHttpUrl {
        host,
        port,
        path,
        host_header,
    })
}

fn parse_authority(authority: &str) -> Result<(String, u16)> {
    if let Some(rest) = authority.strip_prefix('[') {
        let (host, after_host) = rest
            .split_once(']')
            .context("bracketed HTTP probe IPv6 host was not closed")?;
        if after_host.is_empty() {
            return Ok((host.to_owned(), 80));
        }
        let port = after_host
            .strip_prefix(':')
            .context("bracketed HTTP probe IPv6 host had unexpected suffix")?
            .parse::<u16>()
            .context("HTTP probe URL port was not numeric")?;
        return Ok((host.to_owned(), port));
    }

    if authority.matches(':').count() > 1 {
        bail!("HTTP probe IPv6 hosts must be bracketed");
    }
    let Some((host, port)) = authority.rsplit_once(':') else {
        return Ok((authority.to_owned(), 80));
    };
    if host.is_empty() {
        bail!("HTTP probe URL host must not be empty");
    }
    let port = port
        .parse::<u16>()
        .context("HTTP probe URL port was not numeric")?;
    Ok((host.to_owned(), port))
}

fn decode_chunked_body(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut decoded = Vec::new();
    let mut cursor = 0;
    loop {
        let line_end = find_subslice(&bytes[cursor..], b"\r\n")
            .map(|offset| cursor + offset)
            .context("chunked HTTP body had no chunk-size line end")?;
        let size_line = std::str::from_utf8(&bytes[cursor..line_end])
            .context("chunked HTTP body size line was not UTF-8")?;
        let size_hex = size_line.split(';').next().unwrap_or_default().trim();
        let size =
            usize::from_str_radix(size_hex, 16).context("chunked HTTP body size was invalid")?;
        cursor = line_end + 2;
        if size == 0 {
            break;
        }
        let chunk_end = cursor + size;
        if chunk_end + 2 > bytes.len() {
            bail!("chunked HTTP body ended before chunk data was complete");
        }
        decoded.extend_from_slice(&bytes[cursor..chunk_end]);
        cursor = chunk_end;
        if bytes.get(cursor..cursor + 2) != Some(b"\r\n") {
            bail!("chunked HTTP body chunk was not followed by CRLF");
        }
        cursor += 2;
    }
    Ok(decoded)
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn require_non_empty(field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("HTTP probe {field} must not be empty");
    }
    Ok(())
}

fn is_json_scalar(value: &Value) -> bool {
    !matches!(value, Value::Array(_) | Value::Object(_))
}

fn json_value_for_message(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "<unprintable>".to_owned())
}

fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating parent directory {}", parent.display()))?;
    }
    Ok(())
}

fn default_timeout_ms() -> u64 {
    DEFAULT_TIMEOUT_MS
}

#[cfg(test)]
mod tests {
    use super::{decode_chunked_body, parse_loopback_http_url};

    #[test]
    fn parses_loopback_http_urls() {
        let parsed = parse_loopback_http_url("http://127.0.0.1:1990/readyz").unwrap();
        assert_eq!(parsed.host, "127.0.0.1");
        assert_eq!(parsed.port, 1990);
        assert_eq!(parsed.path, "/readyz");

        let parsed = parse_loopback_http_url("http://localhost:8080?health=1").unwrap();
        assert_eq!(parsed.host, "localhost");
        assert_eq!(parsed.port, 8080);
        assert_eq!(parsed.path, "/?health=1");

        let parsed = parse_loopback_http_url("http://[::1]:8080/readyz").unwrap();
        assert_eq!(parsed.host, "::1");
        assert_eq!(parsed.host_header, "[::1]:8080");
    }

    #[test]
    fn rejects_non_loopback_http_urls() {
        assert!(parse_loopback_http_url("https://127.0.0.1/readyz").is_err());
        assert!(parse_loopback_http_url("http://example.invalid/readyz").is_err());
        assert!(parse_loopback_http_url("http://token@127.0.0.1/readyz").is_err());
        assert!(parse_loopback_http_url("http://127.0.0.1/readyz#fragment").is_err());
    }

    #[test]
    fn decodes_chunked_http_body() {
        let decoded = decode_chunked_body(b"7\r\n{\"ok\":1\r\n1\r\n}\r\n0\r\n\r\n").unwrap();
        assert_eq!(decoded, br#"{"ok":1}"#);
    }
}
