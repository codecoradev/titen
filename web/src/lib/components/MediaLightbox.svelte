<script lang="ts">
	/**
	 * Full-screen lightbox for media previews (schedules, posts, media pages).
	 * Controlled component — parent owns `open` state; Escape/backdrop/X close.
	 * Body scroll is locked while open (prevents background page scroll-jump).
	 */
	import * as Dialog from '$lib/components/ui/dialog';

	interface Props {
		url: string | null;
		alt?: string;
		onClose: () => void;
	}

	let { url, alt = 'Media preview', onClose }: Props = $props();

	// Lock body scroll while the lightbox is open. No state writes inside the
	// effect — writing tracked state here would re-trigger the effect and run
	// the cleanup, undoing the lock immediately (CodeCora scan alert #221).
	$effect(() => {
		if (!url) return;
		const prev = document.body.style.overflow;
		document.body.style.overflow = 'hidden';
		return () => {
			document.body.style.overflow = prev;
		};
	});
</script>

<Dialog.Root
	open={url !== null}
	onOpenChange={(o) => {
		if (!o) onClose();
	}}
>
	<Dialog.Content
		class="lightbox-content max-h-[92vh] max-w-[92vw] overflow-hidden border-none bg-black/90 p-1 shadow-2xl sm:max-w-[92vw]"
		aria-describedby={undefined}
	>
		{#if url}
			<img
				src={url}
				{alt}
				class="max-h-[86vh] w-auto max-w-full rounded object-contain"
			/>
		{/if}
		<Dialog.Close
			class="absolute right-2 top-2 flex size-8 items-center justify-center rounded-full bg-white/20 text-white transition-colors hover:bg-white/40"
			aria-label="Close preview"
		>
			<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="size-4">
				<path d="M18 6 6 18" /><path d="m6 6 12 12" />
			</svg>
		</Dialog.Close>
	</Dialog.Content>
</Dialog.Root>

<style>
	.lightbox-content {
		z-index: 60;
	}
</style>
