<script lang="ts">
	import PageHeader from '$lib/components/PageHeader.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import { listAccounts, createAccount, deleteAccount, refreshToken, getOAuthConfig } from '$lib/api';
	import { formatDate as formatDateTz } from '$lib/tz';
	import { toast } from '$lib/toast.svelte';
	import type { Account } from '$lib/types';

	let accounts = $state<Account[]>([]);
	let loading = $state(true);
	let loaded = $state(false);
	let showAddModal = $state(false);
	let deletingId = $state<string | null>(null);
	let refreshingId = $state<string | null>(null);
	let submitting = $state(false);

	let formUserId = $state('');
	let formUsername = $state('');
	let formAccessToken = $state('');
	let formExpiresAt = $state(new Date(Date.now() + 60 * 24 * 60 * 60 * 1000).toISOString().slice(0, 16));
	let formAppId = $state('');
	let formAppSecret = $state('');

	function formatDate(iso: string | null): string {
		if (!iso) return '\u2014';
		return formatDateTz(iso);
	}

	function statusFromAccount(account: Account): string {
		if (account.is_active) {
			return 'active';
		}
		if (account.token_status === 'expired') {
			return 'expired';
		}
		return 'suspended';
	}

	async function loadAccounts() {
		loading = true;
		try {
			accounts = await listAccounts();
		} catch (e: any) {
			toast(e.message || 'Failed to load accounts', 'error');
		} finally {
			loading = false;
			loaded = true;
		}
	}

	async function handleRefreshToken(id: string) {
		refreshingId = id;
		try {
			const res = await refreshToken(id);
			toast('Token refreshed', 'success');
			accounts = accounts.map((a) =>
				a.id === id ? { ...a, expires_at: res.expires_at } : a,
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
		formUserId = '';
		formUsername = '';
		formAccessToken = '';
		formExpiresAt = '';
		formAppId = '';
		formAppSecret = '';
	}

	async function handleAddAccount() {
		if (!formAccessToken) {
			toast('Access token is required', 'error');
			return;
		}
		submitting = true;
		try {
			const created = await createAccount({
				...(formUserId && { user_id: formUserId }),
				...(formUsername && { username: formUsername }),
				access_token: formAccessToken,
				expires_at: new Date(formExpiresAt).toISOString(),
				...(formAppId && { app_id: formAppId }),
				...(formAppSecret && { app_secret: formAppSecret }),
			});
			toast('Account added', 'success');
			accounts = [...accounts, created];
			showAddModal = false;
			resetForm();
		} catch (e: any) {
			toast(e.message || 'Failed to add account', 'error');
		} finally {
			submitting = false;
		}
	}

	async function handleConnectThreads() {
		try {
			// Try server-side OAuth config first
			const config = await getOAuthConfig();
			if (config.app_id) {
				if (!config.secret_configured) {
					toast('Set App Secret in Settings first', 'error');
					return;
				}
				// Server returns ready-to-use authorize URL
				if (config.authorize_url) {
					window.location.href = config.authorize_url;
					return;
				}
			}
			// Fallback: no server config
			toast('Set Threads App ID and Secret in Settings first', 'error');
		} catch {
			toast('Failed to get OAuth config. Set credentials in Settings.', 'error');
		}
	}

	async function handleConnectThreads() {
		try {
			const config = await getOAuthConfig();
			if (config.app_id) {
				if (!config.secret_configured) {
					toast('Set App Secret in Settings first', 'error');
					return;
				}
				if (config.authorize_url) {
					window.location.href = config.authorize_url;
					return;
				}
			}
			toast('Set Threads App ID and Secret in Settings first', 'error');
		} catch {
			toast('Failed to get OAuth config. Set credentials in Settings.', 'error');
		}
	}

	// Fetch once on mount — avoid infinite $effect re-runs
	$effect(() => {
		if (!loaded) loadAccounts();
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
								<div>
									<div style="font-weight:500;">{account.username}</div>
									{#if account.user_id}
										<div style="font-size:var(--text-xs);color:var(--color-muted);">ID: {account.user_id}</div>
									{/if}
								</div>
							</div>
						</td>
						<td><StatusBadge status={statusFromAccount(account)} /></td>
						<td>
							<span class="tabular-nums" style="font-size:var(--text-sm);">
								{#if account.expires_at}
									Expires {formatDate(account.expires_at)}
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

			<!-- OAuth connect -->
			<button
				type="button"
				class="oauth-connect-btn"
				onclick={() => { showAddModal = false; handleConnectThreads(); }}
			>
				<svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
					<path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 3c1.66 0 3 1.34 3 3s-1.34 3-3 3-3-1.34-3-3 1.34-3 3-3zm0 14.2c-2.5 0-4.71-1.28-6-3.22.03-1.99 4-3.08 6-3.08 1.99 0 5.97 1.09 6 3.08-1.29 1.94-3.5 3.22-6 3.22z"/>
				</svg>
				Connect with Threads
			</button>

			<div class="oauth-divider"><span>or add manually</span></div>

			<form onsubmit={(e) => { e.preventDefault(); handleAddAccount(); }}>
				<div class="form-group" style="margin-bottom:var(--space-sm);">
					<label class="form-label" for="user_id">User ID <span style="color:var(--color-muted);font-weight:400;">(optional)</span></label>
					<input id="user_id" class="form-input" type="text" bind:value={formUserId} placeholder="e.g. 1234567890" disabled={submitting} />
				</div>
				<div class="form-group" style="margin-bottom:var(--space-sm);">
					<label class="form-label" for="username">Username <span style="color:var(--color-muted);font-weight:400;">(optional)</span></label>
					<input id="username" class="form-input" type="text" bind:value={formUsername} placeholder="e.g. @username" disabled={submitting} />
				</div>
				<div class="form-group" style="margin-bottom:var(--space-sm);">
					<label class="form-label" for="access_token">Access Token</label>
					<input id="access_token" class="form-input" type="password" bind:value={formAccessToken} placeholder="Long-lived access token" disabled={submitting} />
				</div>
				<div class="form-group" style="margin-bottom:var(--space-sm);">
					<label class="form-label" for="expires_at">Expires At <span style="color:var(--color-muted);font-weight:400;">(optional)</span></label>
					<input id="expires_at" class="form-input" type="datetime-local" bind:value={formExpiresAt} disabled={submitting} />
				</div>
				<div class="form-group" style="margin-bottom:var(--space-sm);">
					<label class="form-label" for="app_id">App ID <span style="color:var(--color-muted);font-weight:400;">(optional)</span></label>
					<input id="app_id" class="form-input" type="text" bind:value={formAppId} placeholder="Facebook App ID" disabled={submitting} />
				</div>
				<div class="form-group" style="margin-bottom:var(--space-md);">
					<label class="form-label" for="app_secret">App Secret <span style="color:var(--color-muted);font-weight:400;">(optional)</span></label>
					<input id="app_secret" class="form-input" type="password" bind:value={formAppSecret} placeholder="Facebook App Secret" disabled={submitting} />
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

<style>
	.oauth-connect-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--space-sm);
		width: 100%;
		padding: var(--space-md) var(--space-lg);
		margin-bottom: var(--space-md);
		background: var(--color-accent);
		color: var(--color-accent-ink);
		font-family: var(--font-display);
		font-weight: 600;
		font-size: var(--text-sm);
		border: none;
		border-radius: var(--radius-md);
		cursor: pointer;
		transition: background-color var(--dur-short) var(--ease-out);
	}
	.oauth-connect-btn:hover {
		background: oklch(50% 0.22 260);
	}

	.oauth-divider {
		display: flex;
		align-items: center;
		gap: var(--space-sm);
		margin-bottom: var(--space-md);
		font-size: var(--text-xs);
		color: var(--color-muted);
	}
	.oauth-divider::before,
	.oauth-divider::after {
		content: '';
		flex: 1;
		border-top: var(--rule-subtle);
	}
</style>