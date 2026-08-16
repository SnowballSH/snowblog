import { describe, expect, it } from 'vitest';
import { isInsidePair } from './smart-indent.js';

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
