use super::super::{self as editor, Error};
use super::{
    plan,
    service::{Access, RunContext, Service},
    store,
};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;
fn invalid(text: impl Into<String>) -> Error {
    Error::Invalid(text.into())
}
pub fn add(data: &mut Value, card: Value, text: &str) {
    data["items"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"card","text":text,"card":card}));
}

impl Service {
    pub(super) async fn card_tool(
        &self,
        scope: super::Turn<'_>,
        context: &mut RunContext,
        name: &str,
        args: &Value,
        data: &mut Value,
    ) -> Result<Value, Error> {
        let super::Turn {
            project,
            id: turn,
            cancel,
        } = scope;
        plan::validate(name, args)?;
        match name {
            "plan_read" => plan::read(&self.pool, project).await,
            "ask_user" => {
                let question = plan::question(args)?;
                let response = self
                    .wait_approval(
                        project,
                        turn,
                        json!({"text":question["question"],"question":question}),
                        900,
                        cancel,
                    )
                    .await?;
                let answer = response.filter(|r| r.allow).and_then(|r| r.answer);
                let text = question["question"].as_str().unwrap();
                let result = if let Some(answer) = answer {
                    add(
                        data,
                        json!({"type":"question","question":question,"answer":answer,"outcome":"answered"}),
                        text,
                    );
                    json!({"answered":true,"answer":answer})
                } else {
                    add(
                        data,
                        json!({"type":"question","question":question,"outcome":"skipped"}),
                        text,
                    );
                    json!({"answered":false,"message":"没有获得回答；这不是批准。沿用已知信息，或说明仍缺少什么；不要重复提问或擅自补全重要约束。"})
                };
                store::update(&self.pool, turn, data).await?;
                Ok(result)
            }
            "brief_write" => {
                let mut current = plan::read(&self.pool, project).await?;
                let version = current["revision"].as_i64().unwrap();
                let brief = plan::brief(args, "agent")?;
                current["brief"] = brief.clone();
                plan::reset_storyboard(&mut current);
                let saved =
                    plan::write(&self.pool, project, version, current, Some(turn), None).await?;
                context.planning = true;
                add(
                    data,
                    json!({"type":"brief","brief":brief}),
                    brief["film"].as_str().unwrap_or("影片简报"),
                );
                Ok(json!({"saved":true,"plan":saved,"next":"提出分镜并获得确认后再构建影片。"}))
            }
            "storyboard_propose" => {
                self.storyboard(project, turn, context, args, cancel, data)
                    .await
            }
            _ => Err(invalid("计划工具不存在")),
        }
    }
    async fn storyboard(
        &self,
        project: i64,
        turn: &str,
        context: &mut RunContext,
        args: &Value,
        cancel: &CancellationToken,
        data: &mut Value,
    ) -> Result<Value, Error> {
        let mut current = plan::read(&self.pool, project).await?;
        let version = current["revision"].as_i64().unwrap();
        let self_approve = args["approve"] == "self";
        if self_approve
            && (context.access != Access::Auto
                || !context.delegate_plan
                || context.sent_back
                || current["storyboard"]["status"] == "declined")
        {
            return Err(invalid(
                "自主确认分镜需用户明确委托且开启自动模式；被退回的分镜仍需用户确认",
            ));
        }
        let mut beats = plan::beats(&args["beats"], None)?;
        let saved = editor::get(&self.pool, project).await?;
        let revision = saved.summary.revision;
        let mut rendered = 0;
        let mut omitted = vec![];
        let mut frames = editor::export::frames::Session::new(cancel).map_err(invalid)?;
        context.planning = true;
        for (i, beat) in beats.as_array_mut().unwrap().iter_mut().enumerate() {
            let Some(still) = beat.as_object_mut().unwrap().remove("still") else {
                continue;
            };
            if rendered == 4 {
                omitted.push(i + 1);
                continue;
            }
            let mut document = saved.document.clone();
            let frame = if let Some(frame) = still.as_u64() {
                frame
            } else {
                let source = still["graphic"]
                    .as_str()
                    .ok_or_else(|| invalid("图形来源无效"))?;
                let source = if source.starts_with("graphics/") {
                    source.to_string()
                } else {
                    format!("graphics/{source}")
                };
                editor::graphics::read(&self.pool, project, &source).await?;
                let at = (still["t"].as_f64().unwrap_or(0.)
                    * document.project["canvas"]["fps"].as_f64().unwrap_or(30.))
                .round();
                if at > 86_400. * 240. {
                    return Err(invalid("分镜画面时间过大"));
                }
                document.project["tracks"] = json!([{"id":"still","kind":"video","name":"分镜画面","items":[{"id":"still-graphic","type":"graphic","src":source,"start":0,"duration":at as u64+1}]}]);
                document.project["markers"] = json!([]);
                at as u64
            };
            let jpeg = frames
                .jpeg(&self.pool, &self.root, project, document, frame, 540)
                .await
                .map_err(invalid)?;
            let id = editor::generate::random_id().map_err(invalid)?;
            let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
            let active:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM editor_agent_turns WHERE id=? AND project_id=? AND status='running')").bind(turn).bind(project).fetch_one(&mut *tx).await?;
            if !active || cancel.is_cancelled() {
                return Err(invalid("助手已停止"));
            }
            let size: i64 = sqlx::query_scalar(
                "SELECT COALESCE(SUM(length(jpeg)),0) FROM editor_agent_images WHERE project_id=?",
            )
            .bind(project)
            .fetch_one(&mut *tx)
            .await?;
            let count: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM editor_agent_images WHERE turn_id=?")
                    .bind(turn)
                    .fetch_one(&mut *tx)
                    .await?;
            if size + jpeg.len() as i64 > 256 * 1024 * 1024 || count >= 64 {
                return Err(invalid("分镜图片已达到工程或本轮上限"));
            }
            sqlx::query("INSERT INTO editor_agent_images(id,project_id,turn_id,revision,jpeg,created_at) VALUES(?,?,?,?,?,?)").bind(&id).bind(project).bind(turn).bind(revision).bind(jpeg).bind(editor::now_ms()).execute(&mut *tx).await?;
            tx.commit().await?;
            beat["still"] = json!(id);
            rendered += 1;
        }
        let automatic = self_approve
            || context.access == Access::Auto
                && context.skip_storyboard
                && !context.sent_back
                && current["storyboard"]["status"] != "declined";
        let mut storyboard = json!({"summary":args["summary"],"beats":beats,"status":if automatic{"approved"}else{"proposed"},"updatedAt":store::timestamp(editor::now_ms()),"projectRevision":revision});
        if automatic {
            storyboard["approvedBy"] = json!(if self_approve { "agent" } else { "setting" });
        }
        current["storyboard"] = storyboard.clone();
        let mut current = plan::write(
            &self.pool,
            project,
            version,
            current,
            Some(turn),
            Some(revision),
        )
        .await?;
        let title = args["summary"].as_str().unwrap_or("分镜");
        if automatic {
            add(
                data,
                json!({"type":"storyboard","storyboard":storyboard,"outcome":if self_approve{"self"}else{"auto"}}),
                title,
            );
            return Ok(json!({"approved":true,"storyboard":storyboard,"omittedStills":omitted}));
        }
        let version = current["revision"].as_i64().unwrap();
        let response = self
            .wait_approval(
                project,
                turn,
                json!({"text":title,"storyboard":storyboard}),
                900,
                cancel,
            )
            .await?;
        if cancel.is_cancelled() {
            return Err(invalid("助手已停止；分镜保留为待确认"));
        }
        let outcome = if let Some(response) = response {
            if response.allow {
                if let Some(beats) = response.beats {
                    storyboard["beats"] = beats;
                }
                storyboard["status"] = json!("approved");
                storyboard["approvedBy"] = json!("user");
                "approved"
            } else {
                context.sent_back = true;
                storyboard["status"] = json!("declined");
                storyboard["note"] = json!(response.note.unwrap_or_default());
                "declined"
            }
        } else {
            "unanswered"
        };
        storyboard["updatedAt"] = json!(store::timestamp(editor::now_ms()));
        current["storyboard"] = storyboard.clone();
        // Do not approve stills from a revision that changed during the user's review.
        plan::write(
            &self.pool,
            project,
            version,
            current,
            Some(turn),
            if outcome == "approved" {
                Some(revision)
            } else {
                None
            },
        )
        .await?;
        add(
            data,
            json!({"type":"storyboard","storyboard":storyboard,"outcome":outcome}),
            title,
        );
        Ok(
            json!({"approved":outcome=="approved","storyboard":storyboard,"omittedStills":omitted,"next":if outcome=="approved"{"按确认的分镜构建影片"}else{"分镜未确认；不要构建影片或把没有回答视为批准"}}),
        )
    }
}
