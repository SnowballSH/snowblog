use metrics_exporter_prometheus::{BuildError, Matcher, PrometheusBuilder, PrometheusHandle};

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
