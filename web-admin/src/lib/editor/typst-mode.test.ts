import { describe, expect, it } from 'vitest';
import { StringStream } from '@codemirror/language';
import { typstStreamParser, type TypstState } from './typst-mode.js';

interface Token {
	text: string;
	type: string | null;
}

function readToken(stream: StringStream, state: TypstState): string | null {
	stream.start = stream.pos;
	for (let i = 0; i < 10; i++) {
		const result = typstStreamParser.token(stream, state);
		if (stream.pos > stream.start) return result;
	}
	throw new Error('typst stream parser failed to advance');
}

function tokenize(source: string): Token[] {
	const state = typstStreamParser.startState!(2);
	const tokens: Token[] = [];
	for (const line of source.split('\n')) {
		const stream = new StringStream(line, 2, 2);
		while (!stream.eol()) {
			const type = readToken(stream, state);
			tokens.push({ text: stream.current(), type });
		}
	}
	return tokens;
}

function typesOf(source: string): (string | null)[] {
	return tokenize(source).map((token) => token.type);
}

describe('typst-mode tokenizer', () => {
	it('tokenizes a line comment', () => {
		const tokens = tokenize('// a note');
		expect(tokens).toContainEqual({ text: '// a note', type: 'comment' });
	});

	it('tokenizes a single-line block comment as one comment span', () => {
		const tokens = tokenize('/* inline */ #rest');
		expect(tokens[0]).toEqual({ text: '/* inline */', type: 'comment' });
	});

	it('carries an unterminated block comment across lines', () => {
		const tokens = tokenize('/* start\nstill inside\nend */ text');
		expect(tokens.some((token) => token.type === 'comment' && token.text.includes('start'))).toBe(
			true
		);
		expect(
			tokens.some((token) => token.type === 'comment' && token.text.includes('still inside'))
		).toBe(true);
		expect(tokens.some((token) => token.type === 'comment' && token.text.includes('end */'))).toBe(
			true
		);
		expect(tokens.some((token) => token.type === null)).toBe(true);
	});

	it('tokenizes a double-quoted string', () => {
		const tokens = tokenize('"hello world"');
		expect(tokens).toContainEqual({ text: '"hello world"', type: 'string' });
	});

	it('tokenizes a string with an escaped quote', () => {
		const tokens = tokenize('"a \\"b\\" c"');
		expect(tokens).toContainEqual({ text: '"a \\"b\\" c"', type: 'string' });
	});

	it('tokenizes math delimiters distinctly from math content', () => {
		const tokens = tokenize('$x + 1$');
		expect(tokens[0]).toEqual({ text: '$', type: 'meta' });
		expect(tokens[tokens.length - 1]).toEqual({ text: '$', type: 'meta' });
		expect(tokens.slice(1, -1).every((token) => token.type === 'atom')).toBe(true);
	});

	it('tokenizes an escape sequence inside math', () => {
		const types = typesOf('$a \\, b$');
		expect(types).toContain('escape');
	});

	it('tokenizes a heading marker at the start of a line', () => {
		const tokens = tokenize('= Title');
		expect(tokens[0]).toEqual({ text: '=', type: 'heading' });
	});

	it('tokenizes a multi-level heading marker', () => {
		const tokens = tokenize('=== Subheading');
		expect(tokens[0]).toEqual({ text: '===', type: 'heading' });
	});

	it('does not treat an indented equals sign as a heading', () => {
		const tokens = tokenize('  = not a heading');
		expect(tokens.some((token) => token.type === 'heading')).toBe(false);
	});

	it('tokenizes a hash function call name', () => {
		const tokens = tokenize('#image("a.png")');
		expect(tokens[0]).toEqual({ text: '#image', type: 'keyword' });
	});

	it('tokenizes a bare hash call with no arguments', () => {
		const tokens = tokenize('#pagebreak');
		expect(tokens).toContainEqual({ text: '#pagebreak', type: 'keyword' });
	});

	it('tokenizes bold markers', () => {
		const tokens = tokenize('*bold text*');
		expect(tokens).toContainEqual({ text: '*bold text*', type: 'strong' });
	});

	it('tokenizes emphasis markers', () => {
		const tokens = tokenize('_em text_');
		expect(tokens).toContainEqual({ text: '_em text_', type: 'emphasis' });
	});

	it('tokenizes a label', () => {
		const tokens = tokenize('<fig-one>');
		expect(tokens).toContainEqual({ text: '<fig-one>', type: 'labelName' });
	});

	it('tokenizes a reference', () => {
		const tokens = tokenize('see @fig-one for details');
		expect(tokens).toContainEqual({ text: '@fig-one', type: 'link' });
	});

	it('tokenizes an escape sequence outside math', () => {
		const tokens = tokenize('a \\@ b');
		expect(tokens).toContainEqual({ text: '\\@', type: 'escape' });
	});

	it('always advances the stream so the tokenizer cannot hang', () => {
		expect(() => tokenize('#()[]{}$$**__<<@@\\\\""//')).not.toThrow();
	});
});
