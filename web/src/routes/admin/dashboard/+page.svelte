<script lang="ts">
	import PageHeader from '$lib/components/PageHeader.svelte';
	import StatSkeleton from '$lib/components/StatSkeleton.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import Skeleton from '$lib/components/ui/skeleton/skeleton.svelte';
	import {
		getHealth,
		listAccounts,
		listPosts,
		listSchedules,
		getUpcomingSchedules,
		getThreadsProfile,
		getAccountInsights,
	} from '$lib/api';
	import { formatDateTime, formatDate } from '$lib/tz';
	import { toast } from '$lib/toast.svelte';
	import type {
		Account,
		Post,
		Schedule,
		HealthResponse,
		AccountInsights,
		ThreadsProfile,
	} from '$lib/types';

	// ── State ──
	let loading = $state(true);
	let loaded = $state(false);
	let health = $state<HealthResponse | null>(null);
	let accounts = $state<Account[]>([]);
	let posts = $state<Post[]>([]);
	let schedules = $state<Schedule[]>([]);
	let upcoming = $state<Schedule[]>([]);

	// Per-account enriched data
	interface AccountCard {
		account: Account;
		profile: ThreadsProfile | null;
		insights: AccountInsights | null;
		profileLoading: boolean;
		insightsLoading: boolean;
		profileError: boolean;
	}
	let cards = $state<AccountCard[]>([]);

	// ── Derived stats ──
	let activeAccounts = $derived(accounts.filter((a) => a.is_active).length);
	let publishedPosts = $derived(posts.filter((p) => p.status === 'published').length);
	let pendingSchedules = $derived(schedules.filter((s) => s.status === 'pending').length);
	let draftSchedules = $derived(schedules.filter((s) => s.status === 'draft').length);

	let recentPosts = $derived(
		[...posts]
			.sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime())
			.slice(0, 5),
	);

	// ── Helpers ──

	function isTokenExpiring(expiresAt: string | null): boolean {
		if (!expiresAt) return false;
		return new Date(expiresAt).getTime() < Date.now() + 7 * 24 * 60 * 60 * 1000;
	}

	// Extract key insight metrics for display
	function getMetric(insights: AccountInsights | null, ...keys: string[]): number | null {
		if (!insights) return null;
		for (const key of keys) {
			const val = insights[key];
			if (typeof val === 'number') return val;
		}
		return null;
	}

	// ── Data loading ──
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

			// Initialize account cards — lazy-load profile + insights for each
			cards = a.map((account) => ({
				account,
				profile: null,
				insights: null,
				profileLoading: false,
				insightsLoading: false,
				profileError: false,
			}));

			// Load profiles + insights for all accounts concurrently.
			// Each loadCardData fires 2 requests (profile + insights) via
			// Promise.allSettled — browser limits concurrent connections
			// per host automatically (typically 6).
			await Promise.all(
				cards.map((_, i) => loadCardData(i)),
			);
		} catch (e: any) {
			toast('Failed to load dashboard data', 'error');
		} finally {
			loading = false;
		}
	}

	async function loadCardData(index: number) {
		if (!cards[index]) return;
		const accountId = cards[index].account.id;

		// Set loading states
		cards[index] = { ...cards[index], profileLoading: true, insightsLoading: true };

		// Fetch profile + insights concurrently
		const [profileResult, insightsResult] = await Promise.allSettled([
			getThreadsProfile(accountId),
			getAccountInsights(accountId),
		]);

		// Apply results in a single mutation (no race)
		cards[index] = {
			...cards[index],
			profile: profileResult.status === 'fulfilled' ? profileResult.value : null,
			profileLoading: false,
			profileError: profileResult.status === 'rejected',
			insights: insightsResult.status === 'fulfilled' ? insightsResult.value : null,
			insightsLoading: false,
		};
	}

	$effect(() => {
		if (!loaded) {
			load();
			loaded = true;
		}
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
	<!-- Overview Stats -->
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
		{/if}
	</div>

	<!-- Per-Account Cards -->
	{#if cards.length > 0}
		<section class="dashboard-section">
			<h2 class="section-heading">Accounts</h2>
			<div class="account-cards-grid">
				{#each cards as card, i (card.account.id)}
					<div class="account-card">
						<!-- Header: avatar + username -->
						<div class="account-card-header">
							<div class="account-avatar">
								{#if card.profileLoading}
									<div class="avatar-skeleton"></div>
								{:else if card.profile?.threads_profile_picture_url}
									<img
										src={card.profile.threads_profile_picture_url}
										alt={card.account.username}
										class="avatar-img"
										loading="lazy"
									/>
								{:else}
									<div class="avatar-placeholder">
										{card.account.username.charAt(0).toUpperCase()}
									</div>
								{/if}
							</div>
							<div class="account-info">
								<div class="account-name">
									@{card.account.username}
								</div>
								<div class="account-status">
									<StatusBadge status={card.account.is_active ? 'active' : 'inactive'} />
									{#if card.account.expires_at && isTokenExpiring(card.account.expires_at)}
										<span class="badge badge--warning">Token expiring</span>
									{/if}
								</div>
							</div>
						</div>

						<!-- Profile metrics -->
						{#if card.profile}
							<div class="account-metrics">
								<div class="metric">
									<div class="metric-value tabular-nums">
										{(card.profile.followers_count ?? 0).toLocaleString()}
									</div>
									<div class="metric-label">Followers</div>
								</div>
								<div class="metric">
									<div class="metric-value tabular-nums">
										{(card.profile.following_count ?? 0).toLocaleString()}
									</div>
									<div class="metric-label">Following</div>
								</div>
								<div class="metric">
									<div class="metric-value tabular-nums">
										{(card.profile.media_count ?? 0).toLocaleString()}
									</div>
									<div class="metric-label">Posts</div>
								</div>
							</div>
						{:else if card.profileLoading}
							<div class="account-metrics">
								{#each Array(3) as _}
									<div class="metric">
										<Skeleton class="h-6 w-12" />
										<Skeleton class="h-3 w-8" />
									</div>
								{/each}
							</div>
						{:else if card.profileError}
							<div class="account-metrics">
								<div class="metric metric-error">
									<span style="font-size: var(--text-xs); color: var(--color-muted);">
										Profile unavailable
									</span>
								</div>
							</div>
						{/if}

						<!-- Engagement insights -->
						{#if card.insights && Object.keys(card.insights).length > 0}
							<div class="account-insights-row">
								{#each Object.entries(card.insights).slice(0, 4) as [key, value]}
									<div class="insight-mini">
										<span class="insight-mini-label">{key.replace(/_/g, ' ')}</span>
										<span class="insight-mini-value tabular-nums">
											{typeof value === 'number' ? value.toLocaleString() : value ?? '—'}
										</span>
									</div>
								{/each}
							</div>
						{:else if card.insightsLoading}
							<div class="account-insights-row">
								{#each Array(4) as _}
									<div class="insight-mini">
										<Skeleton class="h-3 w-10" />
										<Skeleton class="h-4 w-8" />
									</div>
								{/each}
							</div>
						{/if}

						<!-- Bio -->
						{#if card.profile?.threads_biography}
							<p class="account-bio">{card.profile.threads_biography}</p>
						{/if}
					</div>
				{/each}
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

	/* ── Account cards ── */
	.account-cards-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(18rem, 1fr));
		gap: var(--space-md);
	}

	.account-card {
		background: var(--color-bg);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		padding: var(--space-md);
		display: flex;
		flex-direction: column;
		gap: var(--space-sm);
		transition: border-color 0.15s ease;
	}

	.account-card:hover {
		border-color: var(--color-border-hover, var(--color-border));
	}

	.account-card-header {
		display: flex;
		align-items: center;
		gap: var(--space-sm);
	}

	.account-avatar {
		flex-shrink: 0;
		width: 3rem;
		height: 3rem;
	}

	.avatar-img,
	.avatar-placeholder,
	.avatar-skeleton {
		width: 3rem;
		height: 3rem;
		border-radius: 50%;
		object-fit: cover;
	}

	.avatar-placeholder {
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--color-bg-hover);
		font-size: var(--text-lg);
		font-weight: 700;
		color: var(--color-muted);
	}

	.avatar-skeleton {
		background: var(--color-bg-hover);
		animation: pulse 1.5s ease-in-out infinite;
	}

	@keyframes pulse {
		0%, 100% { opacity: 1; }
		50% { opacity: 0.5; }
	}

	.account-info {
		flex: 1;
		min-width: 0;
	}

	.account-name {
		font-weight: 600;
		font-size: var(--text-sm);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.account-status {
		display: flex;
		align-items: center;
		gap: var(--space-2xs);
		margin-top: var(--space-3xs);
		flex-wrap: wrap;
	}

	/* ── Metrics ── */
	.account-metrics {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: var(--space-xs);
		padding: var(--space-xs) 0;
		border-top: 1px solid var(--color-border);
		border-bottom: 1px solid var(--color-border);
	}

	.metric {
		text-align: center;
	}

	.metric-value {
		font-size: var(--text-base);
		font-weight: 700;
	}

	.metric-label {
		font-size: var(--text-2xs);
		color: var(--color-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.metric-error {
		grid-column: 1 / -1;
		text-align: center;
		padding: var(--space-xs);
	}

	/* ── Insights mini row ── */
	.account-insights-row {
		display: flex;
		gap: var(--space-xs);
		flex-wrap: wrap;
	}

	.insight-mini {
		display: flex;
		flex-direction: column;
		gap: var(--space-3xs);
		padding: var(--space-3xs) var(--space-xs);
		background: var(--color-bg-hover);
		border-radius: var(--radius-sm);
		min-width: 4.5rem;
	}

	.insight-mini-label {
		font-size: var(--text-2xs);
		color: var(--color-muted);
		text-transform: capitalize;
	}

	.insight-mini-value {
		font-size: var(--text-sm);
		font-weight: 600;
	}

	/* ── Bio ── */
	.account-bio {
		font-size: var(--text-xs);
		color: var(--color-muted);
		line-height: 1.4;
		margin: 0;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}

	/* ── Legacy ── */
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
