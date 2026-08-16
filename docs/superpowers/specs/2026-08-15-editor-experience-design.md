# Editor experience upgrade — design

Date: 2026-08-15
Area: `web-admin` Typst editor

## Goal

Bring the admin write-view editor up to the "basic VSCode features" bar the user
expects — auto-closing brackets (including `$…$`), bracket matching, line numbers,
search/replace, comment toggling, smart bracket/indent behavior — and materially
improve Typst syntax highlighting. Plus a standard unsaved-changes guard when
leaving the write page.

Highlighting is provided by **`codemirror-lang-typst`** (Typst's real syntax parser
compiled to wasm — the same package `probase-2` uses, upgraded to the latest
`0.6.0`). This replaces the current hand-rolled `StreamLanguage` tokenizer entirely.
Rationale: the grammar gives correct, low-maintenance highlighting; the wasm payload
(~313 KB) is acceptable for an admin-only editor and is code-split out of the main
bundle via dynamic import.

## Non-goals

- No autocomplete/IntelliSense for Typst functions or symbols.
- No multiple-cursor / rectangular-selection headline feature (not requested).
- No auto-continue-lists behavior (not requested).
- No code folding.
- No SSR of the editor grammar — it is browser-only by design.

## Affected files

- `web-admin/src/lib/editor/TypstEditor.svelte` — replace the extension set: load the
  wasm Typst grammar (dynamic import), themed `HighlightStyle`, and all interaction
  extensions.
- `web-admin/src/lib/editor/typst-mode.ts` — **delete** (replaced by the grammar).
- `web-admin/src/lib/editor/typst-mode.test.ts` — **delete** (tokenizer is gone).
- `web-admin/src/lib/editor/dirty.ts` (+ `dirty.test.ts`) — **new** pure helper for the
  unsaved-changes comparison, unit-tested.
- `web-admin/src/lib/editor/smart-brackets.ts` (+ `.test.ts`) — **new, only if**
  CodeMirror's default Enter does not already expand `{|}` into an indented block with
  this grammar (verified during implementation).
- `web-admin/src/routes/posts/[slug]/write/[language]/+page.svelte` — unsaved-changes
  leave guard.
- `web-admin/vite.config.ts` — add `vite-plugin-wasm` + `vite-plugin-top-level-await`.
- `web-admin/package.json` — add deps (updated by `bun add`).

## Dependencies to add

Runtime:
- `codemirror-lang-typst` (`^0.6.0`) — Typst grammar + wasm parser.
- `@codemirror/autocomplete` (`^6`) — `closeBrackets()`, `closeBracketsKeymap`.
- `@codemirror/search` (`^6`) — `search()`, `searchKeymap`, `highlightSelectionMatches()`.

Dev:
- `vite-plugin-wasm` (`^3`) — handle the wasm-bindgen bundler-target `.wasm` import.
- `vite-plugin-top-level-await` (`^1`) — the wasm plugin emits top-level `await`.

Already installed and reused: `bracketMatching`, `indentOnInput`, `indentUnit`
(`@codemirror/language`); `lineNumbers`, `highlightActiveLine`,
`highlightActiveLineGutter`, `drawSelection` (`@codemirror/view`); `toggleComment`,
`indentWithTab` (`@codemirror/commands`); `history`, `historyKeymap`, `defaultKeymap`
(already used).

Install with `bun add` / `bun add -d` in `web-admin` so `bun.lock` updates.

## Build & SSR wiring (vite.config.ts)

`codemirror-lang-typst`'s wasm shim is the wasm-bindgen **bundler target**:
`import * as wasm from "./typst_syntax_bg.wasm"`, and it calls
`wasm.__wbindgen_start()` at import time.

- Add `wasm()` and `topLevelAwait()` to the Vite `plugins` array (order: after
  `tailwindcss()` / `sveltekit()` is fine; plugins are order-independent here).
- Do **not** add `codemirror-lang-typst` to `ssr.noExternal`. It must never be
  imported on the server. The editor keeps it off SSR by **dynamic-importing** the
  grammar inside its browser-only mount effect (see below). This also code-splits the
  313 KB wasm into an async chunk.
- Vitest shares this Vite config; our unit tests import only pure helpers (no wasm),
  so the added plugins do not affect the `node`-environment test run.

## Editor assembly (TypstEditor.svelte)

Current mount creates the `EditorView` synchronously in a Svelte `$effect` with a
minimal extension set. Restructure the mount to load the grammar first:

1. In the mount `$effect` (runs in the browser only), use a `cancelled` flag and
   `const { typst } = await import('codemirror-lang-typst')`.
2. If not cancelled, build the full extension list and create the `EditorView`,
   reading the current `value` at creation time (via `untrack`).
3. Cleanup sets `cancelled = true`, destroys the view, clears `view`.

The existing value-sync `$effect` (`if (!view) return; …`) is unchanged and tolerates
`view` being `undefined` until the async import resolves.

**Grammar + language data.** `typst()` returns a self-contained `LanguageSupport`
(it bundles the wasm parser and its sync `StateField` — verify by reading the
installed `dist/index.js`; add `parser.updateListener()` only if `typst()` omits it).
Layer extra language data onto its `.language` for the two gaps:

```ts
const support = typst();
const languageData = support.language.data.of({
  commentTokens: { line: '//', block: { open: '/*', close: '*/' } },
  closeBrackets: { brackets: ['(', '[', '{', '"', '$', '`'] }
});
```

**Interaction extensions** (full set):
- `support`, `languageData`
- `syntaxHighlighting(typstHighlightStyle)` — our own `HighlightStyle` (below); the
  package's `TypstHighlightSytle` is not used (its colors are hardcoded).
- `closeBrackets()`
- `bracketMatching()`
- `lineNumbers()`, `highlightActiveLine()`, `highlightActiveLineGutter()`
- `search({ top: true })`, `highlightSelectionMatches()`
- `indentOnInput()`, `indentUnit.of('  ')` (2 spaces)
- `drawSelection()`
- `history()`
- `EditorView.lineWrapping`
- `placeholderExtension(placeholder)`
- theme (extended, below)
- the doc-change update listener (unchanged)
- keymap (below)

**Keymap** (precedence first):
`[...closeBracketsKeymap, ...defaultKeymap, ...searchKeymap, ...historyKeymap,
indentWithTab, { key: 'Mod-/', run: toggleComment }]`
(prepend the optional `expandBracket` Enter binding ahead of `defaultKeymap` only if
needed — see smart-bracket note).

**HighlightStyle.** Define over standard `@lezer/highlight` tags, mapped to `--fui-*`
tokens (same technique as the current code). Cover the tags the Typst grammar emits —
derive the exact set by reading the grammar's `styleTags` in
`node_modules/codemirror-lang-typst/dist/index.js`. Starting map (reconcile against the
installed 0.6.0 tag set during implementation):

- `tags.comment` → `--fui-ink-muted`, italic
- `tags.lineComment`, `tags.blockComment` → same as comment (if emitted distinctly)
- `tags.string` → `--fui-code-green`
- `tags.escape` → `--fui-code-green`
- `tags.number` → `--fui-code-blue`
- `tags.bool`, `tags.atom` → `--fui-code-blue`
- `tags.keyword`, `tags.controlKeyword`, `tags.definitionKeyword`,
  `tags.moduleKeyword` → `--fui-accent-strong`, weight 600
- `tags.operator` → `--fui-code-violet`
- `tags.function(tags.variableName)`, `tags.variableName` → `--fui-code-violet`
- `tags.meta` → `--fui-code-violet`
- `tags.heading` → `--fui-ink`, weight 700
- `tags.strong` → weight 700
- `tags.emphasis` → italic
- `tags.monospace` → `--fui-code-warm` (raw)
- `tags.labelName` → `--fui-code-warm`
- `tags.link` → `--fui-code-warm`, underline
- `tags.list`, `tags.processingInstruction` → `--fui-code-warm` (markers)

**Theme additions** to the existing `EditorView.theme({...})`:
- `.cm-matchingBracket` (e.g. bold + subtle accent bg), `.cm-nonmatchingBracket`
  (warn color)
- `.cm-activeLine` (subtle accent-tinted bg), `.cm-activeLineGutter` (matching gutter
  bg)
- `.cm-panel.cm-search` and its inputs/buttons → foundationui surface/ink/border
- `.cm-searchMatch`, `.cm-searchMatch-selected`, `.cm-selectionMatch` → accent tints
- keep existing `&`, `.cm-content`, `.cm-scroller`, `.cm-gutters`, `.cm-cursor`,
  selection, `.cm-placeholder` rules

## Smart bracket / indent

Provided by `indentOnInput()` + `indentUnit.of('  ')` + the default keymap. For
"Enter between `{|}` expands into an indented block": first verify whether
CodeMirror's default `insertNewlineAndIndent` already does this with the Typst grammar
(depends on the grammar defining bracket/indent node props). If it does not, add a
small tested `expandBracket` command in `smart-brackets.ts` and bind it to Enter ahead
of `defaultKeymap`. Keep this out of scope unless the default proves insufficient.

## Unsaved-changes leave guard (+page.svelte)

The write page loads `data.translation.{source,title,description}` into local
`$derived` state edited by the form. Add a dirty guard:

- A pure helper `isDirty(baseline, current)` in `dirty.ts` compares the three fields;
  the page treats the edit as dirty when `isDirty(...)` is true AND not `saving`.
- Track a `baseline` (`$state`) initialized from `data.translation`; reset it when the
  loaded post/language pair changes (mirroring the existing `previewedPairKey` reset)
  and on successful save (in the existing `saveSubmit` handler, set baseline to the
  submitted values).
- `beforeNavigate` (from `$app/navigation`): if dirty, `confirm(...)`; on cancel call
  the navigation's `cancel()`. Covers in-app SvelteKit navigation.
- `window.onbeforeunload`: when dirty, set `event.returnValue` to trigger the browser's
  native "leave site?" dialog on tab close / refresh / external nav. (Modern browsers
  ignore any custom string and show a generic dialog — expected.)

## Testing

- **Unit (`dirty.test.ts`)** — `isDirty` returns false for equal baseline/current,
  true when any of source/title/description differs.
- **Unit (`smart-brackets.test.ts`)** — only if the custom Enter command is added;
  test the `expandBracket` predicate (cursor between a matching pair vs not).
- **Manual (browser, `bun run dev`)** — the editor and guard are DOM-level. Verify:
  Typst highlighting renders (headings, `#let`/`#set`, math `$…$`, strings, comments,
  raw); auto-close + skip + wrap for `()[]{}"$`; bracket-match highlight; line numbers
  + active line; Ctrl/Cmd-F search & replace; `Mod-/` line-comment toggle;
  Enter-between-brackets expansion; leave dialog on in-app nav and on
  tab-close/refresh, dirty vs clean.
- **Build** — `bun run check` (svelte-check) and `bun run build` must pass, confirming
  the wasm/TLA Vite wiring works in a production build.
- **Regression** — `bun run lint` and `bun run test` (vitest) green. The deleted
  tokenizer tests are removed from the suite.

## Risks / open points

- **Wasm on SSR.** Mitigated by dynamic-importing the grammar in the browser-only
  mount; the server never imports `codemirror-lang-typst`.
- **`typst()` self-containedness.** Assumed to bundle its parser-sync `StateField`
  (probase used bare `typst()` successfully). Verify against installed `dist/index.js`;
  add `parser.updateListener()` if absent.
- **Grammar tag set.** The starting `HighlightStyle` map is reconciled against the
  actual 0.6.0 `styleTags` at implementation time; unmapped tags simply inherit
  default text color (acceptable, not an error).
- **Enter-expands-brackets / indentation** depend on grammar node props; the custom
  `expandBracket` command is the documented fallback.
- **`0.6.0` vs `0.4.0` API drift.** probase pinned `0.4.0`; confirm `typst`,
  `TypstHighlightSytle` exports still exist in `0.6.0` after install (we only rely on
  `typst`).
