# Architecture

## Rendering pipeline

Typst compilation is embedded as a library (pinned `typst =0.15.1` with
`typst-html`, `typst-kit` for fonts, `typst-assets` for embedded fonts).
Rendering happens **on write**: every translation or asset mutation compiles
synchronously inside `spawn_blocking` under a wall-clock timeout, and the HTML
artifact is stored in the `renders` table with the renderer version and a
BLAKE3 `input_hash` over the source plus the sorted asset manifest. Public
reads never compile. A render is *stale* when its `input_hash` or
`renderer_version` no longer matches; staleness is derived, surfaced in the
admin API, and resolved by the `rerender` operation — never lazily on the
read path and never by auto-unpublishing.

A failed compile keeps the previous artifact: `renders` rows are replaced
only on success. Publishing requires a fresh, successful render for every
translation.

The timeout aborts waiting on the compile, not the compile thread itself;
an abandoned thread finishes in the background and its result is discarded.

## Spike findings (typst 0.15.1)

- Invocation: `typst::compile::<typst_html::HtmlDocument>(&world)` →
  `typst_html::html(&doc, &HtmlOptions { pretty: false })`. HTML export must
  be enabled by building the library with `Feature::Html`; every compile then
  emits a fixed "html export is under active development" warning, which the
  renderer filters out of stored diagnostics.
- The output is a complete HTML document (doctype, `<head>` with generated
  MathML CSS, `<body>`). v1 stores and serves it whole; consumers embed or
  extract as they see fit.
- Math renders as native MathML (`<math>` elements), no SVG fallback.
- `image()` natively emits `<img>` with a base64 data URI. Because data URIs
  inflate image-heavy posts (862 KB observed for one post), the renderer
  injects a `show image` rule that instead references
  `{asset_url_prefix}/{path}` when an asset URL prefix is configured
  (17 KB for the same post); without a prefix the data-URI behavior stands.
- Paged-only elements (`align`, `pad`, cetz canvases, `place`, …) are
  **dropped with a warning, including their content**. The renderer's
  injected preamble maps `align` to a `text-align` div and unwraps `pad`,
  both guarded on `target() == "html"` so `html.frame` subtrees still lay
  out correctly. cetz canvases are wrapped in `html.frame(...)` at import
  time (balanced-parenthesis wrap of `cetz.canvas(` call sites), which
  renders them as inline SVG.
- `#set page(...)` is ignored with a warning; the importer strips these
  lines from legacy sources to keep stored diagnostics clean.
- Typst packages resolve exclusively from the read-only vendored tree
  `vendor/packages/{namespace}/{name}/{version}` (currently `cetz 0.5.2`
  and its dependency `oxifmt 1.0.0`). No network access at render time.
- Path traversal outside the project root is structurally impossible in
  typst 0.15: `VirtualPath` normalization rejects escaping paths before the
  `World` is consulted; the `World` additionally serves project files only
  from the post's own in-memory asset set.

## Invariants

1. `default_language` always exists as a translation.
2. Publish requires fresh successful renders for all translations; published
   posts may serve stale artifacts transiently after renderer upgrades.
3. One `revision` per post aggregate, bumped by every mutation, checked via
   `If-Match` on all mutating admin endpoints (412 mismatch / 428 missing).
4. Failed renders never overwrite artifacts.
5. Hard delete only; no history tables.

## API

Versioned REST JSON under `/api/v1`; errors are RFC 9457
`application/problem+json` with a stable `code`. Public surface: `health`,
list published posts, post detail per language (with ETag/304), post assets.
Admin surface under `/api/v1/admin`, gated by a bearer token loaded from a
file; without a configured token file the admin router is not mounted.
See the server crate's route modules for the full contract.
