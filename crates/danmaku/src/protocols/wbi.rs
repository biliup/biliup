//! WBI signature implementation for Bilibili API.
//!
//! Bilibili uses WBI (Web Browser Interface) signing for API requests.
//! This involves:
//! 1. Fetching img_key and sub_key from the nav API
//! 2. Creating a mixin key using a fixed mapping
//! 3. Signing query parameters with MD5

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::header::HeaderMap;
use serde::Deserialize;
use tokio::sync::RwLock;
use tracing::debug;

/// WBI key mapping table.
const KEY_MAP: [usize; 64] = [
    46, 47, 18, 2, 53, 8, 23, 32, 15, 50, 10, 31, 58, 3, 45, 35,
    27, 43, 5, 49, 33, 9, 42, 19, 29, 28, 14, 39, 12, 38, 41, 13,
    37, 48, 7, 16, 24, 55, 40, 61, 26, 17, 0, 1, 60, 51, 30, 4,
    22, 25, 54, 21, 56, 59, 6, 63, 57, 62, 11, 36, 20, 34, 44, 52,
];

/// WBI key cache update interval (2 hours).
const UPDATE_INTERVAL: u64 = 2 * 60 * 60;

/// Navigation API response.
#[derive(Debug, Deserialize)]
struct NavResponse {
    data: NavData,
}

#[derive(Debug, Deserialize)]
struct NavData {
    wbi_img: WbiImg,
}

#[derive(Debug, Deserialize)]
struct WbiImg {
    img_url: String,
    sub_url: String,
}

/// WBI signer for Bilibili API requests.
pub struct WbiSigner {
    client: reqwest::Client,
    /// Cached mixin key.
    key: Arc<RwLock<Option<String>>>,
    /// Last update timestamp.
    last_update: Arc<RwLock<u64>>,
}

impl WbiSigner {
    /// Create a new WBI signer.
    pub fn new(client: reqwest::Client) -> Self {
        Self {
            client,
            key: Arc::new(RwLock::new(None)),
            last_update: Arc::new(RwLock::new(0)),
        }
    }

    /// Extract key from URL.
    /// E.g., "https://...abc123.png" -> "abc123"
    fn extract_key(url: &str) -> Option<String> {
        url.rsplit('/')
            .next()
            .and_then(|s| s.split('.').next())
            .map(|s| s.to_string())
    }

    /// Create mixin key from img and sub keys.
    fn create_mixin_key(img: &str, sub: &str) -> String {
        let full: Vec<char> = format!("{}{}", img, sub).chars().collect();
        KEY_MAP.iter()
            .take(32)
            .filter_map(|&i| full.get(i).copied())
            .collect()
    }

    /// Update the WBI key from Bilibili's API.
    async fn update_key(&self, headers: &HeaderMap) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Check if update is needed
        {
            let last = *self.last_update.read().await;
            if now - last < UPDATE_INTERVAL {
                if self.key.read().await.is_some() {
                    return Ok(());
                }
            }
        }

        debug!("Updating WBI key...");

        let resp: NavResponse = self.client
            .get("https://api.bilibili.com/x/web-interface/nav")
            .headers(headers.clone())
            .timeout(Duration::from_secs(5))
            .send()
            .await?
            .json()
            .await?;

        let img = Self::extract_key(&resp.data.wbi_img.img_url)
            .ok_or("Failed to extract img key")?;
        let sub = Self::extract_key(&resp.data.wbi_img.sub_url)
            .ok_or("Failed to extract sub key")?;

        let mixin_key = Self::create_mixin_key(&img, &sub);
        debug!("WBI mixin key created");

        *self.key.write().await = Some(mixin_key);
        *self.last_update.write().await = now;

        Ok(())
    }

    /// Sign query parameters with WBI.
    ///
    /// Adds `wts` (timestamp) and `w_rid` (signature) to the params.
    pub async fn sign(
        &self,
        params: &mut BTreeMap<String, String>,
        headers: &HeaderMap,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Update key if needed
        self.update_key(headers).await?;

        let key = self.key.read().await;
        let key = key.as_ref().ok_or("WBI key not available")?;

        // Add timestamp
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        params.insert("wts".to_string(), ts.to_string());

        // Sanitize and sort params
        let sanitized: BTreeMap<String, String> = params
            .iter()
            .map(|(k, v)| {
                let sanitized_v: String = v.chars()
                    .filter(|c| !['!', '\'', '(', ')', '*'].contains(c))
                    .collect();
                (k.clone(), sanitized_v)
            })
            .collect();

        // Build query string
        let query_string: String = sanitized
            .iter()
            .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        // Calculate MD5 signature
        let to_sign = format!("{}{}", query_string, key);
        let digest = md5::compute(to_sign.as_bytes());
        let w_rid = format!("{:x}", digest);

        params.insert("w_rid".to_string(), w_rid);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_key() {
        assert_eq!(
            WbiSigner::extract_key("https://i0.hdslb.com/bfs/wbi/abc123.png"),
            Some("abc123".to_string())
        );
    }

    #[test]
    fn test_create_mixin_key() {
        // Simple test with known values
        let img = "0123456789abcdef0123456789abcdef";
        let sub = "fedcba9876543210fedcba9876543210";
        let key = WbiSigner::create_mixin_key(img, sub);
        assert_eq!(key.len(), 32);
    }
}
