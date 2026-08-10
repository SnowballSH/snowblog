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
use crate::telemetry::{
    RenderOperation, RenderOutcome as MetricRenderOutcome, record_render_attempt,
    record_render_duration,
};

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
    #[cfg(test)]
    post_render_checkpoint: Option<PostRenderCheckpoint>,
}

#[cfg(test)]
#[derive(Clone)]
struct PostRenderCheckpoint {
    reached: Arc<tokio::sync::Barrier>,
    resume: Arc<tokio::sync::Barrier>,
}

impl BlogService {
    pub fn new(store: Store, renderer: Arc<Renderer>, asset_url_template: Option<String>) -> Self {
        Self {
            store,
            renderer,
            asset_url_template,
            #[cfg(test)]
            post_render_checkpoint: None,
        }
    }

    #[cfg(test)]
    fn with_post_render_checkpoint(mut self, checkpoint: PostRenderCheckpoint) -> Self {
        self.post_render_checkpoint = Some(checkpoint);
        self
    }

    #[cfg(test)]
    async fn wait_at_post_render_checkpoint(&self) {
        if let Some(checkpoint) = &self.post_render_checkpoint {
            checkpoint.reached.wait().await;
            checkpoint.resume.wait().await;
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
        record_render_duration(RenderOperation::Preview, metric_outcome, duration);
        record_render_attempt(RenderOperation::Preview, metric_outcome);
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
        record_render_duration(
            operation,
            match &outcome {
                RenderOutcome::Success { .. } => MetricRenderOutcome::Success,
                RenderOutcome::Failure { .. } => MetricRenderOutcome::Failure,
            },
            duration,
        );
        let render = match outcome {
            RenderOutcome::Success { html, warnings } => {
                #[cfg(test)]
                self.wait_at_post_render_checkpoint().await;
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
                record_render_attempt(
                    operation,
                    if stored {
                        MetricRenderOutcome::Success
                    } else {
                        MetricRenderOutcome::Discarded
                    },
                );
                RenderStatus::Ok { warnings }
            }
            RenderOutcome::Failure { diagnostics } => {
                record_render_attempt(operation, MetricRenderOutcome::Failure);
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

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::Path;

    use metrics::set_default_local_recorder;
    use metrics_exporter_prometheus::PrometheusBuilder;
    use tokio::sync::Barrier;

    use super::*;
    use crate::store::{NewPost, PostPatch};

    #[tokio::test(flavor = "current_thread")]
    async fn revision_change_after_render_discards_artifact_deterministically() {
        let reached = Arc::new(Barrier::new(2));
        let resume = Arc::new(Barrier::new(2));
        let store = Store::in_memory().await.unwrap();
        let renderer = Arc::new(Renderer::new(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/packages"),
            Vec::new(),
            Default::default(),
        ));
        let service = BlogService::new(store, renderer, None).with_post_render_checkpoint(
            PostRenderCheckpoint {
                reached: Arc::clone(&reached),
                resume: Arc::clone(&resume),
            },
        );
        let post_slug = Slug::parse("private_synchronized_discard").unwrap();
        service
            .store()
            .create_post(NewPost {
                slug: post_slug.clone(),
                default_language: Language::parse("en").unwrap(),
                tags: Vec::new(),
                published_at: None,
            })
            .await
            .unwrap();
        let recorder = PrometheusBuilder::new()
            .set_buckets(&[1.0])
            .unwrap()
            .build_recorder();
        let handle = recorder.handle();
        let _recorder_guard = set_default_local_recorder(&recorder);

        let render_task = tokio::spawn({
            let service = service.clone();
            let post_slug = post_slug.clone();
            async move {
                service
                    .save_translation(
                        &post_slug,
                        Revision(1),
                        TranslationInput {
                            language: Language::parse("en").unwrap(),
                            title: "Private synchronized discard".into(),
                            description: String::new(),
                            source: "= private_synchronized_discard_source".into(),
                        },
                    )
                    .await
            }
        });

        reached.wait().await;
        service
            .store()
            .update_post_meta(&post_slug, Revision(2), PostPatch::default())
            .await
            .unwrap();
        resume.wait().await;
        let outcome = render_task.await.unwrap().unwrap();
        assert!(matches!(outcome.renders[0].render, RenderStatus::Ok { .. }));
        assert_eq!(outcome.record.post.revision, Revision(3));
        assert!(
            outcome
                .record
                .render(&Language::parse("en").unwrap())
                .is_none()
        );

        let exposition = handle.render();
        assert_eq!(
            family_samples(&exposition, "snowblog_render_attempts_total"),
            BTreeMap::from([(
                labels(&[("operation", "persisted"), ("outcome", "discarded")]),
                1.0,
            )])
        );
        assert_eq!(
            family_samples(&exposition, "snowblog_render_duration_seconds_count"),
            BTreeMap::from([(
                labels(&[("operation", "persisted"), ("result", "success")]),
                1.0,
            )])
        );
        for forbidden in [
            "private_synchronized_discard",
            "private_synchronized_discard_source",
            "Private synchronized discard",
        ] {
            assert!(!exposition.contains(forbidden), "leaked {forbidden}");
        }
    }

    type Labels = BTreeSet<(String, String)>;

    fn family_samples(exposition: &str, family: &str) -> BTreeMap<Labels, f64> {
        exposition
            .lines()
            .filter_map(|line| {
                let labeled_sample = line.strip_prefix(&format!("{family}{{"))?;
                let (serialized_labels, value) = labeled_sample.split_once("} ")?;
                let labels = serialized_labels
                    .split(',')
                    .map(|label| {
                        let (name, quoted_value) = label.split_once('=')?;
                        let value = quoted_value.strip_prefix('"')?.strip_suffix('"')?;
                        Some((name.to_owned(), value.to_owned()))
                    })
                    .collect::<Option<Labels>>()?;
                Some((labels, value.parse().ok()?))
            })
            .collect()
    }

    fn labels(values: &[(&str, &str)]) -> Labels {
        values
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect()
    }
}
