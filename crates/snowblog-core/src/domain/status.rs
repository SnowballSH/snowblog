use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use super::DomainError;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PostStatus {
    Draft,
    Published,
    Archived,
}

impl PostStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Published => "published",
            Self::Archived => "archived",
        }
    }
}

impl fmt::Display for PostStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for PostStatus {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "draft" => Ok(Self::Draft),
            "published" => Ok(Self::Published),
            "archived" => Ok(Self::Archived),
            other => Err(DomainError::InvalidStatus(other.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_all_statuses() {
        for status in [
            PostStatus::Draft,
            PostStatus::Published,
            PostStatus::Archived,
        ] {
            assert_eq!(status.as_str().parse::<PostStatus>().unwrap(), status);
        }
    }

    #[test]
    fn rejects_unknown_status() {
        assert!("deleted".parse::<PostStatus>().is_err());
        assert!("Draft".parse::<PostStatus>().is_err());
    }
}
