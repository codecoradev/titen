<script lang="ts">
	import '../../app.css';
	import { getIcon } from '$lib/icons';
	import { getToasts } from '$lib/toast.svelte';
	import { page } from '$app/state';

	let { children }: { children: import('svelte').Snippet } = $props();

	const navItems = [
		{ href: '/admin/dashboard', label: 'Dashboard', icon: 'dashboard' },
		{ href: '/admin/accounts', label: 'Accounts', icon: 'accounts' },
		{ href: '/admin/posts', label: 'Posts', icon: 'posts' },
		{ href: '/admin/schedules', label: 'Schedules', icon: 'schedules' },
		{ href: '/admin/comments', label: 'Comments', icon: 'comments' },
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
		class="mobile-menu-btn btn-ghost"
		onclick={toggleSidebar}
		aria-label="Toggle menu"
	>
		<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
			{#if sidebarOpen}
				<path d="M18 6 6 18M6 6l12 12" />
			{:else}
				<path d="M3 12h18M3 6h18M3 18h18" />
			{/if}
		</svg>
	</button>

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
			v0.1.2 · admin
		</div>
	</aside>

	<!-- Main content -->
	<div class="admin-main">
		<main class="admin-content">
			{@render children()}
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
	}

	@media (max-width: 48rem) {
		.mobile-menu-btn {
			display: flex;
		}
	}
</style>