<script lang="ts">
	import '../app.css';
	let { children }: { children: import('svelte').Snippet } = $props();
</script>

<svelte:head>
	<title>Titen — Self-hosted Threads Management</title>
</svelte:head>

{@render children()}
