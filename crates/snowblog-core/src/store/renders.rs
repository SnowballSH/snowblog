use super::{RenderArtifact, Store, StoreError};
use crate::domain::{Language, PostId};

impl Store {
    pub async fn replace_render(
        &self,
        id: &PostId,
        language: &Language,
        artifact: RenderArtifact,
    ) -> Result<(), StoreError> {
        let warnings = serde_json::to_string(&artifact.warnings)
            .map_err(|e| StoreError::Constraint(e.to_string()))?;
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
        .execute(self.pool())
        .await?;
        Ok(())
    }
}
