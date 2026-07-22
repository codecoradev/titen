<script lang="ts">
	interface Props {
		title: string;
		description?: string;
		action?: import('svelte').Snippet;
	}
	let { title, description, action }: Props = $props();
</script>

<div class="page-header">
	<div>
		<h1>{title}</h1>
		{#if description}
			<p class="page-header-desc">{description}</p>
		{/if}
	</div>
	{#if action}
		<div class="page-header-action">
			{@render action()}
		</div>
	{/if}
</div>