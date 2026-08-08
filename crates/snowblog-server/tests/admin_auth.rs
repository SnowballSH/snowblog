mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use common::{TEST_TOKEN, app_with_admin, app_without_admin, send};
use serde_json::json;

fn admin_get(token: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder().uri("/api/v1/admin/posts");
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    builder.body(Body::empty()).unwrap()
}

#[tokio::test]
async fn missing_token_rejected() {
    let app = app_with_admin().await;
    let (status, body) = send(&app, admin_get(None)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], "unauthorized");
}

#[tokio::test]
async fn wrong_token_rejected() {
    let app = app_with_admin().await;
    let (status, body) = send(&app, admin_get(Some("wrong-token"))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], "unauthorized");
}

#[tokio::test]
async fn correct_token_accepted() {
    let app = app_with_admin().await;
    let (status, body) = send(&app, admin_get(Some(TEST_TOKEN))).await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
}

#[tokio::test]
async fn trailing_newline_in_token_file_is_trimmed() {
    let app = app_with_admin().await;
    let (status, _) = send(&app, admin_get(Some(TEST_TOKEN))).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn admin_disabled_when_no_token_file() {
    let app = app_without_admin().await;
    let (status, body) = send(&app, admin_get(Some(TEST_TOKEN))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "not_found");

    let (status, _) = send(
        &app,
        Request::builder()
            .method("POST")
            .uri("/api/v1/admin/posts")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({"slug": "x", "default_language": "en"}).to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn unmatched_admin_paths_still_require_auth() {
    let app = app_with_admin().await;
    let request = Request::builder()
        .uri("/api/v1/admin/definitely_not_a_route")
        .body(Body::empty())
        .unwrap();
    let (status, body) = send(&app, request).await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "admin prefix must authenticate before revealing route existence: {body}"
    );
}
