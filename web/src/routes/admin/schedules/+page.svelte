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
	import * as Table from '$lib/components/ui/table';
	import Skeleton from '$lib/components/ui/skeleton/skeleton.svelte';
	import Textarea from '$lib/components/ui/textarea/textarea.svelte';

	type StatusFilter = 'all' | 'draft' | 'pending' | 'processing' | 'published' | 'failed' | 'rejected';

	let schedules = $state<Schedule[]>([]);
	let accounts = $state<Account[]>([]);
	let loading = $state(true);
	let loaded = $state(false);
	let creating = $state(false);

	// Detail modal
	let detailSchedule = $state<Schedule | null>(null);

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
			// datetime-local value → local ISO; backend treats as start/end of range
			if (filterFrom) params.from = new Date(filterFrom).toISOString();
			if (filterTo) params.to = new Date(`${filterTo}T23:59:59`).toISOString();
			if (filterSearch.trim()) params.search = filterSearch.trim();
			if (filterMediaType !== 'all') params.media_type = filterMediaType;

			const [schedulesData, accountsData] = await Promise.all([
				listSchedules(params),
				listAccounts()
			]);

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

	// HITL: Approve
	async function handleApprove(schedule: Schedule) {
		approvingId = schedule.id;
		try {
			await approveSchedule(schedule.id);
			toast('Schedule approved — will auto-publish when due', 'success');
			await loadData();
		} catch (e: any) {
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
		try {
			await rejectSchedule(rejectTarget.id, rejectReason || undefined);
			toast('Schedule rejected', 'success');
			rejectTarget = null;
			await loadData();
		} catch (e: any) {
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

		<div class="form-group tz-info">
			<span class="tz-badge" title="Display timezone from TZ env var">🕒 {tzLabel}</span>
		</div>

		{#if hasActiveFilters}
			<div class="form-group">
				<label class="form-label">&nbsp;</label>
				<Button variant="outline" size="sm" onclick={resetFilters}>Reset filters</Button>
			</div>
		{/if}
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
	{:else}
		<div class="data-table-wrap">
			<Table.Root>
				<Table.Header>
					<Table.Row>
						<Table.Head>Content</Table.Head>
						<Table.Head>Account</Table.Head>
						<Table.Head>Scheduled</Table.Head>
						<Table.Head>Status</Table.Head>
						<Table.Head>Error</Table.Head>
						<Table.Head>Actions</Table.Head>
					</Table.Row>
				</Table.Header>
				<Table.Body>
					{#if loading}
						{#each Array(5) as _}
							<Table.Row>
								<Table.Cell><Skeleton class="h-4 w-48" /></Table.Cell>
								<Table.Cell><Skeleton class="h-4 w-24" /></Table.Cell>
								<Table.Cell><Skeleton class="h-4 w-32" /></Table.Cell>
								<Table.Cell><Skeleton class="h-4 w-20" /></Table.Cell>
								<Table.Cell><Skeleton class="h-4 w-24" /></Table.Cell>
								<Table.Cell><Skeleton class="h-4 w-16" /></Table.Cell>
							</Table.Row>
						{/each}
					{:else}
						{#each schedules as schedule (schedule.id)}
							<Table.Row
								class={schedule.status === 'draft' ? 'row-draft row-clickable' : 'row-clickable'}
								onclick={() => openDetail(schedule)}
								onkeydown={(e) => e.key === 'Enter' && openDetail(schedule)}
								role="button"
								tabindex={0}
							>
								<Table.Cell class="truncate" title={schedule.caption || '—'}>
									<div class="caption-cell">
										{#if schedule.media_urls}
											{#each schedule.media_urls.split(',').filter(Boolean).slice(0, 3) as url, i}
												<img
													src={url}
													alt="Preview"
													class="row-thumb"
													loading="lazy"
													onerror={(e) => { const t = e.currentTarget as HTMLImageElement; t.style.display = 'none'; }}
												/>
												{#if i === 2 && schedule.media_urls.split(',').filter(Boolean).length > 3}
													<span class="thumb-more">+{schedule.media_urls.split(',').filter(Boolean).length - 3}</span>
												{/if}
											{/each}
										{/if}
										<span>{truncate(schedule.caption || '—', 60)}</span>
									</div>
								</Table.Cell>
								<Table.Cell>
									{accounts.find(a => a.id === schedule.account_id)?.username ?? schedule.account_id.slice(0, 8)}
								</Table.Cell>
								<Table.Cell class="tabular-nums">
									{fmtDate(schedule.scheduled_at)}
								</Table.Cell>
								<Table.Cell>
									<StatusBadge status={schedule.status} />
								</Table.Cell>
								<Table.Cell class="col-error" title={schedule.error ?? ''}>
									{schedule.error ?? '—'}
								</Table.Cell>
								<Table.Cell class="col-actions" onclick={(e) => e.stopPropagation()}>
									{#if schedule.status === 'draft'}
										<Button
											variant="default"
											size="sm"
											class="bg-[var(--color-success)]"
											onclick={() => handleApprove(schedule)}
											disabled={approvingId === schedule.id}
											title="Approve — will publish when due"
										>
											{approvingId === schedule.id ? '…' : 'Approve'}
										</Button>
										<Button
											variant="ghost"
											size="sm"
											onclick={() => openEditModal(schedule)}
											title="Edit"
										>
											Edit
										</Button>
										<Button
											variant="destructive"
											size="sm"
											onclick={() => openRejectModal(schedule)}
											title="Reject"
										>
											Reject
										</Button>
									{:else if schedule.status === 'pending'}
										<Button
											variant="ghost"
											size="sm"
											onclick={() => openEditModal(schedule)}
											title="Edit"
										>
											Edit
										</Button>
										<Button
											variant="ghost"
											size="sm"
											onclick={() => confirmDelete(schedule)}
											disabled={deleting}
											title="Cancel schedule"
										>
											Cancel
										</Button>
									{:else}
										<Button
											variant="ghost"
											size="sm"
											onclick={() => confirmDelete(schedule)}
											disabled={deleting}
											title="Delete"
										>
											Delete
										</Button>
					{/if}
								</Table.Cell>
							</Table.Row>
						{/each}
					{/if}
				</Table.Body>
			</Table.Root>
		</div>
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
	.row-clickable {
		cursor: pointer;
		transition: background-color 0.1s ease;
	}

	.row-clickable:hover {
		background: var(--color-bg-hover, rgba(0, 0, 0, 0.03));
	}

	.badge-draft {
		background: var(--color-warning-bg, #fef3c7);
		color: var(--color-warning-text, #92400e);
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
		background: var(--color-bg-elevated, #f3f4f6);
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
