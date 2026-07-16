use crate::error::Result;
use async_trait::async_trait;

/// Entry in a storage listing
#[derive(Debug, Clone)]
pub struct StorageEntry {
    pub key: String,
    pub size: Option<i64>,
    pub last_modified: Option<String>,
}

/// Trait for S3-compatible storage backends
#[async_trait]
pub trait Storage: Send + Sync {
    /// Upload data to a key, return the public URL
    async fn upload(&self, key: &str, data: &[u8], content_type: &str) -> Result<String>;

    /// Delete a key
    async fn delete(&self, key: &str) -> Result<()>;

    /// Get the public URL for a key
    async fn get_url(&self, key: &str) -> Result<String>;

    /// List entries with a prefix (stub for now)
    async fn list(&self, prefix: &str) -> Result<Vec<StorageEntry>>;
}

/// S3-compatible storage using reqwest directly (avoids complex rust-s3 dependency)
pub struct S3Storage {
    endpoint: String,
    bucket: String,
    #[allow(dead_code)]
    region: String,
    #[allow(dead_code)]
    access_key: String,
    #[allow(dead_code)]
    secret_key: String,
    public_url: Option<String>,
    client: reqwest::Client,
}

impl S3Storage {
    /// Create a new S3 storage client from ENV variables
    pub fn from_env() -> Result<Self> {
        let endpoint = std::env::var("TITEN_S3_ENDPOINT").map_err(|_| {
            crate::error::TitenError::ConfigError(
                "TITEN_S3_ENDPOINT is required for S3 storage".to_string(),
            )
        })?;
        let bucket = std::env::var("TITEN_S3_BUCKET").map_err(|_| {
            crate::error::TitenError::ConfigError(
                "TITEN_S3_BUCKET is required for S3 storage".to_string(),
            )
        })?;
        let access_key = std::env::var("TITEN_S3_ACCESS_KEY").map_err(|_| {
            crate::error::TitenError::ConfigError(
                "TITEN_S3_ACCESS_KEY is required for S3 storage".to_string(),
            )
        })?;
        let secret_key = std::env::var("TITEN_S3_SECRET_KEY").map_err(|_| {
            crate::error::TitenError::ConfigError(
                "TITEN_S3_SECRET_KEY is required for S3 storage".to_string(),
            )
        })?;
        let region = std::env::var("TITEN_S3_REGION").unwrap_or_else(|_| "us-east-1".to_string());
        let public_url = std::env::var("TITEN_S3_PUBLIC_URL").ok();

        Ok(Self {
            endpoint,
            bucket,
            region,
            access_key,
            secret_key,
            public_url,
            client: reqwest::Client::new(),
        })
    }

    /// Build the object URL for a key
    fn object_url(&self, key: &str) -> String {
        format!("{}/{}/{}", self.endpoint, self.bucket, key)
    }

    /// Build the date-based key path: YYYY/MM/DD/uuid.ext
    pub fn build_key(filename: &str) -> String {
        let now = chrono::Utc::now();
        let ext = std::path::Path::new(filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("bin");
        format!(
            "{}/{:02}/{:02}/{}.{}",
            now.format("%Y"),
            now.format("%m"),
            now.format("%d"),
            uuid::Uuid::now_v7(),
            ext
        )
    }
}

#[async_trait]
impl Storage for S3Storage {
    async fn upload(&self, key: &str, data: &[u8], content_type: &str) -> Result<String> {
        let url = self.object_url(key);
        let resp = self
            .client
            .put(&url)
            .header("Content-Type", content_type)
            .body(data.to_vec())
            .send()
            .await
            .map_err(|e| crate::error::TitenError::StorageError(format!("Upload failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(crate::error::TitenError::StorageError(format!(
                "Upload failed with status {status}: {body}"
            )));
        }

        self.get_url(key).await
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let url = self.object_url(key);
        let resp =
            self.client.delete(&url).send().await.map_err(|e| {
                crate::error::TitenError::StorageError(format!("Delete failed: {e}"))
            })?;

        if !resp.status().is_success() && resp.status() != reqwest::StatusCode::NO_CONTENT {
            return Err(crate::error::TitenError::StorageError(format!(
                "Delete failed with status {}",
                resp.status()
            )));
        }

        Ok(())
    }

    async fn get_url(&self, key: &str) -> Result<String> {
        if let Some(base) = &self.public_url {
            Ok(format!("{base}/{key}"))
        } else {
            Ok(self.object_url(key))
        }
    }

    async fn list(&self, _prefix: &str) -> Result<Vec<StorageEntry>> {
        // S3 list API requires signed requests — stub for now
        Ok(Vec::new())
    }
}
