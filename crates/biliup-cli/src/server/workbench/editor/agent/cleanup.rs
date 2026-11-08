//! Turn-end housekeeping adapted from Uncut agentCleanup.ts / tools/generate.ts (MIT).
//! Immutable graphics are retained by the project's revision history; only empty tracks are removed.
use super::{plan, planner, service::{Access, RunContext, Service}, store};
use super::super::{self as editor, Error};
use serde_json::{Value, json};
use std::collections::HashSet;
use tokio_util::sync::CancellationToken;

impl Service {
    pub(super) async fn cleanup_turn(
        &self, project:i64, turn:&str, context:&RunContext,
        cancel:&CancellationToken, data:&mut Value,
    )->Result<(),Error>{
        if context.access==Access::Look || cancel.is_cancelled(){return Ok(())}
        // Read the actual committed revisions, including caption and generated tracks. An
        // attempted or rejected add_track is never treated as a committed addition.
        let added:HashSet<String>=sqlx::query_scalar::<_,String>(
            "SELECT DISTINCT json_extract(t.value,'$.id') FROM editor_agent_edits e \
             JOIN editor_agent_turns a ON a.id=e.turn_id \
             JOIN editor_revisions r ON r.project_id=a.project_id AND r.revision=e.revision \
             JOIN editor_revisions b ON b.project_id=a.project_id AND b.revision=e.revision-1 \
             JOIN editor_revisions original ON original.project_id=a.project_id AND original.revision=a.base_revision, \
             json_each(r.document_json,'$.project.tracks') t \
             WHERE a.id=? AND a.project_id=? AND a.status='running' \
             AND json_type(t.value,'$.id')='text' \
             AND NOT EXISTS(SELECT 1 FROM json_each(b.document_json,'$.project.tracks') old WHERE json_extract(old.value,'$.id')=json_extract(t.value,'$.id')) \
             AND NOT EXISTS(SELECT 1 FROM json_each(original.document_json,'$.project.tracks') old WHERE json_extract(old.value,'$.id')=json_extract(t.value,'$.id'))"
        ).bind(turn).bind(project).fetch_all(&self.pool).await?.into_iter().collect();
        if added.is_empty(){return Ok(())}
        loop {
            if cancel.is_cancelled(){return Ok(())}
            let saved=editor::get(&self.pool,project).await?;
            // Leave interleaved user edits alone. The save also checks this exact revision,
            // so an edit arriving after this read cannot be silently overwritten.
            let owns_head:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM editor_agent_turns WHERE id=? AND project_id=? AND status='running' AND undo_blocked=0 AND last_revision=?)")
                .bind(turn).bind(project).bind(saved.summary.revision).fetch_one(&self.pool).await?;
            if !owns_head{return Ok(())}
            let tracks:Vec<String>=saved.document.project["tracks"].as_array().into_iter().flatten()
                .filter(|track|track["id"].as_str().is_some_and(|id|added.contains(id))
                    && track["role"]!="main" && track["locked"]!=true
                    && track["items"].as_array().is_some_and(Vec::is_empty))
                .filter_map(|track|track["id"].as_str().map(str::to_owned)).take(500).collect();
            if tracks.is_empty(){return Ok(())}
            let current_plan=plan::read(&self.pool,project).await?;
            let summary=format!("移除 {} 条未使用的空轨道",tracks.len());
            let ops:Vec<Value>=tracks.iter().map(|id|json!({"op":"delete_track","track":id})).collect();
            let planned=planner::edit(&self.pool,project,saved.summary.revision,turn,&json!({"summary":summary,"ops":ops}),cancel).await?;
            if cancel.is_cancelled(){return Ok(())}
            editor::save_agent(&self.pool,(project,planned.base_revision),planned.document,store::Journal::Edit{
                turn,summary:&planned.summary,lines:&planned.lines,
                plan_revision:current_plan["revision"].as_i64().unwrap_or(0),
            }).await?;
            let mut removed:Vec<String>=data["cleanup"]["tracks"].as_array().into_iter().flatten().filter_map(|id|id.as_str().map(str::to_owned)).collect();
            removed.extend(tracks);
            data["cleanup"]=json!({"tracks":removed});
            store::update(&self.pool,turn,data).await?;
        }
    }
}
