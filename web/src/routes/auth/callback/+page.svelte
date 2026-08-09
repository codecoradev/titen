<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { oauthExchange, checkSession } from '$lib/api';

	let status: 'loading' | 'success' | 'error' = $state('loading');
	let errorMessage = $state('');
	let hasRun = $state(false);

	async function handleCallback() {
		if (hasRun) return;
		hasRun = true;

		const url = new URL($page.url);
		const code = url.searchParams.get('code');

		if (!code) {
			status = 'error';
			errorMessage = 'No authorization code received from Threads.';
			return;
		}

		// Check if user is authenticated via session cookie
		let session;
		try {
			session = await checkSession();
		} catch {
			session = { authenticated: false, requires_auth: true };
		}

		if (!session.authenticated) {
			const redirect = encodeURIComponent(`/auth/callback${url.search}`);
			window.location.href = `/login?redirect=${redirect}`;
			return;
		}

		// Exchange code — backend reads app_id + app_secret from DB (encrypted)
		const redirectUri = `${window.location.origin}/auth/callback`;

		try {
			await oauthExchange({
				code,
				redirect_uri: redirectUri,
			});
			status = 'success';
		} catch (e: unknown) {
			status = 'error';
			errorMessage = e instanceof Error ? e.message : 'OAuth exchange failed.';
		}
	}

	$effect(() => {
		handleCallback();
	});
</script>

<svelte:head>
	<title>Connecting Threads — Titen</title>
</svelte:head>

<div class="callback-container">
	{#if status === 'loading'}
		<div class="callback-card">
			<div class="callback-spinner"></div>
			<p class="callback-text">Connecting your Threads account...</p>
		</div>
	{:else if status === 'success'}
		<div class="callback-card">
			<svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="var(--color-success)" stroke-width="1.5">
				<path d="M20 6L9 17l-5-5" stroke-linecap="round" stroke-linejoin="round"/>
			</svg>
			<p class="callback-text">Account connected successfully</p>
			<button class="btn-primary" onclick={() => goto('/admin/accounts')}>Go to Accounts</button>
		</div>
	{:else}
		<div class="callback-card">
			<svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="var(--color-error)" stroke-width="1.5">
				<circle cx="12" cy="12" r="10"/>
				<path d="M15 9l-6 6M9 9l6 6" stroke-linecap="round"/>
			</svg>
			<p class="callback-text">{errorMessage}</p>
			<button class="btn-outline" onclick={() => goto('/admin/accounts')}>Back to Accounts</button>
			<button class="btn-outline" onclick={() => goto('/admin/settings')}>Open Settings</button>
		</div>
	{/if}
</div>

<style>
	.callback-container {
		display: flex;
		align-items: center;
		justify-content: center;
		min-height: 100dvh;
		padding: var(--space-lg);
	}

	.callback-card {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-md);
		padding: var(--space-2xl);
		background: var(--surface-raised);
		border: var(--rule-default);
		border-radius: var(--radius-xl);
		max-width: 28rem;
		text-align: center;
	}

	.callback-text {
		font-size: var(--text-sm);
		color: var(--color-muted);
		max-width: 30ch;
	}

	.callback-spinner {
		width: 2.5rem;
		height: 2.5rem;
		border: 2px solid var(--color-rule);
		border-top-color: var(--color-accent);
		border-radius: 50%;
		animation: spin 0.7s linear infinite;
	}

	@keyframes spin {
		to { transform: rotate(360deg); }
	}
</style>