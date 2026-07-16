//! Titen HTTP API — Axum-based REST server.
//!
//! Provides the `serve` function to start the HTTP server with:
//! - API key authentication middleware (`X-API-Key` header)
//! - CORS support
//! - Request tracing
//!
//! Routes are organized under `routes/` modules:
//! - [`routes::accounts`] — account CRUD and token refresh
//! - [`routes::posts`] — post creation, listing, deletion, and insights
//! - [`routes::schedules`] — schedule CRUD and upcoming listing
//! - [`routes::comments`] — comment fetching, listing, and sentiment analysis
//! - [`routes::analytics`] — aggregate analytics and per-post trends
//! - [`routes::media`] — media listing, upload (multipart), and deletion

pub mod routes;
pub mod server;
