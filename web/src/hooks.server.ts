import type { Handle } from '@sveltejs/kit';

const API_INTERNAL = process.env.API_INTERNAL || 'http://localhost:7845';

export const handle: Handle = async ({ event, resolve }) => {
	const { pathname } = event.url;

	// Proxy /api/* requests to the Rust backend.
	if (pathname.startsWith('/api/')) {
		const target = `${API_INTERNAL}${pathname}${event.url.search}`;
		const headers = new Headers(event.request.headers);
		headers.set('host', new URL(API_INTERNAL).host);
		headers.delete('connection');

		try {
			const res = await fetch(target, {
				method: event.request.method,
				headers,
				body: event.request.method !== 'GET' && event.request.method !== 'HEAD'
					? await event.request.arrayBuffer()
					: undefined
			});
			return new Response(res.body, {
				status: res.status,
				headers: res.headers
			});
		} catch {
			return new Response(
				JSON.stringify({ error: 'API upstream unavailable' }),
				{ status: 502, headers: { 'content-type': 'application/json' } }
			);
		}
	}

	return resolve(event);
};
