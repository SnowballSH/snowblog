use std::path::{Path, PathBuf};
use std::time::Duration;

use snowblog_core::render::{RenderAsset, RenderInput, RenderLimits, RenderOutcome, Renderer};

fn package_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/packages")
}

fn renderer() -> Renderer {
    Renderer::new(package_root(), Vec::new(), test_limits())
}

fn test_limits() -> RenderLimits {
    RenderLimits {
        timeout: Duration::from_secs(30),
        ..RenderLimits::default()
    }
}

fn input(source: &str) -> RenderInput {
    RenderInput {
        source: source.to_string(),
        ..Default::default()
    }
}

fn png_pixel() -> Vec<u8> {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/blogs/assets/doleetcodedaily.png");
    std::fs::read(fixture).expect("fixture png")
}

fn expect_success(outcome: RenderOutcome) -> (String, Vec<snowblog_core::domain::Diagnostic>) {
    match outcome {
        RenderOutcome::Success { html, warnings } => (html, warnings),
        RenderOutcome::Failure { diagnostics } => panic!("render failed: {diagnostics:?}"),
    }
}

fn expect_failure(outcome: RenderOutcome) -> Vec<snowblog_core::domain::Diagnostic> {
    match outcome {
        RenderOutcome::Failure { diagnostics } => {
            assert!(!diagnostics.is_empty());
            diagnostics
        }
        RenderOutcome::Success { .. } => panic!("render unexpectedly succeeded"),
    }
}

#[tokio::test]
async fn renders_plain_markup() {
    let (html, warnings) = expect_success(renderer().render(input("= Hello\nBody text.")).await);
    assert!(html.contains("<h"), "no heading in {html}");
    assert!(html.contains("Body text."));
    assert!(
        !warnings
            .iter()
            .any(|w| w.message.contains("active development")),
        "development warning not filtered: {warnings:?}"
    );
}

#[tokio::test]
async fn renders_math_as_mathml() {
    let (html, _) = expect_success(
        renderer()
            .render(input("$ integral_0^1 x^2 dif x = 1/3 $"))
            .await,
    );
    assert!(html.contains("<math"), "no MathML in {html}");
}

#[tokio::test]
async fn renders_provided_asset_as_data_uri() {
    let mut render_input = input("#image(\"assets/dot.png\")");
    render_input.assets.push(RenderAsset {
        path: "assets/dot.png".to_string(),
        content: png_pixel().into(),
    });
    let (html, _) = expect_success(renderer().render(render_input).await);
    assert!(html.contains("data:image/png"), "no data uri in {html}");
}

#[tokio::test]
async fn rewrites_asset_urls_when_prefix_configured() {
    let mut render_input = input("#image(\"./assets/dot.png\", alt: \"a dot\")");
    render_input.assets.push(RenderAsset {
        path: "assets/dot.png".to_string(),
        content: png_pixel().into(),
    });
    render_input.options.asset_url_prefix = Some("/api/v1/posts/demo/assets/".to_string());
    let (html, _) = expect_success(renderer().render(render_input).await);
    assert!(
        html.contains("src=\"/api/v1/posts/demo/assets/assets/dot.png\""),
        "no rewritten url in {html}"
    );
    assert!(html.contains("alt=\"a dot\""));
    assert!(!html.contains("data:image/png"));
}

#[tokio::test]
async fn missing_asset_fails_with_path_in_message() {
    let diagnostics = expect_failure(
        renderer()
            .render(input("#image(\"assets/nope.png\")"))
            .await,
    );
    assert!(
        diagnostics.iter().any(|d| d.message.contains("nope.png")),
        "no path in {diagnostics:?}"
    );
}

#[tokio::test]
async fn traversal_outside_root_fails() {
    let diagnostics = expect_failure(
        renderer()
            .render(input("#image(\"../../etc/passwd\")"))
            .await,
    );
    assert!(
        !diagnostics.is_empty(),
        "traversal should surface a diagnostic"
    );
}

#[tokio::test]
async fn vendored_package_renders_as_svg_frame() {
    let source = r#"#import "@preview/cetz:0.5.2"
#html.frame(cetz.canvas({
  import cetz.draw: *
  line((0, 0), (1, 1))
}))"#;
    let (html, _) = expect_success(renderer().render(input(source)).await);
    assert!(html.contains("<svg"), "no svg in {html}");
}

#[tokio::test]
async fn unvendored_package_fails_without_network() {
    let diagnostics = expect_failure(
        renderer()
            .render(input("#import \"@preview/notreal:1.0.0\": *"))
            .await,
    );
    assert!(
        diagnostics
            .iter()
            .any(|d| d.message.contains("not vendored")),
        "no vendoring message in {diagnostics:?}"
    );
}

#[tokio::test]
async fn timeout_aborts_slow_render() {
    let limits = RenderLimits {
        timeout: Duration::from_millis(1),
        ..RenderLimits::default()
    };
    let slow = Renderer::new(package_root(), Vec::new(), limits);
    let diagnostics = expect_failure(slow.render(input("= Never fast enough")).await);
    assert!(
        diagnostics.iter().any(|d| d.message.contains("timed out")),
        "no timeout in {diagnostics:?}"
    );
}

#[tokio::test]
async fn oversized_source_rejected() {
    let limits = RenderLimits {
        max_source_bytes: 1024,
        ..test_limits()
    };
    let bounded = Renderer::new(package_root(), Vec::new(), limits);
    let big = format!("= T\n{}", "a".repeat(2048));
    let diagnostics = expect_failure(bounded.render(input(&big)).await);
    assert!(diagnostics.iter().any(|d| d.message.contains("byte limit")));
}

#[tokio::test]
async fn oversized_asset_rejected() {
    let limits = RenderLimits {
        max_asset_bytes: 16,
        ..test_limits()
    };
    let bounded = Renderer::new(package_root(), Vec::new(), limits);
    let mut render_input = input("= T");
    render_input.assets.push(RenderAsset {
        path: "assets/big.bin".to_string(),
        content: vec![0u8; 64].into(),
    });
    let diagnostics = expect_failure(bounded.render(render_input).await);
    assert!(
        diagnostics
            .iter()
            .any(|d| d.message.contains("assets/big.bin"))
    );
}

#[tokio::test]
async fn broken_source_reports_error_diagnostics() {
    let diagnostics = expect_failure(renderer().render(input("#undefined_function()")).await);
    assert!(
        diagnostics
            .iter()
            .any(|d| d.severity == snowblog_core::domain::Severity::Error),
        "no error severity in {diagnostics:?}"
    );
}

#[tokio::test]
async fn version_reports_pinned_typst() {
    assert_eq!(renderer().version(), "0.15.1");
}
