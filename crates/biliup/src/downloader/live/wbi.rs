use reqwest::Client;
use reqwest::header::HeaderMap;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::debug;

use super::{LiveError, LiveResult};

const KEY_MAP: [usize; 64] = [
    46, 47, 18, 2, 53, 8, 23, 32, 15, 50, 10, 31, 58, 3, 45, 35, 27, 43, 5, 49, 33, 9, 42, 19, 29,
    28, 14, 39, 12, 38, 41, 13, 37, 48, 7, 16, 24, 55, 40, 61, 26, 17, 0, 1, 60, 51, 30, 4, 22, 25,
    54, 21, 56, 59, 6, 63, 57, 62, 11, 36, 20, 34, 44, 52,
];
const UPDATE_INTERVAL: u64 = 2 * 60 * 60;

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

#[derive(Default)]
struct WbiState {
    key: Option<String>,
    last_update: u64,
}

/// WBI 签名器。key 状态放在 `Arc` 内共享：插件结构体持有一份，
/// 每次 `check_stream` 克隆句柄即可跨轮询复用，使 2 小时更新间隔真正生效。
#[derive(Clone, Default)]
pub struct WbiSigner {
    state: Arc<RwLock<WbiState>>,
}

impl WbiSigner {
    pub fn new() -> Self {
        Self::default()
    }

    fn extract_key(url: &str) -> Option<String> {
        url.rsplit('/')
            .next()
            .and_then(|s| s.split('.').next())
            .map(|s| s.to_string())
    }

    fn create_mixin_key(img: &str, sub: &str) -> String {
        let full: Vec<char> = format!("{}{}", img, sub).chars().collect();
        KEY_MAP
            .iter()
            .take(32)
            .filter_map(|&i| full.get(i).copied())
            .collect()
    }

    fn is_fresh(state: &WbiState, now: u64) -> bool {
        state.key.is_some() && now.saturating_sub(state.last_update) < UPDATE_INTERVAL
    }

    async fn update_key(&self, client: &Client, headers: &HeaderMap) -> LiveResult<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or_default();

        if Self::is_fresh(&*self.state.read().await, now) {
            return Ok(());
        }

        // 写锁内复查，避免并发 check_stream 时重复请求 nav
        let mut state = self.state.write().await;
        if Self::is_fresh(&state, now) {
            return Ok(());
        }

        debug!("Updating WBI key...");

        let resp: NavResponse = client
            .get("https://api.bilibili.com/x/web-interface/nav")
            .headers(headers.clone())
            .timeout(Duration::from_secs(5))
            .send()
            .await?
            .json()
            .await?;

        let img = Self::extract_key(&resp.data.wbi_img.img_url)
            .ok_or_else(|| LiveError::custom("提取 B 站 WBI img key 失败"))?;
        let sub = Self::extract_key(&resp.data.wbi_img.sub_url)
            .ok_or_else(|| LiveError::custom("提取 B 站 WBI sub key 失败"))?;

        state.key = Some(Self::create_mixin_key(&img, &sub));
        state.last_update = now;
        Ok(())
    }

    pub async fn sign(
        &self,
        client: &Client,
        params: &mut BTreeMap<String, String>,
        headers: &HeaderMap,
    ) -> LiveResult<()> {
        self.update_key(client, headers).await?;

        let key = self
            .state
            .read()
            .await
            .key
            .clone()
            .ok_or_else(|| LiveError::custom("B 站 WBI key 为空"))?;

        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or_default();
        params.insert("wts".to_string(), ts.to_string());

        let sanitized: BTreeMap<String, String> = params
            .iter()
            .map(|(k, v)| {
                let sanitized_v: String = v
                    .chars()
                    .filter(|c| !['!', '\'', '(', ')', '*'].contains(c))
                    .collect();
                (k.clone(), sanitized_v)
            })
            .collect();
        let query_string = sanitized
            .iter()
            .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        let mut hasher = md5::Md5::new();
        use md5::Digest;
        hasher.update(format!("{}{}", query_string, key).as_bytes());
        params.insert("w_rid".to_string(), format!("{:x}", hasher.finalize()));
        Ok(())
    }
}
