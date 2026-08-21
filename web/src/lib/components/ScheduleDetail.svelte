<script lang="ts">
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import * as Dialog from '$lib/components/ui/dialog';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import { formatDateTime } from '$lib/tz';
	import { approveSchedule, rejectSchedule, deleteSchedule } from '$lib/api';
	import { toast } from '$lib/toast.svelte';
	import type { Schedule } from '$lib/types';

	interface Props {
		schedule: Schedule;
		onClose: () => void;
		onAction?: () => void;
	}

	let { schedule, onClose, onAction }: Props = $props();

	let showRejectInput = $state(false);
	let rejectReason = $state('');
	let acting = $state(false);
	let showDeleteConfirm = $state(false);

	// Parse media URLs
	let mediaUrls: string[] = $derived(
		schedule.media_urls
			? schedule.media_urls.split(',').map((u) => u.trim()).filter(Boolean)
			: [],
	);

	let statusTimeline = $derived(buildTimeline(schedule));

	function buildTimeline(s: Schedule) {
		const steps: { label: string; date: string | null; done: boolean; error?: boolean }[] = [];
		steps.push({ label: 'Created', date: s.created_at, done: true });
		if (s.status === 'draft') {
			steps.push({ label: 'Awaiting approval', date: null, done: false });
		} else if (s.approved_at) {
			steps.push({ label: 'Approved', date: s.approved_at, done: true });
		}
		if (s.status === 'pending') {
			steps.push({ label: 'Scheduled', date: s.scheduled_at, done: false });
		} else if (s.status === 'processing') {
			steps.push({ label: 'Processing', date: null, done: false });
		} else if (s.status === 'published') {
			steps.push({ label: 'Published', date: s.published_at, done: true });
		} else if (s.status === 'failed') {
			steps.push({ label: 'Failed', date: null, done: true, error: true });
		} else if (s.status === 'rejected') {
			steps.push({ label: 'Rejected', date: s.approved_at, done: true, error: true });
		}
		return steps;
	}

	async function handleApprove() {
		acting = true;
		try {
			await approveSchedule(schedule.id);
			toast('Schedule approved', 'success');
			onAction?.();
			onClose();
		} catch (e: any) {
			toast(e.message || 'Failed to approve', 'error');
		} finally {
			acting = false;
		}
	}

	async function handleReject() {
		if (showRejectInput && rejectReason.trim()) {
			acting = true;
			try {
				await rejectSchedule(schedule.id, rejectReason.trim());
				toast('Schedule rejected', 'success');
				onAction?.();
				onClose();
			} catch (e: any) {
				toast(e.message || 'Failed to reject', 'error');
			} finally {
				acting = false;
			}
		} else {
			showRejectInput = true;
		}
	}

	async function handleDelete() {
		acting = true;
		try {
			await deleteSchedule(schedule.id);
			toast('Schedule deleted', 'success');
			onAction?.();
			onClose();
		} catch (e: any) {
			toast(e.message || 'Failed to delete', 'error');
		} finally {
			acting = false;
			showDeleteConfirm = false;
		}
	}

</script>

<Dialog.Root open onOpenChange={(o) => { if (!o) onClose(); }}>
	<Dialog.Content class="detail-dialog" aria-describedby={undefined}>
		<Dialog.Title class="sr-only">Schedule detail</Dialog.Title>
		<!-- Header -->
		<div class="detail-header">
			<h2 class="detail-title">Schedule Detail</h2>
			<button class="detail-close" onclick={onClose} aria-label="Close">&times;</button>
		</div>

		<!-- Body -->
		<div class="detail-body">
			<!-- Status + scheduled time -->
			<div class="detail-row">
				<span class="detail-label">Status</span>
				<StatusBadge status={schedule.status} />
			</div>
			<div class="detail-row">
				<span class="detail-label">Scheduled at</span>
				<span class="detail-value">{formatDateTime(schedule.scheduled_at)}</span>
			</div>

			<!-- Media type -->
			<div class="detail-row">
				<span class="detail-label">Type</span>
				<span class="detail-value badge-muted">{schedule.media_type}</span>
			</div>

			<!-- Caption -->
			{#if schedule.caption}
				<div class="detail-section">
					<span class="detail-label">Caption</span>
					<p class="detail-caption">{schedule.caption}</p>
				</div>
			{/if}

			<!-- Text attachment -->
			{#if schedule.text_attachment}
				<div class="detail-section">
					<span class="detail-label">Text attachment</span>
					<p class="detail-text-attachment">{schedule.text_attachment}</p>
				</div>
			{/if}

			<!-- Media previews -->
			{#if mediaUrls.length > 0}
				<div class="detail-section">
					<span class="detail-label">Media ({mediaUrls.length})</span>
					<div class="media-grid">
						{#each mediaUrls as url}
							<div class="media-thumb">
								<img src={url} alt="Media preview" loading="lazy" onerror={(e) => { const t = e.currentTarget as HTMLImageElement; t.style.opacity = '0'; t.style.minHeight = '80px'; t.alt = 'Failed to load image'; }} />
							</div>
						{/each}
					</div>
				</div>
			{/if}

			<!-- Status timeline -->
			<div class="detail-section">
				<span class="detail-label">Timeline</span>
				<ol class="timeline">
					{#each statusTimeline as step}
						<li class="timeline-item" class:timeline-error={step.error}>
							<span class="timeline-dot" class:done={step.done}></span>
							<span class="timeline-label">{step.label}</span>
							{#if step.date}
								<span class="timeline-date">{formatDateTime(step.date)}</span>
							{/if}
						</li>
					{/each}
				</ol>
			</div>

			<!-- Approval info -->
			{#if schedule.approved_by}
				<div class="detail-row">
					<span class="detail-label">Approved by</span>
					<span class="detail-value">{schedule.approved_by}</span>
				</div>
			{/if}

			<!-- Published post link -->
			{#if schedule.result_post_id}
				<div class="detail-row">
					<span class="detail-label">Published post</span>
					<span class="detail-value mono">{schedule.result_post_id}</span>
				</div>
			{/if}

			<!-- Error message -->
			{#if schedule.error}
				<div class="detail-error-box">
					<span class="detail-label">Error</span>
					<p class="error-text">{schedule.error}</p>
				</div>
			{/if}

			<!-- Reject reason input -->
			{#if showRejectInput}
				<div class="reject-input">
					<label for="reject-reason">Rejection reason</label>
					<textarea id="reject-reason" bind:value={rejectReason} rows="2" placeholder="Optional reason..."></textarea>
				</div>
			{/if}
		</div>

		<!-- Footer actions -->
		<div class="detail-footer">
			{#if schedule.status === 'draft'}
				<button class="btn-success" onclick={handleApprove} disabled={acting}>
					{acting ? '...' : 'Approve'}
				</button>
				<button class="btn-danger" onclick={handleReject} disabled={acting}>
					{acting ? '...' : showRejectInput ? 'Confirm reject' : 'Reject'}
				</button>
			{/if}
			{#if ['draft', 'rejected', 'failed'].includes(schedule.status)}
				<button class="btn-ghost" onclick={() => (showDeleteConfirm = true)} disabled={acting}>
					Delete
				</button>
			{/if}
			<button class="btn-secondary" onclick={onClose}>Close</button>
		</div>
	</Dialog.Content>
</Dialog.Root>

<ConfirmDialog
	open={showDeleteConfirm}
	title="Delete Schedule"
	message="Are you sure you want to delete this schedule? This action cannot be undone."
	confirmLabel="Delete"
	variant="danger"
	onconfirm={handleDelete}
	oncancel={() => (showDeleteConfirm = false)}
/>

<style>
	.detail-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-sm);
	}

	.detail-label {
		font-size: var(--text-xs);
		color: var(--color-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		font-weight: 600;
	}

	.detail-value {
		font-size: var(--text-sm);
	}

	.badge-muted {
		padding: var(--space-3xs) var(--space-xs);
		background: var(--color-paper-3);
		border-radius: var(--radius-sm);
		font-size: var(--text-xs);
		font-weight: 600;
	}

	.detail-section {
		display: flex;
		flex-direction: column;
		gap: var(--space-xs);
	}

	.detail-caption {
		font-size: var(--text-sm);
		line-height: 1.6;
		margin: 0;
		white-space: pre-wrap;
		word-break: break-word;
	}

	.detail-text-attachment {
		font-size: var(--text-sm);
		line-height: 1.6;
		margin: 0;
		padding: var(--space-sm);
		background: var(--color-paper-2);
		border-radius: var(--radius-sm);
		white-space: pre-wrap;
		word-break: break-word;
	}

	/* Media grid */
	.media-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(6rem, 1fr));
		gap: var(--space-xs);
	}

	.media-thumb {
		aspect-ratio: 1;
		border-radius: var(--radius-sm);
		overflow: hidden;
		border: 1px solid var(--color-rule);
	}

	.media-thumb img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	/* Timeline */
	.timeline {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-sm);
	}

	.timeline-item {
		display: flex;
		align-items: center;
		gap: var(--space-sm);
		position: relative;
	}

	.timeline-item:not(:last-child)::before {
		content: '';
		position: absolute;
		left: 4px;
		top: 1rem;
		bottom: -0.5rem;
		width: 2px;
		background: var(--color-rule);
	}

	.timeline-dot {
		width: 10px;
		height: 10px;
		border-radius: 50%;
		border: 2px solid var(--color-rule);
		background: var(--color-paper);
		flex-shrink: 0;
		z-index: 1;
	}

	.timeline-dot.done {
		background: var(--color-success);
		border-color: var(--color-success);
	}

	.timeline-error .timeline-dot.done {
		background: var(--color-error);
		border-color: var(--color-error);
	}

	.timeline-label {
		font-size: var(--text-sm);
		font-weight: 500;
	}

	.timeline-date {
		font-size: var(--text-xs);
		color: var(--color-muted);
		font-family: var(--font-mono);
		margin-left: auto;
	}

	/* Error box */
	.detail-error-box {
		padding: var(--space-sm);
		background: var(--color-error-dim);
		border: 1px solid var(--color-error);
		border-radius: var(--radius-sm);
		display: flex;
		flex-direction: column;
		gap: var(--space-3xs);
	}

	.error-text {
		font-size: var(--text-xs);
		font-family: var(--font-mono);
		margin: 0;
		color: var(--color-error);
		word-break: break-word;
	}

	/* Reject input */
	.reject-input {
		display: flex;
		flex-direction: column;
		gap: var(--space-2xs);
	}

	.reject-input label {
		font-size: var(--text-xs);
		color: var(--color-muted);
		font-weight: 600;
	}

	.reject-input textarea {
		width: 100%;
		padding: var(--space-xs);
		border: 1px solid var(--color-rule);
		border-radius: var(--radius-sm);
		background: var(--color-paper);
		color: var(--color-ink);
		font-size: var(--text-sm);
		resize: vertical;
	}

	.mono {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
	}
</style>
