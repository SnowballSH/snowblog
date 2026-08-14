import { describe, expect, it } from 'vitest';
import { crossSiteViolation, identityFrom } from './guard.js';

const config = {
	apiUrl: 'http://a',
	adminToken: 't',
	identityHeader: 'remote-user',
	allowedUsers: new Set(['admin']),
	origin: 'https://admin.example.test',
	metricsListen: null
};

describe('identityFrom', () => {
	it('accepts an allowlisted user', () => {
		expect(identityFrom(new Headers({ 'remote-user': 'admin' }), config)).toBe('admin');
	});
	it('rejects a missing header', () => {
		expect(identityFrom(new Headers(), config)).toBeNull();
	});
	it('rejects a non-allowlisted user', () => {
		expect(identityFrom(new Headers({ 'remote-user': 'family' }), config)).toBeNull();
	});
});

function post(headers: Record<string, string>): Request {
	return new Request('https://admin.example.test/', { method: 'POST', headers });
}

describe('crossSiteViolation', () => {
	it('allows same-origin fetch metadata', () => {
		expect(
			crossSiteViolation(
				post({ 'sec-fetch-site': 'same-origin', origin: 'https://admin.example.test' }),
				config
			)
		).toBeNull();
	});
	it('rejects cross-site fetch metadata', () => {
		expect(crossSiteViolation(post({ 'sec-fetch-site': 'cross-site' }), config)).toMatch(
			/sec-fetch-site/i
		);
	});
	it('rejects an origin mismatch', () => {
		expect(crossSiteViolation(post({ origin: 'https://evil.test' }), config)).toMatch(/origin/i);
	});
	it('ignores GET', () => {
		const request = new Request('https://admin.example.test/', { method: 'GET' });
		expect(crossSiteViolation(request, config)).toBeNull();
	});
});
