import { StreamLanguage, type StreamParser } from '@codemirror/language';

export interface TypstState {
	inBlockComment: boolean;
	inMath: boolean;
}

const HASH_CALL = /^#[A-Za-z_][\w-]*/;
const LABEL = /^<[A-Za-z_][\w-]*>/;
const REFERENCE = /^@[A-Za-z_][\w-]*/;
const HEADING = /^=+/;
const ESCAPE = /^\\./;
const BOLD = /^\*[^*\n]+\*/;
const EMPHASIS = /^_[^_\n]+_/;
const BLOCK_COMMENT_END = /^[^*]*\*\//;
const MATH_RUN = /^[^$\\]+/;

export const typstStreamParser: StreamParser<TypstState> = {
	name: 'typst',

	startState(): TypstState {
		return { inBlockComment: false, inMath: false };
	},

	token(stream, state) {
		if (state.inBlockComment) {
			if (stream.match(BLOCK_COMMENT_END)) state.inBlockComment = false;
			else stream.skipToEnd();
			return 'comment';
		}

		if (state.inMath) {
			if (stream.match('$')) {
				state.inMath = false;
				return 'meta';
			}
			if (stream.match(ESCAPE)) return 'escape';
			if (!stream.match(MATH_RUN)) stream.next();
			return 'atom';
		}

		if (stream.sol() && stream.match(HEADING)) return 'heading';
		if (stream.eatSpace()) return null;

		if (stream.match('//')) {
			stream.skipToEnd();
			return 'comment';
		}
		if (stream.match('/*')) {
			if (!stream.match(BLOCK_COMMENT_END)) state.inBlockComment = true;
			return 'comment';
		}
		if (stream.match('$')) {
			state.inMath = true;
			return 'meta';
		}
		if (stream.match(ESCAPE)) return 'escape';
		if (stream.match('"')) {
			while (!stream.eol()) {
				const ch = stream.next();
				if (ch === '\\') stream.next();
				else if (ch === '"') break;
			}
			return 'string';
		}
		if (stream.match(HASH_CALL)) return 'keyword';
		if (stream.match(LABEL)) return 'labelName';
		if (stream.match(REFERENCE)) return 'link';
		if (stream.match(BOLD)) return 'strong';
		if (stream.match(EMPHASIS)) return 'emphasis';

		stream.next();
		return null;
	}
};

export const typstLanguage = StreamLanguage.define(typstStreamParser);
