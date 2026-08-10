use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
use utoipa::{Modify, OpenApi};
use utoipa_swagger_ui::SwaggerUi;

use titen_core::models::{
    Account, AnalyticsSnap, Comment, CommentData, CommentFilter, ContainerStatus, CreateAccount,
    CreatePost, CreateReply, CreateSchedule, InsightMetric, InsightTotalValue, InsightValue,
    Insights, LinkTotalValue, MediaAsset, MediaFilter, Mention, MentionFilter, Post, PostFilter,
    PublishingLimit, PublishingLimitConfig, RateLimits, Schedule, ScheduleFilter, SentimentResult,
    SentimentSummary, UpdateAccount, UpdateCommentReply, UpdateSchedule, UserInsightMetric,
    UserProfile,
};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Titen API",
        version = "0.5.3",
        description = "Self-hosted Threads management platform — schedule posts, manage comments, track analytics.\n\nAuthentication: all `/api/*` routes (except `/api/auth/*`) require an `X-API-Key` header.",
        license(name = "Apache-2.0", url = "https://www.apache.org/licenses/LICENSE-2.0"),
    ),
    servers(
        (url = "/", description = "Local instance"),
    ),
    security(("api_key" = [])),
    components(schemas(
        // Core entities
        Account, CreateAccount, UpdateAccount,
        Post, CreatePost,
        Schedule, CreateSchedule, UpdateSchedule,
        Comment, UpdateCommentReply, CommentData,
        Mention,
        AnalyticsSnap, Insights,
        MediaAsset,
        // Filters
        PostFilter, ScheduleFilter, CommentFilter, MediaFilter, MentionFilter,
        // Threads API types
        ContainerStatus, UserProfile, PublishingLimit, PublishingLimitConfig,
        InsightMetric, InsightValue, InsightTotalValue,
        UserInsightMetric, LinkTotalValue,
        CreateReply,
        // Sentiment
        SentimentResult, SentimentSummary,
        RateLimits,
    )),
    paths(
        // Health & Observability
        crate::server::health_check,
        crate::server::readiness_check,
        crate::server::metrics,
        // Accounts
        crate::routes::accounts::list_accounts,
        crate::routes::accounts::create_account,
        crate::routes::accounts::update_account,
        crate::routes::accounts::delete_account,
        crate::routes::accounts::refresh_token,
        // Posts
        crate::routes::posts::list_posts,
        crate::routes::posts::create_post,
        crate::routes::posts::get_post,
        crate::routes::posts::delete_post,
        crate::routes::posts::get_insights,
        // Schedules
        crate::routes::schedules::list_schedules,
        crate::routes::schedules::create_schedule,
        crate::routes::schedules::get_schedule_by_id,
        crate::routes::schedules::update_schedule,
        crate::routes::schedules::patch_schedule,
        crate::routes::schedules::delete_schedule,
        crate::routes::schedules::approve_schedule,
        crate::routes::schedules::reject_schedule,
        crate::routes::schedules::list_upcoming,
        // Comments
        crate::routes::comments::list_comments,
        crate::routes::comments::fetch_comments,
        crate::routes::comments::get_sentiment,
        crate::routes::comments::update_reply_status,
        crate::routes::comments::reply_to_comment,
        // Analytics
        crate::routes::analytics::list_analytics,
        crate::routes::analytics::post_trend,
        // Media
        crate::routes::media::list_media,
        crate::routes::media::upload_media,
        crate::routes::media::delete_media,
    ),
    tags(
        (name = "health", description = "Health check"),
        (name = "accounts", description = "Account management"),
        (name = "posts", description = "Post management"),
        (name = "schedules", description = "Content scheduling"),
        (name = "comments", description = "Comment and reply management"),
        (name = "analytics", description = "Analytics and insights"),
        (name = "media", description = "Media library"),
        (name = "threads", description = "Direct Threads API operations"),
    ),
    modifiers(&SecurityAddon),
)]
pub struct ApiDoc;

/// Adds the `api_key` security scheme (X-API-Key header) to the OpenAPI spec.
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "api_key",
                SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("X-API-Key"))),
            );
        }
    }
}

/// Build the Swagger UI router for serving interactive API docs.
pub fn swagger_ui() -> SwaggerUi {
    SwaggerUi::new("/api/docs").url("/api/docs/openapi.json", ApiDoc::openapi())
}
