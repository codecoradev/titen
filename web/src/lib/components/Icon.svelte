<script lang="ts">
	// Icon component — renders SVG without {@html} directive.
	// Replaces the icons.ts + {@html getIcon()} pattern for XSS safety.

	interface Props {
		name: string;
		size?: number;
	}

	let { name, size = 20 }: Props = $props();
</script>

{#snippet dashboard()}
	<rect x="3" y="3" width="7" height="7" rx="1" />
	<rect x="14" y="3" width="7" height="7" rx="1" />
	<rect x="3" y="14" width="7" height="7" rx="1" />
	<rect x="14" y="14" width="7" height="7" rx="1" />
{/snippet}

{#snippet accounts()}
	<path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2" />
	<circle cx="9" cy="7" r="4" />
	<path d="M22 21v-2a4 4 0 0 0-3-3.87" />
	<path d="M16 3.13a4 4 0 0 1 0 7.75" />
{/snippet}

{#snippet posts()}
	<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
	<polyline points="14 2 14 8 20 8" />
	<line x1="16" y1="13" x2="8" y2="13" />
	<line x1="16" y1="17" x2="8" y2="17" />
	<polyline points="10 9 9 9 8 9" />
{/snippet}

{#snippet schedules()}
	<circle cx="12" cy="12" r="10" />
	<polyline points="12 6 12 12 16 14" />
{/snippet}

{#snippet comments()}
	<path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
{/snippet}

{#snippet analytics()}
	<line x1="18" y1="20" x2="18" y2="10" />
	<line x1="12" y1="20" x2="12" y2="4" />
	<line x1="6" y1="20" x2="6" y2="14" />
{/snippet}

{#snippet media()}
	<rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
	<circle cx="8.5" cy="8.5" r="1.5" />
	<polyline points="21 15 16 10 5 21" />
{/snippet}

{#snippet settings()}
	<circle cx="12" cy="12" r="3" />
	<path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
{/snippet}

{#snippet mentions()}
	<path d="M21 11.5a8.38 8.38 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.38 8.38 0 0 1-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.38 8.38 0 0 1 3.8-.9h.5a8.48 8.48 0 0 1 8 8v.5z" />
{/snippet}

<svg
	xmlns="http://www.w3.org/2000/svg"
	width={size}
	height={size}
	viewBox="0 0 24 24"
	fill="none"
	stroke="currentColor"
	stroke-width="2"
	stroke-linecap="round"
	stroke-linejoin="round"
>
	{#if name === 'dashboard'} {@render dashboard()}
	{:else if name === 'accounts'} {@render accounts()}
	{:else if name === 'posts'} {@render posts()}
	{:else if name === 'schedules'} {@render schedules()}
	{:else if name === 'comments'} {@render comments()}
	{:else if name === 'analytics'} {@render analytics()}
	{:else if name === 'media'} {@render media()}
	{:else if name === 'settings'} {@render settings()}
	{:else if name === 'mentions'} {@render mentions()}
	{:else} {@render dashboard()}
	{/if}
</svg>
