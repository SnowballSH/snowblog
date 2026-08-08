use axum::http::{HeaderMap, header};

pub fn post_etag(revision: i64, input_hash: &str) -> String {
    let hash_prefix: String = input_hash.chars().take(16).collect();
    format!("\"{revision}-{hash_prefix}\"")
}

pub fn asset_etag(content_hash: &str) -> String {
    format!("\"{content_hash}\"")
}

pub fn if_none_match_hits(headers: &HeaderMap, etag: &str) -> bool {
    headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value == "*"
                || value
                    .split(',')
                    .map(str::trim)
                    .any(|candidate| candidate == etag)
        })
}
