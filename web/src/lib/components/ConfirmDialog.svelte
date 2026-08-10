<script lang="ts">
	import * as AlertDialog from "$lib/components/ui/alert-dialog";
	import { Button } from "$lib/components/ui/button";

	interface Props {
		open: boolean;
		title: string;
		message: string;
		confirmLabel?: string;
		variant?: "danger" | "default";
		onconfirm: () => void;
		oncancel: () => void;
	}

	let {
		open,
		title,
		message,
		confirmLabel = "Confirm",
		variant = "danger",
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
