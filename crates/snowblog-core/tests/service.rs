use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use metrics::set_default_local_recorder;
use metrics_exporter_prometheus::PrometheusBuilder;
use snowblog_core::domain::{Language, PostStatus, Revision, Slug};
use snowblog_core::render::{RenderLimits, RenderOutcome, Renderer};
use snowblog_core::service::{
    BlogService, Freshness, RenderStatus, RerenderOutcome, RerenderScope, ServiceError,
};
use snowblog_core::store::{AssetInput, NewPost, Store, StoreError, TranslationInput};

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
    assert_metric_samples(
        &exposition,
        "snowblog_render_attempts_total",
        &[
            (
                labels(&[("operation", "preview"), ("outcome", "success")]),
                1.0,
            ),
            (
                labels(&[("operation", "preview"), ("outcome", "failure")]),
                1.0,
            ),
        ],
    );
    assert_metric_samples(
        &exposition,
        "snowblog_render_duration_seconds_count",
        &[
            (
                labels(&[("operation", "preview"), ("result", "success")]),
                1.0,
            ),
            (
                labels(&[("operation", "preview"), ("result", "failure")]),
                1.0,
            ),
        ],
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
    assert_metric_samples(
        &exposition,
        "snowblog_render_attempts_total",
        &[
            (
                labels(&[("operation", "persisted"), ("outcome", "success")]),
                1.0,
            ),
            (
                labels(&[("operation", "persisted"), ("outcome", "failure")]),
                1.0,
            ),
        ],
    );
    assert_metric_samples(
        &exposition,
        "snowblog_render_duration_seconds_count",
        &[
            (
                labels(&[("operation", "persisted"), ("result", "success")]),
                1.0,
            ),
            (
                labels(&[("operation", "persisted"), ("result", "failure")]),
                1.0,
            ),
        ],
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
async fn successful_render_with_store_error_keeps_duration_without_attempt_outcome() {
    let service = service().await;
    let post_slug = draft(&service, "private_store_error_slug").await;
    sqlx::query(
        "CREATE TRIGGER private_render_failure
         BEFORE INSERT ON renders
         BEGIN
             SELECT RAISE(ABORT, 'private render persistence error');
         END",
    )
    .execute(service.store().pool())
    .await
    .unwrap();
    let recorder = test_recorder();
    let handle = recorder.handle();
    let _recorder_guard = set_default_local_recorder(&recorder);

    let result = service
        .save_translation(
            &post_slug,
            Revision(1),
            translation("en", "= private_store_error_source"),
        )
        .await;
    match result {
        Err(ServiceError::Store(StoreError::Db(error))) => assert!(
            error
                .to_string()
                .contains("private render persistence error")
        ),
        other => panic!("expected render persistence database error, got {other:?}"),
    }
    let record = service.store().get_post(&post_slug).await.unwrap().unwrap();
    assert_eq!(record.post.revision, Revision(2));
    assert!(record.translation(&lang("en")).is_some());
    assert!(record.render(&lang("en")).is_none());

    let exposition = handle.render();
    assert_metric_samples(&exposition, "snowblog_render_attempts_total", &[]);
    assert_metric_samples(
        &exposition,
        "snowblog_render_duration_seconds_count",
        &[(
            labels(&[("operation", "persisted"), ("result", "success")]),
            1.0,
        )],
    );
    for forbidden in [
        "private_store_error_slug",
        "private_store_error_source",
        "private render persistence error",
        "private_render_failure",
    ] {
        assert!(!exposition.contains(forbidden), "leaked {forbidden}");
    }
}

#[tokio::test(flavor = "current_thread")]
async fn stale_rerender_skips_fresh_posts_without_emitting_extra_metrics() {
    let service = service().await;
    let fresh_slug = draft(&service, "private_fresh_rerender_slug").await;
    service
        .save_translation(
            &fresh_slug,
            Revision(1),
            translation("en", "= private_fresh_rerender_source"),
        )
        .await
        .unwrap();
    let stale_slug = draft(&service, "private_stale_rerender_slug").await;
    service
        .save_translation(&stale_slug, Revision(1), translation("en", "= Initial"))
        .await
        .unwrap();
    service
        .store()
        .upsert_translation(
            &stale_slug,
            Revision(2),
            translation("en", "= private_stale_rerender_source"),
        )
        .await
        .unwrap();
    let recorder = test_recorder();
    let handle = recorder.handle();
    let _recorder_guard = set_default_local_recorder(&recorder);

    let reports = service.rerender(RerenderScope::Stale).await.unwrap();
    assert_eq!(reports.len(), 2);
    let outcome_for = |post_slug: &Slug| {
        reports
            .iter()
            .find(|report| &report.slug == post_slug)
            .unwrap()
            .outcome
            .clone()
    };
    assert_eq!(outcome_for(&fresh_slug), RerenderOutcome::SkippedFresh);
    assert_eq!(outcome_for(&stale_slug), RerenderOutcome::Rerendered);

    let exposition = handle.render();
    assert_metric_samples(
        &exposition,
        "snowblog_render_attempts_total",
        &[(
            labels(&[("operation", "rerender"), ("outcome", "success")]),
            1.0,
        )],
    );
    assert_metric_samples(
        &exposition,
        "snowblog_render_duration_seconds_count",
        &[(
            labels(&[("operation", "rerender"), ("result", "success")]),
            1.0,
        )],
    );
    for forbidden in [
        "private_fresh_rerender_slug",
        "private_fresh_rerender_source",
        "private_stale_rerender_slug",
        "private_stale_rerender_source",
    ] {
        assert!(!exposition.contains(forbidden), "leaked {forbidden}");
    }
}

#[tokio::test(flavor = "current_thread")]
async fn asset_save_and_delete_use_persisted_render_operation() {
    let service = service().await;
    let post_slug = draft(&service, "private_asset_routing_slug").await;
    service
        .save_translation(
            &post_slug,
            Revision(1),
            translation("en", "= Asset routing"),
        )
        .await
        .unwrap();
    let recorder = test_recorder();
    let handle = recorder.handle();
    let _recorder_guard = set_default_local_recorder(&recorder);
    let asset_path = "assets/private_persisted_routing.png";

    service
        .save_asset(
            &post_slug,
            Revision(2),
            AssetInput {
                path: asset_path.into(),
                content: png(),
                content_type: "image/png".into(),
            },
        )
        .await
        .unwrap();
    assert_metric_samples(
        &handle.render(),
        "snowblog_render_attempts_total",
        &[(
            labels(&[("operation", "persisted"), ("outcome", "success")]),
            1.0,
        )],
    );

    service
        .delete_asset(&post_slug, Revision(3), asset_path)
        .await
        .unwrap();
    let exposition = handle.render();
    assert_metric_samples(
        &exposition,
        "snowblog_render_attempts_total",
        &[(
            labels(&[("operation", "persisted"), ("outcome", "success")]),
            2.0,
        )],
    );
    assert_metric_samples(
        &exposition,
        "snowblog_render_duration_seconds_count",
        &[(
            labels(&[("operation", "persisted"), ("result", "success")]),
            2.0,
        )],
    );
    for forbidden in [
        "private_asset_routing_slug",
        "private_persisted_routing.png",
    ] {
        assert!(!exposition.contains(forbidden), "leaked {forbidden}");
    }
}

fn test_recorder() -> metrics_exporter_prometheus::PrometheusRecorder {
    PrometheusBuilder::new()
        .set_buckets(&[1.0])
        .expect("test buckets are non-empty")
        .build_recorder()
}

type Labels = BTreeSet<(String, String)>;

fn assert_metric_samples(exposition: &str, family: &str, expected: &[(Labels, f64)]) {
    let actual = exposition
        .lines()
        .filter_map(|line| {
            let labeled_sample = line.strip_prefix(&format!("{family}{{"))?;
            let (serialized_labels, value) = labeled_sample.split_once("} ")?;
            let labels = serialized_labels
                .split(',')
                .map(|label| {
                    let (name, quoted_value) = label.split_once('=')?;
                    let value = quoted_value.strip_prefix('"')?.strip_suffix('"')?;
                    Some((name.to_owned(), value.to_owned()))
                })
                .collect::<Option<Labels>>()?;
            Some((labels, value.parse::<f64>().ok()?))
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
