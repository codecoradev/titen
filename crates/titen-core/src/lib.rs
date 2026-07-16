//! Titen core library — domain logic, storage, and integrations.
//!
//! This crate provides the shared foundation for all Titen binaries:
//!
//! - **Store** — SQLite-backed data access for accounts, posts, schedules,
//!   comments, media, and analytics.
//! - **ThreadsClient** — HTTP client for the Threads Graph API (publishing,
//!   fetching comments, insights, token refresh).
//! - **SentimentEngine** — pluggable sentiment analysis with [`KeywordEngine`]
//!   (bilingual EN+ID) and [`StubEngine`] (always neutral, for testing).
//! - **Storage** — S3-compatible media storage via the [`Storage`] trait.
//! - **TitenScheduler** — background scheduler that ticks to publish due
//!   scheduled posts.
//!
//! # Re-exports
//!
//! Key types are re-exported at the crate root for convenience:
//! [`Store`], [`ThreadsClient`], [`TitenScheduler`], [`SentimentEngine`],
//! [`KeywordEngine`], [`StubEngine`], [`Storage`], [`S3Storage`].

pub mod error;
pub mod models;
pub mod scheduler;
pub mod sentiment;
pub mod storage;
pub mod store;
pub mod threads_client;

pub use error::{Result, TitenError};
pub use scheduler::TitenScheduler;
pub use sentiment::{KeywordEngine, SentimentEngine, StubEngine, build_engine, compute_summary};
pub use storage::{S3Storage, Storage};
pub use store::Store;
pub use threads_client::ThreadsClient;
