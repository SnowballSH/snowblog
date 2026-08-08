mod fonts;
mod hash;
mod limits;
mod preamble;
mod world;

pub use hash::input_hash;
pub use limits::RenderLimits;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use typst::foundations::Bytes;
use typst::utils::LazyHash;
use typst::{Feature, Library, LibraryExt};
use typst_html::{HtmlDocument, HtmlOptions};
use typst_kit::fonts::FontStore;

use crate::domain::{Diagnostic, Severity};
use world::PostWorld;

const RENDERER_VERSION: &str = env!("SNOWBLOG_TYPST_VERSION");
const DEVELOPMENT_WARNING: &str = "html export is under active development and incomplete";

#[derive(Clone, Debug, Default)]
pub struct RenderOptions {
    pub asset_url_prefix: Option<String>,
}

#[derive(Clone, Debug)]
pub struct RenderAsset {
    pub path: String,
    pub content: bytes::Bytes,
}

#[derive(Clone, Debug, Default)]
pub struct RenderInput {
    pub source: String,
    pub assets: Vec<RenderAsset>,
    pub options: RenderOptions,
}

#[derive(Clone, Debug)]
pub enum RenderOutcome {
    Success {
        html: String,
        warnings: Vec<Diagnostic>,
    },
    Failure {
        diagnostics: Vec<Diagnostic>,
    },
}

impl RenderOutcome {
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }

    fn failure(message: impl Into<String>) -> Self {
        Self::Failure {
            diagnostics: vec![Diagnostic::error(message)],
        }
    }
}

pub struct Renderer {
    library: Arc<LazyHash<Library>>,
    fonts: Arc<FontStore>,
    package_root: PathBuf,
    limits: RenderLimits,
}

impl Renderer {
    pub fn new(package_root: PathBuf, extra_font_dirs: Vec<PathBuf>, limits: RenderLimits) -> Self {
        let library = Library::builder()
            .with_features([Feature::Html].into_iter().collect())
            .build();
        Self {
            library: Arc::new(LazyHash::new(library)),
            fonts: Arc::new(fonts::load_fonts(&extra_font_dirs)),
            package_root,
            limits,
        }
    }

    pub fn version(&self) -> &'static str {
        RENDERER_VERSION
    }

    pub fn limits(&self) -> RenderLimits {
        self.limits
    }

    pub async fn render(&self, input: RenderInput) -> RenderOutcome {
        if input.source.len() > self.limits.max_source_bytes {
            return RenderOutcome::failure(format!(
                "source is {} bytes, exceeding the {} byte limit",
                input.source.len(),
                self.limits.max_source_bytes
            ));
        }
        for asset in &input.assets {
            if asset.content.len() > self.limits.max_asset_bytes {
                return RenderOutcome::failure(format!(
                    "asset {} is {} bytes, exceeding the {} byte limit",
                    asset.path,
                    asset.content.len(),
                    self.limits.max_asset_bytes
                ));
            }
        }

        let source = format!(
            "{}{}",
            preamble::html_preamble(input.options.asset_url_prefix.as_deref()),
            input.source
        );
        let assets: HashMap<String, Bytes> = input
            .assets
            .into_iter()
            .map(|asset| (asset.path, Bytes::new(asset.content.to_vec())))
            .collect();
        let library = Arc::clone(&self.library);
        let fonts = Arc::clone(&self.fonts);
        let package_root = self.package_root.clone();
        let max_html_bytes = self.limits.max_html_bytes;

        let compile = tokio::task::spawn_blocking(move || {
            let world = PostWorld::new(library, fonts, source, assets, package_root);
            let result = typst::compile::<HtmlDocument>(&world);
            let warnings = convert_diagnostics(&result.warnings);
            match result.output {
                Ok(document) => match typst_html::html(&document, &HtmlOptions { pretty: false }) {
                    Ok(html) => RenderOutcome::Success { html, warnings },
                    Err(errors) => RenderOutcome::Failure {
                        diagnostics: convert_diagnostics(&errors),
                    },
                },
                Err(errors) => RenderOutcome::Failure {
                    diagnostics: convert_diagnostics(&errors),
                },
            }
        });

        let outcome = match tokio::time::timeout(self.limits.timeout, compile).await {
            Ok(Ok(outcome)) => outcome,
            Ok(Err(join_error)) => {
                RenderOutcome::failure(format!("render task failed: {join_error}"))
            }
            Err(_) => {
                RenderOutcome::failure(format!("render timed out after {:?}", self.limits.timeout))
            }
        };

        match outcome {
            RenderOutcome::Success { html, .. } if html.len() > max_html_bytes => {
                RenderOutcome::failure(format!(
                    "rendered HTML is {} bytes, exceeding the {} byte limit",
                    html.len(),
                    max_html_bytes
                ))
            }
            other => other,
        }
    }
}

fn convert_diagnostics(diagnostics: &[typst::diag::SourceDiagnostic]) -> Vec<Diagnostic> {
    diagnostics
        .iter()
        .filter(|d| d.message != DEVELOPMENT_WARNING)
        .map(|d| Diagnostic {
            severity: match d.severity {
                typst::diag::Severity::Error => Severity::Error,
                typst::diag::Severity::Warning => Severity::Warning,
            },
            message: d.message.to_string(),
            hints: d.hints.iter().map(|h| h.v.to_string()).collect(),
        })
        .collect()
}
