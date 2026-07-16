use anyhow::Result;

pub async fn run(host: &str, port: u16, _mcp: bool) -> Result<()> {
    // TODO: start embedded HTTP server via titen-api
    println!("titen-api would start on {host}:{port}");
    Ok(())
}
