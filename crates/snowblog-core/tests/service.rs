use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use metrics::set_default_local_recorder;
use metrics_exporter_prometheus::PrometheusBuilder;
use snowblog_core::domain::{Language, PostStatus, Revision, Slug};
use snowblog_core::render::{RenderLimits, RenderOutcome, Renderer};
use snowblog_core::service::{
    BlogService, Freshness, RenderStatus, RerenderOutcome, RerenderScope, ServiceError,
};
use snowblog_core::store::{AssetInput, NewPost, Store, TranslationInput};

fn slug(s: &str) -> Slug {
    Slug::parse(s).unwrap()
}

fn lang(s: &str) -> Language {
    Language::parse(s).unwrap()
}

fn package_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/packages")
}

async fn service() -> BlogService {
    let store = Store::in_memory().await.unwrap();
    let renderer = Arc::new(Renderer::new(
        package_root(),
        Vec::new(),
        RenderLimits::default(),
    ));
    BlogService::new(store, renderer, Some("/api/v1/posts/{slug}/assets/".into()))
}

async fn draft(service: &BlogService, name: &str) -> Slug {
    let s = slug(name);
    service
        .store()
        .create_post(NewPost {
            slug: s.clone(),
            default_language: lang("en"),
            tags: vec![],
            published_at: None,
        })
        .await
        .unwrap();
    s
}

fn translation(language: &str, source: &str) -> TranslationInput {
    TranslationInput {
        language: lang(language),
        title: format!("Title {language}"),
        description: String::new(),
        source: source.to_string(),
    }
}

fn png() -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/blogs/assets/blue_square.png"),
    )
    .unwrap()
}

#[tokio::test]
async fn save_translation_renders_and_stores() {
    let service = service().await;
    let s = draft(&service, "renders").await;
    let outcome = service
        .save_translation(&s, Revision(1), translation("en", "= Hello"))
        .await
        .unwrap();
    assert!(matches!(outcome.renders[0].render, RenderStatus::Ok { .. }));
    let render = outcome.record.render(&lang("en")).unwrap();
    assert!(render.html.contains("Hello"));
    assert_eq!(render.renderer_version, "0.15.1");
    let freshness = service.freshness(&outcome.record);
    assert_eq!(freshness[0].freshness, Freshness::Fresh);
}

#[tokio::test]
async fn broken_source_saves_without_render() {
    let service = service().await;
    let s = draft(&service, "broken").await;
    let outcome = service
        .save_translation(&s, Revision(1), translation("en", "#undefined_fn()"))
        .await
        .unwrap();
    match &outcome.renders[0].render {
        RenderStatus::Failed { diagnostics } => assert!(!diagnostics.is_empty()),
        other => panic!("expected failure, got {other:?}"),
    }
    assert_eq!(
        outcome.record.translation(&lang("en")).unwrap().source,
        "#undefined_fn()"
    );
    assert!(outcome.record.render(&lang("en")).is_none());
    assert_eq!(
        service.freshness(&outcome.record)[0].freshness,
        Freshness::Missing
    );
}

#[tokio::test]
async fn failed_save_keeps_previous_render() {
    let service = service().await;
    let s = draft(&service, "keeps_old").await;
    let good = service
        .save_translation(&s, Revision(1), translation("en", "= Good"))
        .await
        .unwrap();
    let good_html = good.record.render(&lang("en")).unwrap().html.clone();

    let bad = service
        .save_translation(&s, Revision(2), translation("en", "#undefined_fn()"))
        .await
        .unwrap();
    assert!(matches!(bad.renders[0].render, RenderStatus::Failed { .. }));
    let kept = bad.record.render(&lang("en")).unwrap();
    assert_eq!(kept.html, good_html);
    assert_eq!(
        service.freshness(&bad.record)[0].freshness,
        Freshness::Stale
    );
}

#[tokio::test]
async fn asset_upsert_rerenders_and_changes_hash() {
    let service = service().await;
    let s = draft(&service, "asset_flow").await;
    let first = service
        .save_translation(
            &s,
            Revision(1),
            translation("en", "#image(\"assets/pic.png\")"),
        )
        .await
        .unwrap();
    assert!(
        matches!(first.renders[0].render, RenderStatus::Failed { .. }),
        "render should fail before the asset exists"
    );

    let with_asset = service
        .save_asset(
            &s,
            Revision(2),
            AssetInput {
                path: "assets/pic.png".into(),
                content: png(),
                content_type: "image/png".into(),
            },
        )
        .await
        .unwrap();
    assert!(matches!(
        with_asset.renders[0].render,
        RenderStatus::Ok { .. }
    ));
    let html = &with_asset.record.render(&lang("en")).unwrap().html;
    assert!(
        html.contains("/api/v1/posts/asset_flow/assets/assets/pic.png"),
        "asset url not rewritten: {html}"
    );
}

#[tokio::test]
async fn preview_does_not_persist() {
    let service = service().await;
    let s = draft(&service, "previewed").await;
    service
        .save_translation(&s, Revision(1), translation("en", "= Saved"))
        .await
        .unwrap();

    let outcome = service.preview(&s, "= Previewed".into()).await.unwrap();
    match outcome {
        RenderOutcome::Success { html, .. } => assert!(html.contains("Previewed")),
        RenderOutcome::Failure { diagnostics } => panic!("preview failed: {diagnostics:?}"),
    }

    let record = service.store().get_post(&s).await.unwrap().unwrap();
    assert_eq!(record.post.revision, Revision(2), "preview must not mutate");
    assert!(record.render(&lang("en")).unwrap().html.contains("Saved"));
}

#[tokio::test]
async fn publish_requires_fresh_renders() {
    let service = service().await;
    let s = draft(&service, "gated").await;
    service
        .save_translation(&s, Revision(1), translation("en", "= Good"))
        .await
        .unwrap();
    service
        .save_translation(&s, Revision(2), translation("zh", "#undefined_fn()"))
        .await
        .unwrap();

    let blocked = service.publish(&s, Revision(3)).await;
    match blocked {
        Err(ServiceError::PublishBlocked(entries)) => {
            let zh = entries.iter().find(|e| e.language == lang("zh")).unwrap();
            assert_eq!(zh.freshness, Freshness::Missing);
        }
        other => panic!("expected PublishBlocked, got {other:?}"),
    }

    service
        .save_translation(&s, Revision(3), translation("zh", "= Fixed"))
        .await
        .unwrap();
    let published = service.publish(&s, Revision(4)).await.unwrap();
    assert_eq!(published.post.status, PostStatus::Published);
    assert!(published.post.published_at.is_some());
}

#[tokio::test]
async fn stale_published_post_stays_published() {
    let service = service().await;
    let s = draft(&service, "stale_pub").await;
    service
        .save_translation(&s, Revision(1), translation("en", "= V1"))
        .await
        .unwrap();
    service.publish(&s, Revision(2)).await.unwrap();

    let edited = service
        .save_translation(&s, Revision(3), translation("en", "#undefined_fn()"))
        .await
        .unwrap();
    assert_eq!(edited.record.post.status, PostStatus::Published);
    assert_eq!(
        service.freshness(&edited.record)[0].freshness,
        Freshness::Stale
    );
    assert!(
        edited
            .record
            .render(&lang("en"))
            .unwrap()
            .html
            .contains("V1")
    );
}

#[tokio::test]
async fn rerender_stale_fixes_only_stale() {
    let service = service().await;
    let fresh_slug = draft(&service, "fresh_post").await;
    service
        .save_translation(&fresh_slug, Revision(1), translation("en", "= Fresh"))
        .await
        .unwrap();

    let stale_slug = draft(&service, "stale_post").await;
    service
        .save_translation(&stale_slug, Revision(1), translation("en", "= Old"))
        .await
        .unwrap();
    service
        .store()
        .upsert_translation(&stale_slug, Revision(2), translation("en", "= New content"))
        .await
        .unwrap();

    let reports = service.rerender(RerenderScope::Stale).await.unwrap();
    let by_slug = |name: &str| {
        reports
            .iter()
            .find(|r| r.slug.as_str() == name)
            .unwrap()
            .outcome
            .clone()
    };
    assert_eq!(by_slug("fresh_post"), RerenderOutcome::SkippedFresh);
    assert_eq!(by_slug("stale_post"), RerenderOutcome::Rerendered);

    let record = service
        .store()
        .get_post(&stale_slug)
        .await
        .unwrap()
        .unwrap();
    assert!(
        record
            .render(&lang("en"))
            .unwrap()
            .html
            .contains("New content")
    );
}

#[tokio::test]
async fn publish_blocked_without_default_translation() {
    let service = service().await;
    let s = draft(&service, "no_default").await;
    service
        .save_translation(&s, Revision(1), translation("zh", "= Chinese only"))
        .await
        .unwrap();
    let result = service.publish(&s, Revision(2)).await;
    assert!(
        matches!(result, Err(ServiceError::PublishBlocked(_))),
        "publishing without a default-language translation must be blocked: {result:?}"
    );
}

#[tokio::test]
async fn slug_rename_marks_renders_stale() {
    let service = service().await;
    let s = draft(&service, "renamable").await;
    service
        .save_translation(&s, Revision(1), translation("en", "= Content"))
        .await
        .unwrap();
    service
        .store()
        .update_post_meta(
            &s,
            Revision(2),
            snowblog_core::store::PostPatch {
                slug: Some(slug("renamed_now")),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let record = service
        .store()
        .get_post(&slug("renamed_now"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        service.freshness(&record)[0].freshness,
        Freshness::Stale,
        "renders bake the slug into asset URLs, so a rename must read as stale"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn preview_render_metrics_record_success_and_failure_without_private_values() {
    let service = service().await;
    let post_slug = draft(&service, "private_preview_slug").await;
    service
        .store()
        .upsert_asset(
            &post_slug,
            Revision(1),
            AssetInput {
                path: "assets/private_asset_filename.png".into(),
                content: png(),
                content_type: "image/png".into(),
            },
        )
        .await
        .unwrap();
    let recorder = test_recorder();
    let handle = recorder.handle();
    let _recorder_guard = set_default_local_recorder(&recorder);

    let success = service
        .preview(
            &post_slug,
            "private_preview_source #image(\"assets/private_asset_filename.png\")".into(),
        )
        .await
        .unwrap();
    assert!(matches!(success, RenderOutcome::Success { .. }));
    let failure = service
        .preview(
            &post_slug,
            "#image(\"assets/private_diagnostic_filename.png\")".into(),
        )
        .await
        .unwrap();
    match failure {
        RenderOutcome::Failure { diagnostics } => assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("private_diagnostic_filename.png")
        })),
        RenderOutcome::Success { .. } => panic!("missing private asset unexpectedly rendered"),
    }

    let exposition = handle.render();
    assert_eq!(
        metric_sample(
            &exposition,
            "snowblog_render_attempts_total",
            &[("operation", "preview"), ("outcome", "success")],
        ),
        1.0
    );
    assert_eq!(
        metric_sample(
            &exposition,
            "snowblog_render_attempts_total",
            &[("operation", "preview"), ("outcome", "failure")],
        ),
        1.0
    );
    assert_eq!(
        metric_sample(
            &exposition,
            "snowblog_render_duration_seconds_count",
            &[("operation", "preview"), ("result", "success")],
        ),
        1.0
    );
    assert_eq!(
        metric_sample(
            &exposition,
            "snowblog_render_duration_seconds_count",
            &[("operation", "preview"), ("result", "failure")],
        ),
        1.0
    );
    assert_eq!(
        metric_label_values(&exposition, "snowblog_render_attempts_total", "operation"),
        string_set(&["preview"])
    );
    assert_eq!(
        metric_label_values(&exposition, "snowblog_render_attempts_total", "outcome"),
        string_set(&["failure", "success"])
    );
    assert_eq!(
        metric_label_values(
            &exposition,
            "snowblog_render_duration_seconds_count",
            "result"
        ),
        string_set(&["failure", "success"])
    );
    for forbidden in [
        "private_preview_slug",
        "private_preview_source",
        "private_asset_filename.png",
        "private_diagnostic_filename.png",
    ] {
        assert!(!exposition.contains(forbidden), "leaked {forbidden}");
    }
}

#[tokio::test(flavor = "current_thread")]
async fn persisted_render_metrics_record_success_and_failure_without_private_values() {
    let service = service().await;
    let post_slug = draft(&service, "private_persisted_slug").await;
    let recorder = test_recorder();
    let handle = recorder.handle();
    let _recorder_guard = set_default_local_recorder(&recorder);

    let success = service
        .save_translation(
            &post_slug,
            Revision(1),
            translation("en", "= private_persisted_source"),
        )
        .await
        .unwrap();
    assert!(matches!(success.renders[0].render, RenderStatus::Ok { .. }));
    let failure = service
        .save_translation(
            &post_slug,
            Revision(2),
            translation("en", "#image(\"assets/private_persisted_diagnostic.png\")"),
        )
        .await
        .unwrap();
    match &failure.renders[0].render {
        RenderStatus::Failed { diagnostics } => assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("private_persisted_diagnostic.png")
        })),
        RenderStatus::Ok { .. } => panic!("missing private asset unexpectedly rendered"),
    }

    let exposition = handle.render();
    for (outcome, result) in [("success", "success"), ("failure", "failure")] {
        assert_eq!(
            metric_sample(
                &exposition,
                "snowblog_render_attempts_total",
                &[("operation", "persisted"), ("outcome", outcome)],
            ),
            1.0
        );
        assert_eq!(
            metric_sample(
                &exposition,
                "snowblog_render_duration_seconds_count",
                &[("operation", "persisted"), ("result", result)],
            ),
            1.0
        );
    }
    assert_eq!(
        metric_label_values(&exposition, "snowblog_render_attempts_total", "operation"),
        string_set(&["persisted"])
    );
    for forbidden in [
        "private_persisted_slug",
        "private_persisted_source",
        "private_persisted_diagnostic.png",
    ] {
        assert!(!exposition.contains(forbidden), "leaked {forbidden}");
    }
}

#[tokio::test(flavor = "current_thread")]
async fn revision_change_discards_successful_render_and_records_successful_duration() {
    let service = service().await;
    let post_slug = draft(&service, "private_discarded_slug").await;
    let recorder = test_recorder();
    let handle = recorder.handle();
    let _recorder_guard = set_default_local_recorder(&recorder);
    let slow_source = format!(
        "#let private_content = [{}]\n= Finished",
        "private_discarded_source ".repeat(15_000)
    );
    let render_task = tokio::spawn({
        let service = service.clone();
        let post_slug = post_slug.clone();
        async move {
            service
                .save_translation(&post_slug, Revision(1), translation("en", &slow_source))
                .await
        }
    });

    loop {
        let revision = service
            .store()
            .get_post(&post_slug)
            .await
            .unwrap()
            .unwrap()
            .post
            .revision;
        if revision == Revision(2) {
            break;
        }
        assert!(
            !render_task.is_finished(),
            "render finished before revision changed"
        );
        tokio::task::yield_now().await;
    }
    service
        .store()
        .update_post_meta(
            &post_slug,
            Revision(2),
            snowblog_core::store::PostPatch::default(),
        )
        .await
        .unwrap();
    let outcome = render_task.await.unwrap().unwrap();
    assert!(matches!(outcome.renders[0].render, RenderStatus::Ok { .. }));
    assert_eq!(outcome.record.post.revision, Revision(3));
    assert!(outcome.record.render(&lang("en")).is_none());

    let exposition = handle.render();
    assert_eq!(
        metric_sample(
            &exposition,
            "snowblog_render_attempts_total",
            &[("operation", "persisted"), ("outcome", "discarded")],
        ),
        1.0
    );
    assert_eq!(
        metric_sample(
            &exposition,
            "snowblog_render_duration_seconds_count",
            &[("operation", "persisted"), ("result", "success")],
        ),
        1.0
    );
    assert_eq!(
        metric_label_values(&exposition, "snowblog_render_attempts_total", "outcome"),
        string_set(&["discarded"])
    );
    for forbidden in [
        "private_discarded_slug",
        "private_discarded_source",
        "Finished",
    ] {
        assert!(!exposition.contains(forbidden), "leaked {forbidden}");
    }
}

#[tokio::test(flavor = "current_thread")]
async fn rerender_metrics_use_rerender_operation_without_private_values() {
    let service = service().await;
    let post_slug = draft(&service, "private_rerender_slug").await;
    service
        .save_translation(&post_slug, Revision(1), translation("en", "= Initial"))
        .await
        .unwrap();
    service
        .store()
        .upsert_translation(
            &post_slug,
            Revision(2),
            translation("en", "= private_rerender_source"),
        )
        .await
        .unwrap();
    let recorder = test_recorder();
    let handle = recorder.handle();
    let _recorder_guard = set_default_local_recorder(&recorder);

    let reports = service.rerender(RerenderScope::Stale).await.unwrap();
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].outcome, RerenderOutcome::Rerendered);

    let exposition = handle.render();
    assert_eq!(
        metric_sample(
            &exposition,
            "snowblog_render_attempts_total",
            &[("operation", "rerender"), ("outcome", "success")],
        ),
        1.0
    );
    assert_eq!(
        metric_sample(
            &exposition,
            "snowblog_render_duration_seconds_count",
            &[("operation", "rerender"), ("result", "success")],
        ),
        1.0
    );
    assert_eq!(
        metric_label_values(&exposition, "snowblog_render_attempts_total", "operation"),
        string_set(&["rerender"])
    );
    for forbidden in ["private_rerender_slug", "private_rerender_source"] {
        assert!(!exposition.contains(forbidden), "leaked {forbidden}");
    }
}

fn test_recorder() -> metrics_exporter_prometheus::PrometheusRecorder {
    PrometheusBuilder::new()
        .set_buckets(&[1.0])
        .expect("test buckets are non-empty")
        .build_recorder()
}

fn metric_sample(exposition: &str, family: &str, labels: &[(&str, &str)]) -> f64 {
    let line = exposition
        .lines()
        .filter(|line| line.starts_with(family))
        .find(|line| {
            labels
                .iter()
                .all(|(name, value)| line.contains(&format!("{name}=\"{value}\"")))
        })
        .unwrap_or_else(|| panic!("missing {family} sample for {labels:?}"));
    line.rsplit_once(' ')
        .expect("Prometheus sample has a value")
        .1
        .parse()
        .expect("Prometheus sample value is numeric")
}

fn metric_label_values(exposition: &str, family: &str, label: &str) -> BTreeSet<String> {
    let marker = format!("{label}=\"");
    exposition
        .lines()
        .filter(|line| line.starts_with(family))
        .filter_map(|line| {
            let value = line.split_once(&marker)?.1;
            Some(value.split_once('"')?.0.to_owned())
        })
        .collect()
}

fn string_set(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}
