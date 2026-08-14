import { spawn } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { startFixtureAdminApi } from './fixture-admin-api.ts';

const PORT = Number(process.env.SMOKE_PORT ?? 4174);
const METRICS_PORT = 9464;
const ORIGIN = `http://127.0.0.1:${PORT}`;

const tokenDir = mkdtempSync(join(tmpdir(), 'snowblog-admin-smoke-'));
const tokenFile = join(tokenDir, 'token');
writeFileSync(tokenFile, 'smoke-token\n');

const api = startFixtureAdminApi();

const baseEnv = {
	...process.env,
	NODE_ENV: 'production',
	PORT: String(PORT),
	ORIGIN,
	SNOWBLOG_API_URL: `http://127.0.0.1:${api.port}`,
	SNOWBLOG_ADMIN_TOKEN_FILE: tokenFile,
	ADMIN_ALLOWED_USERS: 'smoke',
	SNOWBLOG_ADMIN_METRICS_LISTEN: `127.0.0.1:${METRICS_PORT}`
};

const server = spawn('node', ['build'], { env: baseEnv, stdio: 'inherit' });

let cleanedUp = false;
function cleanup(): void {
	if (cleanedUp) return;
	cleanedUp = true;
	server.kill();
	api.stop();
	rmSync(tokenDir, { recursive: true, force: true });
}

process.once('exit', cleanup);

function shutdown(code: number): never {
	cleanup();
	process.exit(code);
}

async function waitUntilReady(): Promise<void> {
	for (let attempt = 0; attempt < 50; attempt += 1) {
		try {
			const response = await fetch(`${ORIGIN}/healthz`, { redirect: 'manual' });
			if (response.status > 0) return;
		} catch {
			await new Promise((resolve) => setTimeout(resolve, 200));
		}
	}
	console.error('server never became ready');
	shutdown(1);
}

let failures = 0;

async function check(
	label: string,
	path: string,
	status: number,
	contains: string,
	headers?: Record<string, string>
): Promise<void> {
	const response = await fetch(`${ORIGIN}${path}`, { headers });
	const body = await response.text();
	const ok = response.status === status && body.includes(contains);
	console.log(`${ok ? 'ok  ' : 'FAIL'} ${label} [${response.status}] ~ ${contains}`);
	if (!ok) failures += 1;
}

await waitUntilReady();

await check('GET /healthz', '/healthz', 200, '"status":"ok"');
await check('GET / (no identity)', '/', 403, 'forbidden');
await check('GET / (Remote-User: smoke)', '/', 200, 'snowblog admin', {
	'Remote-User': 'smoke'
});
await check('GET / (Remote-User: other)', '/', 403, 'forbidden', {
	'Remote-User': 'other'
});

{
	const response = await fetch(`http://127.0.0.1:${METRICS_PORT}/metrics`);
	const body = await response.text();
	const ok = response.status === 200 && body.includes('snowblog_admin_');
	console.log(
		`${ok ? 'ok  ' : 'FAIL'} GET :${METRICS_PORT}/metrics [${response.status}] ~ snowblog_admin_`
	);
	if (!ok) failures += 1;
}

server.kill();

const failClosedPort = PORT + 2;
const envWithoutAllowedUsers: Record<string, string | undefined> = { ...baseEnv };
delete envWithoutAllowedUsers.ADMIN_ALLOWED_USERS;
let failClosedStderr = '';
const failClosed = spawn('node', ['build'], {
	env: { ...envWithoutAllowedUsers, PORT: String(failClosedPort) },
	stdio: ['ignore', 'ignore', 'pipe']
});
failClosed.stderr.on('data', (chunk: Buffer) => {
	failClosedStderr += chunk.toString();
});
const failClosedExit = await new Promise<number | null>((resolve) => {
	failClosed.once('exit', (code) => resolve(code));
	setTimeout(() => {
		failClosed.kill();
		resolve(null);
	}, 5000);
});
const failClosedOk =
	failClosedExit !== null &&
	failClosedExit !== 0 &&
	failClosedStderr.includes('ADMIN_ALLOWED_USERS');
console.log(
	`${failClosedOk ? 'ok  ' : 'FAIL'} node build without ADMIN_ALLOWED_USERS exits non-zero naming the variable`
);
if (!failClosedOk) failures += 1;

shutdown(failures === 0 ? 0 : 1);
