<script lang="ts">
	import PageHeader from '$lib/components/PageHeader.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import ScheduleDetail from '$lib/components/ScheduleDetail.svelte';
	import { Button } from '$lib/components/ui/button';
	import * as Select from '$lib/components/ui/select';
	import * as Tabs from '$lib/components/ui/tabs';
	import ChevronLeft from '@lucide/svelte/icons/chevron-left';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import { listSchedules, listAccounts } from '$lib/api';
	import type { Schedule, Account } from '$lib/types';
	import { toast } from '$lib/toast.svelte';
	import { getTimezone } from '$lib/tz';
	import { truncate } from '$lib/format';

	type ViewMode = 'week' | 'month';

	// View mode + anchor date. anchorDate is set client-only in a mount effect:
	// SSR has no meaningful "today" and a server-computed Date would hydration-mismatch.
	let view = $state<ViewMode>('week');
	let anchorDate = $state<Date | null>(null);
	let filterAccountId = $state('all');
	let schedules = $state<Schedule[]>([]);
	let accounts = $state<Account[]>([]);
	let loading = $state(false);
	let loadSeq = 0;

	// Detail modal (reuses the schedules page detail component incl. approve/reject/delete)
	let detailSchedule = $state<Schedule | null>(null);

	// ── Date helpers (grid computed in local time; items bucketed in display TZ) ──
	function startOfDay(d: Date): Date {
		const x = new Date(d);
		x.setHours(0, 0, 0, 0);
		return x;
	}
	function addDays(d: Date, n: number): Date {
		const x = new Date(d);
		x.setDate(x.getDate() + n);
		return x;
	}
	function startOfWeek(d: Date): Date {
		// Monday-based week
		const x = startOfDay(d);
		const dow = (x.getDay() + 6) % 7;
		return addDays(x, -dow);
	}
	function startOfMonth(d: Date): Date {
		const x = startOfDay(d);
		x.setDate(1);
		return x;
	}
	function endOfMonth(d: Date): Date {
		const x = startOfMonth(d);
		x.setMonth(x.getMonth() + 1);
		return addDays(x, -1);
	}
	function gridStart(d: Date, v: ViewMode): Date {
		return v === 'week' ? startOfWeek(d) : startOfWeek(startOfMonth(d));
	}
	function gridEnd(d: Date, v: ViewMode): Date {
		return v === 'week' ? addDays(startOfWeek(d), 6) : addDays(startOfWeek(endOfMonth(d)), 6);
	}

	// Display timezone (server TZ from /health, cached by root layout; browser TZ fallback)
	let displayTz = $derived(getTimezone() || Intl.DateTimeFormat().resolvedOptions().timeZone);

	/** YYYY-MM-DD key in the display timezone (en-CA formats as ISO-like). */
	function dayKey(d: Date): string {
		return new Intl.DateTimeFormat('en-CA', {
			timeZone: displayTz,
			year: 'numeric',
			month: '2-digit',
			day: '2-digit'
		}).format(d);
	}
	/**
	 * Cell identity, derived entirely in the display timezone so headers,
	 * today-highlight, in-month checks and item buckets share ONE date space.
	 * (Deriving labels from the local Date while bucketing in displayTz
	 * misplaces items whenever browser TZ differs from display TZ.)
	 */
	function cellMeta(d: Date): { key: string; dayNum: number; dayName: string; month: string } {
		const key = dayKey(d);
		const dayName = new Intl.DateTimeFormat('en-US', {
			timeZone: displayTz,
			weekday: 'short'
		}).format(d);
		return { key, dayNum: Number(key.slice(8, 10)), dayName, month: key.slice(5, 7) };
	}

	function timeLabel(iso: string): string {
		return new Intl.DateTimeFormat('en-US', {
			timeZone: displayTz,
			hour: 'numeric',
			minute: '2-digit'
		}).format(new Date(iso));
	}

	// ── Grid structure ──
	const weekDays = $derived.by(() => {
		if (!anchorDate) return [];
		const start = startOfWeek(anchorDate);
		return Array.from({ length: 7 }, (_, i) => addDays(start, i));
	});

	const monthWeeks = $derived.by(() => {
		if (!anchorDate) return [];
		const start = gridStart(anchorDate, 'month');
		const end = gridEnd(anchorDate, 'month');
		const weeks: Date[][] = [];
		let cursor = start;
		while (cursor <= end) {
			weeks.push(Array.from({ length: 7 }, (_, i) => addDays(cursor, i)));
			cursor = addDays(cursor, 7);
		}
		return weeks;
	});

	/** Items grouped by display-TZ date key, each bucket sorted by scheduled time. */
	const itemsByDay = $derived.by(() => {
		const map = new Map<string, Schedule[]>();
		for (const s of schedules) {
			const key = dayKey(new Date(s.scheduled_at));
			const bucket = map.get(key);
			if (bucket) bucket.push(s);
			else map.set(key, [s]);
		}
		for (const bucket of map.values()) {
			bucket.sort((a, b) => a.scheduled_at.localeCompare(b.scheduled_at));
		}
		return map;
	});

	const todayKey = $derived(anchorDate ? dayKey(new Date()) : '');
	const anchorMonth = $derived(anchorDate ? dayKey(anchorDate).slice(5, 7) : '');

	const rangeLabel = $derived.by(() => {
		if (!anchorDate) return '';
		if (view === 'week') {
			const s = startOfWeek(anchorDate);
			const e = addDays(s, 6);
			const fmt = (d: Date, withYear: boolean) =>
				new Intl.DateTimeFormat('en-US', {
					timeZone: displayTz,
					month: 'short',
					day: 'numeric',
					...(withYear ? { year: 'numeric' } : {})
				}).format(d);
			const sk = dayKey(s);
			const ek = dayKey(e);
			const sameMonth = sk.slice(0, 7) === ek.slice(0, 7);
			return sameMonth
				? `${fmt(s, false)} \u2013 ${ek.slice(8, 10)}, ${ek.slice(0, 4)}`
				: `${fmt(s, false)} \u2013 ${fmt(e, true)}`;
		}
		return new Intl.DateTimeFormat('en-US', {
			timeZone: displayTz,
			month: 'long',
			year: 'numeric'
		}).format(anchorDate);
	});

	// ── Navigation ──
	function goPrev() {
		if (!anchorDate) return;
		anchorDate =
			view === 'week'
				? addDays(anchorDate, -7)
				: addDays(startOfMonth(anchorDate), -1);
	}
	function goNext() {
		if (!anchorDate) return;
		anchorDate =
			view === 'week'
				? addDays(anchorDate, 7)
				: addDays(endOfMonth(anchorDate), 1);
	}
	function goToday() {
		anchorDate = new Date();
	}

	// ── Data fetching ──
	// Fetch window = visible grid ±1 day padding so items near TZ boundaries are
	// always present in state (backend filters on stored UTC scheduled_at).
	async function loadRange(fromISO: string, toISO: string, accountId: string) {
		const seq = ++loadSeq;
		loading = true;
		try {
			const data = await listSchedules({
				from: fromISO,
				to: toISO,
				account_id: accountId === 'all' ? undefined : accountId,
				limit: 1000
			});
			if (seq !== loadSeq) return; // stale response — a newer range superseded it
			schedules = data;
		} catch (e: any) {
			if (seq !== loadSeq) return;
			toast(e.message || 'Failed to load schedules', 'error');
		} finally {
			if (seq === loadSeq) loading = false;
		}
	}

	// Refetch for in-page actions (approve/reject/delete via ScheduleDetail)
	function reload() {
		if (!anchorDate) return;
		const fromISO = addDays(gridStart(anchorDate, view), -1).toISOString();
		const toISO = addDays(gridEnd(anchorDate, view), 1).toISOString();
		loadRange(fromISO, toISO, filterAccountId);
	}

	// Fetch whenever the visible range or account filter changes.
	// Reads only reactive deps here; state writes happen inside loadRange's async
	// callbacks (outside the tracking window) — no re-fetch loop.
	$effect(() => {
		if (!anchorDate) return;
		void view; // track — view switch changes the fetch window
		const accountId = filterAccountId; // track — refetch on account change
		const d = anchorDate;
		const fromISO = addDays(gridStart(d, view), -1).toISOString();
		const toISO = addDays(gridEnd(d, view), 1).toISOString();
		loadRange(fromISO, toISO, accountId);
	});

	// Mount (client-only): set the initial anchor (current week) + load account list once.
	$effect(() => {
		anchorDate = startOfWeek(new Date());
		listAccounts()
			.then((a) => (accounts = a))
			.catch(() => {
				/* account filter just stays empty */
			});
	});
</script>

<div class="calendar-page">
	<PageHeader
		title="Content Calendar"
		description="Weekly and monthly view of scheduled posts with review actions."
	>
		{#snippet action()}
			<span class="range-label">{rangeLabel}</span>
		{/snippet}
	</PageHeader>

	<div class="toolbar">
		<div class="nav-buttons" role="group" aria-label="Calendar navigation">
			<Button variant="outline" size="sm" onclick={goPrev} aria-label="Previous" disabled={!anchorDate}>
				<ChevronLeft class="size-4" />
			</Button>
			<Button variant="outline" size="sm" onclick={goToday} disabled={!anchorDate}>Today</Button>
			<Button variant="outline" size="sm" onclick={goNext} aria-label="Next" disabled={!anchorDate}>
				<ChevronRight class="size-4" />
			</Button>
		</div>

		<div class="form-group">
			<label class="form-label" for="calendar-account">Account</label>
			<Select.Root type="single" bind:value={filterAccountId}>
				<Select.Trigger class="account-trigger" id="calendar-account">
					{filterAccountId === 'all'
						? 'All accounts'
						: accounts.find((a) => a.id === filterAccountId)?.username ?? 'Unknown'}
				</Select.Trigger>
				<Select.Content>
					<Select.Item value="all" label="All accounts">All accounts</Select.Item>
					{#each accounts as acct (acct.id)}
						<Select.Item value={acct.id} label={acct.username}>{acct.username}</Select.Item>
					{/each}
				</Select.Content>
			</Select.Root>
		</div>

		<Tabs.Root bind:value={view} class="view-tabs">
			<Tabs.List aria-label="Calendar view mode">
				<Tabs.Trigger value="week">Week</Tabs.Trigger>
				<Tabs.Trigger value="month">Month</Tabs.Trigger>
			</Tabs.List>
		</Tabs.Root>

		<span class="toolbar-meta" aria-live="polite">
			{#if loading}Loading…{:else}{schedules.length} scheduled{/if}
		</span>
	</div>

	{#if anchorDate}
		{#if view === 'week'}
			<div class="calendar-scroll">
				<div class="week-grid">
					{#each weekDays as day (dayKey(day))}
						{@const cell = cellMeta(day)}
						<section class="day-cell" class:today={cell.key === todayKey}>
							<header class="day-head">
								<span class="day-name">{cell.dayName}</span>
								<span class="day-num">{cell.dayNum}</span>
							</header>
							<div class="day-items">
								{#each itemsByDay.get(cell.key) ?? [] as s (s.id)}
									<button class="cal-item" type="button" onclick={() => (detailSchedule = s)}>
										<span class="cal-time">{timeLabel(s.scheduled_at)}</span>
										<span class="cal-caption">{truncate(s.caption || '(no caption)', 42)}</span>
										<StatusBadge status={s.status} />
									</button>
								{/each}
							</div>
						</section>
					{/each}
				</div>
			</div>
		{:else}
			<div class="calendar-scroll">
				<div class="month-grid">
					{#each monthWeeks[0] ?? [] as day, i (i)}
						<div class="month-dow">{cellMeta(day).dayName}</div>
					{/each}
					{#each monthWeeks as week, wi (wi)}
						{#each week as day (dayKey(day))}
							{@const cell = cellMeta(day)}
							{@const inMonth = cell.month === anchorMonth}
							<div class="month-cell" class:today={cell.key === todayKey} class:outside={!inMonth}>
								<div class="month-cell-head">
									<span class="day-num">{cell.dayNum}</span>
								</div>
								<div class="day-items">
									{#each itemsByDay.get(cell.key) ?? [] as s (s.id)}
										<button class="cal-item" type="button" onclick={() => (detailSchedule = s)}>
											<span class="cal-time">{timeLabel(s.scheduled_at)}</span>
											<span class="cal-caption">{truncate(s.caption || '(no caption)', 30)}</span>
											<StatusBadge status={s.status} />
										</button>
									{/each}
								</div>
							</div>
						{/each}
					{/each}
				</div>
			</div>
		{/if}
	{:else}
		<div class="calendar-loading">Loading calendar…</div>
	{/if}
</div>

{#if detailSchedule}
	<ScheduleDetail
		schedule={detailSchedule}
		onClose={() => (detailSchedule = null)}
		onAction={reload}
	/>
{/if}

<style>
	.calendar-page {
		display: flex;
		flex-direction: column;
		gap: var(--space-md);
	}

	.range-label {
		font-size: var(--text-sm);
		font-weight: 600;
		color: var(--color-ink);
		white-space: nowrap;
	}

	/* ── Toolbar ── */
	.toolbar {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-end;
		gap: var(--space-sm);
	}

	.nav-buttons {
		display: flex;
		gap: var(--space-3xs);
	}

	.toolbar .form-group {
		display: flex;
		flex-direction: column;
		gap: var(--space-3xs);
	}

	.account-trigger {
		min-width: 10rem;
	}

	.view-tabs {
		margin-inline-start: auto;
	}

	.toolbar-meta {
		font-size: var(--text-xs);
		color: var(--color-muted);
		align-self: center;
		white-space: nowrap;
	}

	/* ── Grid shell ── */
	.calendar-scroll {
		overflow-x: auto;
		border: var(--rule-default);
		border-radius: var(--radius-md);
		background: var(--rule-subtle); /* hairline gaps between cells */
	}

	.week-grid {
		display: grid;
		grid-template-columns: repeat(7, minmax(8.5rem, 1fr));
		gap: 1px;
		min-width: 56rem;
	}

	.month-grid {
		display: grid;
		grid-template-columns: repeat(7, minmax(7rem, 1fr));
		gap: 1px;
		min-width: 49rem;
	}

	.month-dow {
		padding: var(--space-3xs) var(--space-xs);
		font-size: var(--text-xs);
		font-weight: 600;
		color: var(--color-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		background: var(--surface-sunken);
	}

	/* ── Day cells ── */
	.day-cell,
	.month-cell {
		display: flex;
		flex-direction: column;
		gap: var(--space-3xs);
		padding: var(--space-xs);
		background: var(--surface-raised);
		min-height: 7.5rem;
	}

	.month-cell {
		min-height: 6rem;
	}

	.month-cell.outside {
		background: var(--surface-sunken);
	}

	.month-cell.outside .day-num {
		color: var(--color-neutral);
	}

	.day-head,
	.month-cell-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-2xs);
	}

	.day-name {
		font-size: var(--text-xs);
		font-weight: 600;
		color: var(--color-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.day-num {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		min-width: 1.5rem;
		height: 1.5rem;
		border-radius: 999px;
		font-size: var(--text-xs);
		font-variant-numeric: tabular-nums;
		color: var(--color-ink);
	}

	.day-cell.today .day-num,
	.month-cell.today .day-num {
		background: var(--color-accent);
		color: var(--color-accent-ink);
		font-weight: 700;
	}

	/* ── Items ── */
	.day-items {
		display: flex;
		flex-direction: column;
		gap: var(--space-3xs);
		overflow-y: auto;
		max-height: 9rem;
	}

	.cal-item {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: var(--space-3xs);
		width: 100%;
		padding: var(--space-3xs) var(--space-2xs);
		text-align: left;
		background: var(--surface-sunken);
		border: var(--rule-subtle);
		border-radius: var(--radius-sm);
		cursor: pointer;
		transition: background var(--dur-short) var(--ease-out);
	}

	.cal-item:hover {
		background: var(--color-paper-2);
	}

	.cal-time {
		font-size: var(--text-xs);
		font-family: var(--font-mono);
		color: var(--color-muted);
		font-variant-numeric: tabular-nums;
	}

	.cal-caption {
		font-size: var(--text-xs);
		color: var(--color-ink);
		overflow: hidden;
		display: -webkit-box;
		-webkit-line-clamp: 1;
		-webkit-box-orient: vertical;
		overflow-wrap: anywhere;
	}

	.calendar-loading {
		display: flex;
		align-items: center;
		justify-content: center;
		min-height: 40vh;
		color: var(--color-muted);
		font-size: var(--text-sm);
	}

	@media (max-width: 640px) {
		.view-tabs {
			margin-inline-start: 0;
		}
	}
</style>
