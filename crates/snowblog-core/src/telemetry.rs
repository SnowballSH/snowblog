use std::time::Duration;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum StoreOperation {
    GetPost,
    ListPosts,
    CreatePost,
    UpdatePostMeta,
    SetStatus,
    DeletePost,
    SaveTranslation,
    DeleteTranslation,
    SaveAsset,
    DeleteAsset,
    GetAsset,
    GetAssets,
    ReplaceRender,
}

impl StoreOperation {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GetPost => "get_post",
            Self::ListPosts => "list_posts",
            Self::CreatePost => "create_post",
            Self::UpdatePostMeta => "update_post_meta",
            Self::SetStatus => "set_status",
            Self::DeletePost => "delete_post",
            Self::SaveTranslation => "save_translation",
            Self::DeleteTranslation => "delete_translation",
            Self::SaveAsset => "save_asset",
            Self::DeleteAsset => "delete_asset",
            Self::GetAsset => "get_asset",
            Self::GetAssets => "get_assets",
            Self::ReplaceRender => "replace_render",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum StoreResult {
    Ok,
    Error,
}

impl StoreResult {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Error => "error",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SqliteContention {
    Busy,
    Locked,
}

impl SqliteContention {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Busy => "busy",
            Self::Locked => "locked",
        }
    }

    pub(crate) const fn from_primary_code(code: u32) -> Option<Self> {
        match code {
            5 => Some(Self::Busy),
            6 => Some(Self::Locked),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum RenderOperation {
    Preview,
    Persisted,
    Rerender,
}

impl RenderOperation {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Preview => "preview",
            Self::Persisted => "persisted",
            Self::Rerender => "rerender",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum RenderOutcome {
    Success,
    Failure,
    Discarded,
}

impl RenderOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failure => "failure",
            Self::Discarded => "discarded",
        }
    }

    const fn duration_result(self) -> &'static str {
        match self {
            Self::Success | Self::Discarded => "success",
            Self::Failure => "failure",
        }
    }
}

pub fn record_store(operation: StoreOperation, result: StoreResult, duration: Duration) {
    metrics::counter!(
        "snowblog_store_operations_total",
        "operation" => operation.as_str(),
        "result" => result.as_str(),
    )
    .increment(1);
    metrics::histogram!(
        "snowblog_store_operation_duration_seconds",
        "operation" => operation.as_str(),
    )
    .record(duration.as_secs_f64());
}

pub fn record_render(operation: RenderOperation, outcome: RenderOutcome, duration: Duration) {
    metrics::counter!(
        "snowblog_render_attempts_total",
        "operation" => operation.as_str(),
        "outcome" => outcome.as_str(),
    )
    .increment(1);
    metrics::histogram!(
        "snowblog_render_duration_seconds",
        "operation" => operation.as_str(),
        "result" => outcome.duration_result(),
    )
    .record(duration.as_secs_f64());
}
