use axum::http::{HeaderMap, HeaderName, HeaderValue};
use std::collections::HashMap;
use std::str::FromStr;

pub mod download;
pub mod upload;
/// 通用工具函数
pub mod util;

pub fn construct_headers(hash_map: &HashMap<String, String>) -> Result<HeaderMap, String> {
    let mut headers = HeaderMap::new();
    for (key, value) in hash_map.iter() {
        let name =
            HeaderName::from_str(key).map_err(|e| format!("invalid header name {key:?}: {e}"))?;
        let value = HeaderValue::from_str(value)
            .map_err(|e| format!("invalid header value for {key:?}: {e}"))?;
        headers.insert(name, value);
    }
    Ok(headers)
}
