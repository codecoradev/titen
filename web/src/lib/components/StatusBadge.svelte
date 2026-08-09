<script lang="ts">
	interface Props {
		status: string;
	}

	let { status }: Props = $props();

		function mapToVariant(s: string): 'success' | 'warning' | 'error' | 'info' | 'neutral' {
		const map: Record<string, 'success' | 'warning' | 'error' | 'info' | 'neutral'> = {
			active: 'success',
			ok: 'success',
			valid: 'success',
			published: 'success',
			completed: 'success',
			positive: 'success',
			suspended: 'warning',
			expired: 'warning',
			pending: 'warning',
			processing: 'info',
			draft: 'info',
			inactive: 'info',
			unknown: 'neutral',
			negative: 'error',
			failed: 'error',
			deleted: 'neutral',
			cancelled: 'neutral',
			neutral: 'neutral',
		};
		return map[s] ?? 'neutral';
	}

	const variant = $derived(mapToVariant(status));
</script>

<span class="badge badge--{variant}">{status}</span>
