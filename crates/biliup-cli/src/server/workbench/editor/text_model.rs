//! Portable model configuration shared by graphics generation and the editor assistant.
use super::ConnectionPool;
use serde::{Deserialize, Serialize};

pub(crate) mod conversation;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub provider: String,
    pub model: String,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            provider: "openai".into(),
            model: "gpt-6-astra".into(),
        }
    }
}
pub async fn get(pool: &ConnectionPool) -> Result<Settings, String> {
    let value: Option<String> =
        sqlx::query_scalar("SELECT settings_json FROM editor_model_settings WHERE id=1")
            .fetch_optional(pool)
            .await
            .map_err(|e| e.to_string())?;
    value
        .map(|v| serde_json::from_str(&v).map_err(|_| "模型设置无效".into()))
        .unwrap_or_else(|| Ok(Settings::default()))
}
pub async fn save(pool: &ConnectionPool, mut settings: Settings) -> Result<Settings, String> {
    settings.model = settings.model.trim().into();
    validate(&settings)?;
    sqlx::query("INSERT INTO editor_model_settings(id,settings_json,updated_at) VALUES(1,?,?) ON CONFLICT(id) DO UPDATE SET settings_json=excluded.settings_json,updated_at=excluded.updated_at")
        .bind(serde_json::to_string(&settings).map_err(|e|e.to_string())?).bind(super::now_ms()).execute(pool).await.map_err(|e|e.to_string())?;
    Ok(settings)
}
fn validate(settings: &Settings) -> Result<(), String> {
    if !["openai", "google"].contains(&settings.provider.as_str())
        || settings.model.is_empty()
        || settings.model.len() > 100
        || !settings
            .model
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    {
        return Err("模型服务或模型名称无效".into());
    }
    Ok(())
}
pub async fn ask(
    settings: &Settings,
    key: &str,
    instructions: &str,
    input: &str,
) -> Result<String, String> {
    let mut conversation = conversation::Conversation::new(settings.clone(), instructions, vec![])?;
    conversation.message("user", input, &[])?;
    Ok(conversation
        .next(key, &tokio_util::sync::CancellationToken::new())
        .await?
        .text)
}
