import { afterEach, describe, expect, it } from 'vitest';
import { site } from './site.js';

afterEach(() => {
	delete process.env.PUBLIC_SITE_NAME;
	delete process.env.PUBLIC_SITE_AUTHOR;
	delete process.env.PUBLIC_SITE_DESCRIPTION;
	delete process.env.PUBLIC_FOOTER_TEXT;
});

describe('site', () => {
	it('falls back to generic defaults', () => {
		expect(site()).toEqual({
			name: 'Blogs',
			author: 'SnowballSH',
			brand: 'SnowballSH Blogs',
			description: 'Personal and Academic Blogs. Maybe Philosophical. Maybe Mathematical. Or both.',
			footerText: 'SnowballSH Blogs'
		});
	});

	it('honors configured values', () => {
		process.env.PUBLIC_SITE_NAME = 'Snowy Notes';
		process.env.PUBLIC_SITE_AUTHOR = '';
		process.env.PUBLIC_SITE_DESCRIPTION = 'Notes from the snow.';
		process.env.PUBLIC_FOOTER_TEXT = '© Snowy';
		expect(site()).toEqual({
			name: 'Snowy Notes',
			author: '',
			brand: 'Snowy Notes',
			description: 'Notes from the snow.',
			footerText: '© Snowy'
		});
	});

	it('falls back footer text to the brand', () => {
		process.env.PUBLIC_SITE_NAME = 'Snowy Notes';
		expect(site().footerText).toBe('SnowballSH Snowy Notes');
	});
});
