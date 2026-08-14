export function startFixtureAdminApi(): { port: number; stop: () => void } {
	const server = Bun.serve({
		port: 0,
		fetch(request) {
			const url = new URL(request.url);
			if (url.pathname === '/api/v1/admin/posts') {
				return Response.json({ posts: [] });
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
