import type { Handle, ServerInit } from '@sveltejs/kit';
import { building } from '$app/environment';
import { observeRequest, startMetricsServer } from '$lib/server/metrics.js';

export const init: ServerInit = () => {
	const listen = process.env.SNOWBLOG_WEB_METRICS_LISTEN;
	if (!building && listen) {
		startMetricsServer(listen);
	}
};

export const handle: Handle = async ({ event, resolve }) => {
	const started = performance.now();
	const response = await resolve(event);
	const route = event.route.id ?? 'unresolved';
	observeRequest(
		event.request.method,
		route,
		response.status,
		(performance.now() - started) / 1000
	);
	return response;
};
