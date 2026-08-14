import { describe, expect, it, vi } from 'vitest';
import { AdminApi, ApiError } from './client.js';

const config = {
	apiUrl: 'http://api.test',
	adminToken: 'tok',
	identityHeader: 'remote-user',
	allowedUsers: new Set(['a']),
	origin: 'https://o.test',
	metricsListen: null
};

function jsonResponse(status: number, body: unknown): Response {
	return new Response(JSON.stringify(body), {
		status,
		headers: { 'content-type': 'application/json' }
	});
}

function htmlResponse(status: number, body: string): Response {
	return new Response(body, {
		status,
		headers: { 'content-type': 'text/html' }
	});
}

describe('AdminApi', () => {
	it('sends the bearer token and quoted If-Match on mutations', async () => {
		const fetchFn = vi.fn().mockResolvedValue(jsonResponse(200, { slug: 's', revision: 4 }));
		const api = new AdminApi(config, fetchFn);
		await api.publish('s', 3);
		const [url, init] = fetchFn.mock.calls[0];
		expect(url).toBe('http://api.test/api/v1/admin/posts/s/publish');
		expect(init.headers.get('authorization')).toBe('Bearer tok');
		expect(init.headers.get('if-match')).toBe('"3"');
	});
	it('maps 412 to a conflict ApiError', async () => {
		const fetchFn = vi
			.fn()
			.mockResolvedValue(jsonResponse(412, { status: 412, detail: 'revision mismatch' }));
		const api = new AdminApi(config, fetchFn);
		const error = await api.publish('s', 3).catch((e) => e);
		expect(error).toBeInstanceOf(ApiError);
		expect(error.conflict).toBe(true);
		expect(error.problem.detail).toBe('revision mismatch');
	});
	it('synthesizes a Problem from a non-JSON error body', async () => {
		const fetchFn = vi
			.fn()
			.mockResolvedValue(htmlResponse(502, '<html><body>Bad Gateway</body></html>'));
		const api = new AdminApi(config, fetchFn);
		const error = await api.publish('s', 3).catch((e) => e);
		expect(error).toBeInstanceOf(ApiError);
		expect(error.problem.status).toBe(502);
		expect(error.conflict).toBe(false);
	});
	it('unwraps the posts envelope from listPosts', async () => {
		const fetchFn = vi.fn().mockResolvedValue(jsonResponse(200, { posts: [{ slug: 'x' }] }));
		const api = new AdminApi(config, fetchFn);
		expect(await api.listPosts()).toEqual([{ slug: 'x' }]);
	});
	it('unwraps the save envelope from putTranslation', async () => {
		const fetchFn = vi
			.fn()
			.mockResolvedValue(jsonResponse(200, { post: { slug: 'x', revision: 5 }, render: {} }));
		const api = new AdminApi(config, fetchFn);
		const post = await api.putTranslation('x', 4, 'en', {
			title: 't',
			description: '',
			source: 'b'
		});
		expect(post.revision).toBe(5);
	});
	it('returns preview failures as a value, not an exception', async () => {
		const fetchFn = vi
			.fn()
			.mockResolvedValue(
				jsonResponse(200, { status: 'failed', diagnostics: [{ message: 'bad' }] })
			);
		const api = new AdminApi(config, fetchFn);
		const result = await api.preview('s', 'src');
		expect(result.status).toBe('failed');
	});
});
