<script lang="ts">
	import PageHeader from '$lib/components/PageHeader.svelte';
	import DataTable from '$lib/components/DataTable.svelte';
	import StatSkeleton from '$lib/components/StatSkeleton.svelte';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import { listAnalytics, listAccounts, ApiError } from '$lib/api';
	import type { AnalyticsSnap, Account } from '$lib/types';

	// ── State ──
	let loading = $state(true);
	let error = $state('');
	let accounts = $state<Account[]>([]);
	let selectedAccountId = $state<string>('');
	let summaries = $state<AnalyticsSnap[]>([]);
	let selectedPeriod = $state('7d');
	let trend = $state<import('$lib/types').AnalyticsTrend[]>([]);
	let trendLoading = $state(false);

	const trendMax = $derived(
		trend.length > 0
			? Math.max(...trend.flatMap((t) => [t.views, t.likes, t.replies]), 1)
			: 1
	);

	const periods = [
		{ value: '7d', label: '7 days' },
		{ value: '30d', label: '30 days' },
		{ value: '90d', label: '90 days' },
	] as const;

	// ── Derived: aggregate totals across accounts ──
	const totals = $derived.by(() => {
		if (summaries.length === 0) return null;
		const agg = summaries.reduce(
			(acc, s) => ({
				total_posts: acc.total_posts + s.total_posts,
				total_likes: acc.total_likes + s.total_likes,
				total_replies: acc.total_replies + s.total_replies,
				total_reposts: acc.total_reposts + s.total_reposts,
				total_views: acc.total_views + s.total_views,
			}),
			{ total_posts: 0, total_likes: 0, total_replies: 0, total_reposts: 0, total_views: 0 },
		);
		// Engagement rate = (likes + replies + reposts) / views * 100
		// Computed AFTER aggregation, not incrementally during reduce
		const interactions = agg.total_likes + agg.total_replies + agg.total_reposts;
		return {
			...agg,
			engagement_rate: agg.total_views > 0 ? (interactions / agg.total_views) * 100 : 0,
		};
	});



	// ── Formatters ──
	function fmt(n: number): string {
		if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + 'M';
		if (n >= 1_000) return (n / 1_000).toFixed(1) + 'K';
		return n.toLocaleString();
	}

	function fmtRate(r: number): string {
		return r.toFixed(2) + '%';
	}

	function fmtDate(d: string): string {
		return new Date(d).toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
	}

	// ── Data fetching ──
	async function loadAccounts() {
		try {
			const res = await listAccounts();
			accounts = res.filter((a) => a.is_active);
		} catch (e) {
			// Accounts not critical for analytics
		}
	}

	async function loadAnalytics() {
		loading = true;
		error = '';
		try {
			const params: { account_id?: string; period?: string } = {};
			if (selectedAccountId) params.account_id = selectedAccountId;
			if (selectedPeriod) params.period = selectedPeriod;
			const res = await listAnalytics(params);
			summaries = res;
		} catch (e) {
			error = e instanceof ApiError ? e.body || e.message : 'Failed to load analytics';
		} finally {
			loading = false;
		}
	}

	async function loadTrend() {
		// Trend endpoint is per-post, not per-account — disabled for now
		trend = [];
		trendLoading = false;
	}

	// ── Init + reactive reload ──
	$effect(() => {
		// Read reactive values to track them
		const _accountId = selectedAccountId;
		const _period = selectedPeriod;
		loadAnalytics();
	});

	$effect(() => {
		const _accountId = selectedAccountId;
		const _period = selectedPeriod;
		loadTrend();
	});

	// Initial accounts load
	loadAccounts();
</script>

<PageHeader title="Analytics" description="Post performance and engagement metrics">
	{#snippet action()}
		<div class="filter-bar">
			<select
				class="form-input form-select"
				bind:value={selectedAccountId}
				aria-label="Filter by account"
			>
				<option value="">All accounts</option>
				{#each accounts as acct}
					<option value={acct.id}>{acct.username}</option>
				{/each}
			</select>
			<div class="period-toggle">
				{#each periods as p}
					<button
						class="btn-outline btn-sm period-btn"
						class:period-active={selectedPeriod === p.value}
						onclick={() => (selectedPeriod = p.value)}
						type="button"
					>
						{p.label}
					</button>
				{/each}
			</div>
		</div>
	{/snippet}
</PageHeader>

{#if error}
	<div class="error-banner">
		<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" width="18" height="18">
			<circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/>
		</svg>
		<span>{error}</span>
		<button class="btn-ghost btn-sm" onclick={loadAnalytics} type="button">Retry</button>
	</div>
{/if}

<!-- Stat cards -->
{#if loading}
	<div class="stat-grid">
		<StatSkeleton />
		<StatSkeleton />
		<StatSkeleton />
		<StatSkeleton />
		<StatSkeleton />
		<StatSkeleton />
	</div>
{:else if summaries.length > 0}
	<div class="stat-grid">
		<div class="stat-card">
			<p class="stat-card-label">Posts</p>
			<p class="stat-card-value">{totals ? fmt(totals.total_posts) : '—'}</p>
		</div>
		<div class="stat-card">
			<p class="stat-card-label">Views</p>
			<p class="stat-card-value">{totals ? fmt(totals.total_views) : '—'}</p>
		</div>
		<div class="stat-card">
			<p class="stat-card-label">Likes</p>
			<p class="stat-card-value">{totals ? fmt(totals.total_likes) : '—'}</p>
		</div>
		<div class="stat-card">
			<p class="stat-card-label">Replies</p>
			<p class="stat-card-value">{totals ? fmt(totals.total_replies) : '—'}</p>
		</div>
		<div class="stat-card">
			<p class="stat-card-label">Reposts</p>
			<p class="stat-card-value">{totals ? fmt(totals.total_reposts) : '—'}</p>
		</div>
		<div class="stat-card">
			<p class="stat-card-label">Engagement rate</p>
			<p class="stat-card-value">{totals ? fmtRate(totals.engagement_rate) : '—'}</p>
		</div>
	</div>
{:else}
	<EmptyState
		title="No analytics data"
		description="Analytics will appear once posts have been published and insights have been collected."
	/>
{/if}

<!-- Trend chart (single account only) -->
{#if selectedAccountId && summaries.length > 0}
	<section class="trend-section">
		<h2 class="section-title">Engagement trend</h2>
		{#if trendLoading}
			<div class="trend-chart">
				<div class="trend-loading">
					<div class="skeleton" style="height: 100%;"></div>
				</div>
			</div>
		{:else if trend.length > 0}
			<div class="trend-chart">
				<!-- Y-axis labels -->
				<div class="trend-y-axis">
					<span>{fmt(trendMax)}</span>
					<span>{fmt(trendMax / 2)}</span>
					<span>0</span>
				</div>
				<!-- Bars -->
				<div class="trend-bars">
					{#each trend as t, i}
						<div class="trend-bar-group" title="{fmtDate(t.date)} — Views: {fmt(t.views)}, Likes: {fmt(t.likes)}, Replies: {fmt(t.replies)}">
							<div class="trend-bar trend-bar--views" style="height: {(t.views / trendMax) * 100}%;"></div>
							<div class="trend-bar trend-bar--likes" style="height: {(t.likes / trendMax) * 100}%;"></div>
							<div class="trend-bar trend-bar--replies" style="height: {(t.replies / trendMax) * 100}%;"></div>
							<span class="trend-bar-label">{fmtDate(t.date)}</span>
						</div>
					{/each}
				</div>
			</div>
			<div class="trend-legend">
				<span class="legend-item"><span class="legend-dot legend-dot--views"></span>Views</span>
				<span class="legend-item"><span class="legend-dot legend-dot--likes"></span>Likes</span>
				<span class="legend-item"><span class="legend-dot legend-dot--replies"></span>Replies</span>
			</div>
		{:else}
			<EmptyState
				title="No trend data"
				description="Trend data is available per account. Insights may not have been collected for this period."
			/>
		{/if}
	</section>
{/if}

<!-- Per-account breakdown table -->
{#if summaries.length > 0 && summaries.length > 1}
	<section class="table-section">
		<h2 class="section-title">Per-account breakdown</h2>
		<DataTable
			columns={[
				{ key: 'account_id', label: 'Account ID', sortable: true, class: 'truncate' },
				{ key: 'period', label: 'Period', sortable: true },
				{ key: 'total_posts', label: 'Posts', sortable: true },
				{ key: 'total_views', label: 'Views', sortable: true },
				{ key: 'total_likes', label: 'Likes', sortable: true },
				{ key: 'total_replies', label: 'Replies', sortable: true },
				{ key: 'total_reposts', label: 'Reposts', sortable: true },
				{ key: 'engagement_rate', label: 'Eng. rate', sortable: true },
			]}
			rows={summaries.map((s) => ({
				...s,
				total_views: fmt(s.total_views),
				total_likes: fmt(s.total_likes),
				total_replies: fmt(s.total_replies),
				total_reposts: fmt(s.total_reposts),
				engagement_rate: fmtRate(s.engagement_rate),
			}))}
			loading={loading}
			emptyTitle="No accounts"
			emptyDesc="Connect accounts to see per-account analytics."
		/>
	</section>
{/if}

<style>
	/* ── Filter bar ── */
	.filter-bar {
		display: flex;
		align-items: center;
		gap: var(--space-sm);
		flex-wrap: wrap;
	}

	.form-select {
		appearance: none;
		background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='%236a6a6a' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpolyline points='6 9 12 15 18 9'/%3E%3C/svg%3E");
		background-repeat: no-repeat;
		background-position: right var(--space-sm) center;
		background-size: 1rem;
		padding-right: var(--space-lg);
		min-width: 10rem;
	}

	.period-toggle {
		display: flex;
		gap: var(--space-3xs);
	}

	.period-btn {
		flex-shrink: 0;
	}

	.period-active {
		background: var(--color-ink);
		color: var(--color-paper);
		border-color: var(--color-ink);
	}

	.period-active:hover {
		background: oklch(25% 0.008 260);
		color: var(--color-paper);
	}

	/* ── Error banner ── */
	.error-banner {
		display: flex;
		align-items: center;
		gap: var(--space-sm);
		padding: var(--space-sm) var(--space-md);
		background: var(--color-error-dim);
		border: 1px solid var(--color-error);
		border-radius: var(--radius-lg);
		margin-bottom: var(--space-lg);
		font-size: var(--text-sm);
		color: var(--color-error-ink);
	}

	.error-banner button {
		margin-inline-start: auto;
		flex-shrink: 0;
	}

	/* ── Section titles ── */
	.section-title {
		font-family: var(--font-display);
		font-size: var(--text-md);
		font-weight: 600;
		letter-spacing: -0.02em;
		margin-bottom: var(--space-md);
	}

	.table-section {
		margin-top: var(--space-xl);
	}

	/* ── Trend chart ── */
	.trend-section {
		margin-top: var(--space-xl);
	}

	.trend-chart {
		display: flex;
		gap: var(--space-xs);
		background: var(--surface-raised);
		border: var(--rule-default);
		border-radius: var(--radius-lg);
		padding: var(--space-md);
		overflow-x: auto;
	}

	.trend-y-axis {
		display: flex;
		flex-direction: column;
		justify-content: space-between;
		align-items: flex-end;
		min-width: 3rem;
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-muted);
		padding-block: var(--space-2xs);
	}

	.trend-bars {
		flex: 1;
		display: flex;
		align-items: flex-end;
		gap: var(--space-xs);
		min-height: 12rem;
	}

	.trend-bar-group {
		flex: 1;
		min-width: 1.5rem;
		max-width: 2.5rem;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 1px;
		position: relative;
	}

	.trend-bar {
		width: 100%;
		border-radius: var(--radius-sm) var(--radius-sm) 0 0;
		min-height: 0;
		transition: height var(--dur-base) var(--ease-out);
	}

	.trend-bar--views {
		background: var(--color-accent);
	}

	.trend-bar--likes {
		background: var(--color-success);
	}

	.trend-bar--replies {
		background: var(--color-info);
	}

	.trend-bar-label {
		position: absolute;
		inset-block-end: -1.4rem;
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		max-width: 100%;
	}

	.trend-loading {
		flex: 1;
		min-height: 12rem;
		border-radius: var(--radius-sm);
	}

	.trend-legend {
		display: flex;
		gap: var(--space-md);
		margin-top: var(--space-sm);
	}

	.legend-item {
		display: flex;
		align-items: center;
		gap: var(--space-2xs);
		font-size: var(--text-sm);
		color: var(--color-muted);
	}

	.legend-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.legend-dot--views {
		background: var(--color-accent);
	}

	.legend-dot--likes {
		background: var(--color-success);
	}

	.legend-dot--replies {
		background: var(--color-info);
	}

	/* ── Responsive ── */
	@media (max-width: 48rem) {
		.filter-bar {
			flex-direction: column;
			align-items: stretch;
		}

		.form-select {
			width: 100%;
		}

		.trend-bar-group {
			min-width: 1rem;
		}

		.trend-bar-label {
			display: none;
		}
	}
</style>
