const posts = [
	{
		id: 'fixture-1',
		slug: 'fixture-post',
		languages: ['en', 'zh'],
		default_language: 'en',
		tags: ['fixture'],
		published_at: '2026-08-01T00:00:00Z',
		title: 'Fixture Post',
		description: 'A post served by the smoke fixture.'
	}
];

const detail = {
	...posts[0],
	language: 'en',
	html: '<p>fixture-body-marker</p><img src="/api/v1/posts/fixture-post/assets/x.png" alt="">',
	rendered_with: { renderer_version: '0.0.0-fixture', rendered_at: '2026-08-01T00:00:00Z' }
};

export function startFixtureApi(): { port: number; stop: () => void } {
	const server = Bun.serve({
		port: 0,
		fetch(request) {
			const url = new URL(request.url);
			if (url.pathname === '/api/v1/health') {
				return Response.json({ database: 'ok' });
			}
			if (url.pathname === '/api/v1/posts') {
				return Response.json({ posts });
			}
			if (url.pathname === '/api/v1/posts/fixture-post') {
				return Response.json(detail);
			}
			return Response.json(
				{ type: 'about:blank', status: 404, code: 'post_not_found' },
				{ status: 404 }
			);
		}
	});
	return { port: server.port, stop: () => server.stop(true) };
}

if (import.meta.main) {
	const { port } = startFixtureApi();
	console.log(`fixture api on http://127.0.0.1:${port}`);
}
