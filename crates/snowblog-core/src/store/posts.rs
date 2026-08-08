use jiff::Timestamp;

use super::{
    NewPost, PostFilter, PostPatch, PostRecord, Store, StoreError, bump_revision,
    is_unique_violation, lock_post, now_string,
};
use crate::domain::{PostId, PostStatus, Revision, Slug};

impl Store {
    pub async fn create_post(&self, new: NewPost) -> Result<PostRecord, StoreError> {
        let id = PostId::generate().to_string();
        let now = now_string();
        let mut tx = self.pool().begin().await?;
        let inserted = sqlx::query(
            "INSERT INTO posts (id, slug, status, default_language, revision, published_at, created_at, updated_at)
             VALUES (?, ?, 'draft', ?, 1, ?, ?, ?)",
        )
        .bind(&id)
        .bind(new.slug.as_str())
        .bind(new.default_language.as_str())
        .bind(new.published_at.map(|t| t.to_string()))
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await;
        if let Err(error) = inserted {
            return Err(if is_unique_violation(&error) {
                StoreError::SlugTaken(new.slug)
            } else {
                error.into()
            });
        }
        insert_tags(&mut tx, &id, &new.tags).await?;
        tx.commit().await?;
        self.fetch_record_by_id(&id).await
    }

    pub async fn update_post_meta(
        &self,
        slug: &Slug,
        expected: Revision,
        patch: PostPatch,
    ) -> Result<PostRecord, StoreError> {
        let mut tx = self.pool().begin().await?;
        let locked = lock_post(&mut tx, slug, expected).await?;

        if let Some(new_slug) = &patch.slug {
            let result = sqlx::query("UPDATE posts SET slug = ? WHERE id = ?")
                .bind(new_slug.as_str())
                .bind(&locked.id)
                .execute(&mut *tx)
                .await;
            if let Err(error) = result {
                return Err(if is_unique_violation(&error) {
                    StoreError::SlugTaken(new_slug.clone())
                } else {
                    error.into()
                });
            }
        }
        if let Some(language) = &patch.default_language {
            let exists = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM post_translations WHERE post_id = ? AND language = ?",
            )
            .bind(&locked.id)
            .bind(language.as_str())
            .fetch_one(&mut *tx)
            .await?;
            if exists == 0 {
                return Err(StoreError::DefaultLanguageViolation);
            }
            sqlx::query("UPDATE posts SET default_language = ? WHERE id = ?")
                .bind(language.as_str())
                .bind(&locked.id)
                .execute(&mut *tx)
                .await?;
        }
        if let Some(tags) = &patch.tags {
            sqlx::query("DELETE FROM post_tags WHERE post_id = ?")
                .bind(&locked.id)
                .execute(&mut *tx)
                .await?;
            insert_tags(&mut tx, &locked.id, tags).await?;
        }
        if let Some(published_at) = &patch.published_at {
            sqlx::query("UPDATE posts SET published_at = ? WHERE id = ?")
                .bind(published_at.map(|t| t.to_string()))
                .bind(&locked.id)
                .execute(&mut *tx)
                .await?;
        }

        bump_revision(&mut tx, &locked.id).await?;
        tx.commit().await?;
        self.fetch_record_by_id(&locked.id).await
    }

    pub async fn set_status(
        &self,
        slug: &Slug,
        expected: Revision,
        status: PostStatus,
        published_at_if_unset: Option<Timestamp>,
    ) -> Result<PostRecord, StoreError> {
        let mut tx = self.pool().begin().await?;
        let locked = lock_post(&mut tx, slug, expected).await?;
        sqlx::query("UPDATE posts SET status = ? WHERE id = ?")
            .bind(status.as_str())
            .bind(&locked.id)
            .execute(&mut *tx)
            .await?;
        if let Some(default_ts) = published_at_if_unset {
            sqlx::query("UPDATE posts SET published_at = ? WHERE id = ? AND published_at IS NULL")
                .bind(default_ts.to_string())
                .bind(&locked.id)
                .execute(&mut *tx)
                .await?;
        }
        bump_revision(&mut tx, &locked.id).await?;
        tx.commit().await?;
        self.fetch_record_by_id(&locked.id).await
    }

    pub async fn delete_post(&self, slug: &Slug, expected: Revision) -> Result<(), StoreError> {
        let mut tx = self.pool().begin().await?;
        let locked = lock_post(&mut tx, slug, expected).await?;
        sqlx::query("DELETE FROM posts WHERE id = ?")
            .bind(&locked.id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn list(&self, filter: PostFilter) -> Result<Vec<PostRecord>, StoreError> {
        self.list_posts(filter).await
    }
}

async fn insert_tags(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    post_id: &str,
    tags: &[String],
) -> Result<(), StoreError> {
    for tag in tags {
        sqlx::query("INSERT OR IGNORE INTO post_tags (post_id, tag) VALUES (?, ?)")
            .bind(post_id)
            .bind(tag)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}
