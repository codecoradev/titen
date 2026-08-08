//! Shared configuration defaults for all Titen binaries.
//!
//! Centralizes environment variable names and default values so that
//! `titen-api`, `titen-cli`, and `titen-mcp` stay in sync.

/// Environment variable for the SQLite database path.
pub const ENV_DB_PATH: &str = "TITEN_DB_PATH";

/// Environment variable for the API listen host.
pub const ENV_HOST: &str = "TITEN_HOST";

/// Environment variable for the API listen port.
pub const ENV_PORT: &str = "TITEN_PORT";

/// Environment variable for global API key authentication.
pub const ENV_API_KEY: &str = "TITEN_API_KEY";

/// Environment variable for the Titen API base URL (used by CLI client).
pub const ENV_URL: &str = "TITEN_URL";

/// Environment variable for the display timezone.
pub const ENV_TZ: &str = "TZ";

/// Default timezone (UTC).
pub const DEFAULT_TZ: &str = "UTC";

/// Default listen host.
pub const DEFAULT_HOST: &str = "0.0.0.0";

/// Default listen port.
pub const DEFAULT_PORT: u16 = 7845;

/// Default API base URL (used by CLI client).
pub const DEFAULT_URL: &str = "http://localhost:7845";

/// Returns the configured or default database path.
///
/// Default: `~/.codecora/titen/titen.db`
///
/// Override with the `TITEN_DB_PATH` environment variable.
pub fn default_db_path() -> String {
    std::env::var(ENV_DB_PATH).unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        format!("{home}/.codecora/titen/titen.db")
    })
}

/// Ensure parent directories exist for the given path.
pub fn ensure_parent_dir(path: &str) {
    if let Some(parent) = std::path::Path::new(path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
}

/// Returns the configured or default listen host.
pub fn default_host() -> String {
    std::env::var(ENV_HOST).unwrap_or_else(|_| DEFAULT_HOST.into())
}

/// Returns the configured or default port.
pub fn default_port() -> u16 {
    std::env::var(ENV_PORT)
        .unwrap_or_else(|_| DEFAULT_PORT.to_string())
        .parse()
        .unwrap_or(DEFAULT_PORT)
}

/// Returns the configured display timezone.
///
/// Reads from the `TZ` environment variable. Defaults to `UTC` if unset.
///
/// Example: `TZ=Asia/Jakarta`
pub fn timezone() -> String {
    std::env::var(ENV_TZ).unwrap_or_else(|_| DEFAULT_TZ.into())
}
