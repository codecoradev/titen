<script lang="ts" generics="T extends Record<string, any>">
	import type { Snippet } from 'svelte';
	import * as Table from '$lib/components/ui/table';
	import Skeleton from '$lib/components/ui/skeleton/skeleton.svelte';

	interface Column {
		key: string;
		label: string;
		sortable?: boolean;
		class?: string;
		/** Hide this column below md breakpoint (dense tables on mobile). */
		hideOnMobile?: boolean;
	}

	interface Props {
		columns: Column[];
		rows: T[];
		loading?: boolean;
		emptyTitle?: string;
		emptyDesc?: string;
		/** Custom cell renderer — receives (row, col.key). Falls back to row[col.key]. */
		cell?: Snippet<[T, string]>;
		/** Row-level actions column content — receives the row. */
		actions?: Snippet<[T]>;
		actionsLabel?: string;
		/** Called when a row is clicked (or Enter pressed while focused). */
		onrowclick?: (row: T) => void;
		/** Per-row extra class, e.g. highlight drafts. */
		rowClass?: (row: T) => string;
		/** Expanded-row detail renderer — receives the row. Requires detailRowKey to track expansion. */
		detail?: Snippet<[T]>;
		/** Row key for tracking which row is expanded. Defaults to getRowKey. */
		expandable?: boolean;
	}

	let {
		columns,
		rows,
		loading = false,
		emptyTitle = 'No data',
		emptyDesc,
		cell,
		actions,
		actionsLabel = 'Actions',
		onrowclick,
		rowClass,
		detail,
		expandable = false
	}: Props = $props();

	let sortKey = $state<string | null>(null);
	let sortAsc = $state(true);
	let expandedKey = $state<string | number | null>(null);

	const allColumns = $derived(
		actions ? [...columns, { key: '__actions', label: actionsLabel, class: 'col-actions' }] : columns
	);

	const sorted = $derived.by(() => {
		if (!sortKey) return rows;
		return [...rows].sort((a, b) => {
			const av = a[sortKey!];
			const bv = b[sortKey!];
			if (av == null) return 1;
			if (bv == null) return -1;
			if (typeof av === 'number' && typeof bv === 'number') return sortAsc ? av - bv : bv - av;
			return sortAsc
				? String(av).localeCompare(String(bv))
				: String(bv).localeCompare(String(av));
		});
	});

	function colClass(col: Column): string | undefined {
		if (!col.hideOnMobile) return col.class;
		return [col.class, 'hidden md:table-cell'].filter(Boolean).join(' ');
	}

	function toggleSort(key: string) {
		if (sortKey === key) {
			sortAsc = !sortAsc;
		} else {
			sortKey = key;
			sortAsc = true;
		}
	}

	function getRowKey(row: T): string {
		return row.id ?? JSON.stringify(row);
	}
</script>

<div class="data-table-wrap">
	{#if loading}
		<Table.Root>
			<Table.Header>
				<Table.Row>
					{#each allColumns as col}
						<Table.Head class={colClass(col)}>{col.label}</Table.Head>
					{/each}
				</Table.Row>
			</Table.Header>
			<Table.Body>
				{#each Array(5) as _}
					<Table.Row>
						{#each allColumns as col}
							<Table.Cell class={colClass(col)}><Skeleton class="h-4 w-full" /></Table.Cell>
						{/each}
					</Table.Row>
				{/each}
			</Table.Body>
		</Table.Root>
	{:else if rows.length === 0}
		<div class="empty-state">
			<p class="empty-state-title">{emptyTitle}</p>
			{#if emptyDesc}
				<p class="empty-state-desc">{emptyDesc}</p>
			{/if}
		</div>
	{:else}
		<Table.Root>
			<Table.Header>
				<Table.Row>
					{#each allColumns as col}
						<Table.Head
							class={[col.sortable ? 'cursor-pointer select-none' : '', colClass(col)].filter(Boolean).join(' ')}
							onclick={() => col.sortable && toggleSort(col.key)}
						>
							{col.label}
							{#if col.sortable && sortKey === col.key}
								<span class="ml-1 text-xs">{sortAsc ? '↑' : '↓'}</span>
							{/if}
						</Table.Head>
					{/each}
				</Table.Row>
			</Table.Header>
			<Table.Body>
				{#each sorted as row (getRowKey(row))}
					{@const rowKey = getRowKey(row)}
					{@const isExpanded = expandable && detail && expandedKey === rowKey}
					<Table.Row
						class={[onrowclick ? 'row-clickable' : '', rowClass?.(row) ?? '', isExpanded ? 'row-expanded' : '']
							.filter(Boolean)
							.join(' ')}
						onclick={() => {
							if (expandable && detail) {
								expandedKey = isExpanded ? null : rowKey;
							}
							onrowclick?.(row);
						}}
						onkeydown={((e: KeyboardEvent) => {
							if (e.key !== 'Enter' && e.key !== ' ') return;
							const t = e.target as HTMLElement;
							if (t.closest('button, a, input, select, textarea')) return;
							if (!expandable || !detail) {
								if (e.key === 'Enter') onrowclick?.(row);
								return;
							}
							e.preventDefault();
							expandedKey = expandedKey === rowKey ? null : rowKey;
						})}
						role={onrowclick || (expandable && detail) ? 'button' : undefined}
						tabindex={onrowclick || (expandable && detail) ? 0 : undefined}
						aria-expanded={expandable && detail ? isExpanded : undefined}
					>
						{#each columns as col}
							<Table.Cell class={colClass(col)}>
								{#if cell}{@render cell(row, col.key)}
								{:else}{row[col.key] ?? '—'}{/if}
							</Table.Cell>
						{/each}
						{#if actions}
							<Table.Cell class="col-actions" onclick={(e) => e.stopPropagation()}>
								{@render actions(row)}
							</Table.Cell>
						{/if}
						</Table.Row>
						{#if isExpanded}
							<Table.Row class="row-detail">
								<Table.Cell colspan={allColumns.length}>
									{@render detail(row)}
								</Table.Cell>
							</Table.Row>
						{/if}
						{/each}
			</Table.Body>
		</Table.Root>
	{/if}
</div>

<style>
	.row-clickable {
		cursor: pointer;
		transition: background-color 0.1s ease;
	}

	.row-clickable:hover {
		background: var(--color-bg-hover, rgba(0, 0, 0, 0.03));
	}

	.row-expanded {
		background: var(--color-bg-hover, rgba(0, 0, 0, 0.04));
	}

	:global(.row-detail td) {
		background: var(--color-bg-subtle, rgba(0, 0, 0, 0.025));
		border-top: none;
	}
</style>
