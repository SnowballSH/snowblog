export interface AdminConfig {
	apiUrl: string;
	adminToken: string;
	identityHeader: string;
	allowedUsers: ReadonlySet<string>;
	origin: string;
	metricsListen: string | null;
}

function required(env: Record<string, string | undefined>, key: string): string {
	const value = env[key]?.trim();
	if (!value) throw new Error(`${key} is required`);
	return value;
}

export function loadConfig(
	env: Record<string, string | undefined>,
	readFile: (path: string) => string
): AdminConfig {
	const tokenFile = required(env, 'SNOWBLOG_ADMIN_TOKEN_FILE');
	const adminToken = readFile(tokenFile).trim();
	if (!adminToken) throw new Error(`admin token file ${tokenFile} is empty`);
	const allowedUsers = new Set(
		required(env, 'ADMIN_ALLOWED_USERS')
			.split(',')
			.map((user) => user.trim())
			.filter(Boolean)
	);
	if (allowedUsers.size === 0) throw new Error('ADMIN_ALLOWED_USERS names no users');
	return {
		apiUrl: required(env, 'SNOWBLOG_API_URL').replace(/\/$/, ''),
		adminToken,
		identityHeader: (env.ADMIN_IDENTITY_HEADER?.trim() || 'Remote-User').toLowerCase(),
		allowedUsers,
		origin: required(env, 'ORIGIN'),
		metricsListen: env.SNOWBLOG_ADMIN_METRICS_LISTEN?.trim() || null
	};
}

let active: AdminConfig | null = null;

export function initConfig(config: AdminConfig): void {
	active = config;
}

export function getConfig(): AdminConfig {
	if (!active) throw new Error('configuration is not initialized');
	return active;
}
