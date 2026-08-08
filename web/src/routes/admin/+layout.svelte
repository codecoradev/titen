<script lang="ts">
	import '../../app.css';
	import { getIcon } from '$lib/icons';
	import { getToasts } from '$lib/toast.svelte';
	import { page } from '$app/state';
	import { checkSession, logout } from '$lib/api';
	import { goto } from '$app/navigation';

	let { children }: { children: import('svelte').Snippet } = $props();

	// Auth guard: verify session cookie is valid, redirect to login if not
	let authed = $state(false);

	$effect(() => {
		if (authed) return;
		(async () => {
			try {
				const session = await checkSession();
				if (session.authenticated) {
					authed = true;
				} else {
					// Not authenticated — redirect to login
					const currentPath = page.url.pathname + page.url.search;
					goto(`/login?redirect=${encodeURIComponent(currentPath)}`);
				}
			} catch {
				// Network error — redirect to login as safe fallback
				const currentPath = page.url.pathname + page.url.search;
				goto(`/login?redirect=${encodeURIComponent(currentPath)}`);
			}
		})();
	});

	async function handleLogout() {
		await logout();
		goto('/login');
	}

	const navItems = [
		{ href: '/admin/dashboard', label: 'Dashboard', icon: 'dashboard' },
		{ href: '/admin/accounts', label: 'Accounts', icon: 'accounts' },
		{ href: '/admin/posts', label: 'Posts', icon: 'posts' },
		{ href: '/admin/schedules', label: 'Schedules', icon: 'schedules' },
		{ href: '/admin/comments', label: 'Comments', icon: 'comments' },
		{ href: '/admin/mentions', label: 'Mentions', icon: 'comments' },
		{ href: '/admin/analytics', label: 'Analytics', icon: 'analytics' },
		{ href: '/admin/media', label: 'Media', icon: 'media' },
		{ href: '/admin/settings', label: 'Settings', icon: 'settings' },
	] as const;

	let sidebarOpen = $state(false);
		function isActive(href: string): boolean {
		return page.url.pathname.startsWith(href);
	}

	function toggleSidebar() {
		sidebarOpen = !sidebarOpen;
	}

	function closeSidebar() {
		sidebarOpen = false;
	}
</script>

<svelte:head>
	<title>Titen Admin</title>
</svelte:head>



<div class="admin-shell">
	<!-- Mobile hamburger -->
	<button
		class="mobile-menu-btn"
		onclick={toggleSidebar}
		aria-label="Toggle menu"
		aria-expanded={sidebarOpen}
	>
		<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
			{#if sidebarOpen}
				<path d="M18 6 6 18M6 6l12 12" />
			{:else}
				<path d="M3 12h18M3 6h18M3 18h18" />
			{/if}
		</svg>
	</button>

	<!-- Mobile sidebar backdrop -->
	{#if sidebarOpen}
		<button
			class="sidebar-backdrop"
			onclick={closeSidebar}
			aria-label="Close menu"
			tabindex="-1"
		></button>
	{/if}

	<!-- Sidebar -->
	<aside class="admin-sidebar" class:is-open={sidebarOpen} role="navigation" aria-label="Admin navigation">
		<div class="sidebar-wordmark">
			<a href="/admin/dashboard" style="text-decoration:none;color:inherit;">Titen</a>
		</div>
		<nav class="sidebar-nav">
			{#each navItems as item}
				<a
					href={item.href}
					class="sidebar-link"
					aria-current={isActive(item.href) ? 'page' : undefined}
					onclick={closeSidebar}
				>
					{@html getIcon(item.icon)}
					{item.label}
				</a>
			{/each}
		</nav>
		<div class="sidebar-footer">
			<button class="sidebar-logout" onclick={handleLogout}>Sign out</button>
			<span class="sidebar-version">v0.4.0 · admin</span>
		</div>
	</aside>

	<!-- Main content -->
	<div class="admin-main">
		<main class="admin-content">
			{#if authed}
				{@render children()}
			{:else}
				<div class="auth-loading">Verifying session…</div>
			{/if}
		</main>
	</div>
</div>

<!-- Toast container -->
<div class="toast-container">
	{#each getToasts().toasts as t (t.id)}
		<div class="toast toast--{t.type}">
			{t.message}
		</div>
	{/each}
</div>

<style>
	.mobile-menu-btn {
		display: none;
		position: fixed;
		inset-block-start: var(--space-sm);
		inset-inline-start: var(--space-sm);
		z-index: calc(var(--z-modal) + 1);
		width: 2.5rem;
		height: 2.5rem;
		align-items: center;
		justify-content: center;
		background: var(--surface-raised);
		border: var(--rule-default);
		border-radius: var(--radius-md);
		color: var(--color-ink);
		box-shadow: var(--shadow-whisper);
	}

	.mobile-menu-btn:hover {
		background: var(--surface-sunken);
	}

	.mobile-menu-btn svg {
		width: 1.25rem;
		height: 1.25rem;
	}

	.sidebar-backdrop {
		display: none;
		position: fixed;
		inset: 0;
		background: oklch(0% 0 0 / 0.4);
		z-index: var(--z-modal);
		border: none;
		cursor: pointer;
		-webkit-tap-highlight-color: transparent;
	}

	@media (max-width: 48rem) {
		.mobile-menu-btn {
			display: flex;
		}

		.sidebar-backdrop {
			display: block;
		}
	}

	.sidebar-footer {
		display: flex;
		flex-direction: column;
		gap: var(--space-2xs);
		align-items: flex-start;
	}

	.sidebar-logout {
		background: none;
		border: none;
		color: var(--color-muted);
		font-size: var(--text-xs);
		font-family: var(--font-mono);
		cursor: pointer;
		padding: var(--space-3xs) 0;
		transition: color 0.15s ease;
	}

	.sidebar-logout:hover {
		color: var(--color-error);
	}

	.sidebar-version {
		font-size: var(--text-xs);
		color: var(--color-muted);
		font-family: var(--font-mono);
	}

	.auth-loading {
		display: flex;
		align-items: center;
		justify-content: center;
		min-height: 50vh;
		color: var(--color-muted);
		font-size: var(--text-sm);
	}
</style>