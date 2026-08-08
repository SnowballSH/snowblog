mod adapt;
mod frontmatter;

pub use adapt::{SourceAdaptation, adapt_source};
pub use frontmatter::{Frontmatter, parse_frontmatter};

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use jiff::Timestamp;
use thiserror::Error;

use crate::domain::{Language, Revision, Slug};
use crate::service::{BlogService, RenderStatus, ServiceError};
use crate::store::{AssetInput, NewPost, TranslationInput};

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("no #metadata((...))<frontmatter> block found")]
    MissingFrontmatter,
    #[error("invalid frontmatter: {0}")]
    InvalidFrontmatter(String),
    #[error("io error on {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    Service(#[from] ServiceError),
    #[error("invalid slug from file name {0:?}")]
    InvalidSlug(String),
    #[error("invalid date {0:?}: expected YYYY-MM-DD")]
    InvalidDate(String),
}

#[derive(Clone, Debug, Default)]
pub struct ImportReport {
    pub imported: Vec<ImportedPost>,
    pub skipped: Vec<String>,
    pub failed: Vec<(String, String)>,
}

#[derive(Clone, Debug)]
pub struct ImportedPost {
    pub slug: String,
    pub status: String,
    pub languages: Vec<String>,
    pub assets: usize,
    pub render_failures: Vec<String>,
}

pub async fn import_dir(
    service: &BlogService,
    dir: &Path,
    adaptation: &SourceAdaptation,
    dry_run: bool,
) -> Result<ImportReport, ImportError> {
    let mut report = ImportReport::default();
    for base_path in base_files(dir)? {
        let stem = base_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        let slug = match Slug::parse(&stem) {
            Ok(slug) => slug,
            Err(_) => {
                report.failed.push((stem, "invalid slug".into()));
                continue;
            }
        };
        match import_one(service, dir, &base_path, &slug, adaptation, dry_run).await {
            Ok(Some(post)) => report.imported.push(post),
            Ok(None) => report.skipped.push(slug.to_string()),
            Err(error) => {
                report.failed.push((slug.to_string(), error.to_string()));
            }
        }
    }
    Ok(report)
}

fn base_files(dir: &Path) -> Result<Vec<PathBuf>, ImportError> {
    let io = |source| ImportError::Io {
        path: dir.to_path_buf(),
        source,
    };
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(io)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().is_some_and(|ext| ext == "typ")
                && !path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.ends_with(".zh.typ"))
        })
        .collect();
    files.sort();
    Ok(files)
}

async fn import_one(
    service: &BlogService,
    dir: &Path,
    base_path: &Path,
    slug: &Slug,
    adaptation: &SourceAdaptation,
    dry_run: bool,
) -> Result<Option<ImportedPost>, ImportError> {
    if service
        .store()
        .get_post(slug)
        .await
        .map_err(ServiceError::Store)?
        .is_some()
    {
        return Ok(None);
    }

    let raw = read(base_path)?;
    let (front, body) = parse_frontmatter(&raw)?;
    let source = adapt_source(&body, adaptation);

    let mut translations = vec![(Language::parse("en").expect("static"), &front, source)];
    let zh_pair;
    if let Some(zh_path) = chinese_path(dir, base_path, front.chinese_source.as_deref()) {
        let zh_raw = read(&zh_path)?;
        let (zh_front, zh_body) = parse_frontmatter(&zh_raw)?;
        let zh_source = adapt_source(&zh_body, adaptation);
        zh_pair = zh_front;
        translations.push((Language::parse("zh").expect("static"), &zh_pair, zh_source));
    }

    let assets = collect_assets(dir, translations.iter().map(|(_, _, s)| s.as_str()))?;
    let published_at = front.date.as_deref().map(parse_date).transpose()?;
    let publish = !front.draft && !front.hidden;

    if dry_run {
        return Ok(Some(ImportedPost {
            slug: slug.to_string(),
            status: if publish {
                "published (dry run)"
            } else {
                "draft (dry run)"
            }
            .into(),
            languages: translations.iter().map(|(l, _, _)| l.to_string()).collect(),
            assets: assets.len(),
            render_failures: Vec::new(),
        }));
    }

    let record = service
        .store()
        .create_post(NewPost {
            slug: slug.clone(),
            default_language: Language::parse("en").expect("static"),
            tags: front.tags.clone(),
            published_at,
        })
        .await
        .map_err(ServiceError::Store)?;
    let mut revision = record.post.revision;

    let result: Result<(Vec<String>, Revision), ImportError> = async {
        let mut render_failures = Vec::new();
        for asset in &assets {
            let outcome = service
                .store()
                .upsert_asset(slug, revision, asset.clone())
                .await
                .map_err(ServiceError::Store)?;
            revision = outcome.post.revision;
        }
        for (language, front, source) in &translations {
            let outcome = service
                .save_translation(
                    slug,
                    revision,
                    TranslationInput {
                        language: language.clone(),
                        title: front.title.clone(),
                        description: front.description.clone().unwrap_or_default(),
                        source: source.clone(),
                    },
                )
                .await?;
            revision = outcome.record.post.revision;
            if matches!(outcome.renders[0].render, RenderStatus::Failed { .. }) {
                render_failures.push(language.to_string());
            }
        }
        Ok((render_failures, revision))
    }
    .await;

    let (render_failures, mut revision) = match result {
        Ok(value) => value,
        Err(error) => {
            let _ = service.store().delete_post(slug, revision).await;
            return Err(error);
        }
    };

    let mut status = "draft".to_string();
    if publish && render_failures.is_empty() {
        match service.publish(slug, revision).await {
            Ok(record) => {
                status = record.post.status.to_string();
                revision = record.post.revision;
            }
            Err(ServiceError::PublishBlocked(_)) => status = "draft (publish blocked)".into(),
            Err(error) => return Err(error.into()),
        }
    } else if publish {
        status = "draft (render failed)".into();
    }
    let _ = revision;

    Ok(Some(ImportedPost {
        slug: slug.to_string(),
        status,
        languages: translations.iter().map(|(l, _, _)| l.to_string()).collect(),
        assets: assets.len(),
        render_failures,
    }))
}

fn read(path: &Path) -> Result<String, ImportError> {
    std::fs::read_to_string(path).map_err(|source| ImportError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn chinese_path(dir: &Path, base_path: &Path, custom: Option<&str>) -> Option<PathBuf> {
    if let Some(custom) = custom {
        let trimmed = custom.trim().trim_start_matches("src/content/blogs/");
        for candidate in [
            dir.join(trimmed),
            dir.join(format!("{trimmed}.typ")),
            dir.join(format!("{trimmed}.zh.typ")),
        ] {
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    let stem = base_path.file_stem()?.to_str()?;
    let fallback = dir.join(format!("{stem}.zh.typ"));
    fallback.is_file().then_some(fallback)
}

fn collect_assets<'a>(
    dir: &Path,
    sources: impl Iterator<Item = &'a str>,
) -> Result<Vec<AssetInput>, ImportError> {
    let mut paths = BTreeSet::new();
    for source in sources {
        for reference in find_asset_references(source) {
            paths.insert(reference);
        }
    }
    paths
        .into_iter()
        .map(|path| {
            let file = dir.join(&path);
            let content = std::fs::read(&file).map_err(|source| ImportError::Io {
                path: file.clone(),
                source,
            })?;
            Ok(AssetInput {
                content_type: content_type_for(&path).to_string(),
                path,
                content,
            })
        })
        .collect()
}

fn find_asset_references(source: &str) -> Vec<String> {
    let mut references = Vec::new();
    for start in ["\"./assets/", "\"assets/"] {
        let mut rest = source;
        while let Some(index) = rest.find(start) {
            let after = &rest[index + 1..];
            if let Some(end) = after.find('"') {
                let raw = &after[..end];
                references.push(raw.trim_start_matches("./").to_string());
                rest = &after[end..];
            } else {
                break;
            }
        }
    }
    references
}

fn content_type_for(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or_default() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

fn parse_date(date: &str) -> Result<Timestamp, ImportError> {
    format!("{date}T00:00:00Z")
        .parse()
        .map_err(|_| ImportError::InvalidDate(date.to_string()))
}
