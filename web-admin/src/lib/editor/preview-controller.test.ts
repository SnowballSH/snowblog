import { describe, expect, it, vi } from 'vitest';
import { createPreviewController } from './preview-controller.js';

function deferred<T>(): {
	promise: Promise<T>;
	resolve: (value: T) => void;
	reject: (error: unknown) => void;
} {
	let resolve!: (value: T) => void;
	let reject!: (error: unknown) => void;
	const promise = new Promise<T>((res, rej) => {
		resolve = res;
		reject = rej;
	});
	return { promise, resolve, reject };
}

describe('createPreviewController', () => {
	it('runs the fetcher and delivers the result', async () => {
		const run = vi.fn(async (key: string) => `result:${key}`);
		const onResult = vi.fn();
		const controller = createPreviewController(run);

		controller.request('a', onResult);
		await Promise.resolve();
		await Promise.resolve();

		expect(run).toHaveBeenCalledTimes(1);
		expect(run).toHaveBeenCalledWith('a', expect.any(AbortSignal));
		expect(onResult).toHaveBeenCalledWith('result:a');
	});

	it('skips a request for the same key that was just delivered', async () => {
		const run = vi.fn(async (key: string) => `result:${key}`);
		const onResult = vi.fn();
		const controller = createPreviewController(run);

		controller.request('a', onResult);
		await Promise.resolve();
		await Promise.resolve();
		controller.request('a', onResult);
		await Promise.resolve();
		await Promise.resolve();

		expect(run).toHaveBeenCalledTimes(1);
		expect(onResult).toHaveBeenCalledTimes(1);
	});

	it('queues the latest key while a request is in flight and runs it once the first settles', async () => {
		const first = deferred<string>();
		const run = vi.fn().mockReturnValueOnce(first.promise).mockResolvedValueOnce('result:c');
		const onResult = vi.fn();
		const controller = createPreviewController(run);

		controller.request('a', onResult);
		controller.request('b', onResult);
		controller.request('c', onResult);
		expect(run).toHaveBeenCalledTimes(1);

		first.resolve('result:a');
		await Promise.resolve();
		await Promise.resolve();
		await Promise.resolve();

		expect(run).toHaveBeenCalledTimes(2);
		expect(run).toHaveBeenLastCalledWith('c', expect.any(AbortSignal));
		expect(onResult).toHaveBeenNthCalledWith(1, 'result:a');
		expect(onResult).toHaveBeenNthCalledWith(2, 'result:c');
	});

	it('discards a response that resolves after reset() bumped the generation', async () => {
		const pending = deferred<string>();
		const run = vi.fn(() => pending.promise);
		const onResult = vi.fn();
		const controller = createPreviewController(run);

		controller.request('stale-language-content', onResult);
		expect(run).toHaveBeenCalledTimes(1);

		controller.reset();
		pending.resolve('result:stale');
		await Promise.resolve();
		await Promise.resolve();

		expect(onResult).not.toHaveBeenCalled();
	});

	it('lets a request started after reset() through normally', async () => {
		const run = vi.fn(async (key: string) => `result:${key}`);
		const onResult = vi.fn();
		const controller = createPreviewController(run);

		controller.reset();
		controller.request('fresh', onResult);
		await Promise.resolve();
		await Promise.resolve();

		expect(onResult).toHaveBeenCalledWith('result:fresh');
	});

	it('aborts the in-flight signal on reset()', () => {
		let capturedSignal: AbortSignal | undefined;
		const run = vi.fn((_key: string, signal: AbortSignal) => {
			capturedSignal = signal;
			return new Promise<string>(() => {});
		});
		const controller = createPreviewController(run);

		controller.request('a', vi.fn());
		expect(capturedSignal?.aborted).toBe(false);

		controller.reset();
		expect(capturedSignal?.aborted).toBe(true);
	});

	it('aborts the in-flight signal on dispose()', () => {
		let capturedSignal: AbortSignal | undefined;
		const run = vi.fn((_key: string, signal: AbortSignal) => {
			capturedSignal = signal;
			return new Promise<string>(() => {});
		});
		const controller = createPreviewController(run);

		controller.request('a', vi.fn());
		controller.dispose();

		expect(capturedSignal?.aborted).toBe(true);
	});

	it('does not throw when reset() or dispose() run with nothing in flight', () => {
		const controller = createPreviewController(vi.fn(async () => 'x'));
		expect(() => controller.reset()).not.toThrow();
		expect(() => controller.dispose()).not.toThrow();
	});
});
