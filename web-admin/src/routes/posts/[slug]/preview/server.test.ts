import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { PreviewResult } from '$lib/api/types.js';

const { preview } = vi.hoisted(() => ({
	preview: vi.fn()
}));

vi.mock('$lib/server/client.js', async (importOriginal) => {
	const actual = await importOriginal<typeof import('$lib/server/client.js')>();
	class FakeAdminApi {
		preview = preview;
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
const { POST } = await import('./+server.js');

function jsonRequest(body: unknown): Request {
	return {
		json: async () => body
	} as unknown as Request;
}

function event(body: unknown) {
	return {
		request: jsonRequest(body),
		params: { slug: 'my-post' },
		locals: { user: 'alice' },
		fetch
	} as never;
}

beforeEach(() => {
	vi.clearAllMocks();
	recordAudit.mockReturnValue('audit-1');
});

describe('POST /posts/[slug]/preview', () => {
	it('returns a successful preview verbatim and records an ok audit', async () => {
		const result: PreviewResult = { status: 'ok', html: '<p>hi</p>', warnings: [] };
		preview.mockResolvedValue(result);

		const response = await POST(event({ source: '= hi' }));

		expect(preview).toHaveBeenCalledWith('my-post', '= hi');
		expect(response.status).toBe(200);
		await expect(response.json()).resolves.toEqual(result);
		expect(recordAudit).toHaveBeenCalledWith(
			expect.objectContaining({
				user: 'alice',
				action: 'preview',
				slug: 'my-post',
				outcome: 'ok',
				status: 200
			})
		);
	});

	it('returns a failed compile verbatim as a 200, still an ok audit outcome', async () => {
		const result: PreviewResult = {
			status: 'failed',
			diagnostics: [{ severity: 'error', message: 'syntax error' }]
		};
		preview.mockResolvedValue(result);

		const response = await POST(event({ source: '= bad' }));

		expect(response.status).toBe(200);
		await expect(response.json()).resolves.toEqual(result);
		expect(recordAudit).toHaveBeenCalledWith(
			expect.objectContaining({ action: 'preview', outcome: 'ok', status: 200 })
		);
	});

	it('rejects a missing source with 400 and does not call the API', async () => {
		const response = await POST(event({}));

		expect(response.status).toBe(400);
		expect(preview).not.toHaveBeenCalled();
	});

	it('rejects a non-string source with 400', async () => {
		const response = await POST(event({ source: 42 }));

		expect(response.status).toBe(400);
		expect(preview).not.toHaveBeenCalled();
	});

	it('rejects an unparsable body with 400', async () => {
		const request = {
			json: async () => {
				throw new SyntaxError('bad json');
			}
		} as unknown as Request;

		const response = await POST({
			request,
			params: { slug: 'my-post' },
			locals: { user: 'alice' },
			fetch
		} as never);

		expect(response.status).toBe(400);
		expect(preview).not.toHaveBeenCalled();
	});

	it('maps an ApiError from the preview call to its status with a safe body and a failed audit', async () => {
		preview.mockRejectedValue(new ApiError({ status: 502, detail: 'compiler unreachable' }, false));

		const response = await POST(event({ source: '= hi' }));

		expect(response.status).toBe(502);
		await expect(response.json()).resolves.toEqual({ message: 'compiler unreachable' });
		expect(recordAudit).toHaveBeenCalledWith(
			expect.objectContaining({ action: 'preview', outcome: 'failed', status: 502 })
		);
	});
});
