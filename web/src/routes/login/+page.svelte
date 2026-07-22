<script lang="ts">
    import { setApiKey } from '$lib/api.js';

    let key = $state('');
    let error = $state('');
    let loading = $state(false);

    async function handleLogin(e: SubmitEvent) {
        e.preventDefault();
        error = '';
        loading = true;

        try {
            const res = await fetch('/api/accounts', {
                headers: { 'X-API-Key': key.trim() }
            });
            if (!res.ok) throw new Error('Invalid API key');
            setApiKey(key.trim());
            window.location.href = '/admin';
        } catch (err) {
            error = err instanceof Error ? err.message : 'Invalid API key';
        } finally {
            loading = false;
        }
    }
</script>

<svelte:head>
    <title>Login — titen</title>
</svelte:head>

<main class="login">
    <div class="login__card">
        <h1 class="login__wordmark">titen</h1>
        <p class="login__subtitle">Enter your API key to continue</p>
        {#if error}
            <p class="login__error">{error}</p>
        {/if}
        <form onsubmit={handleLogin}>
            <input
                type="password"
                bind:value={key}
                placeholder="API key"
                required
                autocomplete="current-password"
                disabled={loading}
            />
            <button type="submit" disabled={loading || !key.trim()}>
                {loading ? 'Verifying…' : 'Continue'}
            </button>
        </form>
    </div>
</main>

<style>
    .login {
        min-height: 100vh;
        display: flex;
        align-items: center;
        justify-content: center;
        background: var(--color-paper-2, oklch(97% 0.005 260));
    }

    .login__card {
        width: 100%;
        max-width: 22rem;
        padding: var(--space-xl, 2rem);
        border: 1px solid var(--color-rule, oklch(92% 0.005 260));
        border-radius: 8px;
        background: var(--color-paper, white);
    }

    .login__wordmark {
        font-family: var(--font-mono, monospace);
        font-size: var(--text-xl, 1.25rem);
        font-weight: 500;
        text-align: center;
        margin-bottom: var(--space-xs, 0.25rem);
    }

    .login__subtitle {
        text-align: center;
        color: var(--color-muted, oklch(55% 0.015 260));
        font-size: var(--text-sm, 0.875rem);
        margin-bottom: var(--space-lg, 1rem);
    }

    .login__error {
        color: var(--color-error, #e11d48);
        font-size: var(--text-sm, 0.875rem);
        margin-bottom: var(--space-sm, 0.5rem);
    }

    form {
        display: flex;
        flex-direction: column;
        gap: var(--space-sm, 0.5rem);
    }

    input {
        padding: 0.5rem 0.75rem;
        border: 1px solid var(--color-rule, oklch(92% 0.005 260));
        border-radius: 6px;
        font-family: var(--font-mono, monospace);
        font-size: var(--text-sm, 0.875rem);
        background: var(--color-paper, white);
        color: var(--color-ink, oklch(20% 0.015 260));
    }

    input:focus {
        outline: 2px solid oklch(55% 0.2 265);
        outline-offset: -1px;
    }

    button {
        padding: 0.5rem 1rem;
        background: var(--color-ink, oklch(20% 0.015 260));
        color: var(--color-paper, white);
        border: none;
        border-radius: 6px;
        font-size: var(--text-sm, 0.875rem);
        font-weight: 500;
        cursor: pointer;
    }

    button:hover:not(:disabled) {
        opacity: 0.85;
    }

    button:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }
</style>
