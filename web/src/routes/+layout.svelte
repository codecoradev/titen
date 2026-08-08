<script lang="ts">
	import '../app.css';
	import { onMount } from 'svelte';
	import { fetchTimezone } from '$lib/tz';

	let { children }: { children: import('svelte').Snippet } = $props();

	// Prefetch timezone from backend on app load
	onMount(() => {
		fetchTimezone();
	});
</script>

<svelte:head>
	<title>Titen — Self-hosted Threads Management</title>
</svelte:head>

{@render children()}
