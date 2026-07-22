<script lang="ts">
	interface Column {
		key: string;
		label: string;
		sortable?: boolean;
		class?: string;
	}

	interface Props {
		columns: Column[];
		rows: Record<string, any>[];
		loading?: boolean;
		emptyTitle?: string;
		emptyDesc?: string;
	}

	let { columns, rows, loading = false, emptyTitle = 'No data', emptyDesc }: Props = $props();

	let sortKey = $state<string | null>(null);
	let sortAsc = $state(true);

	const sorted = $derived.by(() => {
		if (!sortKey) return rows;
		return [...rows].sort((a, b) => {
			const av = a[sortKey!];
			const bv = b[sortKey!];
			if (av == null) return 1;
			if (bv == null) return -1;
			if (typeof av === 'number' && typeof bv === 'number') return sortAsc ? av - bv : bv - av;
			return sortAsc ? String(av).localeCompare(String(bv)) : String(bv).localeCompare(String(av));
		});
	});

	function toggleSort(key: string) {
		if (sortKey === key) {
			sortAsc = !sortAsc;
		} else {
			sortKey = key;
			sortAsc = true;
		}
	}

	function getRowKey(row: Record<string, any>): string {
		return row.id ?? JSON.stringify(row);
	}
</script>

<div class="data-table-wrap">
	{#if loading}
		<table class="data-table">
			<thead>
				<tr>
					{#each columns as col}
						<th>{col.label}</th>
					{/each}
			</tr>
			</thead>
			<tbody>
				{#each Array(5) as _}
						<tr>
							{#each columns as _}
								<td><div class="skeleton" style="height: 1rem;"></div></td>
							{/each}
						</tr>
				{/each}
			</tbody>
		</table>
	{:else if rows.length === 0}
		<div class="empty-state">
			<p class="empty-state-title">{emptyTitle}</p>
			{#if emptyDesc}
				<p class="empty-state-desc">{emptyDesc}</p>
			{/if}
		</div>
	{:else}
		<table class="data-table">
			<thead>
				<tr>
					{#each columns as col}
							<th
								class={col.sortable ? 'sortable' : ''}
								class:sort-active={sortKey === col.key}
								onclick={() => col.sortable && toggleSort(col.key)}
							>
								{col.label}
								{#if col.sortable && sortKey === col.key}
									<span class="sort-arrow">{sortAsc ? '↑' : '↓'}</span>
								{/if}
							</th>
					{/each}
				</tr>
			</thead>
			<tbody>
				{#each sorted as row (getRowKey(row))}
						<tr>
							{#each columns as col}
								<td class={col.class}>
									{row[col.key] ?? '—'}
								</td>
							{/each}
						</tr>
				{/each}
			</tbody>
		</table>
	{/if}
</div>

<style>
	.sortable {
		cursor: pointer;
		user-select: none;
	}

	.sortable:hover {
		color: var(--color-ink);
	}

	.sort-arrow {
		margin-left: var(--space-3xs);
		font-size: var(--text-xs);
	}

	.sort-active {
		color: var(--color-ink);
	}
</style>