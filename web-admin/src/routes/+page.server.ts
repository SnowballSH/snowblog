import { fail, redirect } from '@sveltejs/kit';
import { base } from '$app/paths';
import { AdminApi, ApiError } from '$lib/server/client.js';
import { getConfig } from '$lib/server/config.js';
import { recordAudit } from '$lib/server/audit.js';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ fetch }) => {
	const api = new AdminApi(getConfig(), fetch);
	const posts = await api.listPosts();
	posts.sort((a, b) => b.updated_at.localeCompare(a.updated_at));
	return { posts };
};

export const actions: Actions = {
	create: async ({ request, locals, fetch }) => {
		const form = await request.formData();
		const slug = String(form.get('slug') ?? '').trim();
		const defaultLanguage = String(form.get('default_language') ?? '').trim();
		const tagsField = String(form.get('tags') ?? '').trim();
		const tags = tagsField
			? tagsField
					.split(',')
					.map((tag) => tag.trim())
					.filter(Boolean)
			: undefined;

		const api = new AdminApi(getConfig(), fetch);

		let created;
		try {
			created = await api.createPost({ slug, default_language: defaultLanguage, tags });
		} catch (error) {
			if (!(error instanceof ApiError)) throw error;
			const message = error.problem.detail ?? error.problem.title ?? 'request failed';
			const auditId = recordAudit({
				user: locals.user,
				action: 'create',
				slug,
				outcome: error.conflict ? 'conflict' : 'failed',
				status: error.problem.status
			});
			if (error.conflict) {
				return fail(409, { ok: false, conflict: true, auditId, message });
			}
			return fail(error.problem.status, { ok: false, auditId, message });
		}

		recordAudit({
			user: locals.user,
			action: 'create',
			slug: created.slug,
			outcome: 'ok',
			status: 201
		});

		redirect(303, `${base}/posts/${created.slug}`);
	}
};
