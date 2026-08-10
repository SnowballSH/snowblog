import { describe, expect, it } from 'vitest';
import { postAlternates } from './head.js';

describe('postAlternates', () => {
	it('gives the default language the bare URL and others a lang query', () => {
		const { canonical, alternates } = postAlternates(
			'https://blog.test',
			'my-post',
			['en', 'zh'],
			'en'
		);
		expect(canonical).toBe('https://blog.test/posts/my-post');
		expect(alternates).toEqual([
			{ lang: 'en', href: 'https://blog.test/posts/my-post' },
			{ lang: 'zh', href: 'https://blog.test/posts/my-post?lang=zh' }
		]);
	});

	it('emits a single alternate for a monolingual post', () => {
		const { alternates } = postAlternates('https://blog.test', 'solo', ['en'], 'en');
		expect(alternates).toEqual([{ lang: 'en', href: 'https://blog.test/posts/solo' }]);
	});
});
