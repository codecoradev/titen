<script lang="ts">
	import PageHeader from '$lib/components/PageHeader.svelte';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import { listMedia, uploadMedia, deleteMedia } from '$lib/api';
	import type { MediaItem } from '$lib/types';
	import { formatDateTime } from '$lib/tz';
	import { toast } from '$lib/toast.svelte';
	import { Button } from '$lib/components/ui/button';
	import * as Table from '$lib/components/ui/table';
	import Skeleton from '$lib/components/ui/skeleton/skeleton.svelte';
	import Upload from '@lucide/svelte/icons/upload';
	import Trash2 from '@lucide/svelte/icons/trash-2';

	let media = $state<MediaItem[]>([]);
	let loading = $state(true);
	let loaded = $state(false);
	let deleteTarget = $state<MediaItem | null>(null);

	function formatSize(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`;
		return `${(bytes / 1048576).toFixed(1)} MB`;
	}

	function formatDate(iso: string): string {
		return formatDateTime(iso);
	}

	async function loadMedia() {
		try {
			media = await listMedia();
		} catch (e: any) {
			toast(e.message || 'Failed to load media', 'error');
		}
	}

	async function handleUpload(e: Event) {
		const input = e.target as HTMLInputElement;
		const file = input.files?.[0];
		if (!file) return;
		try {
			await uploadMedia(file);
			toast(`Uploaded ${file.name}`, 'success');
			await loadMedia();
		} catch (err: any) {
			toast(err.message || 'Upload failed', 'error');
		}
		input.value = '';
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
		<Button size="sm" class="relative cursor-pointer">
			<Upload class="size-4" />
			Upload
			<input
				type="file"
				class="absolute inset-0 size-full cursor-pointer opacity-0"
				onchange={handleUpload}
			/>
		</Button>
	{/snippet}
</PageHeader>

<div class="data-table-wrap">
	{#if loading}
		<Table.Root>
			<Table.Header>
				<Table.Row>
					<Table.Head>Preview</Table.Head>
					<Table.Head>Filename</Table.Head>
					<Table.Head>Type</Table.Head>
					<Table.Head>Size</Table.Head>
					<Table.Head>Uploaded</Table.Head>
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
					<Table.Head>Type</Table.Head>
					<Table.Head>Size</Table.Head>
					<Table.Head>Uploaded</Table.Head>
					<Table.Head>Actions</Table.Head>
				</Table.Row>
			</Table.Header>
			<Table.Body>
				{#each media as item (item.id)}
					<Table.Row>
						<Table.Cell>
							<img
								src={item.s3_url || ''}
								alt={item.filename}
								class="size-10 rounded object-cover"
								onerror={(e) => { const t = e.currentTarget as HTMLImageElement; t.style.opacity = '0'; t.style.minHeight = '80px'; t.alt = 'Failed to load image'; }}
							/>
						</Table.Cell>
						<Table.Cell class="max-w-[12rem] overflow-hidden text-ellipsis whitespace-nowrap" title={item.filename}>{item.filename}</Table.Cell>
						<Table.Cell>{item.content_type}</Table.Cell>
						<Table.Cell>{formatSize(item.size_bytes)}</Table.Cell>
						<Table.Cell>{formatDate(item.uploaded_at)}</Table.Cell>
						<Table.Cell>
							<Button variant="ghost" size="sm" onclick={() => (deleteTarget = item)}>
								<Trash2 class="size-4 text-destructive" />
							</Button>
						</Table.Cell>
					</Table.Row>
				{/each}
			</Table.Body>
		</Table.Root>
	{/if}
</div>

<ConfirmDialog
	open={deleteTarget !== null}
	title="Delete Media"
	message={deleteTarget ? `Are you sure you want to delete "${deleteTarget.filename}"? This cannot be undone.` : ''}
	confirmLabel="Delete"
	variant="danger"
	onconfirm={handleDelete}
	oncancel={() => (deleteTarget = null)}
/>
