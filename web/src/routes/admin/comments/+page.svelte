<script lang="ts">
	import PageHeader from '$lib/components/PageHeader.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import { listComments, fetchComments, listPosts } from '$lib/api';
	import type { Comment, Post } from '$lib/types';
	import { toast } from '$lib/toast.svelte';

	let comments = $state<Comment[]>([]);
	let posts = $state<Post[]>([]);
	let loading = $state(true);
	let fetchLoading = $state(false);
	let filterPostId = $state('');
	let showFetchModal = $state(false);
	let fetchPostId = $state('');


	let filtered = $derived(
		filterPostId ? comments.filter((c: Comment) => c.post_id === filterPostId) : comments,
	);

	let sentimentSummary = $derived({
		positive: filtered.filter((c: Comment) => c.sentiment === 'positive').length,
		negative: filtered.filter((c: Comment) => c.sentiment === 'negative').length,
		neutral: filtered.filter((c: Comment) => c.sentiment === 'neutral').length,
	});

	function truncate(text: string, max: number): string {
		return text.length > max ? text.slice(0, max) + '…' : text;
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

	async function loadComments() {
		try {
			const res = await listComments();
			comments = res.data;
		} catch (e: any) {
			toast(e.message || 'Failed to load comments', 'error');
		}
	}

	async function loadPosts() {
		try {
			const res = await listPosts();
			posts = res.data;
		} catch (e: any) {
			toast(e.message || 'Failed to load posts', 'error');
		}
	}

	async function handleFetch() {
		if (!fetchPostId) return;
		fetchLoading = true;
		try {
			const res = await fetchComments(fetchPostId);
			comments = res.data;
			showFetchModal = false;
			fetchPostId = '';
			toast(`Fetched ${res.data.length} comments`, 'success');
		} catch (e: any) {
			toast(e.message || 'Failed to fetch comments', 'error');
		} finally {
			fetchLoading = false;
		}
	}

	async function init() {
		loading = true;
		await Promise.all([loadComments(), loadPosts()]);
		loading = false;
	}

	$effect(() => {
		init();
	});
</script>

<PageHeader title="Comments" description="Post comments and sentiment analysis">
	{#snippet action()}
		<button class="btn-primary btn-sm" onclick={() => (showFetchModal = true)}>Fetch New</button>
	{/snippet}
</PageHeader>

{#if filtered.length > 0}
	<div class="sentiment-bar">
		<span class="sentiment-bar__label">Sentiment:</span>
		<span class="badge badge--success">{sentimentSummary.positive} positive</span>
		<span class="badge badge--error">{sentimentSummary.negative} negative</span>
		<span class="badge badge--neutral">{sentimentSummary.neutral} neutral</span>
	</div>
{/if}

{#if posts.length > 0}
	<div class="filter-row">
		<label for="post-filter" class="filter-label">Filter by post:</label>
		<select id="post-filter" class="select" bind:value={filterPostId}>
			<option value="">All posts</option>
			{#each posts as post}
				<option value={post.id}>{post.id.slice(0, 8)}… — {truncate(post.caption || '(no caption)', 40)}</option>
			{/each}
		</select>
	</div>
{/if}

<div class="data-table-wrap">
	{#if loading}
		<table class="data-table">
			<thead>
				<tr>
					<th>Author</th>
					<th>Comment</th>
					<th>Sentiment</th>
					<th>Score</th>
					<th>Fetched</th>
				</tr>
			</thead>
			<tbody>
				{#each Array(5) as _}
					<tr>
						<td><div class="skeleton" style="height: 1rem;"></div></td>
						<td><div class="skeleton" style="height: 1rem;"></div></td>
						<td><div class="skeleton" style="height: 1rem;"></div></td>
						<td><div class="skeleton" style="height: 1rem;"></div></td>
						<td><div class="skeleton" style="height: 1rem;"></div></td>
					</tr>
				{/each}
			</tbody>
		</table>
	{:else if filtered.length === 0}
		<div class="empty-state">
			<p class="empty-state-title">No comments yet</p>
			<p class="empty-state-desc">Click "Fetch New" to pull comments from a post.</p>
		</div>
	{:else}
		<table class="data-table">
			<thead>
				<tr>
					<th>Author</th>
					<th>Comment</th>
					<th>Sentiment</th>
					<th>Score</th>
					<th>Fetched</th>
				</tr>
			</thead>
			<tbody>
				{#each filtered as comment (comment.id)}
					<tr>
						<td>{comment.author_username}</td>
						<td class="comment-text-cell" title={comment.text}>{truncate(comment.text, 80)}</td>
						<td><StatusBadge status={comment.sentiment} /></td>
						<td>{comment.sentiment_score !== null ? comment.sentiment_score.toFixed(2) : '—'}</td>
						<td>{formatDate(comment.fetched_at)}</td>
					</tr>
				{/each}
			</tbody>
		</table>
	{/if}
</div>

{#if showFetchModal}
	<div class="confirm-overlay" onclick={() => (showFetchModal = false)} role="dialog" aria-modal="true" aria-label="Fetch comments">
		<div class="confirm-dialog" onclick={(e) => e.stopPropagation()}>
			<h3>Fetch Comments</h3>
			<p>Select a post to fetch comments for.</p>
			<select class="select" bind:value={fetchPostId} disabled={fetchLoading}>
				<option value="" disabled>Select a post…</option>
				{#each posts as post}
					<option value={post.id}>{post.id.slice(0, 8)}… — {truncate(post.caption || '(no caption)', 50)}</option>
				{/each}
			</select>
			<div class="confirm-actions">
				<button class="btn-outline btn-sm" onclick={() => (showFetchModal = false)}>Cancel</button>
				<button class="btn-primary btn-sm" onclick={handleFetch} disabled={!fetchPostId || fetchLoading}>
					{fetchLoading ? 'Fetching…' : 'Fetch Comments'}
				</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.comment-text-cell {
		max-width: 20rem;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.sentiment-bar {
		display: flex;
		align-items: center;
		gap: var(--space-xs);
		margin-bottom: var(--space-sm);
	}

	.sentiment-bar__label {
		font-size: var(--text-sm);
		color: var(--color-muted);
		font-weight: 500;
	}

	.filter-row {
		display: flex;
		align-items: center;
		gap: var(--space-xs);
		margin-bottom: var(--space-md);
	}

	.filter-label {
		font-size: var(--text-sm);
		color: var(--color-muted);
	}
</style>
