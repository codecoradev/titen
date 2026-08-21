<script lang="ts">
	import PageHeader from '$lib/components/PageHeader.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import * as Dialog from '$lib/components/ui/dialog';
	import { listAccounts, createAccount, deleteAccount, refreshToken, getOAuthConfig, getThreadsProfile } from '$lib/api';
	import { Button } from '$lib/components/ui/button';
	import * as Table from '$lib/components/ui/table';
	import Skeleton from '$lib/components/ui/skeleton/skeleton.svelte';
	import { formatDate as formatDateTz } from '$lib/tz';
	import { toast } from '$lib/toast.svelte';
	import type { Account, ThreadsProfile } from '$lib/types';

	let accounts = $state<Account[]>([]);
	let loading = $state(true);
	let loaded = $state(false);
	let showAddModal = $state(false);
	let deletingId = $state<string | null>(null);
	let refreshingId = $state<string | null>(null);
	let submitting = $state(false);
	let profiles = $state<Record<string, ThreadsProfile | null>>({});
	let profilesLoading = $state<Set<string>>(new Set());

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
			// Load profiles concurrently in small batches to avoid overwhelming
			// the backend or hitting Threads API rate limits (5 at a time).
			const ids = accounts.map((a) => a.id);
			profilesLoading = new Set(ids);
			const BATCH = 5;
			for (let i = 0; i < ids.length; i += BATCH) {
				const batch = ids.slice(i, i + BATCH);
				const results = await Promise.allSettled(
					batch.map((id) => getThreadsProfile(id)),
				);
				batch.forEach((id, j) => {
					const r = results[j];
					profiles = {
						...profiles,
						[id]: r.status === 'fulfilled' ? r.value : null,
					};
					const next = new Set(profilesLoading);
					next.delete(id);
					profilesLoading = next;
				});
			}
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
				// Fallback: construct authorize URL client-side using known redirect URI.
				// This handles cases where the backend cannot derive redirect_uri
				// (e.g. Host header is internal Docker hostname).
				const redirectUri = `${window.location.origin}/auth/callback`;
				const authorizeUrl = `https://threads.net/oauth/authorize?client_id=${encodeURIComponent(config.app_id)}&redirect_uri=${encodeURIComponent(redirectUri)}&scope=threads_basic,threads_content_publish,threads_manage_replies,threads_manage_mentions,threads_location_tagging&response_type=code`;
				window.location.href = authorizeUrl;
				return;
			}
			// Fallback: no server config
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
		<Button variant="default" onclick={() => (showAddModal = true)}>Add Account</Button>
	{/snippet}
</PageHeader>

<div class="data-table-wrap">
	{#if loading}
		<Table.Root>
			<Table.Header><Table.Row><Table.Head>Username</Table.Head><Table.Head>Status</Table.Head><Table.Head class="hidden md:table-cell">Token</Table.Head><Table.Head class="hidden md:table-cell">Created</Table.Head><Table.Head>Actions</Table.Head></Table.Row></Table.Header>
			<Table.Body>
				{#each Array(3) as _}
					<Table.Row>
						<Table.Cell><Skeleton class="h-4 w-full" /></Table.Cell>
						<Table.Cell><Skeleton class="h-4 w-full" /></Table.Cell>
						<Table.Cell><Skeleton class="h-4 w-full" /></Table.Cell>
						<Table.Cell class="hidden md:table-cell"><Skeleton class="h-4 w-full" /></Table.Cell>
						<Table.Cell class="hidden md:table-cell"><Skeleton class="h-4 w-full" /></Table.Cell>
						<Table.Cell><Skeleton class="h-4 w-full" /></Table.Cell>
					</Table.Row>
				{/each}
			</Table.Body>
		</Table.Root>
	{:else if accounts.length === 0}
		<div class="empty-state">
			<p class="empty-state-title">No accounts yet</p>
			<p class="empty-state-desc">Add a Threads account to get started.</p>
		</div>
	{:else}
		<Table.Root>
			<Table.Header>
				<Table.Row>
					<Table.Head>Account</Table.Head>
					<Table.Head>Followers</Table.Head>
					<Table.Head>Status</Table.Head>
					<Table.Head class="hidden md:table-cell">Token</Table.Head>
					<Table.Head class="hidden md:table-cell">Created</Table.Head>
					<Table.Head>Actions</Table.Head>
				</Table.Row>
			</Table.Header>
			<Table.Body>
				{#each accounts as account (account.id)}
					<Table.Row>
						<Table.Cell>
							<div class="account-cell">
								{#if profiles[account.id]?.threads_profile_picture_url}
									<img
										src={profiles[account.id]!.threads_profile_picture_url!}
										alt={account.username}
										class="account-avatar"
										width="40"
										height="40"
									/>
								{:else if profilesLoading.has(account.id)}
									<Skeleton class="account-avatar" />
								{:else}
									<div class="account-avatar account-avatar-placeholder">
										{account.username.charAt(0).toUpperCase()}
									</div>
								{/if}
								<div>
									<div style="font-weight:500;">
										{profiles[account.id]?.name ?? account.username}
										{#if profiles[account.id]?.username && profiles[account.id]?.username !== account.username}
											<span style="font-size:var(--text-xs);color:var(--color-muted);">@{profiles[account.id]!.username}</span>
										{/if}
									</div>
									{#if profiles[account.id]?.threads_biography}
										<div class="account-bio">{profiles[account.id]!.threads_biography}</div>
									{:else if account.user_id}
										<div class="text-xs-muted">ID: {account.user_id.length > 30 ? account.user_id.slice(0, 12) + '\u2026' : account.user_id}</div>
									{/if}
								</div>
							</div>
						</Table.Cell>
						<Table.Cell>
							{#if profilesLoading.has(account.id)}
								<Skeleton class="h-5 w-12" />
							{:else if profiles[account.id]?.followers_count != null}
								<span class="tabular-nums font-medium">{profiles[account.id]!.followers_count!.toLocaleString()}</span>
							{:else}
								<span class="text-xs-muted">\u2014</span>
							{/if}
						</Table.Cell>
						<Table.Cell><StatusBadge status={statusFromAccount(account)} /></Table.Cell>
						<Table.Cell class="hidden md:table-cell">
						<span class="tabular-nums text-sm">
							{#if account.expires_at}
								Expires {formatDate(account.expires_at)}
							{:else}
								—
							{/if}
						</span>
					</Table.Cell>
						<Table.Cell class="hidden md:table-cell"><span class="tabular-nums">{formatDate(account.created_at)}</span></Table.Cell>
						<Table.Cell>
							<div class="row-gap-xs">
								<Button
									variant="outline"
									size="sm"
									disabled={refreshingId === account.id}
									onclick={() => handleRefreshToken(account.id)}
								>
									{refreshingId === account.id ? '…' : 'Refresh'}
								</Button>
								<Button variant="ghost" size="sm" onclick={() => (deletingId = account.id)}>
									<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="icon-sm-danger">
														<path d="M3 6h18"/><path d="M8 6V4h8v2"/><path d="M19 6l-1 14H6L5 6"/>
									</svg>
								</Button>
							</div>
						</Table.Cell>
						</Table.Row>
						{/each}
						</Table.Body>
						</Table.Root>
						{/if}
						</div>

						<!-- Add Account Modal -->
{#if showAddModal}
	<Dialog.Root open onOpenChange={(o) => { if (!o) showAddModal = false; }}>
		<Dialog.Content class="confirm-dialog" style="max-width:32rem;" aria-describedby={undefined}>
			<Dialog.Title class="text-base font-semibold mb-xs">Add Account</Dialog.Title>

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
				<div class="form-group mb-sm">
					<label class="form-label" for="user_id">User ID <span class="text-hint">(optional)</span></label>
					<input id="user_id" class="form-input" type="text" bind:value={formUserId} placeholder="e.g. 1234567890" disabled={submitting} />
				</div>
				<div class="form-group mb-sm">
					<label class="form-label" for="username">Username <span class="text-hint">(optional)</span></label>
					<input id="username" class="form-input" type="text" bind:value={formUsername} placeholder="e.g. @username" disabled={submitting} />
				</div>
				<div class="form-group mb-sm">
					<label class="form-label" for="access_token">Access Token</label>
					<input id="access_token" class="form-input" type="password" bind:value={formAccessToken} placeholder="Long-lived access token" disabled={submitting} />
				</div>
				<div class="form-group mb-sm">
					<label class="form-label" for="expires_at">Expires At <span class="text-hint">(optional)</span></label>
					<input id="expires_at" class="form-input" type="datetime-local" bind:value={formExpiresAt} disabled={submitting} />
				</div>
				<div class="form-group mb-sm">
					<label class="form-label" for="app_id">App ID <span class="text-hint">(optional)</span></label>
					<input id="app_id" class="form-input" type="text" bind:value={formAppId} placeholder="Facebook App ID" disabled={submitting} />
				</div>
				<div class="form-group mb-md">
					<label class="form-label" for="app_secret">App Secret <span class="text-hint">(optional)</span></label>
					<input id="app_secret" class="form-input" type="password" bind:value={formAppSecret} placeholder="Facebook App Secret" disabled={submitting} />
				</div>
				<div class="confirm-actions">
					<Button type="button" variant="outline" size="sm" onclick={() => (showAddModal = false)}>Cancel</Button>
					<Button type="submit" variant="default" size="sm" disabled={submitting}>{submitting ? 'Adding…' : 'Add Account'}</Button>
				</div>
		</form>
		</Dialog.Content>
	</Dialog.Root>
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
	.account-cell {
		display: flex;
		align-items: center;
		gap: var(--space-sm);
	}
	.account-avatar {
		width: 40px;
		height: 40px;
		border-radius: var(--radius-full);
		object-fit: cover;
		flex-shrink: 0;
	}
	.account-avatar-placeholder {
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--color-accent-subtle, oklch(60% 0.15 260 / 0.15));
		color: var(--color-accent, oklch(60% 0.15 260));
		font-weight: 600;
		font-size: var(--text-sm);
	}
	.account-bio {
		font-size: var(--text-xs);
		color: var(--color-muted);
		max-width: 280px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
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