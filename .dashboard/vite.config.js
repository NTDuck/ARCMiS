import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit()],
	preview: {
		host: '127.0.0.1'
	},
	server: {
		host: '127.0.0.1'
	}
});
