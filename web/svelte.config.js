import adapter from '@sveltejs/adapter-node';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	kit: {
		// Node adapter: SSR via Bun runtime. Generates build/server/index.js.
		adapter: adapter({
			precompress: true
		})
	}
};

export default config;
