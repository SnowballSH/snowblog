import { ulid } from 'ulid';

export interface AuditEntry {
	audit_id: string;
	user: string;
	action: string;
	slug: string | null;
	outcome: 'ok' | 'failed' | 'conflict';
	status: number;
}

export function recordAudit(
	entry: Omit<AuditEntry, 'audit_id'>,
	write: (line: string) => void = console.log
): string {
	const audit_id = ulid();
	const auditEntry: AuditEntry = {
		...entry,
		audit_id
	};

	write(JSON.stringify({ audit: auditEntry }));
	return audit_id;
}
