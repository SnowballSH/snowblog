#![allow(dead_code)]

use std::path::{Path, PathBuf};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, Response, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::Value;
use snowblog_server::{Config, build_app};
use tempfile::TempDir;
use tower::ServiceExt;

pub const TEST_TOKEN: &str = "test-token";

pub struct TestApp {
    pub router: Router,
    _dir: TempDir,
}

pub async fn app_with_admin() -> TestApp {
    build(true).await
}

pub async fn app_without_admin() -> TestApp {
    build(false).await
}

async fn build(with_admin: bool) -> TestApp {
    let dir = TempDir::new().unwrap();
    let token_file = dir.path().join("admin-token");
    if with_admin {
        std::fs::write(&token_file, format!("{TEST_TOKEN}\n")).unwrap();
    }
    let config = Config {
        listen: "127.0.0.1:0".parse().unwrap(),
        database: dir.path().join("test.db"),
        admin_token_file: with_admin.then(|| token_file.clone()),
        package_root: package_root(),
        font_dirs: Vec::new(),
        asset_url_template: "/api/v1/posts/{slug}/assets/".to_string(),
        max_source_bytes: 512 * 1024,
        max_asset_bytes: 64 * 1024,
        max_html_bytes: 2 * 1024 * 1024,
        render_timeout_secs: 30,
    };
    let router = build_app(config).await.unwrap();
    TestApp { router, _dir: dir }
}

pub fn package_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/packages")
}

pub async fn send(app: &TestApp, request: Request<Body>) -> (StatusCode, Value) {
    let response = app.router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes)
            .unwrap_or(Value::String(String::from_utf8_lossy(&bytes).to_string()))
    };
    (status, body)
}

pub async fn send_raw(app: &TestApp, request: Request<Body>) -> Response<Body> {
    app.router.clone().oneshot(request).await.unwrap()
}

pub fn get(uri: &str) -> Request<Body> {
    Request::builder().uri(uri).body(Body::empty()).unwrap()
}

pub fn admin_json(method: &str, uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {TEST_TOKEN}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

pub fn with_if_match(mut request: Request<Body>, revision: i64) -> Request<Body> {
    request
        .headers_mut()
        .insert(header::IF_MATCH, format!("\"{revision}\"").parse().unwrap());
    request
}
