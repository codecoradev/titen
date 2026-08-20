<script lang="ts">
	import PageHeader from '$lib/components/PageHeader.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import ScheduleDetail from '$lib/components/ScheduleDetail.svelte';
	import {
		listSchedules,
		createSchedule,
		deleteSchedule,
		patchSchedule,
		approveSchedule,
		rejectSchedule
	} from '$lib/api';
	import { listAccounts } from '$lib/api';
	import type { Schedule, Account } from '$lib/types';
	import { toast } from '$lib/toast.svelte';
	import { formatDateTime, toDatetimeInput, getTimezone } from '$lib/tz';
	import { truncate } from '$lib/format';
	import { Button } from '$lib/components/ui/button';
	import * as Select from '$lib/components/ui/select';
	import DataTable from '$lib/components/DataTable.svelte';
	import Textarea from '$lib/components/ui/textarea/textarea.svelte';

	type StatusFilter = 'all' | 'draft' | 'pending' | 'processing' | 'published' | 'failed' | 'rejected';

	let schedules = $state<Schedule[]>([]);
	let accounts = $state<Account[]>([]);
	let loading = $state(true);
	let loaded = $state(false);
	let creating = $state(false);
	let loadSeq = 0; // guards against stale async responses

	// Detail modal
	let detailSchedule = $state<Schedule | null>(null);

	// View mode: table (default) or cards (Rungu moderation style).
	// Init client-only (in $effect) to avoid SSR hydration mismatch.
	type ViewMode = 'table' | 'cards';
	let viewMode = $state<ViewMode>('table');
	$effect(() => {
		if (typeof localStorage !== 'undefined' && localStorage.getItem('schedules-view') === 'cards') {
			viewMode = 'cards';
		}
	});
	function setViewMode(m: ViewMode) {
		viewMode = m;
		try { localStorage.setItem('schedules-view', m); } catch { /* ignore */ }
	}

	function openDetail(s: Schedule) {
		detailSchedule = s;
	}
	function closeDetail() {
		detailSchedule = null;
	}
	async function onDetailAction() {
		await loadData();
	}

	let filterAccountId = $state('');
	let filterStatus = $state<StatusFilter>('all');
	let filterFrom = $state('');
	let filterTo = $state('');
	let filterSearch = $state('');
	let filterMediaType = $state<'all' | 'TEXT' | 'IMAGE' | 'CAROUSEL'>('all');

	const hasActiveFilters = $derived(
		filterAccountId !== '' ||
			filterStatus !== 'all' ||
			filterFrom !== '' ||
			filterTo !== '' ||
			filterSearch.trim() !== '' ||
			filterMediaType !== 'all'
	);

	function resetFilters() {
		filterAccountId = '';
		filterStatus = 'all';
		filterFrom = '';
		filterTo = '';
		filterSearch = '';
		filterMediaType = 'all';
	}

	// Debounce search input so typing doesn't refetch every keystroke
	let searchTimer: ReturnType<typeof setTimeout> | undefined;
	$effect(() => {
		void filterSearch; // track dependency
		clearTimeout(searchTimer);
		if (!loaded) return;
		searchTimer = setTimeout(() => loadData(), 300);
		return () => clearTimeout(searchTimer);
	});

	// Timezone display label
	let tzLabel = $derived(getTimezone() || 'Browser TZ');

	// Create modal
	let modalOpen = $state(false);
	let modalAccountId = $state('');
	let modalScheduledAt = $state('');
	let modalCaption = $state('');
	let modalMediaType = $state<'text' | 'IMAGE' | 'CAROUSEL'>('text');
	let modalImageUrl = $state('');
	let modalCarouselUrls = $state<string[]>(['', '']);

	// Edit modal (HITL)
	let editTarget = $state<Schedule | null>(null);
	let editCaption = $state('');
	let editScheduledAt = $state('');
	let editing = $state(false);

	// Delete
	let deleteTarget = $state<Schedule | null>(null);
	let deleting = $state(false);

	// Reject
	let rejectTarget = $state<Schedule | null>(null);
	let rejectReason = $state('');
	let rejecting = $state(false);

	// Approve loading state
	let approvingId = $state<string | null>(null);

	async function loadData() {
		loading = true;
		try {
			const params: {
				account_id?: string;
				status?: string;
				from?: string;
				to?: string;
				search?: string;
				media_type?: string;
			} = {};
			if (filterAccountId) params.account_id = filterAccountId;
			if (filterStatus !== 'all') params.status = filterStatus;
			// Date-only strings parse as UTC midnight — append local time to anchor to user TZ
			if (filterFrom) params.from = new Date(`${filterFrom}T00:00:00`).toISOString();
			if (filterTo) params.to = new Date(`${filterTo}T23:59:59`).toISOString();
			if (filterSearch.trim()) params.search = filterSearch.trim();
			if (filterMediaType !== 'all') params.media_type = filterMediaType;

			const reqId = ++loadSeq;
			const [schedulesData, accountsData] = await Promise.all([
				listSchedules(params),
				listAccounts()
			]);
			if (reqId !== loadSeq) return; // stale response — a newer request superseded it

			schedules = schedulesData;
			accounts = accountsData;
		} catch (e: any) {
			toast(e.message || 'Failed to load schedules', 'error');
		} finally {
			loading = false;
		}
	}

	function openCreateModal() {
		modalAccountId = accounts.length > 0 ? accounts[0].id : '';
		modalScheduledAt = '';
		modalCaption = '';
		modalMediaType = 'text';
		modalImageUrl = '';
		modalCarouselUrls = ['', ''];
		modalOpen = true;
	}

	function closeCreateModal() {
		modalOpen = false;
	}

	function addCarouselUrl() {
		if (modalCarouselUrls.length < 20) {
			modalCarouselUrls = [...modalCarouselUrls, ''];
		}
	}

	function removeCarouselUrl(idx: number) {
		if (modalCarouselUrls.length > 2) {
			modalCarouselUrls = modalCarouselUrls.filter((_, i) => i !== idx);
		}
	}

	async function handleCreate() {
		if (!modalAccountId || !modalScheduledAt) {
			toast('Account and scheduled time are required', 'error');
			return;
		}

		// Validate media URLs based on type
		let mediaType = 'TEXT';
		let mediaUrls: string | undefined;

		if (modalMediaType === 'IMAGE') {
			if (!modalImageUrl.trim()) {
				toast('Image URL is required for IMAGE posts', 'error');
				return;
			}
			mediaType = 'IMAGE';
			mediaUrls = modalImageUrl.trim();
		} else if (modalMediaType === 'CAROUSEL') {
			const validUrls = modalCarouselUrls.filter((u) => u.trim());
			if (validUrls.length < 2) {
				toast('Carousel requires at least 2 image URLs', 'error');
				return;
			}
			if (validUrls.length > 20) {
				toast('Carousel supports a maximum of 20 images', 'error');
				return;
			}
			mediaType = 'CAROUSEL';
			mediaUrls = validUrls.join(',');
		}

		creating = true;
		try {
			await createSchedule({
				account_id: modalAccountId,
				media_type: mediaType,
				scheduled_at: new Date(modalScheduledAt).toISOString(),
				caption: modalCaption || undefined,
				media_urls: mediaUrls || undefined
			});
			toast('Schedule created as draft', 'success');
			closeCreateModal();
			await loadData();
		} catch (e: any) {
			toast(e.message || 'Failed to create schedule', 'error');
		} finally {
			creating = false;
		}
	}

	// HITL: Edit schedule
	function openEditModal(schedule: Schedule) {
		editTarget = schedule;
		editCaption = schedule.caption ?? '';
		editScheduledAt = toDatetimeInput(schedule.scheduled_at);
		editing = false;
	}

	function closeEditModal() {
		editTarget = null;
	}

	async function handleEditSave() {
		if (!editTarget) return;
		editing = true;
		try {
			await patchSchedule(editTarget.id, {
				caption: editCaption || undefined,
				scheduled_at: new Date(editScheduledAt).toISOString()
			});
			toast('Schedule updated', 'success');
			editTarget = null;
			await loadData();
		} catch (e: any) {
			toast(e.message || 'Failed to update schedule', 'error');
		} finally {
			editing = false;
		}
	}

	// Optimistic status update helper (Rungu moderation pattern)
	function optimisticStatus(id: string, status: Schedule['status']) {
		const prev = schedules.find((s) => s.id === id)?.status;
		schedules = schedules.map((s) => (s.id === id ? { ...s, status } : s));
		return prev;
	}

	function revertStatus(id: string, prev: Schedule['status'] | undefined) {
		if (prev === undefined) return;
		schedules = schedules.map((s) => (s.id === id ? { ...s, status: prev } : s));
	}

	// HITL: Approve (optimistic)
	async function handleApprove(schedule: Schedule) {
		approvingId = schedule.id;
		const prev = optimisticStatus(schedule.id, 'approved');
		try {
			await approveSchedule(schedule.id);
			toast('Schedule approved — will auto-publish when due', 'success');
		} catch (e: any) {
			revertStatus(schedule.id, prev);
			toast(e.message || 'Failed to approve schedule', 'error');
		} finally {
			approvingId = null;
		}
	}

	// HITL: Reject
	function openRejectModal(schedule: Schedule) {
		rejectTarget = schedule;
		rejectReason = '';
	}

	function closeRejectModal() {
		rejectTarget = null;
	}

	async function handleReject() {
		if (!rejectTarget) return;
		rejecting = true;
		const target = rejectTarget;
		const prev = optimisticStatus(target.id, 'rejected');
		try {
			await rejectSchedule(target.id, rejectReason || undefined);
			toast('Schedule rejected', 'success');
			rejectTarget = null;
		} catch (e: any) {
			revertStatus(target.id, prev);
			toast(e.message || 'Failed to reject schedule', 'error');
		} finally {
			rejecting = false;
		}
	}

	// Delete
	function confirmDelete(schedule: Schedule) {
		deleteTarget = schedule;
	}

	function cancelDelete() {
		deleteTarget = null;
	}

	async function handleDelete() {
		if (!deleteTarget) return;
		deleting = true;
		try {
			await deleteSchedule(deleteTarget.id);
			toast('Schedule deleted', 'success');
			deleteTarget = null;
			await loadData();
		} catch (e: any) {
			toast(e.message || 'Failed to delete schedule', 'error');
		} finally {
			deleting = false;
		}
	}

	// Format schedule time using TZ from backend
	function fmtDate(iso: string): string {
		return formatDateTime(iso);
	}

	// Count drafts for badge
	let draftCount = $derived(schedules.filter((s) => s.status === 'draft').length);

	// DataTable columns (labels only — cells rendered via snippet)
	const columns = [
		{ key: 'content', label: 'Content', class: 'truncate' },
		{ key: 'account_id', label: 'Account' },
		{ key: 'scheduled_at', label: 'Scheduled', sortable: true },
		{ key: 'status', label: 'Status' },
		{ key: 'error', label: 'Error' }
	];

	// Refetch when any non-search filter changes (search has its own debounced effect)
	$effect(() => {
		void filterAccountId;
		void filterStatus;
		void filterFrom;
		void filterTo;
		void filterMediaType;
		if (!loaded) return;
		loadData();
	});

	$effect(() => {
		if (!loaded) {
			loadData();
			loaded = true;
		}
	});
</script>

<div class="schedules-page">
	<PageHeader title="Schedules">
		{#snippet action()}
			{#if draftCount > 0}
				<span class="badge-draft">{draftCount} draft{draftCount > 1 ? 's' : ''} pending review</span>
			{/if}
			<Button variant="default" onclick={openCreateModal}>New Schedule</Button>
		{/snippet}
	</PageHeader>

	<!-- Filters -->
	<div class="filter-bar">
		<div class="form-group">
			<label class="form-label" for="filter-search">Search</label>
			<input
				id="filter-search"
				class="form-input"
				type="search"
				placeholder="Search caption…"
				bind:value={filterSearch}
			/>
		</div>

		<div class="form-group">
			<label class="form-label">Account</label>
			<Select.Root type="single" bind:value={filterAccountId}>
				<Select.Trigger>
					{filterAccountId ? accounts.find((a) => a.id === filterAccountId)?.username ?? 'Unknown' : 'All accounts'}
				</Select.Trigger>
				<Select.Content>
					<Select.Item value="" label="All accounts">All accounts</Select.Item>
					{#each accounts as acct (acct.id)}
						<Select.Item value={acct.id} label={acct.username}>
							{acct.username}
						</Select.Item>
					{/each}
				</Select.Content>
			</Select.Root>
		</div>

		<div class="form-group">
			<label class="form-label">Status</label>
			<Select.Root type="single" bind:value={filterStatus}>
				<Select.Trigger>
					{filterStatus === 'all' ? 'All' : filterStatus === 'draft' ? 'Draft (Needs Review)' : filterStatus === 'pending' ? 'Pending (Approved)' : filterStatus === 'processing' ? 'Processing' : filterStatus === 'published' ? 'Published' : filterStatus === 'failed' ? 'Failed' : filterStatus === 'rejected' ? 'Rejected' : 'All'}
				</Select.Trigger>
				<Select.Content>
					<Select.Item value="all" label="All">All</Select.Item>
					<Select.Item value="draft" label="Draft (Needs Review)">Draft (Needs Review)</Select.Item>
					<Select.Item value="pending" label="Pending (Approved)">Pending (Approved)</Select.Item>
					<Select.Item value="processing" label="Processing">Processing</Select.Item>
					<Select.Item value="published" label="Published">Published</Select.Item>
					<Select.Item value="failed" label="Failed">Failed</Select.Item>
					<Select.Item value="rejected" label="Rejected">Rejected</Select.Item>
				</Select.Content>
			</Select.Root>
		</div>

		<div class="form-group">
			<label class="form-label" for="filter-from">From</label>
			<input
				id="filter-from"
				class="form-input"
				type="date"
				bind:value={filterFrom}
			/>
		</div>

		<div class="form-group">
			<label class="form-label" for="filter-to">To</label>
			<input
				id="filter-to"
				class="form-input"
				type="date"
				bind:value={filterTo}
			/>
		</div>

		<div class="form-group">
			<label class="form-label">Media Type</label>
			<Select.Root type="single" bind:value={filterMediaType}>
				<Select.Trigger>
					{filterMediaType === 'all' ? 'All' : filterMediaType === 'TEXT' ? 'Text Only' : filterMediaType === 'IMAGE' ? 'Single Image' : 'Carousel'}
				</Select.Trigger>
				<Select.Content>
					<Select.Item value="all" label="All">All</Select.Item>
					<Select.Item value="TEXT" label="Text Only">Text Only</Select.Item>
					<Select.Item value="IMAGE" label="Single Image">Single Image</Select.Item>
					<Select.Item value="CAROUSEL" label="Carousel">Carousel</Select.Item>
				</Select.Content>
			</Select.Root>
		</div>

			{#if hasActiveFilters}
			<div class="form-group">
				<label class="form-label">&nbsp;</label>
				<Button variant="outline" size="sm" onclick={resetFilters}>Reset filters</Button>
			</div>
		{/if}

		<div class="filter-count" aria-live="polite">
			{schedules.length} schedule{schedules.length === 1 ? '' : 's'}
		</div>

		<div class="view-toggle" role="group" aria-label="View mode">
			<Button
				variant={viewMode === 'table' ? 'default' : 'outline'}
				size="sm"
				onclick={() => setViewMode('table')}
				aria-pressed={viewMode === 'table'}
				title="Table view"
			>
				☰
			</Button>
			<Button
				variant={viewMode === 'cards' ? 'default' : 'outline'}
				size="sm"
				onclick={() => setViewMode('cards')}
				aria-pressed={viewMode === 'cards'}
				title="Card view"
			>
				▦
			</Button>
		</div>
	</div>

	<!-- Table -->
	{#if schedules.length === 0 && !loading}
		<EmptyState
			icon="posts"
			title="No schedules yet"
			description="Create your first scheduled post. New schedules start as drafts and need approval before publishing."
		>
			{#snippet action()}
				<Button variant="default" size="sm" onclick={openCreateModal}>New Schedule</Button>
			{/snippet}
		</EmptyState>
	{:else if viewMode === 'table'}
		<DataTable
					columns={columns}
					rows={schedules}
					loading={loading}
					emptyTitle="No schedules match filters"
					expandable
					rowClass={(s) => (s.status === 'draft' ? 'row-draft' : '')}
				>
				{#snippet detail(s)}
					<div class="detail-panel">
						{#if s.caption}
							<p class="detail-caption">{s.caption}</p>
						{/if}
						{#if s.media_urls}
							<div class="detail-media">
								{#each s.media_urls.split(',').filter(Boolean) as url}
									<img
										src={url}
										alt="Media preview"
										class="detail-thumb"
										loading="lazy"
										onerror={(e) => { const t = e.currentTarget as HTMLImageElement; t.style.display = 'none'; }}
									/>
								{/each}
							</div>
						{/if}
						{#if s.error}
							<p class="detail-error">{s.error}</p>
						{/if}
						<dl class="detail-meta">
							<div><dt>Scheduled</dt><dd>{fmtDate(s.scheduled_at)}</dd></div>
							<div><dt>Type</dt><dd>{s.media_type}</dd></div>
							<div><dt>Created</dt><dd>{fmtDate(s.created_at)}</dd></div>
							{#if s.published_at}<div><dt>Published</dt><dd>{fmtDate(s.published_at)}</dd></div>{/if}
						</dl>
					</div>
				{/snippet}
				{#snippet cell(s, key)}
					{#if key === 'content'}
						<div class="caption-cell" title={s.caption || '—'}>
							{#if s.media_urls}
								{#each s.media_urls.split(',').filter(Boolean).slice(0, 3) as url, i}
									<img
										src={url}
										alt="Preview"
										class="row-thumb"
										loading="lazy"
										onerror={(e) => { const t = e.currentTarget as HTMLImageElement; t.style.display = 'none'; }}
									/>
									{#if i === 2 && s.media_urls.split(',').filter(Boolean).length > 3}
										<span class="thumb-more">+{s.media_urls.split(',').filter(Boolean).length - 3}</span>
									{/if}
								{/each}
							{/if}
							<span>{truncate(s.caption || '—', 60)}</span>
						</div>
					{:else if key === 'account_id'}
						{accounts.find((a) => a.id === s.account_id)?.username ?? (s.account_id ?? '—').slice(0, 8)}
					{:else if key === 'scheduled_at'}
						<span class="tabular-nums">{fmtDate(s.scheduled_at)}</span>
					{:else if key === 'status'}
						<StatusBadge status={s.status} />
					{:else if key === 'error'}
						<span class="col-error" title={s.error ?? ''}>{s.error ?? '—'}</span>
					{/if}
				{/snippet}
				{#snippet actions(s)}
					<Button variant="ghost" size="sm" onclick={() => openDetail(s)} title="Full detail">Detail</Button>
					{#if s.status === 'draft'}
						<Button
							variant="default"
							size="sm"
							class="bg-[var(--color-success)]"
							onclick={() => handleApprove(s)}
							disabled={approvingId === s.id}
						>
							{approvingId === s.id ? '…' : 'Approve'}
						</Button>
						<Button variant="ghost" size="sm" onclick={() => openEditModal(s)} title="Edit">Edit</Button>
						<Button variant="destructive" size="sm" onclick={() => openRejectModal(s)} title="Reject">Reject</Button>
					{:else if s.status === 'pending'}
						<Button variant="ghost" size="sm" onclick={() => openEditModal(s)} title="Edit">Edit</Button>
						<Button variant="ghost" size="sm" onclick={() => confirmDelete(s)} disabled={deleting} title="Cancel schedule">Cancel</Button>
					{:else}
						<Button variant="ghost" size="sm" onclick={() => confirmDelete(s)} disabled={deleting} title="Delete">Delete</Button>
					{/if}
				{/snippet}
				</DataTable>
	{:else if viewMode === 'cards'}
		{#if loading && schedules.length === 0}
			<div class="card-grid">
				{#each Array(3) as _}
					<div class="sched-card skeleton-card"><div class="skel-line w-60"></div><div class="skel-line"></div><div class="skel-line w-40"></div></div>
				{/each}
			</div>
		{:else}
			<div class="card-grid">
				{#each schedules as s (s.id)}
					<article class="sched-card" class:draft={s.status === 'draft'}>
						<header class="sched-card-head">
							<StatusBadge status={s.status} />
							<span class="sched-card-time">{fmtDate(s.scheduled_at)}</span>
						</header>

						{#if s.media_urls}
							<div class="sched-card-media">
								{#each s.media_urls.split(',').filter(Boolean).slice(0, 4) as url}
									<img
										src={url}
										alt="Media preview"
										loading="lazy"
										onerror={(e) => { const t = e.currentTarget as HTMLImageElement; t.style.display = 'none'; }}
									/>
								{/each}
							</div>
						{/if}

						<p class="sched-card-caption">{truncate(s.caption || '—', 140)}</p>

						<footer class="sched-card-foot">
							<div class="sched-card-meta">
								<span class="chip">{accounts.find((a) => a.id === s.account_id)?.username ?? s.account_id.slice(0, 8)}</span>
								<span class="chip">{s.media_type}</span>
							</div>
							<div class="sched-card-actions">
								<Button variant="ghost" size="sm" onclick={() => openDetail(s)} title="Full detail">Detail</Button>
								{#if s.status === 'draft'}
									<Button
										variant="default"
										size="sm"
										class="bg-[var(--color-success)]"
										onclick={() => handleApprove(s)}
										disabled={approvingId === s.id}
									>
										{approvingId === s.id ? '…' : 'Approve'}
									</Button>
									<Button variant="ghost" size="sm" onclick={() => openEditModal(s)}>Edit</Button>
									<Button variant="destructive" size="sm" onclick={() => openRejectModal(s)}>Reject</Button>
								{:else if s.status === 'pending'}
									<Button variant="ghost" size="sm" onclick={() => openEditModal(s)}>Edit</Button>
									<Button variant="ghost" size="sm" onclick={() => confirmDelete(s)} disabled={deleting}>Cancel</Button>
								{:else}
									<Button variant="ghost" size="sm" onclick={() => confirmDelete(s)} disabled={deleting}>Delete</Button>
								{/if}
							</div>
						</footer>

						{#if s.error}
							<p class="sched-card-error">{s.error}</p>
						{/if}
					</article>
				{:else}
					<EmptyState
						icon="posts"
						title="No schedules match filters"
						description="Try adjusting or resetting the filters above."
					/>
				{/each}
			</div>
		{/if}
	{/if}
</div>

<!-- Create Schedule Modal -->
{#if modalOpen}
	<div class="confirm-overlay" onclick={closeCreateModal} role="dialog" aria-modal="true" aria-label="New Schedule">
		<div class="confirm-dialog modal-narrow" onclick={(e) => e.stopPropagation()}>
			<h3>New Schedule</h3>
			<p class="modal-desc">
				New schedules are created as <strong>draft</strong>. You'll need to approve them before they can be published.
			</p>
			<div class="modal-stack">
				<div class="form-group">
					<label class="form-label">Account <span class="required">*</span></label>
					<Select.Root type="single" bind:value={modalAccountId}>
						<Select.Trigger>
							{modalAccountId ? accounts.find((a) => a.id === modalAccountId)?.username ?? 'Unknown' : 'Select account...'}
						</Select.Trigger>
						<Select.Content>
							{#each accounts as acct (acct.id)}
								<Select.Item value={acct.id} label={acct.username}>
									{acct.username}
								</Select.Item>
							{/each}
						</Select.Content>
					</Select.Root>
				</div>

				<div class="form-row">
					<div class="form-group flex-1">
						<label class="form-label" for="modal-scheduled">Scheduled At <span class="required">*</span></label>
						<input
							class="form-input"
							type="datetime-local"
							id="modal-scheduled"
							bind:value={modalScheduledAt}
						/>
						<span class="form-helper">Times shown in {tzLabel}</span>
					</div>

					<div class="form-group flex-1">
						<label class="form-label">Media Type</label>
						<Select.Root type="single" bind:value={modalMediaType}>
							<Select.Trigger>
								{modalMediaType === 'text' ? 'Text Only' : modalMediaType === 'IMAGE' ? 'Single Image' : modalMediaType === 'CAROUSEL' ? 'Carousel (2-20)' : 'Text Only'}
							</Select.Trigger>
							<Select.Content>
								<Select.Item value="text" label="Text Only">Text Only</Select.Item>
								<Select.Item value="IMAGE" label="Single Image">Single Image</Select.Item>
								<Select.Item value="CAROUSEL" label="Carousel (2-20)">Carousel (2-20)</Select.Item>
							</Select.Content>
						</Select.Root>
					</div>
				</div>

				{#if modalMediaType === 'IMAGE'}
					<div class="form-group">
						<label class="form-label" for="modal-image-url">Image URL <span class="required">*</span></label>
						<input
							class="form-input"
							type="url"
							id="modal-image-url"
							bind:value={modalImageUrl}
							placeholder="https://example.com/image.jpg"
						/>
						<span class="form-helper">Direct link to image (JPG, PNG)</span>
					</div>
				{:else if modalMediaType === 'CAROUSEL'}
					<div class="form-group">
						<label class="form-label">Carousel Image URLs <span class="required">*</span></label>
						<span class="form-helper">2–20 image URLs. Each becomes a carousel slide.</span>
						{#each modalCarouselUrls as _, idx}
							<div class="carousel-url-row">
								<input
									class="form-input"
									type="url"
									placeholder={`Image ${idx + 1} URL`}
									bind:value={modalCarouselUrls[idx]}
								/>
								{#if modalCarouselUrls.length > 2}
										<Button
											variant="ghost"
											size="sm"
											class="carousel-remove"
											onclick={() => removeCarouselUrl(idx)}
											title="Remove"
											type="button"
										>✕</Button>
									{/if}
							</div>
						{/each}
						{#if modalCarouselUrls.length < 20}
								<Button
									variant="outline"
									size="sm"
									onclick={addCarouselUrl}
									type="button"
									class="mt-2xs"
								>+ Add Image</Button>
							{/if}
					</div>
				{/if}

				<div class="form-group">
					<label class="form-label" for="modal-caption">
						Caption
						<span class="char-count" class:over={modalCaption.length > 500}>
							{modalCaption.length}/500
						</span>
					</label>
					<Textarea
						id="modal-caption"
						bind:value={modalCaption}
						placeholder="Write your post caption..."
						rows={4}
						maxlength={500}
						class="form-input"
					/>
				</div>
			</div>

			<div class="confirm-actions">
				<Button variant="outline" size="sm" onclick={closeCreateModal}>Cancel</Button>
				<Button
					variant="default"
					size="sm"
					onclick={handleCreate}
					disabled={creating}
				>
					{creating ? 'Creating…' : 'Create as Draft'}
				</Button>
			</div>
		</div>
	</div>
{/if}

<!-- Edit Schedule Modal (HITL) -->
{#if editTarget}
	<div class="confirm-overlay" onclick={closeEditModal} role="dialog" aria-modal="true" aria-label="Edit Schedule">
		<div class="confirm-dialog modal-narrow" onclick={(e) => e.stopPropagation()}>
			<h3>Edit Schedule</h3>
			<p class="modal-desc">
				Status: <StatusBadge status={editTarget.status} />
			</p>
			<div class="modal-stack">
				<div class="form-group">
					<label class="form-label" for="edit-scheduled">Scheduled At</label>
					<input
						class="form-input"
						type="datetime-local"
						id="edit-scheduled"
						bind:value={editScheduledAt}
					/>
					<span class="form-helper">Times shown in {tzLabel}</span>
				</div>

				<div class="form-group">
					<label class="form-label" for="edit-caption">
						Caption
						<span class="char-count" class:over={editCaption.length > 500}>
							{editCaption.length}/500
						</span>
					</label>
					<Textarea
						id="edit-caption"
						bind:value={editCaption}
						placeholder="Write your post caption..."
						rows={4}
						maxlength={500}
						class="form-input"
					/>
				</div>
			</div>

			<div class="confirm-actions">
				<Button variant="outline" size="sm" onclick={closeEditModal}>Cancel</Button>
				<Button
					variant="default"
					size="sm"
					onclick={handleEditSave}
					disabled={editing}
				>
					{editing ? 'Saving…' : 'Save Changes'}
				</Button>
			</div>
		</div>
	</div>
{/if}

<!-- Reject Modal -->
{#if rejectTarget}
	<div class="confirm-overlay" onclick={closeRejectModal} role="dialog" aria-modal="true" aria-label="Reject Schedule">
		<div class="confirm-dialog modal-narrow" style="max-width: 28rem;" onclick={(e) => e.stopPropagation()}>
			<h3>Reject Schedule</h3>
			<div class="mb-md">
				<div class="form-group">
							<label class="form-label">Reason (optional)</label>
							<Textarea
								bind:value={rejectReason}
								placeholder="Why is this schedule being rejected?"
								rows={3}
								class="form-input"
							/>
						</div>
			</div>
			<div class="confirm-actions">
				<Button variant="outline" size="sm" onclick={closeRejectModal}>Cancel</Button>
				<Button
					variant="destructive"
					size="sm"
					onclick={handleReject}
					disabled={rejecting}
				>
					{rejecting ? 'Rejecting…' : 'Reject'}
				</Button>
			</div>
		</div>
	</div>
{/if}

<!-- Delete Confirm -->
<ConfirmDialog
	open={deleteTarget !== null}
	title="Delete Schedule"
	message="Are you sure you want to delete this schedule? This action cannot be undone."
	confirmLabel="Delete"
	variant="danger"
	onconfirm={handleDelete}
	oncancel={cancelDelete}
/>

<!-- Schedule Detail Modal -->
{#if detailSchedule}
	<ScheduleDetail
		schedule={detailSchedule}
		onClose={closeDetail}
		onAction={onDetailAction}
	/>
{/if}

<svelte:window onkeydown={(e) => {
	if (e.key === 'Escape') {
		if (detailSchedule) closeDetail();
		if (modalOpen) closeCreateModal();
		if (editTarget) closeEditModal();
		if (rejectTarget) closeRejectModal();
		if (deleteTarget) cancelDelete();
	}
}} />

<style>
	.filter-bar {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-start;
		gap: var(--space-sm);
	}

	.filter-bar .form-group {
		flex: 0 1 auto;
		min-width: 9rem;
	}

	.filter-count {
		margin-left: auto;
		align-self: flex-end;
		padding-bottom: 0.45rem;
		font-size: var(--text-xs);
		color: var(--color-muted);
		white-space: nowrap;
	}

	.view-toggle {
		display: flex;
		gap: 0.25rem;
		align-self: flex-end;
		padding-bottom: 0.25rem;
	}

	.card-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
		gap: var(--space-md, 1rem);
		margin-top: var(--space-md, 1rem);
	}
	.sched-card {
		display: flex;
		flex-direction: column;
		gap: var(--space-sm, 0.5rem);
		padding: var(--space-md, 1rem);
		border: 1px solid var(--color-border, #e2e8f0);
		border-radius: var(--radius-md, 8px);
		background: var(--color-bg, #fff);
	}
	.sched-card.draft {
		border-left: 3px solid var(--color-warning, #f59e0b);
	}
	.sched-card-head {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: var(--space-sm, 0.5rem);
	}
	.sched-card-time {
		font-size: 0.75rem;
		color: var(--color-muted);
		font-variant-numeric: tabular-nums;
		white-space: nowrap;
	}
	.sched-card-media {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-xs, 0.375rem);
	}
	.sched-card-media img {
		height: 64px;
		width: 64px;
		border-radius: var(--radius-sm, 6px);
		object-fit: cover;
	}
	.sched-card-caption {
		font-size: 0.875rem;
		margin: 0;
		overflow-wrap: anywhere;
		display: -webkit-box;
		-webkit-line-clamp: 3;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
	.sched-card-foot {
		display: flex;
		flex-direction: column;
		gap: var(--space-sm, 0.5rem);
		margin-top: auto;
	}
	.sched-card-meta {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-xs, 0.375rem);
	}
	.chip {
		font-size: 0.75rem;
		padding: 0.125rem 0.5rem;
		border-radius: 999px;
		background: var(--color-bg-subtle, rgba(0, 0, 0, 0.04));
		color: var(--color-muted);
	}
	.sched-card-actions {
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem;
	}
	.sched-card-error {
		font-size: 0.75rem;
		color: var(--color-error);
		overflow-wrap: anywhere;
		margin: 0;
	}
	.skeleton-card {
		min-height: 140px;
	}
	.skel-line {
		height: 0.75rem;
		border-radius: 4px;
		background: var(--color-bg-subtle, rgba(0, 0, 0, 0.06));
		animation: pulse 1.5s ease-in-out infinite;
	}
	@keyframes pulse {
		50% { opacity: 0.5; }
	}

	.detail-panel {
		display: flex;
		flex-direction: column;
		gap: var(--space-sm, 0.5rem);
		padding: var(--space-sm, 0.5rem) var(--space-md, 1rem);
	}
	.detail-caption {
		font-size: 0.875rem;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}
	.detail-media {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-xs, 0.375rem);
	}
	.detail-thumb {
		height: 72px;
		width: 72px;
		border-radius: var(--radius-sm, 6px);
		object-fit: cover;
	}
	.detail-error {
		font-size: 0.8125rem;
		color: var(--color-error);
		overflow-wrap: anywhere;
	}
	.detail-meta {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-md, 1rem);
		margin: 0;
		font-size: 0.8125rem;
	}
	.detail-meta div {
		display: flex;
		gap: 0.375rem;
	}
	.detail-meta dt {
		color: var(--color-muted);
	}
	.detail-meta dd {
		margin: 0;
		font-variant-numeric: tabular-nums;
	}

	.filter-bar .form-group:first-child {
		flex: 1 1 16rem;
	}

	.tz-info {
		display: flex;
		align-items: flex-end;
	}

	@media (max-width: 640px) {
		.filter-bar {
			flex-direction: column;
			align-items: stretch;
		}

		.filter-bar .form-group {
			min-width: 0;
		}
	}

	.row-clickable {
		cursor: pointer;
		transition: background-color 0.1s ease;
	}

	.row-clickable:hover {
		background: var(--color-bg-hover, rgba(0, 0, 0, 0.03));
	}

	.badge-draft {
		background: var(--color-warning-bg);
		color: var(--color-warning-text);
		padding: 0.25rem 0.625rem;
		border-radius: 9999px;
		font-size: 0.75rem;
		font-weight: 600;
		white-space: nowrap;
	}

	.row-draft {
		background: var(--color-warning-bg-subtle, rgba(254, 243, 199, 0.3));
	}

	/* Modal utility classes */
	.modal-narrow {
		max-width: 36rem;
	}

	.modal-desc {
		color: var(--color-muted);
		font-size: 0.85rem;
		margin-bottom: var(--space-md);
	}

	.modal-stack {
		display: flex;
		flex-direction: column;
		gap: var(--space-md);
		margin-bottom: var(--space-md);
	}

	.col-actions {
		display: flex;
		gap: 0.25rem;
		flex-wrap: wrap;
	}

	/* Carousel thumbnail preview */
	.caption-cell {
		display: flex;
		align-items: center;
		gap: 0.375rem;
	}

	.row-thumb {
		width: 2rem;
		height: 2rem;
		border-radius: var(--radius-sm);
		object-fit: cover;
		border: 1px solid var(--color-rule);
		flex-shrink: 0;
	}

	.thumb-more {
		font-size: 0.625rem;
		font-weight: 600;
		color: var(--color-muted);
		background: var(--color-paper-2);
		padding: 0.125rem 0.25rem;
		border-radius: var(--radius-sm);
		flex-shrink: 0;
	}

	/* TZ badge */
	.tz-info {
		display: flex;
		align-items: flex-end;
	}
	.tz-badge {
		font-size: 0.75rem;
		color: var(--color-muted);
		background: var(--color-bg-elevated);
		padding: 0.25rem 0.625rem;
		border-radius: 0.375rem;
		white-space: nowrap;
	}

	/* Form helpers */
	.form-row {
		display: flex;
		gap: var(--space-md, 1rem);
	}
	.char-count {
		margin-left: auto;
		font-size: 0.75rem;
		color: var(--color-muted);
		font-variant-numeric: tabular-nums;
	}
	.char-count.over {
		color: var(--color-error);
		font-weight: 600;
	}
	.form-label:has(.char-count) {
		display: flex;
		align-items: baseline;
	}

	/* Carousel URL input rows */
	.carousel-url-row {
		display: flex;
		gap: 0.5rem;
		align-items: center;
		margin-bottom: 0.5rem;
	}
	.carousel-url-row .form-input {
		flex: 1;
	}

	@media (max-width: 640px) {
		.form-row {
			flex-direction: column;
		}
	}
</style>
