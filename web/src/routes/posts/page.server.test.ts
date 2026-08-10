import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { load } from './[slug]/+page.server.js';
import type { PostDetail } from '$lib/api/types.js';

const detail: PostDetail = {
	id: 'id-1',
	slug: 'my-post',
	languages: ['en', 'zh'],
	default_language: 'en',
	tags: ['math'],
	published_at: '2026-08-01T00:00:00Z',
	title: 'My Post',
	description: 'A post.',
	language: 'en',
	html: '<p>hello</p>',
	rendered_with: { renderer_version: '0.15.1', rendered_at: '2026-08-02T00:00:00Z' }
};

interface LoadedPost {
	post: PostDetail;
	canonical: string;
	alternates: { lang: string; href: string }[];
}

function event(query: string, response: () => Response | Promise<Response>) {
	return {
		params: { slug: 'my-post' },
		url: new URL(`http://web.test/posts/my-post${query}`),
		fetch: (() => Promise.resolve(response())) as typeof fetch
	} as never;
}

function run(query: string, response: () => Response | Promise<Response>): Promise<LoadedPost> {
	return load(event(query, response)) as Promise<LoadedPost>;
}

beforeEach(() => {
	process.env.SNOWBLOG_API_URL = 'http://api.test';
});

afterEach(() => {
	delete process.env.SNOWBLOG_API_URL;
});

describe('post load', () => {
	it('loads the default translation and derives head data', async () => {
		const data = await run('', () => Response.json(detail));
		expect(data.post.html).toBe('<p>hello</p>');
		expect(data.canonical).toBe('http://web.test/posts/my-post');
		expect(data.alternates).toHaveLength(2);
	});

	it('requests the translation named by the lang query', async () => {
		const data = await run('?lang=zh', () =>
			Response.json({ ...detail, language: 'zh', title: '我的文章' })
		);
		expect(data.post.language).toBe('zh');
	});

	it('redirects an unknown lang to the bare URL', async () => {
		await expect(load(event('?lang=fr', () => Response.json(detail)))).rejects.toMatchObject({
			status: 302,
			location: '/posts/my-post'
		});
	});

	it('passes an upstream 404 through', async () => {
		await expect(
			load(event('', () => new Response('nope', { status: 404 })))
		).rejects.toMatchObject({ status: 404 });
	});
});
