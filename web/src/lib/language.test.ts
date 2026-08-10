import { describe, expect, it } from 'vitest';
import { languageLabel } from './language.js';

describe('languageLabel', () => {
	it('uses the curated short label when one exists', () => {
		expect(languageLabel('zh')).toBe('中');
		expect(languageLabel('en')).toBe('EN');
	});

	it('falls back to the language autonym', () => {
		expect(languageLabel('ja')).toBe('日本語');
		expect(languageLabel('fr')).toBe('français');
	});

	it('falls back to the raw tag when nothing resolves', () => {
		expect(languageLabel('x-unknown')).toBe('x-unknown');
	});
});
