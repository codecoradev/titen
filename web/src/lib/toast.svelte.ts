// ── Simple toast store (Svelte 5) ──
// Used across admin pages for operation feedback.

export type ToastType = 'success' | 'error' | 'info';

interface Toast {
	id: number;
	message: string;
	type: ToastType;
}

let nextId = 0;
let toasts = $state<Toast[]>([]);

function add(message: string, type: ToastType = 'info') {
	const id = nextId++;
	toasts = [...toasts, { id, message, type }];
	setTimeout(() => {
		toasts = toasts.filter((t) => t.id !== id);
	}, 4000);
}

export function toast(message: string, type?: ToastType) {
	add(message, type);
}

export function getToasts() {
	return { get toasts() { return toasts; } };
}
