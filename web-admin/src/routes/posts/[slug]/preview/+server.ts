import { AdminApi, ApiError } from '$lib/server/client.js';
import { getConfig } from '$lib/server/config.js';
import { recordAudit } from '$lib/server/audit.js';
import type { Problem } from '$lib/api/types.js';
import type { RequestHandler } from './$types';

function problemMessage(problem: Problem): string {
	return problem.detail ?? problem.title ?? 'request failed';
}

async function parseSource(request: Request): Promise<string | null> {
	const body = await request.json().catch(() => null);
	const source = (body as { source?: unknown } | null)?.source;
	return typeof source === 'string' ? source : null;
}

export const POST: RequestHandler = async ({ request, params, locals, fetch }) => {
	const source = await parseSource(request);
	if (source === null) {
		return Response.json({ message: 'source is required' }, { status: 400 });
	}

	const api = new AdminApi(getConfig(), fetch);
	try {
		const result = await api.preview(params.slug, source);
		recordAudit({
			user: locals.user,
			action: 'preview',
			slug: params.slug,
			outcome: 'ok',
			status: 200
		});
		return Response.json(result);
	} catch (err) {
		if (!(err instanceof ApiError)) throw err;
		recordAudit({
			user: locals.user,
			action: 'preview',
			slug: params.slug,
			outcome: 'failed',
			status: err.problem.status
		});
		return Response.json({ message: problemMessage(err.problem) }, { status: err.problem.status });
	}
};
