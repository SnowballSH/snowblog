mod common;

use axum::http::StatusCode;
use common::{TestApp, admin_json, app_with_admin, get, send, with_if_match};
use serde_json::json;

async fn create_with_translation(app: &TestApp, slug: &str, source: &str) {
    let (status, _) = send(
        app,
        admin_json(
            "POST",
            "/api/v1/admin/posts",
            json!({"slug": slug, "default_language": "en"}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let save = with_if_match(
        admin_json(
            "PUT",
            &format!("/api/v1/admin/posts/{slug}/translations/en"),
            json!({"title": "T", "description": "", "source": source}),
        ),
        1,
    );
    let (status, _) = send(app, save).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn preview_renders_without_persisting() {
    let app = app_with_admin().await;
    create_with_translation(&app, "previewable", "= Saved").await;

    let (status, body) = send(
        &app,
        admin_json(
            "POST",
            "/api/v1/admin/posts/previewable/preview",
            json!({"source": "= Previewed instead"}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
    assert!(body["html"].as_str().unwrap().contains("Previewed instead"));

    let (_, admin_view) = send(
        &app,
        admin_json("GET", "/api/v1/admin/posts/previewable", json!(null)),
    )
    .await;
    assert_eq!(admin_view["revision"], 2, "preview must not bump revision");
    assert_eq!(admin_view["translations"][0]["source"], "= Saved");
}

#[tokio::test]
async fn preview_reports_diagnostics_without_500() {
    let app = app_with_admin().await;
    create_with_translation(&app, "diag_preview", "= Fine").await;
    let (status, body) = send(
        &app,
        admin_json(
            "POST",
            "/api/v1/admin/posts/diag_preview/preview",
            json!({"source": "#broken("}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "failed");
    assert!(!body["diagnostics"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn publish_flow_and_public_visibility() {
    let app = app_with_admin().await;
    create_with_translation(&app, "lifecycle", "= Alive").await;

    let (status, _) = send(&app, get("/api/v1/posts/lifecycle")).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "draft must be hidden");

    let publish = with_if_match(
        admin_json("POST", "/api/v1/admin/posts/lifecycle/publish", json!(null)),
        2,
    );
    let (status, body) = send(&app, publish).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["status"], "published");
    assert!(body["published_at"].is_string());

    let (status, body) = send(&app, get("/api/v1/posts/lifecycle")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["html"].as_str().unwrap().contains("Alive"));

    let unpublish = with_if_match(
        admin_json(
            "POST",
            "/api/v1/admin/posts/lifecycle/unpublish",
            json!(null),
        ),
        3,
    );
    let (status, _) = send(&app, unpublish).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = send(&app, get("/api/v1/posts/lifecycle")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let archive = with_if_match(
        admin_json("POST", "/api/v1/admin/posts/lifecycle/archive", json!(null)),
        4,
    );
    let (status, body) = send(&app, archive).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "archived");
}

#[tokio::test]
async fn publish_blocked_on_failed_render() {
    let app = app_with_admin().await;
    create_with_translation(&app, "blocked", "= Good").await;
    let broken = with_if_match(
        admin_json(
            "PUT",
            "/api/v1/admin/posts/blocked/translations/zh",
            json!({"title": "Z", "description": "", "source": "#undefined_fn()"}),
        ),
        2,
    );
    send(&app, broken).await;

    let publish = with_if_match(
        admin_json("POST", "/api/v1/admin/posts/blocked/publish", json!(null)),
        3,
    );
    let (status, body) = send(&app, publish).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "publish_blocked");
    let translations = body["translations"].as_array().unwrap();
    assert!(
        translations
            .iter()
            .any(|t| t["language"] == "zh" && t["freshness"] == "missing"),
        "blocked language not reported: {body}"
    );
}

#[tokio::test]
async fn publish_requires_if_match() {
    let app = app_with_admin().await;
    create_with_translation(&app, "needs_precondition", "= X").await;
    let (status, body) = send(
        &app,
        admin_json(
            "POST",
            "/api/v1/admin/posts/needs_precondition/publish",
            json!(null),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::PRECONDITION_REQUIRED);
    assert_eq!(body["code"], "precondition_required");
}

#[tokio::test]
async fn rerender_stale_updates_published_html() {
    let app = app_with_admin().await;
    create_with_translation(&app, "rerenderable", "= Original").await;
    let publish = with_if_match(
        admin_json(
            "POST",
            "/api/v1/admin/posts/rerenderable/publish",
            json!(null),
        ),
        2,
    );
    send(&app, publish).await;

    let service = app.service().await;
    service
        .store()
        .upsert_translation(
            &snowblog_core::domain::Slug::parse("rerenderable").unwrap(),
            snowblog_core::domain::Revision(3),
            snowblog_core::store::TranslationInput {
                language: snowblog_core::domain::Language::parse("en").unwrap(),
                title: "T".into(),
                description: String::new(),
                source: "= Rewritten".into(),
            },
        )
        .await
        .unwrap();

    let (status, body) = send(
        &app,
        admin_json("POST", "/api/v1/admin/rerender", json!({"scope": "stale"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let reports = body["reports"].as_array().unwrap();
    assert!(
        reports
            .iter()
            .any(|r| r["slug"] == "rerenderable" && r["outcome"] == "rerendered"),
        "{body}"
    );

    let (_, body) = send(&app, get("/api/v1/posts/rerenderable")).await;
    assert!(body["html"].as_str().unwrap().contains("Rewritten"));
}
