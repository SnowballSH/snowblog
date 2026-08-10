export const PAGE_SIZE = 10;

export interface PostSummary {
	id: string;
	slug: string;
	languages: string[];
	default_language: string;
	tags: string[];
	published_at: string | null;
	title: string;
	description: string;
}

export interface PostDetail extends PostSummary {
	language: string;
	html: string;
	rendered_with: {
		renderer_version: string;
		rendered_at: string;
	};
}

export interface PostListPage {
	posts: PostSummary[];
	page: number;
	hasNext: boolean;
}
