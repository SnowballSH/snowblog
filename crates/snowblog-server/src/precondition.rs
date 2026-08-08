use axum::extract::FromRequestParts;
use axum::http::{StatusCode, header, request::Parts};
use snowblog_core::domain::Revision;

use crate::problem::Problem;

pub struct IfMatch(pub Revision);

impl<S: Send + Sync> FromRequestParts<S> for IfMatch {
    type Rejection = Problem;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let value = parts
            .headers
            .get(header::IF_MATCH)
            .ok_or_else(|| {
                Problem::new(
                    StatusCode::PRECONDITION_REQUIRED,
                    "precondition_required",
                    "mutations require an If-Match header carrying the post revision",
                )
            })?
            .to_str()
            .map_err(invalid_if_match)?;
        let revision: i64 = value
            .trim()
            .trim_matches('"')
            .parse()
            .map_err(invalid_if_match)?;
        Ok(Self(Revision(revision)))
    }
}

fn invalid_if_match(_: impl std::fmt::Debug) -> Problem {
    Problem::new(
        StatusCode::BAD_REQUEST,
        "invalid_if_match",
        "If-Match must carry the post revision, e.g. If-Match: \"3\"",
    )
}
