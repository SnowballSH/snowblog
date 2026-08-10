import { PAGE_SIZE, type PostDetail, type PostListPage, type PostSummary } from './types.js';

export class ApiError extends Error {
	constructor(
		public readonly status: number,
		message: string
	) {
		super(message);
		this.name = 'ApiError';
	}
}

export function apiBase(): string {
	const base = process.env.SNOWBLOG_API_URL;
	if (!base) {
		throw new Error('SNOWBLOG_API_URL is not configured');
	}
	return base.replace(/\/+$/, '');
}

async function request(fetchFn: typeof fetch, path: string): Promise<Response> {
	let response: Response;
	try {
		response = await fetchFn(`${apiBase()}${path}`);
	} catch (cause) {
		throw new ApiError(502, `snowblog API unreachable: ${String(cause)}`);
	}
	if (!response.ok) {
		throw new ApiError(response.status, `snowblog API responded ${response.status} for ${path}`);
	}
	return response;
}

export async function fetchPostList(fetchFn: typeof fetch, page: number): Promise<PostListPage> {
	if (!Number.isInteger(page) || page < 1) {
		throw new ApiError(404, `no such page: ${page}`);
	}
	const offset = (page - 1) * PAGE_SIZE;
	const response = await request(fetchFn, `/api/v1/posts?limit=${PAGE_SIZE + 1}&offset=${offset}`);
	const body = (await response.json()) as { posts: PostSummary[] };
	return {
		posts: body.posts.slice(0, PAGE_SIZE),
		page,
		hasNext: body.posts.length > PAGE_SIZE
	};
}

export async function fetchPost(
	fetchFn: typeof fetch,
	slug: string,
	language?: string
): Promise<PostDetail> {
	const query = language ? `?language=${encodeURIComponent(language)}` : '';
	const response = await request(fetchFn, `/api/v1/posts/${encodeURIComponent(slug)}${query}`);
	return (await response.json()) as PostDetail;
}
