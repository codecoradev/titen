<script lang="ts">
	import PageHeader from '$lib/components/PageHeader.svelte';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import { listMedia, uploadMedia, deleteMedia } from '$lib/api';
	import type { MediaItem } from '$lib/types';
	import { formatDateTime } from '$lib/tz';
	import { toast } from '$lib/toast.svelte';
	import { Button } from '$lib/components/ui/button';
	import * as Table from '$lib/components/ui/table';
	import * as Dialog from '$lib/components/ui/dialog';
	import Skeleton from '$lib/components/ui/skeleton/skeleton.svelte';
	import Upload from '@lucide/svelte/icons/upload';
	import Trash2 from '@lucide/svelte/icons/trash-2';
	import X from '@lucide/svelte/icons/x';
	import FileUp from '@lucide/svelte/icons/file-up';
	import Copy from '@lucide/svelte/icons/copy';

	let media = $state<MediaItem[]>([]);
	let loading = $state(true);
	let loaded = $state(false);
	let deleteTarget = $state<MediaItem | null>(null);
	let previewItem = $state<MediaItem | null>(null);
	let isDragging = $state(false);
	let uploading = $state(false);

	// P5.2: Pagination state
	const PAGE_SIZE = 20;
	let currentPage = $state(0);
	let totalCount = $state(0);
	let hasMore = $state(false);

	let totalPages = $derived(Math.max(1, Math.ceil(totalCount / PAGE_SIZE)));

	function formatSize(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`;
		return `${(bytes / 1048576).toFixed(1)} MB`;
	}

	function formatDate(iso: string): string {
		return formatDateTime(iso);
	}

	/// Check if a media item is an image (for preview/lightbox support).
	function isImage(contentType: string): boolean {
		return contentType.startsWith('image/');
	}

	/// Check if a media item is a video.
	function isVideo(contentType: string): boolean {
		return contentType.startsWith('video/');
	}

	async function loadMedia() {
		try {
			const res = await listMedia({
				limit: PAGE_SIZE,
				offset: currentPage * PAGE_SIZE,
			});
			media = res.data;
			totalCount = res.pagination.total;
			hasMore = res.pagination.has_more;
		} catch (e: any) {
			toast(e.message || 'Failed to load media', 'error');
		}
	}

	function goToPage(page: number) {
		if (page < 0 || page >= totalPages) return;
		currentPage = page;
		loading = true;
		loadMedia().then(() => {
			loading = false;
		});
	}

	async function handleUpload(e: Event) {
		const input = e.target as HTMLInputElement;
		const files = input.files;
		if (!files || files.length === 0) return;
		await uploadFiles(Array.from(files));
		input.value = '';
	}

	async function uploadFiles(files: File[]) {
		uploading = true;
		// P5.6: Parallel upload — Promise.allSettled for concurrent transfers
		// Limit concurrency to 3 to avoid overwhelming the server
		const CONCURRENCY = 3;
		const results: { success: number; failed: number } = { success: 0, failed: 0 };

		for (let i = 0; i < files.length; i += CONCURRENCY) {
			const batch = files.slice(i, i + CONCURRENCY);
			const settled = await Promise.allSettled(batch.map((f) => uploadMedia(f)));
			for (const r of settled) {
				if (r.status === 'fulfilled') results.success++;
				else results.failed++;
			}
		}

		uploading = false;
		if (results.success > 0) {
			toast(`Uploaded ${results.success} file${results.success > 1 ? 's' : ''}`, 'success');
			await loadMedia();
		}
		if (results.failed > 0) {
			toast(`${results.failed} file${results.failed > 1 ? 's' : ''} failed to upload`, 'error');
		}
	}

	function handleDrop(e: DragEvent) {
		e.preventDefault();
		isDragging = false;
		const files = e.dataTransfer?.files;
		if (!files || files.length === 0) return;
		uploadFiles(Array.from(files));
	}

	function handleDragOver(e: DragEvent) {
		e.preventDefault();
		isDragging = true;
	}

	function handleDragLeave(e: DragEvent) {
		e.preventDefault();
		isDragging = false;
	}

	async function handleDelete() {
		const target = deleteTarget;
		if (!target) return;
		try {
			await deleteMedia(target.id);
			toast('Media deleted', 'success');
			media = media.filter((m) => m.id !== target.id);
		} catch (e: any) {
			toast(e.message || 'Delete failed', 'error');
		} finally {
			deleteTarget = null;
		}
	}

	/// Copy media URL to clipboard.
	async function copyUrl(url: string) {
		try {
			await navigator.clipboard.writeText(url);
			toast('URL copied to clipboard', 'success');
		} catch {
			toast('Failed to copy URL', 'error');
		}
	}

	$effect(() => {
		if (loaded) return;
		loadMedia().then(() => {
			loading = false;
			loaded = true;
		});
	});
</script>

<PageHeader title="Media" description="Manage uploaded media files">
	{#snippet action()}
		<Button size="sm" class="relative cursor-pointer" disabled={uploading}>
			<Upload class="size-4" />
			{uploading ? 'Uploading...' : 'Upload'}
			<input
				type="file"
				multiple
				class="absolute inset-0 size-full cursor-pointer opacity-0"
				onchange={handleUpload}
				disabled={uploading}
			/>
		</Button>
	{/snippet}
</PageHeader>

<!-- Drag-and-drop zone -->
<div
	class="dropzone mb-4"
	class:dragging={isDragging}
	ondrop={handleDrop}
	ondragover={handleDragOver}
	ondragleave={handleDragLeave}
	role="region"
	aria-label="File upload drop zone"
	tabindex="0"
>
	<FileUp class="size-8 text-muted-foreground" />
	<p class="text-sm text-muted-foreground mt-2">
		{isDragging ? 'Drop files here' : 'Drag and drop files here, or use the Upload button'}
	</p>
</div>

<div class="data-table-wrap">
	{#if loading}
		<Table.Root>
			<Table.Header>
				<Table.Row>
					<Table.Head>Preview</Table.Head>
					<Table.Head>Filename</Table.Head>
					<Table.Head class="hidden md:table-cell">Type</Table.Head>
					<Table.Head class="hidden md:table-cell">Size</Table.Head>
					<Table.Head class="hidden md:table-cell">Uploaded</Table.Head>
					<Table.Head>Actions</Table.Head>
				</Table.Row>
			</Table.Header>
			<Table.Body>
				{#each Array(5) as _}
					<Table.Row>
						<Table.Cell><Skeleton class="size-10 rounded" /></Table.Cell>
						<Table.Cell><Skeleton class="h-4 w-full" /></Table.Cell>
						<Table.Cell><Skeleton class="h-4 w-full" /></Table.Cell>
						<Table.Cell><Skeleton class="h-4 w-full" /></Table.Cell>
						<Table.Cell><Skeleton class="h-4 w-full" /></Table.Cell>
						<Table.Cell><Skeleton class="h-4 w-full" /></Table.Cell>
					</Table.Row>
				{/each}
			</Table.Body>
		</Table.Root>
	{:else if media.length === 0}
		<div class="empty-state">
			<p class="empty-state-title">No media files</p>
			<p class="empty-state-desc">Upload images, videos, or other media to your S3 storage.</p>
		</div>
	{:else}
		<Table.Root>
			<Table.Header>
				<Table.Row>
					<Table.Head>Preview</Table.Head>
					<Table.Head>Filename</Table.Head>
					<Table.Head class="hidden md:table-cell">Type</Table.Head>
					<Table.Head class="hidden md:table-cell">Size</Table.Head>
					<Table.Head class="hidden md:table-cell">Uploaded</Table.Head>
					<Table.Head>Actions</Table.Head>
				</Table.Row>
			</Table.Header>
			<Table.Body>
				{#each media as item (item.id)}
					<Table.Row>
						<Table.Cell>
							{#if isImage(item.content_type)}
								<button
									type="button"
									class="cursor-pointer"
									onclick={() => (previewItem = item)}
									aria-label="Preview {item.filename}"
								>
									<img
										src={item.s3_url || ''}
										alt={item.filename}
										class="size-10 rounded object-cover transition-opacity hover:opacity-80"
										loading="lazy"
										onerror={(e) => {
											const t = e.currentTarget as HTMLImageElement;
											t.style.opacity = '0';
											t.style.minHeight = '80px';
											t.alt = 'Failed to load image';
										}}
									/>
								</button>
							{:else if isVideo(item.content_type)}
								<div
									class="flex size-10 items-center justify-center rounded bg-muted text-xs font-medium"
								>
									VIDEO
								</div>
							{:else}
								<div
									class="flex size-10 items-center justify-center rounded bg-muted text-xs font-medium"
								>
									FILE
								</div>
							{/if}
						</Table.Cell>
						<Table.Cell
							class="max-w-[12rem] overflow-hidden text-ellipsis whitespace-nowrap"
							title={item.filename}>{item.filename}</Table.Cell
						>
						<Table.Cell class="hidden md:table-cell">{item.content_type}</Table.Cell>
						<Table.Cell class="hidden md:table-cell">{formatSize(item.size_bytes)}</Table.Cell>
						<Table.Cell class="hidden md:table-cell">{formatDate(item.uploaded_at)}</Table.Cell>
						<Table.Cell>
							<div class="flex gap-1">
								<Button
									variant="ghost"
									size="sm"
									onclick={() => copyUrl(item.s3_url)}
									title="Copy URL"
								>
									<Copy class="size-4" />
								</Button>
								<Button
									variant="ghost"
									size="sm"
									aria-label={`Delete ${item.filename}`}
									title={`Delete ${item.filename}`}
									onclick={() => (deleteTarget = item)}
								>
									<Trash2 class="size-4 text-destructive" />
								</Button>
							</div>
						</Table.Cell>
					</Table.Row>
				{/each}
			</Table.Body>
		</Table.Root>
	{/if}
</div>

<!-- P5.2: Pagination controls -->
{#if !loading && totalCount > PAGE_SIZE}
	<div class="flex items-center justify-between px-1 py-3">
		<p class="text-sm text-muted-foreground">
			Showing {currentPage * PAGE_SIZE + 1}–{Math.min((currentPage + 1) * PAGE_SIZE, totalCount)} of {totalCount}
		</p>
		<div class="flex gap-2">
			<Button size="sm" variant="outline" disabled={currentPage === 0} onclick={() => goToPage(currentPage - 1)}>
				Previous
			</Button>
			<span class="flex items-center px-2 text-sm text-muted-foreground">
				Page {currentPage + 1} of {totalPages}
			</span>
			<Button size="sm" variant="outline" disabled={!hasMore} onclick={() => goToPage(currentPage + 1)}>
				Next
			</Button>
		</div>
	</div>
{/if}

<!-- Lightbox / Preview Dialog -->
{#if previewItem}
	<Dialog.Root
		open={true}
		onOpenChange={(v) => {
			if (!v) previewItem = null;
		}}
	>
		<Dialog.Content class="max-w-3xl p-2 sm:p-4">
		{#if previewItem}
			<div class="relative flex items-center justify-center">
				{#if isImage(previewItem.content_type)}
					<img
						src={previewItem.s3_url}
						alt={previewItem.filename}
						class="max-h-[70vh] w-auto rounded-lg object-contain"
					/>
				{:else if isVideo(previewItem.content_type)}
					<video
						src={previewItem.s3_url}
						controls
						class="max-h-[70vh] w-auto rounded-lg"
					/>
				{/if}
				<Dialog.Close
					class="absolute right-2 top-2 rounded-full bg-background/80 p-2 backdrop-blur-sm hover:bg-background"
				>
					<X class="size-5" />
				</Dialog.Close>
			</div>
			<div class="mt-3 flex items-center justify-between gap-4 px-2 pb-1">
				<div class="min-w-0">
					<p class="truncate text-sm font-medium">{previewItem.filename}</p>
					<p class="text-xs text-muted-foreground">
						{previewItem.content_type} · {formatSize(previewItem.size_bytes)}
					</p>
				</div>
				<Button size="sm" variant="outline" onclick={() => copyUrl(previewItem!.s3_url)}>
					Copy URL
				</Button>
			</div>
		{/if}
	</Dialog.Content>
	</Dialog.Root>
{/if}

<ConfirmDialog
	open={deleteTarget !== null}
	title="Delete Media"
	message={deleteTarget
		? `Are you sure you want to delete "${deleteTarget.filename}"? This cannot be undone.`
		: ''}
	confirmLabel="Delete"
	variant="danger"
	onconfirm={handleDelete}
	oncancel={() => (deleteTarget = null)}
/>

<style>
	.dropzone {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 0.25rem;
		padding: 2rem;
		border: 2px dashed hsl(var(--border));
		border-radius: var(--radius, 0.5rem);
		background: hsl(var(--muted) / 0.3);
		transition: border-color 0.2s ease, background-color 0.2s ease, box-shadow 0.2s ease;
		cursor: default;
		outline: none;
	}

	.dropzone.dragging {
		border-color: hsl(var(--primary));
		background: hsl(var(--primary) / 0.05);
		transform: scale(1.01);
	}

	.dropzone:focus-visible {
		border-color: hsl(var(--ring));
		box-shadow: 0 0 0 2px hsl(var(--ring) / 0.3);
	}
</style>
