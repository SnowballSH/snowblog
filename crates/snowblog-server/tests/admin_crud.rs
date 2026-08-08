mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use common::{TEST_TOKEN, TestApp, admin_json, app_with_admin, get, send, send_raw, with_if_match};
use serde_json::json;

async fn create_post(app: &TestApp, slug: &str) {
    let (status, body) = send(
        app,
        admin_json(
            "POST",
            "/api/v1/admin/posts",
            json!({"slug": slug, "default_language": "en", "tags": ["t1"]}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create failed: {body}");
    assert_eq!(body["revision"], 1);
}

fn put_translation(uri: &str, source: &str) -> Request<Body> {
    admin_json(
        "PUT",
        uri,
        json!({"title": "T", "description": "D", "source": source}),
    )
}

#[tokio::test]
async fn create_conflicts_and_validation() {
    let app = app_with_admin().await;
    create_post(&app, "made_up").await;

    let (status, body) = send(
        &app,
        admin_json(
            "POST",
            "/api/v1/admin/posts",
            json!({"slug": "made_up", "default_language": "en"}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "slug_taken");

    let (status, body) = send(
        &app,
        admin_json(
            "POST",
            "/api/v1/admin/posts",
            json!({"slug": "Bad Slug!", "default_language": "en"}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], "invalid_slug");

    let (status, body) = send(
        &app,
        admin_json(
            "POST",
            "/api/v1/admin/posts",
            json!({"slug": "fine", "default_language": "not a tag"}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], "invalid_language");
}

#[tokio::test]
async fn mutations_require_if_match() {
    let app = app_with_admin().await;
    create_post(&app, "concurrent").await;

    let no_precondition = admin_json(
        "PATCH",
        "/api/v1/admin/posts/concurrent",
        json!({"tags": ["x"]}),
    );
    let (status, body) = send(&app, no_precondition).await;
    assert_eq!(status, StatusCode::PRECONDITION_REQUIRED);
    assert_eq!(body["code"], "precondition_required");

    let stale = with_if_match(
        admin_json(
            "PATCH",
            "/api/v1/admin/posts/concurrent",
            json!({"tags": ["x"]}),
        ),
        99,
    );
    let (status, body) = send(&app, stale).await;
    assert_eq!(status, StatusCode::PRECONDITION_FAILED);
    assert_eq!(body["code"], "revision_mismatch");
    assert_eq!(body["current_revision"], 1);

    let fresh = with_if_match(
        admin_json(
            "PATCH",
            "/api/v1/admin/posts/concurrent",
            json!({"tags": ["x"]}),
        ),
        1,
    );
    let (status, body) = send(&app, fresh).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["revision"], 2);
    assert_eq!(body["tags"], json!(["x"]));
}

#[tokio::test]
async fn translation_save_returns_render_feedback() {
    let app = app_with_admin().await;
    create_post(&app, "feedback").await;

    let ok = with_if_match(
        put_translation("/api/v1/admin/posts/feedback/translations/en", "= Fine"),
        1,
    );
    let (status, body) = send(&app, ok).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["render"]["status"], "ok");

    let broken = with_if_match(
        put_translation(
            "/api/v1/admin/posts/feedback/translations/en",
            "#undefined_fn()",
        ),
        2,
    );
    let (status, body) = send(&app, broken).await;
    assert_eq!(status, StatusCode::OK, "broken source still saves: {body}");
    assert_eq!(body["render"]["status"], "failed");
    assert!(!body["render"]["diagnostics"].as_array().unwrap().is_empty());

    let (_, admin_view) = send(
        &app,
        admin_json("GET", "/api/v1/admin/posts/feedback", json!(null)),
    )
    .await;
    assert_eq!(
        admin_view["translations"][0]["source"], "#undefined_fn()",
        "broken source must persist"
    );
    assert_eq!(admin_view["freshness"][0]["freshness"], "stale");
}

#[tokio::test]
async fn delete_default_translation_refused() {
    let app = app_with_admin().await;
    create_post(&app, "protected").await;
    let save = with_if_match(
        put_translation("/api/v1/admin/posts/protected/translations/en", "= A"),
        1,
    );
    send(&app, save).await;

    let delete = with_if_match(
        admin_json(
            "DELETE",
            "/api/v1/admin/posts/protected/translations/en",
            json!(null),
        ),
        2,
    );
    let (status, body) = send(&app, delete).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "default_language");
}

#[tokio::test]
async fn slug_rename_moves_admin_resource() {
    let app = app_with_admin().await;
    create_post(&app, "old_slug").await;
    let rename = with_if_match(
        admin_json(
            "PATCH",
            "/api/v1/admin/posts/old_slug",
            json!({"slug": "new_slug"}),
        ),
        1,
    );
    let (status, _) = send(&app, rename).await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = send(
        &app,
        admin_json("GET", "/api/v1/admin/posts/old_slug", json!(null)),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = send(
        &app,
        admin_json("GET", "/api/v1/admin/posts/new_slug", json!(null)),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn asset_upload_serves_publicly_after_publish() {
    let app = app_with_admin().await;
    create_post(&app, "with_asset").await;
    let save = with_if_match(
        put_translation(
            "/api/v1/admin/posts/with_asset/translations/en",
            "#image(\"assets/pic.png\")",
        ),
        1,
    );
    send(&app, save).await;

    let png = std::fs::read(
        common::package_root()
            .join("../../crates/snowblog-core/tests/fixtures/blogs/assets/doleetcodedaily.png"),
    )
    .unwrap();
    let upload = with_if_match(
        Request::builder()
            .method("PUT")
            .uri("/api/v1/admin/posts/with_asset/assets/assets/pic.png")
            .header(header::AUTHORIZATION, format!("Bearer {TEST_TOKEN}"))
            .header(header::CONTENT_TYPE, "image/png")
            .body(Body::from(png))
            .unwrap(),
        2,
    );
    let (status, body) = send(&app, upload).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["renders"][0]["status"], "ok");

    let publish = with_if_match(
        admin_json(
            "POST",
            "/api/v1/admin/posts/with_asset/publish",
            json!(null),
        ),
        3,
    );
    let (status, body) = send(&app, publish).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let response = send_raw(&app, get("/api/v1/posts/with_asset/assets/assets/pic.png")).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CONTENT_TYPE], "image/png");
}

#[tokio::test]
async fn oversized_asset_rejected_with_413() {
    let app = app_with_admin().await;
    create_post(&app, "too_big").await;
    let upload = with_if_match(
        Request::builder()
            .method("PUT")
            .uri("/api/v1/admin/posts/too_big/assets/assets/big.bin")
            .header(header::AUTHORIZATION, format!("Bearer {TEST_TOKEN}"))
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .body(Body::from(vec![0u8; 128 * 1024]))
            .unwrap(),
        1,
    );
    let response = send_raw(&app, upload).await;
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn delete_post_removes_everything() {
    let app = app_with_admin().await;
    create_post(&app, "doomed").await;
    let delete = with_if_match(
        admin_json("DELETE", "/api/v1/admin/posts/doomed", json!(null)),
        1,
    );
    let (status, _) = send(&app, delete).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _) = send(
        &app,
        admin_json("GET", "/api/v1/admin/posts/doomed", json!(null)),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn admin_list_shows_all_statuses() {
    let app = app_with_admin().await;
    create_post(&app, "draft_one").await;
    create_post(&app, "published_one").await;
    let save = with_if_match(
        put_translation("/api/v1/admin/posts/published_one/translations/en", "= P"),
        1,
    );
    send(&app, save).await;
    let publish = with_if_match(
        admin_json(
            "POST",
            "/api/v1/admin/posts/published_one/publish",
            json!(null),
        ),
        2,
    );
    send(&app, publish).await;

    let (status, body) = send(&app, admin_json("GET", "/api/v1/admin/posts", json!(null))).await;
    assert_eq!(status, StatusCode::OK);
    let posts = body["posts"].as_array().unwrap();
    assert_eq!(posts.len(), 2);
}
