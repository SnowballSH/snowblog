# Editor experience upgrade — design

Date: 2026-08-15
Area: `web-admin` Typst editor

## Goal

Bring the admin write-view editor up to the "basic VSCode features" bar the user
expects — auto-closing brackets (including `$…$`), bracket matching, line numbers,
search/replace, comment toggling, smart bracket/indent behavior — and materially
improve Typst syntax highlighting. Plus a standard unsaved-changes guard when
leaving the write page.

Highlighting stays a hand-rolled CodeMirror `StreamLanguage` parser (no wasm
grammar), matching this repo's lightweight, well-tested style. `probase-2` uses
`codemirror-lang-typst` (Typst's real parser compiled to wasm); we deliberately do
not, to avoid the wasm dependency and Vite/SSR build complexity for an admin-only
editor.

## Non-goals

- No wasm Typst grammar (`codemirror-lang-typst`).
- No autocomplete/IntelliSense for Typst functions or symbols.
- No multiple-cursor / rectangular-selection headline feature (not requested).
- No auto-continue-lists behavior (not requested).
- No code folding.

## Affected files

- `web-admin/src/lib/editor/typst-mode.ts` — rewrite as a mode-stack parser; export language data.
- `web-admin/src/lib/editor/typst-mode.test.ts` — extend coverage.
- `web-admin/src/lib/editor/TypstEditor.svelte` — assemble the full extension set + theme + highlight style.
- `web-admin/src/routes/posts/[slug]/write/[language]/+page.svelte` — unsaved-changes leave guard.
- `web-admin/package.json` — add two `@codemirror/*` deps.
- `web-admin/src/lib/editor/smart-brackets.ts` (+ `.test.ts`) — **only if** CodeMirror's
  default Enter does not already expand `{|}` into an indented block for this stream
  language (verified during implementation).

## Dependencies to add

Aligned with the `^6` CodeMirror ecosystem already declared in `package.json`
(`@codemirror/state ^6.7.1`, `view ^6.43.8`, `language ^6.12.4`, `commands ^6.10.4`):

- `@codemirror/autocomplete` (`^6`) — `closeBrackets()`, `closeBracketsKeymap`.
- `@codemirror/search` (`^6`) — `search()`, `searchKeymap`, `highlightSelectionMatches()`.

All other extensions used (`bracketMatching`, `indentOnInput`, `indentUnit` from
`@codemirror/language`; `lineNumbers`, `highlightActiveLine`,
`highlightActiveLineGutter`, `drawSelection`, `rectangularSelection` from
`@codemirror/view`; `toggleComment`, `insertNewlineAndIndent` from
`@codemirror/commands`) are already installed.

Install with `bun add` in `web-admin` so `bun.lock` updates.

## Interaction features (TypstEditor.svelte)

The editor currently assembles a minimal extension set (stream language, one
highlight style, history, `defaultKeymap`, line wrapping, placeholder, theme, an
update listener). We extend that set.

1. **Auto-close brackets & `$…$`** — `closeBrackets()`. The bracket set is supplied
   as language data on the Typst language (see below): `( [ { " $ \``. Same-character
   pairs (`"`, `$`, `` ` ``) get CodeMirror's quote-like handling automatically:
   - typing `$` inserts `$|$` (cursor between),
   - typing `$` when the next char is the closing `$` skips over it,
   - with a selection active, typing `$` wraps the selection.
   `closeBracketsKeymap` is added so Backspace deletes an empty pair.
   `'` (apostrophe) is intentionally excluded — auto-closing it in prose is noise.

2. **Bracket matching** — `bracketMatching()`. Theme `.cm-matchingBracket` and
   `.cm-nonmatchingBracket`.

3. **Line numbers + active line** — `lineNumbers()`, `highlightActiveLine()`,
   `highlightActiveLineGutter()`. The gutter is already themed with `--fui-*`;
   extend with active-line background.

4. **Search / replace** — `search({ top: true })`, `searchKeymap` (Ctrl/Cmd-F),
   `highlightSelectionMatches()`. Theme the `.cm-panel.cm-search` panel and
   `.cm-searchMatch` / `.cm-selectionMatch` to foundationui tokens.

5. **Comment toggle** — bind `Mod-/` to `toggleComment`, backed by `commentTokens`
   language data (`line: "//"`, `block: { open: "/*", close: "*/" }`).

6. **Smart bracket / indent** — `indentOnInput()`, `indentUnit.of("  ")` (2 spaces),
   and a `StreamParser.indent` hook that adds one indent unit after a line whose last
   non-space content is an unmatched open bracket. For "Enter between `{|}` expands
   into an indented block": first verify CodeMirror's default `insertNewlineAndIndent`
   already does this for the ASCII bracket pairs. If it does not fire for this stream
   language, add a small tested `expandBracket` command (`smart-brackets.ts`) bound to
   Enter ahead of the default keymap.

7. **drawSelection()** — proper caret/selection rendering (the built-in native
   selection is only acceptable with the bare default setup; once we customize we
   opt into `drawSelection`).

**Keymap order** (precedence first):
`closeBracketsKeymap, defaultKeymap, searchKeymap, historyKeymap, indentWithTab,
{ key: "Mod-/", run: toggleComment }` — plus the optional Enter expander ahead of
`defaultKeymap` if needed.

**Highlight style additions** — extend the existing `HighlightStyle.define([...])`
with: `tags.number`, `tags.operator`, `tags.function(tags.variableName)` (and/or
`tags.variableName`), `tags.monospace` (raw), and a list-marker tag
(`tags.list` / `tags.processingInstruction`). Map onto existing `--fui-code-*`
tokens. Existing tag mappings (comment, string, escape, meta, atom, keyword,
heading, strong, emphasis, labelName, link) are unchanged.

## Syntax highlighting rewrite (typst-mode.ts)

Replace the flat parser with a **mode-stack** `StreamParser`. State:

```ts
interface TypstState {
  stack: Mode[];            // top is current mode: 'markup' | 'code' | 'math'
  inBlockComment: boolean;
  inRawBlock: boolean;      // inside a ```-fenced raw block
  codeDepth: number;        // ()[]{} nesting within the current code frame
}
```

Modes and transitions:

- **markup** (default): headings (`=` at SOL), `*bold*`, `_emph_`, `<label>`,
  `@ref`, `"strings"`, `\escape`, `//` and `/* */` comments — all kept. **Added:**
  inline raw `` `code` ``, fenced raw ```` ```lang … ``` ```` (multi-line via
  `inRawBlock`), and line-start list/enum markers (`-`, `+`, `N.`, `/ term:`).
  `$` pushes **math**; `#` pushes **code**.
- **code** (pushed by `#`): keywords
  (`let set show if else for while import include return context none auto true
  false in and or not`), numbers with optional units
  (`\d+(\.\d+)?(pt|mm|cm|in|em|fr|deg|%)?`), operators (`= == != <= >= => -> + - *
  / :`), `"strings"`, function-call identifiers (ident directly before `(`), and
  brackets `()[]{}` tracked by `codeDepth`. A content block `[` pushes **markup**
  (and its matching `]` pops back). Code mode pops back to the previous mode on
  whitespace/newline when `codeDepth === 0`.
- **math** (pushed by `$`, popped by `$`): operators, numbers, `&` alignment,
  function-like identifiers, `\escape`, and `#`-escapes to code; delimiters emit
  `meta`. Replaces the old flat `atom` run.

**Safety:** the tokenizer must always advance the stream (fall back to
`stream.next()`), guaranteeing termination — preserve the existing "always
advances" test.

**Exports:**
- `typstStreamParser` — kept (tests import it directly).
- `typstLanguage` — `StreamLanguage.define(typstStreamParser)`.
- `typstLanguageData` (or a `typstSupport()` factory) — the `Language.data.of({...})`
  extension carrying `closeBrackets` and `commentTokens`, consumed by
  `TypstEditor.svelte`.

## Unsaved-changes leave guard (+page.svelte)

The write page loads `data.translation.{source,title,description}` into local
`$derived` state edited by the form. Add a dirty guard:

- Compute `dirty` = any of `source`/`title`/`description` differs from the loaded
  baseline, AND not currently `saving`.
- `beforeNavigate` (from `$app/navigation`): if `dirty`, `confirm(...)`; on cancel,
  call the navigation's `cancel()`. Covers in-app SvelteKit navigation (Back link,
  etc.).
- `window.onbeforeunload`: when `dirty`, set `event.returnValue` so the browser
  shows its native "leave site?" dialog on tab close / refresh / external nav.
- On a successful save (existing `saveSubmit` handler), reset the baseline to the
  just-saved values so the page is clean again.

Baseline resets when the loaded `data.translation` changes (navigating to a
different post/language), mirroring the existing `previewedPairKey` reset pattern.

## Testing

- **Unit (`typst-mode.test.ts`)** — extend with cases for: code-mode keywords,
  numbers with units, operators, function-call names, inline raw, fenced raw
  (multi-line), list/enum markers, a content block nested inside code, math
  operators/numbers/escapes. All existing cases must still pass.
- **Unit (`smart-brackets.test.ts`)** — only if the custom Enter command is added;
  test the `expandBracket` predicate (cursor between a matching pair vs not).
- **Manual (browser, `bun run dev`)** — interaction features and the leave guard are
  DOM-level: verify auto-close/skip/wrap for `()[]{}"$`, bracket-match highlight,
  line numbers + active line, Ctrl/Cmd-F search + replace, Mod-/ comment toggle,
  Enter-between-brackets expansion, and the leave dialog on in-app nav and on
  tab-close/refresh (dirty vs clean).
- **Regression** — `bun run lint`, `bun run check` (svelte-check), and the existing
  `vitest` suite.

## Risks / open points

- **Code-mode boundary detection is heuristic.** Typst's rule ("`#expr` extends as
  far as it parses") is not fully reproducible in a stream parser. The pop-on-
  whitespace-at-depth-0 approximation covers the common forms (`#let x = 1`,
  `#set text(size: 12pt)`, `#figure(image("a.png"), caption: [Hello *world*])`,
  `#for i in range(5) [...]`). Rare constructs may mis-highlight; acceptable per the
  "good, not perfect" decision. Tests pin the common forms.
- **Enter-expands-brackets** may already be handled by CodeMirror's default keymap;
  the custom command is a fallback, kept out of scope unless the default proves
  insufficient.
- **`onbeforeunload` string message** is ignored by modern browsers (they show a
  generic dialog); setting `returnValue` to a non-empty value is the supported
  trigger. That's expected and fine.
