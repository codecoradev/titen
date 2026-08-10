<script lang="ts">
	import * as Empty from "$lib/components/ui/empty";
	import FileText from "@lucide/svelte/icons/file-text";
	import Users from "@lucide/svelte/icons/users";
	import Inbox from "@lucide/svelte/icons/inbox";

	interface Props {
		icon?: string;
		title: string;
		description?: string;
		action?: import("svelte").Snippet;
	}

	let { icon, title, description, action }: Props = $props();
</script>

<Empty.Root>
	<Empty.Media>
		{#if icon === "posts"}
			<FileText class="size-8" />
		{:else if icon === "accounts"}
			<Users class="size-8" />
		{:else}
			<Inbox class="size-8" />
		{/if}
	</Empty.Media>
	<Empty.Header>
		<Empty.Title>{title}</Empty.Title>
		{#if description}
			<Empty.Description>{description}</Empty.Description>
		{/if}
	</Empty.Header>
	{#if action}
		<Empty.Content>
			{@render action()}
		</Empty.Content>
	{/if}
</Empty.Root>
