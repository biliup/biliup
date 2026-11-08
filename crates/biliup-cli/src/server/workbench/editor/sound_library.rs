//! Browser-selected sound folders refer only to managed assets, never to client/server paths.
use super::{ConnectionPool, Error, FileSource, Result, invalid};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::Row;
use std::path::PathBuf;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Sound {
    asset_id: String,
    name: String,
    rel: String,
    bytes: i64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Folder {
    id: String,
    name: String,
    sounds: Vec<Sound>,
    version: Option<i64>,
}
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
}
async fn project(pool: &ConnectionPool, id: i64) -> Result<()> {
    if sqlx::query_scalar::<_, i64>("SELECT id FROM editor_projects WHERE id=?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .is_none()
    {
        return Err(Error::NotFound);
    }
    Ok(())
}
pub async fn list(pool: &ConnectionPool, id: i64) -> Result<Vec<Value>> {
    project(pool, id).await?;
    let folders = sqlx::query("SELECT id,name,updated_at FROM editor_sound_folders ORDER BY updated_at DESC,id")
        .fetch_all(pool)
        .await?;
    let files = sqlx::query("SELECT f.folder_id,f.asset_id,f.name,f.relative_name,f.bytes,a.duration_ms FROM editor_sound_files f JOIN editor_assets a ON a.id=f.asset_id ORDER BY f.relative_name").fetch_all(pool).await?;
    folders.into_iter().map(|folder| {
        let id: String = folder.try_get("id")?;
        let name: String = folder.try_get("name")?;
        let sounds: Vec<Value> = files.iter().filter(|file| file.get::<String,_>("folder_id") == id).map(|file| {
            let asset: String = file.get("asset_id");
            let rel: String = file.get("relative_name");
            let mut sound=json!({"id":format!("{id}:{asset}"),"folder":id,"name":file.get::<String,_>("name"),"ext":rel.rsplit('.').next().unwrap_or(""),"rel":rel,"bytes":file.get::<i64,_>("bytes")});
            if let Some(ms)=file.get::<Option<i64>,_>("duration_ms"){sound["seconds"]=json!(ms as f64/1000.);}
            sound
        }).collect();
        Ok(json!({"id":id,"name":name,"version":folder.get::<i64,_>("updated_at"),"sounds":sounds}))
    }).collect()
}
pub async fn save(pool: &ConnectionPool, project_id: i64, folder: Folder) -> Result<Vec<Value>> {
    project(pool, project_id).await?;
    if !valid_id(&folder.id)
        || folder.name.trim().is_empty()
        || folder.name.len() > 512
        || folder.sounds.is_empty()
        || folder.sounds.len() > 500
        || folder.version.is_some_and(|version| version < 0)
    {
        return Err(invalid("音频文件夹内容无效"));
    }
    let mut tx = pool.begin().await?;
    // A new upload never replaces another folder just because its name matches.
    // Updates name the chosen folder and the version shown before the browser picker opened.
    let changed = if let Some(version) = folder.version {
        sqlx::query("UPDATE editor_sound_folders SET name=?,updated_at=MAX(updated_at+1,?) WHERE id=? AND updated_at=?")
            .bind(&folder.name).bind(super::now_ms()).bind(&folder.id).bind(version)
            .execute(&mut *tx).await?.rows_affected()
    } else {
        sqlx::query("INSERT INTO editor_sound_folders(id,name,updated_at) VALUES(?,?,?) ON CONFLICT(id) DO NOTHING")
            .bind(&folder.id).bind(&folder.name).bind(super::now_ms())
            .execute(&mut *tx).await?.rows_affected()
    };
    if changed != 1 {
        return Err(Error::Conflict);
    }
    sqlx::query("DELETE FROM editor_sound_files WHERE folder_id=?")
        .bind(&folder.id)
        .execute(&mut *tx)
        .await?;
    for sound in folder.sounds {
        if sound.name.is_empty()
            || sound.name.len() > 512
            || sound.rel.len() > 2048
            || sound.bytes <= 0
            || sound.bytes > 20 * 1024 * 1024 * 1024
        {
            return Err(invalid("音频文件信息无效"));
        }
        let audio = sqlx::query_scalar::<_, String>("SELECT kind FROM editor_assets WHERE id=?")
            .bind(&sound.asset_id)
            .fetch_optional(&mut *tx)
            .await?;
        if audio.as_deref() != Some("audio") {
            return Err(invalid("文件夹只能引用已导入的音频"));
        }
        sqlx::query("INSERT OR IGNORE INTO editor_sound_files(folder_id,asset_id,name,relative_name,bytes) VALUES(?,?,?,?,?,?)")
            .bind(&folder.id).bind(sound.asset_id).bind(sound.name).bind(sound.rel).bind(sound.bytes).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    list(pool, project_id).await
}
pub async fn delete(pool: &ConnectionPool, project_id: i64, id: &str) -> Result<Vec<Value>> {
    project(pool, project_id).await?;
    sqlx::query("DELETE FROM editor_sound_folders WHERE id=?")
        .bind(id)
        .execute(pool)
        .await?;
    list(pool, project_id).await
}
pub async fn sound(pool: &ConnectionPool, project_id: i64, id: &str) -> Result<(Value, PathBuf)> {
    let project = super::get(pool, project_id).await?;
    let (folder, asset) = id.split_once(':').ok_or_else(|| invalid("音频标识无效"))?;
    let row=sqlx::query("SELECT a.path,a.file_name,a.duration_ms,a.probe_json,f.name FROM editor_sound_files f JOIN editor_assets a ON a.id=f.asset_id WHERE f.folder_id=? AND f.asset_id=? AND a.kind='audio'")
        .bind(folder).bind(asset).fetch_optional(pool).await?.ok_or(Error::NotFound)?;
    let path = PathBuf::from(row.try_get::<String, _>("path")?);
    let ms: Option<i64> = row.try_get("duration_ms")?;
    let mut probe: Value = serde_json::from_str(row.try_get::<&str, _>("probe_json")?)?;
    let fps = project.document.project["canvas"]["fps"]
        .as_f64()
        .unwrap_or(30.);
    probe["frames"] = json!((ms.unwrap_or(0) as f64 * fps / 1000.).floor().max(1.) as i64);
    let media_id = format!("file-{asset}");
    Ok((
        json!({"media":{"id":media_id,"path":format!("imports/{media_id}"),"name":row.try_get::<String,_>("name")?,"kind":"audio","probe":probe},
        "source":FileSource{kind:"file".into(),asset_id:asset.into(),duration_ms:ms,file_name:row.try_get("file_name")?}}),
        path,
    ))
}
