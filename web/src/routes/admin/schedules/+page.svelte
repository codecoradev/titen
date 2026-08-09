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
			const params: { account_id?: string; status?: string } = {};
			if (filterAccountId) params.account_id = filterAccountId;
			if (filterStatus !== 'all') params.status = filterStatus;

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
			<button class="btn-primary" onclick={openCreateModal}>New Schedule</button>
		{/snippet}
	</PageHeader>

	<!-- Filters -->
	<div class="filter-bar">
		<div class="form-group">
			<label class="form-label" for="filter-account">Account</label>
			<select class="form-input" id="filter-account" bind:value={filterAccountId}>
				<option value="">All accounts</option>
				{#each accounts as acct}
					<option value={acct.id}>{acct.username}</option>
				{/each}
			</select>
		</div>

		<div class="form-group">
			<label class="form-label" for="filter-status">Status</label>
			<select class="form-input" id="filter-status" bind:value={filterStatus}>
				<option value="all">All</option>
				<option value="draft">Draft (Needs Review)</option>
				<option value="pending">Pending (Approved)</option>
				<option value="processing">Processing</option>
				<option value="published">Published</option>
				<option value="failed">Failed</option>
				<option value="rejected">Rejected</option>
			</select>
		</div>

		<div class="form-group tz-info">
			<span class="tz-badge" title="Display timezone from TZ env var">🕒 {tzLabel}</span>
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
				<button class="btn-primary btn-sm" onclick={openCreateModal}>New Schedule</button>
			{/snippet}
		</EmptyState>
	{:else}
		<div class="data-table-wrap">
			<table class="data-table">
				<thead>
					<tr>
						<th>Content</th>
						<th>Account</th>
						<th>Scheduled</th>
						<th>Status</th>
						<th>Error</th>
						<th>Actions</th>
					</tr>
				</thead>
				<tbody>
					{#if loading}
						{#each Array(5) as _}
							<tr>
								<td><div class="skeleton" style="height: 1rem; width: 12rem;"></div></td>
								<td><div class="skeleton" style="height: 1rem; width: 6rem;"></div></td>
								<td><div class="skeleton" style="height: 1rem; width: 8rem;"></div></td>
								<td><div class="skeleton" style="height: 1rem; width: 5rem;"></div></td>
								<td><div class="skeleton" style="height: 1rem; width: 6rem;"></div></td>
								<td><div class="skeleton" style="height: 1rem; width: 4rem;"></div></td>
							</tr>
						{/each}
					{:else}
						{#each schedules as schedule (schedule.id)}
							<tr
								class={schedule.status === 'draft' ? 'row-draft row-clickable' : 'row-clickable'}
								onclick={() => openDetail(schedule)}
								onkeydown={(e) => e.key === 'Enter' && openDetail(schedule)}
								role="button"
								tabindex="0"
							>
							<td class="truncate" title={schedule.caption || '—'}>
								{#if schedule.media_urls}
									<span class="media-tag">{schedule.media_type || 'MEDIA'}</span>
								{/if}
								{truncate(schedule.caption || '—', 60)}
							</td>
								<td>
									{accounts.find(a => a.id === schedule.account_id)?.username ?? schedule.account_id.slice(0, 8)}
								</td>
								<td class="tabular-nums">
									{fmtDate(schedule.scheduled_at)}
								</td>
								<td>
									<StatusBadge status={schedule.status} />
								</td>
								<td class="col-error" title={schedule.error ?? ''}>
									{schedule.error ?? '—'}
								</td>
								<td class="col-actions" onclick={(e) => e.stopPropagation()}>
									{#if schedule.status === 'draft'}
										<button
											class="btn-success btn-sm"
											onclick={() => handleApprove(schedule)}
											disabled={approvingId === schedule.id}
											title="Approve — will publish when due"
										>
											{approvingId === schedule.id ? '…' : 'Approve'}
										</button>
										<button
											class="btn-ghost btn-sm"
											onclick={() => openEditModal(schedule)}
											title="Edit"
										>
											Edit
										</button>
										<button
											class="btn-danger btn-sm"
											onclick={() => openRejectModal(schedule)}
											title="Reject"
										>
											Reject
										</button>
									{:else if schedule.status === 'pending'}
										<button
											class="btn-ghost btn-sm"
											onclick={() => openEditModal(schedule)}
											title="Edit"
										>
											Edit
										</button>
										<button
											class="btn-ghost btn-sm"
											onclick={() => confirmDelete(schedule)}
											disabled={deleting}
											title="Cancel schedule"
										>
											Cancel
										</button>
									{:else}
										<button
											class="btn-ghost btn-sm"
											onclick={() => confirmDelete(schedule)}
											disabled={deleting}
											title="Delete"
										>
											Delete
										</button>
									{/if}
								</td>
							</tr>
						{/each}
					{/if}
				</tbody>
			</table>
		</div>
	{/if}
</div>

<!-- Create Schedule Modal -->
{#if modalOpen}
	<div class="confirm-overlay" onclick={closeCreateModal} role="dialog" aria-modal="true" aria-label="New Schedule">
		<div class="confirm-dialog" style="max-width: 36rem;" onclick={(e) => e.stopPropagation()}>
			<h3>New Schedule</h3>
			<p style="color: var(--color-muted); font-size: 0.85rem; margin-bottom: var(--space-md);">
				New schedules are created as <strong>draft</strong>. You'll need to approve them before they can be published.
			</p>
			<div style="display: flex; flex-direction: column; gap: var(--space-md); margin-bottom: var(--space-md);">
				<div class="form-group">
					<label class="form-label" for="modal-account">Account <span class="required">*</span></label>
					<select class="form-input" id="modal-account" bind:value={modalAccountId}>
						{#each accounts as acct}
							<option value={acct.id}>{acct.username}</option>
						{/each}
					</select>
				</div>

				<div class="form-row">
					<div class="form-group" style="flex: 1;">
						<label class="form-label" for="modal-scheduled">Scheduled At <span class="required">*</span></label>
						<input
							class="form-input"
							type="datetime-local"
							id="modal-scheduled"
							bind:value={modalScheduledAt}
						/>
						<span class="form-hint">Times shown in {tzLabel}</span>
					</div>

					<div class="form-group" style="flex: 1;">
						<label class="form-label" for="modal-media-type">Media Type</label>
						<select class="form-input" id="modal-media-type" bind:value={modalMediaType}>
							<option value="text">Text Only</option>
							<option value="IMAGE">Single Image</option>
							<option value="CAROUSEL">Carousel (2-20)</option>
						</select>
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
						<span class="form-hint">Direct link to image (JPG, PNG)</span>
					</div>
				{:else if modalMediaType === 'CAROUSEL'}
					<div class="form-group">
						<label class="form-label">Carousel Image URLs <span class="required">*</span></label>
						<span class="form-hint">2–20 image URLs. Each becomes a carousel slide.</span>
						{#each modalCarouselUrls as _, idx}
							<div class="carousel-url-row">
								<input
									class="form-input"
									type="url"
									placeholder={`Image ${idx + 1} URL`}
									bind:value={modalCarouselUrls[idx]}
								/>
								{#if modalCarouselUrls.length > 2}
									<button
										class="btn-ghost btn-sm carousel-remove"
										onclick={() => removeCarouselUrl(idx)}
										title="Remove"
										type="button"
									>✕</button>
								{/if}
							</div>
						{/each}
						{#if modalCarouselUrls.length < 20}
							<button
								class="btn-outline btn-sm"
								onclick={addCarouselUrl}
								type="button"
								style="margin-top: 0.5rem;"
							>+ Add Image</button>
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
					<textarea
						class="form-input"
						id="modal-caption"
						bind:value={modalCaption}
						placeholder="Write your post caption..."
						rows="4"
						maxlength="500"
					></textarea>
				</div>
			</div>

			<div class="confirm-actions">
				<button class="btn-outline btn-sm" onclick={closeCreateModal}>Cancel</button>
				<button
					class="btn-primary btn-sm"
					onclick={handleCreate}
					disabled={creating}
				>
					{creating ? 'Creating…' : 'Create as Draft'}
				</button>
			</div>
		</div>
	</div>
{/if}

<!-- Edit Schedule Modal (HITL) -->
{#if editTarget}
	<div class="confirm-overlay" onclick={closeEditModal} role="dialog" aria-modal="true" aria-label="Edit Schedule">
		<div class="confirm-dialog" style="max-width: 36rem;" onclick={(e) => e.stopPropagation()}>
			<h3>Edit Schedule</h3>
			<p style="color: var(--color-muted); font-size: 0.85rem; margin-bottom: var(--space-md);">
				Status: <StatusBadge status={editTarget.status} />
			</p>
			<div style="display: flex; flex-direction: column; gap: var(--space-md); margin-bottom: var(--space-md);">
				<div class="form-group">
					<label class="form-label" for="edit-scheduled">Scheduled At</label>
					<input
						class="form-input"
						type="datetime-local"
						id="edit-scheduled"
						bind:value={editScheduledAt}
					/>
					<span class="form-hint">Times shown in {tzLabel}</span>
				</div>

				<div class="form-group">
					<label class="form-label" for="edit-caption">
						Caption
						<span class="char-count" class:over={editCaption.length > 500}>
							{editCaption.length}/500
						</span>
					</label>
					<textarea
						class="form-input"
						id="edit-caption"
						bind:value={editCaption}
						placeholder="Write your post caption..."
						rows="4"
						maxlength="500"
					></textarea>
				</div>
			</div>

			<div class="confirm-actions">
				<button class="btn-outline btn-sm" onclick={closeEditModal}>Cancel</button>
				<button
					class="btn-primary btn-sm"
					onclick={handleEditSave}
					disabled={editing}
				>
					{editing ? 'Saving…' : 'Save Changes'}
				</button>
			</div>
		</div>
	</div>
{/if}

<!-- Reject Modal -->
{#if rejectTarget}
	<div class="confirm-overlay" onclick={closeRejectModal} role="dialog" aria-modal="true" aria-label="Reject Schedule">
		<div class="confirm-dialog" style="max-width: 28rem;" onclick={(e) => e.stopPropagation()}>
			<h3>Reject Schedule</h3>
			<div style="margin-bottom: var(--space-md);">
				<div class="form-group">
					<label class="form-label" for="reject-reason">Reason (optional)</label>
					<textarea
						class="form-input"
						id="reject-reason"
						bind:value={rejectReason}
						placeholder="Why is this schedule being rejected?"
						rows="3"
					></textarea>
				</div>
			</div>
			<div class="confirm-actions">
				<button class="btn-outline btn-sm" onclick={closeRejectModal}>Cancel</button>
				<button
					class="btn-danger btn-sm"
					onclick={handleReject}
					disabled={rejecting}
				>
					{rejecting ? 'Rejecting…' : 'Reject'}
				</button>
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

	.col-actions {
		display: flex;
		gap: 0.25rem;
		flex-wrap: wrap;
	}

	.btn-success {
		background: var(--color-success, #059669);
		color: white;
		border: none;
		padding: 0.375rem 0.75rem;
		border-radius: 0.375rem;
		font-size: 0.8125rem;
		font-weight: 500;
		cursor: pointer;
		transition: opacity 0.15s;
	}

	.btn-success:hover {
		opacity: 0.9;
	}

	.btn-success:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.btn-danger {
		background: transparent;
		color: var(--color-danger, #dc2626);
		border: 1px solid var(--color-danger-border, #fecaca);
		padding: 0.375rem 0.75rem;
		border-radius: 0.375rem;
		font-size: 0.8125rem;
		font-weight: 500;
		cursor: pointer;
		transition: background 0.15s;
	}

	.btn-danger:hover {
		background: var(--color-danger-bg, #fef2f2);
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

	/* Media type tag in table */
	.media-tag {
		display: inline-block;
		font-size: 0.625rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		background: var(--color-primary-bg, #dbeafe);
		color: var(--color-primary, #2563eb);
		padding: 0.125rem 0.375rem;
		border-radius: 0.25rem;
		margin-right: 0.375rem;
		vertical-align: middle;
	}

	/* Form helpers */
	.form-row {
		display: flex;
		gap: var(--space-md, 1rem);
	}
	.form-hint {
		display: block;
		font-size: 0.75rem;
		color: var(--color-muted);
		margin-top: 0.25rem;
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
	.carousel-remove {
		flex-shrink: 0;
		padding: 0.375rem 0.5rem;
		color: var(--color-danger, #dc2626);
	}

	@media (max-width: 640px) {
		.form-row {
			flex-direction: column;
		}
	}
</style>
