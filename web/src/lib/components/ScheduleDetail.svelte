<script lang="ts">
	import StatusBadge from '$lib/components/StatusBadge.svelte';
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
		if (!confirm('Delete this schedule? This cannot be undone.')) return;
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
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') onClose();
	}
</script>

<svelte:window on:keydown={handleKeydown} />

<div class="modal-overlay" onclick={onClose} role="presentation">
	<div
		class="modal-content"
		role="dialog"
		aria-modal="true"
		aria-label="Schedule detail"
		onclick={(e) => e.stopPropagation()}
	>
		<!-- Header -->
		<div class="modal-header">
			<h2 class="modal-title">Schedule Detail</h2>
			<button class="close-btn" onclick={onClose} aria-label="Close">&times;</button>
		</div>

		<!-- Body -->
		<div class="modal-body">
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
								<img src={url} alt="Media preview" loading="lazy" />
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
		<div class="modal-footer">
			{#if schedule.status === 'draft'}
				<button class="btn btn-success" onclick={handleApprove} disabled={acting}>
					{acting ? '...' : 'Approve'}
				</button>
				<button class="btn btn-danger" onclick={handleReject} disabled={acting}>
					{acting ? '...' : showRejectInput ? 'Confirm reject' : 'Reject'}
				</button>
			{/if}
			<button class="btn btn-ghost" onclick={handleDelete} disabled={acting}>
				Delete
			</button>
			<button class="btn btn-secondary" onclick={onClose}>Close</button>
		</div>
	</div>
</div>

<style>
	.modal-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 50;
		padding: var(--space-md);
	}

	.modal-content {
		background: var(--color-bg);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		max-width: 42rem;
		width: 100%;
		max-height: 85vh;
		display: flex;
		flex-direction: column;
	}

	.modal-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: var(--space-md);
		border-bottom: 1px solid var(--color-border);
	}

	.modal-title {
		font-size: var(--text-lg);
		font-weight: 700;
		margin: 0;
	}

	.close-btn {
		background: none;
		border: none;
		font-size: 1.5rem;
		cursor: pointer;
		color: var(--color-muted);
		padding: 0 var(--space-xs);
		line-height: 1;
	}

	.close-btn:hover {
		color: var(--color-text);
	}

	.modal-body {
		padding: var(--space-md);
		overflow-y: auto;
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: var(--space-sm);
	}

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
		background: var(--color-bg-hover);
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
		background: var(--color-bg-hover);
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
		border: 1px solid var(--color-border);
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
		background: var(--color-border);
	}

	.timeline-dot {
		width: 10px;
		height: 10px;
		border-radius: 50%;
		border: 2px solid var(--color-border);
		background: var(--color-bg);
		flex-shrink: 0;
		z-index: 1;
	}

	.timeline-dot.done {
		background: var(--color-success, #22c55e);
		border-color: var(--color-success, #22c55e);
	}

	.timeline-error .timeline-dot.done {
		background: var(--color-danger, #ef4444);
		border-color: var(--color-danger, #ef4444);
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
		background: var(--color-danger-bg, rgba(239, 68, 68, 0.1));
		border: 1px solid var(--color-danger-border, rgba(239, 68, 68, 0.3));
		border-radius: var(--radius-sm);
		display: flex;
		flex-direction: column;
		gap: var(--space-3xs);
	}

	.error-text {
		font-size: var(--text-xs);
		font-family: var(--font-mono);
		margin: 0;
		color: var(--color-danger, #ef4444);
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
		border: 1px solid var(--color-border);
		border-radius: var(--radius-sm);
		background: var(--color-bg);
		color: var(--color-text);
		font-size: var(--text-sm);
		resize: vertical;
	}

	/* Footer */
	.modal-footer {
		display: flex;
		gap: var(--space-xs);
		padding: var(--space-md);
		border-top: 1px solid var(--color-border);
		justify-content: flex-end;
		flex-wrap: wrap;
	}

	.btn {
		padding: var(--space-xs) var(--space-md);
		border-radius: var(--radius-sm);
		font-size: var(--text-sm);
		font-weight: 600;
		cursor: pointer;
		border: 1px solid transparent;
		transition: opacity 0.15s ease;
	}

	.btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.btn-success {
		background: var(--color-success, #22c55e);
		color: white;
	}

	.btn-danger {
		background: var(--color-danger, #ef4444);
		color: white;
	}

	.btn-secondary {
		background: var(--color-bg-hover);
		color: var(--color-text);
		border-color: var(--color-border);
	}

	.btn-ghost {
		background: transparent;
		color: var(--color-danger, #ef4444);
	}

	.mono {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
	}

	@media (max-width: 30rem) {
		.modal-overlay {
			padding: 0;
		}

		.modal-content {
			max-height: 100vh;
			border-radius: 0;
			height: 100vh;
		}
	}
</style>
