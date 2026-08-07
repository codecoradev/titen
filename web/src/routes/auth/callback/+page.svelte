<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { oauthExchange, checkSession } from '$lib/api';

	let status: 'loading' | 'success' | 'error' = 'loading';
	let errorMessage = '';

	async function handleCallback() {
		const url = new URL($page.url);
		const code = url.searchParams.get('code');
		const state_param = url.searchParams.get('state');

		console.log('[TITEN CALLBACK] invoked', { 
			hasCode: !!code, 
			codeLen: code?.length || 0,
			hasState: !!state_param,
			url: url.toString().split('?')[0],
			searchParams: url.search 
		});

		if (!code) {
			console.warn('[TITEN CALLBACK] no code param, aborting');
			status = 'error';
			errorMessage = 'No authorization code received from Threads.';
			return;
		}

		// Check if user is authenticated via session cookie
		console.log('[TITEN CALLBACK] checking session...');
		let session;
		try {
			session = await checkSession();
			console.log('[TITEN CALLBACK] session result', { 
				authenticated: session.authenticated, 
				requiresAuth: session.requires_auth,
				version: (session as Record<string, unknown>).version 
			});
		} catch (e) {
			console.error('[TITEN CALLBACK] session check threw', e);
			session = { authenticated: false, requires_auth: true };
		}

		if (!session.authenticated) {
			console.warn('[TITEN CALLBACK] NOT authenticated, redirecting to login');
			const redirect = encodeURIComponent(`/auth/callback${url.search}`);
			goto(`/login?redirect=${redirect}`);
			return;
		}

		console.log('[TITEN CALLBACK] authenticated, proceeding with token exchange');

		// Read OAuth config from localStorage (set in Settings)
		let appId = '';
		let appSecret = '';
		try {
			const s = JSON.parse(localStorage.getItem('titen-settings') || '{}');
			appId = s.threadsAppId || '';
			appSecret = s.threadsAppSecret || '';
		} catch { /* ignore */ }
		const redirectUri = localStorage.getItem('titen_oauth_redirect_uri') || `${window.location.origin}/auth/callback`;

		console.log('[TITEN CALLBACK] oauth config', { 
			hasAppId: !!appId, 
			appIdLen: appId.length,
			hasAppSecret: !!appSecret, 
			appSecretLen: appSecret.length,
			redirectUri 
		});

		if (!appId || !appSecret) {
			console.error('[TITEN CALLBACK] missing app credentials');
			status = 'error';
			errorMessage = 'Threads App ID and Secret not configured. Go to Settings to set them up.';
			return;
		}

		try {
			console.log('[TITEN CALLBACK] calling oauthExchange...');
			await oauthExchange({
				code,
				app_id: appId,
				app_secret: appSecret,
				redirect_uri: redirectUri,
			});
			console.log('[TITEN CALLBACK] oauthExchange SUCCESS');
			status = 'success';
		} catch (e: unknown) {
			console.error('[TITEN CALLBACK] oauthExchange FAILED', e);
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