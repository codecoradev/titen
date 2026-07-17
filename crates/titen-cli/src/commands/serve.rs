use anyhow::Result;

pub async fn run(host: &str, port: u16, _mcp: bool) -> Result<()> {
    let db_path = titen_core::config::default_db_path();
    titen_core::config::ensure_parent_dir(&db_path);
    let api_key = std::env::var(titen_core::config::ENV_API_KEY).ok();

    // Start embedded API server
    titen_api::server::serve(host, port, &db_path, api_key).await
}
