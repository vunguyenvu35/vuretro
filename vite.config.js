import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// The Svelte app lives in ui/; Tauri loads ui/dist (see src-tauri/tauri.conf.json).
export default defineConfig({
  root: 'ui',
  plugins: [svelte()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: { outDir: 'dist', emptyOutDir: true, target: 'safari15' },
});
