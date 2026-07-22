<script lang="ts">
	import PageHeader from '$lib/components/PageHeader.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import {
		listSchedules,
		createSchedule,
		updateSchedule,
		deleteSchedule
	} from '$lib/api';
	import { listAccounts } from '$lib/api';
	import type { Schedule, Account } from '$lib/types';
	import { toast } from '$lib/toast.svelte';

	type StatusFilter = 'all' | 'pending' | 'processing' | 'completed' | 'failed' | 'cancelled';

	let schedules = $state<Schedule[]>([]);
	let accounts = $state<Account[]>([]);
	let loading = $state(true);
	let creating = $state(false);

	let filterAccountId = $state('');
	let filterStatus = $state<StatusFilter>('all');

	let modalOpen = $state(false);
	let modalAccountId = $state('');
	let modalScheduledAt = $state('');
	let modalCaption = $state('');

	let deleteTarget = $state<Schedule | null>(null);
	let cancelling = $state(false);
	let deleting = $state(false);

	async function loadData() {
		loading = true;
		try {
			const params: { account_id?: string; status?: string } = {};
			if (filterAccountId) params.account_id = filterAccountId;
			if (filterStatus !== 'all') params.status = filterStatus;

			const [schedulesRes, accountsRes] = await Promise.all([
				listSchedules(params),
				listAccounts()
			]);

			schedules = schedulesRes.data;
			accounts = accountsRes.data;
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
		modalOpen = true;
	}

	function closeCreateModal() {
		modalOpen = false;
	}

	async function handleCreate() {
		if (!modalAccountId || !modalScheduledAt) {
			toast('Account and scheduled time are required', 'error');
			return;
		}
		creating = true;
		try {
			await createSchedule({
				account_id: modalAccountId,
				scheduled_at: new Date(modalScheduledAt).toISOString(),
				post_data: { caption: modalCaption }
			});
			toast('Schedule created', 'success');
			closeCreateModal();
			await loadData();
		} catch (e: any) {
			toast(e.message || 'Failed to create schedule', 'error');
		} finally {
			creating = false;
		}
	}

	async function handleCancel(schedule: Schedule) {
		cancelling = true;
		try {
			await updateSchedule(schedule.id, { status: 'cancelled' });
			toast('Schedule cancelled', 'success');
			await loadData();
		} catch (e: any) {
			toast(e.message || 'Failed to cancel schedule', 'error');
		} finally {
			cancelling = false;
		}
	}

	async function handleComplete(schedule: Schedule) {
		cancelling = true;
		try {
			await updateSchedule(schedule.id, { status: 'completed' });
			toast('Schedule marked as completed', 'success');
			await loadData();
		} catch (e: any) {
			toast(e.message || 'Failed to complete schedule', 'error');
		} finally {
			cancelling = false;
		}
	}

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

	function formatDateTime(iso: string): string {
		const d = new Date(iso);
		return d.toLocaleDateString('en-US', {
			month: 'short',
			day: 'numeric',
			year: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	function truncate(s: string, max: number = 60): string {
		if (s.length <= max) return s;
		return s.slice(0, max).trimEnd() + '…';
	}

	$effect(() => {
		loadData();
	});
</script>

<div class="schedules-page">
	<PageHeader title="Schedules">
		{#snippet action()}
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
				<option value="pending">Pending</option>
				<option value="processing">Processing</option>
				<option value="completed">Completed</option>
				<option value="failed">Failed</option>
				<option value="cancelled">Cancelled</option>
			</select>
		</div>
	</div>

	<!-- Table -->
	{#if schedules.length === 0 && !loading}
		<EmptyState
			icon="posts"
			title="No schedules yet"
			description="Create your first scheduled post to get started."
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
							<tr>
								<td class="truncate" title={schedule.caption || JSON.stringify(schedule.post_data)}>
									{truncate(schedule.caption || JSON.stringify(schedule.post_data).slice(0, 60), 60)}
								</td>
								<td>
									{schedule.account?.username ?? '—'}
								</td>
								<td class="tabular-nums">
									{formatDateTime(schedule.scheduled_at)}
								</td>
								<td>
									<StatusBadge status={schedule.status} />
								</td>
								<td class="col-error" title={schedule.error_message ?? ''}>
									{schedule.error_message ?? '—'}
								</td>
								<td>
									<div class="action-group">
										{#if schedule.status === 'pending' || schedule.status === 'processing'}
											<button
												class="btn-ghost btn-sm"
												onclick={() => handleCancel(schedule)}
												disabled={cancelling}
												title="Cancel"
											>
												Cancel
											</button>
										{/if}
										{#if schedule.status === 'pending' || schedule.status === 'processing'}
											<button
												class="btn-ghost btn-sm"
												onclick={() => handleComplete(schedule)}
												disabled={cancelling}
												title="Mark completed"
											>
												Complete
											</button>
										{/if}
										<button
											class="btn-ghost btn-sm"
											onclick={() => confirmDelete(schedule)}
											disabled={deleting}
											title="Delete"
										>
											Delete
										</button>
									</div>
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
		<div class="confirm-dialog" style="max-width: 32rem;" onclick={(e) => e.stopPropagation()}>
			<h3>New Schedule</h3>
			<div style="display: flex; flex-direction: column; gap: var(--space-md); margin-bottom: var(--space-md);">
				<div class="form-group">
					<label class="form-label" for="modal-account">Account <span class="required">*</span></label>
					<select class="form-input" id="modal-account" bind:value={modalAccountId}>
						{#each accounts as acct}
							<option value={acct.id}>{acct.username}</option>
						{/each}
					</select>
				</div>

				<div class="form-group">
					<label class="form-label" for="modal-scheduled">Scheduled At <span class="required">*</span></label>
					<input
						class="form-input"
						type="datetime-local"
						id="modal-scheduled"
						bind:value={modalScheduledAt}
					/>
				</div>

				<div class="form-group">
					<label class="form-label" for="modal-caption">Caption</label>
					<textarea
						class="form-input"
						id="modal-caption"
						bind:value={modalCaption}
						placeholder="Write your post caption..."
						rows="4"
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
					{creating ? 'Creating…' : 'Create Schedule'}
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

<svelte:window onkeydown={(e) => {
	if (e.key === 'Escape') {
		if (modalOpen) closeCreateModal();
		if (deleteTarget) cancelDelete();
	}
}} />
