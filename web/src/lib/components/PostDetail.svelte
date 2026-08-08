<script lang="ts">
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import { formatDateTime } from '$lib/tz';
	import { getPostInsights, getAnalyticsTrend } from '$lib/api';
	import { toast } from '$lib/toast.svelte';
	import type { Post, Insights, AnalyticsTrend } from '$lib/types';

	interface Props {
		post: Post;
		onClose: () => void;
	}

	let { post, onClose }: Props = $props();

	let insights = $state<Insights | null>(null);
	let trend = $state<AnalyticsTrend[]>([]);
	let loadingInsights = $state(false);
	let loadingTrend = $state(false);

	// Parse media URLs from carousel_children JSON
	let mediaUrls: string[] = $derived.by(() => {
		if (!post.carousel_children) return [];
		try {
			const parsed = JSON.parse(post.carousel_children);
			if (!Array.isArray(parsed)) return [];
			return parsed
				.map((c: any) => c?.image_url || c?.url || '')
				.filter(Boolean);
		} catch {
			return [];
		}
	});

	// Threads permalink (constructed from threads_post_id)
	let permalink = $derived(
		post.threads_post_id && post.account?.username
			? `https://www.threads.net/@${post.account.username}/post/${post.threads_post_id}`
			: null
	);

	// Load insights + trend once on mount with cleanup
	$effect(() => {
		let cancelled = false;

		loadingInsights = true;
		getPostInsights(post.id)
			.then((data) => { if (!cancelled) insights = data; })
			.catch(() => {})
			.finally(() => { if (!cancelled) loadingInsights = false; });

		loadingTrend = true;
		getAnalyticsTrend(post.id)
			.then((data) => { if (!cancelled) trend = data; })
			.catch(() => {})
			.finally(() => { if (!cancelled) loadingTrend = false; });

		return () => { cancelled = true; };
	});

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') onClose();
	}
</script>

<svelte:window on:keydown={handleKeydown} />

<div class="modal-overlay" onclick={onClose} role="presentation">
	<div
		class="modal-content"
		role="dialog"
		aria-modal="true"
		aria-label="Post detail"
		onclick={(e) => e.stopPropagation()}
	>
		<!-- Header -->
		<div class="modal-header">
			<h2 class="modal-title">Post Detail</h2>
			<button class="close-btn" onclick={onClose} aria-label="Close">&times;</button>
		</div>

		<!-- Body -->
		<div class="modal-body">
			<!-- Status -->
			<div class="detail-row">
				<span class="detail-label">Status</span>
				<StatusBadge status={post.status} />
			</div>

			<!-- Published date -->
			{#if post.published_at}
				<div class="detail-row">
					<span class="detail-label">Published</span>
					<span class="detail-value">{formatDateTime(post.published_at)}</span>
				</div>
			{/if}

			<!-- Created date -->
			<div class="detail-row">
				<span class="detail-label">Created</span>
				<span class="detail-value">{formatDateTime(post.created_at)}</span>
			</div>

			<!-- Media type -->
			<div class="detail-row">
				<span class="detail-label">Type</span>
				<span class="detail-value badge-muted">{post.media_type}</span>
			</div>

			<!-- Permalink -->
			{#if permalink}
				<div class="detail-row">
					<span class="detail-label">Permalink</span>
					<a
						href={permalink}
						target="_blank"
						rel="noopener noreferrer"
						class="permalink-link"
					>
						View on Threads ↗
					</a>
				</div>
			{/if}

			<!-- Caption -->
			{#if post.caption}
				<div class="detail-section">
					<span class="detail-label">Caption</span>
					<p class="detail-caption">{post.caption}</p>
				</div>
			{/if}

			<!-- Media previews -->
			{#if mediaUrls.length > 0}
				<div class="detail-section">
					<span class="detail-label">Media ({mediaUrls.length})</span>
					<div class="media-grid">
						{#each mediaUrls as url}
							<div class="media-thumb">
								<img src={url} alt="Media" loading="lazy" />
							</div>
						{/each}
					</div>
				</div>
			{/if}

			<!-- Engagement metrics -->
			<div class="detail-section">
				<span class="detail-label">Engagement</span>
				{#if loadingInsights}
					<div class="metrics-grid">
						{#each Array(5) as _}
							<div class="metric-card">
								<div class="skeleton" style="height: 1.5rem; width: 3rem;"></div>
								<div class="skeleton" style="height: 0.75rem; width: 2rem;"></div>
							</div>
						{/each}
					</div>
				{:else if insights}
					<div class="metrics-grid">
						<div class="metric-card">
							<div class="metric-value tabular-nums">{(insights.views ?? 0).toLocaleString()}</div>
							<div class="metric-label">Views</div>
						</div>
						<div class="metric-card">
							<div class="metric-value tabular-nums">{(insights.likes ?? 0).toLocaleString()}</div>
							<div class="metric-label">Likes</div>
						</div>
						<div class="metric-card">
							<div class="metric-value tabular-nums">{(insights.replies ?? 0).toLocaleString()}</div>
							<div class="metric-label">Replies</div>
						</div>
						<div class="metric-card">
							<div class="metric-value tabular-nums">{(insights.reposts ?? 0).toLocaleString()}</div>
							<div class="metric-label">Reposts</div>
						</div>
						<div class="metric-card">
							<div class="metric-value tabular-nums">{(insights.quotes ?? 0).toLocaleString()}</div>
							<div class="metric-label">Quotes</div>
						</div>
					</div>
				{:else}
					<p class="no-data">No insights available</p>
				{/if}
			</div>

			<!-- Analytics trend -->
			{#if trend.length > 1}
				<div class="detail-section">
					<span class="detail-label">Trend</span>
					<div class="trend-table-wrap">
						<table class="trend-table">
							<thead>
								<tr>
									<th>Date</th>
									<th class="num">Views</th>
									<th class="num">Likes</th>
									<th class="num">Replies</th>
								</tr>
							</thead>
							<tbody>
								{#each trend.slice(-10).reverse() as snap}
									<tr>
										<td class="mono">{formatDateTime(snap.date)}</td>
										<td class="num tabular-nums">{snap.views?.toLocaleString() ?? '—'}</td>
										<td class="num tabular-nums">{snap.likes?.toLocaleString() ?? '—'}</td>
										<td class="num tabular-nums">{snap.replies?.toLocaleString() ?? '—'}</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
				</div>
			{/if}
		</div>

		<!-- Footer -->
		<div class="modal-footer">
			<button class="btn btn-secondary" onclick={onClose}>Close</button>
		</div>
	</div>
</div>

<style>
	.modal-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 50;
		padding: var(--space-md);
	}

	.modal-content {
		background: var(--color-bg);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		max-width: 42rem;
		width: 100%;
		max-height: 85vh;
		display: flex;
		flex-direction: column;
	}

	.modal-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: var(--space-md);
		border-bottom: 1px solid var(--color-border);
	}

	.modal-title {
		font-size: var(--text-lg);
		font-weight: 700;
		margin: 0;
	}

	.close-btn {
		background: none;
		border: none;
		font-size: 1.5rem;
		cursor: pointer;
		color: var(--color-muted);
		padding: 0 var(--space-xs);
		line-height: 1;
	}

	.close-btn:hover {
		color: var(--color-text);
	}

	.modal-body {
		padding: var(--space-md);
		overflow-y: auto;
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: var(--space-sm);
	}

	.detail-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-sm);
	}

	.detail-label {
		font-size: var(--text-xs);
		color: var(--color-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		font-weight: 600;
	}

	.detail-value {
		font-size: var(--text-sm);
	}

	.badge-muted {
		padding: var(--space-3xs) var(--space-xs);
		background: var(--color-bg-hover);
		border-radius: var(--radius-sm);
		font-size: var(--text-xs);
		font-weight: 600;
	}

	.permalink-link {
		font-size: var(--text-sm);
		color: var(--color-accent, #3b82f6);
		text-decoration: none;
		font-weight: 600;
	}

	.permalink-link:hover {
		text-decoration: underline;
	}

	.detail-section {
		display: flex;
		flex-direction: column;
		gap: var(--space-xs);
	}

	.detail-caption {
		font-size: var(--text-sm);
		line-height: 1.6;
		margin: 0;
		white-space: pre-wrap;
		word-break: break-word;
	}

	/* Media grid */
	.media-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(6rem, 1fr));
		gap: var(--space-xs);
	}

	.media-thumb {
		aspect-ratio: 1;
		border-radius: var(--radius-sm);
		overflow: hidden;
		border: 1px solid var(--color-border);
	}

	.media-thumb img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	/* Metrics */
	.metrics-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(5rem, 1fr));
		gap: var(--space-xs);
	}

	.metric-card {
		text-align: center;
		padding: var(--space-xs);
		background: var(--color-bg-hover);
		border-radius: var(--radius-sm);
	}

	.metric-value {
		font-size: var(--text-lg);
		font-weight: 700;
	}

	.metric-label {
		font-size: var(--text-2xs);
		color: var(--color-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.no-data {
		font-size: var(--text-sm);
		color: var(--color-muted);
		margin: 0;
	}

	/* Trend table */
	.trend-table-wrap {
		overflow-x: auto;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-sm);
	}

	.trend-table {
		width: 100%;
		border-collapse: collapse;
		font-size: var(--text-xs);
	}

	.trend-table th {
		text-align: left;
		padding: var(--space-2xs) var(--space-xs);
		border-bottom: var(--table-border);
		font-weight: 600;
		color: var(--color-muted);
		text-transform: uppercase;
		font-size: var(--text-2xs);
	}

	.trend-table td {
		padding: var(--space-2xs) var(--space-xs);
		border-bottom: var(--table-border);
	}

	.trend-table tr:last-child td {
		border-bottom: none;
	}

	.num {
		text-align: right;
	}

	.mono {
		font-family: var(--font-mono);
	}

	/* Skeleton */
	.skeleton {
		background: var(--color-bg-hover);
		border-radius: var(--radius-2xs);
		animation: pulse 1.5s ease-in-out infinite;
	}

	@keyframes pulse {
		0%, 100% { opacity: 1; }
		50% { opacity: 0.5; }
	}

	/* Footer */
	.modal-footer {
		display: flex;
		gap: var(--space-xs);
		padding: var(--space-md);
		border-top: 1px solid var(--color-border);
		justify-content: flex-end;
	}

	.btn {
		padding: var(--space-xs) var(--space-md);
		border-radius: var(--radius-sm);
		font-size: var(--text-sm);
		font-weight: 600;
		cursor: pointer;
		border: 1px solid transparent;
	}

	.btn-secondary {
		background: var(--color-bg-hover);
		color: var(--color-text);
		border-color: var(--color-border);
	}

	@media (max-width: 30rem) {
		.modal-overlay {
			padding: 0;
		}

		.modal-content {
			max-height: 100vh;
			border-radius: 0;
			height: 100vh;
		}
	}
</style>
