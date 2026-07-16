use anyhow::{Context, Result};
use serde_json::Value;

/// Titen CLI configuration
pub struct TitenConfig {
    pub base_url: String,
    pub api_key: Option<String>,
    #[allow(dead_code)]
    pub db_path: String,
}

impl TitenConfig {
    pub fn from_env() -> Self {
        Self {
            base_url: std::env::var("TITEN_URL").unwrap_or_else(|_| "http://localhost:7845".into()),
            api_key: std::env::var("TITEN_API_KEY").ok(),
            db_path: std::env::var("TITEN_DB_PATH").unwrap_or_else(|_| "./titen.db".into()),
        }
    }
}

/// HTTP client wrapper for Titen API
pub struct TitenApi {
    client: reqwest::Client,
    base_url: String,
    api_key: Option<String>,
}

impl TitenApi {
    pub fn new(config: &TitenConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: config.base_url.clone(),
            api_key: config.api_key.clone(),
        }
    }

    fn build_url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }

    async fn request(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value> {
        let url = self.build_url(path);
        let mut req = self.client.request(method, &url);

        if let Some(key) = &self.api_key {
            req = req.header("X-API-Key", key);
        }

        if let Some(b) = body {
            req = req.json(&b);
        }

        let resp = req.send().await.context("Request failed")?;
        let status = resp.status();
        let json: Value = resp.json().await.context("Failed to parse response")?;

        if !status.is_success() {
            let err = json
                .get("error")
                .and_then(|e| e.as_str())
                .unwrap_or("Unknown error");
            let code = json
                .get("code")
                .and_then(|c| c.as_str())
                .unwrap_or("UNKNOWN");
            anyhow::bail!("{code}: {err}");
        }

        Ok(json)
    }

    pub async fn get(&self, path: &str) -> Result<Value> {
        self.request(reqwest::Method::GET, path, None).await
    }

    pub async fn post(&self, path: &str, body: Value) -> Result<Value> {
        self.request(reqwest::Method::POST, path, Some(body)).await
    }

    #[allow(dead_code)]
    pub async fn put(&self, path: &str, body: Value) -> Result<Value> {
        self.request(reqwest::Method::PUT, path, Some(body)).await
    }

    pub async fn delete(&self, path: &str) -> Result<Value> {
        self.request(reqwest::Method::DELETE, path, None).await
    }
}

/// Print JSON data field nicely
pub fn print_data(val: &Value) {
    if let Some(data) = val.get("data") {
        let pretty = serde_json::to_string_pretty(data).unwrap_or_else(|_| data.to_string());
        println!("{pretty}");
    } else if let Some(error) = val.get("error") {
        let code = val.get("code").and_then(|c| c.as_str()).unwrap_or("ERROR");
        eprintln!("{code}: {error}");
    } else {
        let pretty = serde_json::to_string_pretty(val).unwrap_or_else(|_| val.to_string());
        println!("{pretty}");
    }
}
