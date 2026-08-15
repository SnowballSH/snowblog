import { beforeEach, describe, expect, it, vi } from 'vitest';
import { isHttpError } from '@sveltejs/kit';
import type { AdminPost } from '$lib/api/types.js';

interface Outcome {
	ok: boolean;
	auditId: string;
	message: string;
	conflict?: boolean;
	revision?: number;
}

const { getPost, putTranslation } = vi.hoisted(() => ({
	getPost: vi.fn(),
	putTranslation: vi.fn()
}));

vi.mock('$lib/server/client.js', async (importOriginal) => {
	const actual = await importOriginal<typeof import('$lib/server/client.js')>();
	class FakeAdminApi {
		getPost = getPost;
		putTranslation = putTranslation;
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

function post(overrides: Partial<AdminPost> = {}): AdminPost {
	return {
		id: 'id',
		slug: 'my-post',
		status: 'draft',
		default_language: 'en',
		revision: 3,
		tags: [],
		published_at: null,
		created_at: '2026-01-01T00:00:00Z',
		updated_at: '2026-01-01T00:00:00Z',
		translations: [
			{
				language: 'en',
				title: 'Title',
				description: 'Desc',
				source: '= hi',
				updated_at: '2026-01-01T00:00:00Z'
			}
		],
		renders: [],
		assets: [],
		freshness: [],
		...overrides
	};
}

function formRequest(fields: Record<string, string> = {}): Request {
	const formData = new FormData();
	for (const [key, value] of Object.entries(fields)) formData.set(key, value);
	return { formData: async () => formData } as unknown as Request;
}

function event(fields: Record<string, string> = {}, language = 'en') {
	return {
		request: formRequest(fields),
		params: { slug: 'my-post', language },
		locals: { user: 'alice' },
		fetch
	} as never;
}

beforeEach(() => {
	vi.clearAllMocks();
	recordAudit.mockReturnValue('audit-1');
});

describe('load', () => {
	it('returns the post and the matching translation', async () => {
		getPost.mockResolvedValue(post());

		const result = (await load({
			params: { slug: 'my-post', language: 'en' },
			fetch
		} as never)) as { post: AdminPost; translation: AdminPost['translations'][number] };

		expect(getPost).toHaveBeenCalledWith('my-post');
		expect(result.translation.language).toBe('en');
	});

	it('404s when the language has no translation', async () => {
		getPost.mockResolvedValue(post());

		const thrown = await Promise.resolve(
			load({ params: { slug: 'my-post', language: 'zh' }, fetch } as never)
		).catch((e: unknown) => e);

		expect(isHttpError(thrown, 404)).toBe(true);
	});

	it('maps a 404 ApiError on the post itself to a SvelteKit error(404)', async () => {
		getPost.mockRejectedValue(new ApiError({ status: 404, title: 'Not Found' }, false));

		const thrown = await Promise.resolve(
			load({ params: { slug: 'missing', language: 'en' }, fetch } as never)
		).catch((e: unknown) => e);

		expect(isHttpError(thrown, 404)).toBe(true);
	});

	it('rethrows a non-404 ApiError', async () => {
		getPost.mockRejectedValue(new ApiError({ status: 500, title: 'Boom' }, false));

		const thrown = await Promise.resolve(
			load({ params: { slug: 'my-post', language: 'en' }, fetch } as never)
		).catch((e: unknown) => e);

		expect(isHttpError(thrown)).toBe(false);
		expect(thrown).toBeInstanceOf(ApiError);
	});
});

describe('actions.save', () => {
	it('threads the form revision into putTranslation and returns the new revision', async () => {
		putTranslation.mockResolvedValue(post({ revision: 4 }));

		const result = (await actions.save(
			event({ revision: '3', title: 'Title', description: 'Desc', source: '= hi' })
		)) as Outcome;

		expect(putTranslation).toHaveBeenCalledWith('my-post', 3, 'en', {
			title: 'Title',
			description: 'Desc',
			source: '= hi'
		});
		expect(typeof putTranslation.mock.calls[0][1]).toBe('number');
		expect(result).toEqual({
			ok: true,
			auditId: 'audit-1',
			message: expect.any(String),
			revision: 4
		});
	});

	it('rejects a non-numeric revision with fail(400) without calling the API', async () => {
		const result = (await actions.save(
			event({ revision: 'x', title: 'Title', description: 'Desc', source: 'src' })
		)) as { status: number; data: Outcome };

		expect(result.status).toBe(400);
		expect(putTranslation).not.toHaveBeenCalled();
		expect(recordAudit).toHaveBeenCalledTimes(1);
	});

	it('maps a 412 ApiError to fail(409) with conflict: true', async () => {
		putTranslation.mockRejectedValue(
			new ApiError({ status: 412, title: 'Precondition Failed' }, true)
		);

		const result = (await actions.save(
			event({ revision: '3', title: 'Title', description: 'Desc', source: '= hi' })
		)) as { status: number; data: Outcome };

		expect(result.status).toBe(409);
		expect(result.data).toMatchObject({ ok: false, conflict: true, auditId: 'audit-1' });
	});
});
