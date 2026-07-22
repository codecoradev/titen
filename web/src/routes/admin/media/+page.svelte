<script lang="ts">
	import PageHeader from '$lib/components/PageHeader.svelte';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import { listMedia, uploadMedia, deleteMedia } from '$lib/api';
	import type { MediaItem } from '$lib/types';
	import { toast } from '$lib/toast.svelte';

	let media = $state<MediaItem[]>([]);
	let loading = $state(true);
	let deleteTarget = $state<MediaItem | null>(null);

	function formatSize(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`;
		return `${(bytes / 1048576).toFixed(1)} MB`;
	}

	function formatDate(iso: string): string {
		return new Date(iso).toLocaleDateString('en-US', {
			month: 'short',
			day: 'numeric',
			year: 'numeric',
			hour: '2-digit',
			minute: '2-digit',
		});
	}

	async function loadMedia() {
		try {
			const res = await listMedia();
			media = res.data;
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
		loadMedia().then(() => {
			loading = false;
		});
	});
</script>

<PageHeader title="Media" description="Manage uploaded media files">
	{#snippet action()}
		<label class="btn-primary btn-sm upload-label">
			Upload
			<input
				type="file"
				class="media-upload-input"
				onchange={handleUpload}
			/>
		</label>
	{/snippet}
</PageHeader>

<div class="data-table-wrap">
	{#if loading}
		<table class="data-table">
			<thead>
				<tr>
					<th>Preview</th>
					<th>Filename</th>
					<th>Type</th>
					<th>Size</th>
					<th>Uploaded</th>
					<th>Actions</th>
				</tr>
			</thead>
			<tbody>
				{#each Array(5) as _}
						<tr>
							<td><div class="skeleton" style="height: 2.5rem; width: 2.5rem;"></div></td>
							<td><div class="skeleton" style="height: 1rem;"></div></td>
							<td><div class="skeleton" style="height: 1rem;"></div></td>
							<td><div class="skeleton" style="height: 1rem;"></div></td>
							<td><div class="skeleton" style="height: 1rem;"></div></td>
							<td><div class="skeleton" style="height: 1rem;"></div></td>
						</tr>
				{/each}
			</tbody>
		</table>
	{:else if media.length === 0}
		<div class="empty-state">
			<p class="empty-state-title">No media files</p>
			<p class="empty-state-desc">Upload images, videos, or other media to your S3 storage.</p>
		</div>
	{:else}
		<table class="data-table">
			<thead>
				<tr>
					<th>Preview</th>
					<th>Filename</th>
					<th>Type</th>
					<th>Size</th>
					<th>Uploaded</th>
					<th>Actions</th>
				</tr>
			</thead>
			<tbody>
				{#each media as item (item.id)}
						<tr>
							<td>
								<img
									src={item.s3_url}
									alt={item.filename}
									class="media-thumb"
								/>
							</td>
							<td class="media-filename-cell" title={item.filename}>{item.filename}</td>
							<td>{item.content_type}</td>
							<td>{formatSize(item.size_bytes)}</td>
							<td>{formatDate(item.uploaded_at)}</td>
							<td>
								<button
									class="btn-ghost btn-sm"
									onclick={() => (deleteTarget = item)}
								>
									<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="width:1rem;height:1rem;color:var(--color-error);">
										<path d="M3 6h18"/><path d="M8 6V4h8v2"/><path d="M19 6l-1 14H6L5 6"/>
									</svg>
								</button>
							</td>
						</tr>
				{/each}
			</tbody>
		</table>
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

<style>
	.upload-label {
		cursor: pointer;
	}

	.media-upload-input {
		position: absolute;
		width: 0;
		height: 0;
		opacity: 0;
		pointer-events: none;
	}

	.media-thumb {
		width: 2.5rem;
		height: 2.5rem;
		object-fit: cover;
		border-radius: var(--radius-sm);
	}

	.media-filename-cell {
		max-width: 12rem;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>