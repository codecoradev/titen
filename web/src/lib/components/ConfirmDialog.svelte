<script lang="ts">
	interface Props {
		open: boolean;
		title: string;
		message: string;
		confirmLabel?: string;
		variant?: 'danger' | 'default';
		onconfirm: () => void;
		oncancel: () => void;
	}

	let {
		open,
		title,
		message,
		confirmLabel = 'Confirm',
		variant = 'danger',
		onconfirm,
		oncancel,
	}: Props = $props();

	function handleKey(e: KeyboardEvent) {
		if (e.key === 'Escape' && open) oncancel();
	}
</script>

<svelte:window onkeydown={handleKey} />

{#if open}
	<div class="confirm-overlay" onclick={oncancel} role="dialog" aria-modal="true" aria-label={title}>
		<div class="confirm-dialog" onclick={(e) => e.stopPropagation()}>
			<h3>{title}</h3>
			<p>{message}</p>
			<div class="confirm-actions">
				<button class="btn-outline btn-sm" onclick={oncancel}>Cancel</button>
				<button
					class="btn-primary btn-sm"
					class:btn-danger={variant === 'danger'}
					onclick={onconfirm}
				>
					{confirmLabel}
				</button>
			</div>
		</div>
	</div>
{/if}
