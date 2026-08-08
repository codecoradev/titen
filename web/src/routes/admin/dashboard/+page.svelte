<script lang="ts">
	import PageHeader from '$lib/components/PageHeader.svelte';
	import StatSkeleton from '$lib/components/StatSkeleton.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import { getHealth, listAccounts, listPosts, listSchedules, getUpcomingSchedules, getAccountInsights } from '$lib/api';
	import { toast } from '$lib/toast.svelte';
	import type { Account, Post, Schedule, HealthResponse, AccountInsights } from '$lib/types';

	let loading = $state(true);
	let health = $state<HealthResponse | null>(null);
	let accounts = $state<Account[]>([]);
	let posts = $state<Post[]>([]);
	let schedules = $state<Schedule[]>([]);
	let upcoming = $state<Schedule[]>([]);
	let insights = $state<AccountInsights | null>(null);
	let insightsLoading = $state(false);
	let insightsAccountId = $state('');

	let activeAccounts = $derived(accounts.filter((a) => a.is_active).length);
	let publishedPosts = $derived(posts.filter((p) => p.status === 'published').length);
	let pendingSchedules = $derived(schedules.filter((s) => s.status === 'pending').length);
	let draftSchedules = $derived(schedules.filter((s) => s.status === 'draft').length);

	let recentPosts = $derived(
		[...posts].sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime()).slice(0, 5),
	);

	function formatDate(iso: string): string {
		return new Date(iso).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
	}

	function formatDateTime(iso: string): string {
		return new Date(iso).toLocaleDateString('en-US', {
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit',
		});
	}

	function isTokenExpiring(expiresAt: string | null): boolean {
		if (!expiresAt) return false;
		return new Date(expiresAt).getTime() < Date.now() + 7 * 24 * 60 * 60 * 1000;
	}

	async function load() {
		loading = true;
		try {
			const [h, a, p, s, u] = await Promise.all([
				getHealth().catch(() => null),
				listAccounts().catch(() => []),
				listPosts().catch(() => []),
				listSchedules().catch(() => []),
				getUpcomingSchedules().catch(() => []),
			]);
			health = h;
			accounts = a;
			posts = p;
			schedules = s;
			upcoming = u;
			// Auto-load insights for first active account
			if (a.length > 0) {
				insightsAccountId = a[0].id;
				await loadInsights();
			}
		} catch (e: any) {
			toast('Failed to load dashboard data', 'error');
		} finally {
			loading = false;
		}
	}

	async function loadInsights() {
		if (!insightsAccountId) return;
		insightsLoading = true;
		try {
			insights = await getAccountInsights(insightsAccountId);
		} catch {
			insights = null; // silently fail — insights are optional
		} finally {
			insightsLoading = false;
		}
	}

	$effect(() => {
		load();
	});
</script>

<PageHeader title="Dashboard" />

{#if loading}
	<div class="stat-grid">
		<StatSkeleton />
		<StatSkeleton />
		<StatSkeleton />
		<StatSkeleton />
	</div>
{:else}
	<!-- Stats -->
	<div class="stat-grid">
		<div class="stat-card">
			<div class="stat-card-label">Active Accounts</div>
			<div class="stat-card-value tabular-nums">{activeAccounts}</div>
		</div>
		<div class="stat-card">
			<div class="stat-card-label">Total Posts</div>
			<div class="stat-card-value tabular-nums">{posts.length.toLocaleString()}</div>
		</div>
		<div class="stat-card">
			<div class="stat-card-label">Published</div>
			<div class="stat-card-value tabular-nums">{publishedPosts.toLocaleString()}</div>
		</div>
		{#if draftSchedules > 0}
			<div class="stat-card" style="border-color: var(--color-warning-border, #fcd34d);">
				<div class="stat-card-label">Drafts (Needs Review)</div>
				<div class="stat-card-value tabular-nums">{draftSchedules.toLocaleString()}</div>
			</div>
		{/if}
		<div class="stat-card">
			<div class="stat-card-label">Approved (Pending)</div>
			<div class="stat-card-value tabular-nums">{pendingSchedules.toLocaleString()}</div>
		</div>
		{#if health}
			<div class="stat-card">
				<div class="stat-card-label">System Health</div>
				<div class="stat-card-value" style="padding-top: var(--space-2xs);">
					<StatusBadge status={health.status} />
				</div>
			</div>
			<div class="stat-card">
				<div class="stat-card-label">Version</div>
				<div class="stat-card-value tabular-nums">{health.version}</div>
			</div>
		{/if}
	</div>

	<!-- Token health -->
	{#if accounts.length > 0}
		<section class="dashboard-section">
			<h2 class="section-heading">Token Health</h2>
			<div class="data-table-wrap">
				<div class="token-list">
					{#each accounts as account}
						<div class="token-row">
							<span class="token-username">{account.username}</span>
							<StatusBadge status={account.is_active ? 'active' : 'inactive'} />
							<span class="token-expiry">
								{#if account.expires_at}
									{#if isTokenExpiring(account.expires_at)}
										<span class="badge badge--warning">Expires {formatDate(account.expires_at)}</span>
									{:else}
										{formatDate(account.expires_at)}
									{/if}
								{:else}
									<span style="color: var(--color-muted);">—</span>
								{/if}
							</span>
						</div>
					{/each}
				</div>
			</div>
		</section>
	{/if}

	<!-- Account insights -->
	{#if accounts.length > 0}
		<section class="dashboard-section">
			<div class="insights-header">
				<h2 class="section-heading" style="margin: 0;">Account Insights</h2>
				<select class="form-input insights-select" bind:value={insightsAccountId} onchange={loadInsights}>
					{#each accounts as account}
						<option value={account.id}>@{account.username}</option>
					{/each}
				</select>
			</div>
			<div class="data-table-wrap">
				{#if insightsLoading}
					<div class="insights-grid">
						{#each Array(6) as _}
							<div class="insight-card"><div class="skeleton" style="height: 2rem;"></div></div>
						{/each}
					</div>
				{:else if insights}
					<div class="insights-grid">
						{#each Object.entries(insights) as [key, value]}
							<div class="insight-card">
								<div class="insight-label">{key.replace(/_/g, ' ')}</div>
								<div class="insight-value tabular-nums">{typeof value === 'number' ? value.toLocaleString() : value ?? '—'}</div>
							</div>
						{/each}
					</div>
				{:else}
					<div class="empty-state" style="padding: var(--space-lg);">
						<p class="empty-state-title" style="font-size: var(--text-sm);">No insights available</p>
						<p class="empty-state-desc" style="font-size: var(--text-xs);">Insights may require a valid Threads API token.</p>
					</div>
				{/if}
			</div>
		</section>
	{/if}

	<!-- Two-column: Recent posts + Upcoming schedules -->
	<div class="dashboard-grid">
		<!-- Recent posts -->
		<section>
			<h2 class="section-heading">Recent Posts</h2>
			<div class="data-table-wrap">
				{#if recentPosts.length === 0}
					<div class="empty-state" style="padding: var(--space-lg);">
						<p class="empty-state-title" style="font-size: var(--text-sm);">No posts yet</p>
					</div>
			{:else}
				<ul class="compact-list">
					{#each recentPosts as post}
					<li class="compact-row">
						<div class="compact-content">
							<span class="truncate">{post.caption || '(no caption)'}</span>
						</div>
						<div class="compact-meta">
							<StatusBadge status={post.status} />
							<span class="compact-date">{formatDate(post.created_at)}</span>
						</div>
					</li>
				{/each}
				</ul>
			{/if}
			</div>
		</section>

		<!-- Upcoming schedules -->
		<section>
			<h2 class="section-heading">Upcoming</h2>
			<div class="data-table-wrap">
				{#if upcoming.length === 0}
					<div class="empty-state" style="padding: var(--space-lg);">
						<p class="empty-state-title" style="font-size: var(--text-sm);">No upcoming schedules</p>
					</div>
			{:else}
				<ul class="compact-list">
					{#each upcoming.slice(0, 5) as schedule}
					<li class="compact-row">
						<div class="compact-content">
							<span class="truncate">{schedule.caption || '(no caption)'}</span>
						</div>
						<div class="compact-meta">
							<StatusBadge status={schedule.status} />
							<span class="compact-date">{formatDateTime(schedule.scheduled_at)}</span>
						</div>
					</li>
				{/each}
				</ul>
			{/if}
			</div>
		</section>
	</div>
{/if}

<style>
	.dashboard-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--space-lg);
	}

	.dashboard-section {
		margin-bottom: var(--space-lg);
	}

	.insights-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-sm);
		margin-bottom: var(--space-sm);
		flex-wrap: wrap;
	}

	.insights-select {
		width: auto;
		font-size: var(--text-sm);
	}

	@media (max-width: 30rem) {
		.insights-header {
			flex-direction: column;
			align-items: stretch;
		}

		.insights-select {
			width: 100%;
		}
	}

	.insights-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(8rem, 1fr));
		gap: var(--space-sm);
		padding: var(--space-md);
	}

	.insight-card {
		padding: var(--space-sm) var(--space-md);
		background: var(--color-bg-hover);
		border-radius: var(--radius-sm);
		text-align: center;
	}

	.insight-label {
		font-size: var(--text-xs);
		color: var(--color-muted);
		text-transform: capitalize;
		margin-bottom: var(--space-3xs);
	}

	.insight-value {
		font-size: var(--text-lg);
		font-weight: 700;
	}

	.token-list {
		display: flex;
		flex-direction: column;
	}

	.token-row {
		display: flex;
		align-items: center;
		gap: var(--space-sm);
		padding: var(--space-sm) var(--space-md);
		border-bottom: var(--table-border);
		font-size: var(--text-sm);
	}

	.token-row:last-child {
		border-bottom: none;
	}

	@media (max-width: 30rem) {
		.token-row {
			flex-wrap: wrap;
			padding: var(--space-xs) var(--space-sm);
		}
	}

	.token-username {
		font-weight: 500;
		flex: 1;
	}

	.token-expiry {
		font-size: var(--text-xs);
		color: var(--color-muted);
	}

	.compact-list {
		list-style: none;
		padding: 0;
		margin: 0;
	}

	.compact-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-sm);
		padding: var(--space-sm) var(--space-md);
		border-bottom: var(--table-border);
	}

	.compact-row:last-child {
		border-bottom: none;
	}

	.compact-content {
		flex: 1;
		min-width: 0;
		font-size: var(--text-sm);
	}

	.compact-meta {
		display: flex;
		align-items: center;
		gap: var(--space-xs);
		flex-shrink: 0;
	}

	.compact-date {
		font-size: var(--text-xs);
		color: var(--color-muted);
		font-family: var(--font-mono);
	}

	@media (max-width: 30rem) {
		.compact-row {
			flex-direction: column;
			align-items: flex-start;
			gap: var(--space-2xs);
			padding: var(--space-xs) var(--space-sm);
		}

		.compact-meta {
			width: 100%;
			justify-content: space-between;
		}
	}

	@media (max-width: 48rem) {
		.dashboard-grid {
			grid-template-columns: 1fr !important;
		}
	}
</style>
