//! Project effect templates. Applied clips retain a complete immutable definition in their document.
use super::{ConnectionPool, Error};
use serde_json::{Value, json};
use sqlx::Row;
pub async fn version(pool: &ConnectionPool, project: i64) -> Result<i64, Error> {
    Ok(sqlx::query_scalar(
        "SELECT COALESCE(SUM(revision),0) FROM editor_shaders WHERE project_id=?",
    )
    .bind(project)
    .fetch_one(pool)
    .await?)
}
pub async fn list(pool: &ConnectionPool, project: i64) -> Result<Value, Error> {
    super::get(pool, project).await?;
    let mut tx = pool.begin().await?;
    let rows = sqlx::query(
        "SELECT revision,definition_json,tested FROM editor_shaders WHERE project_id=? ORDER BY id",
    )
    .bind(project)
    .fetch_all(&mut *tx)
    .await?;
    let mut definitions = vec![];
    let mut revision = 0;
    for row in rows {
        revision += row.try_get::<i64, _>("revision")?;
        let mut def: Value = serde_json::from_str(row.try_get::<&str, _>("definition_json")?)?;
        def["tested"] = json!(row.try_get::<bool, _>("tested")?);
        definitions.push(def);
    }
    tx.commit().await?;
    Ok(json!({"revision":revision,"definitions":definitions}))
}
pub async fn definitions(pool: &ConnectionPool, project: i64) -> Result<Value, Error> {
    let mut definitions = Vec::new();
    definitions.extend(
        list(pool, project).await?["definitions"]
            .as_array()
            .cloned()
            .unwrap_or_default(),
    );
    Ok(Value::Array(definitions))
}
