<script lang="ts">
	import PageHeader from '$lib/components/PageHeader.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import { listAccounts, createAccount, deleteAccount, refreshToken } from '$lib/api';
	import { toast } from '$lib/toast.svelte';
	import type { Account } from '$lib/types';

	let accounts = $state<Account[]>([]);
	let loading = $state(true);
	let showAddModal = $state(false);
	let deletingId = $state<string | null>(null);
	let refreshingId = $state<string | null>(null);
	let submitting = $state(false);

	let formThreadsUserId = $state('');
	let formAccessToken = $state('');
	let formRefreshToken = $state('');

	function formatDate(iso: string | null): string {
		if (!iso) return '—';
		return new Date(iso).toLocaleDateString('en-US', {
			year: 'numeric',
			month: 'short',
			day: 'numeric',
		});
	}

	async function loadAccounts() {
		loading = true;
		try {
			const res = await listAccounts();
			accounts = res.data;
		} catch (e: any) {
			toast(e.message || 'Failed to load accounts', 'error');
		} finally {
			loading = false;
		}
	}

	async function handleRefreshToken(id: string) {
		refreshingId = id;
		try {
			const res = await refreshToken(id);
			toast('Token refreshed', 'success');
			accounts = accounts.map((a) =>
				a.id === id ? { ...a, token_expires_at: res.data.token_expires_at } : a,
			);
		} catch (e: any) {
			toast(e.message || 'Failed to refresh token', 'error');
		} finally {
			refreshingId = null;
		}
	}

	async function handleDelete() {
		if (!deletingId) return;
		try {
			await deleteAccount(deletingId);
			toast('Account deleted', 'success');
			accounts = accounts.filter((a) => a.id !== deletingId);
		} catch (e: any) {
			toast(e.message || 'Failed to delete account', 'error');
		} finally {
			deletingId = null;
		}
	}

	function resetForm() {
		formThreadsUserId = '';
		formAccessToken = '';
		formRefreshToken = '';
	}

	async function handleAddAccount() {
		if (!formThreadsUserId || !formAccessToken || !formRefreshToken) {
			toast('All fields are required', 'error');
			return;
		}
		submitting = true;
		try {
			const res = await createAccount({
				threads_user_id: formThreadsUserId,
				access_token: formAccessToken,
				refresh_token: formRefreshToken,
			});
			toast('Account added', 'success');
			accounts = [...accounts, res.data];
			showAddModal = false;
			resetForm();
		} catch (e: any) {
			toast(e.message || 'Failed to add account', 'error');
		} finally {
			submitting = false;
		}
	}

	$effect(() => {
		loadAccounts();
	});
</script>

<PageHeader title="Accounts">
	{#snippet action()}
		<button class="btn-primary" onclick={() => (showAddModal = true)}>Add Account</button>
	{/snippet}
</PageHeader>

<div class="data-table-wrap">
	{#if loading}
		<table class="data-table">
			<thead><tr><th>Username</th><th>Status</th><th>Token</th><th>Created</th><th>Actions</th></tr></thead>
			<tbody>
				{#each Array(3) as _}
						<tr>
							{#each Array(5) as _}
								<td><div class="skeleton" style="height: 1rem;"></div></td>
							{/each}
						</tr>
				{/each}
			</tbody>
		</table>
	{:else if accounts.length === 0}
		<div class="empty-state">
			<p class="empty-state-title">No accounts yet</p>
			<p class="empty-state-desc">Add a Threads account to get started.</p>
		</div>
	{:else}
		<table class="data-table">
			<thead>
				<tr>
					<th>Username</th>
					<th>Status</th>
					<th>Token</th>
					<th>Created</th>
					<th>Actions</th>
				</tr>
			</thead>
			<tbody>
				{#each accounts as account (account.id)}
						<tr>
							<td>
								<div style="display:flex;align-items:center;gap:var(--space-sm);">
									{#if account.profile_pic_url}
										<img
											src={account.profile_pic_url}
											alt=""
											style="width:2rem;height:2rem;border-radius:50%;object-fit:cover;flex-shrink:0;"
										/>
									{/if}
									<div>
										<div style="font-weight:500;">{account.display_name || account.username}</div>
										<div style="font-size:var(--text-xs);color:var(--color-muted);">@{account.username}</div>
									</div>
								</div>
							</td>
							<td><StatusBadge status={account.status} /></td>
							<td>
								<span class="tabular-nums" style="font-size:var(--text-sm);">
									{#if account.token_expires_at}
										Expires {formatDate(account.token_expires_at)}
									{:else}
										—
									{/if}
								</span>
							</td>
							<td><span class="tabular-nums">{formatDate(account.created_at)}</span></td>
							<td>
								<div style="display:flex;gap:var(--space-2xs);">
									<button
										class="btn-outline btn-sm"
											disabled={refreshingId === account.id}
											onclick={() => handleRefreshToken(account.id)}
										>
											{refreshingId === account.id ? '…' : 'Refresh'}
										</button>
									<button class="btn-ghost btn-sm" onclick={() => (deletingId = account.id)}>
											<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="width:1rem;height:1rem;color:var(--color-error);">
												<path d="M3 6h18"/><path d="M8 6V4h8v2"/><path d="M19 6l-1 14H6L5 6"/>
											</svg>
										</button>
									</div>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
	{/if}
</div>

<!-- Add Account Modal -->
{#if showAddModal}
	<div class="confirm-overlay" onclick={() => (showAddModal = false)} role="dialog" aria-modal="true" aria-label="Add Account">
		<div class="confirm-dialog" style="max-width:32rem;" onclick={(e) => e.stopPropagation()}>
			<h3>Add Account</h3>
			<p style="margin-bottom:var(--space-md);">Provide the Threads account credentials.</p>
			<form onsubmit={(e) => { e.preventDefault(); handleAddAccount(); }}>
				<div class="form-group" style="margin-bottom:var(--space-sm);">
					<label class="form-label" for="threads_user_id">Threads User ID</label>
					<input id="threads_user_id" class="form-input" type="text" bind:value={formThreadsUserId} placeholder="e.g. 1234567890" disabled={submitting} />
				</div>
				<div class="form-group" style="margin-bottom:var(--space-sm);">
					<label class="form-label" for="access_token">Access Token</label>
					<input id="access_token" class="form-input" type="password" bind:value={formAccessToken} placeholder="Long-lived access token" disabled={submitting} />
				</div>
				<div class="form-group" style="margin-bottom:var(--space-md);">
					<label class="form-label" for="refresh_token">Refresh Token</label>
					<input id="refresh_token" class="form-input" type="password" bind:value={formRefreshToken} placeholder="Refresh token" disabled={submitting} />
				</div>
				<div class="confirm-actions">
					<button type="button" class="btn-outline btn-sm" onclick={() => (showAddModal = false)}>Cancel</button>
					<button type="submit" class="btn-primary btn-sm" disabled={submitting}>{submitting ? 'Adding…' : 'Add Account'}</button>
				</div>
		</form>
		</div>
	</div>
{/if}

<!-- Delete Confirmation -->
<ConfirmDialog
	open={deletingId !== null}
	title="Delete Account"
	message="This will permanently remove this account and all its associated data. This action cannot be undone."
	confirmLabel="Delete"
	onconfirm={handleDelete}
	oncancel={() => (deletingId = null)}
/>