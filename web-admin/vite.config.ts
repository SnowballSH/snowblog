import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vitest/config';
import adapter from '@sveltejs/adapter-node';
import { sveltekit } from '@sveltejs/kit/vite';
import wasm from 'vite-plugin-wasm';
import topLevelAwait from 'vite-plugin-top-level-await';

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
		wasm(),
		topLevelAwait()
	],
	build: {
		// vite-plugin-top-level-await falls back to a legacy multi-browser
		// esbuild target when `build.target` is unset, and current esbuild
		// fails to downlevel some destructuring patterns the plugin's TLA
		// glue code generates for that target combination. Building for
		// esnext skips that downlevel step entirely (TLA already requires a
		// modern browser).
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
