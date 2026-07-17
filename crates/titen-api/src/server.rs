use std::sync::Arc;

use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::routes;
use titen_core::{Store, ThreadsClient};

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Store>,
    pub threads_client: Arc<ThreadsClient>,
    pub api_key: Option<String>,
}

#[derive(serde::Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
    db: &'static str,
}

#[derive(serde::Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}

pub fn error_response(
    status: StatusCode,
    code: &str,
    msg: &str,
) -> (StatusCode, Json<ErrorResponse>) {
    (
        status,
        Json(ErrorResponse {
            error: msg.to_string(),
            code: code.to_string(),
        }),
    )
}

/// API key auth middleware — checks X-API-Key header or api_key query param
pub async fn api_key_auth(
    State(state): State<AppState>,
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> Result<axum::http::Response<axum::body::Body>, (StatusCode, Json<ErrorResponse>)> {
    // Skip auth for health endpoint
    if req.uri().path() == "/health" {
        return Ok(next.run(req).await);
    }

    // If no API key configured, allow all (dev mode)
    let required_key = match &state.api_key {
        Some(key) if !key.is_empty() => key,
        _ => return Ok(next.run(req).await),
    };

    // Check header
    let provided = req
        .headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let provided = provided.or_else(|| {
        // Check query param
        req.uri().query().and_then(|q| {
            q.split('&')
                .find(|p| p.starts_with("api_key="))
                .map(|p| p.trim_start_matches("api_key=").to_string())
        })
    });

    match provided {
        Some(key) if key == *required_key => Ok(next.run(req).await),
        _ => Err(error_response(
            StatusCode::UNAUTHORIZED,
            "UNAUTHORIZED",
            "Invalid or missing API key",
        )),
    }
}

async fn health_check(State(_state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        db: "ok",
    })
}

pub async fn serve(
    host: &str,
    port: u16,
    db_path: &str,
    api_key: Option<String>,
) -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "titen_api=debug,tower_http=debug".parse().unwrap()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let pool = sqlx::SqlitePool::connect(&format!("sqlite:{db_path}?mode=rwc")).await?;
    let store = Store::new(pool.clone());
    store.migrate().await?;

    let store = Arc::new(store);
    let threads_client = Arc::new(ThreadsClient::new(store.clone()));

    let state = AppState {
        store: store.clone(),
        threads_client,
        api_key,
    };

    let protected_routes = Router::new()
        .route(
            "/api/accounts",
            get(routes::accounts::list_accounts).post(routes::accounts::create_account),
        )
        .route(
            "/api/accounts/{id}",
            put(routes::accounts::update_account).delete(routes::accounts::delete_account),
        )
        .route(
            "/api/accounts/{id}/refresh-token",
            post(routes::accounts::refresh_token),
        )
        .route(
            "/api/accounts/{id}/profile",
            get(routes::threads::get_user_profile),
        )
        .route(
            "/api/accounts/{id}/publishing-limit",
            get(routes::threads::get_publishing_limit),
        )
        .route(
            "/api/accounts/check-tokens",
            get(routes::threads::check_all_tokens),
        )
        .route(
            "/api/posts",
            get(routes::posts::list_posts).post(routes::posts::create_post),
        )
        .route(
            "/api/posts/{id}",
            get(routes::posts::get_post).delete(routes::posts::delete_post),
        )
        .route("/api/posts/{id}/insights", get(routes::posts::get_insights))
        .route(
            "/api/schedules",
            get(routes::schedules::list_schedules).post(routes::schedules::create_schedule),
        )
        .route(
            "/api/schedules/{id}",
            put(routes::schedules::update_schedule).delete(routes::schedules::delete_schedule),
        )
        .route(
            "/api/schedules/upcoming",
            get(routes::schedules::list_upcoming),
        )
        .route(
            "/api/posts/{id}/comments",
            get(routes::comments::list_comments),
        )
        .route(
            "/api/posts/{id}/comments/fetch",
            post(routes::comments::fetch_comments),
        )
        .route(
            "/api/posts/{id}/comments/sentiment",
            get(routes::comments::get_sentiment),
        )
        .route(
            "/api/analytics/posts",
            get(routes::analytics::list_analytics),
        )
        .route(
            "/api/analytics/posts/{id}/trend",
            get(routes::analytics::post_trend),
        )
        .route(
            "/api/media",
            get(routes::media::list_media).post(routes::media::upload_media),
        )
        .route("/api/media/{id}", delete(routes::media::delete_media))
        .route(
            "/api/threads/container",
            post(routes::threads::create_container),
        )
        .route(
            "/api/threads/container/{id}/publish",
            post(routes::threads::publish_container),
        )
        .route(
            "/api/threads/container/{id}/status",
            post(routes::threads::get_container_status),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            api_key_auth,
        ));

    let app = Router::new()
        .route("/health", get(health_check))
        .merge(protected_routes)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("{host}:{port}");
    tracing::info!("titen-api listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

pub fn main() {
    let host = std::env::var("TITEN_HOST").unwrap_or_else(|_| "0.0.0.0".into());
    let port: u16 = std::env::var("TITEN_PORT")
        .unwrap_or_else(|_| "7845".into())
        .parse()
        .unwrap_or(7845);
    let db_path = std::env::var("TITEN_DB_PATH").unwrap_or_else(|_| "./titen.db".into());
    let api_key = std::env::var("TITEN_API_KEY").ok();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to build tokio runtime");

    runtime.block_on(async {
        if let Err(e) = serve(&host, port, &db_path, api_key).await {
            tracing::error!("Server error: {e}");
            std::process::exit(1);
        }
    });
}
