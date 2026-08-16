# Editor Experience Upgrade — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give the `web-admin` Typst write-view a proper editor — real Typst syntax highlighting plus VSCode-style basics (auto-close brackets incl. `$…$`, bracket matching, line numbers, search/replace, comment toggle, smart indent) — and a standard unsaved-changes leave guard.

**Architecture:** Replace the hand-rolled `StreamLanguage` tokenizer with the `codemirror-lang-typst` wasm grammar, dynamically imported inside the browser-only editor mount so it stays off the SSR path and code-splits the wasm. All interaction niceties are stock CodeMirror 6 extensions assembled in `TypstEditor.svelte`, themed with foundationui `--fui-*` tokens. The leave guard lives in the write page and uses a small pure `isDirty` helper.

**Tech Stack:** SvelteKit (Svelte 5 runes) + Vite + adapter-node, bun, CodeMirror 6, `codemirror-lang-typst` (wasm-bindgen bundler target), Vitest.

## Global Constraints

- Package manager is **bun**; install deps with `bun add` / `bun add -d` so `bun.lock` updates. Run scripts with `bun run <script>`.
- All work happens under `web-admin/`. The current branch is `feat/editor-experience`.
- CodeMirror deps are the `^6` line (already: `@codemirror/state ^6.7.1`, `view ^6.43.8`, `language ^6.12.4`, `commands ^6.10.4`, `@lezer/highlight ^1.2.3`). New CM deps use `^6`.
- `codemirror-lang-typst` target version: **`^0.6.0`**.
- The grammar (`codemirror-lang-typst`) must **never** be imported on the server — only via dynamic `import()` inside a browser-only mount. Do not add it to `ssr.noExternal`.
- Theme every editor surface with existing `--fui-*` CSS variables (see `TypstEditor.svelte` for the ones already in use: `--fui-surface-base`, `--fui-ink`, `--fui-ink-muted`, `--fui-ink-secondary`, `--fui-accent`, `--fui-accent-strong`, `--fui-code-green`, `--fui-code-blue`, `--fui-code-violet`, `--fui-code-warm`, `--fui-font-mono`, `--fui-line`/`border-line`).
- Indentation unit is **2 spaces**.
- Vitest environment is `node`; unit tests must import only pure modules (no `.svelte`, no wasm grammar).
- Verification commands: `bun run test`, `bun run lint`, `bun run check`, `bun run build`. Claims of "passing" require actually running these and seeing the output.

---

## Task 1: Add dependencies and Vite wasm wiring

Establishes the build foundation. No behavior change yet, but ends with a green build so the wasm/TLA plumbing is proven before any editor code depends on it.

**Files:**
- Modify: `web-admin/package.json` (via `bun add`)
- Modify: `web-admin/vite.config.ts`

**Interfaces:**
- Produces: `codemirror-lang-typst`, `@codemirror/autocomplete`, `@codemirror/search` available as runtime deps; `vite-plugin-wasm`, `vite-plugin-top-level-await` as dev deps; Vite configured to bundle `.wasm` bundler-target imports.

- [ ] **Step 1: Install runtime deps**

Run in `web-admin/`:

```bash
cd web-admin && bun add codemirror-lang-typst@^0.6.0 @codemirror/autocomplete@^6 @codemirror/search@^6
```

- [ ] **Step 2: Install dev deps**

```bash
cd web-admin && bun add -d vite-plugin-wasm vite-plugin-top-level-await
```

- [ ] **Step 3: Wire the Vite plugins**

Edit `web-admin/vite.config.ts`. Add the imports at the top:

```ts
import wasm from 'vite-plugin-wasm';
import topLevelAwait from 'vite-plugin-top-level-await';
```

Add both to the `plugins` array (after the existing `sveltekit({...})` entry):

```ts
	plugins: [
		tailwindcss(),
		sveltekit({
			compilerOptions: {
				runes: ({ filename }) =>
					filename.split(/[/\\]/).includes('node_modules') ? undefined : true
			},
			adapter: adapter(),
			paths: { base: '/blogs' }
		}),
		wasm(),
		topLevelAwait()
	],
```

Leave `ssr.noExternal: ['ulid']` unchanged — do NOT add the grammar there.

- [ ] **Step 4: Confirm the grammar API shape (read, don't guess)**

Read `web-admin/node_modules/codemirror-lang-typst/dist/index.js` and `dist/index.d.ts`. Confirm/record:
- `typst()` is exported and returns a `LanguageSupport`.
- Whether `typst()`'s `LanguageSupport` already includes the parser-sync `StateField` (look for `updateListener()` being added to the `Language`'s extension array). Note the answer — Task 3 depends on it.
- The full set of `@lezer/highlight` `tags.*` used in the grammar's `styleTags` map. Note the list — Task 3's `HighlightStyle` maps these.
- The wasm shim `wasm/typst_syntax.js` does `import * as wasm from "./typst_syntax_bg.wasm"` (bundler target) — confirms the Vite plugins are required.

- [ ] **Step 5: Verify install + build still pass**

Run:

```bash
cd web-admin && bun run check && bun run build
```

Expected: both succeed. `bun run build` proves the wasm/TLA plugins load without error even though nothing imports the grammar yet.

- [ ] **Step 6: Commit**

```bash
cd web-admin && git add package.json bun.lock vite.config.ts && git commit -m "build(web-admin): add codemirror-lang-typst + wasm vite plugins"
```

---

## Task 2: Unsaved-changes dirty helper

A pure, unit-testable helper for the leave guard. Isolated so its logic is proven before it's wired into the Svelte page (Task 5).

**Files:**
- Create: `web-admin/src/lib/editor/dirty.ts`
- Test: `web-admin/src/lib/editor/dirty.test.ts`

**Interfaces:**
- Produces:
  ```ts
  export interface EditorFields { source: string; title: string; description: string }
  export function isDirty(baseline: EditorFields, current: EditorFields): boolean
  ```
  Returns `true` iff any of the three fields differs.

- [ ] **Step 1: Write the failing test**

Create `web-admin/src/lib/editor/dirty.test.ts`:

```ts
import { describe, expect, it } from 'vitest';
import { isDirty, type EditorFields } from './dirty.js';

const base: EditorFields = { source: 'a', title: 't', description: 'd' };

describe('isDirty', () => {
	it('is false when all fields match', () => {
		expect(isDirty(base, { ...base })).toBe(false);
	});

	it('is true when source differs', () => {
		expect(isDirty(base, { ...base, source: 'a2' })).toBe(true);
	});

	it('is true when title differs', () => {
		expect(isDirty(base, { ...base, title: 't2' })).toBe(true);
	});

	it('is true when description differs', () => {
		expect(isDirty(base, { ...base, description: 'd2' })).toBe(true);
	});
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd web-admin && bun run test:unit -- --run src/lib/editor/dirty.test.ts
```

Expected: FAIL — cannot resolve `./dirty.js`.

- [ ] **Step 3: Write the implementation**

Create `web-admin/src/lib/editor/dirty.ts`:

```ts
export interface EditorFields {
	source: string;
	title: string;
	description: string;
}

export function isDirty(baseline: EditorFields, current: EditorFields): boolean {
	return (
		baseline.source !== current.source ||
		baseline.title !== current.title ||
		baseline.description !== current.description
	);
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd web-admin && bun run test:unit -- --run src/lib/editor/dirty.test.ts
```

Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
cd web-admin && git add src/lib/editor/dirty.ts src/lib/editor/dirty.test.ts && git commit -m "feat(web-admin): add isDirty helper for the write-view leave guard"
```

---

## Task 3: Rebuild TypstEditor on the wasm grammar + full extension set

The core change. Swaps the hand-rolled parser for the wasm grammar (dynamic-imported), adds every interaction extension, and themes them. Deletes the obsolete tokenizer and its tests.

**Files:**
- Rewrite: `web-admin/src/lib/editor/TypstEditor.svelte`
- Delete: `web-admin/src/lib/editor/typst-mode.ts`
- Delete: `web-admin/src/lib/editor/typst-mode.test.ts`

**Interfaces:**
- Consumes: `codemirror-lang-typst` `typst()`; `@codemirror/autocomplete` `closeBrackets`, `closeBracketsKeymap`; `@codemirror/search` `search`, `searchKeymap`, `highlightSelectionMatches`; `@codemirror/language` `bracketMatching`, `indentOnInput`, `indentUnit`, `HighlightStyle`, `syntaxHighlighting`; `@codemirror/view` `lineNumbers`, `highlightActiveLine`, `highlightActiveLineGutter`, `drawSelection`; `@codemirror/commands` `toggleComment`, `indentWithTab`.
- Produces: `<TypstEditor bind:value placeholder />` — unchanged public props, so `+page.svelte` needs no change for the editor swap.

- [ ] **Step 1: Delete the obsolete tokenizer + its test**

```bash
cd web-admin && git rm src/lib/editor/typst-mode.ts src/lib/editor/typst-mode.test.ts
```

- [ ] **Step 2: Rewrite `TypstEditor.svelte`**

**IMPORTANT — verified integration facts (do not deviate without re-checking the package source):**

- `typst()` returns a `LanguageSupport` whose highlighting is done by an exported
  `ViewPlugin` (`typstHighlighting`) that runs its OWN wasm parser per view and applies
  decorations, resolving each token's CSS class via `highlightingFor(state, [tag])`
  (reads the registered `HighlightStyle`s). It ALSO bundles two `syntaxHighlighting(...)`
  calls with hardcoded colors. `highlightingFor` **combines** classes from every
  registered style, so layering our style on top of `typst()` wholesale would apply both
  palettes. Therefore assemble the language manually: use `typst().language` +
  the exported `typstHighlighting` ViewPlugin + only OUR `syntaxHighlighting(...)`.
- Importing anything from `codemirror-lang-typst` triggers wasm load at module eval
  (top-level `get_highlight_tags()`). So the grammar import stays inside the browser-only
  mount, and because our `HighlightStyle` references the package's custom tags
  (`typstTags.{mathDelimiter,listMarker,interpolated}`), the `HighlightStyle` is built
  INSIDE that async callback too — not at module top level.
- No parser-sync `StateField`/`updateListener` is needed (deprecated no-op in 0.6.0).
- Exact tag set the highlighter emits (map every one): `tags.comment`,
  `tags.punctuation`, `tags.escape` (also labels/refs/math-ops), `tags.strong` (also
  terms), `tags.emphasis`, `tags.link`, `tags.monospace` (raw), `tags.heading`,
  `tags.keyword`, `tags.operator`, `tags.number`, `tags.string`,
  `tags.function(tags.variableName)`, `tags.invalid` (errors), `tags.paren` (math
  groups), and custom `typstTags.mathDelimiter`, `typstTags.listMarker`,
  `typstTags.interpolated`.

Replace the entire file `web-admin/src/lib/editor/TypstEditor.svelte` with:

```svelte
<script lang="ts">
	import { untrack } from 'svelte';
	import { EditorState, Prec, type Extension } from '@codemirror/state';
	import {
		EditorView,
		keymap,
		lineNumbers,
		highlightActiveLine,
		highlightActiveLineGutter,
		drawSelection,
		placeholder as placeholderExtension
	} from '@codemirror/view';
	import {
		defaultKeymap,
		history,
		historyKeymap,
		indentWithTab,
		toggleComment
	} from '@codemirror/commands';
	import {
		HighlightStyle,
		bracketMatching,
		indentOnInput,
		indentUnit,
		syntaxHighlighting
	} from '@codemirror/language';
	import { closeBrackets, closeBracketsKeymap } from '@codemirror/autocomplete';
	import { search, searchKeymap, highlightSelectionMatches } from '@codemirror/search';
	import { tags } from '@lezer/highlight';
	import type { Tag } from '@lezer/highlight';

	let { value = $bindable(''), placeholder = '' }: { value?: string; placeholder?: string } =
		$props();

	let host: HTMLDivElement | undefined;
	let view: EditorView | undefined = $state();

	// Built inside the async mount because it references the grammar's custom tags,
	// and importing the grammar triggers wasm load (must stay off SSR).
	function makeHighlightStyle(typstTags: {
		mathDelimiter: Tag;
		listMarker: Tag;
		interpolated: Tag;
	}): HighlightStyle {
		return HighlightStyle.define([
			{ tag: tags.comment, color: 'var(--fui-ink-muted)', fontStyle: 'italic' },
			{ tag: tags.string, color: 'var(--fui-code-green)' },
			{ tag: tags.escape, color: 'var(--fui-code-green)' },
			{ tag: tags.number, color: 'var(--fui-code-blue)' },
			{ tag: tags.keyword, color: 'var(--fui-accent-strong)', fontWeight: '600' },
			{ tag: tags.operator, color: 'var(--fui-code-violet)' },
			{ tag: tags.function(tags.variableName), color: 'var(--fui-code-violet)' },
			{ tag: tags.heading, color: 'var(--fui-ink)', fontWeight: '700' },
			{ tag: tags.strong, fontWeight: '700' },
			{ tag: tags.emphasis, fontStyle: 'italic' },
			{ tag: tags.link, color: 'var(--fui-code-warm)', textDecoration: 'underline' },
			{ tag: tags.monospace, color: 'var(--fui-code-warm)' },
			{ tag: tags.punctuation, color: 'var(--fui-ink-secondary)' },
			{ tag: tags.paren, color: 'var(--fui-ink-secondary)' },
			{ tag: tags.invalid, color: 'var(--fui-code-warm)' },
			{ tag: typstTags.mathDelimiter, color: 'var(--fui-code-violet)' },
			{ tag: typstTags.listMarker, color: 'var(--fui-code-warm)' },
			{ tag: typstTags.interpolated, color: 'var(--fui-code-violet)' }
		]);
	}

	const theme = EditorView.theme({
		'&': {
			height: '100%',
			fontSize: '0.9rem',
			backgroundColor: 'var(--fui-surface-base)',
			color: 'var(--fui-ink)'
		},
		'&.cm-focused': { outline: 'none' },
		'.cm-content': {
			fontFamily: 'var(--fui-font-mono)',
			caretColor: 'var(--fui-accent)'
		},
		'.cm-scroller': { overflow: 'auto', fontFamily: 'var(--fui-font-mono)' },
		'.cm-gutters': {
			backgroundColor: 'var(--fui-surface-base)',
			color: 'var(--fui-ink-muted)',
			border: 'none'
		},
		'.cm-cursor': { borderLeftColor: 'var(--fui-accent)' },
		'&.cm-focused .cm-selectionBackground, .cm-selectionBackground': {
			backgroundColor: 'color-mix(in oklab, var(--fui-accent) 24%, transparent)'
		},
		'.cm-activeLine': {
			backgroundColor: 'color-mix(in oklab, var(--fui-accent) 7%, transparent)'
		},
		'.cm-activeLineGutter': {
			backgroundColor: 'color-mix(in oklab, var(--fui-accent) 10%, transparent)',
			color: 'var(--fui-ink-secondary)'
		},
		'.cm-matchingBracket, &.cm-focused .cm-matchingBracket': {
			backgroundColor: 'color-mix(in oklab, var(--fui-accent) 22%, transparent)',
			color: 'var(--fui-ink)',
			fontWeight: '700'
		},
		'.cm-nonmatchingBracket': { color: 'var(--fui-code-warm)' },
		'.cm-selectionMatch': {
			backgroundColor: 'color-mix(in oklab, var(--fui-accent) 14%, transparent)'
		},
		'.cm-searchMatch': {
			backgroundColor: 'color-mix(in oklab, var(--fui-code-warm) 32%, transparent)'
		},
		'.cm-searchMatch-selected': {
			backgroundColor: 'color-mix(in oklab, var(--fui-accent) 40%, transparent)'
		},
		'.cm-panel.cm-search': {
			backgroundColor: 'var(--fui-surface-base)',
			color: 'var(--fui-ink)',
			borderTop: '1px solid var(--fui-line)'
		},
		'.cm-panel.cm-search input, .cm-panel.cm-search button, .cm-panel.cm-search label': {
			fontFamily: 'var(--fui-font-mono)',
			color: 'var(--fui-ink)'
		},
		'.cm-placeholder': { color: 'var(--fui-ink-muted)' }
	});

	function buildExtensions(
		language: Extension,
		languageData: Extension,
		typstHighlighting: Extension,
		highlightStyle: HighlightStyle
	): Extension[] {
		return [
			language,
			languageData,
			typstHighlighting,
			syntaxHighlighting(highlightStyle),
			closeBrackets(),
			bracketMatching(),
			lineNumbers(),
			highlightActiveLine(),
			highlightActiveLineGutter(),
			search({ top: true }),
			highlightSelectionMatches(),
			indentOnInput(),
			indentUnit.of('  '),
			drawSelection(),
			history(),
			keymap.of([
				...closeBracketsKeymap,
				...defaultKeymap,
				...searchKeymap,
				...historyKeymap,
				indentWithTab,
				{ key: 'Mod-/', run: toggleComment }
			]),
			EditorView.lineWrapping,
			placeholderExtension(placeholder),
			theme,
			EditorView.updateListener.of((update) => {
				if (update.docChanged) value = update.state.doc.toString();
			})
		];
	}

	$effect(() => {
		if (!host) return;
		const mountPoint = host;
		let cancelled = false;
		void (async () => {
			const { typst, typstHighlighting, typstTags } = await import('codemirror-lang-typst');
			if (cancelled) return;
			const language = typst().language;
			// Our commentTokens must win over the grammar's block-only default so that
			// Mod-/ line-comment toggle works; Prec.high makes ours resolve first.
			const languageData = Prec.high(
				language.data.of({
					commentTokens: { line: '//', block: { open: '/*', close: '*/' } },
					closeBrackets: { brackets: ['(', '[', '{', '"', '$', '`'] }
				})
			);
			const highlightStyle = makeHighlightStyle(typstTags);
			const initialDoc = untrack(() => value);
			view = new EditorView({
				state: EditorState.create({
					doc: initialDoc,
					extensions: buildExtensions(language, languageData, typstHighlighting, highlightStyle)
				}),
				parent: mountPoint
			});
		})();
		return () => {
			cancelled = true;
			view?.destroy();
			view = undefined;
		};
	});

	$effect(() => {
		const next = value;
		if (!view) return;
		if (next === view.state.doc.toString()) return;
		view.dispatch({
			changes: { from: 0, to: view.state.doc.length, insert: next }
		});
	});
</script>

<div class="h-full w-full" bind:this={host}></div>
```

- [ ] **Step 3: Reconcile the HighlightStyle and grammar API against the installed package**

The code above already reflects the verified 0.6.0 integration (Task 1's Step 4 findings). Confirm against the installed package before trusting the checker:
- Confirm `typstHighlighting` and `typstTags` are named exports of `codemirror-lang-typst` (grep `node_modules/codemirror-lang-typst/dist/index.js` for `export {`). They are in 0.6.0; if a version bump renamed them, adjust the import.
- Confirm `--fui-ink-secondary` exists as a CSS token (grep the foundationui package / existing styles). If not, fall back to `--fui-ink-muted` for `punctuation`/`paren`.
- `bun run check` (next step) surfaces any `tags.*` name that does not exist on `@lezer/highlight`; remove or correct it. The tags used above are all real 0.6.0-emitted tags, so none should fail — but do not add speculative tags that the highlighter never emits.

- [ ] **Step 4: Type/lint check**

```bash
cd web-admin && bun run check
```

Expected: no errors from `TypstEditor.svelte`. Fix any tag/type mismatch surfaced here (this is where a wrong `tags.*` name shows up).

- [ ] **Step 5: Verify the remaining unit suite is green (tokenizer tests are gone)**

```bash
cd web-admin && bun run test
```

Expected: PASS. The `typst-mode.test.ts` cases are removed; `dirty.test.ts` and other existing suites (`debounce`, `preview-controller`) pass.

- [ ] **Step 6: Manual browser verification**

```bash
cd web-admin && bun run dev
```

Open a write view (`/blogs/posts/<slug>/write/<language>`). Confirm:
- Typst highlighting: a heading (`= Title`), `#let x = 1`, `#set text(size: 12pt)`, `$x + 1$`, a `"string"`, `// comment`, and inline `` `raw` `` all colorize distinctly.
- Auto-close: typing `(`, `[`, `{`, `"`, `$` inserts the pair; typing the closer over it skips; selecting text then typing `$` wraps it.
- Bracket matching highlights the partner bracket.
- Line numbers show; the active line + its gutter are tinted.
- `Cmd/Ctrl-F` opens the search panel (themed), find & replace work.
- `Cmd/Ctrl-/` toggles `//` line comments on the selection.
- Pressing Enter with the caret between `{` and `}` — note whether it expands into an indented block (informs Task 4).

Record the Enter-between-brackets result; stop the dev server.

- [ ] **Step 7: Commit**

```bash
cd web-admin && git add -A src/lib/editor && git commit -m "feat(web-admin): rebuild the Typst editor on the wasm grammar with VSCode-style features"
```

---

## Task 4: Smart Enter-expands-brackets (CONDITIONAL)

Only do this task if Task 3 Step 6 showed that Enter between `{|}` does **not** expand into an indented block. If it already works, mark this task skipped and move on.

**Files:**
- Create: `web-admin/src/lib/editor/smart-brackets.ts`
- Test: `web-admin/src/lib/editor/smart-brackets.test.ts`
- Modify: `web-admin/src/lib/editor/TypstEditor.svelte` (keymap)

**Interfaces:**
- Produces:
  ```ts
  // Pure predicate — testable without a DOM.
  export function isInsidePair(before: string, after: string): boolean;
  // CodeMirror command bound to Enter.
  export const expandBracketOnEnter: (view: EditorView) => boolean;
  ```
  `isInsidePair` returns true when `before`/`after` are a matching open/close pair among `()`, `[]`, `{}`.

- [ ] **Step 1: Write the failing test**

Create `web-admin/src/lib/editor/smart-brackets.test.ts`:

```ts
import { describe, expect, it } from 'vitest';
import { isInsidePair } from './smart-brackets.js';

describe('isInsidePair', () => {
	it('is true for matching pairs', () => {
		expect(isInsidePair('{', '}')).toBe(true);
		expect(isInsidePair('[', ']')).toBe(true);
		expect(isInsidePair('(', ')')).toBe(true);
	});

	it('is false for mismatched or non-bracket chars', () => {
		expect(isInsidePair('{', ')')).toBe(false);
		expect(isInsidePair('a', 'b')).toBe(false);
		expect(isInsidePair('', '')).toBe(false);
		expect(isInsidePair('}', '{')).toBe(false);
	});
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd web-admin && bun run test:unit -- --run src/lib/editor/smart-brackets.test.ts
```

Expected: FAIL — cannot resolve `./smart-brackets.js`.

- [ ] **Step 3: Implement**

Create `web-admin/src/lib/editor/smart-brackets.ts`:

```ts
import { EditorSelection } from '@codemirror/state';
import type { EditorView } from '@codemirror/view';

const PAIRS: Record<string, string> = { '(': ')', '[': ']', '{': '}' };

export function isInsidePair(before: string, after: string): boolean {
	return before in PAIRS && PAIRS[before] === after;
}

// If every selection range is an empty caret sitting between a matching bracket
// pair, insert an indented blank line and push the closer down (VSCode behavior).
export const expandBracketOnEnter = (view: EditorView): boolean => {
	const { state } = view;
	const ranges = state.selection.ranges;
	if (!ranges.every((r) => r.empty)) return false;
	const applicable = ranges.every((r) => {
		const before = state.doc.sliceString(r.from - 1, r.from);
		const after = state.doc.sliceString(r.from, r.from + 1);
		return isInsidePair(before, after);
	});
	if (!applicable) return false;

	const indentUnit = '  ';
	const changes = state.changeByRange((range) => {
		const line = state.doc.lineAt(range.from);
		const baseIndent = /^\s*/.exec(line.text)?.[0] ?? '';
		const inner = baseIndent + indentUnit;
		const insert = `\n${inner}\n${baseIndent}`;
		return {
			changes: { from: range.from, insert },
			range: EditorSelection.cursor(range.from + 1 + inner.length)
		};
	});
	view.dispatch(state.update(changes, { scrollIntoView: true, userEvent: 'input' }));
	return true;
};
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd web-admin && bun run test:unit -- --run src/lib/editor/smart-brackets.test.ts
```

Expected: PASS.

- [ ] **Step 5: Bind it to Enter in `TypstEditor.svelte`**

Add the import:

```ts
	import { expandBracketOnEnter } from './smart-brackets.js';
```

Prepend an Enter binding **before** `...defaultKeymap` in the `keymap.of([...])` array:

```ts
			keymap.of([
				...closeBracketsKeymap,
				{ key: 'Enter', run: expandBracketOnEnter },
				...defaultKeymap,
				...searchKeymap,
				...historyKeymap,
				indentWithTab,
				{ key: 'Mod-/', run: toggleComment }
			]),
```

- [ ] **Step 6: Verify in the browser**

```bash
cd web-admin && bun run dev
```

Confirm Enter between `{|}` now expands into an indented block with the closer on its own line; a normal Enter elsewhere is unaffected. Stop the server.

- [ ] **Step 7: Commit**

```bash
cd web-admin && git add src/lib/editor/smart-brackets.ts src/lib/editor/smart-brackets.test.ts src/lib/editor/TypstEditor.svelte && git commit -m "feat(web-admin): expand brackets on Enter in the Typst editor"
```

---

## Task 5: Unsaved-changes leave guard in the write page

Wires the `isDirty` helper into the write route, guarding both in-app navigation and browser unload.

**Files:**
- Modify: `web-admin/src/routes/posts/[slug]/write/[language]/+page.svelte`

**Interfaces:**
- Consumes: `isDirty`, `EditorFields` from `$lib/editor/dirty.js`; `beforeNavigate` from `$app/navigation`.

- [ ] **Step 1: Import the helper and `beforeNavigate`**

In the `<script>` of `web-admin/src/routes/posts/[slug]/write/[language]/+page.svelte`, add to the existing imports:

```ts
	import { beforeNavigate } from '$app/navigation';
	import { isDirty, type EditorFields } from '$lib/editor/dirty.js';
```

- [ ] **Step 2: Track a baseline and a `dirty` flag**

After the existing local state declarations (`let source = ...; let saving = $state(false);` etc.), add a baseline that resets when the loaded translation changes, plus a derived dirty flag. Place this near the other `$derived`/`$state` declarations:

```ts
	let baseline = $state<EditorFields>({
		source: data.translation.source,
		title: data.translation.title,
		description: data.translation.description
	});

	// Re-baseline when navigating to a different post/language pair.
	let baselinedPairKey: string | null = null;
	$effect(() => {
		const pairKey = `${post.slug}::${language}`;
		if (pairKey === baselinedPairKey) return;
		baselinedPairKey = pairKey;
		baseline = {
			source: data.translation.source,
			title: data.translation.title,
			description: data.translation.description
		};
	});

	const dirty = $derived(!saving && isDirty(baseline, { source, title, description }));
```

- [ ] **Step 3: Reset the baseline on successful save**

In the existing `saveSubmit` handler, after a successful result, set the baseline to the values that were just saved. Update the success branch:

```ts
	const saveSubmit: SubmitFunction = () => {
		saving = true;
		return async ({ result, update }) => {
			saving = false;
			if (result.type === 'success' && result.data && typeof result.data.revision === 'number') {
				revision = result.data.revision;
				baseline = { source, title, description };
			}
			await update({ invalidateAll: false });
		};
	};
```

- [ ] **Step 4: Guard in-app navigation and browser unload**

Add near the other `$effect` blocks:

```ts
	beforeNavigate((navigation) => {
		if (!dirty) return;
		const leave = confirm('You have unsaved changes. Leave without saving?');
		if (!leave) navigation.cancel();
	});

	$effect(() => {
		const handler = (event: BeforeUnloadEvent) => {
			if (!dirty) return;
			event.preventDefault();
			event.returnValue = '';
		};
		window.addEventListener('beforeunload', handler);
		return () => window.removeEventListener('beforeunload', handler);
	});
```

- [ ] **Step 5: Type/lint check**

```bash
cd web-admin && bun run check && bun run lint
```

Expected: no errors. (If `bun run lint` reports formatting, run `bun run format` then re-run `bun run lint`.)

- [ ] **Step 6: Manual browser verification**

```bash
cd web-admin && bun run dev
```

In a write view:
- With no edits, clicking `Back` navigates away with no prompt; refreshing the tab does not prompt.
- After editing the source/title/description, clicking `Back` prompts to confirm; cancelling stays on the page, confirming leaves.
- After editing, refreshing/closing the tab shows the browser's native "leave site?" dialog.
- Saving (Cmd/Ctrl-S or the Save button), then navigating away — no prompt (baseline reset).

Stop the server.

- [ ] **Step 7: Commit**

```bash
cd web-admin && git add src/routes/posts/\[slug\]/write/\[language\]/+page.svelte && git commit -m "feat(web-admin): warn on leaving the write view with unsaved changes"
```

---

## Task 6: Full verification sweep

Final gate across the whole change set.

**Files:** none (verification only)

- [ ] **Step 1: Run the full checks**

```bash
cd web-admin && bun run test && bun run lint && bun run check && bun run build
```

Expected: all four pass. `bun run build` re-confirms the wasm production build. If `lint` flags formatting, `bun run format`, commit the formatting, and re-run.

- [ ] **Step 2: Sanity-check the production bundle for the wasm split**

Confirm the grammar landed in an async chunk (not the main entry) — the dynamic import should have code-split it:

```bash
cd web-admin && find build -name '*.wasm' -exec ls -lh {} \;
```

Expected: the ~313 KB `typst_syntax*.wasm` is present under `build/` as an emitted asset.

- [ ] **Step 3: Final commit if any formatting/lint fixes were made**

```bash
cd web-admin && git add -A && git commit -m "chore(web-admin): formatting after editor upgrade" || echo "nothing to commit"
```

---

## Self-review notes

- **Spec coverage:** deps + wasm wiring (Task 1); highlighting via grammar + themed HighlightStyle (Task 3); auto-close incl. `$`/backtick + bracket matching + line numbers/active line + search + comment toggle + smart indent (Task 3); Enter-expands-brackets fallback (Task 4, conditional per spec); leave guard on nav + unload with baseline reset (Tasks 2 + 5); deletion of the tokenizer + tests (Task 3); testing + build verification (Tasks 2–6). All spec sections map to a task.
- **Line-comment gap** (grammar only ships block tokens) is closed by the `commentTokens.line: '//'` language data in Task 3 Step 2.
- **`$…$` auto-close** is closed by the `closeBrackets.brackets` language data in Task 3 Step 2.
- **SSR safety** enforced by the dynamic `import('codemirror-lang-typst')` in Task 3 Step 2 and the "do not add to `ssr.noExternal`" constraint.
- **Type consistency:** `EditorFields`/`isDirty` (Task 2) are consumed verbatim in Task 5; `isInsidePair`/`expandBracketOnEnter` (Task 4) match their binding site.
