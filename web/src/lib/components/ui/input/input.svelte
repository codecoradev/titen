<script lang="ts">
	import { cn, type WithElementRef, type WithoutChildren } from "$lib/utils.js";
	import type { HTMLInputAttributes } from "svelte/elements";

	let {
		ref = $bindable(null),
		value = $bindable(),
		class: className,
		"data-slot": dataSlot = "input",
		...restProps
	}: WithoutChildren<WithElementRef<HTMLInputAttributes>> = $props();
</script>

<input
	bind:this={ref}
	data-slot={dataSlot}
	class={cn(
		"rounded-lg border border-input bg-transparent px-2.5 py-2 text-base transition-colors selection:bg-primary selection:text-primary-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 disabled:bg-input/50 aria-invalid:border-destructive aria-invalid:ring-3 aria-invalid:ring-destructive/20 md:text-sm dark:bg-input/30 dark:disabled:bg-input/80 dark:aria-invalid:border-destructive/50 dark:aria-invalid:ring-destructive/40 h-9 w-full outline-none placeholder:text-muted-foreground file:h-7 file:border-0 file:bg-transparent file:font-medium disabled:cursor-not-allowed disabled:opacity-50",
		className
	)}
	bind:value
	{...restProps} />
