mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use axum::middleware;
use axum::routing::get as route_get;
use common::{TEST_TOKEN, app_with_admin, get, send_raw};
use metrics::set_default_local_recorder;
use metrics_exporter_prometheus::PrometheusBuilder;
use snowblog_core::domain::{Language, PostStatus, Revision, Slug};
use snowblog_core::store::{NewPost, TranslationInput};
use snowblog_server::telemetry::install_prometheus_recorder;
use snowblog_server::telemetry::{initialize_build_info, refresh_content_metrics};
use tokio::sync::Barrier;
use tower::ServiceExt;

// Break caught: changing the stable HTTP metric vocabulary, including route
// fallback canonicalization, bounded method/status normalization, or labels.
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
                .method(Method::HEAD)
                .uri("/api/v1/health")
                .body(Body::empty())
                .expect("the request is valid"),
        )
        .await
        .status(),
        StatusCode::OK
    );
    assert_eq!(
        send_raw(
            &app,
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/api/v1/health")
                .body(Body::empty())
                .expect("the request is valid"),
        )
        .await
        .status(),
        StatusCode::METHOD_NOT_ALLOWED
    );
    assert_eq!(
        send_raw(
            &app,
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/admin/private-admin-path?token=admin-secret")
                .header(header::AUTHORIZATION, format!("Bearer {TEST_TOKEN}"))
                .body(Body::empty())
                .expect("the request is valid"),
        )
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
    let app_samples = vec![
        (
            labels(&[
                ("route", "/api/v1/health"),
                ("method", "get"),
                ("status", "2xx"),
            ]),
            1.0,
        ),
        (
            labels(&[
                ("route", "/api/v1/health"),
                ("method", "head"),
                ("status", "2xx"),
            ]),
            1.0,
        ),
        (
            labels(&[
                ("route", "/api/v1/health"),
                ("method", "options"),
                ("status", "4xx"),
            ]),
            1.0,
        ),
        (
            labels(&[
                ("route", "/api/v1/posts/{slug}"),
                ("method", "get"),
                ("status", "4xx"),
            ]),
            1.0,
        ),
        (
            labels(&[
                ("route", "/api/v1/admin/posts"),
                ("method", "post"),
                ("status", "4xx"),
            ]),
            1.0,
        ),
        (
            labels(&[("route", "unmatched"), ("method", "get"), ("status", "4xx")]),
            2.0,
        ),
        (
            labels(&[
                ("route", "unmatched"),
                ("method", "other"),
                ("status", "4xx"),
            ]),
            1.0,
        ),
    ];
    let app_in_flight_samples = vec![
        (
            labels(&[("route", "/api/v1/health"), ("method", "get")]),
            0.0,
        ),
        (
            labels(&[("route", "/api/v1/health"), ("method", "head")]),
            0.0,
        ),
        (
            labels(&[("route", "/api/v1/health"), ("method", "options")]),
            0.0,
        ),
        (
            labels(&[("route", "/api/v1/posts/{slug}"), ("method", "get")]),
            0.0,
        ),
        (
            labels(&[("route", "/api/v1/admin/posts"), ("method", "post")]),
            0.0,
        ),
        (labels(&[("route", "unmatched"), ("method", "get")]), 0.0),
        (labels(&[("route", "unmatched"), ("method", "other")]), 0.0),
    ];
    assert_family_samples(&exposition, "snowblog_http_requests_total", &app_samples);
    assert_family_samples(
        &exposition,
        "snowblog_http_request_duration_seconds_count",
        &app_samples,
    );
    for forbidden in [
        "private-slug",
        "raw-secret-path",
        "token",
        "bearer-secret",
        "private-admin-path",
        "admin-secret",
    ] {
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
    let mut held_in_flight_samples = app_in_flight_samples.clone();
    held_in_flight_samples.push((labels(&[("route", "/hold"), ("method", "get")]), 1.0));
    assert_family_samples(
        &handle.render(),
        "snowblog_http_requests_in_flight",
        &held_in_flight_samples,
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
    held_in_flight_samples
        .last_mut()
        .expect("the held request sample exists")
        .1 = 0.0;
    assert_family_samples(
        &handle.render(),
        "snowblog_http_requests_in_flight",
        &held_in_flight_samples,
    );

    let custom_status_router = Router::new()
        .route(
            "/custom-status",
            route_get(|| async { StatusCode::from_u16(600).expect("600 is a custom status") }),
        )
        .layer(middleware::from_fn(
            snowblog_server::telemetry::record_http_request,
        ));
    assert_eq!(
        custom_status_router
            .oneshot(get("/custom-status"))
            .await
            .expect("the custom-status request succeeds")
            .status(),
        StatusCode::from_u16(600).expect("600 is a custom status")
    );
    let mut final_counter_samples = app_samples;
    final_counter_samples.push((
        labels(&[("route", "/hold"), ("method", "get"), ("status", "5xx")]),
        1.0,
    ));
    final_counter_samples.push((
        labels(&[
            ("route", "/custom-status"),
            ("method", "get"),
            ("status", "5xx"),
        ]),
        1.0,
    ));
    assert_family_samples(
        &handle.render(),
        "snowblog_http_requests_total",
        &final_counter_samples,
    );
}

#[tokio::test(flavor = "current_thread")]
async fn content_gauges_reconcile_all_statuses_and_build_info_is_bounded() {
    let recorder = PrometheusBuilder::new().build_recorder();
    let handle = recorder.handle();
    let _recorder_guard = set_default_local_recorder(&recorder);
    let app = app_with_admin().await;
    let service = app.service().await;

    initialize_build_info(&service).await.unwrap();
    refresh_content_metrics(&service).await.unwrap();
    assert_content_samples(&handle.render(), 0.0, 0.0, 0.0);

    let private_slug = Slug::parse("private_content_gauge_slug").unwrap();
    service
        .store()
        .create_post(NewPost {
            slug: private_slug.clone(),
            default_language: Language::parse("en").unwrap(),
            tags: vec!["private-content-tag".into()],
            published_at: None,
        })
        .await
        .unwrap();
    refresh_content_metrics(&service).await.unwrap();
    refresh_content_metrics(&service).await.unwrap();
    assert_content_samples(&handle.render(), 1.0, 0.0, 0.0);

    service
        .save_translation(
            &private_slug,
            Revision(1),
            TranslationInput {
                language: Language::parse("en").unwrap(),
                title: "Private content title".into(),
                description: "Private content description".into(),
                source: "= private_content_source".into(),
            },
        )
        .await
        .unwrap();
    refresh_content_metrics(&service).await.unwrap();
    assert_content_samples(&handle.render(), 1.0, 0.0, 0.0);

    service.publish(&private_slug, Revision(2)).await.unwrap();
    refresh_content_metrics(&service).await.unwrap();
    assert_content_samples(&handle.render(), 0.0, 1.0, 0.0);

    service
        .set_status(&private_slug, Revision(3), PostStatus::Archived)
        .await
        .unwrap();
    refresh_content_metrics(&service).await.unwrap();
    refresh_content_metrics(&service).await.unwrap();
    let exposition = handle.render();
    assert_content_samples(&exposition, 0.0, 0.0, 1.0);
    assert_family_samples(
        &exposition,
        "snowblog_build_info",
        &[(
            (labels(&[
                ("service_version", env!("CARGO_PKG_VERSION")),
                ("renderer_version", service.renderer().version()),
                (
                    "schema_version",
                    &service.store().schema_version().await.unwrap().to_string(),
                ),
            ])),
            1.0,
        )],
    );
    for forbidden in [
        "private_content_gauge_slug",
        "private-content-tag",
        "Private content title",
        "Private content description",
        "private_content_source",
    ] {
        assert!(!exposition.contains(forbidden), "leaked {forbidden}");
    }
}

fn assert_content_samples(exposition: &str, draft: f64, published: f64, archived: f64) {
    assert_family_samples(
        exposition,
        "snowblog_content_posts",
        &[
            (labels(&[("status", "draft")]), draft),
            (labels(&[("status", "published")]), published),
            (labels(&[("status", "archived")]), archived),
        ],
    );
}

type Labels = BTreeSet<(String, String)>;

fn assert_family_samples(exposition: &str, family: &str, expected: &[(Labels, f64)]) {
    let actual = exposition
        .lines()
        .filter_map(|line| {
            let labeled_sample = line.strip_prefix(&format!("{family}{{"))?;
            let (serialized_labels, value) = labeled_sample.split_once("} ")?;
            let labels = serialized_labels
                .split(',')
                .map(|label| {
                    let (name, quoted_value) =
                        label.split_once('=').expect("Prometheus label has a value");
                    let value = quoted_value
                        .strip_prefix('\"')
                        .and_then(|value| value.strip_suffix('\"'))
                        .expect("Prometheus label value is quoted");
                    (name.to_owned(), value.to_owned())
                })
                .collect();
            Some((
                labels,
                value.parse::<f64>().expect("Prometheus sample is numeric"),
            ))
        })
        .collect::<BTreeMap<_, _>>();
    let expected = expected.iter().cloned().collect::<BTreeMap<_, _>>();
    assert_eq!(actual, expected, "wrong {family} samples");
}

fn labels(values: &[(&str, &str)]) -> Labels {
    values
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect()
}
