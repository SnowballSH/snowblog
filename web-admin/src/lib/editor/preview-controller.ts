export interface PreviewController<T> {
	request(key: string, onResult: (value: T) => void): void;
	reset(): void;
	dispose(): void;
}

export function createPreviewController<T>(
	run: (key: string, signal: AbortSignal) => Promise<T>
): PreviewController<T> {
	let generation = 0;
	let inFlight = false;
	let lastRequestedKey: string | null = null;
	let queuedKey: string | null = null;
	let controller: AbortController | null = null;

	async function runRequest(key: string, onResult: (value: T) => void): Promise<void> {
		const requestGeneration = generation;
		lastRequestedKey = key;
		inFlight = true;
		controller = new AbortController();
		try {
			const value = await run(key, controller.signal);
			if (requestGeneration === generation) onResult(value);
		} finally {
			inFlight = false;
			if (requestGeneration === generation && queuedKey !== null) {
				const next = queuedKey;
				queuedKey = null;
				void runRequest(next, onResult);
			}
		}
	}

	return {
		request(key, onResult) {
			if (key === lastRequestedKey) return;
			if (inFlight) {
				queuedKey = key;
				return;
			}
			void runRequest(key, onResult);
		},
		reset() {
			generation += 1;
			inFlight = false;
			lastRequestedKey = null;
			queuedKey = null;
			controller?.abort();
			controller = null;
		},
		dispose() {
			controller?.abort();
		}
	};
}
