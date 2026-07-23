<script lang="ts">
	import PageHeader from '$lib/components/PageHeader.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import { listComments, fetchComments, listPosts } from '$lib/api';
	import type { Comment, Post } from '$lib/types';
	import { toast } from '$lib/toast.svelte';

	let comments = $state<Comment[]>([]);
	let posts = $state<Post[]>([]);
	let loading = $state(true);
	let commentsLoading = $state(false);
	let fetchLoading = $state(false);
	let selectedPostId = $state('');

	let sentimentSummary = $derived({
		positive: comments.filter((c) => c.sentiment === 'positive').length,
		negative: comments.filter((c) => c.sentiment === 'negative').length,
		neutral: comments.filter((c) => c.sentiment === 'neutral').length,
	});

	function truncate(text: string, max: number): string {
		return text.length > max ? text.slice(0, max) + '\u2026' : text;
	}

	function formatDate(iso: string): string {
		return new Date(iso).toLocaleDateString('en-US', {
			month: 'short',
			hour: '2-digit',
			year: 'numeric',
		});
	}

	async function loadPosts() {
		try {
			posts = await listPosts().catch(() => []);
			if (posts.length > 0 && !selectedPostId) {
				selectedPostId = posts[0].id;
			}
		} catch (e: any) {
			toast(e.message || 'Failed to load posts', 'error');
		}
	}

	async function loadComments() {
		if (!selectedPostId) return;
		commentsLoading = true;
		try {
			comments = await listComments(selectedPostId);
		} catch (e: any) {
			toast(e.message || 'Failed to load comments', 'error');
			comments = [];
		} finally {
			commentsLoading = false;
		}
	}

	async function handleFetch() {
		if (!selectedPostId) return;
		fetchLoading = true;
		try {
			comments = await fetchComments(selectedPostId);
			toast(`Fetched ${comments.length} comments`, 'success');
		} catch (e: any) {
			toast(e.message || 'Failed to fetch comments', 'error');
		} finally {
			fetchLoading = false;
		}
	}

	$effect(() => {
		if (selectedPostId) loadComments();
	});

	async function init() {
		loading = true;
		await loadPosts();
		loading = false;
	}

	$effect(() => {
		init();
	});
</script>

<PageHeader title="Comments" description="Post comments and sentiment analysis">
	{#snippet action()}
		<button class="btn-primary btn-sm" onclick={handleFetch} disabled={!selectedPostId || fetchLoading}>
			{fetchLoading ? 'Fetching...' : 'Fetch New'}
		</button>
	{/snippet}
</PageHeader>

{#if posts.length > 0}
<div class="filter-row">
	<label for="post-filter" class="filter-label">Post:</label>
	<select id="post-filter" class="select" bind:value={selectedPostId}>
		{#each posts as post}
			<option value={post.id}>{post.id.slice(0, 8)}... {truncate(post.caption || '(no caption)', 40)}</option>
		{/each}
	</select>
</div>
{/if}

{#if comments.length > 0}
<div class="sentiment-bar">
	<span class="sentiment-bar__label">Sentiment:</span>
	<span class="badge badge--success">{sentimentSummary.positive} positive</span>
	<span class="badge badge--error">{sentimentSummary.negative} negative</span>
	<span class="badge badge--neutral">{sentimentSummary.neutral} neutral</span>
</div>
{/if}

<div class="data-table-wrap">
	{#if loading || commentsLoading}
	<table class="data-table">
		<thead><tr><th>Author</th><th>Comment</th><th>Sentiment</th><th>Score</th><th>Fetched</th></tr></thead>
		<tbody>{#each Array(5) as _}<tr>{#each Array(5) as _}<td><div class="skeleton" style="height: 1rem;"></div></td>{/each}</tr>{/each}</tbody>
	</table>
	{:else if comments.length === 0}
	<div class="empty-state">
		<p class="empty-state-title">No comments yet</p>
		<p class="empty-state-desc">Select a post above, then click "Fetch New" to pull comments.</p>
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
			{#each comments as comment (comment.id)}
				<tr>
					<td>{comment.author_username ?? 'anonymous'}</td>
					<td class="comment-text-cell" title={comment.text}>{truncate(comment.text, 80)}</td>
					<td><StatusBadge status={comment.sentiment ?? 'neutral'} /></td>
					<td>{comment.sentiment_score !== null ? comment.sentiment_score.toFixed(2) : '---'}</td>
					<td>{formatDate(comment.fetched_at)}</td>
				</tr>
			{/each}
		</tbody>
	</table>
	{/if}
</div>

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
