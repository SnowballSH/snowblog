import { describe, expect, it } from 'vitest';
import { formatDate } from './format.js';

describe('formatDate', () => {
	it('renders the calendar date of an RFC 3339 timestamp', () => {
		expect(formatDate('2026-08-01T00:00:00Z')).toBe('2026-08-01');
	});

	it('renders nothing for an unset date', () => {
		expect(formatDate(null)).toBe('');
	});
});
