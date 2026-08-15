import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { debounce } from './debounce.js';

describe('debounce', () => {
	beforeEach(() => {
		vi.useFakeTimers();
	});

	afterEach(() => {
		vi.useRealTimers();
	});

	it('delays invocation until the wait elapses', () => {
		const fn = vi.fn();
		const debounced = debounce(fn, 900);

		debounced();
		expect(fn).not.toHaveBeenCalled();

		vi.advanceTimersByTime(899);
		expect(fn).not.toHaveBeenCalled();

		vi.advanceTimersByTime(1);
		expect(fn).toHaveBeenCalledTimes(1);
	});

	it('collapses rapid calls into a single trailing invocation with the latest arguments', () => {
		const fn = vi.fn();
		const debounced = debounce(fn, 900);

		debounced('a');
		vi.advanceTimersByTime(400);
		debounced('b');
		vi.advanceTimersByTime(400);
		debounced('c');
		vi.advanceTimersByTime(900);

		expect(fn).toHaveBeenCalledTimes(1);
		expect(fn).toHaveBeenCalledWith('c');
	});

	it('cancel prevents a pending invocation', () => {
		const fn = vi.fn();
		const debounced = debounce(fn, 900);

		debounced();
		debounced.cancel();
		vi.advanceTimersByTime(1000);

		expect(fn).not.toHaveBeenCalled();
	});

	it('allows a later call to schedule again after cancel', () => {
		const fn = vi.fn();
		const debounced = debounce(fn, 900);

		debounced();
		debounced.cancel();
		debounced();
		vi.advanceTimersByTime(900);

		expect(fn).toHaveBeenCalledTimes(1);
	});
});
