use std::path::{Path, PathBuf};
use std::sync::Arc;

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
