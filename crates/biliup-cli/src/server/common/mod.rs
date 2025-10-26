use axum::http::{HeaderMap, HeaderName, HeaderValue};
use std::collections::HashMap;
use std::str::FromStr;

pub mod download;
pub mod upload;
/// 通用工具函数
pub mod util;

pub fn construct_headers(hash_map: &HashMap<String, String>) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for (key, value) in hash_map.iter() {
        headers.insert(
            HeaderName::from_str(key).unwrap(),
            HeaderValue::from_str(value).unwrap(),
        );
    }
    headers
}
