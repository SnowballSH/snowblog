import { describe, expect, it, beforeEach, afterEach } from 'vitest';
import { load } from './+page.server.js';
import { PAGE_SIZE, type PostListPage } from '$lib/api/types.js';

function summaries(count: number) {
	return Array.from({ length: count }, (_, i) => ({
		id: `id-${i}`,
		slug: `post-${i}`,
		languages: ['en'],
		default_language: 'en',
		tags: [],
		published_at: null,
		title: `Post ${i}`,
		description: ''
	}));
}

function event(query: string, posts: unknown[]) {
	return {
		url: new URL(`http://web.test/${query}`),
		fetch: (() => Promise.resolve(Response.json({ posts }))) as typeof fetch
	} as never;
}

function run(query: string, posts: unknown[]): Promise<PostListPage> {
	return load(event(query, posts)) as Promise<PostListPage>;
}

beforeEach(() => {
	process.env.SNOWBLOG_API_URL = 'http://api.test';
});

afterEach(() => {
	delete process.env.SNOWBLOG_API_URL;
});

describe('home load', () => {
	it('serves page one by default', async () => {
		const data = await run('', summaries(3));
		expect(data.page).toBe(1);
		expect(data.posts).toHaveLength(3);
		expect(data.hasNext).toBe(false);
	});

	it('parses the page query and reports a next page', async () => {
		const data = await run('?page=2', summaries(PAGE_SIZE + 1));
		expect(data.page).toBe(2);
		expect(data.posts).toHaveLength(PAGE_SIZE);
		expect(data.hasNext).toBe(true);
	});

	it('rejects a malformed page as not found', async () => {
		await expect(load(event('?page=abc', []))).rejects.toMatchObject({ status: 404 });
	});

	it('rethrows upstream failures with their status', async () => {
		const failing = {
			url: new URL('http://web.test/'),
			fetch: (() => Promise.reject(new Error('down'))) as typeof fetch
		} as never;
		await expect(load(failing)).rejects.toMatchObject({ status: 502 });
	});
});
