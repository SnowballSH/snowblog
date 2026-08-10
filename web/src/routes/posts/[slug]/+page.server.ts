import { error, redirect } from '@sveltejs/kit';
import { ApiError, fetchPost } from '$lib/api/client.js';
import { postAlternates } from '$lib/head.js';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ params, url, fetch }) => {
	const lang = url.searchParams.get('lang') ?? undefined;
	const bare = `/posts/${encodeURIComponent(params.slug)}`;
	let post;
	try {
		post = await fetchPost(fetch, params.slug, lang);
	} catch (cause) {
		if (cause instanceof ApiError) {
			if (cause.status === 404 && lang) {
				redirect(302, bare);
			}
			error(cause.status, cause.message);
		}
		throw cause;
	}
	if (lang && !post.languages.includes(lang)) {
		redirect(302, bare);
	}
	return {
		post,
		...postAlternates(url.origin, post.slug, post.languages, post.default_language)
	};
};
