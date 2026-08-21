<script lang="ts">
	import PageHeader from '$lib/components/PageHeader.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import PostDetail from '$lib/components/PostDetail.svelte';
	import { listPosts, deletePost, getPostInsights, listAccounts } from '$lib/api';
	import { Button } from '$lib/components/ui/button';
	import * as Select from '$lib/components/ui/select';
	import * as Table from '$lib/components/ui/table';
	import Skeleton from '$lib/components/ui/skeleton/skeleton.svelte';
	import { formatDateShort } from '$lib/tz';
	import { toast } from '$lib/toast.svelte';
	import type { Post, Account } from '$lib/types';

	let loading = $state(true);
	let loaded = $state(false);
	let posts = $state<Post[]>([]);
	let accounts = $state<Account[]>([]);
	let filterAccount = $state('');
	let filterStatus = $state('');
	let expandedPostId = $state<string | null>(null);
	let insights: Record<string, any> = $state({});
	let insightsLoading = $state<string | null>(null);
	let confirmDelete = $state<{ open: boolean; post: Post | null }>({ open: false, post: null });

	// Detail modal
	let detailPost = $state<Post | null>(null);

	function openDetail(post: Post) {
		// Enrich with account info for permalink
		const enriched = { ...post, account: accounts.find(a => a.id === post.account_id) };
		detailPost = enriched;
	}
	function closeDetail() {
		detailPost = null;
	}

	const filtered = $derived.by(() => {
		let result = posts;
		if (filterAccount) result = result.filter((p) => p.account_id === filterAccount);
		if (filterStatus) result = result.filter((p) => p.status === filterStatus);
		return result;
	});

	function formatDate(iso: string | null): string {
		if (!iso) return '\u2014';
		return formatDateShort(iso);
	}

	async function loadPosts() {
		loading = true;
		try {
			const [p, a] = await Promise.all([
				listPosts().catch(() => []),
				listAccounts().catch(() => []),
			]);
			posts = p;
			accounts = a;
		} catch (e: any) {
			toast('Failed to load posts', 'error');
		} finally {
			loading = false;
			loaded = true;
		}
	}

	function getAccountUsername(accountId: string): string {
		return accounts.find((a) => a.id === accountId)?.username ?? accountId.slice(0, 8);
	}

	async function toggleInsights(postId: string) {
		if (expandedPostId === postId) {
			expandedPostId = null;
			return;
		}
		expandedPostId = postId;
		if (!insights[postId]) {
			insightsLoading = postId;
			try {
				const res = await getPostInsights(postId);
				insights[postId] = res;
			} catch {
				toast('Failed to load insights', 'error');
				expandedPostId = null;
			} finally {
				insightsLoading = null;
			}
		}
	}

	async function handleDelete() {
		if (!confirmDelete.post) return;
		try {
			await deletePost(confirmDelete.post.id);
			toast('Post deleted', 'success');
			posts = posts.filter((p) => p.id !== confirmDelete.post!.id);
		} catch {
			toast('Failed to delete post', 'error');
		} finally {
			confirmDelete = { open: false, post: null };
		}
	}

	$effect(() => {
		if (!loaded) loadPosts();
	});
</script>

<PageHeader title="Posts" description="Manage and monitor your Threads content.">
	{#snippet action()}
		<div class="row-gap-sm">
			<Select.Root type="single" bind:value={filterAccount}>
				<Select.Trigger>
					{filterAccount ? `@${accounts.find((a) => a.id === filterAccount)?.username ?? ''}` : 'All Accounts'}
				</Select.Trigger>
				<Select.Content>
					<Select.Item value="" label="All Accounts">All Accounts</Select.Item>
					{#each accounts as account (account.id)}
						<Select.Item value={account.id} label={`@${account.username}`}>
							@{account.username}
						</Select.Item>
					{/each}
				</Select.Content>
			</Select.Root>
			<Select.Root type="single" bind:value={filterStatus}>
				<Select.Trigger>
					{filterStatus === '' ? 'All Status' : filterStatus.charAt(0).toUpperCase() + filterStatus.slice(1)}
				</Select.Trigger>
				<Select.Content>
					<Select.Item value="" label="All Status">All Status</Select.Item>
					<Select.Item value="draft" label="Draft">Draft</Select.Item>
					<Select.Item value="published" label="Published">Published</Select.Item>
					<Select.Item value="failed" label="Failed">Failed</Select.Item>
					<Select.Item value="deleted" label="Deleted">Deleted</Select.Item>
				</Select.Content>
			</Select.Root>
		</div>
	{/snippet}
</PageHeader>

<div class="data-table-wrap">
	{#if loading}
		<Table.Root>
			<Table.Header><Table.Row><Table.Head>Content</Table.Head><Table.Head>Account</Table.Head><Table.Head class="hidden md:table-cell">Type</Table.Head><Table.Head>Status</Table.Head><Table.Head class="hidden md:table-cell">Published</Table.Head><Table.Head>Actions</Table.Head></Table.Row></Table.Header>
			<Table.Body>
				{#each Array(4) as _}
					<Table.Row>
						{#each Array(6) as _}
							<Table.Cell><Skeleton class="h-4 w-full" /></Table.Cell>
						{/each}
					</Table.Row>
				{/each}
			</Table.Body>
		</Table.Root>
	{:else if filtered.length === 0}
		<div class="empty-state">
			<p class="empty-state-title">No posts yet</p>
			<p class="empty-state-desc">Posts will appear here once you publish content.</p>
		</div>
	{:else}
		<Table.Root>
			<Table.Header>
				<Table.Row>
					<Table.Head>Content</Table.Head>
					<Table.Head>Account</Table.Head>
					<Table.Head class="hidden md:table-cell">Type</Table.Head>
					<Table.Head>Status</Table.Head>
					<Table.Head class="hidden md:table-cell">Published</Table.Head>
					<Table.Head>Actions</Table.Head>
				</Table.Row>
			</Table.Header>
			<Table.Body>
				{#each filtered as post (post.id)}
					<Table.Row class="row-clickable" onclick={() => openDetail(post)} role="button" tabindex={0} onkeydown={(e) => e.key === 'Enter' && openDetail(post)}>
						<Table.Cell class="truncate truncate-mw-40">
							{post.caption ? (post.caption.length > 40 ? post.caption.slice(0, 40) + '…' : post.caption) : '(no caption)'}
						</Table.Cell>
						<Table.Cell><span style="color:var(--color-muted);">@{getAccountUsername(post.account_id)}</span></Table.Cell>
						<Table.Cell class="hidden md:table-cell">{post.media_type}</Table.Cell>
						<Table.Cell><StatusBadge status={post.status} /></Table.Cell>
						<Table.Cell class="hidden md:table-cell"><span class="tabular-nums">{formatDate(post.published_at)}</span></Table.Cell>
						<Table.Cell onclick={(e) => e.stopPropagation()}>
							<div class="row-gap-xs">
								<Button variant="outline" size="sm" onclick={() => openDetail(post)}>Detail</Button>
								<Button variant="ghost" size="sm" onclick={() => (confirmDelete = { open: true, post })}>
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

<!-- Post Detail Modal -->
{#if detailPost}
	{@const postWithAccount = { ...detailPost, account: accounts.find(a => a.id === detailPost?.account_id) }}
	<PostDetail post={postWithAccount} onClose={closeDetail} />
{/if}

<ConfirmDialog
	open={confirmDelete.open}
	title="Delete Post"
	message="This will permanently remove this post. This action cannot be undone."
	confirmLabel="Delete"
	variant="danger"
	onconfirm={handleDelete}
	oncancel={() => (confirmDelete = { open: false, post: null })}
/>

<svelte:window onkeydown={(e) => {
	if (e.key === 'Escape' && detailPost) closeDetail();
}} />

<style>
	.row-clickable {
		cursor: pointer;
		transition: background-color 0.1s ease;
	}

	.row-clickable:hover {
		background: var(--color-bg-hover, rgba(0, 0, 0, 0.03));
	}
</style>