//! 节点上本机登记的 B 站账号。
//!
//! 只读 `configuration` 表里 `bilibili-cookies` 登记的凭据文件，从里面取 `token_info.mid`，不联网。
//! 上报给控制面的只有 mid 与昵称；凭据文件路径只在本机用来把模板的 `account_mid` 换回 `user_cookie`。

use super::model::Account;
use crate::server::infrastructure::connection_pool::ConnectionPool;
use serde_json::Value;
use std::path::Path;
use tracing::debug;

/// 本机的一个账号与它的凭据文件
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalAccount {
    pub mid: u64,
    pub uname: String,
    /// 登记时的规范路径，与 `uploadstreamers.user_cookie` 同一写法
    pub path: String,
}

impl LocalAccount {
    pub fn public(&self) -> Account {
        Account {
            mid: self.mid,
            uname: self.uname.clone(),
        }
    }
}

/// 从凭据文件内容里取 mid 与昵称。biliup-rs 登录写出的文件没有昵称，这时昵称为空。
pub fn parse_credential(text: &str) -> Option<(u64, String)> {
    let value: Value = serde_json::from_str(text).ok()?;
    let mid = value.pointer("/token_info/mid").and_then(|mid| {
        mid.as_u64()
            .or_else(|| mid.as_str().and_then(|s| s.trim().parse().ok()))
    })?;
    if mid == 0 {
        return None;
    }
    let uname = ["/uname", "/token_info/uname", "/name"]
        .iter()
        .find_map(|pointer| value.pointer(pointer).and_then(Value::as_str))
        .map(|name| name.trim().to_string())
        .unwrap_or_default();
    Some((mid, uname))
}

/// 本机登记的账号，按登记顺序；同一个 mid 登记了几份凭据文件时取最早登记的那份
pub async fn scan(pool: &ConnectionPool) -> Vec<LocalAccount> {
    let paths: Vec<String> = match sqlx::query_scalar(
        "SELECT value FROM configuration WHERE key = 'bilibili-cookies' ORDER BY id",
    )
    .fetch_all(pool)
    .await
    {
        Ok(paths) => paths,
        Err(e) => {
            debug!(error = %e, "could not list bilibili credentials");
            return Vec::new();
        }
    };
    let mut accounts: Vec<LocalAccount> = Vec::new();
    for path in paths {
        let Ok(text) = tokio::fs::read_to_string(Path::new(&path)).await else {
            continue;
        };
        let Some((mid, uname)) = parse_credential(&text) else {
            continue;
        };
        if accounts.iter().any(|account| account.mid == mid) {
            continue;
        }
        accounts.push(LocalAccount { mid, uname, path });
    }
    accounts
}

pub fn public(accounts: &[LocalAccount]) -> Vec<Account> {
    accounts.iter().map(LocalAccount::public).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mid_comes_from_token_info_and_the_rest_is_ignored() {
        let text = serde_json::json!({
            "cookie_info": { "cookies": [{ "name": "SESSDATA", "value": "secret" }] },
            "token_info": { "access_token": "secret", "mid": 42, "expires_in": 1, "refresh_token": "r" },
            "platform": "BiliTV",
        })
        .to_string();
        assert_eq!(parse_credential(&text), Some((42, String::new())));
        let named =
            serde_json::json!({ "token_info": { "mid": "7" }, "uname": " 甲 " }).to_string();
        assert_eq!(parse_credential(&named), Some((7, "甲".into())));
        assert_eq!(parse_credential("{}"), None);
        assert_eq!(parse_credential("not json"), None);
        assert_eq!(parse_credential(r#"{"token_info":{"mid":0}}"#), None);
    }

    #[tokio::test]
    async fn registered_credential_files_are_scanned_without_network() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("data.sqlite3");
        let pool = crate::server::infrastructure::connection_pool::ConnectionManager::new_pool(
            db.to_str().unwrap(),
        )
        .await
        .unwrap();
        let write = |name: &str, body: Value| {
            let path = dir.path().join(name);
            std::fs::write(&path, body.to_string()).unwrap();
            path.to_string_lossy().into_owned()
        };
        let first = write("a.json", serde_json::json!({ "token_info": { "mid": 42 } }));
        let duplicate = write("b.json", serde_json::json!({ "token_info": { "mid": 42 } }));
        let broken = write("c.json", serde_json::json!({ "cookie_info": {} }));
        let other = write("d.json", serde_json::json!({ "token_info": { "mid": 7 } }));
        let missing = dir.path().join("gone.json").to_string_lossy().into_owned();
        for path in [&first, &duplicate, &broken, &missing, &other] {
            sqlx::query("INSERT INTO configuration (key, value) VALUES ('bilibili-cookies', ?)")
                .bind(path)
                .execute(&pool)
                .await
                .unwrap();
        }
        let accounts = scan(&pool).await;
        assert_eq!(
            accounts
                .iter()
                .map(|a| (a.mid, a.path.as_str()))
                .collect::<Vec<_>>(),
            [(42, first.as_str()), (7, other.as_str())]
        );
        assert_eq!(
            public(&accounts),
            [
                Account {
                    mid: 42,
                    uname: String::new()
                },
                Account {
                    mid: 7,
                    uname: String::new()
                }
            ]
        );
    }
}
