use std::time::Instant;

use sqlx::Row;

use super::{
    Asset, AssetInput, PostRecord, Store, StoreError, bump_revision, lock_post, now_string,
    parse_timestamp,
};
use crate::domain::{Revision, Slug};
use crate::telemetry::{StoreOperation, record_store};

impl Store {
    pub async fn upsert_asset(
        &self,
        slug: &Slug,
        expected: Revision,
        asset: AssetInput,
    ) -> Result<PostRecord, StoreError> {
        let started = Instant::now();
        let result = async {
            let content_hash = blake3::hash(&asset.content).to_hex().to_string();
            let mut tx = self.pool().begin().await?;
            let locked = lock_post(&mut tx, slug, expected).await?;
            sqlx::query(
                "INSERT INTO assets (post_id, path, content, content_type, content_hash, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?)
                 ON CONFLICT (post_id, path) DO UPDATE
                 SET content = excluded.content, content_type = excluded.content_type,
                     content_hash = excluded.content_hash, updated_at = excluded.updated_at",
            )
            .bind(&locked.id)
            .bind(&asset.path)
            .bind(&asset.content)
            .bind(&asset.content_type)
            .bind(&content_hash)
            .bind(now_string())
            .execute(&mut *tx)
            .await?;
            bump_revision(&mut tx, &locked.id).await?;
            tx.commit().await?;
            self.fetch_record_by_id(&locked.id).await
        }
        .await;
        record_store(StoreOperation::SaveAsset, &result, started.elapsed());
        result
    }

    pub async fn delete_asset(
        &self,
        slug: &Slug,
        expected: Revision,
        path: &str,
    ) -> Result<PostRecord, StoreError> {
        let started = Instant::now();
        let result = async {
            let mut tx = self.pool().begin().await?;
            let locked = lock_post(&mut tx, slug, expected).await?;
            let deleted = sqlx::query("DELETE FROM assets WHERE post_id = ? AND path = ?")
                .bind(&locked.id)
                .bind(path)
                .execute(&mut *tx)
                .await?;
            if deleted.rows_affected() == 0 {
                return Err(StoreError::NotFound);
            }
            bump_revision(&mut tx, &locked.id).await?;
            tx.commit().await?;
            self.fetch_record_by_id(&locked.id).await
        }
        .await;
        record_store(StoreOperation::DeleteAsset, &result, started.elapsed());
        result
    }

    pub async fn get_asset(&self, slug: &Slug, path: &str) -> Result<Option<Asset>, StoreError> {
        let started = Instant::now();
        let result = async {
            let row = sqlx::query(
                "SELECT a.path, a.content, a.content_type, a.content_hash, a.updated_at
                 FROM assets a JOIN posts p ON p.id = a.post_id
                 WHERE p.slug = ? AND a.path = ?",
            )
            .bind(slug.as_str())
            .bind(path)
            .fetch_optional(self.pool())
            .await?;
            row.map(|r| {
                Ok(Asset {
                    path: r.get("path"),
                    content: r.get("content"),
                    content_type: r.get("content_type"),
                    content_hash: r.get("content_hash"),
                    updated_at: parse_timestamp(r.get("updated_at"))?,
                })
            })
            .transpose()
        }
        .await;
        record_store(StoreOperation::GetAsset, &result, started.elapsed());
        result
    }

    pub async fn get_assets(&self, slug: &Slug) -> Result<Vec<Asset>, StoreError> {
        let started = Instant::now();
        let result = async {
            let rows = sqlx::query(
                "SELECT a.path, a.content, a.content_type, a.content_hash, a.updated_at
                 FROM assets a JOIN posts p ON p.id = a.post_id
                 WHERE p.slug = ? ORDER BY a.path",
            )
            .bind(slug.as_str())
            .fetch_all(self.pool())
            .await?;
            rows.iter()
                .map(|r| {
                    Ok(Asset {
                        path: r.get("path"),
                        content: r.get("content"),
                        content_type: r.get("content_type"),
                        content_hash: r.get("content_hash"),
                        updated_at: parse_timestamp(r.get("updated_at"))?,
                    })
                })
                .collect()
        }
        .await;
        record_store(StoreOperation::GetAssets, &result, started.elapsed());
        result
    }
}
