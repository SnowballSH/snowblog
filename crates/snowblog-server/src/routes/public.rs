use axum::Json;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use serde_json::json;
use snowblog_core::domain::{Language, PostStatus, Slug};
use snowblog_core::store::{PostFilter, PostRecord, StoredRender, Translation};

use crate::etag::{asset_etag, if_none_match_hits, post_etag};
use crate::problem::Problem;
use crate::state::AppState;

const MAX_LIMIT: u32 = 100;
const DEFAULT_LIMIT: u32 = 20;

#[derive(Deserialize)]
pub struct ListQuery {
    tag: Option<String>,
    language: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
}

#[derive(Deserialize)]
pub struct DetailQuery {
    language: Option<String>,
}

#[derive(Serialize)]
struct PostSummary {
    id: String,
    slug: String,
    languages: Vec<String>,
    default_language: String,
    tags: Vec<String>,
    published_at: Option<Timestamp>,
    title: String,
    description: String,
}

#[derive(Serialize)]
struct RenderedWith {
    renderer_version: String,
    rendered_at: Timestamp,
}

#[derive(Serialize)]
struct PostDetail {
    #[serde(flatten)]
    summary: PostSummary,
    language: String,
    html: String,
    rendered_with: RenderedWith,
}

pub async fn list_posts(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<serde_json::Value>, Problem> {
    let language = parse_optional_language(query.language.as_deref())?;
    let records = state
        .service
        .store()
        .list_posts(PostFilter {
            status: Some(PostStatus::Published),
            tag: query.tag,
            limit: query.limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT),
            offset: query.offset.unwrap_or(0),
        })
        .await
        .map_err(Problem::from)?;
    let posts: Vec<PostSummary> = records
        .iter()
        .map(|record| summary(record, choose_translation(record, language.as_ref())))
        .collect();
    Ok(Json(json!({ "posts": posts })))
}

pub async fn get_post(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(query): Query<DetailQuery>,
    headers: HeaderMap,
) -> Result<Response, Problem> {
    let record = published_record(&state, &slug).await?;
    let language = parse_optional_language(query.language.as_deref())?;
    let translation = match &language {
        Some(requested) => record.translation(requested).ok_or(Problem::new(
            StatusCode::NOT_FOUND,
            "language_not_available",
            format!("post {slug} has no {requested} translation"),
        ))?,
        None => record
            .translation(&record.post.default_language)
            .ok_or_else(|| Problem::internal("default translation missing"))?,
    };
    let render = record
        .render(&translation.language)
        .ok_or_else(|| Problem::internal("published post lacks a render"))?;

    let etag = post_etag(record.post.revision.0, &render.input_hash);
    if if_none_match_hits(&headers, &etag) {
        return Ok(not_modified(&etag));
    }

    let detail = PostDetail {
        summary: summary(&record, translation),
        language: translation.language.to_string(),
        html: render.html.clone(),
        rendered_with: rendered_with(render),
    };
    let mut response = Json(detail).into_response();
    response
        .headers_mut()
        .insert(header::ETAG, etag.parse().expect("valid etag"));
    Ok(response)
}

pub async fn get_asset(
    State(state): State<AppState>,
    Path((slug, path)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Response, Problem> {
    let record = published_record(&state, &slug).await?;
    let asset = state
        .service
        .store()
        .get_asset(&record.post.slug, &path)
        .await
        .map_err(Problem::from)?
        .ok_or_else(Problem::not_found)?;

    let etag = asset_etag(&asset.content_hash);
    if if_none_match_hits(&headers, &etag) {
        return Ok(not_modified(&etag));
    }
    let mut response = Response::new(Body::from(asset.content));
    let content_type = asset
        .content_type
        .parse()
        .unwrap_or_else(|_| "application/octet-stream".parse().expect("static"));
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, content_type);
    response
        .headers_mut()
        .insert(header::ETAG, etag.parse().expect("valid etag"));
    Ok(response)
}

async fn published_record(state: &AppState, slug: &str) -> Result<PostRecord, Problem> {
    let slug = Slug::parse(slug).map_err(|_| Problem::not_found())?;
    let record = state
        .service
        .store()
        .get_post(&slug)
        .await
        .map_err(Problem::from)?
        .ok_or_else(Problem::not_found)?;
    if record.post.status != PostStatus::Published {
        return Err(Problem::not_found());
    }
    Ok(record)
}

fn parse_optional_language(raw: Option<&str>) -> Result<Option<Language>, Problem> {
    raw.map(|value| {
        Language::parse(value).map_err(|_| {
            Problem::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "invalid_language",
                format!("{value:?} is not a valid BCP-47 language tag"),
            )
        })
    })
    .transpose()
}

fn choose_translation<'a>(record: &'a PostRecord, requested: Option<&Language>) -> &'a Translation {
    requested
        .and_then(|language| record.translation(language))
        .or_else(|| record.translation(&record.post.default_language))
        .or_else(|| record.translations.first())
        .expect("published posts always carry a translation")
}

fn summary(record: &PostRecord, translation: &Translation) -> PostSummary {
    PostSummary {
        id: record.post.id.to_string(),
        slug: record.post.slug.to_string(),
        languages: record
            .translations
            .iter()
            .map(|t| t.language.to_string())
            .collect(),
        default_language: record.post.default_language.to_string(),
        tags: record.post.tags.clone(),
        published_at: record.post.published_at,
        title: translation.title.clone(),
        description: translation.description.clone(),
    }
}

fn rendered_with(render: &StoredRender) -> RenderedWith {
    RenderedWith {
        renderer_version: render.renderer_version.clone(),
        rendered_at: render.rendered_at,
    }
}

fn not_modified(etag: &str) -> Response {
    let mut response = StatusCode::NOT_MODIFIED.into_response();
    response
        .headers_mut()
        .insert(header::ETAG, etag.parse().expect("valid etag"));
    response
}
