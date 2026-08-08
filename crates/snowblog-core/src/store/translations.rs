use super::{
    PostRecord, Store, StoreError, TranslationInput, bump_revision, lock_post, now_string,
};
use crate::domain::{Language, Revision, Slug};

impl Store {
    pub async fn upsert_translation(
        &self,
        slug: &Slug,
        expected: Revision,
        translation: TranslationInput,
    ) -> Result<PostRecord, StoreError> {
        let mut tx = self.pool().begin().await?;
        let locked = lock_post(&mut tx, slug, expected).await?;
        sqlx::query(
            "INSERT INTO post_translations (post_id, language, title, description, source, updated_at)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT (post_id, language) DO UPDATE
             SET title = excluded.title, description = excluded.description,
                 source = excluded.source, updated_at = excluded.updated_at",
        )
        .bind(&locked.id)
        .bind(translation.language.as_str())
        .bind(&translation.title)
        .bind(&translation.description)
        .bind(&translation.source)
        .bind(now_string())
        .execute(&mut *tx)
        .await?;
        bump_revision(&mut tx, &locked.id).await?;
        tx.commit().await?;
        self.fetch_record_by_id(&locked.id).await
    }

    pub async fn delete_translation(
        &self,
        slug: &Slug,
        expected: Revision,
        language: &Language,
    ) -> Result<PostRecord, StoreError> {
        let mut tx = self.pool().begin().await?;
        let locked = lock_post(&mut tx, slug, expected).await?;
        if &locked.default_language == language {
            return Err(StoreError::DefaultLanguageViolation);
        }
        let result =
            sqlx::query("DELETE FROM post_translations WHERE post_id = ? AND language = ?")
                .bind(&locked.id)
                .bind(language.as_str())
                .execute(&mut *tx)
                .await?;
        if result.rows_affected() == 0 {
            return Err(StoreError::NotFound);
        }
        bump_revision(&mut tx, &locked.id).await?;
        tx.commit().await?;
        self.fetch_record_by_id(&locked.id).await
    }
}
