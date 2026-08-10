use std::time::Instant;

use sqlx::Row;

use super::{RenderArtifact, Store, StoreError};
use crate::domain::{Language, PostId, Revision};
use crate::telemetry::{StoreOperation, record_store};

impl Store {
    pub async fn replace_render(
        &self,
        id: &PostId,
        language: &Language,
        snapshot: Revision,
        artifact: RenderArtifact,
    ) -> Result<bool, StoreError> {
        let started = Instant::now();
        let result = async {
            let warnings = serde_json::to_string(&artifact.warnings)
                .map_err(|e| StoreError::Constraint(e.to_string()))?;
            let mut tx = self.pool().begin().await?;
            let current = sqlx::query("SELECT revision FROM posts WHERE id = ?")
                .bind(id.to_string())
                .fetch_optional(&mut *tx)
                .await?;
            let Some(row) = current else {
                return Ok(false);
            };
            if Revision(row.get::<i64, _>("revision")) != snapshot {
                return Ok(false);
            }
            sqlx::query(
                "INSERT INTO renders (post_id, language, html, renderer_version, input_hash, warnings, rendered_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?)
                 ON CONFLICT (post_id, language) DO UPDATE
                 SET html = excluded.html, renderer_version = excluded.renderer_version,
                     input_hash = excluded.input_hash, warnings = excluded.warnings,
                     rendered_at = excluded.rendered_at",
            )
            .bind(id.to_string())
            .bind(language.as_str())
            .bind(&artifact.html)
            .bind(&artifact.renderer_version)
            .bind(&artifact.input_hash)
            .bind(&warnings)
            .bind(artifact.rendered_at.to_string())
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            Ok(true)
        }
        .await;
        record_store(StoreOperation::ReplaceRender, &result, started.elapsed());
        result
    }
}
