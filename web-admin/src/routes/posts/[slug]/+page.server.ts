import { error, fail, redirect } from '@sveltejs/kit';
import { base } from '$app/paths';
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

function parseTags(raw: FormDataEntryValue | null): string[] {
	const value = typeof raw === 'string' ? raw.trim() : '';
	if (!value) return [];
	return value
		.split(',')
		.map((tag) => tag.trim())
		.filter(Boolean);
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
		return { post };
	} catch (err) {
		if (err instanceof ApiError && err.problem.status === 404) {
			error(404, problemMessage(err.problem));
		}
		throw err;
	}
};

export const actions: Actions = {
	saveMeta: async ({ request, params, locals, fetch }) => {
		const form = await request.formData();
		const revision = parseRevision(form);
		if (revision === null) return invalidRevision(locals, 'saveMeta', params.slug);

		const patch = {
			slug: String(form.get('slug') ?? '').trim(),
			default_language: String(form.get('default_language') ?? '').trim(),
			tags: parseTags(form.get('tags'))
		};

		const api = new AdminApi(getConfig(), fetch);
		const result = await runMutation(locals, 'saveMeta', params.slug, () =>
			api.patchPost(params.slug, revision, patch)
		);
		if (!result.success) return result.failure;

		const auditId = recordAudit({
			user: locals.user,
			action: 'saveMeta',
			slug: params.slug,
			outcome: 'ok',
			status: 200
		});
		if (result.value.slug !== params.slug) {
			redirect(303, `${base}/posts/${result.value.slug}`);
		}
		return { ok: true, auditId, message: 'metadata saved' };
	},

	saveTranslation: async ({ request, params, locals, fetch }) => {
		const form = await request.formData();
		const revision = parseRevision(form);
		if (revision === null) return invalidRevision(locals, 'saveTranslation', params.slug);

		const language = String(form.get('language') ?? '').trim();
		const translation = {
			title: String(form.get('title') ?? '').trim(),
			description: String(form.get('description') ?? '').trim(),
			source: String(form.get('source') ?? '')
		};

		const api = new AdminApi(getConfig(), fetch);
		const result = await runMutation(locals, 'saveTranslation', params.slug, () =>
			api.putTranslation(params.slug, revision, language, translation)
		);
		if (!result.success) return result.failure;

		const auditId = recordAudit({
			user: locals.user,
			action: 'saveTranslation',
			slug: params.slug,
			outcome: 'ok',
			status: 200
		});
		return { ok: true, auditId, message: `translation ${language} saved` };
	},

	deleteTranslation: async ({ request, params, locals, fetch }) => {
		const form = await request.formData();
		const revision = parseRevision(form);
		if (revision === null) return invalidRevision(locals, 'deleteTranslation', params.slug);

		const language = String(form.get('language') ?? '').trim();

		const api = new AdminApi(getConfig(), fetch);
		const result = await runMutation(locals, 'deleteTranslation', params.slug, () =>
			api.deleteTranslation(params.slug, revision, language)
		);
		if (!result.success) return result.failure;

		const auditId = recordAudit({
			user: locals.user,
			action: 'deleteTranslation',
			slug: params.slug,
			outcome: 'ok',
			status: 200
		});
		return { ok: true, auditId, message: `translation ${language} deleted` };
	},

	uploadAsset: async ({ request, params, locals, fetch }) => {
		const form = await request.formData();
		const revision = parseRevision(form);
		if (revision === null) return invalidRevision(locals, 'uploadAsset', params.slug);

		const file = form.get('file');
		if (!(file instanceof File) || file.size === 0) {
			const auditId = recordAudit({
				user: locals.user,
				action: 'uploadAsset',
				slug: params.slug,
				outcome: 'failed',
				status: 400
			});
			return fail(400, { ok: false as const, auditId, message: 'file is required' });
		}

		const explicitPath = String(form.get('path') ?? '').trim();
		const path = explicitPath || file.name;
		const contentType = file.type || 'application/octet-stream';
		const content = new Uint8Array(await file.arrayBuffer());

		const api = new AdminApi(getConfig(), fetch);
		const result = await runMutation(locals, 'uploadAsset', params.slug, () =>
			api.putAsset(params.slug, revision, path, content, contentType)
		);
		if (!result.success) return result.failure;

		const auditId = recordAudit({
			user: locals.user,
			action: 'uploadAsset',
			slug: params.slug,
			outcome: 'ok',
			status: 200
		});
		return { ok: true, auditId, message: `asset ${path} uploaded` };
	},

	deleteAsset: async ({ request, params, locals, fetch }) => {
		const form = await request.formData();
		const revision = parseRevision(form);
		if (revision === null) return invalidRevision(locals, 'deleteAsset', params.slug);

		const path = String(form.get('path') ?? '').trim();

		const api = new AdminApi(getConfig(), fetch);
		const result = await runMutation(locals, 'deleteAsset', params.slug, () =>
			api.deleteAsset(params.slug, revision, path)
		);
		if (!result.success) return result.failure;

		const auditId = recordAudit({
			user: locals.user,
			action: 'deleteAsset',
			slug: params.slug,
			outcome: 'ok',
			status: 200
		});
		return { ok: true, auditId, message: `asset ${path} deleted` };
	},

	publish: async ({ request, params, locals, fetch }) => {
		const form = await request.formData();
		const revision = parseRevision(form);
		if (revision === null) return invalidRevision(locals, 'publish', params.slug);

		const api = new AdminApi(getConfig(), fetch);
		const result = await runMutation(locals, 'publish', params.slug, () =>
			api.publish(params.slug, revision)
		);
		if (!result.success) return result.failure;

		const auditId = recordAudit({
			user: locals.user,
			action: 'publish',
			slug: params.slug,
			outcome: 'ok',
			status: 200
		});
		return { ok: true, auditId, message: 'post published' };
	},

	unpublish: async ({ request, params, locals, fetch }) => {
		const form = await request.formData();
		const revision = parseRevision(form);
		if (revision === null) return invalidRevision(locals, 'unpublish', params.slug);

		const api = new AdminApi(getConfig(), fetch);
		const result = await runMutation(locals, 'unpublish', params.slug, () =>
			api.unpublish(params.slug, revision)
		);
		if (!result.success) return result.failure;

		const auditId = recordAudit({
			user: locals.user,
			action: 'unpublish',
			slug: params.slug,
			outcome: 'ok',
			status: 200
		});
		return { ok: true, auditId, message: 'post unpublished' };
	},

	archive: async ({ request, params, locals, fetch }) => {
		const form = await request.formData();
		const revision = parseRevision(form);
		if (revision === null) return invalidRevision(locals, 'archive', params.slug);

		const api = new AdminApi(getConfig(), fetch);
		const result = await runMutation(locals, 'archive', params.slug, () =>
			api.archive(params.slug, revision)
		);
		if (!result.success) return result.failure;

		const auditId = recordAudit({
			user: locals.user,
			action: 'archive',
			slug: params.slug,
			outcome: 'ok',
			status: 200
		});
		return { ok: true, auditId, message: 'post archived' };
	},

	deletePost: async ({ request, params, locals, fetch }) => {
		const form = await request.formData();
		const revision = parseRevision(form);
		if (revision === null) return invalidRevision(locals, 'deletePost', params.slug);

		const api = new AdminApi(getConfig(), fetch);
		const result = await runMutation(locals, 'deletePost', params.slug, () =>
			api.deletePost(params.slug, revision)
		);
		if (!result.success) return result.failure;

		recordAudit({
			user: locals.user,
			action: 'deletePost',
			slug: params.slug,
			outcome: 'ok',
			status: 204
		});
		redirect(303, base || '/');
	}
};
