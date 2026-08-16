export interface EditorFields {
	source: string;
	title: string;
	description: string;
}

export function isDirty(baseline: EditorFields, current: EditorFields): boolean {
	return (
		baseline.source !== current.source ||
		baseline.title !== current.title ||
		baseline.description !== current.description
	);
}
