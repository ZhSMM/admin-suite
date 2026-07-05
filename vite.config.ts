import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { fileURLToPath, URL } from 'node:url'
import { readFileSync } from 'node:fs'

// Tauri expects a fixed dev port and disabled HMR overlay on errors.
const pkg = JSON.parse(
  readFileSync(new URL('./package.json', import.meta.url), 'utf-8')
)

export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: '127.0.0.1',
    watch: {
      // Don't watch src-tauri to avoid rebuild loops.
      ignored: ['**/src-tauri/**']
    }
  },
  envPrefix: ['VITE_', 'TAURI_'],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url))
    }
  },
  define: {
    // Inlined at build time so the bundle is self-describing without a runtime
    // fetch. Used by the crash store and the auto-update banner.
    __APP_VERSION__: JSON.stringify(pkg.version)
  },
  build: {
    target: 'es2021',
    minify: 'esbuild',
    sourcemap: false
  },
  css: {
    preprocessorOptions: {
      scss: {
        api: 'modern-compiler'
      }
    }
  }
})