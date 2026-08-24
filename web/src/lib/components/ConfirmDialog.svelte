<script lang="ts">
	import * as AlertDialog from "$lib/components/ui/alert-dialog";
	import { Button } from "$lib/components/ui/button";

	interface Props {
		open: boolean;
		title: string;
		message: string;
		confirmLabel?: string;
		variant?: "danger" | "default";
		/** Show a type-to-confirm input bound to confirmText. */
		showInput?: boolean;
		confirmText?: string;
		onconfirm: () => void;
		oncancel: () => void;
	}

	let {
		open,
		title,
		message,
		confirmLabel = "Confirm",
		variant = "danger",
		confirmText = $bindable(""),
		showInput = false,
		onconfirm,
		oncancel,
	}: Props = $props();
</script>

<AlertDialog.Root bind:open>
	<AlertDialog.Content>
		<AlertDialog.Header>
			<AlertDialog.Title>{title}</AlertDialog.Title>
			<AlertDialog.Description>{message}</AlertDialog.Description>
		</AlertDialog.Header>
		{#if showInput}
			<input
				type="text"
				class="confirm-input"
				placeholder="Type to confirm"
				bind:value={confirmText}
			/>
		{/if}
		<AlertDialog.Footer>
			<Button variant="outline" onclick={oncancel}>Cancel</Button>
			<Button
				variant={variant === "danger" ? "destructive" : "default"}
				onclick={onconfirm}
			>
				{confirmLabel}
			</Button>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>

<style>
	.confirm-input {
		width: 100%;
		margin-top: 0.75rem;
		padding: 0.5rem 0.75rem;
		border: 1px solid var(--border, #ccc);
		border-radius: 0.375rem;
		font-size: 0.875rem;
	}
</style>
