use thiserror::Error;

use crate::domain::{Revision, Slug};
use crate::telemetry::{SqliteContention, StoreResult};

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("revision mismatch: expected {expected}, actual {actual}")]
    RevisionMismatch {
        expected: Revision,
        actual: Revision,
    },
    #[error("slug {0} is already taken")]
    SlugTaken(Slug),
    #[error("post not found")]
    NotFound,
    #[error("the default language must keep a translation")]
    DefaultLanguageViolation,
    #[error("constraint violation: {0}")]
    Constraint(String),
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
}

impl StoreError {
    pub const fn metric_result(&self) -> StoreResult {
        StoreResult::Error
    }

    pub fn sqlite_contention(&self) -> Option<SqliteContention> {
        let code = match self {
            Self::Db(error) => error.as_database_error()?.code()?,
            _ => return None,
        };
        let extended_code = code.parse::<u32>().ok()?;
        SqliteContention::from_primary_code(extended_code & 0xff)
    }
}
