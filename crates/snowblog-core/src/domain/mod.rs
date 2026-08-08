mod diagnostics;
mod ids;
mod language;
mod slug;
mod status;

pub use diagnostics::{Diagnostic, Severity};
pub use ids::{PostId, Revision};
pub use language::Language;
pub use slug::Slug;
pub use status::PostStatus;

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error(
        "invalid slug {0:?}: expected lowercase alphanumerics separated by single '-' or '_', at most 100 characters"
    )]
    InvalidSlug(String),
    #[error("invalid BCP-47 language tag {0:?}")]
    InvalidLanguage(String),
    #[error("invalid post status {0:?}: expected draft, published, or archived")]
    InvalidStatus(String),
    #[error("invalid post id {0:?}")]
    InvalidId(String),
}
