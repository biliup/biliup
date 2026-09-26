//! 控制面上的房间、投稿模板与节点上报的 B 站账号：`fleet_rooms` / `fleet_templates` /
//! `fleet_node_accounts` 的读写，以及房间分派（迁移）的状态机。
//!
//! 分派状态机：`node_id` 是房间应该在哪台节点上录，`releasing_node_id` 是还没确认释放的上一台。
//! 只要 `releasing_node_id` 非空，这个房间就不在任何节点的期望状态里：先让上一台停，
//! 确认停了（它的 Ack 里不再持有这个房间）才交给 `node_id`，同一个房间不会有两台同时录。
//! 每次改分派 `epoch` +1，节点重连时按它对账。

use super::model::{Account, DesiredRoom, DesiredTemplate, RoomSpec, TemplateSpec};
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use error_stack::ResultExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn db_error(what: &'static str) -> AppError {
    AppError::Custom(format!("fleet database: {what}"))
}

fn to_json<T: Serialize>(value: &Option<T>) -> Option<String> {
    value
        .as_ref()
        .and_then(|value| serde_json::to_string(value).ok())
}

fn from_json<T: serde::de::DeserializeOwned>(text: Option<String>) -> Option<T> {
    text.and_then(|text| serde_json::from_str(&text).ok())
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .is_some_and(|e| e.is_unique_violation())
}

/// 控制面上的一个投稿模板
#[derive(Debug, Clone, Serialize)]
pub struct Template {
    pub id: i64,
    #[serde(flatten)]
    pub spec: TemplateSpec,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(sqlx::FromRow)]
struct TemplateRow {
    id: i64,
    template_name: String,
    title: Option<String>,
    tid: Option<u16>,
    tid_v2: Option<u32>,
    copyright: Option<u8>,
    copyright_source: Option<String>,
    cover_path: Option<String>,
    description: Option<String>,
    dynamic: Option<String>,
    dtime: Option<u32>,
    dolby: Option<u8>,
    hires: Option<u8>,
    charging_pay: Option<u8>,
    no_reprint: Option<u8>,
    is_only_self: Option<u8>,
    uploader: Option<String>,
    account_mid: Option<i64>,
    tags: String,
    credits: Option<String>,
    up_selection_reply: Option<u8>,
    up_close_reply: Option<u8>,
    up_close_danmu: Option<u8>,
    extra_fields: Option<String>,
    created_at: i64,
    updated_at: i64,
}

impl From<TemplateRow> for Template {
    fn from(row: TemplateRow) -> Self {
        Template {
            id: row.id,
            spec: TemplateSpec {
                template_name: row.template_name,
                title: row.title,
                tid: row.tid,
                tid_v2: row.tid_v2,
                copyright: row.copyright,
                copyright_source: row.copyright_source,
                cover_path: row.cover_path,
                description: row.description,
                dynamic: row.dynamic,
                dtime: row.dtime,
                dolby: row.dolby,
                hires: row.hires,
                charging_pay: row.charging_pay,
                no_reprint: row.no_reprint,
                is_only_self: row.is_only_self,
                uploader: row.uploader,
                account_mid: row.account_mid.and_then(|mid| u64::try_from(mid).ok()),
                tags: serde_json::from_str(&row.tags).unwrap_or_default(),
                credits: from_json(row.credits),
                up_selection_reply: row.up_selection_reply,
                up_close_reply: row.up_close_reply,
                up_close_danmu: row.up_close_danmu,
                extra_fields: row.extra_fields,
            },
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

const TEMPLATE_COLUMNS: &str = "template_name, title, tid, tid_v2, copyright, copyright_source, cover_path, \
     description, dynamic, dtime, dolby, hires, charging_pay, no_reprint, is_only_self, uploader, \
     account_mid, tags, credits, up_selection_reply, up_close_reply, up_close_danmu, extra_fields";

fn bind_template<'q>(
    query: sqlx::query::QueryAs<'q, sqlx::Sqlite, TemplateRow, sqlx::sqlite::SqliteArguments<'q>>,
    spec: &'q TemplateSpec,
) -> sqlx::query::QueryAs<'q, sqlx::Sqlite, TemplateRow, sqlx::sqlite::SqliteArguments<'q>> {
    query
        .bind(&spec.template_name)
        .bind(&spec.title)
        .bind(spec.tid)
        .bind(spec.tid_v2)
        .bind(spec.copyright)
        .bind(&spec.copyright_source)
        .bind(&spec.cover_path)
        .bind(&spec.description)
        .bind(&spec.dynamic)
        .bind(spec.dtime)
        .bind(spec.dolby)
        .bind(spec.hires)
        .bind(spec.charging_pay)
        .bind(spec.no_reprint)
        .bind(spec.is_only_self)
        .bind(&spec.uploader)
        .bind(spec.account_mid.map(|mid| mid as i64))
        .bind(serde_json::to_string(&spec.tags).unwrap_or_else(|_| "[]".into()))
        .bind(to_json(&spec.credits))
        .bind(spec.up_selection_reply)
        .bind(spec.up_close_reply)
        .bind(spec.up_close_danmu)
        .bind(&spec.extra_fields)
}

pub async fn list_templates(pool: &ConnectionPool) -> AppResult<Vec<Template>> {
    let rows: Vec<TemplateRow> = sqlx::query_as("SELECT * FROM fleet_templates ORDER BY id")
        .fetch_all(pool)
        .await
        .change_context(db_error("list templates"))?;
    Ok(rows.into_iter().map(Template::from).collect())
}

pub async fn template(pool: &ConnectionPool, id: i64) -> AppResult<Option<Template>> {
    let row: Option<TemplateRow> = sqlx::query_as("SELECT * FROM fleet_templates WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .change_context(db_error("read template"))?;
    Ok(row.map(Template::from))
}

pub async fn insert_template(
    pool: &ConnectionPool,
    spec: &TemplateSpec,
    now: i64,
) -> AppResult<Template> {
    let sql = format!(
        "INSERT INTO fleet_templates ({TEMPLATE_COLUMNS}, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING *"
    );
    let row = bind_template(sqlx::query_as(&sql), spec)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await
        .change_context(db_error("insert template"))?;
    Ok(row.into())
}

pub async fn update_template(
    pool: &ConnectionPool,
    id: i64,
    spec: &TemplateSpec,
    now: i64,
) -> AppResult<Option<Template>> {
    let assignments = TEMPLATE_COLUMNS
        .split(',')
        .map(|column| format!("{} = ?", column.trim()))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "UPDATE fleet_templates SET {assignments}, updated_at = ? WHERE id = ? RETURNING *"
    );
    let row = bind_template(sqlx::query_as(&sql), spec)
        .bind(now)
        .bind(id)
        .fetch_optional(pool)
        .await
        .change_context(db_error("update template"))?;
    Ok(row.map(Template::from))
}

#[derive(Debug, PartialEq, Eq)]
pub enum DeleteTemplate {
    Deleted,
    NotFound,
    /// 还有这么多个房间在用
    InUse(i64),
}

pub async fn delete_template(pool: &ConnectionPool, id: i64) -> AppResult<DeleteTemplate> {
    let mut tx = pool
        .begin()
        .await
        .change_context(db_error("begin transaction"))?;
    let used: i64 = sqlx::query_scalar("SELECT count(*) FROM fleet_rooms WHERE template_id = ?")
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .change_context(db_error("count template users"))?;
    if used > 0 {
        return Ok(DeleteTemplate::InUse(used));
    }
    let affected = sqlx::query("DELETE FROM fleet_templates WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .change_context(db_error("delete template"))?
        .rows_affected();
    tx.commit()
        .await
        .change_context(db_error("commit transaction"))?;
    Ok(if affected > 0 {
        DeleteTemplate::Deleted
    } else {
        DeleteTemplate::NotFound
    })
}

/// 控制面上的一个房间：录制设置加分派状态
#[derive(Debug, Clone, Serialize)]
pub struct Room {
    pub id: i64,
    #[serde(flatten)]
    pub spec: RoomSpec,
    pub template_id: Option<i64>,
    /// 应该在哪台节点上录；`None` 为未分派
    pub node_id: Option<i64>,
    pub epoch: i64,
    pub paused: bool,
    /// 还没确认释放的上一台；非空时迁移（或删除）还在进行
    pub releasing_node_id: Option<i64>,
    #[serde(skip)]
    pub release_after: Option<i64>,
    /// 非空表示已删除、正在等上一台释放
    pub deleted_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Room {
    /// 此刻在（或可能在）录这个房间的节点
    pub fn holder(&self) -> Option<i64> {
        self.releasing_node_id.or(self.node_id)
    }

    pub fn assignment(&self) -> Assignment {
        Assignment {
            node_id: self.node_id,
            releasing_node_id: self.releasing_node_id,
            epoch: self.epoch,
        }
    }

    pub fn desired(&self) -> DesiredRoom {
        DesiredRoom {
            id: self.id,
            epoch: self.epoch,
            paused: self.paused,
            template_id: self.template_id,
            spec: self.spec.clone(),
        }
    }
}

#[derive(sqlx::FromRow)]
struct RoomRow {
    id: i64,
    url: String,
    remark: String,
    filename_prefix: Option<String>,
    time_range: Option<String>,
    template_id: Option<i64>,
    format: Option<String>,
    #[sqlx(rename = "override")]
    override_cfg: Option<String>,
    preprocessor: Option<String>,
    segment_processor: Option<String>,
    downloaded_processor: Option<String>,
    postprocessor: Option<String>,
    opt_args: Option<String>,
    excluded_keywords: Option<String>,
    node_id: Option<i64>,
    epoch: i64,
    paused: bool,
    releasing_node_id: Option<i64>,
    release_after: Option<i64>,
    deleted_at: Option<i64>,
    created_at: i64,
    updated_at: i64,
}

impl From<RoomRow> for Room {
    fn from(row: RoomRow) -> Self {
        Room {
            id: row.id,
            spec: RoomSpec {
                url: row.url,
                remark: row.remark,
                filename_prefix: row.filename_prefix,
                time_range: row.time_range,
                format: row.format,
                override_cfg: from_json(row.override_cfg),
                preprocessor: from_json(row.preprocessor),
                segment_processor: from_json(row.segment_processor),
                downloaded_processor: from_json(row.downloaded_processor),
                postprocessor: from_json(row.postprocessor),
                opt_args: from_json(row.opt_args),
                excluded_keywords: from_json(row.excluded_keywords),
            },
            template_id: row.template_id,
            node_id: row.node_id,
            epoch: row.epoch,
            paused: row.paused,
            releasing_node_id: row.releasing_node_id,
            release_after: row.release_after,
            deleted_at: row.deleted_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

fn bind_room<'q, O>(
    query: sqlx::query::QueryAs<'q, sqlx::Sqlite, O, sqlx::sqlite::SqliteArguments<'q>>,
    spec: &'q RoomSpec,
) -> sqlx::query::QueryAs<'q, sqlx::Sqlite, O, sqlx::sqlite::SqliteArguments<'q>> {
    query
        .bind(&spec.url)
        .bind(&spec.remark)
        .bind(&spec.filename_prefix)
        .bind(&spec.time_range)
        .bind(&spec.format)
        .bind(to_json(&spec.override_cfg))
        .bind(to_json(&spec.preprocessor))
        .bind(to_json(&spec.segment_processor))
        .bind(to_json(&spec.downloaded_processor))
        .bind(to_json(&spec.postprocessor))
        .bind(to_json(&spec.opt_args))
        .bind(to_json(&spec.excluded_keywords))
}

const ROOM_SPEC_COLUMNS: &str = "url, remark, filename_prefix, time_range, format, override, preprocessor, \
     segment_processor, downloaded_processor, postprocessor, opt_args, excluded_keywords";

/// 房间地址已被另一个房间占用（包括正在删除、等释放的房间）
#[derive(Debug, PartialEq, Eq)]
pub struct UrlTaken;

pub async fn list_rooms(pool: &ConnectionPool) -> AppResult<Vec<Room>> {
    let rows: Vec<RoomRow> = sqlx::query_as("SELECT * FROM fleet_rooms ORDER BY id")
        .fetch_all(pool)
        .await
        .change_context(db_error("list rooms"))?;
    Ok(rows.into_iter().map(Room::from).collect())
}

pub async fn room(pool: &ConnectionPool, id: i64) -> AppResult<Option<Room>> {
    let row: Option<RoomRow> = sqlx::query_as("SELECT * FROM fleet_rooms WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .change_context(db_error("read room"))?;
    Ok(row.map(Room::from))
}

/// 新建房间。`node_id` 非空时直接分派给它（新房间没有上一台，不用等释放）。
pub async fn insert_room(
    pool: &ConnectionPool,
    spec: &RoomSpec,
    template_id: Option<i64>,
    node_id: Option<i64>,
    paused: bool,
    now: i64,
) -> AppResult<Result<Room, UrlTaken>> {
    let sql = format!(
        "INSERT INTO fleet_rooms ({ROOM_SPEC_COLUMNS}, template_id, node_id, paused, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING *"
    );
    let inserted: Result<RoomRow, sqlx::Error> = bind_room(sqlx::query_as(&sql), spec)
        .bind(template_id)
        .bind(node_id)
        .bind(paused)
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await;
    match inserted {
        Ok(row) => Ok(Ok(row.into())),
        Err(e) if is_unique_violation(&e) => Ok(Err(UrlTaken)),
        Err(e) => Err(e).change_context(db_error("insert room")),
    }
}

/// 改录制设置与模板，不动分派。已删除（等释放）的房间不能改。
pub async fn update_room(
    pool: &ConnectionPool,
    id: i64,
    spec: &RoomSpec,
    template_id: Option<i64>,
    now: i64,
) -> AppResult<Result<Option<Room>, UrlTaken>> {
    let assignments = ROOM_SPEC_COLUMNS
        .split(',')
        .map(|column| format!("{} = ?", column.trim()))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "UPDATE fleet_rooms SET {assignments}, template_id = ?, updated_at = ? \
         WHERE id = ? AND deleted_at IS NULL RETURNING *"
    );
    let updated: Result<Option<RoomRow>, sqlx::Error> = bind_room(sqlx::query_as(&sql), spec)
        .bind(template_id)
        .bind(now)
        .bind(id)
        .fetch_optional(pool)
        .await;
    match updated {
        Ok(row) => Ok(Ok(row.map(Room::from))),
        Err(e) if is_unique_violation(&e) => Ok(Err(UrlTaken)),
        Err(e) => Err(e).change_context(db_error("update room")),
    }
}

pub async fn set_paused(
    pool: &ConnectionPool,
    id: i64,
    paused: bool,
    now: i64,
) -> AppResult<Option<Room>> {
    let row: Option<RoomRow> = sqlx::query_as(
        "UPDATE fleet_rooms SET paused = ?, updated_at = ? WHERE id = ? AND deleted_at IS NULL RETURNING *",
    )
    .bind(paused)
    .bind(now)
    .bind(id)
    .fetch_optional(pool)
    .await
    .change_context(db_error("pause room"))?;
    Ok(row.map(Room::from))
}

/// 一个房间的分派状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Assignment {
    pub node_id: Option<i64>,
    pub releasing_node_id: Option<i64>,
    pub epoch: i64,
}

/// 把房间改派给 `target`（`None` 为取消分派）后的分派状态；已经是 `target` 时返回 `None`。
///
/// 此刻可能在录的是 `releasing_node_id`（迁移还没完成）或 `node_id`。改派后它成了要等释放的那台；
/// 改派回它自己就不用等了（等于撤销迁移）。
pub fn plan_assignment(current: Assignment, target: Option<i64>) -> Option<Assignment> {
    if current.node_id == target {
        return None;
    }
    let holder = current.releasing_node_id.or(current.node_id);
    Some(Assignment {
        node_id: target,
        releasing_node_id: holder.filter(|holder| Some(*holder) != target),
        epoch: current.epoch + 1,
    })
}

async fn write_assignment(
    tx: &mut sqlx::SqliteConnection,
    room: &Room,
    next: Assignment,
    version: i64,
    now: i64,
) -> AppResult<Room> {
    // 要等的那台换了人才重新记版本号；还是同一台时沿用原来的，它早先的 Ack 仍然算数
    let release_after = match next.releasing_node_id {
        None => None,
        Some(node) if Some(node) == room.releasing_node_id => room.release_after,
        Some(_) => Some(version),
    };
    let row: RoomRow = sqlx::query_as(
        "UPDATE fleet_rooms SET node_id = ?, releasing_node_id = ?, release_after = ?, epoch = ?, \
         updated_at = ? WHERE id = ? RETURNING *",
    )
    .bind(next.node_id)
    .bind(next.releasing_node_id)
    .bind(release_after)
    .bind(next.epoch)
    .bind(now)
    .bind(room.id)
    .fetch_one(&mut *tx)
    .await
    .change_context(db_error("assign room"))?;
    Ok(row.into())
}

/// 改派房间。`version` 是控制面此刻的期望状态版本号：上一台发来的版本号大于它的 Ack 才能确认释放。
/// 房间不存在或已删除时返回 `None`；已经分派给 `target` 时原样返回。
pub async fn assign_room(
    pool: &ConnectionPool,
    id: i64,
    target: Option<i64>,
    version: i64,
    now: i64,
) -> AppResult<Option<Room>> {
    let mut tx = pool
        .begin()
        .await
        .change_context(db_error("begin transaction"))?;
    let row: Option<RoomRow> =
        sqlx::query_as("SELECT * FROM fleet_rooms WHERE id = ? AND deleted_at IS NULL")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .change_context(db_error("read room"))?;
    let Some(room) = row.map(Room::from) else {
        return Ok(None);
    };
    let Some(next) = plan_assignment(room.assignment(), target) else {
        return Ok(Some(room));
    };
    let room = write_assignment(&mut tx, &room, next, version, now).await?;
    tx.commit()
        .await
        .change_context(db_error("commit transaction"))?;
    Ok(Some(room))
}

/// 强制迁移：不再等上一台确认释放。上一台如果其实还在录，会与新节点同时录，直到它连回来按期望状态停下。
/// 返回 `None` 表示房间不存在；已删除的房间强制后直接删掉，返回删除前的样子。
pub async fn force_release(pool: &ConnectionPool, id: i64, now: i64) -> AppResult<Option<Room>> {
    let mut tx = pool
        .begin()
        .await
        .change_context(db_error("begin transaction"))?;
    let row: Option<RoomRow> = sqlx::query_as("SELECT * FROM fleet_rooms WHERE id = ?")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .change_context(db_error("read room"))?;
    let Some(room) = row.map(Room::from) else {
        return Ok(None);
    };
    let room = if room.deleted_at.is_some() {
        sqlx::query("DELETE FROM fleet_rooms WHERE id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await
            .change_context(db_error("delete room"))?;
        room
    } else {
        let row: RoomRow = sqlx::query_as(
            "UPDATE fleet_rooms SET releasing_node_id = NULL, release_after = NULL, updated_at = ? \
             WHERE id = ? RETURNING *",
        )
        .bind(now)
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .change_context(db_error("force release"))?;
        row.into()
    };
    tx.commit()
        .await
        .change_context(db_error("commit transaction"))?;
    Ok(Some(room))
}

#[derive(Debug)]
pub enum DeleteRoom {
    NotFound,
    /// 已经删掉
    Deleted(Room),
    /// 还有节点可能在录：先标记删除，等它确认释放后再删
    Releasing(Room),
}

/// 删除房间。没有节点在录（或 `force`）时直接删；否则先从期望状态里拿掉，等节点确认释放。
pub async fn delete_room(
    pool: &ConnectionPool,
    id: i64,
    force: bool,
    version: i64,
    now: i64,
) -> AppResult<DeleteRoom> {
    let mut tx = pool
        .begin()
        .await
        .change_context(db_error("begin transaction"))?;
    let row: Option<RoomRow> = sqlx::query_as("SELECT * FROM fleet_rooms WHERE id = ?")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .change_context(db_error("read room"))?;
    let Some(room) = row.map(Room::from) else {
        return Ok(DeleteRoom::NotFound);
    };
    let outcome = if force || room.holder().is_none() {
        sqlx::query("DELETE FROM fleet_rooms WHERE id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await
            .change_context(db_error("delete room"))?;
        DeleteRoom::Deleted(room)
    } else if room.deleted_at.is_some() {
        DeleteRoom::Releasing(room)
    } else {
        let next = plan_assignment(room.assignment(), None).unwrap_or(room.assignment());
        let room = write_assignment(&mut tx, &room, next, version, now).await?;
        let row: RoomRow =
            sqlx::query_as("UPDATE fleet_rooms SET deleted_at = ? WHERE id = ? RETURNING *")
                .bind(now)
                .bind(room.id)
                .fetch_one(&mut *tx)
                .await
                .change_context(db_error("mark room deleted"))?;
        DeleteRoom::Releasing(row.into())
    };
    tx.commit()
        .await
        .change_context(db_error("commit transaction"))?;
    Ok(outcome)
}

/// 节点 `node` 发来了对期望状态 `version` 的 Ack，此刻持有 `held` 这些房间：
/// 等它释放、且它确实已经不再持有的房间就此确认释放（已删除的房间整行删掉）。返回确认了的房间 id。
pub async fn confirm_releases(
    pool: &ConnectionPool,
    node: i64,
    version: i64,
    held: &[i64],
    now: i64,
) -> AppResult<Vec<i64>> {
    let mut tx = pool
        .begin()
        .await
        .change_context(db_error("begin transaction"))?;
    let waiting: Vec<(i64, Option<i64>, Option<i64>)> = sqlx::query_as(
        "SELECT id, release_after, deleted_at FROM fleet_rooms WHERE releasing_node_id = ?",
    )
    .bind(node)
    .fetch_all(&mut *tx)
    .await
    .change_context(db_error("read releasing rooms"))?;
    let mut confirmed = Vec::new();
    for (id, release_after, deleted_at) in waiting {
        if held.contains(&id) || release_after.is_some_and(|after| version <= after) {
            continue;
        }
        if deleted_at.is_some() {
            sqlx::query("DELETE FROM fleet_rooms WHERE id = ?")
                .bind(id)
                .execute(&mut *tx)
                .await
                .change_context(db_error("delete room"))?;
        } else {
            sqlx::query(
                "UPDATE fleet_rooms SET releasing_node_id = NULL, release_after = NULL, updated_at = ? \
                 WHERE id = ?",
            )
            .bind(now)
            .bind(id)
            .execute(&mut *tx)
            .await
            .change_context(db_error("confirm release"))?;
        }
        confirmed.push(id);
    }
    tx.commit()
        .await
        .change_context(db_error("commit transaction"))?;
    Ok(confirmed)
}

/// 节点 `node` 的期望状态：分派给它、迁移已完成、没被删除的房间，以及这些房间引用的模板
pub async fn desired_state(
    pool: &ConnectionPool,
    node: i64,
) -> AppResult<(Vec<DesiredRoom>, Vec<DesiredTemplate>)> {
    let rows: Vec<RoomRow> = sqlx::query_as(
        "SELECT * FROM fleet_rooms WHERE node_id = ? AND releasing_node_id IS NULL \
         AND deleted_at IS NULL ORDER BY id",
    )
    .bind(node)
    .fetch_all(pool)
    .await
    .change_context(db_error("read desired rooms"))?;
    let rooms: Vec<Room> = rows.into_iter().map(Room::from).collect();
    let templates: Vec<TemplateRow> = sqlx::query_as(
        "SELECT * FROM fleet_templates WHERE id IN (SELECT template_id FROM fleet_rooms \
         WHERE node_id = ? AND releasing_node_id IS NULL AND deleted_at IS NULL) ORDER BY id",
    )
    .bind(node)
    .fetch_all(pool)
    .await
    .change_context(db_error("read desired templates"))?;
    Ok((
        rooms.iter().map(Room::desired).collect(),
        templates
            .into_iter()
            .map(|row| {
                let template = Template::from(row);
                DesiredTemplate {
                    id: template.id,
                    spec: template.spec,
                }
            })
            .collect(),
    ))
}

/// 节点被移除（或主动离开）：分派给它的房间变成未分派，等它释放的房间不再等。
/// 被移除的节点按约定把托管房间转成本地房间继续录，控制面不再管它。返回受影响的房间 id。
pub async fn unassign_node(pool: &ConnectionPool, node: i64, now: i64) -> AppResult<Vec<i64>> {
    let mut tx = pool
        .begin()
        .await
        .change_context(db_error("begin transaction"))?;
    let released: Vec<i64> = sqlx::query_scalar(
        "UPDATE fleet_rooms SET releasing_node_id = NULL, release_after = NULL, updated_at = ? \
         WHERE releasing_node_id = ? RETURNING id",
    )
    .bind(now)
    .bind(node)
    .fetch_all(&mut *tx)
    .await
    .change_context(db_error("drop releasing rooms"))?;
    let unassigned: Vec<i64> = sqlx::query_scalar(
        "UPDATE fleet_rooms SET node_id = NULL, epoch = epoch + 1, updated_at = ? \
         WHERE node_id = ? RETURNING id",
    )
    .bind(now)
    .bind(node)
    .fetch_all(&mut *tx)
    .await
    .change_context(db_error("unassign rooms"))?;
    sqlx::query(
        "DELETE FROM fleet_rooms WHERE deleted_at IS NOT NULL AND releasing_node_id IS NULL",
    )
    .execute(&mut *tx)
    .await
    .change_context(db_error("drop deleted rooms"))?;
    tx.commit()
        .await
        .change_context(db_error("commit transaction"))?;
    let mut affected = released;
    affected.extend(unassigned);
    affected.sort_unstable();
    affected.dedup();
    Ok(affected)
}

/// 每台节点分到的房间数（不含已删除的）
pub async fn assigned_counts(pool: &ConnectionPool) -> AppResult<HashMap<i64, i64>> {
    let rows: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT node_id, count(*) FROM fleet_rooms WHERE node_id IS NOT NULL \
         AND deleted_at IS NULL GROUP BY node_id",
    )
    .fetch_all(pool)
    .await
    .change_context(db_error("count assigned rooms"))?;
    Ok(rows.into_iter().collect())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NodeAccount {
    pub node_id: i64,
    pub mid: u64,
    pub uname: String,
    pub reported_at: i64,
}

/// 用节点这次上报的账号整份替换它原来的
pub async fn replace_accounts(
    pool: &ConnectionPool,
    node: i64,
    accounts: &[Account],
    now: i64,
) -> AppResult<()> {
    let mut tx = pool
        .begin()
        .await
        .change_context(db_error("begin transaction"))?;
    sqlx::query("DELETE FROM fleet_node_accounts WHERE node_id = ?")
        .bind(node)
        .execute(&mut *tx)
        .await
        .change_context(db_error("clear node accounts"))?;
    for account in accounts {
        sqlx::query(
            "INSERT OR REPLACE INTO fleet_node_accounts (node_id, mid, uname, reported_at) VALUES (?, ?, ?, ?)",
        )
        .bind(node)
        .bind(account.mid as i64)
        .bind(&account.uname)
        .bind(now)
        .execute(&mut *tx)
        .await
        .change_context(db_error("record node account"))?;
    }
    tx.commit()
        .await
        .change_context(db_error("commit transaction"))?;
    Ok(())
}

/// 未被移除的节点上报的账号
pub async fn list_accounts(pool: &ConnectionPool) -> AppResult<Vec<NodeAccount>> {
    let rows: Vec<(i64, i64, String, i64)> = sqlx::query_as(
        "SELECT a.node_id, a.mid, a.uname, a.reported_at FROM fleet_node_accounts a \
         JOIN fleet_nodes n ON n.id = a.node_id WHERE n.revoked_at IS NULL ORDER BY a.mid, a.node_id",
    )
    .fetch_all(pool)
    .await
    .change_context(db_error("list node accounts"))?;
    Ok(rows
        .into_iter()
        .map(|(node_id, mid, uname, reported_at)| NodeAccount {
            node_id,
            mid: mid as u64,
            uname,
            reported_at,
        })
        .collect())
}

pub async fn node_has_account(pool: &ConnectionPool, node: i64, mid: u64) -> AppResult<bool> {
    let found: Option<i64> =
        sqlx::query_scalar("SELECT 1 FROM fleet_node_accounts WHERE node_id = ? AND mid = ?")
            .bind(node)
            .bind(mid as i64)
            .fetch_optional(pool)
            .await
            .change_context(db_error("check node account"))?;
    Ok(found.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::fleet::FLEET_MIGRATOR;
    use crate::server::fleet::store;
    use crate::server::infrastructure::connection_pool::ConnectionManager;

    async fn pool() -> (tempfile::TempDir, ConnectionPool) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fleet.sqlite3");
        let pool = ConnectionManager::new_pool_with(path.to_str().unwrap(), &FLEET_MIGRATOR)
            .await
            .unwrap();
        (dir, pool)
    }

    async fn node(pool: &ConnectionPool, key: &str) -> i64 {
        let (token, secret) = store::create_token(pool, None, 1, 10_000).await.unwrap();
        match store::redeem_token(pool, &token.id, &secret, key, key, false, 2)
            .await
            .unwrap()
        {
            store::Redeem::Joined(node) => node.id,
            other => panic!("{other:?}"),
        }
    }

    fn spec(url: &str) -> RoomSpec {
        serde_json::from_value(serde_json::json!({ "url": url, "remark": "r" })).unwrap()
    }

    fn state(node: Option<i64>, releasing: Option<i64>, epoch: i64) -> Assignment {
        Assignment {
            node_id: node,
            releasing_node_id: releasing,
            epoch,
        }
    }

    #[test]
    fn assignment_waits_for_whoever_may_still_be_recording() {
        // 未分派 → A：没有上一台
        assert_eq!(
            plan_assignment(state(None, None, 1), Some(1)),
            Some(state(Some(1), None, 2))
        );
        // A → B：等 A 释放
        assert_eq!(
            plan_assignment(state(Some(1), None, 2), Some(2)),
            Some(state(Some(2), Some(1), 3))
        );
        // A → B 还没完成又改成 C：仍然等 A（B 从没拿到过）
        assert_eq!(
            plan_assignment(state(Some(2), Some(1), 3), Some(3)),
            Some(state(Some(3), Some(1), 4))
        );
        // A → B 还没完成又改回 A：撤销迁移，不用等
        assert_eq!(
            plan_assignment(state(Some(2), Some(1), 3), Some(1)),
            Some(state(Some(1), None, 4))
        );
        // 取消分派：等当前那台释放
        assert_eq!(
            plan_assignment(state(Some(2), None, 5), None),
            Some(state(None, Some(2), 6))
        );
        // 已经是目标：不变，epoch 不涨
        assert_eq!(plan_assignment(state(Some(2), None, 5), Some(2)), None);
        assert_eq!(plan_assignment(state(None, Some(2), 6), None), None);
    }

    #[tokio::test]
    async fn templates_round_trip_and_cannot_be_deleted_while_used() {
        let (_dir, pool) = pool().await;
        let fields: TemplateSpec = serde_json::from_value(serde_json::json!({
            "template_name": "t",
            "tid": 171,
            "account_mid": 42,
            "tags": ["a", "b"],
            "credits": [{ "username": "x", "uid": 1 }],
            "up_close_danmu": 1,
        }))
        .unwrap();
        let template = insert_template(&pool, &fields, 10).await.unwrap();
        assert_eq!(template.spec.account_mid, Some(42));
        assert_eq!(template.spec.tags, ["a", "b"]);
        assert_eq!(template.spec.tid, Some(171));

        let mut changed = fields.clone();
        changed.title = Some("标题".into());
        let updated = update_template(&pool, template.id, &changed, 20)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.spec.title.as_deref(), Some("标题"));
        assert_eq!(updated.updated_at, 20);
        assert!(
            update_template(&pool, 999, &changed, 20)
                .await
                .unwrap()
                .is_none()
        );

        let room = insert_room(&pool, &spec("u"), Some(template.id), None, false, 1)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            delete_template(&pool, template.id).await.unwrap(),
            DeleteTemplate::InUse(1)
        );
        assert!(matches!(
            delete_room(&pool, room.id, false, 1, 2).await.unwrap(),
            DeleteRoom::Deleted(_)
        ));
        assert_eq!(
            delete_template(&pool, template.id).await.unwrap(),
            DeleteTemplate::Deleted
        );
        assert_eq!(
            delete_template(&pool, template.id).await.unwrap(),
            DeleteTemplate::NotFound
        );
    }

    #[tokio::test]
    async fn room_urls_are_unique_and_specs_round_trip() {
        let (_dir, pool) = pool().await;
        let mut first = spec("https://live.example/1");
        first.postprocessor = serde_json::from_value(serde_json::json!(["rm"])).unwrap();
        first.excluded_keywords = Some(serde_json::json!(["回放"]));
        let room = insert_room(&pool, &first, None, None, true, 1)
            .await
            .unwrap()
            .unwrap();
        assert!(room.paused);
        assert_eq!(room.epoch, 1);
        let back = super::room(&pool, room.id).await.unwrap().unwrap();
        assert_eq!(
            serde_json::to_value(&back.spec).unwrap(),
            serde_json::to_value(&first).unwrap()
        );
        assert_eq!(
            insert_room(&pool, &first, None, None, false, 2)
                .await
                .unwrap()
                .unwrap_err(),
            UrlTaken
        );
        let second = insert_room(&pool, &spec("https://live.example/2"), None, None, false, 3)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            update_room(&pool, second.id, &first, None, 4)
                .await
                .unwrap()
                .unwrap_err(),
            UrlTaken
        );
        let paused = set_paused(&pool, second.id, true, 5)
            .await
            .unwrap()
            .unwrap();
        assert!(paused.paused);
    }

    #[tokio::test]
    async fn migration_is_confirmed_only_by_a_newer_ack_without_the_room() {
        let (_dir, pool) = pool().await;
        let a = node(&pool, "aa").await;
        let b = node(&pool, "bb").await;
        let room = insert_room(&pool, &spec("u"), None, Some(a), false, 1)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(desired_state(&pool, a).await.unwrap().0.len(), 1);

        let moved = assign_room(&pool, room.id, Some(b), 100, 2)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            (moved.node_id, moved.releasing_node_id, moved.epoch),
            (Some(b), Some(a), 2)
        );
        // 迁移中：两台的期望状态里都没有它
        assert!(desired_state(&pool, a).await.unwrap().0.is_empty());
        assert!(desired_state(&pool, b).await.unwrap().0.is_empty());

        // 更早的 Ack 不算；A 还持有也不算；B 的 Ack 不算
        assert!(
            confirm_releases(&pool, a, 100, &[], 3)
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            confirm_releases(&pool, a, 101, &[room.id], 3)
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            confirm_releases(&pool, b, 101, &[], 3)
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            confirm_releases(&pool, a, 101, &[], 3).await.unwrap(),
            [room.id]
        );
        let (rooms, _) = desired_state(&pool, b).await.unwrap();
        assert_eq!(rooms.len(), 1);
        assert_eq!(rooms[0].epoch, 2);
        assert_eq!(assigned_counts(&pool).await.unwrap().get(&b), Some(&1));
    }

    #[tokio::test]
    async fn force_skips_the_wait_and_deletes_wait_for_release() {
        let (_dir, pool) = pool().await;
        let a = node(&pool, "aa").await;
        let b = node(&pool, "bb").await;
        let room = insert_room(&pool, &spec("u"), None, Some(a), false, 1)
            .await
            .unwrap()
            .unwrap();
        assign_room(&pool, room.id, Some(b), 10, 2).await.unwrap();
        let forced = force_release(&pool, room.id, 3).await.unwrap().unwrap();
        assert_eq!(forced.releasing_node_id, None);
        assert_eq!(desired_state(&pool, b).await.unwrap().0.len(), 1);

        // 有节点在录：先标记删除，从期望状态里拿掉，URL 仍被占着
        let DeleteRoom::Releasing(deleted) =
            delete_room(&pool, room.id, false, 20, 4).await.unwrap()
        else {
            panic!()
        };
        assert_eq!(deleted.releasing_node_id, Some(b));
        assert!(deleted.deleted_at.is_some());
        assert!(desired_state(&pool, b).await.unwrap().0.is_empty());
        assert!(
            insert_room(&pool, &spec("u"), None, None, false, 5)
                .await
                .unwrap()
                .is_err()
        );
        assert!(
            assign_room(&pool, room.id, Some(a), 21, 5)
                .await
                .unwrap()
                .is_none()
        );
        // B 确认释放后整行删掉
        assert_eq!(
            confirm_releases(&pool, b, 21, &[], 6).await.unwrap(),
            [room.id]
        );
        assert!(super::room(&pool, room.id).await.unwrap().is_none());

        // 强制删除不等
        let other = insert_room(&pool, &spec("v"), None, Some(a), false, 7)
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(
            delete_room(&pool, other.id, true, 30, 8).await.unwrap(),
            DeleteRoom::Deleted(_)
        ));
        assert!(matches!(
            delete_room(&pool, other.id, true, 30, 8).await.unwrap(),
            DeleteRoom::NotFound
        ));
    }

    #[tokio::test]
    async fn removing_a_node_unassigns_its_rooms() {
        let (_dir, pool) = pool().await;
        let a = node(&pool, "aa").await;
        let b = node(&pool, "bb").await;
        let kept = insert_room(&pool, &spec("u1"), None, Some(a), false, 1)
            .await
            .unwrap()
            .unwrap();
        let moving = insert_room(&pool, &spec("u2"), None, Some(a), false, 1)
            .await
            .unwrap()
            .unwrap();
        let deleting = insert_room(&pool, &spec("u3"), None, Some(a), false, 1)
            .await
            .unwrap()
            .unwrap();
        assign_room(&pool, moving.id, Some(b), 10, 2).await.unwrap();
        delete_room(&pool, deleting.id, false, 10, 2).await.unwrap();

        let affected = unassign_node(&pool, a, 3).await.unwrap();
        assert_eq!(affected, [kept.id, moving.id, deleting.id]);
        let kept = super::room(&pool, kept.id).await.unwrap().unwrap();
        assert_eq!((kept.node_id, kept.epoch), (None, 2));
        let moving = super::room(&pool, moving.id).await.unwrap().unwrap();
        assert_eq!((moving.node_id, moving.releasing_node_id), (Some(b), None));
        assert!(super::room(&pool, deleting.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn node_accounts_are_replaced_wholesale() {
        let (_dir, pool) = pool().await;
        let a = node(&pool, "aa").await;
        let accounts = [
            Account {
                mid: 42,
                uname: "甲".into(),
            },
            Account {
                mid: 7,
                uname: String::new(),
            },
        ];
        replace_accounts(&pool, a, &accounts, 1).await.unwrap();
        assert!(node_has_account(&pool, a, 42).await.unwrap());
        replace_accounts(&pool, a, &accounts[1..], 2).await.unwrap();
        assert!(!node_has_account(&pool, a, 42).await.unwrap());
        let listed = list_accounts(&pool).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!((listed[0].mid, listed[0].reported_at), (7, 2));
        store::revoke_node(&pool, a, 3).await.unwrap();
        assert!(list_accounts(&pool).await.unwrap().is_empty());
    }
}
