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
