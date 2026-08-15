<script lang="ts">
	import { untrack } from 'svelte';
	import { EditorState, type Extension } from '@codemirror/state';
	import { EditorView, keymap, placeholder as placeholderExtension } from '@codemirror/view';
	import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
	import { HighlightStyle, StreamLanguage, syntaxHighlighting } from '@codemirror/language';
	import { tags } from '@lezer/highlight';
	import { typstStreamParser } from './typst-mode.js';

	let { value = $bindable(''), placeholder = '' }: { value?: string; placeholder?: string } =
		$props();

	let host: HTMLDivElement | undefined;
	let view: EditorView | undefined = $state();

	const highlightStyle = HighlightStyle.define([
		{ tag: tags.comment, color: 'var(--fui-ink-muted)', fontStyle: 'italic' },
		{ tag: tags.string, color: 'var(--fui-code-green)' },
		{ tag: tags.escape, color: 'var(--fui-code-green)' },
		{ tag: tags.meta, color: 'var(--fui-code-violet)' },
		{ tag: tags.atom, color: 'var(--fui-code-blue)' },
		{ tag: tags.keyword, color: 'var(--fui-accent-strong)', fontWeight: '600' },
		{ tag: tags.heading, color: 'var(--fui-ink)', fontWeight: '700' },
		{ tag: tags.strong, fontWeight: '700' },
		{ tag: tags.emphasis, fontStyle: 'italic' },
		{ tag: tags.labelName, color: 'var(--fui-code-warm)' },
		{ tag: tags.link, color: 'var(--fui-code-warm)', textDecoration: 'underline' }
	]);

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
		'.cm-placeholder': { color: 'var(--fui-ink-muted)' }
	});

	function extensions(): Extension[] {
		return [
			StreamLanguage.define(typstStreamParser),
			syntaxHighlighting(highlightStyle),
			history(),
			keymap.of([...defaultKeymap, ...historyKeymap, indentWithTab]),
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
		untrack(() => {
			view = new EditorView({
				state: EditorState.create({ doc: value, extensions: extensions() }),
				parent: mountPoint
			});
		});
		return () => {
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
