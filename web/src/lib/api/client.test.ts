import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { ApiError, apiBase, fetchPost, fetchPostList } from './client.js';
import { PAGE_SIZE, type PostSummary } from './types.js';

function summary(n: number): PostSummary {
	return {
		id: `id-${n}`,
		slug: `post-${n}`,
		languages: ['en'],
		default_language: 'en',
		tags: ['math'],
		published_at: '2026-08-01T00:00:00Z',
		title: `Post ${n}`,
		description: `Description ${n}`
	};
}

function stubFetch(handler: (url: URL) => Response): typeof fetch {
	return (input) => Promise.resolve(handler(new URL(String(input))));
}

beforeEach(() => {
	process.env.SNOWBLOG_API_URL = 'http://api.test';
});

afterEach(() => {
	delete process.env.SNOWBLOG_API_URL;
});

describe('apiBase', () => {
	it('returns the configured base without a trailing slash', () => {
		process.env.SNOWBLOG_API_URL = 'http://api.test/';
		expect(apiBase()).toBe('http://api.test');
	});

	it('throws when unconfigured', () => {
		delete process.env.SNOWBLOG_API_URL;
		expect(() => apiBase()).toThrow(/SNOWBLOG_API_URL/);
	});
});

describe('fetchPostList', () => {
	it('requests one past the page size and maps the fields', async () => {
		let requested: URL | undefined;
		const posts = [summary(1), summary(2)];
		const page = await fetchPostList(
			stubFetch((url) => {
				requested = url;
				return Response.json({ posts });
			}),
			1
		);
		expect(requested?.pathname).toBe('/api/v1/posts');
		expect(requested?.searchParams.get('limit')).toBe(String(PAGE_SIZE + 1));
		expect(requested?.searchParams.get('offset')).toBe('0');
		expect(page).toEqual({ posts, page: 1, hasNext: false });
	});

	it('derives hasNext from the probe row and trims it', async () => {
		const posts = Array.from({ length: PAGE_SIZE + 1 }, (_, i) => summary(i));
		const page = await fetchPostList(
			stubFetch(() => Response.json({ posts })),
			1
		);
		expect(page.posts).toHaveLength(PAGE_SIZE);
		expect(page.hasNext).toBe(true);
	});

	it('keeps a full page without a probe row as the last page', async () => {
		const posts = Array.from({ length: PAGE_SIZE }, (_, i) => summary(i));
		const page = await fetchPostList(
			stubFetch(() => Response.json({ posts })),
			1
		);
		expect(page.posts).toHaveLength(PAGE_SIZE);
		expect(page.hasNext).toBe(false);
	});

	it('offsets by whole pages', async () => {
		let requested: URL | undefined;
		await fetchPostList(
			stubFetch((url) => {
				requested = url;
				return Response.json({ posts: [] });
			}),
			3
		);
		expect(requested?.searchParams.get('offset')).toBe(String(2 * PAGE_SIZE));
	});

	it('rejects page numbers below one as not found', async () => {
		await expect(
			fetchPostList(
				stubFetch(() => Response.json({ posts: [] })),
				0
			)
		).rejects.toMatchObject({ status: 404 });
	});
});

describe('fetchPost', () => {
	it('fetches a post by slug', async () => {
		let requested: URL | undefined;
		const detail = {
			...summary(1),
			language: 'en',
			html: '<p>hi</p>',
			rendered_with: { renderer_version: '0.15.1', rendered_at: '2026-08-01T00:00:00Z' }
		};
		const post = await fetchPost(
			stubFetch((url) => {
				requested = url;
				return Response.json(detail);
			}),
			'post-1'
		);
		expect(requested?.pathname).toBe('/api/v1/posts/post-1');
		expect(requested?.searchParams.has('language')).toBe(false);
		expect(post).toEqual(detail);
	});

	it('passes the requested language through', async () => {
		let requested: URL | undefined;
		await fetchPost(
			stubFetch((url) => {
				requested = url;
				return Response.json({});
			}),
			'post-1',
			'zh'
		);
		expect(requested?.searchParams.get('language')).toBe('zh');
	});

	it('maps upstream statuses onto ApiError', async () => {
		await expect(
			fetchPost(
				stubFetch(() => new Response('nope', { status: 404 })),
				'missing'
			)
		).rejects.toMatchObject({ status: 404 });
	});

	it('maps network failure onto a 502 ApiError', async () => {
		const failing: typeof fetch = () => Promise.reject(new Error('conn refused'));
		const attempt = fetchPost(failing, 'post-1');
		await expect(attempt).rejects.toBeInstanceOf(ApiError);
		await expect(attempt).rejects.toMatchObject({ status: 502 });
	});
});
