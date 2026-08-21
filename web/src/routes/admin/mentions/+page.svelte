<script lang="ts">
	import PageHeader from '$lib/components/PageHeader.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import { fetchMentions, createReply, listAccounts } from '$lib/api';
	import type { Mention, Account } from '$lib/types';
	import { formatDateTimeShort } from '$lib/tz';
	import { Button, buttonVariants } from '$lib/components/ui/button';
	import * as Select from '$lib/components/ui/select';
	import * as Table from '$lib/components/ui/table';
	import Skeleton from '$lib/components/ui/skeleton/skeleton.svelte';
	import Textarea from '$lib/components/ui/textarea/textarea.svelte';
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
		<Button
			variant="default"
			size="sm"
			onclick={handleFetch}
			disabled={!selectedAccountId || fetchLoading}
		>
			{fetchLoading ? 'Fetching...' : 'Fetch Mentions'}
		</Button>
	{/snippet}
</PageHeader>

{#if accounts.length > 0}
<div class="filter-row">
	<label class="filter-label">Account:</label>
	<Select.Root type="single" bind:value={selectedAccountId}>
		<Select.Trigger>
			{selectedAccountId ? `@${accounts.find((a) => a.id === selectedAccountId)?.username ?? ''}` : 'Select account...'}
		</Select.Trigger>
		<Select.Content>
			{#each accounts as account (account.id)}
				<Select.Item value={account.id} label={`@${account.username}`}>
					@{account.username}
				</Select.Item>
			{/each}
		</Select.Content>
	</Select.Root>
</div>
{/if}

<div class="data-table-wrap">
	{#if loading}
		<Table.Root>
			<Table.Header><Table.Row><Table.Head>Author</Table.Head><Table.Head class="hidden md:table-cell">Post</Table.Head><Table.Head>Date</Table.Head><Table.Head>Action</Table.Head></Table.Row></Table.Header>
			<Table.Body>
				{#each Array(3) as _}
					<Table.Row>
						{#each Array(4) as _}
							<Table.Cell><Skeleton class="h-4 w-full" /></Table.Cell>
						{/each}
					</Table.Row>
				{/each}
			</Table.Body>
		</Table.Root>
	{:else if mentions.length === 0}
		<div class="empty-state">
			<p class="empty-state-title">No mentions loaded</p>
			<p class="empty-state-desc">Select an account above, then click "Fetch Mentions" to pull recent mentions.</p>
		</div>
	{:else}
		<Table.Root>
			<Table.Header>
				<Table.Row>
					<Table.Head>Author</Table.Head>
					<Table.Head class="hidden md:table-cell">Post</Table.Head>
					<Table.Head>Date</Table.Head>
					<Table.Head>Action</Table.Head>
				</Table.Row>
			</Table.Header>
			<Table.Body>
				{#each mentions as mention (mention.id)}
					<Table.Row>
						<Table.Cell>@{mention.username ?? 'unknown'}</Table.Cell>
						<Table.Cell class="mention-text-cell hidden md:table-cell" title={mention.text}>{truncate(mention.text, 80)}</Table.Cell>
						<Table.Cell>{formatDate(mention.timestamp)}</Table.Cell>
						<Table.Cell>
							<Button variant="ghost" size="sm" onclick={() => startReply(mention)}>Reply</Button>
							{#if mention.permalink}
								<a href={mention.permalink} target="_blank" rel="noopener" class={buttonVariants({ variant: 'ghost', size: 'sm' })}>View</a>
							{/if}
						</Table.Cell>
					</Table.Row>
				{/each}
			</Table.Body>
		</Table.Root>
	{/if}
</div>

{#if replyingTo}
<div class="reply-overlay" role="dialog" aria-label="Reply to mention">
	<div class="reply-modal">
		<h3>Reply to @{replyingTo.username ?? 'unknown'}</h3>
		<p class="reply-original">{replyingTo.text}</p>
		<Textarea
			bind:value={replyText}
			placeholder="Type your reply..."
			rows={4}
			maxlength={500}
			class="reply-textarea"
		/>
		<div class="reply-actions">
			<Button variant="ghost" onclick={cancelReply} disabled={replyLoading}>Cancel</Button>
			<Button variant="default" onclick={handleReply} disabled={!replyText.trim() || replyLoading}>
				{replyLoading ? 'Posting...' : 'Post Reply'}
			</Button>
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
