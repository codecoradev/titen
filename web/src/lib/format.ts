// ── String formatting utilities ──
// Shared across admin pages to prevent duplication.

/**
 * Truncate text to max characters, appending an ellipsis if cut.
 */
export function truncate(text: string, max: number = 60): string {
	if (text.length <= max) return text;
	return text.slice(0, max).trimEnd() + '\u2026';
}
