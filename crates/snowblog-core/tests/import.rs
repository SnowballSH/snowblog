use std::path::{Path, PathBuf};
use std::sync::Arc;

use snowblog_core::domain::{Language, PostStatus, Slug};
use snowblog_core::import::{ImportError, SourceAdaptation, import_dir, parse_frontmatter};
use snowblog_core::render::{RenderLimits, Renderer};
use snowblog_core::service::BlogService;
use snowblog_core::store::Store;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/blogs")
}

async fn service() -> BlogService {
    let store = Store::in_memory().await.unwrap();
    let renderer = Arc::new(Renderer::new(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/packages"),
        Vec::new(),
        RenderLimits {
            timeout: std::time::Duration::from_secs(60),
            ..RenderLimits::default()
        },
    ));
    BlogService::new(store, renderer, Some("/api/v1/posts/{slug}/assets/".into()))
}

#[test]
fn parses_hello_world_frontmatter() {
    let raw = std::fs::read_to_string(fixtures().join("hello_world.typ")).unwrap();
    let (front, body) = parse_frontmatter(&raw).unwrap();
    assert_eq!(front.title, "Hello, Snowblog");
    assert_eq!(
        front.description.as_deref(),
        Some("A demonstration post exercising the importer's frontmatter handling.")
    );
    assert_eq!(front.date.as_deref(), Some("2024-01-15"));
    assert_eq!(front.tags, vec!["demo", "typst", "blog"]);
    assert!(!front.draft);
    assert!(!front.hidden);
    assert!(!body.contains("#metadata"));
    assert!(!body.contains("<frontmatter>"));
    assert!(body.starts_with("#set page") || body.starts_with("#set par"));
}

#[test]
fn commented_out_flags_are_ignored() {
    let raw = std::fs::read_to_string(fixtures().join("graphics_demo.typ")).unwrap();
    let (front, _) = parse_frontmatter(&raw).unwrap();
    assert!(!front.hidden, "commented-out hidden flag must not count");
    assert_eq!(front.tags, vec!["demo"]);
}

#[test]
fn missing_frontmatter_is_an_error() {
    assert!(matches!(
        parse_frontmatter("= Just a document"),
        Err(ImportError::MissingFrontmatter)
    ));
}

#[tokio::test]
async fn import_dir_brings_over_the_fixtures() {
    let service = service().await;
    let report = import_dir(&service, &fixtures(), &SourceAdaptation::default(), false)
        .await
        .unwrap();
    assert_eq!(report.failed, Vec::<(String, String)>::new());
    assert_eq!(report.skipped, Vec::<String>::new());
    assert_eq!(report.imported.len(), 4);
    for post in &report.imported {
        assert_eq!(
            post.render_failures,
            Vec::<String>::new(),
            "{} failed to render",
            post.slug
        );
        assert_eq!(post.status, "published", "{}", post.slug);
    }

    let first = service
        .store()
        .get_post(&Slug::parse("hello_world").unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(first.post.status, PostStatus::Published);
    assert_eq!(first.translations.len(), 2, "zh pairing missing");
    assert_eq!(
        first.post.published_at.unwrap().to_string(),
        "2024-01-15T00:00:00Z"
    );
    let zh = first.translation(&Language::parse("zh").unwrap()).unwrap();
    assert_eq!(zh.title, "你好，雪博客");
    assert!(first.render(&Language::parse("zh").unwrap()).is_some());

    let media = service
        .store()
        .get_post(&Slug::parse("media_demo").unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(media.asset_manifest.len(), 3);
    let html = &media.render(&Language::parse("en").unwrap()).unwrap().html;
    assert!(
        html.contains("/api/v1/posts/media_demo/assets/assets/photo.jpg"),
        "asset url rewrite missing"
    );

    let graphics = service
        .store()
        .get_post(&Slug::parse("graphics_demo").unwrap())
        .await
        .unwrap()
        .unwrap();
    let graphics_html = &graphics
        .render(&Language::parse("en").unwrap())
        .unwrap()
        .html;
    assert!(
        graphics_html.contains("<svg"),
        "cetz canvases must render as svg"
    );
}

#[tokio::test]
async fn second_import_run_is_idempotent() {
    let service = service().await;
    import_dir(&service, &fixtures(), &SourceAdaptation::default(), false)
        .await
        .unwrap();
    let first = service
        .store()
        .get_post(&Slug::parse("hello_world").unwrap())
        .await
        .unwrap()
        .unwrap();

    let second = import_dir(&service, &fixtures(), &SourceAdaptation::default(), false)
        .await
        .unwrap();
    assert_eq!(second.imported.len(), 0);
    assert_eq!(second.skipped.len(), 4);

    let unchanged = service
        .store()
        .get_post(&Slug::parse("hello_world").unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(unchanged.post.revision, first.post.revision);
}

#[tokio::test]
async fn dry_run_writes_nothing() {
    let service = service().await;
    let report = import_dir(&service, &fixtures(), &SourceAdaptation::default(), true)
        .await
        .unwrap();
    assert_eq!(report.imported.len(), 4);
    let posts = service
        .store()
        .list_posts(Default::default())
        .await
        .unwrap();
    assert!(posts.is_empty(), "dry run must not write");
}
