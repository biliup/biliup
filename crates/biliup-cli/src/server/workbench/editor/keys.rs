//! Provider credentials are encrypted at rest and never included in editor documents or replies.
use super::ConnectionPool;
use ring::{
    aead,
    rand::{SecureRandom, SystemRandom},
};
use serde_json::{Value, json};
use std::path::Path;
static MASTER_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
pub const PROVIDERS: [(&str, &str, &str); 5] = [
    ("openai", "OpenAI", "GPT Image / SVG / HTML"),
    ("google", "Google", "Gemini Omni Flash / SVG / HTML"),
    ("elevenlabs", "ElevenLabs", "Sound effects"),
    ("freesound", "Freesound", "Music and sound effects search"),
    ("brave", "Brave Search", "Web search"),
];
fn provider(id: &str) -> Result<(), String> {
    if PROVIDERS.iter().any(|p| p.0 == id) {
        Ok(())
    } else {
        Err("未知的密钥供应商".into())
    }
}
async fn master(pool: &ConnectionPool, root: &Path) -> Result<aead::LessSafeKey, String> {
    let _guard = MASTER_LOCK.lock().await;
    let path = root.join("editor-credentials.key");
    let data = match tokio::fs::read(&path).await {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM editor_provider_keys")
                .fetch_one(pool)
                .await
                .map_err(|_| "无法读取密钥存储")?;
            if count > 0 {
                return Err("服务端加密主密钥缺失，请恢复 editor-credentials.key".into());
            }
            tokio::fs::create_dir_all(root)
                .await
                .map_err(|_| "无法创建密钥目录")?;
            let mut bytes = vec![0; 32];
            SystemRandom::new()
                .fill(&mut bytes)
                .map_err(|_| "无法生成加密主密钥")?;
            let directory=root.to_path_buf();
            tokio::task::spawn_blocking(move ||->Result<Vec<u8>,String>{
                use std::io::Write;
                // tempfile uses mode 0600 on Unix; Windows inherits the service data directory ACL.
                let mut file=tempfile::NamedTempFile::new_in(directory).map_err(|_|"无法创建加密主密钥")?;
                file.write_all(&bytes).map_err(|_|"无法保存加密主密钥")?;
                file.as_file().sync_all().map_err(|_|"无法保存加密主密钥")?;
                file.persist_noclobber(path).map_err(|_|"无法保存加密主密钥")?;
                Ok(bytes)
            }).await.map_err(|_|"无法保存加密主密钥")??
        }
        Err(_) => return Err("无法读取加密主密钥".into()),
    };
    if data.len() != 32 {
        return Err("服务端加密主密钥无效".into());
    }
    aead::UnboundKey::new(&aead::AES_256_GCM, &data)
        .map(aead::LessSafeKey::new)
        .map_err(|_| "无法初始化密钥存储".into())
}
pub async fn list(pool: &ConnectionPool) -> Result<Vec<Value>, String> {
    let stored: Vec<String> = sqlx::query_scalar("SELECT provider FROM editor_provider_keys")
        .fetch_all(pool)
        .await
        .map_err(|_| "无法读取密钥状态")?;
    Ok(PROVIDERS.iter().map(|(id,name,usage)|json!({"provider":id,"name":name,"set":stored.iter().any(|s|s==id),"storage":"server","usedBy":[usage]})).collect())
}
pub async fn set(
    pool: &ConnectionPool,
    root: &Path,
    id: &str,
    raw: &str,
) -> Result<Vec<Value>, String> {
    provider(id)?;
    let secret = raw.trim();
    if secret.is_empty()
        || secret.len() > 512
        || !secret.is_ascii()
        || secret.chars().any(|c| c.is_whitespace() || c.is_control())
    {
        return Err("API 密钥应为一行、不含空白字符且不超过 512 字节".into());
    }
    let key = master(pool, root).await?;
    let mut nonce = [0; 12];
    SystemRandom::new()
        .fill(&mut nonce)
        .map_err(|_| "无法生成密钥随机数")?;
    let mut encrypted = secret.as_bytes().to_vec();
    key.seal_in_place_append_tag(
        aead::Nonce::assume_unique_for_key(nonce),
        aead::Aad::from(id.as_bytes()),
        &mut encrypted,
    )
    .map_err(|_| "无法加密密钥")?;
    let mut ciphertext = nonce.to_vec();
    ciphertext.extend(encrypted);
    sqlx::query("INSERT INTO editor_provider_keys(provider,ciphertext,updated_at) VALUES(?,?,?) ON CONFLICT(provider) DO UPDATE SET ciphertext=excluded.ciphertext,updated_at=excluded.updated_at")
        .bind(id).bind(ciphertext).bind(super::now_ms()).execute(pool).await.map_err(|_|"无法保存密钥")?;
    list(pool).await
}
pub async fn remove(pool: &ConnectionPool, id: &str) -> Result<Vec<Value>, String> {
    provider(id)?;
    sqlx::query("DELETE FROM editor_provider_keys WHERE provider=?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|_| "无法移除密钥")?;
    list(pool).await
}
pub async fn get(pool: &ConnectionPool, root: &Path, id: &str) -> Result<Option<String>, String> {
    provider(id)?;
    let stored: Option<Vec<u8>> =
        sqlx::query_scalar("SELECT ciphertext FROM editor_provider_keys WHERE provider=?")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(|_| "无法读取密钥")?;
    let Some(mut data) = stored else {
        return Ok(None);
    };
    if data.len() < 28 {
        return Err("密钥记录损坏".into());
    }
    let key = master(pool, root).await?;
    let nonce: [u8; 12] = data[..12].try_into().unwrap();
    let plaintext = key
        .open_in_place(
            aead::Nonce::assume_unique_for_key(nonce),
            aead::Aad::from(id.as_bytes()),
            &mut data[12..],
        )
        .map_err(|_| "密钥解密失败，请检查服务端加密主密钥")?;
    String::from_utf8(plaintext.to_vec())
        .map(Some)
        .map_err(|_| "密钥记录无效".into())
}
