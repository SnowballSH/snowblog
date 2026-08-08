use thiserror::Error;

use crate::domain::{Revision, Slug};

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
