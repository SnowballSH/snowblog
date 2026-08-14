import type { AdminConfig } from './config.js';
import type { AdminPost, PreviewResult, Problem } from '../api/types.js';

export class ApiError extends Error {
	constructor(
		public readonly problem: Problem,
		public readonly conflict: boolean
	) {
		super(problem.detail ?? problem.title ?? `request failed with status ${problem.status}`);
		this.name = 'ApiError';
	}
}

interface RequestOptions {
	revision?: number;
	json?: unknown;
	body?: Uint8Array;
	contentType?: string;
}

export class AdminApi {
	private readonly config: AdminConfig;
	private readonly fetchFn: typeof fetch;

	constructor(config: AdminConfig, fetchFn: typeof fetch = fetch) {
		this.config = config;
		this.fetchFn = fetchFn;
	}

	async listPosts(): Promise<AdminPost[]> {
		const body = await this.request<{ posts: AdminPost[] }>('GET', '/api/v1/admin/posts');
		return body.posts;
	}

	async getPost(slug: string): Promise<AdminPost> {
		return this.request<AdminPost>('GET', `/api/v1/admin/posts/${encodeURIComponent(slug)}`);
	}

	async createPost(body: {
		slug: string;
		default_language: string;
		tags?: string[];
	}): Promise<AdminPost> {
		return this.request<AdminPost>('POST', '/api/v1/admin/posts', { json: body });
	}

	async patchPost(slug: string, revision: number, patch: object): Promise<AdminPost> {
		return this.request<AdminPost>('PATCH', `/api/v1/admin/posts/${encodeURIComponent(slug)}`, {
			revision,
			json: patch
		});
	}

	async deletePost(slug: string, revision: number): Promise<void> {
		await this.request<void>('DELETE', `/api/v1/admin/posts/${encodeURIComponent(slug)}`, {
			revision
		});
	}

	async putTranslation(
		slug: string,
		revision: number,
		language: string,
		body: { title: string; description: string; source: string }
	): Promise<AdminPost> {
		const result = await this.request<{ post: AdminPost }>(
			'PUT',
			`/api/v1/admin/posts/${encodeURIComponent(slug)}/translations/${encodeURIComponent(language)}`,
			{ revision, json: body }
		);
		return result.post;
	}

	async deleteTranslation(slug: string, revision: number, language: string): Promise<AdminPost> {
		return this.request<AdminPost>(
			'DELETE',
			`/api/v1/admin/posts/${encodeURIComponent(slug)}/translations/${encodeURIComponent(language)}`,
			{ revision }
		);
	}

	async putAsset(
		slug: string,
		revision: number,
		path: string,
		content: Uint8Array,
		contentType: string
	): Promise<AdminPost> {
		const result = await this.request<{ post: AdminPost }>(
			'PUT',
			`/api/v1/admin/posts/${encodeURIComponent(slug)}/assets/${encodeURIComponent(path)}`,
			{ revision, body: content, contentType }
		);
		return result.post;
	}

	async deleteAsset(slug: string, revision: number, path: string): Promise<AdminPost> {
		const result = await this.request<{ post: AdminPost }>(
			'DELETE',
			`/api/v1/admin/posts/${encodeURIComponent(slug)}/assets/${encodeURIComponent(path)}`,
			{ revision }
		);
		return result.post;
	}

	async preview(slug: string, source: string): Promise<PreviewResult> {
		return this.request<PreviewResult>(
			'POST',
			`/api/v1/admin/posts/${encodeURIComponent(slug)}/preview`,
			{ json: { source } }
		);
	}

	async publish(slug: string, revision: number): Promise<AdminPost> {
		return this.request<AdminPost>(
			'POST',
			`/api/v1/admin/posts/${encodeURIComponent(slug)}/publish`,
			{ revision }
		);
	}

	async unpublish(slug: string, revision: number): Promise<AdminPost> {
		return this.request<AdminPost>(
			'POST',
			`/api/v1/admin/posts/${encodeURIComponent(slug)}/unpublish`,
			{ revision }
		);
	}

	async archive(slug: string, revision: number): Promise<AdminPost> {
		return this.request<AdminPost>(
			'POST',
			`/api/v1/admin/posts/${encodeURIComponent(slug)}/archive`,
			{ revision }
		);
	}

	private async request<T>(method: string, path: string, options: RequestOptions = {}): Promise<T> {
		const headers = new Headers();
		headers.set('authorization', `Bearer ${this.config.adminToken}`);
		if (options.revision !== undefined) {
			headers.set('if-match', `"${options.revision}"`);
		}

		let requestBody: BodyInit | undefined;
		if (options.json !== undefined) {
			headers.set('content-type', 'application/json');
			requestBody = JSON.stringify(options.json);
		} else if (options.body !== undefined) {
			headers.set('content-type', options.contentType ?? 'application/octet-stream');
			requestBody = new Blob([Uint8Array.from(options.body)]);
		}

		const response = await this.fetchFn(`${this.config.apiUrl}${path}`, {
			method,
			headers,
			body: requestBody
		});

		if (!response.ok) {
			const problem = (await response.json()) as Problem;
			throw new ApiError(problem, response.status === 412);
		}

		if (response.status === 204) {
			return undefined as T;
		}

		return (await response.json()) as T;
	}
}
