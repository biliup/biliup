//! Visible conversation history and the journal committed atomically with each project revision.
use super::super::{self as editor, ConnectionPool, Error};
use serde_json::{Value, json};
use sqlx::{Row, Sqlite, Transaction};

pub fn timestamp(ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ms)
        .unwrap_or_default()
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
pub async fn initialize(pool: &ConnectionPool) -> Result<(), sqlx::Error> {
    let now = editor::now_ms();
    sqlx::query("UPDATE editor_agent_turns SET status='error',approval_json=NULL,data_json=json_set(data_json,'$.error','服务重启，助手已停止；已保存的编辑仍然保留','$.finishedAt',?),updated_at=? WHERE status='running'")
        .bind(timestamp(now)).bind(now).execute(pool).await?;
    Ok(())
}
pub async fn state(
    pool: &ConnectionPool,
    project: i64,
    before: Option<i64>,
) -> Result<Value, Error> {
    state_page(pool, project, before, None).await
}
pub async fn state_page(
    pool: &ConnectionPool,
    project: i64,
    before: Option<i64>,
    before_id: Option<&str>,
) -> Result<Value, Error> {
    if before.is_some_and(|n| n < 0)
        || before_id.is_some_and(|id| before.is_none() || id.is_empty() || id.len() > 100)
    {
        return Err(Error::Invalid("助手历史游标无效".into()));
    }
    let saved = editor::get(pool, project).await?;
    let (content_revision, lineage) =
        editor::history::cursor::view(pool, project, saved.summary.revision).await?;
    let active: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM editor_agent_turns WHERE project_id=? AND status='running')",
    )
    .bind(project)
    .fetch_one(pool)
    .await?;
    let thread: Option<i64> =
        sqlx::query_scalar("SELECT started_at FROM editor_agent_threads WHERE project_id=?")
            .bind(project)
            .fetch_optional(pool)
            .await?;
    let mut rows = sqlx::query("SELECT * FROM editor_agent_turns WHERE project_id=? AND (? IS NULL OR created_at<? OR (created_at=? AND id<?)) ORDER BY created_at DESC,id DESC LIMIT 31")
        .bind(project).bind(before).bind(before).bind(before).bind(before_id).fetch_all(pool).await?;
    let more = rows.len() > 30;
    rows.truncate(30);
    let mut turns = vec![];
    let mut approval = Value::Null;
    let mut running = Value::Null;
    let mut ui = Value::Null;
    for row in rows.iter().rev() {
        let id: String = row.try_get("id")?;
        let status: String = row.try_get("status")?;
        let mut data: Value = serde_json::from_str(row.try_get::<&str, _>("data_json")?)?;
        data["id"] = json!(id);
        data["status"] = json!(status);
        let files=sqlx::query("SELECT id,revision,name,format,cues FROM editor_agent_files WHERE turn_id=? AND project_id=? ORDER BY created_at,id").bind(&id).bind(project).fetch_all(pool).await?;
        data["files"]=Value::Array(files.iter().map(|f|Ok(json!({"id":f.try_get::<String,_>("id")?,"projectId":project,"revision":f.try_get::<i64,_>("revision")?,"name":f.try_get::<String,_>("name")?,"format":f.try_get::<String,_>("format")?,"cues":f.try_get::<i64,_>("cues")?}))).collect::<Result<Vec<_>,Error>>()?);
        let exports=sqlx::query("SELECT id,revision,job_json FROM editor_exports WHERE agent_turn_id=? AND project_id=? AND phase='done' ORDER BY id LIMIT 3").bind(&id).bind(project).fetch_all(pool).await?;
        for row in exports {
            let job: i64 = row.try_get("id")?;
            let revision: i64 = row.try_get("revision")?;
            let output: Value = serde_json::from_str(row.try_get("job_json")?)?;
            let files = data["files"].as_array_mut().unwrap();
            files.push(json!({"id":format!("export-{job}"),"exportJob":job,"projectId":project,"revision":revision,"name":output["output"],"format":"mp4","cues":0}));
            if let Some(name) = output["subtitlesFile"].as_str() {
                files.push(json!({"id":format!("export-{job}-subtitles"),"exportJob":job,"projectId":project,"revision":revision,"name":name,"format":if name.ends_with(".vtt"){"vtt"}else{"srt"},"cues":0}));
            }
        }
        if status != "running"
            && let Some(items) = data["items"].as_array_mut()
        {
            for item in items {
                if item["status"] == "running" {
                    item["status"] = json!("error");
                    item["error"] = json!("操作已中断");
                }
            }
        }
        let edits = sqlx::query("SELECT revision,summary,lines_json,created_at FROM editor_agent_edits WHERE turn_id=? ORDER BY revision")
            .bind(&id).fetch_all(pool).await?;
        let steps = data["items"].as_array();
        data["edits"] = Value::Array(edits.iter().map(|e| {
            let revision=e.try_get::<i64,_>("revision")?;
            let step=steps.and_then(|items|items.iter().enumerate().find(|(_,item)|item["revision"].as_i64()==Some(revision)));
            let mut edit=json!({"revision":revision,"summary":e.try_get::<String,_>("summary")?,"lines":serde_json::from_str::<Value>(e.try_get::<&str,_>("lines_json")?)?});
            if let Some((index,item))=step{edit["at"]=json!(index);if item["importedMedia"].is_array(){edit["media"]=item["importedMedia"].clone();}}
            Ok(edit)
        }).collect::<Result<Vec<_>,Error>>()?);
        let undone: Option<i64> = row.try_get("undone_at")?;
        let last: Option<i64> = row.try_get("last_revision")?;
        let represented = edits
            .iter()
            .any(|edit| lineage.contains(&edit.get::<i64, _>("revision")));
        if (represented || last.is_none()) && data["ui"].is_object() {
            ui = data["ui"].clone();
        }
        let eligible = represented
            && !row.try_get::<bool, _>("undo_blocked")?
            && status != "running"
            && last == Some(content_revision);
        data["undoRevision"] = if eligible {
            json!(saved.summary.revision)
        } else {
            Value::Null
        };
        data["undo"] = if last.is_none() {
            json!({"state":"none"})
        } else if !represented {
            if let Some(at) = undone {
                data["undoneAt"] = json!(timestamp(at));
            }
            json!({"state":"undone"})
        } else if active {
            json!({"state":"blocked","reason":"请先停止或等待本轮完成"})
        } else if !eligible {
            json!({"state":"blocked","reason":"工程有后续或交错编辑，请在历史中查看版本"})
        } else {
            json!({"state":"available"})
        };
        if status == "running" {
            running = json!(id);
            if let Some(text) = row.try_get::<Option<String>, _>("approval_json")? {
                approval = serde_json::from_str(&text)?;
            }
        }
        turns.push(data);
    }
    let model = editor::text_model::get(pool)
        .await
        .map_err(Error::Invalid)?;
    let configured = editor::keys::list(pool)
        .await
        .map_err(Error::Invalid)?
        .iter()
        .any(|key| key["provider"] == model.provider && key["set"] == true);
    let screen_take:Option<String>=sqlx::query_scalar("SELECT d.id FROM editor_downloads d JOIN editor_agent_turns t ON t.id=d.agent_turn_id WHERE d.project_id=? AND t.status='running' AND d.phase IN ('capture-waiting','capture-recording','capture-uploading') ORDER BY d.created_at DESC LIMIT 1").bind(project).fetch_optional(pool).await?;
    let cursor = if more {
        rows.last()
            .map(|r| json!({"createdAt":r.get::<i64,_>("created_at"),"id":r.get::<String,_>("id")}))
    } else {
        None
    };
    Ok(
        json!({"dir":format!("biliup:{project}"),"provider":model.provider,"model":model.model,"configured":configured,"projectRevision":saved.summary.revision,"historyBranch":saved.history.branch_revision,"shaderVersion":editor::shaders::version(pool,project).await?,"resourceVersion":sqlx::query_scalar::<_,i64>("SELECT version FROM editor_resource_state WHERE id=1").fetch_one(pool).await?,"running":running,"screenTake":screen_take,"turns":turns,"ui":ui,"approval":approval,"plan":super::plan::read(pool,project).await?,"threadStart":thread.map(timestamp),"historyCursor":cursor,"before":rows.last().map(|r| r.get::<i64,_>("created_at"))}),
    )
}
pub async fn update(pool: &ConnectionPool, turn: &str, data: &Value) -> Result<(), Error> {
    let text = serde_json::to_string(data)?;
    if text.len() > 2 * 1024 * 1024 {
        return Err(Error::Invalid("助手会话已达到长度限制".into()));
    }
    sqlx::query("UPDATE editor_agent_turns SET data_json=?,updated_at=? WHERE id=?")
        .bind(text)
        .bind(editor::now_ms())
        .bind(turn)
        .execute(pool)
        .await?;
    Ok(())
}

pub enum Journal<'a> {
    User(&'a editor::history::UserNote),
    Move {
        action: editor::history::cursor::Action,
        target: i64,
    },
    Edit {
        turn: &'a str,
        summary: &'a str,
        lines: &'a [String],
        plan_revision: i64,
    },
    Undo {
        turn: &'a str,
    },
}
/// Called inside editor::save's IMMEDIATE transaction. A cancelled or stale turn cannot commit.
pub async fn commit(
    tx: &mut Transaction<'_, Sqlite>,
    project: i64,
    previous: i64,
    revision: i64,
    journal: Journal<'_>,
    now: i64,
) -> Result<(), Error> {
    match journal {
        Journal::Move { action, target } => {
            editor::history::write(
                tx,
                project,
                revision,
                editor::history::Note {
                    kind: action.kind(),
                    author: "user",
                    label: &action.label(target),
                    lines: &[],
                    turn: None,
                },
            )
            .await?;
        }
        Journal::User(note) => {
            // A multiline clip name in a summary must never make the actual edit fail.
            let label = note
                .label
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .chars()
                .take(500)
                .collect::<String>();
            let label = if label.is_empty() {
                "编辑时间轴"
            } else {
                &label
            };
            editor::history::write(
                tx,
                project,
                revision,
                editor::history::Note {
                    kind: match note.kind {
                        editor::history::UserKind::Edit => "edit",
                        editor::history::UserKind::Jump => "jump",
                    },
                    author: "user",
                    label,
                    lines: &[],
                    turn: None,
                },
            )
            .await?;
        }
        Journal::Edit {
            turn,
            summary,
            lines,
            plan_revision,
        } => {
            let current: Option<i64> =
                sqlx::query_scalar("SELECT revision FROM editor_agent_plans WHERE project_id=?")
                    .bind(project)
                    .fetch_optional(&mut **tx)
                    .await?;
            if current.unwrap_or(0) != plan_revision {
                return Err(Error::Conflict);
            }
            let row = sqlx::query("SELECT status,base_revision,last_revision FROM editor_agent_turns WHERE id=? AND project_id=?")
                .bind(turn).bind(project).fetch_optional(&mut **tx).await?.ok_or(Error::NotFound)?;
            if row.try_get::<&str, _>("status")? != "running" {
                return Err(Error::Invalid("助手已停止，编辑未写入".into()));
            }
            let expected = row
                .try_get::<Option<i64>, _>("last_revision")?
                .unwrap_or(row.try_get("base_revision")?);
            sqlx::query("INSERT INTO editor_agent_edits(turn_id,revision,summary,lines_json,created_at) VALUES(?,?,?,?,?)")
                .bind(turn).bind(revision).bind(summary).bind(serde_json::to_string(lines)?).bind(now).execute(&mut **tx).await?;
            editor::history::write(
                tx,
                project,
                revision,
                editor::history::Note {
                    kind: "edit",
                    author: "agent",
                    label: summary,
                    lines,
                    turn: Some(turn),
                },
            )
            .await?;
            sqlx::query("UPDATE editor_agent_turns SET last_revision=?,undo_blocked=MAX(undo_blocked,?),updated_at=? WHERE id=?")
                .bind(revision).bind(expected != previous).bind(now).bind(turn).execute(&mut **tx).await?;
        }
        Journal::Undo { turn } => {
            let content_revision = editor::history::cursor::resolve(tx, project, previous).await?;
            let result = sqlx::query("UPDATE editor_agent_turns SET undone_at=?,updated_at=? WHERE id=? AND project_id=? AND status!='running' AND undo_blocked=0 AND last_revision=? AND NOT EXISTS(SELECT 1 FROM editor_agent_turns WHERE project_id=? AND status='running')")
                .bind(now).bind(now).bind(turn).bind(project).bind(content_revision).bind(project).execute(&mut **tx).await?;
            if result.rows_affected() != 1 {
                return Err(Error::Conflict);
            }
            editor::history::write(
                tx,
                project,
                revision,
                editor::history::Note {
                    kind: "undo-turn",
                    author: "user",
                    label: "撤销助手本轮编辑",
                    lines: &[],
                    turn: Some(turn),
                },
            )
            .await?;
        }
    }
    Ok(())
}
pub async fn undo(pool: &ConnectionPool, project: i64, turn: &str) -> Result<editor::Saved, Error> {
    let row = sqlx::query(
        "SELECT base_revision,last_revision FROM editor_agent_turns WHERE project_id=? AND id=?",
    )
    .bind(project)
    .bind(turn)
    .fetch_optional(pool)
    .await?
    .ok_or(Error::NotFound)?;
    let previous: i64 = row
        .try_get::<Option<i64>, _>("last_revision")?
        .ok_or_else(|| Error::Invalid("本轮没有可撤销的编辑".into()))?;
    let current = editor::get(pool, project).await?;
    let (content_revision, _) =
        editor::history::cursor::view(pool, project, current.summary.revision).await?;
    if content_revision != previous {
        return Err(Error::Conflict);
    }
    let document = editor::history::cursor::restore_document(
        &current.document,
        editor::revision(pool, project, row.try_get("base_revision")?).await?,
    );
    editor::save_agent(
        pool,
        (project, current.summary.revision),
        document,
        Journal::Undo { turn },
    )
    .await
}
