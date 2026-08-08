<script lang="ts">
	import PageHeader from '$lib/components/PageHeader.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import PostDetail from '$lib/components/PostDetail.svelte';
	import { listPosts, deletePost, getPostInsights, listAccounts } from '$lib/api';
	import { toast } from '$lib/toast.svelte';
	import type { Post, Account } from '$lib/types';

	let loading = $state(true);
	let posts = $state<Post[]>([]);
	let accounts = $state<Account[]>([]);
	let filterAccount = $state('');
	let filterStatus = $state('');
	let expandedPostId = $state<string | null>(null);
	let insights: Record<string, any> = $state({});
	let insightsLoading = $state<string | null>(null);
	let confirmDelete = $state<{ open: boolean; post: Post | null }>({ open: false, post: null });

	// Detail modal
	let detailPost = $state<Post | null>(null);

	function openDetail(post: Post) {
		// Enrich with account info for permalink
		const enriched = { ...post, account: accounts.find(a => a.id === post.account_id) };
		detailPost = enriched;
	}
	function closeDetail() {
		detailPost = null;
	}

	const filtered = $derived.by(() => {
		let result = posts;
		if (filterAccount) result = result.filter((p) => p.account_id === filterAccount);
		if (filterStatus) result = result.filter((p) => p.status === filterStatus);
		return result;
	});

	function formatDate(iso: string | null): string {
		if (!iso) return '—';
		return new Date(iso).toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
	}

	async function loadPosts() {
		loading = true;
		try {
			const [p, a] = await Promise.all([
				listPosts().catch(() => []),
				listAccounts().catch(() => []),
			]);
			posts = p;
			accounts = a;
		} catch (e: any) {
			toast('Failed to load posts', 'error');
		} finally {
			loading = false;
		}
	}

	function getAccountUsername(accountId: string): string {
		return accounts.find((a) => a.id === accountId)?.username ?? accountId.slice(0, 8);
	}

	async function toggleInsights(postId: string) {
		if (expandedPostId === postId) {
			expandedPostId = null;
			return;
		}
		expandedPostId = postId;
		if (!insights[postId]) {
			insightsLoading = postId;
			try {
				const res = await getPostInsights(postId);
				insights[postId] = res;
			} catch {
				toast('Failed to load insights', 'error');
				expandedPostId = null;
			} finally {
				insightsLoading = null;
			}
		}
	}

	async function handleDelete() {
		if (!confirmDelete.post) return;
		try {
			await deletePost(confirmDelete.post.id);
			toast('Post deleted', 'success');
			posts = posts.filter((p) => p.id !== confirmDelete.post!.id);
		} catch {
			toast('Failed to delete post', 'error');
		} finally {
			confirmDelete = { open: false, post: null };
		}
	}

	$effect(() => {
		loadPosts();
	});
</script>

<PageHeader title="Posts" description="Manage and monitor your Threads content.">
	{#snippet action()}
		<div style="display: flex; gap: var(--space-xs); align-items: center;">
			<select class="form-input" bind:value={filterAccount} style="width: auto; font-size: var(--text-sm);">
				<option value="">All Accounts</option>
				{#each accounts as account}
					<option value={account.id}>@{account.username}</option>
				{/each}
			</select>
			<select class="form-input" bind:value={filterStatus} style="width: auto; font-size: var(--text-sm);">
				<option value="">All Status</option>
				<option value="draft">Draft</option>
				<option value="published">Published</option>
				<option value="failed">Failed</option>
				<option value="deleted">Deleted</option>
			</select>
		</div>
	{/snippet}
</PageHeader>

<div class="data-table-wrap">
	{#if loading}
		<table class="data-table">
			<thead><tr><th>Content</th><th>Account</th><th>Type</th><th>Status</th><th>Published</th><th>Actions</th></tr></thead>
			<tbody>{#each Array(4) as _}<tr>{#each Array(6) as _}<td><div class="skeleton" style="height: 1rem;"></div></td>{/each}</tr>{/each}</tbody>
		</table>
	{:else if filtered.length === 0}
		<div class="empty-state">
			<p class="empty-state-title">No posts yet</p>
			<p class="empty-state-desc">Posts will appear here once you publish content.</p>
		</div>
	{:else}
		<table class="data-table">
			<thead>
				<tr>
					<th>Content</th>
					<th>Account</th>
					<th>Type</th>
					<th>Status</th>
					<th>Published</th>
					<th>Actions</th>
				</tr>
			</thead>
			<tbody>
				{#each filtered as post (post.id)}
					<tr class="row-clickable" onclick={() => openDetail(post)} role="button" tabindex="0" onkeydown={(e) => e.key === 'Enter' && openDetail(post)}>
						<td class="truncate" style="max-width:40ch;">
							{post.caption ? (post.caption.length > 40 ? post.caption.slice(0, 40) + '…' : post.caption) : '(no caption)'}
						</td>
						<td><span style="color:var(--color-muted);">@{getAccountUsername(post.account_id)}</span></td>
						<td>{post.media_type}</td>
						<td><StatusBadge status={post.status} /></td>
						<td><span class="tabular-nums">{formatDate(post.published_at)}</span></td>
						<td onclick={(e) => e.stopPropagation()}>
							<div style="display:flex;gap:var(--space-2xs);">
								<button class="btn-outline btn-sm" onclick={() => openDetail(post)}>Detail</button>
								<button class="btn-ghost btn-sm" onclick={() => (confirmDelete = { open: true, post })}>
									<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="width:1rem;height:1rem;color:var(--color-error);">
										<path d="M3 6h18"/><path d="M8 6V4h8v2"/><path d="M19 6l-1 14H6L5 6"/>
									</svg>
								</button>
							</div>
						</td>
					</tr>
				{/each}
			</tbody>
		</table>
	{/if}
</div>

<!-- Post Detail Modal -->
{#if detailPost}
	{@const postWithAccount = { ...detailPost, account: accounts.find(a => a.id === detailPost.account_id) }}
	<PostDetail post={postWithAccount} onClose={closeDetail} />
{/if}

<ConfirmDialog
	open={confirmDelete.open}
	title="Delete Post"
	message="This will permanently remove this post. This action cannot be undone."
	confirmLabel="Delete"
	variant="danger"
	onconfirm={handleDelete}
	oncancel={() => (confirmDelete = { open: false, post: null })}
/>

<svelte:window onkeydown={(e) => {
	if (e.key === 'Escape' && detailPost) closeDetail();
}} />

<style>
	.row-clickable {
		cursor: pointer;
		transition: background-color 0.1s ease;
	}

	.row-clickable:hover {
		background: var(--color-bg-hover, rgba(0, 0, 0, 0.03));
	}
</style>