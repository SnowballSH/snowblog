use axum::Json;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use serde_json::json;
use snowblog_core::domain::{Language, PostStatus, Slug};
use snowblog_core::render::RenderOutcome;
use snowblog_core::service::{RerenderScope, SaveOutcome, TranslationFreshness, TranslationRender};
use snowblog_core::store::{
    AssetInput, AssetRef, NewPost, PostFilter, PostPatch, PostRecord, TranslationInput,
};

use crate::precondition::IfMatch;
use crate::problem::Problem;
use crate::state::AppState;

#[derive(Serialize)]
pub struct AdminPost {
    id: String,
    slug: String,
    status: String,
    default_language: String,
    revision: i64,
    tags: Vec<String>,
    published_at: Option<Timestamp>,
    created_at: Timestamp,
    updated_at: Timestamp,
    translations: Vec<AdminTranslation>,
    renders: Vec<AdminRender>,
    assets: Vec<AssetRef>,
    freshness: Vec<TranslationFreshness>,
}

#[derive(Serialize)]
struct AdminTranslation {
    language: String,
    title: String,
    description: String,
    source: String,
    updated_at: Timestamp,
}

#[derive(Serialize)]
struct AdminRender {
    language: String,
    renderer_version: String,
    input_hash: String,
    warnings: Vec<snowblog_core::domain::Diagnostic>,
    rendered_at: Timestamp,
}

fn admin_post(state: &AppState, record: &PostRecord) -> AdminPost {
    AdminPost {
        id: record.post.id.to_string(),
        slug: record.post.slug.to_string(),
        status: record.post.status.to_string(),
        default_language: record.post.default_language.to_string(),
        revision: record.post.revision.0,
        tags: record.post.tags.clone(),
        published_at: record.post.published_at,
        created_at: record.post.created_at,
        updated_at: record.post.updated_at,
        translations: record
            .translations
            .iter()
            .map(|t| AdminTranslation {
                language: t.language.to_string(),
                title: t.title.clone(),
                description: t.description.clone(),
                source: t.source.clone(),
                updated_at: t.updated_at,
            })
            .collect(),
        renders: record
            .renders
            .iter()
            .map(|r| AdminRender {
                language: r.language.to_string(),
                renderer_version: r.renderer_version.clone(),
                input_hash: r.input_hash.clone(),
                warnings: r.warnings.clone(),
                rendered_at: r.rendered_at,
            })
            .collect(),
        assets: record.asset_manifest.clone(),
        freshness: state.service.freshness(record),
    }
}

fn parse_slug(raw: &str) -> Result<Slug, Problem> {
    Slug::parse(raw).map_err(|e| {
        Problem::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_slug",
            e.to_string(),
        )
    })
}

fn parse_language(raw: &str) -> Result<Language, Problem> {
    Language::parse(raw).map_err(|e| {
        Problem::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_language",
            e.to_string(),
        )
    })
}

async fn record_or_404(state: &AppState, slug: &Slug) -> Result<PostRecord, Problem> {
    state
        .service
        .store()
        .get_post(slug)
        .await
        .map_err(Problem::from)?
        .ok_or_else(Problem::not_found)
}

pub async fn list_posts(State(state): State<AppState>) -> Result<Json<serde_json::Value>, Problem> {
    let records = state
        .service
        .store()
        .list_posts(PostFilter {
            limit: u32::MAX,
            ..Default::default()
        })
        .await
        .map_err(Problem::from)?;
    let posts: Vec<AdminPost> = records.iter().map(|r| admin_post(&state, r)).collect();
    Ok(Json(json!({ "posts": posts })))
}

pub async fn get_post(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<AdminPost>, Problem> {
    let slug = parse_slug(&slug)?;
    let record = record_or_404(&state, &slug).await?;
    Ok(Json(admin_post(&state, &record)))
}

#[derive(Deserialize)]
pub struct CreateBody {
    slug: String,
    default_language: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    published_at: Option<Timestamp>,
}

pub async fn create_post(
    State(state): State<AppState>,
    Json(body): Json<CreateBody>,
) -> Result<Response, Problem> {
    let new_post = NewPost {
        slug: parse_slug(&body.slug)?,
        default_language: parse_language(&body.default_language)?,
        tags: body.tags,
        published_at: body.published_at,
    };
    let record = state
        .service
        .store()
        .create_post(new_post)
        .await
        .map_err(Problem::from)?;
    Ok((StatusCode::CREATED, Json(admin_post(&state, &record))).into_response())
}

#[derive(Deserialize)]
pub struct PatchBody {
    #[serde(default)]
    slug: Option<String>,
    #[serde(default)]
    default_language: Option<String>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default, deserialize_with = "double_option")]
    published_at: Option<Option<Timestamp>>,
}

fn double_option<'de, D>(deserializer: D) -> Result<Option<Option<Timestamp>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<Timestamp>::deserialize(deserializer).map(Some)
}

pub async fn patch_post(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    IfMatch(revision): IfMatch,
    Json(body): Json<PatchBody>,
) -> Result<Json<AdminPost>, Problem> {
    let slug = parse_slug(&slug)?;
    let patch = PostPatch {
        slug: body.slug.as_deref().map(parse_slug).transpose()?,
        default_language: body
            .default_language
            .as_deref()
            .map(parse_language)
            .transpose()?,
        tags: body.tags,
        published_at: body.published_at,
    };
    let record = state
        .service
        .store()
        .update_post_meta(&slug, revision, patch)
        .await
        .map_err(Problem::from)?;
    Ok(Json(admin_post(&state, &record)))
}

pub async fn delete_post(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    IfMatch(revision): IfMatch,
) -> Result<StatusCode, Problem> {
    let slug = parse_slug(&slug)?;
    state
        .service
        .store()
        .delete_post(&slug, revision)
        .await
        .map_err(Problem::from)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct TranslationBody {
    title: String,
    #[serde(default)]
    description: String,
    source: String,
}

pub async fn put_translation(
    State(state): State<AppState>,
    Path((slug, language)): Path<(String, String)>,
    IfMatch(revision): IfMatch,
    Json(body): Json<TranslationBody>,
) -> Result<Json<serde_json::Value>, Problem> {
    let slug = parse_slug(&slug)?;
    let language = parse_language(&language)?;
    let outcome = state
        .service
        .save_translation(
            &slug,
            revision,
            TranslationInput {
                language,
                title: body.title,
                description: body.description,
                source: body.source,
            },
        )
        .await
        .map_err(Problem::from)?;
    Ok(Json(save_response(&state, &outcome, true)))
}

pub async fn delete_translation(
    State(state): State<AppState>,
    Path((slug, language)): Path<(String, String)>,
    IfMatch(revision): IfMatch,
) -> Result<Json<AdminPost>, Problem> {
    let slug = parse_slug(&slug)?;
    let language = parse_language(&language)?;
    let record = state
        .service
        .store()
        .delete_translation(&slug, revision, &language)
        .await
        .map_err(Problem::from)?;
    Ok(Json(admin_post(&state, &record)))
}

pub async fn put_asset(
    State(state): State<AppState>,
    Path((slug, path)): Path<(String, String)>,
    IfMatch(revision): IfMatch,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<serde_json::Value>, Problem> {
    let slug = parse_slug(&slug)?;
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();
    let outcome = state
        .service
        .save_asset(
            &slug,
            revision,
            AssetInput {
                path,
                content: body.to_vec(),
                content_type,
            },
        )
        .await
        .map_err(Problem::from)?;
    Ok(Json(save_response(&state, &outcome, false)))
}

pub async fn delete_asset(
    State(state): State<AppState>,
    Path((slug, path)): Path<(String, String)>,
    IfMatch(revision): IfMatch,
) -> Result<Json<serde_json::Value>, Problem> {
    let slug = parse_slug(&slug)?;
    let outcome = state
        .service
        .delete_asset(&slug, revision, &path)
        .await
        .map_err(Problem::from)?;
    Ok(Json(save_response(&state, &outcome, false)))
}

#[derive(Deserialize)]
pub struct PreviewBody {
    #[serde(default)]
    #[allow(dead_code)]
    language: Option<String>,
    source: String,
}

pub async fn preview(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Json(body): Json<PreviewBody>,
) -> Result<Json<serde_json::Value>, Problem> {
    let slug = parse_slug(&slug)?;
    let outcome = state
        .service
        .preview(&slug, body.source)
        .await
        .map_err(Problem::from)?;
    Ok(Json(match outcome {
        RenderOutcome::Success { html, warnings } => {
            json!({"status": "ok", "html": html, "warnings": warnings})
        }
        RenderOutcome::Failure { diagnostics } => {
            json!({"status": "failed", "diagnostics": diagnostics})
        }
    }))
}

pub async fn publish(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    IfMatch(revision): IfMatch,
) -> Result<Json<AdminPost>, Problem> {
    let slug = parse_slug(&slug)?;
    let record = state
        .service
        .publish(&slug, revision)
        .await
        .map_err(Problem::from)?;
    Ok(Json(admin_post(&state, &record)))
}

pub async fn unpublish(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    IfMatch(revision): IfMatch,
) -> Result<Json<AdminPost>, Problem> {
    set_status(state, slug, revision, PostStatus::Draft).await
}

pub async fn archive(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    IfMatch(revision): IfMatch,
) -> Result<Json<AdminPost>, Problem> {
    set_status(state, slug, revision, PostStatus::Archived).await
}

async fn set_status(
    state: AppState,
    slug: String,
    revision: snowblog_core::domain::Revision,
    status: PostStatus,
) -> Result<Json<AdminPost>, Problem> {
    let slug = parse_slug(&slug)?;
    let record = state
        .service
        .set_status(&slug, revision, status)
        .await
        .map_err(Problem::from)?;
    Ok(Json(admin_post(&state, &record)))
}

#[derive(Deserialize)]
pub struct RerenderBody {
    scope: RerenderScope,
}

pub async fn rerender(
    State(state): State<AppState>,
    Json(body): Json<RerenderBody>,
) -> Result<Json<serde_json::Value>, Problem> {
    let reports = state
        .service
        .rerender(body.scope)
        .await
        .map_err(Problem::from)?;
    Ok(Json(json!({ "reports": reports })))
}

fn save_response(state: &AppState, outcome: &SaveOutcome, single: bool) -> serde_json::Value {
    let post = admin_post(state, &outcome.record);
    if single {
        json!({"post": post, "render": render_json(&outcome.renders[0])})
    } else {
        json!({
            "post": post,
            "renders": outcome.renders.iter().map(render_json).collect::<Vec<_>>(),
        })
    }
}

fn render_json(render: &TranslationRender) -> serde_json::Value {
    serde_json::to_value(render).unwrap_or(serde_json::Value::Null)
}
