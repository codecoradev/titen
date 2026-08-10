<script lang="ts">
	import Skeleton from "$lib/components/ui/skeleton/skeleton.svelte";

	interface Props {
		label?: string;
	}

	let { label = "Loading" }: Props = $props();
</script>

<div class="stat-card">
	<Skeleton class="h-2.4 w-20" />
	<Skeleton class="h-6 w-12 mt-1" />
</div>
