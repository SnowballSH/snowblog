use std::time::Instant;

use axum::extract::{MatchedPath, Request};
use axum::http::{Method, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use metrics_exporter_prometheus::{BuildError, Matcher, PrometheusBuilder, PrometheusHandle};
use snowblog_core::service::BlogService;
use snowblog_core::store::StoreError;

const HTTP_BUCKETS: [f64; 11] = [
    0.005, 0.010, 0.025, 0.050, 0.100, 0.250, 0.500, 1.0, 2.5, 5.0, 10.0,
];
const STORE_BUCKETS: [f64; 10] = [
    0.001, 0.0025, 0.005, 0.010, 0.025, 0.050, 0.100, 0.250, 0.500, 1.0,
];
const RENDER_BUCKETS: [f64; 8] = [0.050, 0.100, 0.250, 0.500, 1.0, 2.5, 5.0, 10.0];

fn prometheus_builder() -> Result<PrometheusBuilder, BuildError> {
    PrometheusBuilder::new()
        .set_buckets_for_metric(
            Matcher::Full("snowblog_http_request_duration_seconds".to_owned()),
            &HTTP_BUCKETS,
        )?
        .set_buckets_for_metric(
            Matcher::Full("snowblog_store_operation_duration_seconds".to_owned()),
            &STORE_BUCKETS,
        )?
        .set_buckets_for_metric(
            Matcher::Full("snowblog_render_duration_seconds".to_owned()),
            &RENDER_BUCKETS,
        )
}

pub fn install_prometheus_recorder() -> anyhow::Result<PrometheusHandle> {
    Ok(prometheus_builder()?.install_recorder()?)
}

pub async fn initialize_build_info(service: &BlogService) -> Result<(), StoreError> {
    let schema_version = service.store().schema_version().await?.to_string();
    metrics::gauge!(
        "snowblog_build_info",
        "service_version" => env!("CARGO_PKG_VERSION"),
        "renderer_version" => service.renderer().version(),
        "schema_version" => schema_version,
    )
    .set(1.0);
    Ok(())
}

pub async fn refresh_content_metrics(service: &BlogService) -> Result<(), StoreError> {
    let counts = service.store().content_counts().await?;
    for (status, count) in [
        ("draft", counts.draft),
        ("published", counts.published),
        ("archived", counts.archived),
    ] {
        metrics::gauge!("snowblog_content_posts", "status" => status).set(count as f64);
    }
    Ok(())
}

pub async fn record_http_request(request: Request, next: Next) -> Response {
    let route = normalize_route(request.extensions().get::<MatchedPath>());
    let method = normalize_method(request.method());
    let _in_flight = InFlightRequest::new(&route, method);
    let started_at = Instant::now();
    let response = next.run(request).await;
    let status = status_class(response.status());

    metrics::counter!(
        "snowblog_http_requests_total",
        "route" => route.clone(),
        "method" => method,
        "status" => status,
    )
    .increment(1);
    metrics::histogram!(
        "snowblog_http_request_duration_seconds",
        "route" => route,
        "method" => method,
        "status" => status,
    )
    .record(started_at.elapsed().as_secs_f64());

    response
}

struct InFlightRequest(metrics::Gauge);

impl InFlightRequest {
    fn new(route: &str, method: &'static str) -> Self {
        let gauge = metrics::gauge!(
            "snowblog_http_requests_in_flight",
            "route" => route.to_owned(),
            "method" => method,
        );
        gauge.increment(1.0);
        Self(gauge)
    }
}

impl Drop for InFlightRequest {
    fn drop(&mut self) {
        self.0.decrement(1.0);
    }
}

fn normalize_method(method: &Method) -> &'static str {
    match *method {
        Method::GET => "get",
        Method::HEAD => "head",
        Method::POST => "post",
        Method::PUT => "put",
        Method::PATCH => "patch",
        Method::DELETE => "delete",
        Method::OPTIONS => "options",
        _ => "other",
    }
}

fn normalize_route(matched_path: Option<&MatchedPath>) -> String {
    match matched_path.map(MatchedPath::as_str) {
        Some("/api/v1/admin/") | Some("/api/v1/admin/{*unmatched}") | None => {
            "unmatched".to_owned()
        }
        Some(route) => route.to_owned(),
    }
}

fn status_class(status: StatusCode) -> &'static str {
    match status.as_u16() {
        100..=199 => "1xx",
        200..=299 => "2xx",
        300..=399 => "3xx",
        400..=499 => "4xx",
        _ => "5xx",
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use metrics::with_local_recorder;

    use super::prometheus_builder;

    #[test]
    fn prometheus_histograms_use_the_exact_service_buckets() {
        let recorder = prometheus_builder()
            .expect("bucket configuration is valid")
            .build_recorder();
        let handle = recorder.handle();

        with_local_recorder(&recorder, || {
            metrics::histogram!("snowblog_http_request_duration_seconds").record(0.0);
            metrics::histogram!("snowblog_store_operation_duration_seconds").record(0.0);
            metrics::histogram!("snowblog_render_duration_seconds").record(0.0);
        });

        let exposition = handle.render();
        assert_boundaries(
            &exposition,
            "snowblog_http_request_duration_seconds",
            &[
                "0.005", "0.01", "0.025", "0.05", "0.1", "0.25", "0.5", "1", "2.5", "5", "10",
            ],
        );
        assert_boundaries(
            &exposition,
            "snowblog_store_operation_duration_seconds",
            &[
                "0.001", "0.0025", "0.005", "0.01", "0.025", "0.05", "0.1", "0.25", "0.5", "1",
            ],
        );
        assert_boundaries(
            &exposition,
            "snowblog_render_duration_seconds",
            &["0.05", "0.1", "0.25", "0.5", "1", "2.5", "5", "10"],
        );

        for family in [
            "snowblog_http_request_duration_seconds",
            "snowblog_store_operation_duration_seconds",
            "snowblog_render_duration_seconds",
        ] {
            assert!(
                !exposition.contains(&format!("{family}_bucket{{le=\"0.075\"}}")),
                "{family} used an exporter-default boundary"
            );
        }
    }

    fn assert_boundaries(exposition: &str, family: &str, expected: &[&str]) {
        let prefix = format!("{family}_bucket{{le=\"");
        let actual = exposition
            .lines()
            .filter_map(|line| {
                let boundary = line.strip_prefix(&prefix)?.split_once('"')?.0;
                (boundary != "+Inf").then(|| boundary.to_owned())
            })
            .collect::<BTreeSet<_>>();
        let expected = expected
            .iter()
            .map(|boundary| (*boundary).to_owned())
            .collect::<BTreeSet<_>>();
        assert_eq!(actual, expected, "wrong boundaries for {family}");
    }
}
