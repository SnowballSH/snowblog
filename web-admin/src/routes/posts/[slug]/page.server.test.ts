import { beforeEach, describe, expect, it, vi } from 'vitest';
import { isHttpError, isRedirect } from '@sveltejs/kit';
import type { AdminPost } from '$lib/api/types.js';

interface Outcome {
	ok: boolean;
	auditId: string;
	message: string;
	conflict?: boolean;
}

const {
	getPost,
	patchPost,
	deletePost,
	putTranslation,
	deleteTranslation,
	putAsset,
	deleteAsset,
	publish,
	unpublish,
	archive
} = vi.hoisted(() => ({
	getPost: vi.fn(),
	patchPost: vi.fn(),
	deletePost: vi.fn(),
	putTranslation: vi.fn(),
	deleteTranslation: vi.fn(),
	putAsset: vi.fn(),
	deleteAsset: vi.fn(),
	publish: vi.fn(),
	unpublish: vi.fn(),
	archive: vi.fn()
}));

vi.mock('$lib/server/client.js', async (importOriginal) => {
	const actual = await importOriginal<typeof import('$lib/server/client.js')>();
	class FakeAdminApi {
		getPost = getPost;
		patchPost = patchPost;
		deletePost = deletePost;
		putTranslation = putTranslation;
		deleteTranslation = deleteTranslation;
		putAsset = putAsset;
		deleteAsset = deleteAsset;
		publish = publish;
		unpublish = unpublish;
		archive = archive;
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
		translations: [],
		renders: [],
		assets: [],
		freshness: [],
		...overrides
	};
}

function formRequest(fields: Record<string, string> = {}, file?: File): Request {
	const formData = new FormData();
	for (const [key, value] of Object.entries(fields)) formData.set(key, value);
	if (file) formData.set('file', file);
	return { formData: async () => formData } as unknown as Request;
}

function event(fields: Record<string, string> = {}, file?: File) {
	return {
		request: formRequest(fields, file),
		params: { slug: 'my-post' },
		locals: { user: 'alice' },
		fetch
	} as never;
}

beforeEach(() => {
	vi.clearAllMocks();
	recordAudit.mockReturnValue('audit-1');
});

describe('load', () => {
	it('returns the post for a known slug', async () => {
		getPost.mockResolvedValue(post());

		const result = (await load({ params: { slug: 'my-post' }, fetch } as never)) as {
			post: AdminPost;
		};

		expect(getPost).toHaveBeenCalledWith('my-post');
		expect(result.post.slug).toBe('my-post');
	});

	it('maps a 404 ApiError to a SvelteKit error(404)', async () => {
		getPost.mockRejectedValue(new ApiError({ status: 404, title: 'Not Found' }, false));

		const thrown = await Promise.resolve(
			load({ params: { slug: 'missing' }, fetch } as never)
		).catch((e: unknown) => e);

		expect(isHttpError(thrown, 404)).toBe(true);
	});

	it('rethrows a non-404 ApiError', async () => {
		getPost.mockRejectedValue(new ApiError({ status: 500, title: 'Boom' }, false));

		const thrown = await Promise.resolve(
			load({ params: { slug: 'my-post' }, fetch } as never)
		).catch((e: unknown) => e);

		expect(isHttpError(thrown)).toBe(false);
		expect(thrown).toBeInstanceOf(ApiError);
	});
});

describe('actions.saveMeta', () => {
	it('patches slug/default_language/tags and records an ok audit', async () => {
		patchPost.mockResolvedValue(post({ slug: 'my-post' }));

		const result = (await actions.saveMeta(
			event({ revision: '3', slug: 'my-post', default_language: 'zh', tags: 'a, b ,c' })
		)) as Outcome;

		expect(patchPost).toHaveBeenCalledWith('my-post', 3, {
			slug: 'my-post',
			default_language: 'zh',
			tags: ['a', 'b', 'c']
		});
		expect(typeof patchPost.mock.calls[0][1]).toBe('number');
		expect(result).toEqual({ ok: true, auditId: 'audit-1', message: expect.any(String) });
		expect(recordAudit).toHaveBeenCalledTimes(1);
	});

	it('clears tags when the tags field is blank', async () => {
		patchPost.mockResolvedValue(post());

		await actions.saveMeta(
			event({ revision: '3', slug: 'my-post', default_language: 'en', tags: '' })
		);

		expect(patchPost).toHaveBeenCalledWith('my-post', 3, {
			slug: 'my-post',
			default_language: 'en',
			tags: []
		});
	});

	it('redirects to the new slug when the rename changes it', async () => {
		patchPost.mockResolvedValue(post({ slug: 'renamed' }));

		const redirected = await Promise.resolve(
			actions.saveMeta(event({ revision: '3', slug: 'renamed', default_language: 'en', tags: '' }))
		).catch((e: unknown) => e);

		expect(isRedirect(redirected)).toBe(true);
		expect((redirected as { status: number }).status).toBe(303);
		expect((redirected as { location: string }).location).toBe('/posts/renamed');
	});

	it('rejects a non-numeric revision with fail(400) without calling the API', async () => {
		const result = (await actions.saveMeta(
			event({ revision: 'not-a-number', slug: 'my-post', default_language: 'en', tags: '' })
		)) as { status: number; data: Outcome };

		expect(result.status).toBe(400);
		expect(result.data.ok).toBe(false);
		expect(patchPost).not.toHaveBeenCalled();
		expect(recordAudit).toHaveBeenCalledTimes(1);
	});

	it('maps a 412 ApiError to fail(409) with conflict: true', async () => {
		patchPost.mockRejectedValue(new ApiError({ status: 412, title: 'Precondition Failed' }, true));

		const result = (await actions.saveMeta(
			event({ revision: '3', slug: 'my-post', default_language: 'en', tags: '' })
		)) as { status: number; data: Outcome };

		expect(result.status).toBe(409);
		expect(result.data).toMatchObject({ ok: false, conflict: true, auditId: 'audit-1' });
	});
});

describe('actions.saveTranslation', () => {
	it('threads the form revision into putTranslation as a number', async () => {
		putTranslation.mockResolvedValue(post());

		await actions.saveTranslation(
			event({ revision: '7', language: 'en', title: 'Title', description: 'Desc', source: '= hi' })
		);

		expect(putTranslation).toHaveBeenCalledWith('my-post', 7, 'en', {
			title: 'Title',
			description: 'Desc',
			source: '= hi'
		});
		expect(typeof putTranslation.mock.calls[0][1]).toBe('number');
	});

	it('maps a 412 ApiError to fail(409) with conflict: true', async () => {
		putTranslation.mockRejectedValue(
			new ApiError({ status: 412, title: 'Precondition Failed' }, true)
		);

		const result = (await actions.saveTranslation(
			event({ revision: '7', language: 'en', title: 'Title', description: 'Desc', source: '= hi' })
		)) as { status: number; data: Outcome };

		expect(result.status).toBe(409);
		expect(result.data).toMatchObject({ ok: false, conflict: true, auditId: 'audit-1' });
	});

	it('rejects a non-numeric revision with fail(400)', async () => {
		const result = (await actions.saveTranslation(
			event({ revision: 'x', language: 'en', title: 'Title', description: 'Desc', source: 'src' })
		)) as { status: number; data: Outcome };

		expect(result.status).toBe(400);
		expect(putTranslation).not.toHaveBeenCalled();
		expect(recordAudit).toHaveBeenCalledTimes(1);
	});
});

describe('actions.deleteTranslation', () => {
	it('calls deleteTranslation with the language and revision, records an ok audit', async () => {
		deleteTranslation.mockResolvedValue(post());

		const result = (await actions.deleteTranslation(
			event({ revision: '3', language: 'zh' })
		)) as Outcome;

		expect(deleteTranslation).toHaveBeenCalledWith('my-post', 3, 'zh');
		expect(result.ok).toBe(true);
	});
});

describe('actions.uploadAsset', () => {
	it('uses the explicit path field over the file name, and the file content type', async () => {
		putAsset.mockResolvedValue(post());
		const file = new File(['hello'], 'ignored.png', { type: 'image/png' });

		await actions.uploadAsset(event({ revision: '3', path: 'assets/real.png' }, file));

		expect(putAsset).toHaveBeenCalledTimes(1);
		const call = putAsset.mock.calls[0];
		expect(call[0]).toBe('my-post');
		expect(call[1]).toBe(3);
		expect(call[2]).toBe('assets/real.png');
		expect(call[3]).toBeInstanceOf(Uint8Array);
		expect(Buffer.from(call[3] as Uint8Array).toString()).toBe('hello');
		expect(call[4]).toBe('image/png');
	});

	it('falls back to the file name and octet-stream content type', async () => {
		putAsset.mockResolvedValue(post());
		const file = new File(['bytes'], 'picture.bin');

		await actions.uploadAsset(event({ revision: '3' }, file));

		const call = putAsset.mock.calls[0];
		expect(call[2]).toBe('picture.bin');
		expect(call[4]).toBe('application/octet-stream');
	});

	it('rejects a missing file with fail(400)', async () => {
		const result = (await actions.uploadAsset(event({ revision: '3' }))) as {
			status: number;
			data: Outcome;
		};

		expect(result.status).toBe(400);
		expect(putAsset).not.toHaveBeenCalled();
		expect(recordAudit).toHaveBeenCalledTimes(1);
	});
});

describe('actions.deleteAsset', () => {
	it('calls deleteAsset with the path and revision', async () => {
		deleteAsset.mockResolvedValue(post());

		const result = (await actions.deleteAsset(
			event({ revision: '3', path: 'assets/real.png' })
		)) as Outcome;

		expect(deleteAsset).toHaveBeenCalledWith('my-post', 3, 'assets/real.png');
		expect(result.ok).toBe(true);
	});
});

describe('actions.publish', () => {
	it('calls the API even though the loaded post has no fresh renders (the server is authoritative)', async () => {
		publish.mockResolvedValue(post({ freshness: [{ language: 'en', freshness: 'stale' }] }));

		const result = (await actions.publish(event({ revision: '3' }))) as Outcome;

		expect(publish).toHaveBeenCalledWith('my-post', 3);
		expect(result.ok).toBe(true);
	});
});

describe('actions.unpublish', () => {
	it('calls unpublish and records an ok audit', async () => {
		unpublish.mockResolvedValue(post());

		const result = (await actions.unpublish(event({ revision: '3' }))) as Outcome;

		expect(unpublish).toHaveBeenCalledWith('my-post', 3);
		expect(result.ok).toBe(true);
	});
});

describe('actions.archive', () => {
	it('calls archive and records an ok audit', async () => {
		archive.mockResolvedValue(post());

		const result = (await actions.archive(event({ revision: '3' }))) as Outcome;

		expect(archive).toHaveBeenCalledWith('my-post', 3);
		expect(result.ok).toBe(true);
	});
});

describe('actions.deletePost', () => {
	it('redirects to / on a 204 delete', async () => {
		deletePost.mockResolvedValue(undefined);

		const redirected = await Promise.resolve(actions.deletePost(event({ revision: '3' }))).catch(
			(e: unknown) => e
		);

		expect(isRedirect(redirected)).toBe(true);
		expect((redirected as { status: number }).status).toBe(303);
		expect((redirected as { location: string }).location).toBe('/');
		expect(recordAudit).toHaveBeenCalledWith(
			expect.objectContaining({ action: 'deletePost', outcome: 'ok', status: 204 })
		);
	});

	it('maps a 412 ApiError to fail(409) with conflict: true, no redirect', async () => {
		deletePost.mockRejectedValue(new ApiError({ status: 412, title: 'Precondition Failed' }, true));

		const result = (await actions.deletePost(event({ revision: '3' }))) as {
			status: number;
			data: Outcome;
		};

		expect(result.status).toBe(409);
		expect(result.data).toMatchObject({ ok: false, conflict: true, auditId: 'audit-1' });
	});
});
