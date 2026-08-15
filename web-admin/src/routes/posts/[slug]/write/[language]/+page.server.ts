import { error, fail } from '@sveltejs/kit';
import { AdminApi, ApiError } from '$lib/server/client.js';
import { getConfig } from '$lib/server/config.js';
import { recordAudit } from '$lib/server/audit.js';
import type { Problem } from '$lib/api/types.js';
import type { Actions, PageServerLoad } from './$types';

interface ActionOutcome {
	ok: false;
	auditId: string;
	message: string;
	conflict?: boolean;
}

type Failure = ReturnType<typeof fail<ActionOutcome>>;
type MutationResult<T> = { success: true; value: T } | { success: false; failure: Failure };

function problemMessage(problem: Problem): string {
	return problem.detail ?? problem.title ?? 'request failed';
}

function parseRevision(form: FormData): number | null {
	const revision = Number(form.get('revision'));
	return Number.isNaN(revision) ? null : revision;
}

function invalidRevision(locals: App.Locals, action: string, slug: string): Failure {
	const auditId = recordAudit({ user: locals.user, action, slug, outcome: 'failed', status: 400 });
	return fail(400, { ok: false as const, auditId, message: 'revision is required' });
}

async function runMutation<T>(
	locals: App.Locals,
	action: string,
	slug: string,
	call: () => Promise<T>
): Promise<MutationResult<T>> {
	try {
		return { success: true, value: await call() };
	} catch (err) {
		if (!(err instanceof ApiError)) throw err;
		const message = problemMessage(err.problem);
		const auditId = recordAudit({
			user: locals.user,
			action,
			slug,
			outcome: err.conflict ? 'conflict' : 'failed',
			status: err.problem.status
		});
		const failure = err.conflict
			? fail(409, { ok: false as const, conflict: true, auditId, message })
			: fail(err.problem.status, { ok: false as const, auditId, message });
		return { success: false, failure };
	}
}

export const load: PageServerLoad = async ({ params, fetch }) => {
	const api = new AdminApi(getConfig(), fetch);
	try {
		const post = await api.getPost(params.slug);
		const translation = post.translations.find((entry) => entry.language === params.language);
		if (!translation) {
			error(404, `no ${params.language} translation for ${params.slug}`);
		}
		return { post, translation };
	} catch (err) {
		if (err instanceof ApiError && err.problem.status === 404) {
			error(404, problemMessage(err.problem));
		}
		throw err;
	}
};

export const actions: Actions = {
	save: async ({ request, params, locals, fetch }) => {
		const form = await request.formData();
		const revision = parseRevision(form);
		if (revision === null) return invalidRevision(locals, 'save', params.slug);

		const translation = {
			title: String(form.get('title') ?? '').trim(),
			description: String(form.get('description') ?? '').trim(),
			source: String(form.get('source') ?? '')
		};

		const api = new AdminApi(getConfig(), fetch);
		const result = await runMutation(locals, 'save', params.slug, () =>
			api.putTranslation(params.slug, revision, params.language, translation)
		);
		if (!result.success) return result.failure;

		const auditId = recordAudit({
			user: locals.user,
			action: 'save',
			slug: params.slug,
			outcome: 'ok',
			status: 200
		});
		return {
			ok: true,
			auditId,
			message: `translation ${params.language} saved`,
			revision: result.value.revision
		};
	}
};
