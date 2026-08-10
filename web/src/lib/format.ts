export function formatDate(timestamp: string | null): string {
	if (!timestamp) return '';
	return timestamp.slice(0, 10);
}
