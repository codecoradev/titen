<script lang="ts">
	import PageHeader from '$lib/components/PageHeader.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import { fetchMentions, createReply, listAccounts } from '$lib/api';
	import type { Mention, Account } from '$lib/types';
	import { formatDateTimeShort } from '$lib/tz';
	import { truncate } from '$lib/format';
	import { toast } from '$lib/toast.svelte';

	let mentions = $state<Mention[]>([]);
	let accounts = $state<Account[]>([]);
	let loading = $state(true);
	let loaded = $state(false);
	let fetchLoading = $state(false);
	let selectedAccountId = $state('');

	// Reply state
	let replyingTo = $state<Mention | null>(null);
	let replyText = $state('');
	let replyLoading = $state(false);

	function formatDate(iso?: string): string {
		if (!iso) return '---';
		return formatDateTimeShort(iso);
	}

	async function loadAccounts() {
		try {
			accounts = await listAccounts();
			if (accounts.length > 0 && !selectedAccountId) {
				selectedAccountId = accounts[0].id;
			}
		} catch (e: any) {
			toast(e.message || 'Failed to load accounts', 'error');
		}
	}

	async function handleFetch() {
		if (!selectedAccountId) return;
		fetchLoading = true;
		try {
			mentions = await fetchMentions(selectedAccountId, 25);
			toast(`Found ${mentions.length} mentions`, 'success');
		} catch (e: any) {
			toast(e.message || 'Failed to fetch mentions', 'error');
			mentions = [];
		} finally {
			fetchLoading = false;
		}
	}

	async function handleReply() {
		if (!replyingTo || !replyText.trim() || !selectedAccountId) return;
		replyLoading = true;
		try {
			await createReply({
				account_id: selectedAccountId,
				reply_to_id: replyingTo.id,
				text: replyText.trim(),
			});
			toast('Reply posted successfully', 'success');
			replyingTo = null;
			replyText = '';
		} catch (e: any) {
			toast(e.message || 'Failed to post reply', 'error');
		} finally {
			replyLoading = false;
		}
	}

	function startReply(mention: Mention) {
		replyingTo = mention;
		replyText = '';
	}

	function cancelReply() {
		replyingTo = null;
		replyText = '';
	}

	$effect(() => {
		if (loaded) return;
		(async () => {
			loading = true;
			await loadAccounts();
			loading = false;
			loaded = true;
		})();
	});
</script>

<PageHeader title="Mentions" description="Posts where your account is mentioned — reply directly">
	{#snippet action()}
		<button
			class="btn-primary btn-sm"
			onclick={handleFetch}
			disabled={!selectedAccountId || fetchLoading}
		>
			{fetchLoading ? 'Fetching...' : 'Fetch Mentions'}
		</button>
	{/snippet}
</PageHeader>

{#if accounts.length > 0}
<div class="filter-row">
	<label for="account-filter" class="filter-label">Account:</label>
	<select id="account-filter" class="select" bind:value={selectedAccountId}>
		{#each accounts as account}
			<option value={account.id}>@{account.username}</option>
		{/each}
	</select>
</div>
{/if}

<div class="data-table-wrap">
	{#if loading}
		<table class="data-table">
			<thead><tr><th>Author</th><th>Post</th><th>Date</th><th>Action</th></tr></thead>
			<tbody>
				{#each Array(3) as _}
					<tr>{#each Array(4) as _}<td><div class="skeleton" style="height: 1rem;"></div></td>{/each}</tr>
				{/each}
			</tbody>
		</table>
	{:else if mentions.length === 0}
		<div class="empty-state">
			<p class="empty-state-title">No mentions loaded</p>
			<p class="empty-state-desc">Select an account above, then click "Fetch Mentions" to pull recent mentions.</p>
		</div>
	{:else}
		<table class="data-table">
			<thead>
				<tr>
					<th>Author</th>
					<th>Post</th>
					<th>Date</th>
					<th>Action</th>
				</tr>
			</thead>
			<tbody>
				{#each mentions as mention (mention.id)}
					<tr>
						<td>@{mention.username ?? 'unknown'}</td>
						<td class="mention-text-cell" title={mention.text}>{truncate(mention.text, 80)}</td>
						<td>{formatDate(mention.timestamp)}</td>
						<td>
							<button class="btn-ghost btn-sm" onclick={() => startReply(mention)}>Reply</button>
							{#if mention.permalink}
								<a href={mention.permalink} target="_blank" rel="noopener" class="btn-ghost btn-sm">View</a>
							{/if}
						</td>
					</tr>
				{/each}
			</tbody>
		</table>
	{/if}
</div>

{#if replyingTo}
<div class="reply-overlay" role="dialog" aria-label="Reply to mention">
	<div class="reply-modal">
		<h3>Reply to @{replyingTo.username ?? 'unknown'}</h3>
		<p class="reply-original">{replyingTo.text}</p>
		<textarea
			bind:value={replyText}
			placeholder="Type your reply..."
			rows="4"
			maxlength="500"
			class="reply-textarea"
		></textarea>
		<div class="reply-actions">
			<button class="btn-ghost" onclick={cancelReply} disabled={replyLoading}>Cancel</button>
			<button class="btn-primary" onclick={handleReply} disabled={!replyText.trim() || replyLoading}>
				{replyLoading ? 'Posting...' : 'Post Reply'}
			</button>
		</div>
	</div>
</div>
{/if}

<style>
	.mention-text-cell {
		max-width: 20rem;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.filter-row {
		display: flex;
		align-items: center;
		gap: var(--space-xs);
		margin-bottom: var(--space-md);
	}

	.filter-label {
		font-size: var(--text-sm);
		color: var(--color-muted);
	}

	.reply-overlay {
		position: fixed;
		inset: 0;
		background: var(--overlay-scrim);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: var(--z-modal);
	}

	.reply-modal {
		background: var(--color-bg);
		border-radius: var(--radius-md);
		padding: var(--space-lg);
		max-width: 32rem;
		width: 90%;
		box-shadow: var(--shadow-lg);
	}

	.reply-modal h3 {
		margin-bottom: var(--space-xs);
	}

	.reply-original {
		font-size: var(--text-sm);
		color: var(--color-muted);
		padding: var(--space-xs) var(--space-sm);
		background: var(--color-bg-hover);
		border-radius: var(--radius-sm);
		margin-bottom: var(--space-sm);
		border-left: 3px solid var(--color-border);
	}

	.reply-textarea {
		width: 100%;
		resize: vertical;
		margin-bottom: var(--space-sm);
	}

	.reply-actions {
		display: flex;
		justify-content: flex-end;
		gap: var(--space-xs);
	}
</style>
