import { defineConfig } from 'vitest/config'
import vue from '@vitejs/plugin-vue'
import { fileURLToPath, URL } from 'node:url'

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url))
    }
  },
  // Force Vite to re-transform every file on each run.  Without this, the
  // dependency-pre-bundle cache (`node_modules/.vite/deps`) holds onto a
  // compiled copy of `@/i18n/index.ts` from a previous test run and our
  // edits to that file go silently ignored.
  cacheDir: '.vitest-cache',
  test: {
    cache: false,
    environment: 'happy-dom',
    globals: true,
    include: ['src/**/*.{test,spec}.{ts,js}'],
    setupFiles: ['./test/setup.ts']
  }
})