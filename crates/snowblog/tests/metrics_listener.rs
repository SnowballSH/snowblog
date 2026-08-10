use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use tempfile::TempDir;

const STARTUP_TIMEOUT: Duration = Duration::from_secs(10);
const RETRY_INTERVAL: Duration = Duration::from_millis(25);

// Break caught: SNOWBLOG_METRICS_LISTEN no longer starts the private Prometheus
// listener, or that listener exposes anything beyond GET /metrics while the API
// router keeps its normal problem+json /metrics fallback.
#[test]
fn metrics_listener_is_opt_in_and_private() {
    let temp_dir = TempDir::new().expect("temporary database directory is created");

    let disabled_api = unused_loopback_address();
    let disabled = spawn_server(disabled_api, temp_dir.path().join("disabled.db"), None);
    let api_health = wait_for_response(disabled_api, "/api/v1/health");
    assert_eq!(api_health.status, 200);
    drop(disabled);

    let [enabled_api, enabled_metrics] = distinct_loopback_addresses();
    let enabled = spawn_server(
        enabled_api,
        temp_dir.path().join("enabled.db"),
        Some(enabled_metrics),
    );

    let initial_metrics = wait_for_response(enabled_metrics, "/metrics");
    assert_eq!(initial_metrics.status, 200);
    assert_eq!(
        initial_metrics
            .headers
            .get("content-type")
            .map(String::as_str),
        Some("text/plain; version=0.0.4; charset=utf-8")
    );
    assert!(initial_metrics.body.contains("snowblog_build_info{"));
    assert!(initial_metrics.body.contains("snowblog_content_posts{"));
    assert_plain_empty_404(wait_for_response(enabled_metrics, "/api/v1/health"));
    assert_plain_empty_404(wait_for_method_response(
        HttpMethod::Head,
        enabled_metrics,
        "/metrics",
    ));
    assert_plain_empty_404(wait_for_method_response(
        HttpMethod::Post,
        enabled_metrics,
        "/metrics",
    ));

    let health = wait_for_response(enabled_api, "/api/v1/health");
    assert_eq!(health.status, 200);
    let metrics = wait_for_body(enabled_metrics, "/metrics", |body| {
        body.lines().any(|line| {
            line.starts_with("snowblog_http_requests_total{")
                && line.contains("method=\"get\"")
                && line.contains("route=\"/api/v1/health\"")
                && line.contains("status=\"2xx\"")
        })
    });
    assert!(metrics.body.contains("snowblog_http_requests_total{"));

    let api_metrics = wait_for_response(enabled_api, "/metrics");
    assert_eq!(api_metrics.status, 404);
    assert_eq!(
        api_metrics.headers.get("content-type").map(String::as_str),
        Some("application/problem+json")
    );

    drop(enabled);
}

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn spawn_server(
    api_address: SocketAddr,
    database: std::path::PathBuf,
    metrics_address: Option<SocketAddr>,
) -> ChildGuard {
    let mut command = Command::new(env!("CARGO_BIN_EXE_snowblog"));
    command
        .args([
            "serve",
            "--listen",
            &api_address.to_string(),
            "--database",
            database.to_str().expect("temporary path is UTF-8"),
        ])
        .env_remove("SNOWBLOG_METRICS_LISTEN")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(metrics_address) = metrics_address {
        command.env("SNOWBLOG_METRICS_LISTEN", metrics_address.to_string());
    }
    ChildGuard(command.spawn().expect("snowblog subprocess starts"))
}

fn distinct_loopback_addresses() -> [SocketAddr; 2] {
    let first = TcpListener::bind("127.0.0.1:0").expect("first loopback listener binds");
    let second = TcpListener::bind("127.0.0.1:0").expect("second loopback listener binds");
    [
        first.local_addr().expect("first listener has an address"),
        second.local_addr().expect("second listener has an address"),
    ]
}

fn unused_loopback_address() -> SocketAddr {
    TcpListener::bind("127.0.0.1:0")
        .expect("ephemeral loopback listener binds")
        .local_addr()
        .expect("ephemeral listener has an address")
}

fn wait_for_response(address: SocketAddr, path: &str) -> HttpResponse {
    wait_for_method_response(HttpMethod::Get, address, path)
}

fn wait_for_method_response(method: HttpMethod, address: SocketAddr, path: &str) -> HttpResponse {
    wait_for_method_body(method, address, path, |_| true)
}

fn wait_for_body(
    address: SocketAddr,
    path: &str,
    predicate: impl Fn(&str) -> bool,
) -> HttpResponse {
    wait_for_method_body(HttpMethod::Get, address, path, predicate)
}

fn wait_for_method_body(
    method: HttpMethod,
    address: SocketAddr,
    path: &str,
    predicate: impl Fn(&str) -> bool,
) -> HttpResponse {
    let deadline = Instant::now() + STARTUP_TIMEOUT;
    loop {
        if let Ok(response) = request(method, address, path)
            && predicate(&response.body)
        {
            return response;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {path} on {address}"
        );
        thread::sleep(RETRY_INTERVAL);
    }
}

#[derive(Clone, Copy)]
enum HttpMethod {
    Get,
    Head,
    Post,
}

impl HttpMethod {
    fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Head => "HEAD",
            Self::Post => "POST",
        }
    }
}

fn request(method: HttpMethod, address: SocketAddr, path: &str) -> std::io::Result<HttpResponse> {
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_millis(250))?;
    stream.set_read_timeout(Some(Duration::from_secs(1)))?;
    write!(
        stream,
        "{} {path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n",
        method.as_str()
    )?;
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes)?;
    HttpResponse::parse(&bytes)
}

fn assert_plain_empty_404(response: HttpResponse) {
    assert_eq!(response.status, 404);
    assert!(response.body.is_empty());
    assert!(!response.headers.contains_key("content-type"));
}

struct HttpResponse {
    status: u16,
    headers: BTreeMap<String, String>,
    body: String,
}

impl HttpResponse {
    fn parse(bytes: &[u8]) -> std::io::Result<Self> {
        let response = String::from_utf8_lossy(bytes);
        let (head, body) = response.split_once("\r\n\r\n").ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "missing HTTP header end")
        })?;
        let mut lines = head.lines();
        let status = lines
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|status| status.parse().ok())
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid HTTP status")
            })?;
        let headers = lines
            .filter_map(|line| line.split_once(':'))
            .map(|(name, value)| (name.to_ascii_lowercase(), value.trim().to_owned()))
            .collect();
        Ok(Self {
            status,
            headers,
            body: body.to_owned(),
        })
    }
}
