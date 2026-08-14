export interface AdminTranslation {
	language: string;
	title: string;
	description: string;
	source: string;
	updated_at: string;
}
export interface AdminRender {
	language: string;
	renderer_version: string;
	input_hash: string;
	warnings: Diagnostic[];
	rendered_at: string;
}
export interface Diagnostic {
	severity: string;
	message: string;
	hints?: string[];
}
export interface AssetRef {
	path: string;
	content_type: string;
	content_hash: string;
}
export interface TranslationFreshness {
	language: string;
	freshness: 'fresh' | 'stale' | 'missing';
}
export interface AdminPost {
	id: string;
	slug: string;
	status: 'draft' | 'published' | 'archived';
	default_language: string;
	revision: number;
	tags: string[];
	published_at: string | null;
	created_at: string;
	updated_at: string;
	translations: AdminTranslation[];
	renders: AdminRender[];
	assets: AssetRef[];
	freshness: TranslationFreshness[];
}
export interface Problem {
	status: number;
	type?: string;
	title?: string;
	detail?: string;
}
export type PreviewResult =
	| { status: 'ok'; html: string; warnings: Diagnostic[] }
	| { status: 'failed'; diagnostics: Diagnostic[] };
