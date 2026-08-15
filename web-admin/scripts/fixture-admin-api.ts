const smokePost = {
	id: 'smoke-post-id',
	slug: 'smoke-post',
	status: 'draft',
	default_language: 'en',
	revision: 1,
	tags: [],
	published_at: null,
	created_at: '2026-01-01T00:00:00Z',
	updated_at: '2026-01-01T00:00:00Z',
	translations: [
		{
			language: 'en',
			title: 'Smoke post',
			description: 'A post used by the smoke test',
			source: '= Smoke\n\nHello.',
			updated_at: '2026-01-01T00:00:00Z'
		}
	],
	renders: [],
	assets: [],
	freshness: [{ language: 'en', freshness: 'fresh' }]
};

export function startFixtureAdminApi(): { port: number; stop: () => void } {
	const server = Bun.serve({
		port: 0,
		fetch(request) {
			const url = new URL(request.url);
			if (url.pathname === '/api/v1/admin/posts') {
				return Response.json({ posts: [] });
			}
			if (url.pathname === '/api/v1/admin/posts/smoke-post') {
				return Response.json(smokePost);
			}
			return Response.json(
				{ type: 'about:blank', status: 404, code: 'not_found' },
				{ status: 404 }
			);
		}
	});
	return { port: server.port, stop: () => server.stop(true) };
}

if (import.meta.main) {
	const { port } = startFixtureAdminApi();
	console.log(`fixture admin api on http://127.0.0.1:${port}`);
}
