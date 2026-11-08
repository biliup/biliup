//! Independent critic: fresh provider conversation, service-rendered evidence and revision checks.
//! Review policy is adapted from Uncut; no desktop CLI or builder reasoning is used.
use super::super::{
    self as editor, ConnectionPool, Error,
    text_model::conversation::{Conversation, Image, Tool},
};
use super::{
    cards, plan,
    service::{RunContext, Service},
    store, views,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{path::Path, sync::LazyLock};
use tokio_util::sync::CancellationToken;
fn invalid(value: impl Into<String>) -> Error {
    Error::Invalid(value.into())
}
static SPEC: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../resources/editor/agent-review-tools.json"
    )))
    .expect("bundled review tool")
});
static VALIDATOR: LazyLock<jsonschema::Validator> = LazyLock::new(|| {
    jsonschema::validator_for(&SPEC["tools"][0]["inputSchema"]).expect("review schema")
});
pub fn tool() -> Tool {
    let t = &SPEC["tools"][0];
    Tool {
        name: "review_submit".into(),
        description: t["description"].as_str().unwrap().into(),
        input_schema: t["inputSchema"].clone(),
    }
}
#[derive(Default)]
pub(super) struct State {
    pub rounds: u64,
    pub attempts: u64,
    pub last_stamp: Option<String>,
}

/// Include non-timeline inputs too: a changed font, kit or skill invalidates old observations.
pub async fn environment(
    pool: &ConnectionPool,
    root: &Path,
    project: i64,
    document: &editor::Document,
) -> Result<(String, Value), Error> {
    let active = editor::resources::list(pool, project, "active").await?;
    let selected = active
        .iter()
        .find(|a| a["scope"] == "project")
        .map(|a| &a["value"]);
    let kit = selected.and_then(|a| a["kit"].as_str());
    let skill_ids = selected
        .and_then(|a| a["skills"].as_array())
        .cloned()
        .unwrap_or_default();
    let mut kits = editor::resources::list(pool, project, "kit").await?;
    kits.retain(|r| r["id"].as_str() == kit);
    let mut skills = editor::resources::list(pool, project, "skill").await?;
    skills.retain(|r| skill_ids.contains(&r["id"]));
    let snapshot = editor::fonts::snapshot(pool, root, project).await?;
    let frozen = serde_json::to_value(&snapshot)?;
    let assets = editor::fonts::materialize(root, &snapshot).await;
    let mut fonts = editor::fonts::bundled();
    fonts.extend(assets.fonts);
    let families: Vec<_> = serde_json::to_value(&fonts)?
        .as_array()
        .unwrap()
        .iter()
        .map(|f| json!({"family":f["family"]}))
        .collect();
    let graphics = editor::graphics::load(pool, project, &document.project).await?;
    let env = json!({"active":active,"kits":kits,"skills":skills,"fonts":families,"graphics":graphics,"warnings":assets.warnings,"snapshot":frozen});
    if serde_json::to_vec(&env)?.len() > 8 * 1024 * 1024 {
        return Err(invalid("审阅上下文超过 8 MiB"));
    }
    let stamp = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(
            &json!({"document":document,"environment":env})
        )?)
    );
    Ok((stamp, env))
}

async fn render(
    session: &mut editor::export::frames::Session,
    pool: &ConnectionPool,
    root: &Path,
    project: i64,
    document: &editor::Document,
    frames: &[u64],
    width: u32,
) -> Result<Vec<editor::export::frames::Picture>, Error> {
    let mut out = vec![];
    for chunk in frames.chunks(24) {
        out.extend(
            session
                .render(
                    pool,
                    root,
                    project,
                    editor::export::frames::FrameRequest {
                        document: document.clone(),
                        frames: chunk,
                        width,
                        motion_blur: false,
                        census: true,
                    },
                )
                .await
                .map_err(invalid)?,
        );
    }
    Ok(out)
}
fn picture(label: String, jpeg: &[u8]) -> Image {
    Image {
        name: label,
        mime_type: "image/jpeg".into(),
        data: STANDARD.encode(jpeg),
    }
}

impl Service {
    pub(super) async fn review_tool(
        &self,
        project: i64,
        turn: &str,
        context: &mut RunContext,
        args: &Value,
        cancel: &CancellationToken,
        data: &mut Value,
    ) -> Result<Value, Error> {
        if serde_json::to_vec(args)?.len() > 32 * 1024 {
            return Err(invalid("审阅参数过大"));
        }
        VALIDATOR
            .validate(args)
            .map_err(|e| invalid(format!("审阅参数无效：{e}")))?;
        if context.review.attempts >= 8 {
            return Err(invalid("本轮审阅请求已达到上限，请报告尚未解决的问题"));
        }
        let saved = editor::get(&self.pool, project).await?;
        let revision = saved.summary.revision;
        let (stamp, env) = environment(&self.pool, &self.root, project, &saved.document).await?;
        let mut plan = plan::read(&self.pool, project).await?;
        let expected = plan["revision"]
            .as_i64()
            .ok_or_else(|| invalid("计划版本无效"))?;
        let review_stamp = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&json!([
                stamp,
                plan["brief"],
                plan["storyboard"],
                plan["intended"]
            ]))?)
        );
        // Only actual human prompts from this thread; never pass assistant text or its tool history.
        let prompts:Vec<String>=sqlx::query_scalar("SELECT json_extract(data_json,'$.prompt') FROM editor_agent_turns WHERE project_id=? AND undone_at IS NULL AND created_at>=COALESCE((SELECT started_at FROM editor_agent_threads WHERE project_id=?),0) ORDER BY created_at DESC LIMIT 12").bind(project).bind(project).fetch_all(&self.pool).await?;
        let prompts = prompts.into_iter().rev().collect::<Vec<_>>();
        let request = prompts.join("\n\n");
        let input = json!({"turn":turn,"revision":revision,"rounds":context.review.rounds,"stamp":stamp,"reviewStamp":review_stamp,"lastStamp":context.review.last_stamp,"request":request,"prompts":prompts,"looked":{"revision":context.looked.revision,"stamp":context.looked.stamp,"sheet":context.looked.sheet,"phone":context.looked.phone,"frames":context.looked.frames},"args":args});
        let prepared = views::eval(
            &format!(
                "BiliupExport.agentReviewPlan({},{plan},{env},{input})",
                saved.document.project
            ),
            cancel,
        )
        .await?;
        let mut session = editor::export::frames::Session::new(cancel).map_err(invalid)?;
        let sheet_frames: Vec<u64> = serde_json::from_value(prepared["sheet"].clone())?;
        let phone_frames: Vec<u64> = serde_json::from_value(prepared["phones"].clone())?;
        let fixed: Vec<u64> = prepared["fixed"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|f| f["frame"].as_u64())
            .collect();
        let sheet = render(
            &mut session,
            &self.pool,
            &self.root,
            project,
            &saved.document,
            &sheet_frames,
            prepared["grid"]["cellWidth"]
                .as_u64()
                .ok_or_else(|| invalid("联系表尺寸无效"))? as u32,
        )
        .await?;
        let cells = Value::Array(
            sheet
                .iter()
                .map(|p| json!({"jpeg":STANDARD.encode(&p.jpeg),"frame":p.evidence["frame"]}))
                .collect(),
        );
        let jpeg = editor::export::frames::sheet(
            &cells,
            &prepared["input"]["canvas"]["fps"],
            &prepared["grid"],
            cancel,
        )
        .await
        .map_err(invalid)?;
        let mut images = vec![picture(
            format!("contact sheet; revision {revision}; frames {sheet_frames:?}"),
            &jpeg,
        )];
        let mut stored = vec![
            json!({"label":images[0].name,"id":views::keep(&self.pool,project,turn,revision,&jpeg).await?}),
        ];
        let phones = render(
            &mut session,
            &self.pool,
            &self.root,
            project,
            &saved.document,
            &phone_frames,
            prepared["phoneWidth"].as_u64().unwrap() as u32,
        )
        .await?;
        let fix_width = saved.document.project["canvas"]["width"]
            .as_u64()
            .unwrap_or(720)
            .min(720) as u32;
        let fixes = render(
            &mut session,
            &self.pool,
            &self.root,
            project,
            &saved.document,
            &fixed,
            fix_width,
        )
        .await?;
        for p in &phones {
            images.push(picture(
                format!("phone frame {}; revision {revision}", p.evidence["frame"]),
                &p.jpeg,
            ));
            stored.push(json!({"label":images.last().unwrap().name,"id":views::keep(&self.pool,project,turn,revision,&p.jpeg).await?}));
        }
        for (i, p) in fixes.iter().enumerate() {
            images.push(picture(
                format!(
                    "claimed fix {}; frame {}; revision {revision}",
                    prepared["fixed"][i]["defect"], p.evidence["frame"]
                ),
                &p.jpeg,
            ));
            stored.push(json!({"label":images.last().unwrap().name,"id":views::keep(&self.pool,project,turn,revision,&p.jpeg).await?}));
        }
        // Each graphic's midpoint is drawn in two fresh Chromium sessions, just as in the reference.
        let mut checked = vec![];
        let mut differed = vec![];
        for graphic in prepared["graphics"].as_array().unwrap() {
            let frames = [graphic["frame"].as_u64().unwrap()];
            let first = render(
                &mut session,
                &self.pool,
                &self.root,
                project,
                &saved.document,
                &frames,
                480,
            )
            .await?;
            let mut other = editor::export::frames::Session::new(cancel).map_err(invalid)?;
            let second = render(
                &mut other,
                &self.pool,
                &self.root,
                project,
                &saved.document,
                &frames,
                480,
            )
            .await?;
            let name = graphic["name"].as_str().unwrap().to_owned();
            if Sha256::digest(&first[0].jpeg) != Sha256::digest(&second[0].jpeg) {
                differed.push(name.clone());
            }
            checked.push(name);
        }
        let temporal = match super::motion::collect(
            &self.pool,
            &self.root,
            project,
            &saved.document,
            &prepared,
            &mut session,
            cancel,
        )
        .await
        {
            Ok(result) => result,
            Err(error) if !cancel.is_cancelled() => super::motion::Evidence {
                analysis: json!({"checks":[],"lines":[format!("Motion scan unavailable: {error}. Missing measurements are unknown, not passed.")],"strips":[],"blur":[],"scanned":0,"planned":prepared["temporal"]["scanList"].as_array().map_or(0,Vec::len),"cut":true}),
                images: vec![],
            },
            Err(error) => return Err(error),
        };
        for (label, jpeg) in temporal.images {
            images.push(picture(label.clone(), &jpeg));
            stored.push(json!({"label":label,"id":views::keep(&self.pool,project,turn,revision,&jpeg).await?}));
        }
        let rehearsal = match session
            .rehearse(&self.pool, &self.root, project, &saved.document)
            .await
        {
            Ok(result) => result,
            Err(error) if !cancel.is_cancelled() => {
                tracing::warn!("审阅预演未完成：{error}");
                Value::Null
            }
            Err(error) => return Err(invalid(error)),
        };
        let rendered = json!({"census":sheet.iter().map(|p|&p.evidence).collect::<Vec<_>>(),"fixed":fixes.iter().map(|p|&p.evidence).collect::<Vec<_>>(),"images":images.iter().map(|i|json!({"label":i.name})).collect::<Vec<_>>(),"determinism":{"checked":checked,"differed":differed},"motion":temporal.analysis,"rehearsal":rehearsal});
        let evidence = views::eval(
            &format!("BiliupExport.agentReviewInput({prepared},{rendered})"),
            cancel,
        )
        .await?;
        // Reject a stale image set before spending a provider request.
        let current = editor::get(&self.pool, project).await?;
        if current.summary.revision != revision
            || environment(&self.pool, &self.root, project, &current.document)
                .await?
                .0
                != stamp
            || plan::read(&self.pool, project).await?["revision"] != expected
        {
            return Err(Error::Conflict);
        }
        let key = editor::keys::get(&self.pool, &self.root, &context.model.provider)
            .await
            .map_err(invalid)?
            .ok_or_else(|| invalid("请先配置审阅模型密钥"))?;
        let mut critic = Conversation::new(
            context.model.clone(),
            evidence["system"].as_str().unwrap(),
            vec![],
        )
        .map_err(invalid)?;
        critic
            .message("user", evidence["text"].as_str().unwrap(), &[])
            .map_err(invalid)?;
        let mut batch = vec![];
        let mut bytes = 0;
        for (index, image) in images.into_iter().enumerate() {
            if !batch.is_empty()
                && (batch.len() >= 8 || bytes + image.data.len() > 24 * 1024 * 1024)
            {
                critic
                    .message(
                        "user",
                        "Rendered evidence images in the stated order.",
                        &batch,
                    )
                    .map_err(invalid)?;
                batch.clear();
                bytes = 0;
            }
            bytes += image.data.len();
            batch.push(image);
            if index == stored.len() - 1 {
                critic
                    .message(
                        "user",
                        "Rendered evidence images in the stated order.",
                        &batch,
                    )
                    .map_err(invalid)?;
            }
        }
        context.review.attempts += 1;
        let reply = critic.next(&key, cancel).await.map_err(invalid)?;
        for (key, amount) in [
            ("inputTokens", reply.input_tokens),
            ("outputTokens", reply.output_tokens),
        ] {
            data["usage"][key] = json!(
                data["usage"][key]
                    .as_u64()
                    .unwrap_or(0)
                    .saturating_add(amount)
            );
        }
        store::update(&self.pool, turn, data).await?;
        if !reply.calls.is_empty() {
            return Err(invalid("独立审阅不能调用工具"));
        }
        let mut result = views::eval(
            &format!(
                "BiliupExport.agentReviewFinish({},{prepared},{evidence})",
                json!(reply.text)
            ),
            cancel,
        )
        .await?;
        context.review.rounds += 1;
        context.review.last_stamp = Some(review_stamp);
        context.looked = Default::default();
        let current = editor::get(&self.pool, project).await?;
        if current.summary.revision != revision
            || environment(&self.pool, &self.root, project, &current.document)
                .await?
                .0
                != stamp
        {
            return Err(Error::Conflict);
        }
        result["review"]["model"] = json!(context.model);
        result["review"]["stamp"] = json!(stamp);
        result["review"]["images"] = json!(stored);
        result["review"]["coverageMissing"] = evidence["missing"].clone();
        result["review"]["measurements"] =
            json!({"motion":rendered["motion"],"rehearsal":rendered["rehearsal"]});
        let review = result["review"].clone();
        plan["reviews"]
            .as_array_mut()
            .ok_or_else(|| invalid("审阅记录无效"))?
            .push(review.clone());
        plan::write(
            &self.pool,
            project,
            expected,
            plan,
            Some(turn),
            Some(revision),
        )
        .await?;
        cards::add(data, json!({"type":"review","review":review}), "独立审阅");
        store::update(&self.pool, turn, data).await?;
        Ok(result)
    }
}
