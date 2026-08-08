<script lang="ts">
	import PageHeader from '$lib/components/PageHeader.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import {
		listComments,
		fetchComments,
		listPosts,
		updateCommentReply,
		replyToComment,
	} from '$lib/api';
	import type { Comment, Post } from '$lib/types';
	import { toast } from '$lib/toast.svelte';

	let comments = $state<Comment[]>([]);
	let posts = $state<Post[]>([]);
	let loading = $state(true);
	let commentsLoading = $state(false);
	let fetchLoading = $state(false);
	let selectedPostId = $state('');
	let replyStatusFilter = $state(''); // '' = all
	let sentimentFilter = $state(''); // '' = all
	let searchQuery = $state('');

	// Reply workflow state
	let replyingTo = $state<string | null>(null);
	let replyText = $state('');
	let replyLoading = $state(false);
	let actionLoading = $state<string | null>(null); // commentId being updated

	let filteredComments = $derived(
		comments.filter((c) => {
			if (replyStatusFilter && c.reply_status !== replyStatusFilter) return false;
			if (sentimentFilter && c.sentiment !== sentimentFilter) return false;
			if (searchQuery && !c.text.toLowerCase().includes(searchQuery.toLowerCase()))
				return false;
			return true;
		}),
	);

	let sentimentSummary = $derived({
		positive: comments.filter((c) => c.sentiment === 'positive').length,
		negative: comments.filter((c) => c.sentiment === 'negative').length,
		neutral: comments.filter((c) => c.sentiment === 'neutral').length,
	});

	let replyStatusSummary = $derived({
		new: comments.filter((c) => c.reply_status === 'new').length,
		needs_reply: comments.filter((c) => c.reply_status === 'needs_reply').length,
		replied: comments.filter((c) => c.reply_status === 'replied').length,
		skipped: comments.filter((c) => c.reply_status === 'skipped').length,
	});

	function truncate(text: string, max: number): string {
		return text.length > max ? text.slice(0, max) + '\u2026' : text;
	}

	function formatDate(iso: string): string {
		return new Date(iso).toLocaleDateString('en-US', {
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit',
			year: 'numeric',
		});
	}

	const REPLY_STATUS_COLORS: Record<string, string> = {
		new: 'badge--neutral',
		needs_reply: 'badge--warning',
		replied: 'badge--success',
		skipped: 'badge--muted',
	};

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

	async function handleStatusChange(commentId: string, status: string) {
		actionLoading = commentId;
		try {
			const updated = await updateCommentReply(commentId, { reply_status: status });
			const idx = comments.findIndex((c) => c.id === commentId);
			if (idx >= 0) comments[idx] = updated;
			toast(`Marked as ${status}`, 'success');
		} catch (e: any) {
			toast(e.message || 'Failed to update status', 'error');
		} finally {
			actionLoading = null;
		}
	}

	function startReply(commentId: string) {
		replyingTo = commentId;
		const existing = comments.find((c) => c.id === commentId)?.reply_text;
		replyText = existing ?? '';
	}

	function cancelReply() {
		replyingTo = null;
		replyText = '';
	}

	async function handleReply(commentId: string) {
		if (!replyText.trim()) {
			toast('Reply text cannot be empty', 'error');
			return;
		}
		replyLoading = true;
		try {
			const result = await replyToComment(commentId, replyText.trim());
			const idx = comments.findIndex((c) => c.id === commentId);
			if (idx >= 0) comments[idx] = result.data;
			toast('Reply published to Threads', 'success');
			cancelReply();
		} catch (e: any) {
			toast(e.message || 'Failed to publish reply', 'error');
		} finally {
			replyLoading = false;
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

<PageHeader title="Comments" description="Comment inbox with reply workflow & sentiment analysis">
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
<!-- Summary bar: sentiment + reply status counts -->
<div class="summary-bar">
	<div class="summary-group">
		<span class="summary-label">Sentiment:</span>
		<span class="badge badge--success">{sentimentSummary.positive}+</span>
		<span class="badge badge--error">{sentimentSummary.negative}-</span>
		<span class="badge badge--neutral">{sentimentSummary.neutral}~</span>
	</div>
	<div class="summary-group">
		<span class="summary-label">Reply status:</span>
		<span class="badge badge--neutral">{replyStatusSummary.new} new</span>
		<span class="badge badge--warning">{replyStatusSummary.needs_reply} pending</span>
		<span class="badge badge--success">{replyStatusSummary.replied} done</span>
		<span class="badge badge--muted">{replyStatusSummary.skipped} skip</span>
	</div>
</div>

<!-- Filter bar -->
<div class="filter-bar">
	<div class="filter-group">
		<label class="filter-label">Status:</label>
		<select class="select select--sm" bind:value={replyStatusFilter}>
			<option value="">All</option>
			<option value="new">New</option>
			<option value="needs_reply">Needs Reply</option>
			<option value="replied">Replied</option>
			<option value="skipped">Skipped</option>
		</select>
	</div>
	<div class="filter-group">
		<label class="filter-label">Sentiment:</label>
		<select class="select select--sm" bind:value={sentimentFilter}>
			<option value="">All</option>
			<option value="positive">Positive</option>
			<option value="negative">Negative</option>
			<option value="neutral">Neutral</option>
		</select>
	</div>
	<div class="filter-group filter-group--grow">
		<input
			type="text"
			class="input input--sm"
			placeholder="Search comments..."
			bind:value={searchQuery}
		/>
	</div>
</div>
{/if}

<div class="comments-list">
	{#if loading || commentsLoading}
		<div class="comment-card comment-card--skeleton">
			{#each Array(5) as _}
				<div class="skeleton" style="height: 4rem; margin-bottom: var(--space-sm);"></div>
			{/each}
		</div>
	{:else if comments.length === 0}
		<div class="empty-state">
			<p class="empty-state-title">No comments yet</p>
			<p class="empty-state-desc">Select a post above, then click "Fetch New" to pull comments.</p>
		</div>
	{:else if filteredComments.length === 0}
		<div class="empty-state">
			<p class="empty-state-title">No comments match your filters</p>
			<p class="empty-state-desc">Try adjusting the filters above.</p>
		</div>
	{:else}
		{#each filteredComments as comment (comment.id)}
			<div class="comment-card" class:comment-card--replied={comment.reply_status === 'replied'}>
				<div class="comment-header">
					<div class="comment-author">
						<span class="comment-author-name">{comment.author_username ?? 'anonymous'}</span>
						<span class="comment-date">{formatDate(comment.fetched_at)}</span>
					</div>
					<div class="comment-badges">
						<StatusBadge status={comment.sentiment ?? 'neutral'} />
						<span class="badge {REPLY_STATUS_COLORS[comment.reply_status] ?? 'badge--neutral'}">
							{comment.reply_status.replace('_', ' ')}
						</span>
					</div>
				</div>

				<p class="comment-text">{comment.text}</p>

				{#if comment.reply_text}
					<div class="comment-reply-preview">
						<span class="reply-label">Reply:</span>
						<span class="reply-text">{truncate(comment.reply_text, 120)}</span>
					</div>
				{/if}

				{#if comment.sentiment_score !== null}
					<div class="comment-score">
						Score: {comment.sentiment_score.toFixed(2)}
					</div>
				{/if}

				<!-- Reply input (inline) -->
				{#if replyingTo === comment.id}
					<div class="reply-box">
						<textarea
							class="textarea textarea--sm"
							placeholder="Type your reply..."
							bind:value={replyText}
							rows="2"
						></textarea>
						<div class="reply-actions">
							<button
								class="btn-primary btn-sm"
								onclick={() => handleReply(comment.id)}
								disabled={replyLoading || !replyText.trim()}
							>
								{replyLoading ? 'Publishing...' : 'Publish Reply'}
							</button>
							<button class="btn-ghost btn-sm" onclick={cancelReply} disabled={replyLoading}>
								Cancel
							</button>
						</div>
					</div>
				{:else}
					<!-- Action buttons -->
					<div class="comment-actions">
						{#if comment.reply_status !== 'replied' && comment.threads_comment_id}
							<button
								class="btn-ghost btn-sm"
								onclick={() => startReply(comment.id)}
								disabled={actionLoading === comment.id}
							>
								Reply
							</button>
						{/if}
						{#if comment.reply_status !== 'needs_reply'}
							<button
								class="btn-ghost btn-sm"
								onclick={() => handleStatusChange(comment.id, 'needs_reply')}
								disabled={actionLoading === comment.id}
							>
								Mark Pending
							</button>
						{/if}
						{#if comment.reply_status !== 'skipped'}
							<button
								class="btn-ghost btn-sm"
								onclick={() => handleStatusChange(comment.id, 'skipped')}
								disabled={actionLoading === comment.id}
							>
								Skip
							</button>
						{/if}
					</div>
				{/if}
			</div>
		{/each}
	{/if}
</div>

<style>
	.filter-row {
		display: flex;
		align-items: center;
		gap: var(--space-xs);
		margin-bottom: var(--space-md);
	}

	.filter-label {
		font-size: var(--text-sm);
		color: var(--color-muted);
		font-weight: 500;
		white-space: nowrap;
	}

	.summary-bar {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-md);
		padding: var(--space-sm) 0;
		margin-bottom: var(--space-sm);
		border-bottom: 1px solid var(--color-border);
	}

	.summary-group {
		display: flex;
		align-items: center;
		gap: var(--space-xs);
	}

	.summary-label {
		font-size: var(--text-sm);
		color: var(--color-muted);
		font-weight: 600;
	}

	.filter-bar {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-sm);
		margin-bottom: var(--space-md);
	}

	.filter-group {
		display: flex;
		align-items: center;
		gap: var(--space-xs);
	}

	.filter-group--grow {
		flex: 1;
		min-width: 12rem;
	}

	.comments-list {
		display: flex;
		flex-direction: column;
		gap: var(--space-sm);
	}

	.comment-card {
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		padding: var(--space-md);
		background: var(--color-surface);
		transition: border-color 0.15s ease;
	}

	.comment-card--replied {
		opacity: 0.7;
	}

	.comment-card--skeleton {
		gap: var(--space-sm);
	}

	.comment-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		margin-bottom: var(--space-xs);
	}

	.comment-author {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.comment-author-name {
		font-weight: 600;
		font-size: var(--text-sm);
	}

	.comment-date {
		font-size: var(--text-xs);
		color: var(--color-muted);
	}

	.comment-badges {
		display: flex;
		gap: var(--space-xs);
	}

	.comment-text {
		font-size: var(--text-sm);
		line-height: 1.5;
		margin: var(--space-xs) 0;
	}

	.comment-score {
		font-size: var(--text-xs);
		color: var(--color-muted);
	}

	.comment-reply-preview {
		display: flex;
		gap: var(--space-xs);
		padding: var(--space-xs) var(--space-sm);
		margin-top: var(--space-xs);
		background: var(--color-bg);
		border-radius: var(--radius-sm);
		font-size: var(--text-sm);
	}

	.reply-label {
		font-weight: 600;
		color: var(--color-muted);
	}

	.reply-text {
		color: var(--color-text);
	}

	.comment-actions {
		display: flex;
		gap: var(--space-xs);
		margin-top: var(--space-sm);
	}

	.reply-box {
		margin-top: var(--space-sm);
	}

	.reply-actions {
		display: flex;
		gap: var(--space-xs);
		margin-top: var(--space-xs);
	}

	.badge--warning {
		background: var(--color-warning-bg, #fef3c7);
		color: var(--color-warning-text, #92400e);
	}

	.badge--muted {
		background: var(--color-muted-bg, #f3f4f6);
		color: var(--color-muted, #6b7280);
	}
</style>
