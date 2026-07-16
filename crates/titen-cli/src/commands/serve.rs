use anyhow::Result;

pub async fn run(host: &str, port: u16, _mcp: bool) -> Result<()> {
    let db_path = std::env::var("TITEN_DB_PATH").unwrap_or_else(|_| "./titen.db".into());
    let api_key = std::env::var("TITEN_API_KEY").ok();

    // Start embedded API server
    titen_api::server::serve(host, port, &db_path, api_key).await
}
