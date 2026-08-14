import type { AdminConfig } from './config.js';

export function identityFrom(headers: Headers, config: AdminConfig): string | null {
	const value = headers.get(config.identityHeader)?.trim();
	if (!value) return null;
	return config.allowedUsers.has(value) ? value : null;
}

export function crossSiteViolation(request: Request, config: AdminConfig): string | null {
	if (request.method === 'GET' || request.method === 'HEAD') return null;

	const fetchSite = request.headers.get('sec-fetch-site');
	if (fetchSite !== null && fetchSite !== 'same-origin' && fetchSite !== 'none') {
		return `cross-site request rejected: sec-fetch-site is ${fetchSite}`;
	}

	const origin = request.headers.get('origin');
	if (origin !== null && origin !== config.origin) {
		return `cross-site request rejected: origin ${origin} does not match ${config.origin}`;
	}

	return null;
}
