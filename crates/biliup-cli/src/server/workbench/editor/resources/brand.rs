//! One brand/skill view for the assistant and persisted resource APIs.
pub struct Update<'a> {
    pub revision: i64,
    pub expected: &'a str,
    pub kit: Option<Value>,
    pub active: Option<Value>,
}

use super::{ConnectionPool, Error, Result, invalid};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{Row, SqliteConnection};
use std::collections::BTreeMap;
use tokio_util::sync::CancellationToken;

async fn rows(conn: &mut SqliteConnection, project: i64) -> Result<Vec<Value>> {
    let result=sqlx::query("SELECT scope,kind,id,value_json FROM editor_resources WHERE kind IN ('kit','skill','active') AND (project_id IS NULL OR project_id=?) ORDER BY scope='global' DESC,scope,kind,id")
        .bind(project).fetch_all(&mut *conn).await?.into_iter().map(|row|Ok(json!({"scope":row.try_get::<String,_>("scope")?,"kind":row.try_get::<String,_>("kind")?,"id":row.try_get::<String,_>("id")?,"raw":row.try_get::<String,_>("value_json")?}))).collect::<Result<Vec<_>>>()?;
    Ok(result)
}
fn stamp(rows: &[Value]) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(rows).expect("resource rows"))
    )
}
pub struct Snapshot {
    pub state: Value,
    pub stamp: String,
}
pub async fn snapshot(pool: &ConnectionPool, project: i64) -> Result<Snapshot> {
    super::exists(pool, project).await?;
    let mut tx = pool.begin().await?;
    let rows = rows(&mut tx, project).await?;
    let assets=sqlx::query("SELECT scope,kit,name FROM editor_brand_files WHERE project_id IS NULL OR project_id=? ORDER BY scope='global' DESC,scope,kit,name").bind(project).fetch_all(&mut *tx).await?;
    tx.commit().await?;
    let mut kits = BTreeMap::<String, Value>::new();
    let mut skills: BTreeMap<String, Value> = super::skills::all()
        .iter()
        .map(|skill| (skill["id"].as_str().unwrap().to_string(), skill.clone()))
        .collect();
    let mut active = json!({"kit":null,"skills":[]});
    for row in &rows {
        let value: Value = serde_json::from_str(row["raw"].as_str().unwrap())?;
        let id = row["id"].as_str().unwrap().to_string();
        match row["kind"].as_str() {
            Some("kit") => {
                kits.insert(id, value);
            }
            Some("skill") => {
                skills.insert(id, value);
            }
            Some("active") => active = value,
            _ => {}
        }
    }
    if active["kit"]
        .as_str()
        .is_some_and(|id| !kits.contains_key(id))
    {
        active["kit"] = Value::Null;
    }
    if let Some(ids) = active["skills"].as_array_mut() {
        ids.retain(|id| id.as_str().is_some_and(|id| skills.contains_key(id)));
    }
    let mut files = json!({});
    for asset in assets {
        let kit: String = asset.try_get("kit")?;
        let scope: String = asset.try_get("scope")?;
        let name: String = asset.try_get("name")?;
        let scope = if scope == "global" {
            "global"
        } else {
            "project"
        };
        if kits.get(&kit).is_some_and(|k| k["scope"] == scope) {
            if files[&kit].is_null() {
                files[&kit] = json!({});
            }
            files[&kit][&name] = json!(format!(
                "/v1/editor/projects/{project}/brand/{scope}/{kit}/files/{}",
                urlencoding::encode(&name)
            ));
        }
    }
    Ok(Snapshot {
        stamp: stamp(&rows),
        state: json!({"kits":kits.into_values().collect::<Vec<_>>(),"skills":skills.into_values().collect::<Vec<_>>(),"active":active,"files":files}),
    })
}

pub async fn commit(
    pool: &ConnectionPool,
    project: i64,
    turn: &str,
    Update {
        revision,
        expected,
        kit,
        active,
    }: Update<'_>,

    cancel: &CancellationToken,
) -> Result<()> {
    let kit = kit
        .map(|k| {
            let id = k["id"]
                .as_str()
                .ok_or_else(|| invalid("品牌标识无效"))?
                .to_string();
            let target = k["scope"]
                .as_str()
                .ok_or_else(|| invalid("品牌范围无效"))?
                .to_string();
            Ok::<_, Error>((
                id.clone(),
                target.clone(),
                super::normalize("kit", &id, &target, k)?,
            ))
        })
        .transpose()?;
    let active = active
        .map(|a| super::normalize("active", "active", "project", a))
        .transpose()?;
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    let current: Option<i64> =
        sqlx::query_scalar("SELECT revision FROM editor_projects WHERE id=?")
            .bind(project)
            .fetch_optional(&mut *tx)
            .await?;
    if current != Some(revision) || stamp(&rows(&mut tx, project).await?) != expected {
        return Err(Error::Conflict);
    }
    let running:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM editor_agent_turns WHERE id=? AND project_id=? AND status='running')").bind(turn).bind(project).fetch_one(&mut *tx).await?;
    if !running || cancel.is_cancelled() {
        return Err(invalid("助手已停止"));
    }
    if let Some((id, target, kit)) = kit {
        let (scope, owner) = super::scope(project, &target)?;
        let refs = kit["logos"]
            .as_object()
            .into_iter()
            .flat_map(|o| o.values())
            .chain(
                kit["fonts"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|f| f.get("file")),
            )
            .chain(kit["examples"].as_array().into_iter().flatten());
        for name in refs.filter_map(Value::as_str) {
            let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM editor_brand_files WHERE scope=? AND kit=? AND name=?)").bind(&scope).bind(&id).bind(name).fetch_one(&mut *tx).await?;
            if !exists {
                return Err(invalid("品牌文件已改变，请重新读取品牌资源"));
            }
        }
        sqlx::query("INSERT INTO editor_resources(scope,project_id,kind,id,value_json,updated_at) VALUES(?,?,'kit',?,?,?) ON CONFLICT(scope,kind,id) DO UPDATE SET value_json=excluded.value_json,updated_at=excluded.updated_at")
            .bind(scope).bind(owner).bind(id).bind(kit.to_string()).bind(super::super::now_ms()).execute(&mut *tx).await?;
    }
    if let Some(active) = active {
        sqlx::query("INSERT INTO editor_resources(scope,project_id,kind,id,value_json,updated_at) VALUES(?,?,'active','active',?,?) ON CONFLICT(scope,kind,id) DO UPDATE SET value_json=excluded.value_json,updated_at=excluded.updated_at")
            .bind(project.to_string()).bind(project).bind(active.to_string()).bind(super::super::now_ms()).execute(&mut *tx).await?;
    }
    if cancel.is_cancelled() {
        return Err(invalid("助手已停止"));
    }
    tx.commit().await?;
    Ok(())
}
