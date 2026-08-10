use std::sync::Arc;
use std::time::Instant;

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::{Diagnostic, Language, PostStatus, Revision, Slug};
use crate::render::{RenderAsset, RenderInput, RenderOptions, RenderOutcome, Renderer, input_hash};
use crate::store::{
    AssetInput, PostFilter, PostRecord, RenderArtifact, Store, StoreError, TranslationInput,
};
use crate::telemetry::{RenderOperation, RenderOutcome as MetricRenderOutcome, record_render};

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum RenderStatus {
    Ok { warnings: Vec<Diagnostic> },
    Failed { diagnostics: Vec<Diagnostic> },
}

#[derive(Clone, Debug, Serialize)]
pub struct TranslationRender {
    pub language: Language,
    #[serde(flatten)]
    pub render: RenderStatus,
}

#[derive(Clone, Debug)]
pub struct SaveOutcome {
    pub record: PostRecord,
    pub renders: Vec<TranslationRender>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Freshness {
    Fresh,
    Stale,
    Missing,
}

#[derive(Clone, Debug, Serialize)]
pub struct TranslationFreshness {
    pub language: Language,
    pub freshness: Freshness,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RerenderScope {
    Stale,
    All,
}

#[derive(Clone, Debug, Serialize)]
pub struct RerenderReport {
    pub slug: Slug,
    pub language: Language,
    pub outcome: RerenderOutcome,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RerenderOutcome {
    Rerendered,
    SkippedFresh,
    Failed,
}

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("publish blocked: renders are stale, missing, or failed")]
    PublishBlocked(Vec<TranslationFreshness>),
}

#[derive(Clone)]
pub struct BlogService {
    store: Store,
    renderer: Arc<Renderer>,
    asset_url_template: Option<String>,
}

impl BlogService {
    pub fn new(store: Store, renderer: Arc<Renderer>, asset_url_template: Option<String>) -> Self {
        Self {
            store,
            renderer,
            asset_url_template,
        }
    }

    pub fn store(&self) -> &Store {
        &self.store
    }

    pub fn renderer(&self) -> &Renderer {
        &self.renderer
    }

    pub fn freshness(&self, record: &PostRecord) -> Vec<TranslationFreshness> {
        let manifest = manifest_pairs(record);
        let prefix = self.asset_url_prefix(record);
        record
            .translations
            .iter()
            .map(|translation| {
                let freshness = match record.render(&translation.language) {
                    None => Freshness::Missing,
                    Some(render) => {
                        let expected_hash =
                            input_hash(&translation.source, &manifest, prefix.as_deref());
                        if render.input_hash == expected_hash
                            && render.renderer_version == self.renderer.version()
                        {
                            Freshness::Fresh
                        } else {
                            Freshness::Stale
                        }
                    }
                };
                TranslationFreshness {
                    language: translation.language.clone(),
                    freshness,
                }
            })
            .collect()
    }

    pub async fn save_translation(
        &self,
        slug: &Slug,
        expected: Revision,
        translation: TranslationInput,
    ) -> Result<SaveOutcome, ServiceError> {
        let language = translation.language.clone();
        let record = self
            .store
            .upsert_translation(slug, expected, translation)
            .await?;
        let renders = vec![
            self.render_and_store(&record, &language, RenderOperation::Persisted)
                .await?,
        ];
        let record = self.reload(&record).await?;
        Ok(SaveOutcome { record, renders })
    }

    pub async fn save_asset(
        &self,
        slug: &Slug,
        expected: Revision,
        asset: AssetInput,
    ) -> Result<SaveOutcome, ServiceError> {
        let record = self.store.upsert_asset(slug, expected, asset).await?;
        self.render_all(record).await
    }

    pub async fn delete_asset(
        &self,
        slug: &Slug,
        expected: Revision,
        path: &str,
    ) -> Result<SaveOutcome, ServiceError> {
        let record = self.store.delete_asset(slug, expected, path).await?;
        self.render_all(record).await
    }

    pub async fn preview(
        &self,
        slug: &Slug,
        source: String,
    ) -> Result<RenderOutcome, ServiceError> {
        let record = self
            .store
            .get_post(slug)
            .await?
            .ok_or(StoreError::NotFound)?;
        let render_input = self.render_input(&record, source).await?;
        let started = Instant::now();
        let outcome = self.renderer.render(render_input).await;
        let duration = started.elapsed();
        let metric_outcome = match &outcome {
            RenderOutcome::Success { .. } => MetricRenderOutcome::Success,
            RenderOutcome::Failure { .. } => MetricRenderOutcome::Failure,
        };
        record_render(RenderOperation::Preview, metric_outcome, duration);
        Ok(outcome)
    }

    pub async fn publish(
        &self,
        slug: &Slug,
        expected: Revision,
    ) -> Result<PostRecord, ServiceError> {
        let record = self
            .store
            .get_post(slug)
            .await?
            .ok_or(StoreError::NotFound)?;
        let mut freshness = self.freshness(&record);
        if record.translation(&record.post.default_language).is_none() {
            freshness.push(TranslationFreshness {
                language: record.post.default_language.clone(),
                freshness: Freshness::Missing,
            });
        }
        if record.translations.is_empty()
            || freshness.iter().any(|f| f.freshness != Freshness::Fresh)
        {
            return Err(ServiceError::PublishBlocked(freshness));
        }
        Ok(self
            .store
            .set_status(
                slug,
                expected,
                PostStatus::Published,
                Some(Timestamp::now()),
            )
            .await?)
    }

    pub async fn set_status(
        &self,
        slug: &Slug,
        expected: Revision,
        status: PostStatus,
    ) -> Result<PostRecord, ServiceError> {
        Ok(self.store.set_status(slug, expected, status, None).await?)
    }

    pub async fn rerender(
        &self,
        scope: RerenderScope,
    ) -> Result<Vec<RerenderReport>, ServiceError> {
        let records = self
            .store
            .list_posts(PostFilter {
                limit: u32::MAX,
                ..Default::default()
            })
            .await?;
        let mut reports = Vec::new();
        for record in records {
            let freshness = self.freshness(&record);
            for entry in freshness {
                let skip = scope == RerenderScope::Stale && entry.freshness == Freshness::Fresh;
                let outcome = if skip {
                    RerenderOutcome::SkippedFresh
                } else {
                    match self
                        .render_and_store(&record, &entry.language, RenderOperation::Rerender)
                        .await?
                        .render
                    {
                        RenderStatus::Ok { .. } => RerenderOutcome::Rerendered,
                        RenderStatus::Failed { .. } => RerenderOutcome::Failed,
                    }
                };
                reports.push(RerenderReport {
                    slug: record.post.slug.clone(),
                    language: entry.language,
                    outcome,
                });
            }
        }
        Ok(reports)
    }

    async fn render_all(&self, record: PostRecord) -> Result<SaveOutcome, ServiceError> {
        let languages: Vec<Language> = record
            .translations
            .iter()
            .map(|t| t.language.clone())
            .collect();
        let mut renders = Vec::with_capacity(languages.len());
        for language in languages {
            renders.push(
                self.render_and_store(&record, &language, RenderOperation::Persisted)
                    .await?,
            );
        }
        let record = self.reload(&record).await?;
        Ok(SaveOutcome { record, renders })
    }

    async fn render_and_store(
        &self,
        record: &PostRecord,
        language: &Language,
        operation: RenderOperation,
    ) -> Result<TranslationRender, ServiceError> {
        let translation = record
            .translation(language)
            .ok_or(StoreError::NotFound)?
            .clone();
        let render_input = self
            .render_input(record, translation.source.clone())
            .await?;
        let started = Instant::now();
        let outcome = self.renderer.render(render_input).await;
        let duration = started.elapsed();
        let render = match outcome {
            RenderOutcome::Success { html, warnings } => {
                let artifact = RenderArtifact {
                    html,
                    renderer_version: self.renderer.version().to_string(),
                    input_hash: input_hash(
                        &translation.source,
                        &manifest_pairs(record),
                        self.asset_url_prefix(record).as_deref(),
                    ),
                    warnings: warnings.clone(),
                    rendered_at: Timestamp::now(),
                };
                let stored = self
                    .store
                    .replace_render(&record.post.id, language, record.post.revision, artifact)
                    .await?;
                if !stored {
                    tracing::info!(
                        slug = %record.post.slug,
                        %language,
                        "render discarded: the post changed while compiling"
                    );
                }
                record_render(
                    operation,
                    if stored {
                        MetricRenderOutcome::Success
                    } else {
                        MetricRenderOutcome::Discarded
                    },
                    duration,
                );
                RenderStatus::Ok { warnings }
            }
            RenderOutcome::Failure { diagnostics } => {
                record_render(operation, MetricRenderOutcome::Failure, duration);
                RenderStatus::Failed { diagnostics }
            }
        };
        Ok(TranslationRender {
            language: language.clone(),
            render,
        })
    }

    fn asset_url_prefix(&self, record: &PostRecord) -> Option<String> {
        self.asset_url_template
            .as_ref()
            .map(|template| template.replace("{slug}", record.post.slug.as_str()))
    }

    async fn render_input(
        &self,
        record: &PostRecord,
        source: String,
    ) -> Result<RenderInput, ServiceError> {
        let assets = self
            .store
            .get_assets(&record.post.slug)
            .await?
            .into_iter()
            .map(|asset| RenderAsset {
                path: asset.path,
                content: asset.content.into(),
            })
            .collect();
        let asset_url_prefix = self.asset_url_prefix(record);
        Ok(RenderInput {
            source,
            assets,
            options: RenderOptions { asset_url_prefix },
        })
    }

    async fn reload(&self, record: &PostRecord) -> Result<PostRecord, ServiceError> {
        Ok(self
            .store
            .get_post(&record.post.slug)
            .await?
            .ok_or(StoreError::NotFound)?)
    }
}

fn manifest_pairs(record: &PostRecord) -> Vec<(String, String)> {
    record
        .asset_manifest
        .iter()
        .map(|asset| (asset.path.clone(), asset.content_hash.clone()))
        .collect()
}
