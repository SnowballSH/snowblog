use jiff::Timestamp;
use serde::Serialize;

use crate::domain::{Diagnostic, Language, PostId, PostStatus, Revision, Slug};

#[derive(Clone, Debug)]
pub struct NewPost {
    pub slug: Slug,
    pub default_language: Language,
    pub tags: Vec<String>,
    pub published_at: Option<Timestamp>,
}

#[derive(Clone, Debug, Default)]
pub struct PostPatch {
    pub slug: Option<Slug>,
    pub default_language: Option<Language>,
    pub tags: Option<Vec<String>>,
    pub published_at: Option<Option<Timestamp>>,
}

#[derive(Clone, Debug)]
pub struct TranslationInput {
    pub language: Language,
    pub title: String,
    pub description: String,
    pub source: String,
}

#[derive(Clone, Debug)]
pub struct AssetInput {
    pub path: String,
    pub content: Vec<u8>,
    pub content_type: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Post {
    pub id: PostId,
    pub slug: Slug,
    pub status: PostStatus,
    pub default_language: Language,
    pub revision: Revision,
    pub tags: Vec<String>,
    pub published_at: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

#[derive(Clone, Debug, Serialize)]
pub struct Translation {
    pub language: Language,
    pub title: String,
    pub description: String,
    pub source: String,
    pub updated_at: Timestamp,
}

#[derive(Clone, Debug, Serialize)]
pub struct StoredRender {
    pub language: Language,
    pub html: String,
    pub renderer_version: String,
    pub input_hash: String,
    pub warnings: Vec<Diagnostic>,
    pub rendered_at: Timestamp,
}

#[derive(Clone, Debug)]
pub struct RenderArtifact {
    pub html: String,
    pub renderer_version: String,
    pub input_hash: String,
    pub warnings: Vec<Diagnostic>,
    pub rendered_at: Timestamp,
}

#[derive(Clone, Debug, Serialize)]
pub struct AssetRef {
    pub path: String,
    pub content_type: String,
    pub content_hash: String,
}

#[derive(Clone, Debug)]
pub struct Asset {
    pub path: String,
    pub content: Vec<u8>,
    pub content_type: String,
    pub content_hash: String,
    pub updated_at: Timestamp,
}

#[derive(Clone, Debug)]
pub struct PostRecord {
    pub post: Post,
    pub translations: Vec<Translation>,
    pub renders: Vec<StoredRender>,
    pub asset_manifest: Vec<AssetRef>,
}

impl PostRecord {
    pub fn translation(&self, language: &Language) -> Option<&Translation> {
        self.translations.iter().find(|t| &t.language == language)
    }

    pub fn render(&self, language: &Language) -> Option<&StoredRender> {
        self.renders.iter().find(|r| &r.language == language)
    }
}

#[derive(Clone, Debug)]
pub struct PostFilter {
    pub status: Option<PostStatus>,
    pub tag: Option<String>,
    pub limit: u32,
    pub offset: u32,
}

impl Default for PostFilter {
    fn default() -> Self {
        Self {
            status: None,
            tag: None,
            limit: 20,
            offset: 0,
        }
    }
}
