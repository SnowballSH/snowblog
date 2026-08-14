import { describe, expect, it, vi } from 'vitest';
import { isRedirect } from '@sveltejs/kit';
import type { AdminPost } from '$lib/api/types.js';

interface CreateOutcome {
	ok: boolean;
	auditId: string;
	message: string;
	conflict?: boolean;
}

const { listPosts, createPost } = vi.hoisted(() => ({
	listPosts: vi.fn(),
	createPost: vi.fn()
}));

vi.mock('$lib/server/client.js', async (importOriginal) => {
	const actual = await importOriginal<typeof import('$lib/server/client.js')>();
	class FakeAdminApi {
		listPosts = listPosts;
		createPost = createPost;
	}
	return {
		...actual,
		AdminApi: FakeAdminApi
	};
});

vi.mock('$lib/server/config.js', () => ({
	getConfig: () => ({
		apiUrl: 'http://api.test',
		adminToken: 'tok',
		identityHeader: 'remote-user',
		allowedUsers: new Set(['a']),
		origin: 'https://o.test',
		metricsListen: null
	})
}));

const recordAudit = vi.hoisted(() => vi.fn(() => 'audit-1'));
vi.mock('$lib/server/audit.js', () => ({ recordAudit }));

const { ApiError } = await import('$lib/server/client.js');
const { load, actions } = await import('./+page.server.js');

function post(overrides: Partial<Record<string, unknown>> = {}) {
	return {
		id: overrides.id ?? 'id',
		slug: overrides.slug ?? 'slug',
		status: 'draft',
		default_language: 'en',
		revision: 1,
		tags: [],
		published_at: null,
		created_at: '2026-01-01T00:00:00Z',
		updated_at: overrides.updated_at ?? '2026-01-01T00:00:00Z',
		translations: [],
		renders: [],
		assets: [],
		freshness: [],
		...overrides
	};
}

function formRequest(fields: Record<string, string>): Request {
	const formData = new FormData();
	for (const [key, value] of Object.entries(fields)) formData.set(key, value);
	return { formData: async () => formData } as unknown as Request;
}

describe('load', () => {
	it('sorts posts by updated_at descending', async () => {
		listPosts.mockResolvedValue([
			post({ id: 'a', updated_at: '2026-01-01T00:00:00Z' }),
			post({ id: 'b', updated_at: '2026-03-01T00:00:00Z' }),
			post({ id: 'c', updated_at: '2026-02-01T00:00:00Z' })
		]);

		const result = (await load({ fetch } as never)) as { posts: AdminPost[] };

		expect(result.posts.map((p) => p.id)).toEqual(['b', 'c', 'a']);
	});
});

describe('actions.create', () => {
	it('calls createPost, records an ok audit, and redirects to the new post', async () => {
		createPost.mockResolvedValue(post({ slug: 'new-post' }));

		const event = {
			request: formRequest({ slug: 'new-post', default_language: 'en' }),
			locals: { user: 'alice' },
			fetch
		} as never;

		const redirected = await Promise.resolve(actions.create(event)).catch((e: unknown) => e);

		expect(isRedirect(redirected)).toBe(true);
		expect((redirected as { status: number }).status).toBe(303);
		expect((redirected as { location: string }).location).toBe('/posts/new-post');
		expect(createPost).toHaveBeenCalledWith({
			slug: 'new-post',
			default_language: 'en',
			tags: undefined
		});
		expect(recordAudit).toHaveBeenCalledWith(
			expect.objectContaining({
				user: 'alice',
				action: 'create',
				slug: 'new-post',
				outcome: 'ok'
			})
		);
	});

	it('splits a comma-separated tags field', async () => {
		createPost.mockResolvedValue(post({ slug: 'tagged' }));

		const event = {
			request: formRequest({ slug: 'tagged', default_language: 'zh', tags: 'a, b ,c' }),
			locals: { user: 'alice' },
			fetch
		} as never;

		await Promise.resolve(actions.create(event)).catch((e: unknown) => e);

		expect(createPost).toHaveBeenCalledWith({
			slug: 'tagged',
			default_language: 'zh',
			tags: ['a', 'b', 'c']
		});
	});

	it('maps a 422 ApiError to fail(422) with the problem detail and an auditId', async () => {
		createPost.mockRejectedValue(
			new ApiError({ status: 422, title: 'Unprocessable', detail: 'slug is invalid' }, false)
		);

		const event = {
			request: formRequest({ slug: 'bad slug', default_language: 'en' }),
			locals: { user: 'alice' },
			fetch
		} as never;

		const result = (await actions.create(event)) as { status: number; data: CreateOutcome };

		expect(result.status).toBe(422);
		expect(result.data).toEqual({
			ok: false,
			auditId: 'audit-1',
			message: 'slug is invalid'
		});
	});

	it('maps a 412 ApiError to fail(409) with conflict: true', async () => {
		createPost.mockRejectedValue(new ApiError({ status: 412, title: 'Precondition Failed' }, true));

		const event = {
			request: formRequest({ slug: 'clashing', default_language: 'en' }),
			locals: { user: 'alice' },
			fetch
		} as never;

		const result = (await actions.create(event)) as { status: number; data: CreateOutcome };

		expect(result.status).toBe(409);
		expect(result.data).toMatchObject({ ok: false, conflict: true, auditId: 'audit-1' });
	});
});
