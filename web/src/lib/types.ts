// ── Titen Admin API types ──
// Mirrors actual Rust models in titen-core/src/models.rs
// and safe_account_json in titen-api/src/routes/accounts.rs

// ─── Account (safe JSON from list/get endpoints) ───
export interface Account {
	id: string;
	username: string;
	user_id: string;
	is_active: boolean;
	expires_at: string | null;
	token_status: 'valid' | 'expired' | 'unknown';
	created_at: string;
}

// ─── Post ───
export interface Post {
	id: string;
	threads_post_id: string | null;
	account_id: string;
	media_type: string;
	caption: string | null;
	text_attachment: string | null;
	carousel_children: string | null;
	status: string;
	scheduled_id: string | null;
	published_at: string | null;
	insights_json: string | null;
	created_at: string;
	updated_at: string;
	// Joined (may be populated by backend)
	account?: Account;
}

// ─── Schedule ───
export interface Schedule {
	id: string;
	account_id: string;
	media_type: string;
	caption: string | null;
	text_attachment: string | null;
	media_urls: string | null;
	scheduled_at: string;
	status: string;
	published_at: string | null;
	result_post_id: string | null;
	result_json: string | null;
	error: string | null;
	approved_by: string | null;
	approved_at: string | null;
	created_at: string;
	updated_at: string;
}

// ─── Comment ───
export interface Comment {
	id: string;
	post_id: string;
	threads_comment_id: string | null;
	author_username: string | null;
	author_user_id: string | null;
	text: string;
	sentiment: string | null;
	sentiment_score: number | null;
	reply_status: string; // new | needs_reply | replied | skipped
	replied_at: string | null;
	reply_text: string | null;
	assigned_priority: number;
	fetched_at: string;
}

// ─── Insights (per-post metric snapshot) ───
export interface Insights {
	likes: number | null;
	replies: number | null;
	reposts: number | null;
	views: number | null;
	quotes: number | null;
	shares: number | null;
}

// ─── Media Asset ───
export interface MediaItem {
	id: string;
	filename: string;
	content_type: string;
	size_bytes: number;
	s3_key: string;
	s3_url: string | null;
	uploaded_at: string;
}

// ─── Analytics Snap ───
export interface AnalyticsSnap {
	id: string;
	account_id: string;
	period: string;
	total_posts: number;
	total_likes: number;
	total_replies: number;
	total_reposts: number;
	total_views: number;
	engagement_rate: number;
	snapshot_at: string;
}

// ─── Analytics Trend (per-post) ───
export interface AnalyticsTrend {
	post_id: string;
	date: string;
	likes: number;
	replies: number;
	reposts: number;
	views: number;
}

// ─── Sentiment Summary ───
export interface SentimentSummary {
	total: number;
	positive: number;
	negative: number;
	neutral: number;
	average_score: number;
}

// ─── Health ───
export interface HealthResponse {
	status: string;
	version: string;
	db: string;
	timezone: string;
}

// ─── Mention (from Threads mentions API) ───
export interface Mention {
	id: string;
	text: string;
	username?: string;
	timestamp?: string;
	permalink?: string;
	media_url?: string;
}

// ─── Account Insights (aggregate metrics) ───
export interface AccountInsights {
	[key: string]: number | string | null;
}

// ─── Threads Profile (from Meta API via /accounts/{id}/profile) ───
export interface ThreadsProfile {
	id: string;
	username: string;
	name?: string;
	threads_profile_picture_url?: string;
	threads_biography?: string;
	followers_count?: number;
	following_count?: number;
	media_count?: number;
}

// ─── Dashboard Summary (computed client-side from list endpoints) ───
export interface DashboardSummary {
	total_accounts: number;
	active_accounts: number;
	total_posts: number;
	published_posts: number;
	pending_schedules: number;
	total_comments: number;
	positive_sentiment_pct: number;
}
