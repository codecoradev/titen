// ── Titen Admin API client ──
// All endpoints hit /api/* — no auth header needed for same-origin

const BASE = '/api';

async function request<T>(path: string, init?: RequestInit): Promise<T> {
	const res = await fetch(`${BASE}${path}`, {
		headers: { 'Content-Type': 'application/json', ...init?.headers },
		...init,
	});
	if (!res.ok) {
		const body = await res.text().catch(() => '');
		throw new ApiError(res.status, res.statusText, body);
	}
	return res.json();
}

export class ApiError extends Error {
	constructor(
		public status: number,
		public statusText: string,
		public body: string,
	) {
		super(`API ${status}: ${statusText}`);
	}
}

// ── Health ──
export const getHealth = () => request<import('./types').HealthCheck>('/system/health');

// ── Accounts ──
export const listAccounts = () =>
	request<import('./types').ApiResponse<import('./types').Account[]>>('/accounts');

export const createAccount = (data: { threads_user_id: string; access_token: string; refresh_token: string }) =>
	request<import('./types').ApiResponse<import('./types').Account>>('/accounts', {
		method: 'POST',
		body: JSON.stringify(data),
	});

export const getAccount = (id: string) =>
	request<import('./types').ApiResponse<import('./types').Account>>(`/accounts/${id}`);

export const deleteAccount = (id: string) =>
	request<void>(`/accounts/${id}`, { method: 'DELETE' });

export const refreshToken = (id: string) =>
	request<import('./types').ApiResponse<{ token_expires_at: string }>>(`/accounts/${id}/refresh-token`, { method: 'POST' });

export const checkToken = (id: string) =>
	request<import('./types').ApiResponse<{ valid: boolean; expires_at: string | null }>>(`/accounts/${id}/check-token`);

// ── Posts ──
export const listPosts = (params?: { account_id?: string; status?: string; page?: number; per_page?: number }) => {
	const q = new URLSearchParams();
	if (params?.account_id) q.set('account_id', params.account_id);
	if (params?.status) q.set('status', params.status);
	if (params?.page) q.set('page', String(params.page));
	if (params?.per_page) q.set('per_page', String(params.per_page));
	const qs = q.toString();
	return request<import('./types').ApiResponse<import('./types').Post[]>>(`/posts${qs ? `?${qs}` : ''}`);
};

export const createPost = (data: { account_id: string; caption: string; media_type?: string; image_url?: string }) =>
	request<import('./types').ApiResponse<import('./types').Post>>('/posts', {
		method: 'POST',
		body: JSON.stringify(data),
	});

export const getPost = (id: string) =>
	request<import('./types').ApiResponse<import('./types').Post>>(`/posts/${id}`);

export const deletePost = (id: string) =>
	request<void>(`/posts/${id}`, { method: 'DELETE' });

export const getPostInsights = (id: string) =>
	request<import('./types').ApiResponse<import('./types').Insight[]>>(`/posts/${id}/insights`);

// ── Schedules ──
export const listSchedules = (params?: { account_id?: string; status?: string }) => {
	const q = new URLSearchParams();
	if (params?.account_id) q.set('account_id', params.account_id);
	if (params?.status) q.set('status', params.status);
	const qs = q.toString();
	return request<import('./types').ApiResponse<import('./types').Schedule[]>>(`/schedules${qs ? `?${qs}` : ''}`);
};

export const createSchedule = (data: { account_id: string; scheduled_at: string; post_data: Record<string, unknown> }) =>
	request<import('./types').ApiResponse<import('./types').Schedule>>('/schedules', {
		method: 'POST',
		body: JSON.stringify(data),
	});

export const updateSchedule = (id: string, data: { scheduled_at?: string; status?: string }) =>
	request<import('./types').ApiResponse<import('./types').Schedule>>(`/schedules/${id}`, {
		method: 'PUT',
		body: JSON.stringify(data),
	});

export const deleteSchedule = (id: string) =>
	request<void>(`/schedules/${id}`, { method: 'DELETE' });

export const getUpcomingSchedules = () =>
	request<import('./types').ApiResponse<import('./types').Schedule[]>>('/schedules/upcoming');

// ── Comments ──
export const listComments = (post_id?: string) => {
	const q = post_id ? `?post_id=${post_id}` : '';
	return request<import('./types').ApiResponse<import('./types').Comment[]>>(`/comments${q}`);
};

export const fetchComments = (post_id: string) =>
	request<import('./types').ApiResponse<import('./types').Comment[]>>(`/comments/fetch`, {
		method: 'POST',
		body: JSON.stringify({ post_id }),
	});

// ── Analytics ──
export const listAnalytics = (params?: { account_id?: string; period?: string }) => {
	const q = new URLSearchParams();
	if (params?.account_id) q.set('account_id', params.account_id);
	if (params?.period) q.set('period', params.period);
	const qs = q.toString();
	return request<import('./types').ApiResponse<import('./types').AnalyticsSummary[]>>(`/analytics${qs ? `?${qs}` : ''}`);
};

export const getAnalyticsTrend = (account_id: string, period?: string) => {
	const q = new URLSearchParams({ account_id });
	if (period) q.set('period', period);
	return request<import('./types').ApiResponse<import('./types').AnalyticsTrend[]>>(`/analytics/trend?${q}`);
};

// ── Media ──
export const listMedia = () =>
	request<import('./types').ApiResponse<import('./types').MediaItem[]>>('/media');

export const uploadMedia = (file: File) => {
	const form = new FormData();
	form.append('file', file);
	return request<import('./types').ApiResponse<import('./types').MediaItem>>('/media/upload', {
		method: 'POST',
		body: form,
		headers: {}, // let browser set multipart boundary
	});
};

export const deleteMedia = (id: string) =>
	request<void>(`/media/${id}`, { method: 'DELETE' });

// ── Threads ──
export const getThreadsContainer = (account_id: string) =>
	request<unknown>(`/threads/container?account_id=${account_id}`);

export const getThreadsProfile = (account_id: string) =>
	request<import('./types').ApiResponse<import('./types').ThreadsProfile>>(`/threads/profile?account_id=${account_id}`);

export const getPublishingLimit = (account_id: string) =>
	request<import('./types').ApiResponse<import('./types').PublishingLimit>>(`/threads/publishing-limit?account_id=${account_id}`);
