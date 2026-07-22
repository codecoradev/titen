<script lang="ts">
	type StatusVariant = 'active' | 'suspended' | 'expired' | 'published' | 'draft' | 'failed' | 'deleted' | 'pending' | 'processing' | 'completed' | 'cancelled' | 'positive' | 'negative' | 'neutral';

	interface Props {
		status: string;
	}

	let { status }: Props = $props();

	const variant = $derived(maptVariant(status));

	function maptVariant(s: string): 'success' | 'warning' | 'error' | 'info' | 'neutral' {
		const map: Record<string, 'success' | 'warning' | 'error' | 'info' | 'neutral'> = {
			active: 'success',
			published: 'success',
			completed: 'success',
			positive: 'success',
			suspended: 'warning',
			pending: 'warning',
			processing: 'info',
			draft: 'info',
			negative: 'error',
			failed: 'error',
			deleted: 'neutral',
			cancelled: 'neutral',
			expired: 'warning',
			neutral: 'neutral',
		};
		return map[s] ?? 'neutral';
	}
</script>

<span class="badge badge--{variant}">{status}</span>
