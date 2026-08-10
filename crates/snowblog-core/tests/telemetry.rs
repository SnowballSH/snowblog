use std::borrow::Cow;
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::path::{Path, PathBuf};
use std::time::Duration;

use metrics::{set_default_local_recorder, with_local_recorder};
use metrics_exporter_prometheus::PrometheusBuilder;
use snowblog_core::domain::{Language, Revision, Slug};
use snowblog_core::store::{NewPost, PostPatch, Store, StoreError};
use snowblog_core::telemetry::{
    RenderOperation, RenderOutcome, SqliteContention, StoreOperation, StoreResult, record_render,
    record_store,
};
use sqlx::error::{DatabaseError, ErrorKind};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Connection, SqliteConnection};

#[test]
fn telemetry_uses_only_bounded_labels_and_structured_sqlite_codes() {
    let recorder = PrometheusBuilder::new()
        .set_buckets(&[1.0])
        .expect("test buckets are non-empty")
        .build_recorder();
    let handle = recorder.handle();

    with_local_recorder(&recorder, || {
        for operation in [
            StoreOperation::GetPost,
            StoreOperation::ListPosts,
            StoreOperation::CreatePost,
            StoreOperation::UpdatePostMeta,
            StoreOperation::SetStatus,
            StoreOperation::DeletePost,
            StoreOperation::SaveTranslation,
            StoreOperation::DeleteTranslation,
            StoreOperation::SaveAsset,
            StoreOperation::DeleteAsset,
            StoreOperation::GetAsset,
            StoreOperation::GetAssets,
            StoreOperation::ReplaceRender,
        ] {
            record_store(
                operation,
                &Ok::<(), StoreError>(()),
                Duration::from_millis(1),
            );
            record_store(
                operation,
                &Err::<(), StoreError>(StoreError::NotFound),
                Duration::from_millis(1),
            );
        }

        for operation in [
            RenderOperation::Preview,
            RenderOperation::Persisted,
            RenderOperation::Rerender,
        ] {
            record_render(operation, RenderOutcome::Success, Duration::from_millis(1));
            record_render(operation, RenderOutcome::Failure, Duration::from_millis(1));
            record_render(
                operation,
                RenderOutcome::Discarded,
                Duration::from_millis(1),
            );
        }
    });

    let exposition = handle.render();
    assert_eq!(
        label_values(&exposition, "snowblog_store_operations_total", "operation"),
        set(&[
            "create_post",
            "delete_asset",
            "delete_post",
            "delete_translation",
            "get_asset",
            "get_assets",
            "get_post",
            "list_posts",
            "replace_render",
            "save_asset",
            "save_translation",
            "set_status",
            "update_post_meta",
        ])
    );
    assert_eq!(
        label_values(&exposition, "snowblog_store_operations_total", "result"),
        set(&["error", "ok"])
    );
    assert_eq!(
        label_values(
            &exposition,
            "snowblog_store_operation_duration_seconds_count",
            "operation",
        ),
        set(&[
            "create_post",
            "delete_asset",
            "delete_post",
            "delete_translation",
            "get_asset",
            "get_assets",
            "get_post",
            "list_posts",
            "replace_render",
            "save_asset",
            "save_translation",
            "set_status",
            "update_post_meta",
        ])
    );
    assert!(
        label_values(
            &exposition,
            "snowblog_store_operation_duration_seconds_count",
            "result",
        )
        .is_empty(),
        "store duration samples must not carry a result label"
    );
    assert_eq!(
        label_values(&exposition, "snowblog_render_attempts_total", "operation"),
        set(&["persisted", "preview", "rerender"])
    );
    assert_eq!(
        label_values(&exposition, "snowblog_render_attempts_total", "outcome"),
        set(&["discarded", "failure", "success"])
    );
    assert_eq!(
        label_values(&exposition, "snowblog_render_duration_seconds", "result"),
        set(&["failure", "success"])
    );
    for operation in ["persisted", "preview", "rerender"] {
        assert_eq!(
            sample_value(
                &exposition,
                "snowblog_render_duration_seconds_count",
                &[("operation", operation), ("result", "success")],
            ),
            2.0,
            "successful and discarded {operation} renders must both count as successful durations"
        );
        assert_eq!(
            sample_value(
                &exposition,
                "snowblog_render_duration_seconds_count",
                &[("operation", operation), ("result", "failure")],
            ),
            1.0,
            "only failed {operation} renders may count as failed durations"
        );
    }

    for forbidden in [
        "secret-value",
        "raw/path",
        "database is locked",
        "post-id",
        "en-US",
    ] {
        assert!(!exposition.contains(forbidden), "leaked {forbidden}");
    }

    assert_eq!(StoreError::NotFound.metric_result(), StoreResult::Error);
}

#[tokio::test(flavor = "current_thread")]
async fn store_boundaries_record_results_and_structured_contention_without_dynamic_values() {
    let recorder = PrometheusBuilder::new()
        .set_buckets(&[1.0])
        .expect("test buckets are non-empty")
        .build_recorder();
    let handle = recorder.handle();
    let _recorder_guard = set_default_local_recorder(&recorder);
    let database = TestDatabase::new();
    let store = Store::open(database.path()).await.unwrap();
    sqlx::query("PRAGMA busy_timeout = 0")
        .execute(store.pool())
        .await
        .unwrap();

    let observed_slug = slug("secret_observed_post");
    store
        .create_post(new_post(observed_slug.clone()))
        .await
        .unwrap();
    store.get_post(&observed_slug).await.unwrap().unwrap();
    let missing_asset = store
        .delete_asset(&observed_slug, Revision::INITIAL, "raw/missing-asset.png")
        .await
        .unwrap_err();
    assert!(matches!(missing_asset, StoreError::NotFound));
    let conflict = store
        .update_post_meta(&observed_slug, Revision(99), PostPatch::default())
        .await
        .unwrap_err();
    assert!(matches!(conflict, StoreError::RevisionMismatch { .. }));

    let options = SqliteConnectOptions::new().filename(database.path());
    let mut locking_connection = SqliteConnection::connect_with(&options).await.unwrap();
    sqlx::query("BEGIN EXCLUSIVE")
        .execute(&mut locking_connection)
        .await
        .unwrap();
    let lock_error = store
        .create_post(new_post(slug("secret_locked_post")))
        .await
        .unwrap_err();
    assert!(matches!(lock_error, StoreError::Db(_)));

    let exposition = handle.render();
    assert_eq!(
        sample_value(
            &exposition,
            "snowblog_store_operations_total",
            &[("operation", "create_post"), ("result", "ok")],
        ),
        1.0
    );
    assert_eq!(
        sample_value(
            &exposition,
            "snowblog_store_operations_total",
            &[("operation", "create_post"), ("result", "error")],
        ),
        1.0
    );
    assert_eq!(
        sample_value(
            &exposition,
            "snowblog_store_operations_total",
            &[("operation", "get_post"), ("result", "ok")],
        ),
        1.0
    );
    assert_eq!(
        sample_value(
            &exposition,
            "snowblog_store_operations_total",
            &[("operation", "delete_asset"), ("result", "error")],
        ),
        1.0
    );
    assert_eq!(
        sample_value(
            &exposition,
            "snowblog_store_operations_total",
            &[("operation", "update_post_meta"), ("result", "error")],
        ),
        1.0
    );
    assert_eq!(
        sample_value(
            &exposition,
            "snowblog_sqlite_contention_total",
            &[("operation", "create_post"), ("kind", "busy")],
        ),
        1.0
    );
    for (operation, expected_count) in [
        ("create_post", 2.0),
        ("get_post", 1.0),
        ("delete_asset", 1.0),
        ("update_post_meta", 1.0),
    ] {
        assert_eq!(
            sample_value(
                &exposition,
                "snowblog_store_operation_duration_seconds_count",
                &[("operation", operation)],
            ),
            expected_count,
            "unexpected duration count for {operation}"
        );
    }
    for forbidden in [
        "database is locked",
        "INSERT INTO posts",
        "secret_observed_post",
        "secret_locked_post",
        "raw/missing-asset.png",
    ] {
        assert!(!exposition.contains(forbidden), "leaked {forbidden}");
    }
}

#[test]
fn sqlite_contention_uses_only_numeric_primary_codes_five_and_six() {
    for (code, expected) in [
        ("5", Some(SqliteContention::Busy)),
        ("6", Some(SqliteContention::Locked)),
        ("261", Some(SqliteContention::Busy)),
        ("262", Some(SqliteContention::Locked)),
        ("517", Some(SqliteContention::Busy)),
        ("518", Some(SqliteContention::Locked)),
        ("1", None),
        ("19", None),
        ("not-numeric", None),
    ] {
        assert_eq!(
            database_error(code, "database is locked").sqlite_contention(),
            expected,
            "unexpected classification for SQLite code {code}"
        );
    }
}

fn slug(value: &str) -> Slug {
    Slug::parse(value).unwrap()
}

fn new_post(slug: Slug) -> NewPost {
    NewPost {
        slug,
        default_language: Language::parse("en").unwrap(),
        tags: vec!["private-tag".to_owned()],
        published_at: None,
    }
}

fn label_values(exposition: &str, family: &str, label: &str) -> BTreeSet<String> {
    let marker = format!("{label}=\"");
    exposition
        .lines()
        .filter(|line| line.starts_with(family))
        .filter_map(|line| {
            let value = line.split_once(&marker)?.1;
            Some(value.split_once('"')?.0.to_owned())
        })
        .collect()
}

fn set(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn sample_value(exposition: &str, family: &str, labels: &[(&str, &str)]) -> f64 {
    let line = exposition
        .lines()
        .filter(|line| line.starts_with(family))
        .find(|line| {
            labels
                .iter()
                .all(|(name, value)| line.contains(&format!("{name}=\"{value}\"")))
        })
        .unwrap_or_else(|| panic!("missing {family} sample for {labels:?}"));
    line.rsplit_once(' ')
        .expect("Prometheus sample has a value")
        .1
        .parse()
        .expect("Prometheus sample value is numeric")
}

fn database_error(code: &'static str, message: &'static str) -> StoreError {
    StoreError::Db(sqlx::Error::database(TestDatabaseError { code, message }))
}

struct TestDatabase {
    path: PathBuf,
}

impl TestDatabase {
    fn new() -> Self {
        Self {
            path: std::env::temp_dir().join(format!(
                "snowblog-telemetry-{}.sqlite",
                uuid::Uuid::now_v7()
            )),
        }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDatabase {
    fn drop(&mut self) {
        for suffix in ["", "-shm", "-wal"] {
            let mut path = self.path.as_os_str().to_os_string();
            path.push(suffix);
            let _ = std::fs::remove_file(PathBuf::from(path));
        }
    }
}

#[derive(Debug)]
struct TestDatabaseError {
    code: &'static str,
    message: &'static str,
}

impl Display for TestDatabaseError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message)
    }
}

impl Error for TestDatabaseError {}

impl DatabaseError for TestDatabaseError {
    fn message(&self) -> &str {
        self.message
    }

    fn code(&self) -> Option<Cow<'_, str>> {
        Some(Cow::Borrowed(self.code))
    }

    fn as_error(&self) -> &(dyn Error + Send + Sync + 'static) {
        self
    }

    fn as_error_mut(&mut self) -> &mut (dyn Error + Send + Sync + 'static) {
        self
    }

    fn into_error(self: Box<Self>) -> Box<dyn Error + Send + Sync + 'static> {
        self
    }

    fn kind(&self) -> ErrorKind {
        ErrorKind::Other
    }
}
