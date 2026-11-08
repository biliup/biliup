//! Publish immutable editor/render outputs through the existing Bilibili uploader.
//! Draft IDs are submission receipts: repeating a confirmed request never uploads twice.
use super::{clips::publish::{self, queue::{Bilibili, Failure}}, recorder::now_ms, renders::{self, RenderJobs}, retention::MEDIA_LIFECYCLE};
use crate::server::{
    common::{upload::{desc_with_credits, studio_from_template}, upload_observation::{NewTask, UploadObservations, scrub}, util::Recorder},
    infrastructure::{connection_pool::ConnectionPool, models::{StreamerInfo, upload_streamer::UploadStreamer}},
};
use biliup::bilibili::Studio;
use chrono::{DateTime, Utc};
use ormlite::Model;
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::{path::{Path, PathBuf}, sync::Arc};
pub(crate) mod covers;

const DRAFT_LIFETIME_MS: i64 = 30 * 60 * 1000;
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")] Invalid(String),
    #[error("{0}")] Conflict(String),
    #[error("成片或投稿预览不存在")] NotFound,
    #[error(transparent)] Db(#[from] sqlx::Error),
    #[error(transparent)] Json(#[from] serde_json::Error),
}
type Result<T> = std::result::Result<T, Error>;
fn invalid(message: impl Into<String>) -> Error { Error::Invalid(message.into()) }

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fields {
    pub title: Option<String>,
    pub desc: Option<String>,
    pub tags: Option<Vec<String>>,
    pub tid: Option<u16>,
    pub tid_v2: Option<u32>,
    pub copyright: Option<u8>,
    pub source: Option<String>,
    pub dtime: Option<u32>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preview {
    pub template_id: i64,
    #[serde(default)] pub fields: Fields,
    #[serde(default)] pub cover: covers::Selection,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Submit {
    pub draft: String,
    #[serde(default)] pub confirm_previous: bool,
}
#[derive(Serialize, sqlx::FromRow)]
pub struct Receipt {
    pub id: String,
    pub state: String,
    pub task_id: Option<String>,
    pub bvid: Option<String>,
    pub error: Option<String>,
    pub updated_at: i64,
}
pub async fn latest(pool: &ConnectionPool, kind: &str, output: i64) -> Result<Option<Receipt>> {
    Ok(sqlx::query_as("SELECT id,state,task_id,bvid,error,updated_at FROM output_publications WHERE kind=? AND output_id=? AND state<>'draft' ORDER BY created_at DESC,id DESC LIMIT 1")
        .bind(kind).bind(output).fetch_optional(pool).await?)
}
pub async fn recover(pool: &ConnectionPool) -> sqlx::Result<()> {
    sqlx::query("UPDATE output_publications SET state=CASE WHEN state='submitting' THEN 'unknown' ELSE 'failed' END,error=CASE WHEN state='submitting' THEN '服务在提交时中断，请先核实 B 站投稿结果' ELSE '服务重启，上传已中断' END,updated_at=? WHERE state IN ('uploading','submitting')")
        .bind(now_ms()).execute(pool).await?;
    sqlx::query("DELETE FROM output_publications WHERE state='draft' AND created_at<?")
        .bind(now_ms()-DRAFT_LIFETIME_MS).execute(pool).await?;
    covers::cleanup(pool).await?;
    Ok(())
}

struct Output { path: PathBuf, name: String, info: StreamerInfo, session: Option<i64>, duration_ms: i64 }
async fn output(pool: &ConnectionPool, jobs: &RenderJobs, kind: &str, id: i64) -> Result<Output> {
    if id <= 0 { return Err(invalid("成片标识无效")); }
    let (path, name, session, created, duration_ms) = match kind {
        "editor" => {
            let row=sqlx::query("SELECT project_id,document_json,created_at,job_json FROM editor_exports WHERE id=? AND phase='done'")
                .bind(id).fetch_optional(pool).await?.ok_or(Error::NotFound)?;
            let document:super::editor::Document=serde_json::from_str(row.try_get("document_json")?)?;
            let (path,name)=jobs.editor_exports.download(row.try_get("project_id")?,id).await.map_err(invalid)?;
            let job:Value=serde_json::from_str(row.try_get("job_json")?)?;
            let seconds=job["seconds"].as_f64().filter(|seconds|seconds.is_finite()&&*seconds>0.).ok_or_else(||invalid("成片时长无效"))?;
            (path,name,document.origin.session_id,row.try_get::<i64,_>("created_at")?,(seconds*1000.).round() as i64)
        }
        "render" => {
            let record=renders::store::get(pool,id).await?.ok_or(Error::NotFound)?;
            if record.view.state!="ready" { return Err(invalid("成片尚未完成")); }
            if record.view.clip_id.is_some() { return Err(invalid("此成片请使用关联切片的投稿入口")); }
            (record.output_path.ok_or(Error::NotFound)?,record.view.file_name.ok_or(Error::NotFound)?,Some(record.view.session_id),record.view.created_at,record.view.duration_ms.ok_or_else(||invalid("成片时长无效"))?)
        }
        _ => return Err(invalid("成片类型无效")),
    };
    // A deleted source session does not make an independently retained export disappear.
    let info=if let Some(session)=session {
        StreamerInfo::select().where_("id = ?").bind(session).fetch_optional(pool).await.map_err(|_|invalid("读取来源场次失败"))?
    } else { None }.unwrap_or_else(||StreamerInfo::new("", "", &name, DateTime::from_timestamp_millis(created).unwrap_or_else(Utc::now), ""));
    Ok(Output{path,name,info,session,duration_ms})
}
async fn template(pool: &ConnectionPool, id: i64) -> Result<UploadStreamer> {
    let template=UploadStreamer::select().where_("id = ?").bind(id).fetch_optional(pool).await
        .map_err(|_|invalid("读取投稿模板失败"))?.ok_or_else(||invalid("投稿模板不存在"))?;
    if template.is_noop_uploader() { return Err(invalid("此模板设置为不上传，请选择其他投稿模板")); }
    Ok(template)
}
fn template_hash(template: &UploadStreamer) -> Result<String> {
    Ok(format!("{:x}",Sha256::digest(serde_json::to_vec(template)?)))
}
async fn file_stamp(path: &Path) -> Result<String> {
    let meta=tokio::fs::metadata(path).await.map_err(|_|invalid("成片或封面文件不可用"))?;
    if !meta.is_file() || meta.len()==0 { return Err(invalid("成片或封面文件为空")); }
    let modified=meta.modified().map_err(|_|invalid("读取文件修改时间失败"))?
        .duration_since(std::time::UNIX_EPOCH).map_err(|_|invalid("文件修改时间无效"))?.as_nanos();
    Ok(format!("{}:{modified}",meta.len()))
}
fn problem(studio: &Studio) -> Option<String> {
    let fields=publish::StudioOverride{title:Some(studio.title.clone()),desc:Some(studio.desc.clone()),tags:Some(studio.tag.split(',').map(str::to_owned).collect()),..Default::default()};
    if let Err(error)=fields.validate(){return Some(error)}
    if studio.title.trim().is_empty(){return Some("请输入投稿标题".into())}
    if studio.tag.trim().is_empty(){return Some("请至少填写一个标签".into())}
    if studio.tid==0{return Some("请选择投稿分区".into())}
    if !matches!(studio.copyright,1|2){return Some("请选择自制或转载".into())}
    if studio.copyright==2&&studio.source.trim().is_empty(){return Some("转载稿件需要填写来源".into())}
    studio.dtime.and_then(|at|publish::schedule_problem(at,Utc::now().timestamp() as u64))
}
fn studio(template: &UploadStreamer, output: &Output, fields: Fields) -> Result<Studio> {
    let vars=publish::ClipVars{id:0,title:output.name.clone(),at_ms:output.info.date.timestamp_millis()};
    let mut base=template.clone();
    base.title=Some(String::new());base.description=Some(String::new());
    let mut studio=studio_from_template(&base,vec![],&Recorder::new(base.title.clone(),output.info.clone()));
    studio.title=fields.title.unwrap_or_else(||publish::render(template.title.as_deref().filter(|s|!s.trim().is_empty()).unwrap_or("{clip_title}"),&output.info,&vars));
    let desc=fields.desc.unwrap_or_else(||publish::render(template.description.as_deref().unwrap_or(""),&output.info,&vars));
    (studio.desc,studio.desc_v2)=desc_with_credits(desc,template.credits.as_ref());
    if let Some(tags)=fields.tags{studio.tag=tags.into_iter().map(|tag|tag.trim().to_owned()).filter(|tag|!tag.is_empty()).collect::<Vec<_>>().join(",")}
    if let Some(tid)=fields.tid{studio.tid=tid;studio.tid_v2=fields.tid_v2;}
    if let Some(copyright)=fields.copyright{studio.copyright=copyright}
    if let Some(source)=fields.source{studio.source=source}
    if let Some(at)=fields.dtime{studio.dtime=(at!=0).then_some(at)}
    // Extra template fields must not silently replace the fields reviewed in this dialog.
    let extras=std::mem::take(&mut studio.extra_fields);
    let known=serde_json::to_value(&studio)?;
    if extras.as_ref().is_some_and(|extras|extras.keys().any(|key|known.get(key).is_some()||matches!(key.as_str(),"tid_v2"|"dtime"|"aid"))) {
        return Err(invalid("模板额外字段与投稿字段冲突，请先修改模板"));
    }
    studio.extra_fields=extras;
    Ok(studio)
}

pub async fn preview(pool: &ConnectionPool, jobs: &RenderJobs, kind: &str, output_id: i64, input: Preview) -> Result<Value> {
    let _lifecycle=MEDIA_LIFECYCLE.read().await;
    let output=output(pool,jobs,kind,output_id).await?;
    let template=template(pool,input.template_id).await?;
    let mut studio=studio(&template,&output,input.fields)?;
    let source_stamp=file_stamp(&output.path).await?;
    let (cover,cover_problem)=match covers::select(pool,kind,output_id,&source_stamp,&studio.cover,input.cover).await {
        Ok(cover)=>(cover,None),
        Err(Error::Invalid(message))=>(None,Some(message)),
        Err(error)=>return Err(error),
    };
    studio.cover.clear();
    let previous=latest(pool,kind,output_id).await?;
    let mut receipt_id=[0u8;16];
    SystemRandom::new().fill(&mut receipt_id).map_err(|_|invalid("无法生成投稿收据标识"))?;
    let id:String=receipt_id.iter().map(|byte|format!("{byte:02x}")).collect();let now=now_ms();
    let studio_json=serde_json::to_string(&studio)?;
    sqlx::query("DELETE FROM output_publications WHERE state='draft' AND created_at<?").bind(now-DRAFT_LIFETIME_MS).execute(pool).await?;
    // Invalid previews cannot be submitted, including an unreadable requested cover.
    let problem=cover_problem.clone().or_else(||problem(&studio));
    let draft=if problem.is_none(){
        sqlx::query("INSERT INTO output_publications(id,kind,output_id,template_id,template_hash,source_stamp,cover_id,studio_json,previous_id,state,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,'draft',?,?)")
            .bind(&id).bind(kind).bind(output_id).bind(template.id).bind(template_hash(&template)?).bind(&source_stamp).bind(cover.as_ref().map(|cover|&cover.id)).bind(studio_json).bind(previous.as_ref().map(|receipt|&receipt.id)).bind(now).bind(now).execute(pool).await?;
        Some(id)
    }else{None};
    Ok(json!({"draft":draft,"expires_at":now+DRAFT_LIFETIME_MS,"file_name":output.name,"duration_ms":output.duration_ms,"template_name":template.template_name,
        "studio":{"title":studio.title,"desc":studio.desc,"tags":studio.tag.split(',').collect::<Vec<_>>(),"tid":studio.tid,"tid_v2":studio.tid_v2,"copyright":studio.copyright,"source":studio.source,"dtime":studio.dtime,"dynamic":studio.dynamic,"only_self":studio.is_only_self==Some(1)},
        "cover":cover,"cover_error":cover_problem,"problem":problem,"previous":previous}))
}

pub async fn submit(pool: ConnectionPool, jobs: Arc<RenderJobs>, observations: Arc<UploadObservations>, backend: Arc<dyn Bilibili>, kind: String, output_id: i64, input: Submit) -> Result<Receipt> {
    let lifecycle=MEDIA_LIFECYCLE.read().await;
    let row=sqlx::query("SELECT * FROM output_publications WHERE id=? AND kind=? AND output_id=?")
        .bind(&input.draft).bind(&kind).bind(output_id).fetch_optional(&pool).await?.ok_or(Error::NotFound)?;
    if row.try_get::<&str,_>("state")?!="draft" {
        let receipt=sqlx::query_as("SELECT id,state,task_id,bvid,error,updated_at FROM output_publications WHERE id=?").bind(&input.draft).fetch_one(&pool).await?;
        return Ok(receipt);
    }
    if row.try_get::<i64,_>("created_at")?<now_ms()-DRAFT_LIFETIME_MS {return Err(Error::Conflict("预览已过期，请重新预览投稿内容".into()))}
    // Resolve files before taking a write transaction. Never borrow a second pool
    // connection while holding SQLite's writer lock.
    let output=output(&pool,&jobs,&kind,output_id).await?;
    if file_stamp(&output.path).await?!=row.try_get::<&str,_>("source_stamp")? {
        return Err(Error::Conflict("成片已变化，请重新预览".into()));
    }
    let mut studio:Studio=serde_json::from_str(row.try_get("studio_json")?)?;
    if let Some(problem)=problem(&studio){return Err(invalid(problem))}
    let cover_bytes=if let Some(id)=row.try_get::<Option<&str>,_>("cover_id")? {
        Some(covers::bytes(&pool,&kind,output_id,id).await?)
    }else{None};
    let file_name=output.path.file_name().ok_or_else(||invalid("成片文件名无效"))?.to_string_lossy().into_owned();
    let file_bytes=tokio::fs::metadata(&output.path).await.ok().map(|meta|meta.len());
    let mut tx=pool.begin_with("BEGIN IMMEDIATE").await?;
    let current:Receipt=sqlx::query_as("SELECT id,state,task_id,bvid,error,updated_at FROM output_publications WHERE id=?")
        .bind(&input.draft).fetch_optional(&mut *tx).await?.ok_or(Error::NotFound)?;
    if current.state!="draft"{return Ok(current)}
    let template=UploadStreamer::select().where_("id = ?").bind(row.try_get::<i64,_>("template_id")?)
        .fetch_optional(&mut *tx).await.map_err(|_|invalid("读取投稿模板失败"))?.ok_or_else(||invalid("投稿模板不存在"))?;
    if template_hash(&template)?!=row.try_get::<&str,_>("template_hash")? {
        return Err(Error::Conflict("模板已变化，请重新预览".into()));
    }
    let previous:Option<Receipt>=sqlx::query_as("SELECT id,state,task_id,bvid,error,updated_at FROM output_publications WHERE kind=? AND output_id=? AND state<>'draft' ORDER BY created_at DESC,id DESC LIMIT 1")
        .bind(&kind).bind(output_id).fetch_optional(&mut *tx).await?;
    if previous.as_ref().map(|receipt|receipt.id.as_str())!=row.try_get::<Option<&str>,_>("previous_id")? {
        return Err(Error::Conflict("投稿记录已变化，请重新预览".into()));
    }
    if let Some(previous)=previous {
        if matches!(previous.state.as_str(),"uploading"|"submitting"){return Err(Error::Conflict("此成片正在投稿，请查看上传队列".into()))}
        if matches!(previous.state.as_str(),"succeeded"|"unknown")&&!input.confirm_previous{return Err(Error::Conflict("请先核实已有投稿，再确认是否重新投稿".into()))}
    }
    let guard=observations.register(NewTask{source:"manual",scheduler_group:"manual",title:&studio.title,streamer_name:(!output.info.name.is_empty()).then_some(output.info.name.as_str()),session_id:output.session,template_name:Some(&template.template_name),fifo:false});
    let observation=guard.observation();let task_id=observation.id().to_owned();
    observation.prepare_files([(file_name,file_bytes)]);
    sqlx::query("UPDATE output_publications SET state='uploading',task_id=?,updated_at=? WHERE id=?")
        .bind(&task_id).bind(now_ms()).bind(&input.draft).execute(&mut *tx).await?;
    tx.commit().await?;
    // Network transfers of completed outputs must not block live-recording retention.
    drop(lifecycle);
    let receipt=Receipt{id:input.draft.clone(),state:"uploading".into(),task_id:Some(task_id),bvid:None,error:None,updated_at:now_ms()};
    // No await between commit and spawn: a disconnected HTTP response still has one worker.
    tokio::spawn(async move {
        let result=guard.scope(async {
            observation.phase("initializing");
            let connection=backend.connect(&template).await?;
            let mut video=connection.upload(&output.path,&|_|true).await?;
            video.title=Some(studio.title.clone());studio.videos=vec![video];
            if let Some(bytes)=cover_bytes {
                observation.phase("cover");
                let path=tempfile::Builder::new().prefix("biliup-cover-").suffix(".jpg").tempfile()
                    .map_err(|_|Failure::Other("无法准备投稿封面".into()))?.into_temp_path();
                tokio::fs::write(&path,&bytes).await.map_err(|_|Failure::Other("无法准备投稿封面".into()))?;
                studio.cover=connection.cover(&path).await?;
            }
            if let Some(problem)=problem(&studio){return Err(Failure::Other(problem))}
            sqlx::query("UPDATE output_publications SET state='submitting',updated_at=? WHERE id=? AND state='uploading'")
                .bind(now_ms()).bind(&input.draft).execute(&pool).await.map_err(|_|Failure::Other("无法保存投稿状态，尚未提交稿件".into()))?;
            connection.submit(&studio).await
        }).await;
        let (state,error,bvid)=match result {
            Ok(submitted) => ("succeeded",None,(!submitted.bvid.is_empty()).then_some(submitted.bvid)),
            Err(error) => (if matches!(&error,Failure::Unknown(_)){"unknown"}else{"failed"},Some(scrub(error.message())),None),
        };
        let saved=sqlx::query("UPDATE output_publications SET state=?,error=?,bvid=?,updated_at=? WHERE id=?")
            .bind(state).bind(&error).bind(&bvid).bind(now_ms()).bind(&input.draft).execute(&pool).await;
        if saved.is_err(){tracing::error!(publication=%input.draft,"无法保存成片投稿结果，请核实 B 站稿件管理");}
        guard.finish(state,error.as_deref(),bvid.as_deref());
        observations.flush().await;
    });
    Ok(receipt)
}
