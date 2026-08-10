<script lang="ts">
	import { goto } from '$app/navigation';
	import { loginWithApiKey } from '$lib/api';
	import { Button } from '$lib/components/ui/button';

	let apiKey = $state('');
	let error = $state('');
	let loading = $state(false);

	async function handleSubmit(e: Event) {
		e.preventDefault();
		error = '';
		loading = true;

		const trimmed = apiKey.trim();
		if (!trimmed) {
			error = 'API key is required.';
			loading = false;
			return;
		}

		try {
			const result = await loginWithApiKey(trimmed);
			if (!result.valid) {
				throw new Error('Invalid API key');
			}
			// Redirect to dashboard or original destination
			const params = new URLSearchParams(window.location.search);
			const redirect = params.get('redirect') || '/admin/dashboard';
			// Prevent open redirect — only allow relative paths within the app
			const safeRedirect = redirect.startsWith('/') && !redirect.startsWith('//') ? redirect : '/admin/dashboard';
			goto(safeRedirect);
		} catch {
			error = 'Invalid API key. Make sure it matches the TITEN_API_KEY on the server.';
		} finally {
			loading = false;
		}
	}
</script>

<svelte:head>
	<title>Sign in — Titen</title>
</svelte:head>

<div class="login-container">
	<div class="login-card">
		<a href="/" class="login-wordmark">titen</a>
		<h1 class="login-title">Sign in</h1>
		<p class="login-desc">
			Enter the API key configured on your Titen server
			(<code>TITEN_API_KEY</code> environment variable).
		</p>

		<form class="login-form" onsubmit={handleSubmit}>
			<div class="form-group">
				<label class="form-label" for="apiKey">API Key</label>
				<input
					class="form-input"
					id="apiKey"
					type="password"
					placeholder="titen_••••••••••••"
					bind:value={apiKey}
					autocomplete="off"
					autocapitalize="off"
					spellcheck="false"
					disabled={loading}
				/>
				{#if error}
					<span class="form-helper is-error">{error}</span>
				{:else}
					<span class="form-helper">Required to access the admin dashboard.</span>
				{/if}
			</div>

			<Button variant="default" class="login-submit" type="submit" disabled={loading}>
				{#if loading}
					<span class="login-spinner"></span>
					Verifying…
				{:else}
					Sign in
				{/if}
				</Button>
		</form>

		<div class="login-hint">
			<span>Don't have a key?</span>
			<a href="https://github.com/codecoradev/titen#configuration" target="_blank" rel="noopener noreferrer">
				Read setup guide ↗
			</a>
		</div>
	</div>
</div>

<style>
	.login-container {
		display: flex;
		align-items: center;
		justify-content: center;
		min-height: 100dvh;
		padding: var(--space-lg);
	}

	.login-card {
		display: flex;
		flex-direction: column;
		gap: var(--space-md);
		padding: var(--space-2xl);
		background: var(--surface-raised);
		border: var(--rule-default);
		border-radius: var(--radius-xl);
		max-width: 26rem;
		width: 100%;
	}

	.login-wordmark {
		font-family: var(--font-mono);
		font-size: var(--text-md);
		font-weight: 500;
		color: var(--color-ink);
		text-decoration: none;
		letter-spacing: -0.01em;
	}

	.login-title {
		font-family: var(--font-display);
		font-size: var(--text-xl);
		font-weight: 600;
		color: var(--color-ink);
	}

	.login-desc {
		font-size: var(--text-sm);
		color: var(--color-muted);
		line-height: 1.6;
	}

	.login-desc code {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		background: var(--color-paper-2);
		padding: var(--space-3xs) var(--space-2xs);
		border-radius: var(--radius-sm);
	}

	.login-form {
		display: flex;
		flex-direction: column;
		gap: var(--space-md);
	}

	.login-spinner {
		width: 1rem;
		height: 1rem;
		border: 2px solid currentColor;
		border-top-color: transparent;
		border-radius: 50%;
		animation: spin 0.7s linear infinite;
		opacity: 0.8;
	}

	@keyframes spin {
		to { transform: rotate(360deg); }
	}

	.login-hint {
		display: flex;
		align-items: center;
		gap: var(--space-xs);
		font-size: var(--text-xs);
		color: var(--color-muted);
		padding-top: var(--space-xs);
		border-top: var(--rule-default);
	}

	.login-hint a {
		color: var(--color-accent);
		text-decoration: none;
	}

	.login-hint a:hover {
		text-decoration: underline;
	}

	@media (max-width: 30rem) {
		.login-card {
			padding: var(--space-xl);
		}
	}
</style>
