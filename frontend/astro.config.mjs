// @ts-check
import { defineConfig } from 'astro/config';

// https://astro.build/config
export default defineConfig({
  vite: {
    build: {
      // Enable manual module resolution for external dependencies
      rollupOptions: {
        external: ['easymde']
      }
    },
    optimizeDeps: {
      include: ['marked', 'katex'],
      exclude: ['easymde']
    },
    // Enable direct import from node_modules in browser
    server: {
      fs: {
        allow: ['..']
      }
    }
  }
});
