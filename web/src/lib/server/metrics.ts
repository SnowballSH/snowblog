import { createServer, type Server } from 'node:http';
import client from 'prom-client';

export const registry = new client.Registry();

client.collectDefaultMetrics({ register: registry });

export const httpRequestsTotal = new client.Counter({
	name: 'snowblog_web_http_requests_total',
	help: 'Requests served by the snowblog web frontend',
	labelNames: ['method', 'status', 'route'] as const,
	registers: [registry]
});

export const httpRequestDuration = new client.Histogram({
	name: 'snowblog_web_http_request_duration_seconds',
	help: 'Request duration for the snowblog web frontend',
	labelNames: ['method', 'route'] as const,
	buckets: [0.005, 0.025, 0.1, 0.25, 0.5, 1, 2.5, 5],
	registers: [registry]
});

export const upstreamFailures = new client.Counter({
	name: 'snowblog_web_upstream_failures_total',
	help: 'Server-side loads that failed against the snowblog API',
	registers: [registry]
});

export function observeRequest(
	method: string,
	route: string,
	status: number,
	seconds: number
): void {
	httpRequestsTotal.inc({ method, status: String(status), route });
	httpRequestDuration.observe({ method, route }, seconds);
	if (status >= 502 && status <= 504) {
		upstreamFailures.inc();
	}
}

export function startMetricsServer(listen: string): Server {
	const [host, portText] = listen.split(':');
	const port = Number(portText);
	if (!host || !Number.isInteger(port) || port < 1 || port > 65535) {
		throw new Error(`invalid SNOWBLOG_WEB_METRICS_LISTEN: ${listen}`);
	}
	const server = createServer((request, response) => {
		if (request.url === '/metrics') {
			registry
				.metrics()
				.then((body) => {
					response.writeHead(200, { 'content-type': registry.contentType });
					response.end(body);
				})
				.catch(() => {
					response.writeHead(500);
					response.end();
				});
			return;
		}
		response.writeHead(404);
		response.end();
	});
	server.listen(port, host);
	server.unref();
	process.once('sveltekit:shutdown', () => {
		server.close();
	});
	return server;
}
