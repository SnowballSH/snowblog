mod common;

use axum::http::StatusCode;
use common::{app_without_admin, get, send, send_raw};

#[tokio::test]
async fn health_reports_versions_and_database() {
    let app = app_without_admin().await;
    let (status, body) = send(&app, get("/api/v1/health")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["database"], "ok");
    assert_eq!(body["renderer_version"], "0.15.1");
    assert!(body["service_version"].is_string());
}

#[tokio::test]
async fn unknown_route_returns_problem_json() {
    let app = app_without_admin().await;
    let response = send_raw(&app, get("/api/v1/nope")).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        response.headers()["content-type"],
        "application/problem+json"
    );
    let (_, body) = send(&app, get("/api/v1/nope")).await;
    assert_eq!(body["code"], "not_found");
    assert_eq!(body["status"], 404);
}
