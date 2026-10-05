import { defineConfig } from 'vite';
import { resolve } from 'node:path';

export default defineConfig({
  base: './',
  server: { host: '127.0.0.1', port: 1420, strictPort: true, watch: { ignored: ['**/src-tauri/**', '**/.impeccable/**'] } },
  build: { rolldownOptions: { input: {
    pet: resolve(import.meta.dirname, 'index.html'),
    summary: resolve(import.meta.dirname, 'summary.html'),
    panel: resolve(import.meta.dirname, 'panel.html')
  } } }
});
