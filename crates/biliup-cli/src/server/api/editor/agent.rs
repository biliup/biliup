use super::{ConnectionPool, RenderJobs, response};
use crate::server::{
    api::access::Caller,
    workbench::editor::agent::{
        plan,
        service::{Approval, Request},
        store,
    },
};
use axum::{
    Json,
    extract::{Path, Query, State},
    response::Response,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct History {
    before: Option<i64>,
    #[serde(rename="beforeId")]
    before_id:Option<String>,
}
pub async fn screen_take(State(jobs):State<Arc<RenderJobs>>,caller:Caller,Path(project):Path<i64>,Json(answer):Json<crate::server::workbench::editor::agent::capture::Answer>)->Response{
    response(jobs.editor_agent.screen_answer(project,answer,&caller).await)
}
pub async fn state(
    State(pool): State<ConnectionPool>,
    Path(project): Path<i64>,
    Query(query): Query<History>,
) -> Response {
    response(store::state_page(&pool, project, query.before,query.before_id.as_deref()).await)
}
pub async fn send(
    State(jobs): State<Arc<RenderJobs>>,
    caller: Caller,
    Path(project): Path<i64>,
    Json(request): Json<Request>,
) -> Response {
    response(
        jobs.editor_agent
            .start(project, request, caller)
            .await
            .map(|id| json!({"turnId":id})),
    )
}
pub async fn cancel(
    State(jobs): State<Arc<RenderJobs>>,
    caller: Caller,
    Path(project): Path<i64>,
) -> Response {
    response(jobs.editor_agent.cancel(project, &caller).await)
}
pub async fn approve(
    State(jobs): State<Arc<RenderJobs>>,
    caller: Caller,
    Path(project): Path<i64>,
    Json(request): Json<Approval>,
) -> Response {
    response(
        jobs.editor_agent
            .approve(project, request, &caller)
            .map(|accepted| json!({"accepted":accepted})),
    )
}
pub async fn undo(
    State(pool): State<ConnectionPool>,
    Path((project, turn)): Path<(i64, String)>,
) -> Response {
    response(store::undo(&pool, project, &turn).await)
}
pub async fn new_thread(State(jobs): State<Arc<RenderJobs>>, Path(project): Path<i64>) -> Response {
    response(
        jobs.editor_agent
            .new_thread(project)
            .await
            .map(|start| json!({"threadStart":start})),
    )
}
pub async fn plan_get(State(pool): State<ConnectionPool>, Path(project): Path<i64>) -> Response {
    response(
        async {
            crate::server::workbench::editor::get(&pool, project).await?;
            plan::read(&pool, project).await
        }
        .await,
    )
}
pub async fn plan_save(
    State(pool): State<ConnectionPool>,
    Path(project): Path<i64>,
    Json(input): Json<plan::Save>,
) -> Response {
    response(plan::save_user(&pool, project, input).await)
}
pub async fn image(
    State(pool): State<ConnectionPool>,
    Path((project, id)): Path<(i64, String)>,
) -> Response {
    use axum::{
        http::{StatusCode, header},
        response::IntoResponse,
    };
    let result = sqlx::query_scalar::<_, Vec<u8>>(
        "SELECT jpeg FROM editor_agent_images WHERE project_id=? AND id=?",
    )
    .bind(project)
    .bind(id)
    .fetch_optional(&pool)
    .await;
    match result {
        Ok(Some(bytes)) => (
            [
                (header::CONTENT_TYPE, "image/jpeg"),
                (
                    header::CACHE_CONTROL,
                    "private, max-age=31536000, immutable",
                ),
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            ],
            bytes,
        )
            .into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => response::<()>(Err(error.into())),
    }
}
pub async fn file(State(pool):State<ConnectionPool>,Path((project,id)):Path<(i64,String)>)->Response{
    use axum::{http::{StatusCode,header},response::IntoResponse};
    let result=sqlx::query_as::<_,(String,String,String)>("SELECT name,format,content FROM editor_agent_files WHERE project_id=? AND id=?").bind(project).bind(id).fetch_optional(&pool).await;
    match result{
        Ok(Some((name,format,content)))=>{
            let mut response=([(header::CONTENT_TYPE,if format=="vtt"{"text/vtt; charset=utf-8"}else{"application/x-subrip; charset=utf-8"}),(header::CACHE_CONTROL,"private, no-store"),(header::X_CONTENT_TYPE_OPTIONS,"nosniff")],content).into_response();
            let disposition=format!("attachment; filename=\"subtitles.{format}\"; filename*=UTF-8''{}",urlencoding::encode(&name));
            if let Ok(value)=disposition.parse(){response.headers_mut().insert(header::CONTENT_DISPOSITION,value);}
            response
        },
        Ok(None)=>StatusCode::NOT_FOUND.into_response(),
        Err(error)=>response::<()>(Err(error.into()))
    }
}
