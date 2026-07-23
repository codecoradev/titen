<script lang="ts">
	interface Props {
		label?: string;
	}

	let { label = 'Loading' }: Props = $props();
</script>

<div class="stat-card skeleton-wrap">
	<div class="skeleton" style="height: 0.6rem; width: 5rem;"></div>
	<div class="skeleton" style="height: 1.5rem; width: 3rem; margin-top: var(--space-2xs);"></div>
</div>