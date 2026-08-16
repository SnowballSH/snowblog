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
