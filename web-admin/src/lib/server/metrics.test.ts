import { afterAll, describe, expect, it } from 'vitest';
import type { Server } from 'node:http';
import { observeRequest, registry, startMetricsServer } from './metrics.js';

let server: Server | undefined;

afterAll(() => {
	server?.close();
});

describe('metrics', () => {
	it('records requests, durations, and upstream failures', async () => {
		observeRequest('GET', '/', 200, 0.05);
		observeRequest('GET', '/posts/[slug]', 502, 0.01);
		const body = await registry.metrics();
		expect(body).toContain(
			'snowblog_admin_http_requests_total{method="GET",status="200",route="/"} 1'
		);
		expect(body).toContain(
			'snowblog_admin_http_requests_total{method="GET",status="502",route="/posts/[slug]"} 1'
		);
		expect(body).toContain('snowblog_admin_upstream_failures_total 1');
		expect(body).toContain('snowblog_admin_http_request_duration_seconds_bucket');
		expect(body).toContain('process_cpu_user_seconds_total');
	});

	it('serves the registry over its own listener and 404s elsewhere', async () => {
		server = startMetricsServer('127.0.0.1:19103');
		await new Promise((resolve) => server?.once('listening', resolve));
		const metrics = await fetch('http://127.0.0.1:19103/metrics');
		expect(metrics.status).toBe(200);
		expect(await metrics.text()).toContain('snowblog_admin_http_requests_total');
		const other = await fetch('http://127.0.0.1:19103/other');
		expect(other.status).toBe(404);
	});

	it('rejects a malformed listen address', () => {
		expect(() => startMetricsServer('nonsense')).toThrow(/SNOWBLOG_ADMIN_METRICS_LISTEN/);
	});
});
