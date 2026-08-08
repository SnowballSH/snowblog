use std::fmt;

use serde::{Deserialize, Serialize};

use super::DomainError;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Language(String);

impl Language {
    pub fn parse(input: &str) -> Result<Self, DomainError> {
        oxilangtag::LanguageTag::parse_and_normalize(input)
            .map(|tag| Self(tag.into_inner()))
            .map_err(|_| DomainError::InvalidLanguage(input.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for Language {
    type Error = DomainError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl From<Language> for String {
    fn from(language: Language) -> Self {
        language.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_bcp47_tags() {
        for valid in ["en", "zh", "zh-Hant", "pt-BR", "sr-Latn-RS"] {
            assert_eq!(Language::parse(valid).unwrap().as_str(), valid);
        }
    }

    #[test]
    fn rejects_invalid_tags() {
        for invalid in ["", "english language", "zh_CN", "-en", "a", "0en"] {
            assert!(Language::parse(invalid).is_err(), "accepted {invalid:?}");
        }
    }

    #[test]
    fn canonicalizes_case() {
        assert_eq!(
            Language::parse("ZH-hant").unwrap(),
            Language::parse("zh-Hant").unwrap()
        );
        assert_eq!(Language::parse("PT-br").unwrap().as_str(), "pt-BR");
    }
}
