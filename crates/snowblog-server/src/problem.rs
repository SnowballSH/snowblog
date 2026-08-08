use axum::Json;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use http_body_util::BodyExt;
use serde_json::{Map, Value, json};
use snowblog_core::service::ServiceError;
use snowblog_core::store::StoreError;

#[derive(Debug)]
pub struct Problem {
    pub status: StatusCode,
    pub code: &'static str,
    pub detail: String,
    pub extensions: Map<String, Value>,
}

impl Problem {
    pub fn new(status: StatusCode, code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            status,
            code,
            detail: detail.into(),
            extensions: Map::new(),
        }
    }

    pub fn with(mut self, key: &str, value: Value) -> Self {
        self.extensions.insert(key.to_string(), value);
        self
    }

    pub fn not_found() -> Self {
        Self::new(StatusCode::NOT_FOUND, "not_found", "resource not found")
    }

    pub fn unauthorized() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "missing or invalid bearer token",
        )
    }

    pub fn internal(detail: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal", detail)
    }
}

impl IntoResponse for Problem {
    fn into_response(self) -> Response {
        let mut body = json!({
            "type": "about:blank",
            "title": self.status.canonical_reason().unwrap_or("Error"),
            "status": self.status.as_u16(),
            "code": self.code,
            "detail": self.detail,
        });
        if let Value::Object(object) = &mut body {
            object.extend(self.extensions);
        }
        let mut response = (self.status, Json(body)).into_response();
        response.headers_mut().insert(
            axum::http::header::CONTENT_TYPE,
            axum::http::HeaderValue::from_static("application/problem+json"),
        );
        response
    }
}

impl From<StoreError> for Problem {
    fn from(error: StoreError) -> Self {
        match error {
            StoreError::RevisionMismatch { expected, actual } => Self::new(
                StatusCode::PRECONDITION_FAILED,
                "revision_mismatch",
                format!("expected revision {expected}, actual {actual}"),
            )
            .with("current_revision", json!(actual)),
            StoreError::SlugTaken(slug) => Self::new(
                StatusCode::CONFLICT,
                "slug_taken",
                format!("slug {slug} is already taken"),
            ),
            StoreError::NotFound => Self::not_found(),
            StoreError::DefaultLanguageViolation => Self::new(
                StatusCode::CONFLICT,
                "default_language",
                "the default language must keep a translation",
            ),
            StoreError::Constraint(detail) => Self::internal(detail),
            StoreError::Db(error) => Self::internal(error.to_string()),
        }
    }
}

impl From<ServiceError> for Problem {
    fn from(error: ServiceError) -> Self {
        match error {
            ServiceError::Store(store_error) => store_error.into(),
            ServiceError::PublishBlocked(freshness) => Self::new(
                StatusCode::CONFLICT,
                "publish_blocked",
                "publish requires fresh successful renders for every translation",
            )
            .with(
                "translations",
                serde_json::to_value(freshness).unwrap_or(Value::Null),
            ),
        }
    }
}

pub async fn normalize_error_responses(request: Request, next: Next) -> Response {
    let response = next.run(request).await;
    let status = response.status();
    if !(status.is_client_error() || status.is_server_error()) {
        return response;
    }
    let is_problem = response
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("application/problem+json"));
    if is_problem {
        return response;
    }
    let (_, body) = response.into_parts();
    let detail = match body.collect().await {
        Ok(collected) => String::from_utf8_lossy(&collected.to_bytes())
            .trim()
            .to_string(),
        Err(_) => String::new(),
    };
    Problem::new(status, code_for_status(status), detail).into_response()
}

fn code_for_status(status: StatusCode) -> &'static str {
    match status {
        StatusCode::BAD_REQUEST => "bad_request",
        StatusCode::NOT_FOUND => "not_found",
        StatusCode::METHOD_NOT_ALLOWED => "method_not_allowed",
        StatusCode::PAYLOAD_TOO_LARGE => "payload_too_large",
        StatusCode::UNSUPPORTED_MEDIA_TYPE => "unsupported_media_type",
        StatusCode::UNPROCESSABLE_ENTITY => "invalid_body",
        _ => "error",
    }
}
