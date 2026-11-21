import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [
		tailwindcss(),
		sveltekit({
			preprocess: vitePreprocess(),
			adapter: adapter({
				pages: 'build',
				assets: 'build',
				precompress: false,
				strict: true
			})
		})
	],
	server: {
		port: 5173,
		strictPort: true
	},
	optimizeDeps: {
		// Pre-bundle the Tauri modules so dev never reloads mid-test.
		include: [
			'@tauri-apps/api/core',
			'@tauri-apps/api/event',
			'@tauri-apps/api/window'
		]
	}
});
