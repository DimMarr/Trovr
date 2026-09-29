/// <reference types="vitest/config" />
import path from 'node:path'

import tailwindcss from '@tailwindcss/vite'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: { '@': path.resolve(import.meta.dirname, 'src') },
  },
  server: {
    // Same origin as the API in development: no CORS needed.
    proxy: { '/api': 'http://localhost:8080' },
  },
  test: {
    environment: 'jsdom',
    setupFiles: ['src/test/setup.ts'],
  },
})
