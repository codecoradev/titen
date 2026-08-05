import type { Handle } from '@sveltejs/kit';

const API_INTERNAL = process.env.API_INTERNAL || 'http://api:7845';

/**
 * SvelteKit server hook — proxy /api/* and /health to the Rust backend.
 *
 * The web container (SvelteKit SSR) is the single public entry point.
 * API requests are forwarded to the api container on the Docker internal
 * network via API_INTERNAL (default: http://api:7845).
 *
 * Cookies (titen_session) are forwarded intentionally — the API uses
 * them for session auth alongside X-API-Key header auth.
 */
export const handle: Handle = async ({ event, resolve }) => {
	const { pathname } = event.url;

	// Proxy /api/* and /health requests to the Rust backend.
	if (pathname.startsWith('/api/') || pathname === '/health') {
		const target = `${API_INTERNAL}${pathname}${event.url.search}`;
		const headers = new Headers(event.request.headers);
		headers.set('host', new URL(API_INTERNAL).host);
		// Forward cookies for session auth (intentional — API checks titen_session).
		// Remove hop-by-hop headers per RFC 7230.
		headers.delete('connection');

		let body: ArrayBuffer | undefined;
		if (event.request.method !== 'GET' && event.request.method !== 'HEAD') {
			try {
				body = await event.request.arrayBuffer();
			} catch {
				return new Response(
					JSON.stringify({ error: 'Failed to read request body' }),
					{ status: 400, headers: { 'content-type': 'application/json' } }
				);
			}
		}

		try {
			const res = await fetch(target, {
				method: event.request.method,
				redirect: 'manual',
				headers,
				body
			});
			// Strip hop-by-hop headers from the upstream response.
			const resHeaders = new Headers(res.headers);
			resHeaders.delete('connection');
			resHeaders.delete('keep-alive');
			resHeaders.delete('transfer-encoding');
			return new Response(res.body, {
				status: res.status,
				headers: resHeaders
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
