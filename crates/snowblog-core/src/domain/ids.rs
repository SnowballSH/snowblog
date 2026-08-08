use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::DomainError;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PostId(Uuid);

impl PostId {
    pub fn generate() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn parse(input: &str) -> Result<Self, DomainError> {
        Uuid::parse_str(input)
            .map(Self)
            .map_err(|_| DomainError::InvalidId(input.to_string()))
    }
}

impl fmt::Display for PostId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Revision(pub i64);

impl Revision {
    pub const INITIAL: Revision = Revision(1);

    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

impl fmt::Display for Revision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn generates_unique_parseable_ids() {
        let ids: HashSet<String> = (0..100).map(|_| PostId::generate().to_string()).collect();
        assert_eq!(ids.len(), 100);
        for id in &ids {
            assert!(PostId::parse(id).is_ok());
        }
    }

    #[test]
    fn rejects_invalid_id() {
        assert!(PostId::parse("not-a-uuid").is_err());
    }

    #[test]
    fn revision_increments() {
        assert_eq!(Revision::INITIAL.next(), Revision(2));
        assert_eq!(Revision(41).next(), Revision(42));
    }
}
