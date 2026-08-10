<script lang="ts">
	import { Badge } from "$lib/components/ui/badge";
	import type { Snippet } from "svelte";

	interface Props {
		status: string;
		children?: Snippet;
	}

	let { status, children }: Props = $props();

	type BadgeVariant = "default" | "secondary" | "destructive" | "outline";

	function mapToVariant(s: string): { variant: BadgeVariant; class: string } {
		const successVariants: Record<string, true> = {
			active: true,
			ok: true,
			valid: true,
			published: true,
			completed: true,
			positive: true,
		};
		const warningVariants: Record<string, true> = {
			suspended: true,
			expired: true,
			pending: true,
		};
		const errorVariants: Record<string, true> = {
			negative: true,
			failed: true,
		};

		if (successVariants[s]) {
			return { variant: "default", class: "bg-[var(--color-success)] text-[var(--color-success-ink)]" };
		}
		if (warningVariants[s]) {
			return { variant: "default", class: "bg-[var(--color-warning)] text-[var(--color-warning-ink)]" };
		}
		if (errorVariants[s]) {
			return { variant: "destructive", class: "" };
		}
		return { variant: "secondary", class: "" };
	}

	const mapped = $derived(mapToVariant(status));
</script>

<Badge variant={mapped.variant} class={mapped.class}>
	{#if children}
		{@render children()}
	{:else}
		{status}
	{/if}
</Badge>
