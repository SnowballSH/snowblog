use std::borrow::Cow;
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::time::Duration;

use metrics::with_local_recorder;
use metrics_exporter_prometheus::PrometheusBuilder;
use snowblog_core::store::StoreError;
use snowblog_core::telemetry::{
    RenderOperation, RenderOutcome, SqliteContention, StoreOperation, StoreResult, record_render,
    record_store,
};
use sqlx::error::{DatabaseError, ErrorKind};

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
            record_store(operation, StoreResult::Ok, Duration::from_millis(1));
            record_store(operation, StoreResult::Error, Duration::from_millis(1));
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
    let busy = database_error("261", "unrelated text").sqlite_contention();
    assert_eq!(busy, Some(SqliteContention::Busy));
    assert_eq!(busy.map(SqliteContention::as_str), Some("busy"));
    let locked = database_error("262", "unrelated text").sqlite_contention();
    assert_eq!(locked, Some(SqliteContention::Locked));
    assert_eq!(locked.map(SqliteContention::as_str), Some("locked"));
    assert_eq!(
        database_error("1", "database is locked").sqlite_contention(),
        None
    );
    assert_eq!(
        database_error("not-numeric", "database is locked").sqlite_contention(),
        None
    );
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

fn database_error(code: &'static str, message: &'static str) -> StoreError {
    StoreError::Db(sqlx::Error::database(TestDatabaseError { code, message }))
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
