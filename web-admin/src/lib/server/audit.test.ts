import { describe, it, expect } from 'vitest';
import { recordAudit } from './audit';

describe('recordAudit', () => {
	it('writes one line with audit_id, rounds trips all fields, and returns matching id', () => {
		const lines: string[] = [];
		const capture = (line: string) => lines.push(line);

		const returnedId = recordAudit(
			{
				user: 'a',
				action: 'publish',
				slug: 's',
				outcome: 'ok',
				status: 200
			},
			capture
		);

		expect(lines).toHaveLength(1);

		const parsed = JSON.parse(lines[0]);
		const { audit: entry } = parsed;

		expect(entry.audit_id).toMatch(/^[0-9A-HJKMNP-TV-Z]{26}$/);
		expect(entry.user).toBe('a');
		expect(entry.action).toBe('publish');
		expect(entry.slug).toBe('s');
		expect(entry.outcome).toBe('ok');
		expect(entry.status).toBe(200);

		expect(returnedId).toBe(entry.audit_id);
	});
});
