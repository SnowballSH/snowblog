import { describe, expect, it } from 'vitest';
import { loadConfig } from './config.js';

const env = {
	SNOWBLOG_API_URL: 'http://api.internal:8080',
	SNOWBLOG_ADMIN_TOKEN_FILE: '/run/token',
	ADMIN_ALLOWED_USERS: 'alice, bob',
	ORIGIN: 'https://admin.example.test'
};
const readFile = () => 'secret-token\n';

describe('loadConfig', () => {
	it('loads a complete configuration', () => {
		const config = loadConfig(env, readFile);
		expect(config.apiUrl).toBe('http://api.internal:8080');
		expect(config.adminToken).toBe('secret-token');
		expect(config.identityHeader).toBe('remote-user');
		expect(config.allowedUsers).toEqual(new Set(['alice', 'bob']));
		expect(config.origin).toBe('https://admin.example.test');
		expect(config.metricsListen).toBeNull();
	});
	it('honors ADMIN_IDENTITY_HEADER lowercased', () => {
		const config = loadConfig({ ...env, ADMIN_IDENTITY_HEADER: 'X-Auth-User' }, readFile);
		expect(config.identityHeader).toBe('x-auth-user');
	});
	for (const key of [
		'SNOWBLOG_API_URL',
		'SNOWBLOG_ADMIN_TOKEN_FILE',
		'ADMIN_ALLOWED_USERS',
		'ORIGIN'
	]) {
		it(`throws when ${key} is missing`, () => {
			expect(() => loadConfig({ ...env, [key]: undefined }, readFile)).toThrow(key);
		});
	}
	it('throws on an empty token file', () => {
		expect(() => loadConfig(env, () => '  \n')).toThrow('token');
	});
	it('throws on an empty allowlist', () => {
		expect(() => loadConfig({ ...env, ADMIN_ALLOWED_USERS: ' , ' }, readFile)).toThrow(
			'ADMIN_ALLOWED_USERS'
		);
	});
});
