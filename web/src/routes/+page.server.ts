import { error } from '@sveltejs/kit';
import { ApiError, fetchPostList } from '$lib/api/client.js';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ url, fetch }) => {
	const raw = url.searchParams.get('page') ?? '1';
	const page = Number(raw);
	if (!/^\d+$/.test(raw) || page < 1) {
		error(404, 'no such page');
	}
	try {
		return await fetchPostList(fetch, page);
	} catch (cause) {
		if (cause instanceof ApiError) {
			error(cause.status, cause.message);
		}
		throw cause;
	}
};
