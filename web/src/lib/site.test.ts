import { afterEach, describe, expect, it } from 'vitest';
import { site } from './site.js';

afterEach(() => {
	delete process.env.PUBLIC_SITE_NAME;
	delete process.env.PUBLIC_SITE_DESCRIPTION;
	delete process.env.PUBLIC_FOOTER_TEXT;
});

describe('site', () => {
	it('falls back to generic defaults', () => {
		expect(site()).toEqual({
			name: 'SnowBlog',
			description: 'A blog written in Typst and rendered to fast, clean pages.',
			footerText: 'SnowBlog'
		});
	});

	it('honors configured values', () => {
		process.env.PUBLIC_SITE_NAME = 'Snowy Notes';
		process.env.PUBLIC_SITE_DESCRIPTION = 'Notes from the snow.';
		process.env.PUBLIC_FOOTER_TEXT = '© Snowy';
		expect(site()).toEqual({
			name: 'Snowy Notes',
			description: 'Notes from the snow.',
			footerText: '© Snowy'
		});
	});

	it('falls back footer text to the site name', () => {
		process.env.PUBLIC_SITE_NAME = 'Snowy Notes';
		expect(site().footerText).toBe('Snowy Notes');
	});
});
