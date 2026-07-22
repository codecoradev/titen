<script lang="ts">
	interface Props {
		icon?: string;
		title: string;
		description?: string;
		action?: import('svelte').Snippet;
	}

	let { icon, title, description, action }: Props = $props();
</script>

<div class="empty-state">
	{#if icon === 'posts'}
		<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
			<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/>
		</svg>
	{:else if icon === 'accounts'}
		<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
			<path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/>
		</svg>
	{:else}
		<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
			<circle cx="12" cy="12" r="10"/><path d="M8 15h8M9 9h.01M15 9h.01"/>
		</svg>
	{/if}

	<p class="empty-state-title">{title}</p>
	{#if description}
		<p class="empty-state-desc">{description}</p>
	{/if}
	{#if action}
		<div style="margin-top: var(--space-sm);">
			{@render action()}
		</div>
	{/if}
</div>