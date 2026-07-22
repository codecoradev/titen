<script lang="ts">
	import PageHeader from '$lib/components/PageHeader.svelte';
	import StatSkeleton from '$lib/components/StatSkeleton.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import { getHealth, listAccounts, listPosts, listSchedules, getUpcomingSchedules } from '$lib/api';
	import { toast } from '$lib/toast.svelte';
	import type { Account, Post, Schedule, HealthCheck } from '$lib/types';

	let loading = $state(true);
	let health = $state<HealthCheck | null>(null);
	let accounts = $state<Account[]>([]);
	let posts = $state<Post[]>([]);
	let schedules = $state<Schedule[]>([]);
	let upcoming = $state<Schedule[]>([]);

	let activeAccounts = $derived(accounts.filter((a) => a.status === 'active').length);
	let publishedPosts = $derived(posts.filter((p) => p.status === 'published').length);
	let pendingSchedules = $derived(schedules.filter((s) => s.status === 'pending').length);

	let recentPosts = $derived(
		[...posts].sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime()).slice(0, 5),
	);

	function formatUptime(seconds: number): string {
		if (seconds < 60) return `${seconds}s`;
		if (seconds < 3600) return `${Math.floor(seconds / 60)}m ${seconds % 60}s`;
		const h = Math.floor(seconds / 3600);
		const m = Math.floor((seconds % 3600) / 60);
		return `${h}h ${m}m`;
	}

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
				listAccounts().then((r) => r.data).catch(() => []),
				listPosts().then((r) => r.data).catch(() => []),
				listSchedules().then((r) => r.data).catch(() => []),
				getUpcomingSchedules().then((r) => r.data).catch(() => []),
			]);
			health = h;
			accounts = a;
			posts = p;
			schedules = s;
			upcoming = u;
		} catch (e: any) {
			toast('Failed to load dashboard data', 'error');
		} finally {
			loading = false;
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
		<div class="stat-card">
			<div class="stat-card-label">Pending Schedules</div>
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
				<div class="stat-card-label">Uptime</div>
				<div class="stat-card-value tabular-nums">{formatUptime(health.uptime)}</div>
			</div>
		{/if}
	</div>

	<!-- Token health -->
	{#if accounts.length > 0}
		<section style="margin-bottom: var(--space-lg);">
			<h2 style="font-size: var(--text-md); font-weight: 600; margin-bottom: var(--space-sm);">Token Health</h2>
			<div class="data-table-wrap">
				<div class="token-list">
					{#each accounts as account}
						<div class="token-row">
							<span class="token-username">{account.display_name || account.username}</span>
							<StatusBadge status={account.status} />
							<span class="token-expiry">
								{#if account.token_expires_at}
									{#if isTokenExpiring(account.token_expires_at)}
										<span class="badge badge--warning">Expires {formatDate(account.token_expires_at)}</span>
									{:else}
										{formatDate(account.token_expires_at)}
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

	<!-- Two-column: Recent posts + Upcoming schedules -->
	<div style="display: grid; grid-template-columns: 1fr 1fr; gap: var(--space-lg);">
		<!-- Recent posts -->
		<section>
			<h2 style="font-size: var(--text-md); font-weight: 600; margin-bottom: var(--space-sm);">Recent Posts</h2>
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
			<h2 style="font-size: var(--text-md); font-weight: 600; margin-bottom: var(--space-sm);">Upcoming</h2>
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

	@media (max-width: 48rem) {
		div[style*='grid-template-columns: 1fr 1fr'] {
			grid-template-columns: 1fr !important;
		}
	}
</style>
