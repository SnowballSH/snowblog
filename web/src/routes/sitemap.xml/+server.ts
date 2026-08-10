import { fetchPostList } from '$lib/api/client.js';
import { postAlternates } from '$lib/head.js';
import type { PostSummary } from '$lib/api/types.js';
import type { RequestHandler } from './$types';

function escapeXml(value: string): string {
	return value
		.replaceAll('&', '&amp;')
		.replaceAll('<', '&lt;')
		.replaceAll('>', '&gt;')
		.replaceAll("'", '&apos;')
		.replaceAll('"', '&quot;');
}

export const GET: RequestHandler = async ({ url, fetch }) => {
	const posts: PostSummary[] = [];
	let page = 1;
	let hasNext = true;
	while (hasNext) {
		const result = await fetchPostList(fetch, page);
		posts.push(...result.posts);
		hasNext = result.hasNext;
		page += 1;
	}

	const urls = posts.flatMap((post) =>
		postAlternates(url.origin, post.slug, post.languages, post.default_language).alternates.map(
			(alt) => `\t<url>\n\t\t<loc>${escapeXml(alt.href)}</loc>\n\t</url>`
		)
	);
	const body = [
		'<?xml version="1.0" encoding="UTF-8"?>',
		'<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">',
		...urls,
		'</urlset>',
		''
	].join('\n');
	return new Response(body, { headers: { 'content-type': 'application/xml; charset=utf-8' } });
};
