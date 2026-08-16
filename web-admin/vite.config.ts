import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vitest/config';
import adapter from '@sveltejs/adapter-node';
import { sveltekit } from '@sveltejs/kit/vite';
import wasm from 'vite-plugin-wasm';

export default defineConfig({
	plugins: [
		tailwindcss(),
		sveltekit({
			compilerOptions: {
				runes: ({ filename }) =>
					filename.split(/[/\\]/).includes('node_modules') ? undefined : true
			},
			adapter: adapter(),
			paths: { base: '/blogs' }
		}),
		wasm()
	],
	build: {
		// vite-plugin-wasm compiles the Typst grammar's `.wasm` import into a
		// top-level-await module. Targeting esnext emits that TLA natively, so we
		// don't need vite-plugin-top-level-await — whose esbuild shim breaks under
		// bun's container install (`virtualModule.require` is undefined). TLA
		// already implies a modern browser, which this admin tool targets.
		target: 'esnext'
	},
	server: {
		proxy: process.env.SNOWBLOG_API_URL
			? { '/api/v1': { target: process.env.SNOWBLOG_API_URL, changeOrigin: true } }
			: undefined
	},
	ssr: {
		noExternal: ['ulid']
	},
	test: {
		expect: { requireAssertions: true },
		environment: 'node',
		include: ['src/**/*.{test,spec}.{js,ts}']
	}
});
