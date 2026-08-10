import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { GET as robots } from './robots.txt/+server.js';
import { GET as sitemap } from './sitemap.xml/+server.js';
import { PAGE_SIZE } from '$lib/api/types.js';

function summary(n: number, languages: string[] = ['en']) {
	return {
		id: `id-${n}`,
		slug: `post-${n}`,
		languages,
		default_language: 'en',
		tags: [],
		published_at: null,
		title: `Post ${n}`,
		description: ''
	};
}

beforeEach(() => {
	process.env.SNOWBLOG_API_URL = 'http://api.test';
});

afterEach(() => {
	delete process.env.SNOWBLOG_API_URL;
});

describe('robots.txt', () => {
	it('allows everything and points at the sitemap', async () => {
		const response = await robots({ url: new URL('https://blog.test/robots.txt') } as never);
		expect(response.headers.get('content-type')).toContain('text/plain');
		const body = await response.text();
		expect(body).toContain('User-agent: *');
		expect(body).toContain('Allow: /');
		expect(body).toContain('Sitemap: https://blog.test/sitemap.xml');
	});
});

describe('sitemap.xml', () => {
	it('walks every list page and emits one url per language', async () => {
		const pageOne = Array.from({ length: PAGE_SIZE + 1 }, (_, i) => summary(i));
		const pageTwo = [summary(PAGE_SIZE, ['en', 'zh'])];
		const fetchFn = ((input: unknown) => {
			const url = new URL(String(input));
			const offset = Number(url.searchParams.get('offset'));
			return Promise.resolve(Response.json({ posts: offset === 0 ? pageOne : pageTwo }));
		}) as typeof fetch;

		const response = await sitemap({
			url: new URL('https://blog.test/sitemap.xml'),
			fetch: fetchFn
		} as never);
		expect(response.headers.get('content-type')).toContain('application/xml');
		const body = await response.text();
		expect(body.match(/<url>/g)).toHaveLength(PAGE_SIZE + 2);
		expect(body).toContain('<loc>https://blog.test/posts/post-0</loc>');
		expect(body).toContain(`<loc>https://blog.test/posts/post-${PAGE_SIZE}?lang=zh</loc>`);
	});
});
