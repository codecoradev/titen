//! Integration tests for titen-cli argument parsing.
//!
//! Verifies that all CLI subcommands parse correctly via clap derive,
//! including defaults, required fields, and optional fields.

use clap::Parser;
use titen_cli::commands::{
    account::AccountAction, analytics::AnalyticsAction, comment::CommentAction, media::MediaAction,
    post::PostAction, schedule::ScheduleAction,
};
use titen_cli::{Cli, Commands};

// ── Serve subcommand ─────────────────────────────────────────────────

#[test]
fn test_serve_with_all_args() {
    let cli = Cli::parse_from([
        "titen", "serve", "--port", "8080", "--host", "0.0.0.0", "--mcp",
    ]);
    match cli.command {
        Commands::Serve { port, host, mcp } => {
            assert_eq!(port, 8080);
            assert_eq!(host, "0.0.0.0");
            assert!(mcp);
        }
        other => panic!("expected Serve, got {other:?}"),
    }
}

#[test]
fn test_serve_defaults() {
    let cli = Cli::parse_from(["titen", "serve"]);
    match cli.command {
        Commands::Serve { port, host, mcp } => {
            assert_eq!(port, 7845);
            assert_eq!(host, "0.0.0.0");
            assert!(!mcp);
        }
        other => panic!("expected Serve, got {other:?}"),
    }
}

#[test]
fn test_serve_short_flags() {
    let cli = Cli::parse_from(["titen", "serve", "-p", "3000", "--host", "127.0.0.1"]);
    match cli.command {
        Commands::Serve { port, host, mcp } => {
            assert_eq!(port, 3000);
            assert_eq!(host, "127.0.0.1");
            assert!(!mcp);
        }
        other => panic!("expected Serve, got {other:?}"),
    }
}

// ── Account subcommand ──────────────────────────────────────────────

#[test]
fn test_account_list() {
    let cli = Cli::parse_from(["titen", "account", "list"]);
    match cli.command {
        Commands::Account { action } => match action {
            AccountAction::List => {}
            other => panic!("expected List, got {other:?}"),
        },
        other => panic!("expected Account, got {other:?}"),
    }
}

#[test]
fn test_account_add_full() {
    let cli = Cli::parse_from([
        "titen",
        "account",
        "add",
        "--access-token",
        "tok123",
        "--username",
        "alice",
        "--user-id",
        "uid456",
    ]);
    match cli.command {
        Commands::Account { action } => match action {
            AccountAction::Add {
                access_token,
                username,
                user_id,
                ..
            } => {
                assert_eq!(access_token, "tok123");
                assert_eq!(username.as_deref(), Some("alice"));
                assert_eq!(user_id.as_deref(), Some("uid456"));
            }
            other => panic!("expected Add, got {other:?}"),
        },
        other => panic!("expected Account, got {other:?}"),
    }
}

#[test]
fn test_account_add_minimal() {
    let cli = Cli::parse_from(["titen", "account", "add", "--access-token", "only_token"]);
    match cli.command {
        Commands::Account { action } => match action {
            AccountAction::Add {
                access_token,
                username,
                user_id,
                app_secret,
                expires_at,
            } => {
                assert_eq!(access_token, "only_token");
                assert!(username.is_none());
                assert!(user_id.is_none());
                assert!(app_secret.is_none());
                assert!(expires_at.is_none());
            }
            other => panic!("expected Add, got {other:?}"),
        },
        other => panic!("expected Account, got {other:?}"),
    }
}

#[test]
fn test_account_remove() {
    let cli = Cli::parse_from(["titen", "account", "remove", "abc123"]);
    match cli.command {
        Commands::Account { action } => match action {
            AccountAction::Remove { id } => assert_eq!(id, "abc123"),
            other => panic!("expected Remove, got {other:?}"),
        },
        other => panic!("expected Account, got {other:?}"),
    }
}

#[test]
fn test_account_refresh() {
    let cli = Cli::parse_from(["titen", "account", "refresh", "acct-99"]);
    match cli.command {
        Commands::Account { action } => match action {
            AccountAction::Refresh { id } => assert_eq!(id, "acct-99"),
            other => panic!("expected Refresh, got {other:?}"),
        },
        other => panic!("expected Account, got {other:?}"),
    }
}

// ── Post subcommand ─────────────────────────────────────────────────

#[test]
fn test_post_list() {
    let cli = Cli::parse_from(["titen", "post", "list"]);
    match cli.command {
        Commands::Post { action } => match action {
            PostAction::List { .. } => {}
            other => panic!("expected List, got {other:?}"),
        },
        other => panic!("expected Post, got {other:?}"),
    }
}

#[test]
fn test_post_create() {
    let cli = Cli::parse_from(["titen", "post", "create", "acct-1", "--text", "hello world"]);
    match cli.command {
        Commands::Post { action } => match action {
            PostAction::Create {
                account,
                text,
                media_type,
                image_url,
            } => {
                assert_eq!(account, "acct-1");
                assert_eq!(text, "hello world");
                assert!(media_type.is_none());
                assert!(image_url.is_none());
            }
            other => panic!("expected Create, got {other:?}"),
        },
        other => panic!("expected Post, got {other:?}"),
    }
}

#[test]
fn test_post_list_with_filters() {
    let cli = Cli::parse_from([
        "titen",
        "post",
        "list",
        "--account",
        "acct-1",
        "--status",
        "published",
    ]);
    match cli.command {
        Commands::Post { action } => match action {
            PostAction::List { account, status } => {
                assert_eq!(account.as_deref(), Some("acct-1"));
                assert_eq!(status.as_deref(), Some("published"));
            }
            other => panic!("expected List, got {other:?}"),
        },
        other => panic!("expected Post, got {other:?}"),
    }
}

#[test]
fn test_post_delete() {
    let cli = Cli::parse_from(["titen", "post", "delete", "post-42"]);
    match cli.command {
        Commands::Post { action } => match action {
            PostAction::Delete { post_id } => assert_eq!(post_id, "post-42"),
            other => panic!("expected Delete, got {other:?}"),
        },
        other => panic!("expected Post, got {other:?}"),
    }
}

// ── Schedule subcommand ─────────────────────────────────────────────

#[test]
fn test_schedule_list() {
    let cli = Cli::parse_from(["titen", "schedule", "list"]);
    match cli.command {
        Commands::Schedule { action } => match action {
            ScheduleAction::List { .. } => {}
            other => panic!("expected List, got {other:?}"),
        },
        other => panic!("expected Schedule, got {other:?}"),
    }
}

#[test]
fn test_schedule_create() {
    let cli = Cli::parse_from([
        "titen",
        "schedule",
        "create",
        "acct-1",
        "--text",
        "hello",
        "--at",
        "2026-01-01T00:00:00",
    ]);
    match cli.command {
        Commands::Schedule { action } => match action {
            ScheduleAction::Create {
                account,
                text,
                at,
                media_type,
            } => {
                assert_eq!(account, "acct-1");
                assert_eq!(text, "hello");
                assert_eq!(at, "2026-01-01T00:00:00");
                assert!(media_type.is_none());
            }
            other => panic!("expected Create, got {other:?}"),
        },
        other => panic!("expected Schedule, got {other:?}"),
    }
}

#[test]
fn test_schedule_upcoming() {
    let cli = Cli::parse_from(["titen", "schedule", "upcoming"]);
    match cli.command {
        Commands::Schedule { action } => match action {
            ScheduleAction::Upcoming => {}
            other => panic!("expected Upcoming, got {other:?}"),
        },
        other => panic!("expected Schedule, got {other:?}"),
    }
}

#[test]
fn test_schedule_cancel() {
    let cli = Cli::parse_from(["titen", "schedule", "cancel", "sched-5"]);
    match cli.command {
        Commands::Schedule { action } => match action {
            ScheduleAction::Cancel { id } => assert_eq!(id, "sched-5"),
            other => panic!("expected Cancel, got {other:?}"),
        },
        other => panic!("expected Schedule, got {other:?}"),
    }
}

// ── Comment subcommand ──────────────────────────────────────────────

#[test]
fn test_comment_fetch() {
    let cli = Cli::parse_from(["titen", "comment", "fetch", "post-123"]);
    match cli.command {
        Commands::Comment { action } => match action {
            CommentAction::Fetch { post_id } => assert_eq!(post_id, "post-123"),
            other => panic!("expected Fetch, got {other:?}"),
        },
        other => panic!("expected Comment, got {other:?}"),
    }
}

#[test]
fn test_comment_list() {
    let cli = Cli::parse_from(["titen", "comment", "list", "post-77"]);
    match cli.command {
        Commands::Comment { action } => match action {
            CommentAction::List { post_id } => assert_eq!(post_id, "post-77"),
            other => panic!("expected List, got {other:?}"),
        },
        other => panic!("expected Comment, got {other:?}"),
    }
}

// ── Analytics subcommand ────────────────────────────────────────────

#[test]
fn test_analytics_posts() {
    let cli = Cli::parse_from([
        "titen",
        "analytics",
        "posts",
        "acct-1",
        "--from",
        "2026-01-01",
        "--to",
        "2026-01-31",
    ]);
    match cli.command {
        Commands::Analytics { action } => match action {
            AnalyticsAction::Posts { account, from, to } => {
                assert_eq!(account, "acct-1");
                assert_eq!(from.as_deref(), Some("2026-01-01"));
                assert_eq!(to.as_deref(), Some("2026-01-31"));
            }
            other => panic!("expected Posts, got {other:?}"),
        },
        other => panic!("expected Analytics, got {other:?}"),
    }
}

#[test]
fn test_analytics_trend() {
    let cli = Cli::parse_from(["titen", "analytics", "trend", "post-99"]);
    match cli.command {
        Commands::Analytics { action } => match action {
            AnalyticsAction::Trend { post_id } => assert_eq!(post_id, "post-99"),
            other => panic!("expected Trend, got {other:?}"),
        },
        other => panic!("expected Analytics, got {other:?}"),
    }
}

// ── Media subcommand ────────────────────────────────────────────────

#[test]
fn test_media_list() {
    let cli = Cli::parse_from(["titen", "media", "list"]);
    match cli.command {
        Commands::Media { action } => match action {
            MediaAction::List => {}
            other => panic!("expected List, got {other:?}"),
        },
        other => panic!("expected Media, got {other:?}"),
    }
}

#[test]
fn test_media_upload() {
    let cli = Cli::parse_from([
        "titen",
        "media",
        "upload",
        "/tmp/image.png",
        "--content-type",
        "image/png",
    ]);
    match cli.command {
        Commands::Media { action } => match action {
            MediaAction::Upload {
                file_path,
                content_type,
            } => {
                assert_eq!(file_path, "/tmp/image.png");
                assert_eq!(content_type.as_deref(), Some("image/png"));
            }
            other => panic!("expected Upload, got {other:?}"),
        },
        other => panic!("expected Media, got {other:?}"),
    }
}

#[test]
fn test_media_delete() {
    let cli = Cli::parse_from(["titen", "media", "delete", "media-3"]);
    match cli.command {
        Commands::Media { action } => match action {
            MediaAction::Delete { id } => assert_eq!(id, "media-3"),
            other => panic!("expected Delete, got {other:?}"),
        },
        other => panic!("expected Media, got {other:?}"),
    }
}

// ── TokenCheck subcommand ───────────────────────────────────────────

#[test]
fn test_token_check() {
    let cli = Cli::parse_from(["titen", "token-check"]);
    match cli.command {
        Commands::TokenCheck => {}
        other => panic!("expected TokenCheck, got {other:?}"),
    }
}

// ── Invalid input ───────────────────────────────────────────────────

#[test]
fn test_unknown_subcommand_fails() {
    let result = Cli::try_parse_from(["titen", "nonexistent"]);
    assert!(result.is_err(), "should fail on unknown subcommand");
}

#[test]
fn test_account_add_missing_token_fails() {
    // --access-token is required
    let result = Cli::try_parse_from(["titen", "account", "add", "--username", "bob"]);
    assert!(result.is_err(), "should fail without --access-token");
}

#[test]
fn test_post_create_missing_required_text_fails() {
    // text (-t/--text) is required for post create
    let result = Cli::try_parse_from(["titen", "post", "create", "acct-1"]);
    assert!(result.is_err(), "should fail without --text");
}
