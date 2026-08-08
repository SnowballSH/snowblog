use std::fmt;

use serde::{Deserialize, Serialize};

use super::DomainError;

const MAX_SLUG_LEN: usize = 100;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Slug(String);

impl Slug {
    pub fn parse(input: &str) -> Result<Self, DomainError> {
        let invalid = || DomainError::InvalidSlug(input.to_string());
        if input.is_empty() || input.len() > MAX_SLUG_LEN {
            return Err(invalid());
        }
        let alnum = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit();
        let mut previous_was_separator = true;
        for c in input.chars() {
            if alnum(c) {
                previous_was_separator = false;
            } else if (c == '-' || c == '_') && !previous_was_separator {
                previous_was_separator = true;
            } else {
                return Err(invalid());
            }
        }
        if previous_was_separator {
            return Err(invalid());
        }
        Ok(Self(input.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Slug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for Slug {
    type Error = DomainError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl From<Slug> for String {
    fn from(slug: Slug) -> Self {
        slug.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_slugs() {
        for valid in ["first_blog", "abc-457", "a1", "x", "a-b_c-9"] {
            assert_eq!(Slug::parse(valid).unwrap().as_str(), valid);
        }
    }

    #[test]
    fn rejects_invalid_slugs() {
        for invalid in [
            "", "Ab", "-x", "x-", "a--b", "a__b", "a-_b", "../etc", "a b", "é", "a.b",
        ] {
            assert!(Slug::parse(invalid).is_err(), "accepted {invalid:?}");
        }
    }

    #[test]
    fn rejects_overlong_slug() {
        let long = "a".repeat(MAX_SLUG_LEN + 1);
        assert!(Slug::parse(&long).is_err());
        let max = "a".repeat(MAX_SLUG_LEN);
        assert!(Slug::parse(&max).is_ok());
    }
}
