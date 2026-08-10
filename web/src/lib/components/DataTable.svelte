<script lang="ts">
	import * as Table from "$lib/components/ui/table";
	import Skeleton from "$lib/components/ui/skeleton/skeleton.svelte";

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

	let { columns, rows, loading = false, emptyTitle = "No data", emptyDesc }: Props = $props();

	let sortKey = $state<string | null>(null);
	let sortAsc = $state(true);

	const sorted = $derived.by(() => {
		if (!sortKey) return rows;
		return [...rows].sort((a, b) => {
			const av = a[sortKey!];
			const bv = b[sortKey!];
			if (av == null) return 1;
			if (bv == null) return -1;
			if (typeof av === "number" && typeof bv === "number") return sortAsc ? av - bv : bv - av;
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
		<Table.Root>
			<Table.Header>
				<Table.Row>
					{#each columns as col}
						<Table.Head>{col.label}</Table.Head>
					{/each}
				</Table.Row>
			</Table.Header>
			<Table.Body>
				{#each Array(5) as _}
					<Table.Row>
						{#each columns as _}
							<Table.Cell>
								<Skeleton class="h-4 w-full" />
							</Table.Cell>
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
					{#each columns as col}
						<Table.Head
							class={col.sortable ? "cursor-pointer select-none" : ""}
							onclick={() => col.sortable && toggleSort(col.key)}
						>
							{col.label}
							{#if col.sortable && sortKey === col.key}
								<span class="ml-1 text-xs">{sortAsc ? "↑" : "↓"}</span>
							{/if}
						</Table.Head>
					{/each}
				</Table.Row>
			</Table.Header>
			<Table.Body>
				{#each sorted as row (getRowKey(row))}
					<Table.Row>
						{#each columns as col}
							<Table.Cell class={col.class}>
								{row[col.key] ?? "—"}
							</Table.Cell>
						{/each}
					</Table.Row>
				{/each}
			</Table.Body>
		</Table.Root>
	{/if}
</div>
