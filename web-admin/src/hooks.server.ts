import type { Handle, ServerInit } from '@sveltejs/kit';
import { readFileSync } from 'node:fs';
import { building } from '$app/environment';
import { base } from '$app/paths';
import { getConfig, initConfig, loadConfig } from '$lib/server/config.js';
import { crossSiteViolation, identityFrom } from '$lib/server/guard.js';
import { observeRequest, startMetricsServer } from '$lib/server/metrics.js';

export const init: ServerInit = () => {
	if (building) return;
	const config = loadConfig(process.env, (path) => readFileSync(path, 'utf8'));
	initConfig(config);
	if (config.metricsListen) startMetricsServer(config.metricsListen);
};

export const handle: Handle = async ({ event, resolve }) => {
	const started = performance.now();
	let response: Response;
	if (event.url.pathname === `${base}/healthz`) {
		response = Response.json({ status: 'ok' });
	} else {
		const config = getConfig();
		const user = identityFrom(event.request.headers, config);
		const violation = user === null ? null : crossSiteViolation(event.request, config);
		if (user === null) {
			response = new Response('forbidden', { status: 403 });
		} else if (violation !== null) {
			response = new Response(violation, { status: 403 });
		} else {
			event.locals.user = user;
			response = await resolve(event);
		}
	}
	observeRequest(
		event.request.method,
		event.route.id ?? event.url.pathname,
		response.status,
		(performance.now() - started) / 1000
	);
	return response;
};
