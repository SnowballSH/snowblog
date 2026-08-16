import { EditorSelection, type Extension } from '@codemirror/state';
import type { EditorView } from '@codemirror/view';
import { getIndentUnit, indentService } from '@codemirror/language';

const PAIRS: Record<string, string> = { '(': ')', '[': ']', '{': '}' };

export function isInsidePair(before: string, after: string): boolean {
	return before in PAIRS && PAIRS[before] === after;
}

// Enter between a matching bracket pair → explode into an indented block with the
// closer on its own line (VSCode behavior). Returns false (falling through to the
// default newline) whenever the caret is not an empty selection between a pair.
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

	const unit = ' '.repeat(getIndentUnit(state));
	const changes = state.changeByRange((range) => {
		const line = state.doc.lineAt(range.from);
		const baseIndent = /^\s*/.exec(line.text)?.[0] ?? '';
		const inner = baseIndent + unit;
		const insert = `\n${inner}\n${baseIndent}`;
		return {
			changes: { from: range.from, insert },
			range: EditorSelection.cursor(range.from + 1 + inner.length)
		};
	});
	view.dispatch(state.update(changes, { scrollIntoView: true, userEvent: 'input' }));
	return true;
};

// General auto-indent: the Typst grammar ships no indentation service, so a new line
// would otherwise start at column 0. Copy the previous line's indent, plus one unit
// when that line ends with an open bracket.
export const typstIndentService: Extension = indentService.of((context, pos) => {
	const line = context.lineAt(pos, -1);
	const leading = /^\s*/.exec(line.text)?.[0]?.length ?? 0;
	const trimmed = line.text.replace(/\s+$/, '');
	return /[([{]$/.test(trimmed) ? leading + getIndentUnit(context.state) : leading;
});
