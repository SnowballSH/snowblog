mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use common::{TestApp, app_without_admin, get, send, send_raw};
use snowblog_core::domain::{Language, PostStatus, Revision, Slug};
use snowblog_core::store::{AssetInput, NewPost, TranslationInput};

fn slug(s: &str) -> Slug {
    Slug::parse(s).unwrap()
}

fn lang(s: &str) -> Language {
    Language::parse(s).unwrap()
}

async fn seed(app: &TestApp) {
    let service = app.service().await;
    for (name, tags, days_ago) in [
        ("alpha", vec!["typst".to_string()], 1),
        ("beta", vec!["typst".to_string(), "math".to_string()], 2),
    ] {
        service
            .store()
            .create_post(NewPost {
                slug: slug(name),
                default_language: lang("en"),
                tags,
                published_at: Some(
                    jiff::Timestamp::now() - jiff::SignedDuration::from_hours(24 * days_ago),
                ),
            })
            .await
            .unwrap();
        service
            .save_translation(
                &slug(name),
                Revision(1),
                TranslationInput {
                    language: lang("en"),
                    title: format!("{name} title"),
                    description: format!("{name} description"),
                    source: format!("= {name} body"),
                },
            )
            .await
            .unwrap();
    }
    service
        .save_translation(
            &slug("alpha"),
            Revision(2),
            TranslationInput {
                language: lang("zh"),
                title: "阿尔法".to_string(),
                description: "中文描述".to_string(),
                source: "= 中文正文".to_string(),
            },
        )
        .await
        .unwrap();
    service
        .save_asset(
            &slug("alpha"),
            Revision(3),
            AssetInput {
                path: "assets/pic.png".to_string(),
                content: std::fs::read(
                    common::package_root().join(
                        "../../crates/snowblog-core/tests/fixtures/blogs/assets/mathishard.png",
                    ),
                )
                .unwrap(),
                content_type: "image/png".to_string(),
            },
        )
        .await
        .unwrap();
    service.publish(&slug("alpha"), Revision(4)).await.unwrap();
    service.publish(&slug("beta"), Revision(2)).await.unwrap();

    service
        .store()
        .create_post(NewPost {
            slug: slug("hidden_draft"),
            default_language: lang("en"),
            tags: vec![],
            published_at: None,
        })
        .await
        .unwrap();
    service
        .save_translation(
            &slug("hidden_draft"),
            Revision(1),
            TranslationInput {
                language: lang("en"),
                title: "draft".into(),
                description: String::new(),
                source: "= draft".into(),
            },
        )
        .await
        .unwrap();

    service
        .store()
        .create_post(NewPost {
            slug: slug("archived_post"),
            default_language: lang("en"),
            tags: vec![],
            published_at: None,
        })
        .await
        .unwrap();
    service
        .store()
        .set_status(
            &slug("archived_post"),
            Revision(1),
            PostStatus::Archived,
            None,
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn list_returns_published_only_ordered() {
    let app = app_without_admin().await;
    seed(&app).await;
    let (status, body) = send(&app, get("/api/v1/posts")).await;
    assert_eq!(status, StatusCode::OK);
    let slugs: Vec<&str> = body["posts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["slug"].as_str().unwrap())
        .collect();
    assert_eq!(slugs, vec!["alpha", "beta"]);
    let alpha = &body["posts"][0];
    assert_eq!(alpha["title"], "alpha title");
    assert_eq!(alpha["default_language"], "en");
    assert_eq!(
        alpha["languages"].as_array().unwrap().len(),
        2,
        "alpha should list en and zh"
    );
}

#[tokio::test]
async fn list_filters_by_tag_and_language() {
    let app = app_without_admin().await;
    seed(&app).await;
    let (_, body) = send(&app, get("/api/v1/posts?tag=math")).await;
    let posts = body["posts"].as_array().unwrap();
    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0]["slug"], "beta");

    let (_, body) = send(&app, get("/api/v1/posts?language=zh")).await;
    let posts = body["posts"].as_array().unwrap();
    assert_eq!(posts[0]["title"], "阿尔法", "zh title for alpha");
    assert_eq!(
        posts[1]["title"], "beta title",
        "beta falls back to default language"
    );
}

#[tokio::test]
async fn list_clamps_limit_and_paginates() {
    let app = app_without_admin().await;
    seed(&app).await;
    let (_, body) = send(&app, get("/api/v1/posts?limit=1&offset=1")).await;
    let posts = body["posts"].as_array().unwrap();
    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0]["slug"], "beta");

    let (status, _) = send(&app, get("/api/v1/posts?limit=1000")).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn detail_serves_html_with_language_fallback() {
    let app = app_without_admin().await;
    seed(&app).await;
    let (status, body) = send(&app, get("/api/v1/posts/alpha")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["language"], "en");
    assert!(body["html"].as_str().unwrap().contains("alpha body"));
    assert_eq!(body["rendered_with"]["renderer_version"], "0.15.1");

    let (_, body) = send(&app, get("/api/v1/posts/alpha?language=zh")).await;
    assert_eq!(body["language"], "zh");
    assert!(body["html"].as_str().unwrap().contains("中文正文"));
}

#[tokio::test]
async fn detail_hides_unpublished_posts() {
    let app = app_without_admin().await;
    seed(&app).await;
    for name in ["hidden_draft", "archived_post", "never_existed"] {
        let (status, body) = send(&app, get(&format!("/api/v1/posts/{name}"))).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{name} leaked");
        assert_eq!(body["code"], "not_found", "{name} distinguishable");
    }
}

#[tokio::test]
async fn detail_unavailable_language_404s() {
    let app = app_without_admin().await;
    seed(&app).await;
    let (status, body) = send(&app, get("/api/v1/posts/beta?language=zh")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "language_not_available");

    let (status, body) = send(&app, get("/api/v1/posts/beta?language=not%20a%20tag")).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], "invalid_language");
}

#[tokio::test]
async fn detail_etag_round_trip() {
    let app = app_without_admin().await;
    seed(&app).await;
    let first = send_raw(&app, get("/api/v1/posts/alpha")).await;
    assert_eq!(first.status(), StatusCode::OK);
    let etag = first.headers()[header::ETAG].to_str().unwrap().to_string();

    let request = Request::builder()
        .uri("/api/v1/posts/alpha")
        .header(header::IF_NONE_MATCH, &etag)
        .body(Body::empty())
        .unwrap();
    let cached = send_raw(&app, request).await;
    assert_eq!(cached.status(), StatusCode::NOT_MODIFIED);

    let service = app.service().await;
    service
        .save_translation(
            &slug("alpha"),
            Revision(5),
            TranslationInput {
                language: lang("en"),
                title: "alpha title".into(),
                description: "alpha description".into(),
                source: "= alpha body edited".into(),
            },
        )
        .await
        .unwrap();
    let request = Request::builder()
        .uri("/api/v1/posts/alpha")
        .header(header::IF_NONE_MATCH, &etag)
        .body(Body::empty())
        .unwrap();
    let fresh = send_raw(&app, request).await;
    assert_eq!(fresh.status(), StatusCode::OK, "etag must change on edit");
    assert_ne!(fresh.headers()[header::ETAG].to_str().unwrap(), etag);
}

#[tokio::test]
async fn asset_served_with_content_type_and_etag() {
    let app = app_without_admin().await;
    seed(&app).await;
    let response = send_raw(&app, get("/api/v1/posts/alpha/assets/assets/pic.png")).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CONTENT_TYPE], "image/png");
    let etag = response.headers()[header::ETAG]
        .to_str()
        .unwrap()
        .to_string();

    let request = Request::builder()
        .uri("/api/v1/posts/alpha/assets/assets/pic.png")
        .header(header::IF_NONE_MATCH, &etag)
        .body(Body::empty())
        .unwrap();
    let cached = send_raw(&app, request).await;
    assert_eq!(cached.status(), StatusCode::NOT_MODIFIED);
}

#[tokio::test]
async fn asset_traversal_and_missing_404() {
    let app = app_without_admin().await;
    seed(&app).await;
    for path in [
        "/api/v1/posts/alpha/assets/assets/../../secret",
        "/api/v1/posts/alpha/assets/nope.png",
        "/api/v1/posts/hidden_draft/assets/assets/pic.png",
    ] {
        let response = send_raw(&app, get(path)).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path} leaked");
    }
}
