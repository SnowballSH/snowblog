import { spawn } from 'node:child_process';
import { startFixtureApi } from './fixture-api.ts';

const PORT = Number(process.env.SMOKE_PORT ?? 4173);
const METRICS_PORT = PORT + 1;
const ORIGIN = `http://127.0.0.1:${PORT}`;

const api = startFixtureApi();
const server = spawn('node', ['build'], {
	env: {
		...process.env,
		NODE_ENV: 'production',
		PORT: String(PORT),
		ORIGIN,
		SNOWBLOG_API_URL: `http://127.0.0.1:${api.port}`,
		SNOWBLOG_WEB_METRICS_LISTEN: `127.0.0.1:${METRICS_PORT}`
	},
	stdio: 'inherit'
});

function shutdown(code: number): never {
	server.kill();
	api.stop();
	process.exit(code);
}

async function waitUntilReady(): Promise<void> {
	for (let attempt = 0; attempt < 50; attempt += 1) {
		try {
			const response = await fetch(`${ORIGIN}/`, { redirect: 'manual' });
			if (response.status > 0) return;
		} catch {
			await new Promise((resolve) => setTimeout(resolve, 200));
		}
	}
	console.error('server never became ready');
	shutdown(1);
}

interface Check {
	path: string;
	status: number;
	contains: string;
}

const checks: Check[] = [
	{ path: '/', status: 200, contains: 'Fixture Post' },
	{ path: '/posts/fixture-post', status: 200, contains: 'fixture-body-marker' },
	{ path: '/posts/fixture-post', status: 200, contains: '/api/v1/posts/fixture-post/assets/x.png' },
	{ path: '/posts/unknown', status: 404, contains: "This page doesn't exist." },
	{ path: '/robots.txt', status: 200, contains: `Sitemap: ${ORIGIN}/sitemap.xml` },
	{ path: '/sitemap.xml', status: 200, contains: '/posts/fixture-post?lang=zh' }
];

await waitUntilReady();
let failures = 0;
{
	const response = await fetch(`http://127.0.0.1:${METRICS_PORT}/metrics`);
	const body = await response.text();
	const ok = response.status === 200 && body.includes('snowblog_web_http_requests_total');
	console.log(
		`${ok ? 'ok  ' : 'FAIL'} :${METRICS_PORT}/metrics [${response.status}] ~ snowblog_web_http_requests_total`
	);
	if (!ok) failures += 1;
}
for (const check of checks) {
	const response = await fetch(`${ORIGIN}${check.path}`);
	const body = await response.text();
	const ok = response.status === check.status && body.includes(check.contains);
	console.log(`${ok ? 'ok  ' : 'FAIL'} ${check.path} [${response.status}] ~ ${check.contains}`);
	if (!ok) failures += 1;
}
shutdown(failures === 0 ? 0 : 1);
