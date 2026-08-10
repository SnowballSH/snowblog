mod common;

use std::collections::BTreeSet;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use axum::middleware;
use axum::routing::get as route_get;
use common::{app_with_admin, get, send_raw};
use snowblog_server::telemetry::install_prometheus_recorder;
use tokio::sync::Barrier;
use tower::ServiceExt;

// Break caught: omitting HTTP instrumentation, recording raw request data, or
// classifying requests with unbounded labels.
#[tokio::test(flavor = "current_thread")]
async fn http_metrics_use_normalized_routes_and_bounded_labels() {
    let handle = install_prometheus_recorder().expect("the test installs one recorder");
    let app = app_with_admin().await;

    assert_eq!(
        send_raw(&app, get("/api/v1/health")).await.status(),
        StatusCode::OK
    );
    assert_eq!(
        send_raw(&app, get("/api/v1/posts/private-slug"))
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send_raw(
            &app,
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/admin/posts")
                .body(Body::empty())
                .expect("the request is valid"),
        )
        .await
        .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send_raw(
            &app,
            Request::builder()
                .method(Method::from_bytes(b"BREW").expect("BREW is a valid extension method"))
                .uri("/api/v1/raw-secret-path?token=bearer-secret")
                .body(Body::empty())
                .expect("the request is valid"),
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );

    let metrics_response = send_raw(&app, get("/metrics")).await;
    assert_eq!(metrics_response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        metrics_response.headers()[header::CONTENT_TYPE],
        "application/problem+json"
    );

    let exposition = handle.render();
    for (path, method, status) in [
        ("/api/v1/health", "get", "2xx"),
        ("/api/v1/posts/{slug}", "get", "4xx"),
        ("/api/v1/admin/posts", "post", "4xx"),
        ("unmatched", "other", "4xx"),
    ] {
        assert_sample(
            &exposition,
            "snowblog_http_requests_total",
            &[("path", path), ("method", method), ("status", status)],
            1.0,
        );
        assert_sample(
            &exposition,
            "snowblog_http_request_duration_seconds_count",
            &[("path", path), ("method", method), ("status", status)],
            1.0,
        );
    }
    assert_sample(
        &exposition,
        "snowblog_http_requests_total",
        &[("path", "unmatched"), ("method", "get"), ("status", "4xx")],
        1.0,
    );

    assert_eq!(
        label_values(&exposition, "snowblog_http_requests_total", "path"),
        set(&[
            "/api/v1/admin/posts",
            "/api/v1/health",
            "/api/v1/posts/{slug}",
            "unmatched",
        ])
    );
    assert_eq!(
        label_values(&exposition, "snowblog_http_requests_total", "method"),
        set(&["get", "other", "post"])
    );
    assert_eq!(
        label_values(&exposition, "snowblog_http_requests_total", "status"),
        set(&["2xx", "4xx"])
    );
    for forbidden in ["private-slug", "raw-secret-path", "token", "bearer-secret"] {
        assert!(!exposition.contains(forbidden), "leaked {forbidden}");
    }

    let entered = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let held_router = Router::new()
        .route(
            "/hold",
            route_get({
                let entered = Arc::clone(&entered);
                let release = Arc::clone(&release);
                move || {
                    let entered = Arc::clone(&entered);
                    let release = Arc::clone(&release);
                    async move {
                        entered.wait().await;
                        release.wait().await;
                        StatusCode::INTERNAL_SERVER_ERROR
                    }
                }
            }),
        )
        .layer(middleware::from_fn(
            snowblog_server::telemetry::record_http_request,
        ));
    let held_request = tokio::spawn(held_router.oneshot(get("/hold")));

    entered.wait().await;
    assert_sample(
        &handle.render(),
        "snowblog_http_requests_in_flight",
        &[("path", "/hold"), ("method", "get")],
        1.0,
    );
    release.wait().await;
    assert_eq!(
        held_request
            .await
            .expect("the held request task completes")
            .expect("the held request succeeds")
            .status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_sample(
        &handle.render(),
        "snowblog_http_requests_in_flight",
        &[("path", "/hold"), ("method", "get")],
        0.0,
    );
}

fn assert_sample(exposition: &str, family: &str, labels: &[(&str, &str)], expected: f64) {
    let line = exposition
        .lines()
        .filter(|line| line.starts_with(family))
        .find(|line| {
            labels
                .iter()
                .all(|(name, value)| line.contains(&format!("{name}=\"{value}\"")))
        })
        .unwrap_or_else(|| panic!("missing {family} sample for {labels:?}"));
    let actual = line
        .rsplit_once(' ')
        .expect("Prometheus sample has a value")
        .1
        .parse::<f64>()
        .expect("Prometheus sample value is numeric");
    assert_eq!(actual, expected, "wrong {family} value for {labels:?}");
}

fn label_values(exposition: &str, family: &str, label: &str) -> BTreeSet<String> {
    let marker = format!("{label}=\"");
    exposition
        .lines()
        .filter(|line| line.starts_with(family))
        .filter_map(|line| {
            let value = line.split_once(&marker)?.1;
            Some(value.split_once('\"')?.0.to_owned())
        })
        .collect()
}

fn set(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}
