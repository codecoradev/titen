// ── Titen Admin API types ──
// Mirrors DESIGN.md ERD

export interface Account {
	id: string;
	threads_user_id: string;
	username: string;
	display_name: string;
	profile_pic_url: string | null;
	platform: string;
	status: 'active' | 'suspended' | 'expired';
	token_expires_at: string | null;
	created_at: string;
	updated_at: string;
}

export interface Post {
	id: string;
	account_id: string;
	threads_post_id: string | null;
	media_type: 'TEXT' | 'IMAGE' | 'CAROUSEL' | 'VIDEO';
	caption: string;
	text_attachment: string | null;
	image_url: string | null;
	image_urls: string[] | null;
	alt_text: string | null;
	status: 'draft' | 'published' | 'failed' | 'deleted';
	threads_api_error: string | null;
	created_at: string;
	published_at: string | null;
	// Joined
	account?: Account;
	insights?: Insight[];
}

export interface Schedule {
	id: string;
	account_id: string;
	post_data: Record<string, unknown>;
	media_type: string;
	caption: string;
	scheduled_at: string;
	status: 'pending' | 'processing' | 'completed' | 'failed' | 'cancelled';
	post_id: string | null;
	error_message: string | null;
	created_at: string;
	// Joined
	account?: Account;
}

export interface Comment {
	id: string;
	post_id: string;
	threads_comment_id: string;
	author_username: string;
	text: string;
	sentiment: 'positive' | 'negative' | 'neutral';
	sentiment_score: number | null;
	fetched_at: string;
}

export interface Insight {
	id: string;
	post_id: string;
	likes: number;
	replies: number;
	reposts: number;
	views: number;
	quotes: number;
	snapshot_at: string;
}

export interface MediaItem {
	id: string;
	s3_key: string;
	s3_url: string;
	filename: string;
	content_type: string;
	size_bytes: number;
	uploaded_at: string;
}

export interface AnalyticsSummary {
	account_id: string;
	period: string;
	total_posts: number;
	total_likes: number;
	total_replies: number;
	total_reposts: number;
	total_views: number;
	engagement_rate: number;
}

export interface AnalyticsTrend {
	date: string;
	likes: number;
	replies: number;
	reposts: number;
	views: number;
}

export interface ThreadsProfile {
	username: string;
	bio: string | null;
	profile_pic_url: string | null;
	follower_count: number;
	following_count: number;
	verified: boolean;
}

export interface PublishingLimit {
	quota_remaining: number;
	quota_total: number;
	window_start: string;
	window_end: string;
}

export interface HealthCheck {
	status: 'healthy' | 'degraded' | 'unhealthy';
	version: string;
	uptime: number;
	db_size: number;
	account_count: number;
	post_count: number;
	schedule_count: number;
}

// API response wrappers
export interface ApiResponse<T> {
	data: T;
	message?: string;
}

export interface PaginatedResponse<T> {
	data: T[];
	total: number;
	page: number;
	per_page: number;
}
