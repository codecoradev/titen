<script lang="ts">
	import PageHeader from '$lib/components/PageHeader.svelte';
	import { getHealth, getSettings, updateSettings, ApiError } from '$lib/api';
	import { toast } from '$lib/toast.svelte';
	import { Button } from '$lib/components/ui/button';
	import type { HealthResponse } from '$lib/types';

	// ── State ──
	let activeTab = $state<'general' | 'api-keys' | 'danger'>('general');
	let saving = $state(false);
	let loading = $state(false);

	// General settings
	let instanceName = $state('');
	let autoFetchComments = $state(true);
	let commentFetchInterval = $state('30');
	let scheduleLookaheadHours = $state('24');

	// API keys (masked)
	let threadsAppId = $state('');
	let threadsAppSecret = $state('');
	let secretIsSet = $state(false); // true if backend has a secret stored
	let showAppId = $state(false);
	let showAppSecret = $state(false);

	// Track whether user typed a new secret
	let secretDirty = $state(false);

	// Danger zone
	let confirmPurgeText = $state('');
	let confirmDeleteText = $state('');

	// Health
	let health = $state<HealthResponse | null>(null);
	let healthLoading = $state(false);

	// Lifecycle guard
	let loaded = $state(false);

	const tabs = [
		{ id: 'general' as const, label: 'General' },
		{ id: 'api-keys' as const, label: 'API Keys' },
		{ id: 'danger' as const, label: 'Danger Zone' },
	];

	// ── Lifecycle ──
	$effect(() => {
		if (!loaded) {
			loaded = true;
			loadSettings();
		}
	});

	async function loadSettings() {
		loading = true;
		try {
			const s = await getSettings();
			instanceName = s.instance_name ?? '';
			autoFetchComments = s.auto_fetch_comments ?? true;
			commentFetchInterval = s.comment_fetch_interval ?? '30';
			scheduleLookaheadHours = s.schedule_lookahead_hours ?? '24';
			threadsAppId = s.threads_app_id ?? '';
			secretIsSet = s.threads_app_secret_set ?? false;
			threadsAppSecret = '';
			secretDirty = false;
		} catch (e) {
			if (e instanceof ApiError) {
				toast(`Failed to load settings: ${e.status}`, 'error');
			} else {
				toast('Failed to load settings', 'error');
			}
		} finally {
			loading = false;
		}
	}

	async function saveGeneral() {
		saving = true;
		try {
			const s = await updateSettings({
				instance_name: instanceName,
				auto_fetch_comments: autoFetchComments,
				comment_fetch_interval: commentFetchInterval,
				schedule_lookahead_hours: scheduleLookaheadHours,
			});
			secretIsSet = s.threads_app_secret_set ?? secretIsSet;
			toast('General settings saved', 'success');
		} catch (e) {
			if (e instanceof ApiError) {
				toast(`Failed to save: ${e.status}`, 'error');
			} else {
				toast('Failed to save settings', 'error');
			}
		} finally {
			saving = false;
		}
	}

	async function saveApiKeys() {
		saving = true;
		try {
			const payload: Record<string, string> = {};
			if (threadsAppId) payload.threads_app_id = threadsAppId;
			// Only send secret if user typed a new one
			if (secretDirty && threadsAppSecret) {
				payload.threads_app_secret = threadsAppSecret;
			}
			const s = await updateSettings(payload);
			secretIsSet = s.threads_app_secret_set ?? secretIsSet;
			threadsAppSecret = '';
			secretDirty = false;
			showAppId = false;
			showAppSecret = false;
			toast('API keys saved', 'success');
		} catch (e) {
			if (e instanceof ApiError) {
				toast(`Failed to save: ${e.status}`, 'error');
			} else {
				toast('Failed to save API keys', 'error');
			}
		} finally {
			saving = false;
		}
	}

	async function refreshHealth() {
		healthLoading = true;
		try {
			health = await getHealth();
		} catch (e) {
			if (e instanceof ApiError) {
				toast(`Health check failed: ${e.status}`, 'error');
			} else {
				toast('Health check failed', 'error');
			}
			health = null;
		} finally {
			healthLoading = false;
		}
	}


	async function purgeFailedPosts() {
		if (confirmPurgeText !== 'PURGE') return;
		try {
			const res = await fetch('/api/posts/purge-failed', { method: 'POST' });
			if (!res.ok) throw new Error(`HTTP ${res.status}`);
			const data = await res.json();
			toast(`Purged ${data.deleted ?? 0} failed posts`, 'success');
			confirmPurgeText = '';
		} catch {
			toast('Purge failed — endpoint may not be available yet', 'error');
		}
	}

	async function deleteAllSchedules() {
		if (confirmDeleteText !== 'DELETE ALL') return;
		try {
			const res = await fetch('/api/schedules', { method: 'DELETE' });
			if (!res.ok) throw new Error(`HTTP ${res.status}`);
			const data = await res.json();
			toast(`Deleted ${data.deleted ?? 0} schedules`, 'success');
			confirmDeleteText = '';
		} catch {
			toast('Delete failed — endpoint may not be available yet', 'error');
		}
	}
</script>

<PageHeader title="Settings" description="Instance configuration and API credentials" />

<!-- Tabs -->
<div class="settings-tabs">
	{#each tabs as tab}
		<button
			class="settings-tab"
			class:is-active={activeTab === tab.id}
			onclick={() => (activeTab = tab.id)}
			type="button"
		>
			{tab.label}
		</button>
	{/each}
</div>

<!-- ── General ── -->
{#if activeTab === 'general'}
	<section class="settings-section">
		<h2 class="settings-section-title">Instance</h2>
		<div class="settings-card">
			<div class="form-group">
				<label class="form-label" for="instance-name">Instance Name</label>
				<input
					id="instance-name"
					class="form-input"
					type="text"
					bind:value={instanceName}
					placeholder="My Titen Instance"
				/>
				<span class="form-helper">Display name shown in the sidebar and page title</span>
			</div>
		</div>

		<h2 class="settings-section-title">Automation</h2>
		<div class="settings-card">
			<div class="form-row">
				<div class="form-group">
					<label class="form-label" for="comment-interval">Comment Fetch Interval</label>
					<input
						id="comment-interval"
						class="form-input"
						type="number"
						min="5"
						max="1440"
						bind:value={commentFetchInterval}
					/>
					<span class="form-helper">Minutes between auto-fetch cycles (5–1440)</span>
				</div>
				<div class="form-group">
					<label class="form-label" for="schedule-lookahead">Schedule Lookahead</label>
					<input
						id="schedule-lookahead"
						class="form-input"
						type="number"
						min="1"
						max="168"
						bind:value={scheduleLookaheadHours}
					/>
					<span class="form-helper">Hours ahead to show upcoming schedules</span>
				</div>
			</div>

			<label class="form-toggle">
				<input type="checkbox" bind:checked={autoFetchComments} />
				<span class="form-toggle-track"></span>
				<span>Auto-fetch comments for published posts</span>
			</label>
		</div>

		<div class="settings-actions">
			<Button variant="default" onclick={saveGeneral} disabled={saving}>
				{saving ? 'Saving…' : 'Save Changes'}
			</Button>
		</div>
	</section>

	<!-- System Health -->
	<section class="settings-section">
		<div class="settings-section-row">
			<h2 class="settings-section-title">System Health</h2>
			<Button variant="outline" size="sm" onclick={refreshHealth} disabled={healthLoading}>
				{healthLoading ? 'Checking…' : 'Refresh'}
			</Button>
		</div>
		{#if health}
			<div class="settings-card">
				<div class="health-grid">
					<div class="health-item">
						<span class="health-label">Status</span>
						<span class="badge badge--{health.status === 'ok' ? 'success' : 'error'}">{health.status}</span>
					</div>
					<div class="health-item">
						<span class="health-label">Version</span>
						<span class="health-value tabular-nums">{health.version}</span>
					</div>
					<div class="health-item">
						<span class="health-label">Database</span>
						<span class="health-value tabular-nums">{health.db}</span>
					</div>
				</div>
			</div>
		{:else if !healthLoading}
			<div class="settings-card">
				<p class="form-helper">Click <strong>Refresh</strong> to check system health</p>
			</div>
		{/if}
	</section>
{/if}

<!-- ── API Keys ── -->
{#if activeTab === 'api-keys'}
	<section class="settings-section">
		<div class="settings-card settings-card--info">
			<p class="settings-info-text">
				Credentials are encrypted at rest (AES-256-GCM) and stored server-side.
				The App Secret is never exposed to the browser after saving.
			</p>
		</div>

		<h2 class="settings-section-title">Threads API</h2>
		<div class="settings-card">
			<div class="form-group">
				<label class="form-label" for="threads-app-id">App ID</label>
				<div class="input-reveal">
					<input
						id="threads-app-id"
						class="form-input"
						type={showAppId ? 'text' : 'password'}
						bind:value={threadsAppId}
						placeholder="Threads App ID"
					/>
					<Button variant="ghost" class="reveal-btn" type="button" onclick={() => (showAppId = !showAppId)} aria-label={showAppId ? 'Hide' : 'Show'}>
						{#if showAppId}
							<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94"/><path d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19"/><line x1="1" y1="1" x2="23" y2="23"/></svg>
						{:else}
							<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>
						{/if}
						</Button>
						</div>
						</div>

			<div class="form-group">
				<label class="form-label" for="threads-app-secret">App Secret</label>
				{#if secretIsSet && !secretDirty}
					<div class="secret-status">
						<span class="badge badge--success">✓ Configured</span>
						<Button variant="ghost" size="sm" type="button" onclick={() => { secretDirty = true; showAppSecret = true; }}>Replace</Button>
					</div>
				{:else}
					<div class="input-reveal">
						<input
							id="threads-app-secret"
							class="form-input"
							type={showAppSecret ? 'text' : 'password'}
							bind:value={threadsAppSecret}
							oninput={() => { secretDirty = true; }}
							placeholder={secretIsSet ? 'Enter new secret to replace' : 'Threads App Secret'}
							autocomplete="off"
						/>
						<Button variant="ghost" class="reveal-btn" type="button" onclick={() => (showAppSecret = !showAppSecret)} aria-label={showAppSecret ? 'Hide' : 'Show'}>
							{#if showAppSecret}
								<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94"/><path d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19"/><line x1="1" y1="1" x2="23" y2="23"/></svg>
							{:else}
								<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>
							{/if}
							</Button>
							</div>
				{/if}
			</div>
		</div>

		<div class="settings-actions">
			<Button variant="default" onclick={saveApiKeys} disabled={saving}>
				{saving ? 'Saving…' : 'Save API Keys'}
			</Button>
		</div>
	</section>
{/if}

<!-- ── Danger Zone ── -->
{#if activeTab === 'danger'}
	<section class="settings-section">
		<div class="settings-card settings-card--danger">
			<h3 class="settings-danger-title">Purge Failed Posts</h3>
			<p class="settings-danger-desc">
				Permanently delete all posts with a <span class="badge badge--error">failed</span> status.
				This action cannot be undone.
			</p>
			<div class="danger-confirm">
				<label class="form-label" for="confirm-purge">Type <code>PURGE</code> to confirm</label>
				<input
					id="confirm-purge"
					class="form-input"
					type="text"
					bind:value={confirmPurgeText}
					placeholder="PURGE"
				/>
			</div>
			<Button variant="destructive"
			onclick={purgeFailedPosts}
			disabled={confirmPurgeText !== 'PURGE'}
		>
				Purge Failed Posts
			</Button>
		</div>

		<div class="settings-card settings-card--danger">
			<h3 class="settings-danger-title">Delete All Schedules</h3>
			<p class="settings-danger-desc">
				Remove all scheduled posts including pending, processing, and failed entries.
				This action cannot be undone.
			</p>
			<div class="danger-confirm">
				<label class="form-label" for="confirm-delete">Type <code>DELETE ALL</code> to confirm</label>
				<input
					id="confirm-delete"
					class="form-input"
					type="text"
					bind:value={confirmDeleteText}
					placeholder="DELETE ALL"
				/>
			</div>
			<Button variant="destructive"
			onclick={deleteAllSchedules}
			disabled={confirmDeleteText !== 'DELETE ALL'}
		>
				Delete All Schedules
			</Button>
		</div>
	</section>
{/if}

<style>
	/* ── Settings tabs ── */
	.settings-tabs {
		display: flex;
		gap: var(--space-2xs);
		border-bottom: var(--rule-default);
		margin-bottom: var(--space-lg);
	}

	.settings-tab {
		font-family: var(--font-body);
		font-size: var(--text-sm);
		font-weight: 500;
		color: var(--color-muted);
		background: none;
		border: none;
		padding: var(--space-sm) var(--space-md);
		cursor: pointer;
		transition: color var(--dur-short) var(--ease-out);
		border-bottom: 2px solid transparent;
		margin-bottom: -1px;
	}

	.settings-tab:hover {
		color: var(--color-ink);
	}

	.settings-tab.is-active {
		color: var(--color-ink);
		border-bottom-color: var(--color-accent);
	}

	/* ── Settings sections ── */
	.settings-section {
		margin-bottom: var(--space-xl);
	}

	.settings-section-title {
		font-size: var(--text-md);
		font-weight: 600;
		margin-bottom: var(--space-sm);
		margin-top: var(--space-lg);
	}

	.settings-section-title:first-child {
		margin-top: 0;
	}

	.settings-section-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-sm);
		margin-bottom: var(--space-sm);
		flex-wrap: wrap;
	}

	.settings-section-row .settings-section-title {
		margin-bottom: 0;
		margin-top: 0;
	}

	/* ── Settings card ── */
	.settings-card {
		background: var(--surface-raised);
		border: var(--rule-default);
		border-radius: var(--radius-lg);
		padding: var(--space-lg);
		display: flex;
		flex-direction: column;
		gap: var(--space-md);
	}

	.settings-card--info {
		border-color: var(--color-accent);
		background: var(--color-accent-dim, color-mix(in srgb, var(--color-accent) 8%, var(--surface-raised)));
	}

	.settings-info-text {
		font-size: var(--text-sm);
		color: var(--color-muted);
		line-height: 1.6;
	}

	.secret-status {
		display: flex;
		align-items: center;
		gap: var(--space-sm);
	}

	.settings-card--danger {
		border-color: var(--color-error);
		border-width: 1.5px;
	}

	/* ── Form row (side-by-side fields) ── */
	.form-row {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(14rem, 1fr));
		gap: var(--space-md);
	}

	/* ── Toggle switch ── */
	.form-toggle {
		display: flex;
		align-items: center;
		gap: var(--space-sm);
		font-size: var(--text-sm);
		cursor: pointer;
		user-select: none;
	}

	.form-toggle input {
		position: absolute;
		opacity: 0;
		width: 0;
		height: 0;
	}

	.form-toggle-track {
		position: relative;
		width: 2.5rem;
		height: 1.375rem;
		background: var(--color-rule-2);
		border-radius: var(--radius-pill);
		transition: background-color var(--dur-short) var(--ease-out);
		flex-shrink: 0;
	}

	.form-toggle-track::after {
		content: '';
		position: absolute;
		inset-block-start: 2px;
		inset-inline-start: 2px;
		width: calc(1.375rem - 4px);
		height: calc(1.375rem - 4px);
		background: white;
		border-radius: 50%;
		transition: transform var(--dur-short) var(--ease-out);
		box-shadow: var(--shadow-whisper);
	}

	.form-toggle input:checked + .form-toggle-track {
		background: var(--color-accent);
	}

	.form-toggle input:checked + .form-toggle-track::after {
		transform: translateX(calc(1.125rem));
	}

	.form-toggle input:focus-visible + .form-toggle-track {
		outline: 2px solid var(--color-focus);
		outline-offset: 2px;
	}

	/* ── Input with reveal button ── */
	.input-reveal {
		display: flex;
		gap: var(--space-2xs);
		align-items: center;
	}

	.input-reveal .form-input {
		flex: 1;
	}

	/* ── Save actions ── */
	.settings-actions {
		display: flex;
		justify-content: flex-start;
		margin-top: var(--space-md);
	}

	/* ── Health grid ── */
	.health-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(10rem, 1fr));
		gap: var(--space-md);
	}

	.health-item {
		display: flex;
		flex-direction: column;
		gap: var(--space-3xs);
	}

	.health-label {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-muted);
		text-transform: uppercase;
		letter-spacing: 0.08em;
	}

	.health-value {
		font-family: var(--font-display);
		font-size: var(--text-md);
		font-weight: 600;
	}

	/* ── Danger zone ── */
	.settings-danger-title {
		font-size: var(--text-base);
		font-weight: 600;
		color: var(--color-error);
	}

	.settings-danger-desc {
		font-size: var(--text-sm);
		color: var(--color-muted);
		line-height: 1.6;
		margin-bottom: var(--space-xs);
	}

	.danger-confirm {
		display: flex;
		flex-direction: column;
		gap: var(--space-2xs);
		margin-top: var(--space-xs);
	}

	.danger-confirm .form-input {
		max-width: 20rem;
	}

	/* ── Responsive ── */
	@media (max-width: 48rem) {
		.settings-card {
			padding: var(--space-md);
		}

		.form-row {
			grid-template-columns: 1fr;
		}
	}
</style>
