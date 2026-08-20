// ── Titen Admin API client ──
// Matches titen-api backend routes (Axum)
// Dev: Vite proxies /api/* → localhost:7845
// Prod: TITEN_API_BASE env var (defaults to same origin)
//
// Auth: httpOnly cookie set by POST /api/auth/login.
// Cookie auto-attaches via `credentials: 'same-origin'`.

import type {
	Account, Post, Schedule, Comment, Insights,
	MediaItem, AnalyticsSnap, AnalyticsTrend,
	SentimentSummary, HealthResponse,
	Mention, AccountInsights, ThreadsProfile,
} from './types';

const BASE = import.meta.env.TITEN_API_BASE || '/api';

async function request<T>(path: string, init?: RequestInit): Promise<T> {
	const isForm = init?.body instanceof FormData;
	const res = await fetch(`${BASE}${path}`, {
		...init,
		credentials: 'same-origin',
		headers: isForm
			? { ...(init?.headers as Record<string, string>) }
			: { 'Content-Type': 'application/json', ...(init?.headers as Record<string, string>) },
	});

	if (!res.ok) {
		const body = await res.text().catch(() => '');
		let msg = `API ${res.status}`;
		try {
			const parsed = JSON.parse(body);
			msg = parsed.error || parsed.message || msg;
		} catch {
			if (body) msg = body;
		}
		throw new ApiError(res.status, res.statusText, msg);
	}
	const json = await res.json().catch(() => ({}));
	// Backend wraps all responses in { "data": ... }
	return (json.data !== undefined ? json.data : json) as T;
}

export class ApiError extends Error {
	constructor(
		public status: number,
		public statusText: string,
		public body: string,
	) {
		super(body || `API ${status}: ${statusText}`);
	}
}

// ── Health ──
export const getHealth = (): Promise<HealthResponse> =>
	request<HealthResponse>('/health');

// ── Accounts ──
export const listAccounts = (): Promise<Account[]> =>
	request<Account[]>('/accounts');

export const createAccount = (data: {
	username?: string;
	user_id?: string;
	access_token: string;
	expires_at: string;
	app_id?: string;
	app_secret?: string;
}): Promise<Account> =>
	request<Account>('/accounts', {
		method: 'POST',
		body: JSON.stringify(data),
	});

export const getAccount = (id: string): Promise<Account> =>
	request<Account>(`/accounts/${id}`);

export const updateAccount = (id: string, data: Record<string, unknown>): Promise<Account> =>
	request<Account>(`/accounts/${id}`, {
		method: 'PUT',
		body: JSON.stringify(data),
	});

export const deleteAccount = (id: string): Promise<void> =>
	request<void>(`/accounts/${id}`, { method: 'DELETE' });

export const refreshToken = (id: string): Promise<Account> =>
	request<Account>(`/accounts/${id}/refresh-token`, {
		method: 'POST',
	});

export const checkAllTokens = (): Promise<Account[]> =>
	request<Account[]>('/accounts/check-tokens');

export const getThreadsProfile = (accountId: string): Promise<ThreadsProfile> =>
	request<{ data: ThreadsProfile } & ThreadsProfile>(`/accounts/${accountId}/profile`).then(
		(r) => r.data ?? r
	);

export const getPublishingLimit = (accountId: string): Promise<unknown> =>
	request<unknown>(`/accounts/${accountId}/publishing-limit`);

// ── Posts ──
export const listPosts = (params?: {
	account_id?: string;
	status?: string;
	limit?: number;
	offset?: number;
}): Promise<Post[]> => {
	const q = new URLSearchParams();
	if (params?.account_id) q.set('account_id', params.account_id);
	if (params?.status) q.set('status', params.status);
	if (params?.limit) q.set('limit', String(params.limit));
	if (params?.offset) q.set('offset', String(params.offset));
	const qs = q.toString();
	return request<Post[]>(`/posts${qs ? `?${qs}` : ''}`);
};

export const createPost = (data: {
	account_id: string;
	caption?: string;
	media_type?: string;
	image_url?: string;
	image_urls?: string[];
	media_ids?: string[];
	video_url?: string;
	text_attachment?: string;
	alt_text?: string;
}): Promise<Post> =>
	request<Post>('/posts', {
		method: 'POST',
		body: JSON.stringify(data),
	});

export const getPost = (id: string): Promise<Post> =>
	request<Post>(`/posts/${id}`);

export const deletePost = (id: string): Promise<void> =>
	request<void>(`/posts/${id}`, { method: 'DELETE' });

export const getPostInsights = (id: string): Promise<Insights> =>
	request<Insights>(`/posts/${id}/insights`);

// ── Schedules ──
export const getSchedule = (id: string): Promise<Schedule> =>
	request<Schedule>(`/schedules/${id}`);

export const listSchedules = (params?: {
	account_id?: string;
	status?: string;
	from?: string;
	to?: string;
	search?: string;
	media_type?: string;
}): Promise<Schedule[]> => {
	const q = new URLSearchParams();
	if (params?.account_id) q.set('account_id', params.account_id);
	if (params?.status) q.set('status', params.status);
	if (params?.from) q.set('from', params.from);
	if (params?.to) q.set('to', params.to);
	if (params?.search) q.set('search', params.search);
	if (params?.media_type) q.set('media_type', params.media_type);
	const qs = q.toString();
	return request<Schedule[]>(`/schedules${qs ? `?${qs}` : ''}`);
};

export const createSchedule = (data: {
	account_id: string;
	scheduled_at: string;
	media_type?: string;
	caption?: string;
	text_attachment?: string;
	media_urls?: string;
}): Promise<Schedule> =>
	request<Schedule>('/schedules', {
		method: 'POST',
		body: JSON.stringify(data),
	});

export const updateSchedule = (id: string, data: {
	scheduled_at?: string;
	status?: string;
}): Promise<Schedule> =>
	request<Schedule>(`/schedules/${id}`, {
		method: 'PUT',
		body: JSON.stringify(data),
	});

export const deleteSchedule = (id: string): Promise<void> =>
	request<void>(`/schedules/${id}`, { method: 'DELETE' });

export const patchSchedule = (id: string, data: {
	caption?: string;
	media_type?: string;
	media_urls?: string[];
	scheduled_at?: string;
}): Promise<Schedule> =>
	request<Schedule>(`/schedules/${id}`, {
		method: 'PATCH',
		body: JSON.stringify(data),
	});

export const approveSchedule = (id: string): Promise<Schedule> =>
	request<Schedule>(`/schedules/${id}/approve`, { method: 'POST' });

export const rejectSchedule = (id: string, reason?: string): Promise<Schedule> =>
	request<Schedule>(`/schedules/${id}/reject`, {
		method: 'POST',
		body: JSON.stringify(reason ? { reason } : {}),
	});

export const getUpcomingSchedules = (): Promise<Schedule[]> =>
	request<Schedule[]>('/schedules/upcoming');

// ── Comments (nested under posts in backend) ──
export const listComments = (postId: string, params?: {
	sentiment?: string;
	reply_status?: string;
	search?: string;
	limit?: number;
	offset?: number;
}): Promise<Comment[]> => {
	const qs = params ? '?' + new URLSearchParams(
		Object.entries(params).filter(([, v]) => v != null)
			.map(([k, v]) => [k, String(v)])
	).toString() : '';
	return request<Comment[]>(`/posts/${postId}/comments${qs}`);
};

export const fetchComments = (postId: string): Promise<Comment[]> =>
	request<Comment[]>(`/posts/${postId}/comments/fetch`, {
		method: 'POST',
	});

export const getCommentSentiment = (postId: string): Promise<SentimentSummary> =>
	request<SentimentSummary>(`/posts/${postId}/comments/sentiment`);

// ── Comment reply workflow ──
export const updateCommentReply = (commentId: string, body: {
	reply_status?: string;
	reply_text?: string;
}): Promise<Comment> =>
	request<Comment>(`/comments/${commentId}`, {
		method: 'PATCH',
		body: JSON.stringify(body),
	});

export const replyToComment = (commentId: string, replyText: string): Promise<{
	data: Comment;
	threads_reply_id: string;
}> =>
	request(`/comments/${commentId}/reply`, {
		method: 'POST',
		body: JSON.stringify({ reply_text: replyText }),
	});

// ── Analytics ──
export const listAnalytics = (params?: {
	account_id?: string;
	period?: string;
}): Promise<AnalyticsSnap[]> => {
	const q = new URLSearchParams();
	if (params?.account_id) q.set('account_id', params.account_id);
	if (params?.period) q.set('period', params.period);
	const qs = q.toString();
	return request<AnalyticsSnap[]>(`/analytics/posts${qs ? `?${qs}` : ''}`);
};

export const getAnalyticsTrend = (postId: string, period?: string): Promise<AnalyticsTrend[]> => {
	const q = new URLSearchParams();
	if (period) q.set('period', period);
	const qs = q.toString();
	return request<AnalyticsTrend[]>(`/analytics/posts/${postId}/trend${qs ? `?${qs}` : ''}`);
};

// ── Media ──
export interface MediaPagination {
	total: number;
	limit: number;
	offset: number;
	has_more: boolean;
}

export interface MediaListResponse {
	data: MediaItem[];
	pagination: MediaPagination;
}

export const listMedia = (params?: {
	limit?: number;
	offset?: number;
	search?: string;
}): Promise<MediaListResponse> => {
	const q = new URLSearchParams();
	if (params?.limit !== undefined) q.set('limit', String(params.limit));
	if (params?.offset !== undefined) q.set('offset', String(params.offset));
	if (params?.search) q.set('search', params.search);
	const qs = q.toString();
	// Backend returns { data, pagination } — extract .data for backward compat
	return request<MediaListResponse>(`/media${qs ? `?${qs}` : ''}`);
};

export const uploadMedia = (file: File): Promise<MediaItem> => {
	const form = new FormData();
	form.append('file', file);
	return request<MediaItem>('/media', {
		method: 'POST',
		body: form,
	});
};

export const deleteMedia = (id: string): Promise<void> =>
	request<void>(`/media/${id}`, { method: 'DELETE' });

// ── Threads proxy ──
export const checkTokens = (): Promise<unknown> =>
	request<unknown>('/accounts/check-tokens');

// ── Threads: Mentions & Reply ──
export const fetchMentions = (accountId: string, limit?: number): Promise<Mention[]> =>
	request<{ data: Mention[]; fetched: number; stored: number; failed: number } & Mention[]>(
		'/threads/mentions',
		{
			method: 'POST',
			body: JSON.stringify({ account_id: accountId, limit: limit ?? 25 }),
		}
	).then((r) => r.data ?? r);

export const createReply = (data: {
	account_id: string;
	reply_to_id: string;
	text: string;
}): Promise<unknown> =>
	request<unknown>('/threads/reply', {
		method: 'POST',
		body: JSON.stringify(data),
	});

// ── Account Insights (aggregate) ──
export const getAccountInsights = (accountId: string, params?: {
	metrics?: string;
	since?: number;
	until?: number;
}): Promise<AccountInsights> => {
	const q = new URLSearchParams();
	if (params?.metrics) q.set('metrics', params.metrics);
	if (params?.since) q.set('since', String(params.since));
	if (params?.until) q.set('until', String(params.until));
	const qs = q.toString();
	return request<AccountInsights>(`/accounts/${accountId}/insights${qs ? `?${qs}` : ''}`);
};

// ── OAuth ──
export const oauthExchange = (data: {
	code: string;
	redirect_uri: string;
	app_id?: string;
	app_secret?: string;
}): Promise<Account> =>
	request<Account>('/oauth/exchange', {
		method: 'POST',
		body: JSON.stringify(data),
	});

// ── Auth (session/cookie-based) ──
export async function loginWithApiKey(apiKey: string): Promise<{ valid: boolean }> {
	return await request<{ valid: boolean }>('/auth/login', {
		method: 'POST',
		body: JSON.stringify({ api_key: apiKey }),
	});
}

export async function checkSession(): Promise<{ requires_auth: boolean; authenticated: boolean; version?: string }> {
	try {
		return await request<{ requires_auth: boolean; authenticated: boolean; version: string }>('/auth/session');
	} catch {
		// Backend unreachable — treat as unauthenticated (safe default)
		return { requires_auth: true, authenticated: false };
	}
}

export async function logout(): Promise<void> {
	try {
		await request<void>('/auth/logout', { method: 'POST' });
	} catch {
		// ignore — cookie may already be expired
	}
}

// ── Settings ──
export interface AppSettings {
	instance_name: string;
	auto_fetch_comments: boolean;
	comment_fetch_interval: string;
	schedule_lookahead_hours: string;
	threads_app_id: string | null;
	threads_app_secret_set: boolean;
}

export interface OAuthConfig {
	app_id: string | null;
	redirect_uri: string;
	authorize_url: string | null;
	secret_configured: boolean;
}

export async function getSettings(): Promise<AppSettings> {
	return await request<AppSettings>('/settings');
}

export async function updateSettings(data: Partial<{
	instance_name: string;
	auto_fetch_comments: boolean;
	comment_fetch_interval: string;
	schedule_lookahead_hours: string;
	threads_app_id: string;
	threads_app_secret: string;
}>): Promise<AppSettings> {
	return await request<AppSettings>('/settings', {
		method: 'PUT',
		body: JSON.stringify(data),
	});
}

export async function getOAuthConfig(): Promise<OAuthConfig> {
	return await request<OAuthConfig>('/settings/oauth-config');
}
