<script lang="ts">
	import PageHeader from '$lib/components/PageHeader.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
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
					<tr>
						<td class="truncate" style="max-width:40ch;">
							<button class="btn-ghost" style="text-align:left;padding:0;" onclick={() => toggleInsights(post.id)}>
								{post.caption ? (post.caption.length > 40 ? post.caption.slice(0, 40) + '…' : post.caption) : '(no caption)'}
							</button>
						</td>
						<td><span style="color:var(--color-muted);">@{getAccountUsername(post.account_id)}</span></td>
						<td>{post.media_type}</td>
						<td><StatusBadge status={post.status} /></td>
						<td><span class="tabular-nums">{formatDate(post.published_at)}</span></td>
						<td>
							<div style="display:flex;gap:var(--space-2xs);">
								<button class="btn-outline btn-sm" onclick={() => toggleInsights(post.id)}>
									{insightsLoading === post.id ? '…' : 'Insights'}
								</button>
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

<!-- Insights panel -->
{#if expandedPostId && insights[expandedPostId]}
	<div class="insights-panel" style="margin-top: var(--space-md);">
		<h3 style="font-size: var(--text-sm); font-weight: 600; margin-bottom: var(--space-sm);">Insights</h3>
		<div class="stat-grid" style="grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));">
			{#each [
				{ label: 'Likes', value: insights[expandedPostId]?.likes ?? 0 },
				{ label: 'Replies', value: insights[expandedPostId]?.replies ?? 0 },
				{ label: 'Reposts', value: insights[expandedPostId]?.reposts ?? 0 },
				{ label: 'Views', value: insights[expandedPostId]?.views ?? 0 },
				{ label: 'Quotes', value: insights[expandedPostId]?.quotes ?? 0 },
			] as stat}
				<div class="stat-card">
					<div class="stat-card-label">{stat.label}</div>
					<div class="stat-card-value tabular-nums">{stat.value.toLocaleString()}</div>
				</div>
			{/each}
		</div>
	</div>
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