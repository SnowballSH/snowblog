mod assets;
mod error;
mod posts;
mod renders;
mod translations;
mod types;

pub use error::StoreError;
pub use types::{
    Asset, AssetInput, AssetRef, NewPost, Post, PostFilter, PostPatch, PostRecord, RenderArtifact,
    StoredRender, Translation, TranslationInput,
};

use std::path::Path;
use std::str::FromStr;
use std::time::Instant;

use jiff::Timestamp;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};

use crate::domain::{Diagnostic, Language, PostId, PostStatus, Revision, Slug};
use crate::telemetry::{StoreOperation, record_store};

#[derive(Clone)]
pub struct Store {
    pool: SqlitePool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ContentCounts {
    pub draft: u64,
    pub published: u64,
    pub archived: u64,
}

impl Store {
    pub async fn open(path: &Path) -> Result<Self, StoreError> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .foreign_keys(true);
        Self::connect(options, SqlitePoolOptions::new()).await
    }

    pub async fn in_memory() -> Result<Self, StoreError> {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")
            .expect("static options")
            .foreign_keys(true);
        let pool_options = SqlitePoolOptions::new()
            .max_connections(1)
            .idle_timeout(None)
            .max_lifetime(None);
        Self::connect(options, pool_options).await
    }

    async fn connect(
        options: SqliteConnectOptions,
        pool_options: SqlitePoolOptions,
    ) -> Result<Self, StoreError> {
        let pool = pool_options.connect_with(options).await?;
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| StoreError::Constraint(e.to_string()))?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn content_counts(&self) -> Result<ContentCounts, StoreError> {
        let row = sqlx::query(
            "SELECT
                 COALESCE(SUM(CASE WHEN status = 'draft' THEN 1 ELSE 0 END), 0) AS draft,
                 COALESCE(SUM(CASE WHEN status = 'published' THEN 1 ELSE 0 END), 0) AS published,
                 COALESCE(SUM(CASE WHEN status = 'archived' THEN 1 ELSE 0 END), 0) AS archived
             FROM posts",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(ContentCounts {
            draft: row.get::<i64, _>("draft") as u64,
            published: row.get::<i64, _>("published") as u64,
            archived: row.get::<i64, _>("archived") as u64,
        })
    }

    pub async fn schema_version(&self) -> Result<i64, StoreError> {
        Ok(sqlx::query_scalar(
            "SELECT COALESCE(MAX(version), 0) FROM _sqlx_migrations WHERE success = TRUE",
        )
        .fetch_one(&self.pool)
        .await?)
    }

    pub async fn get_post(&self, slug: &Slug) -> Result<Option<PostRecord>, StoreError> {
        let started = Instant::now();
        let result = async {
            let row = sqlx::query("SELECT * FROM posts WHERE slug = ?")
                .bind(slug.as_str())
                .fetch_optional(&self.pool)
                .await?;
            match row {
                Some(row) => Ok(Some(self.load_record(&row).await?)),
                None => Ok(None),
            }
        }
        .await;
        record_store(StoreOperation::GetPost, &result, started.elapsed());
        result
    }

    pub async fn list_posts(&self, filter: PostFilter) -> Result<Vec<PostRecord>, StoreError> {
        let started = Instant::now();
        let result = async {
            let rows = sqlx::query(
                "SELECT DISTINCT p.* FROM posts p
                 LEFT JOIN post_tags t ON t.post_id = p.id
                 WHERE (?1 IS NULL OR p.status = ?1)
                   AND (?2 IS NULL OR t.tag = ?2)
                 ORDER BY p.published_at IS NULL, p.published_at DESC, p.created_at DESC
                 LIMIT ?3 OFFSET ?4",
            )
            .bind(filter.status.map(PostStatus::as_str))
            .bind(filter.tag.as_deref())
            .bind(filter.limit)
            .bind(filter.offset)
            .fetch_all(&self.pool)
            .await?;
            let mut records = Vec::with_capacity(rows.len());
            for row in &rows {
                records.push(self.load_record(row).await?);
            }
            Ok(records)
        }
        .await;
        record_store(StoreOperation::ListPosts, &result, started.elapsed());
        result
    }

    async fn load_record(&self, row: &sqlx::sqlite::SqliteRow) -> Result<PostRecord, StoreError> {
        let id: String = row.get("id");
        let tags = sqlx::query_scalar::<_, String>(
            "SELECT tag FROM post_tags WHERE post_id = ? ORDER BY tag",
        )
        .bind(&id)
        .fetch_all(&self.pool)
        .await?;
        let post = row_to_post(row, tags)?;

        let translation_rows = sqlx::query(
            "SELECT language, title, description, source, updated_at
             FROM post_translations WHERE post_id = ? ORDER BY language",
        )
        .bind(&id)
        .fetch_all(&self.pool)
        .await?;
        let translations = translation_rows
            .iter()
            .map(|r| {
                Ok(Translation {
                    language: parse_language(r.get("language"))?,
                    title: r.get("title"),
                    description: r.get("description"),
                    source: r.get("source"),
                    updated_at: parse_timestamp(r.get("updated_at"))?,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;

        let render_rows = sqlx::query(
            "SELECT language, html, renderer_version, input_hash, warnings, rendered_at
             FROM renders WHERE post_id = ? ORDER BY language",
        )
        .bind(&id)
        .fetch_all(&self.pool)
        .await?;
        let renders = render_rows
            .iter()
            .map(|r| {
                let warnings: Vec<Diagnostic> =
                    serde_json::from_str(r.get::<String, _>("warnings").as_str())
                        .map_err(|e| StoreError::Constraint(e.to_string()))?;
                Ok(StoredRender {
                    language: parse_language(r.get("language"))?,
                    html: r.get("html"),
                    renderer_version: r.get("renderer_version"),
                    input_hash: r.get("input_hash"),
                    warnings,
                    rendered_at: parse_timestamp(r.get("rendered_at"))?,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;

        let asset_rows = sqlx::query(
            "SELECT path, content_type, content_hash FROM assets WHERE post_id = ? ORDER BY path",
        )
        .bind(&id)
        .fetch_all(&self.pool)
        .await?;
        let asset_manifest = asset_rows
            .iter()
            .map(|r| AssetRef {
                path: r.get("path"),
                content_type: r.get("content_type"),
                content_hash: r.get("content_hash"),
            })
            .collect();

        Ok(PostRecord {
            post,
            translations,
            renders,
            asset_manifest,
        })
    }

    pub(crate) async fn fetch_record_by_id(&self, id: &str) -> Result<PostRecord, StoreError> {
        let row = sqlx::query("SELECT * FROM posts WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(StoreError::NotFound)?;
        self.load_record(&row).await
    }
}

pub(crate) struct LockedPost {
    pub id: String,
    pub revision: Revision,
    pub default_language: Language,
}

pub(crate) async fn lock_post(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    slug: &Slug,
    expected: Revision,
) -> Result<LockedPost, StoreError> {
    let row = sqlx::query("SELECT id, revision, default_language FROM posts WHERE slug = ?")
        .bind(slug.as_str())
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(StoreError::NotFound)?;
    let locked = LockedPost {
        id: row.get("id"),
        revision: Revision(row.get::<i64, _>("revision")),
        default_language: parse_language(row.get("default_language"))?,
    };
    if locked.revision != expected {
        return Err(StoreError::RevisionMismatch {
            expected,
            actual: locked.revision,
        });
    }
    Ok(locked)
}

pub(crate) async fn bump_revision(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    post_id: &str,
) -> Result<(), StoreError> {
    sqlx::query("UPDATE posts SET revision = revision + 1, updated_at = ? WHERE id = ?")
        .bind(now_string())
        .bind(post_id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub(crate) fn now_string() -> String {
    Timestamp::now().to_string()
}

pub(crate) fn parse_timestamp(value: String) -> Result<Timestamp, StoreError> {
    value
        .parse()
        .map_err(|_| StoreError::Constraint(format!("invalid timestamp {value:?}")))
}

pub(crate) fn parse_language(value: String) -> Result<Language, StoreError> {
    Language::parse(&value).map_err(|e| StoreError::Constraint(e.to_string()))
}

pub(crate) fn is_unique_violation(error: &sqlx::Error) -> bool {
    matches!(error, sqlx::Error::Database(db) if db.is_unique_violation())
}

fn row_to_post(row: &sqlx::sqlite::SqliteRow, tags: Vec<String>) -> Result<Post, StoreError> {
    let published_at: Option<String> = row.get("published_at");
    Ok(Post {
        id: PostId::parse(row.get::<String, _>("id").as_str())
            .map_err(|e| StoreError::Constraint(e.to_string()))?,
        slug: Slug::parse(row.get::<String, _>("slug").as_str())
            .map_err(|e| StoreError::Constraint(e.to_string()))?,
        status: row
            .get::<String, _>("status")
            .parse::<PostStatus>()
            .map_err(|e| StoreError::Constraint(e.to_string()))?,
        default_language: parse_language(row.get("default_language"))?,
        revision: Revision(row.get::<i64, _>("revision")),
        tags,
        published_at: published_at.map(parse_timestamp).transpose()?,
        created_at: parse_timestamp(row.get("created_at"))?,
        updated_at: parse_timestamp(row.get("updated_at"))?,
    })
}
