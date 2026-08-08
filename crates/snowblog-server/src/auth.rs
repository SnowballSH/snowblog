use std::path::Path;
use std::sync::Arc;

use axum::extract::Request;
use axum::http::header;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use constant_time_eq::constant_time_eq;

use crate::problem::Problem;

#[derive(Clone)]
pub struct AdminToken(Arc<Vec<u8>>);

impl AdminToken {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let raw = std::fs::read_to_string(path)?;
        let token = raw.trim();
        anyhow::ensure!(!token.is_empty(), "admin token file {path:?} is empty");
        Ok(Self(Arc::new(token.as_bytes().to_vec())))
    }

    fn authorizes(&self, headers: &axum::http::HeaderMap) -> bool {
        headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .is_some_and(|presented| constant_time_eq(presented.as_bytes(), &self.0))
    }
}

pub async fn require_admin(token: AdminToken, request: Request, next: Next) -> Response {
    if token.authorizes(request.headers()) {
        next.run(request).await
    } else {
        Problem::unauthorized().into_response()
    }
}
