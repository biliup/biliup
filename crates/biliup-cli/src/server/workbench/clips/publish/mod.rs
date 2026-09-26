//! 发布：按上传模板把切片投到 B 站。
//!
//! - 模板：切片自己选的（`clips.template_id`），没选就用主播绑定的；工作台可以覆盖标题、简介、标签、
//!   封面、分区、定时（[`StudioOverride`]，存在 `clips.studio_override`）；
//! - 标题、简介支持模板原有的 `{streamer}` `{title}` `{url}` 和 strftime（按开播时间），另加
//!   `{clip_title}`（切片标题）、`{clip_time}`（切片入点的墙钟时间）；
//! - 版权一律按转载提交，来源用模板里填的，没填就是直播间地址：切片是别人直播的片段，模板选了
//!   「自制」也不跟随（[`enforce_reprint`]）；
//! - 上传与投稿在 [`queue`] 里排队，一次只做一个。

pub mod queue;

use crate::server::common::upload::{desc_with_credits, resolve_source, studio_from_template};
use crate::server::common::util::Recorder;
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::models::StreamerInfo;
use crate::server::infrastructure::models::upload_streamer::UploadStreamer;
use biliup::bilibili::{Credit, Studio, Video};
use chrono::format::{Item, StrftimeItems};
use chrono::{Local, TimeZone};
use ormlite::Model;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// B 站的限制。
pub const MAX_TITLE_CHARS: usize = 80;
pub const MAX_DESC_CHARS: usize = 2000;
pub const MAX_TAGS: usize = 12;
pub const MAX_TAG_CHARS: usize = 20;
/// 一个多 P 稿件最多合几个切片。
pub const MAX_PARTS: usize = 100;
/// 转载。
pub const COPYRIGHT_REPRINT: u8 = 2;
/// 定时发布的范围：至少 4 小时之后、最多 15 天之内（与上传模板页的定时选项一致）。
pub const MIN_SCHEDULE_HOURS: u64 = 4;
pub const MAX_SCHEDULE_DAYS: u64 = 15;

/// 工作台选的定时发布时间不在范围内时给人看的原因。
pub fn schedule_problem(dtime: u32, now_unix: u64) -> Option<String> {
    let at = crate::server::common::upload::scheduled_publish_ts(Some(dtime), now_unix)? as u64;
    if at < now_unix + MIN_SCHEDULE_HOURS * 3600 {
        Some(format!(
            "定时发布要在 {MIN_SCHEDULE_HOURS} 小时之后：在发布设置里改一个时间"
        ))
    } else if at > now_unix + MAX_SCHEDULE_DAYS * 86400 {
        Some(format!(
            "定时发布最多在 {MAX_SCHEDULE_DAYS} 天之内：在发布设置里改一个时间"
        ))
    } else {
        None
    }
}

/// 工作台上覆盖模板的部分；没给的字段沿用模板。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StudioOverride {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desc: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tid: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tid_v2: Option<u32>,
    /// 定时发布的 Unix 秒；`0` = 立即发布（不用模板里的定时）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dtime: Option<u32>,
    /// 用切片自己的封面文件（[`cover_path`]）；`None` = 模板的封面。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover: Option<Cover>,
}

/// 切片封面从哪来（文件都存成 [`cover_path`]）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "lowercase")]
pub enum Cover {
    /// 从录像取的一帧，`t_ms` 是场次时间。
    Frame { t_ms: i64 },
    /// 用户上传的图片。
    Upload,
    /// 开播时记下的直播间封面。
    Live,
}

impl StudioOverride {
    pub fn parse(json: Option<&str>) -> Self {
        json.and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default()
    }

    pub fn to_json(&self) -> Option<String> {
        (*self != Self::default()).then(|| serde_json::to_string(self).unwrap_or_default())
    }

    /// 检查长度和数量；给人看的原因。
    pub fn validate(&self) -> Result<(), String> {
        if let Some(title) = &self.title {
            if title.chars().count() > MAX_TITLE_CHARS {
                return Err(format!("稿件标题最多 {MAX_TITLE_CHARS} 个字"));
            }
            if title.chars().any(char::is_control) {
                return Err("稿件标题里不能有换行或控制字符".into());
            }
        }
        if let Some(desc) = &self.desc
            && desc.chars().count() > MAX_DESC_CHARS
        {
            return Err(format!("简介最多 {MAX_DESC_CHARS} 个字"));
        }
        if let Some(tags) = &self.tags {
            if tags.len() > MAX_TAGS {
                return Err(format!("标签最多 {MAX_TAGS} 个"));
            }
            if let Some(tag) = tags
                .iter()
                .find(|t| t.chars().count() > MAX_TAG_CHARS || t.contains(','))
            {
                return Err(format!(
                    "标签「{tag}」太长或含逗号：每个标签最多 {MAX_TAG_CHARS} 个字"
                ));
            }
        }
        Ok(())
    }
}

/// 切片封面文件：场次的切片目录（[`ClipExports::dir`](super::export::ClipExports::dir)）下的
/// `<切片>-cover.jpg`。不用 `<切片>.` 开头，重新导出时不会被当成旧产物删掉。
pub fn cover_file(session_dir: &Path, clip_id: i64) -> PathBuf {
    session_dir.join(format!("{clip_id}-cover.jpg"))
}

/// 一个切片填进标题模板的值。
#[derive(Debug, Clone)]
pub struct ClipVars {
    pub id: i64,
    pub title: String,
    /// 入点的墙钟时间（Unix 毫秒）。
    pub at_ms: i64,
}

impl ClipVars {
    /// 没起名的切片叫「切片 #id」，与发布界面里列出切片时的叫法一致。
    fn title_text(&self) -> String {
        if self.title.trim().is_empty() {
            format!("切片 #{}", self.id)
        } else {
            self.title.clone()
        }
    }

    fn time_text(&self) -> String {
        Local
            .timestamp_millis_opt(self.at_ms)
            .single()
            .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_default()
    }
}

fn escape(value: &str) -> String {
    value.replace('%', "%%")
}

/// 填模板：先换 `{...}` 变量（值里的 `%` 转义，不会被当成时间格式），再按开播时间展开 strftime。
/// 模板里有写错的 `%` 格式时不展开，原样保留，不会像整场投稿那样 panic。
pub fn render(template: &str, info: &StreamerInfo, clip: &ClipVars) -> String {
    let text = template
        .replace("{streamer}", &escape(&info.name))
        .replace("{title}", &escape(&info.title))
        .replace("{url}", &escape(&info.url))
        .replace("{clip_title}", &escape(&clip.title_text()))
        .replace("{clip_time}", &escape(&clip.time_text()));
    let items: Vec<Item> = StrftimeItems::new(&text).collect();
    if items.iter().any(|item| matches!(item, Item::Error)) {
        return text.replace("%%", "%");
    }
    info.date
        .with_timezone(&Local)
        .format_with_items(items.iter())
        .to_string()
}

fn has_clip_vars(template: &str) -> bool {
    template.contains("{clip_title}") || template.contains("{clip_time}")
}

/// 稿件标题的模板：工作台填了用工作台的；模板标题里有切片变量时用模板的（整场录播的标题模板不适合切片，
/// 同一场的切片会同名）；都没有时用切片标题，切片没起名就是「主播 时间」。多 P 合集用 `combined`。
pub fn title_template(
    template: &UploadStreamer,
    over: &StudioOverride,
    clip_title: &str,
    combined: bool,
) -> String {
    if let Some(title) = over.title.as_deref().filter(|t| !t.trim().is_empty()) {
        return title.to_string();
    }
    if let Some(title) = template.title.as_deref().filter(|t| has_clip_vars(t)) {
        return title.to_string();
    }
    if combined {
        "{streamer} 切片合集 {clip_time}".into()
    } else if clip_title.trim().is_empty() {
        "{streamer} {clip_time}".into()
    } else {
        "{clip_title}".into()
    }
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_string()
    } else {
        Video::truncate_title(text, max)
    }
}

/// 一个稿件要投的内容（每个切片一个稿件时只有一个 P）。
#[derive(Debug, Clone)]
pub struct Archive {
    pub template: UploadStreamer,
    pub over: StudioOverride,
    pub info: StreamerInfo,
    /// 各 P 的切片，按时间顺序。
    pub parts: Vec<ClipVars>,
    /// 封面文件（切片封面）；`None` 时用模板的 `cover_path`。
    pub cover: Option<PathBuf>,
}

/// 预览和投稿用的最终字段。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Rendered {
    pub title: String,
    pub desc: String,
    pub tags: Vec<String>,
    pub tid: Option<u16>,
    pub tid_v2: Option<u32>,
    pub copyright: u8,
    pub source: String,
    /// 定时发布的 Unix 秒。
    pub dtime: Option<u32>,
    pub part_titles: Vec<String>,
    /// 模板本来选的是自制：界面上提示「切片按转载投」。
    pub template_self_made: bool,
    pub template_name: String,
}

impl Archive {
    fn first(&self) -> ClipVars {
        self.parts.first().cloned().unwrap_or(ClipVars {
            id: 0,
            title: String::new(),
            at_ms: self.info.date.timestamp_millis(),
        })
    }

    pub fn part_title(&self, index: usize) -> String {
        let clip = &self.parts[index];
        let title = if clip.title.trim().is_empty() {
            format!("P{} {}", index + 1, clip.time_text())
        } else {
            clip.title.clone()
        };
        truncate(&title, MAX_TITLE_CHARS)
    }

    /// 覆盖值合进模板之后的模板（标题、简介另行渲染）。
    fn effective_template(&self) -> UploadStreamer {
        let mut template = self.template.clone();
        if let Some(tags) = &self.over.tags {
            template.tags = tags.clone();
        }
        if self.over.tid.is_some() {
            template.tid = self.over.tid;
            template.tid_v2 = self.over.tid_v2;
        }
        match self.over.dtime {
            Some(0) => template.dtime = None,
            Some(t) => template.dtime = Some(t),
            None => {}
        }
        if let Some(cover) = &self.cover {
            template.cover_path = Some(cover.to_string_lossy().into_owned());
        }
        // 标题、简介在这里渲染；不交给 Recorder（它对模板里的 `%` 不做转义）
        template.title = Some(String::new());
        template.description = Some(String::new());
        template
    }

    /// 渲染后的简介，`@credit` 还没展开。
    fn desc_text(&self) -> String {
        let desc_tpl = self
            .over
            .desc
            .clone()
            .or_else(|| self.template.description.clone())
            .unwrap_or_default();
        truncate(
            &render(&desc_tpl, &self.info, &self.first()),
            MAX_DESC_CHARS,
        )
    }

    /// 提交用的简介和 `desc_v2`：`@credit` 按模板的 credits 展开，与整场投稿相同。
    fn desc_with_credits(&self) -> (String, Option<Vec<Credit>>) {
        desc_with_credits(self.desc_text(), self.template.credits.as_ref())
    }

    pub fn render(&self) -> Rendered {
        let first = self.first();
        let title_tpl = title_template(
            &self.template,
            &self.over,
            &first.title,
            self.parts.len() > 1,
        );
        let effective = self.effective_template();
        let tags = effective
            .tags
            .iter()
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect();
        Rendered {
            title: truncate(
                render(&title_tpl, &self.info, &first).trim(),
                MAX_TITLE_CHARS,
            ),
            desc: self.desc_with_credits().0,
            tags,
            tid: effective.tid,
            tid_v2: effective.tid_v2,
            copyright: COPYRIGHT_REPRINT,
            source: resolve_source(self.template.copyright_source.as_deref(), &self.info.url),
            dtime: crate::server::common::upload::scheduled_publish_ts(
                effective.dtime,
                chrono::Utc::now().timestamp() as u64,
            ),
            part_titles: (0..self.parts.len()).map(|i| self.part_title(i)).collect(),
            template_self_made: self.template.copyright == Some(1),
            template_name: self.template.template_name.clone(),
        }
    }

    /// 发布前的检查；给人看的原因。
    pub fn problem(&self) -> Option<String> {
        let rendered = self.render();
        if rendered.title.is_empty() {
            return Some("稿件标题是空的，在发布设置里填一个".into());
        }
        if rendered.tags.is_empty() {
            return Some("B 站要求至少一个标签：在发布设置或上传模板里加上".into());
        }
        if let Some(dtime) = self.over.dtime.filter(|&t| t != 0)
            && let Some(problem) = schedule_problem(dtime, chrono::Utc::now().timestamp() as u64)
        {
            return Some(problem);
        }
        if self.template.is_noop_uploader() {
            return Some(format!(
                "上传模板「{}」的上传方式是 Noop（不上传），换一个模板",
                self.template.template_name
            ));
        }
        None
    }

    /// 按模板拼出稿件（`cover` 仍是本地路径），覆盖值合进去，再强制转载。
    pub fn studio(&self, videos: Vec<Video>) -> Studio {
        let rendered = self.render();
        let recorder = Recorder::new(Some(String::new()), self.info.clone());
        let mut studio = studio_from_template(&self.effective_template(), videos, &recorder);
        let (desc, desc_v2) = self.desc_with_credits();
        studio.title = rendered.title;
        studio.desc = desc;
        studio.desc_v2 = desc_v2;
        studio.tag = rendered.tags.join(",");
        enforce_reprint(&mut studio, &self.template, &self.info.url);
        studio
    }
}

/// 切片一律按转载投：版权 = 2，来源 = 模板的来源，没填就是直播间地址。`extra_fields` 里
/// 同名的键一并去掉，免得序列化出两个 `copyright`。
pub fn enforce_reprint(studio: &mut Studio, template: &UploadStreamer, live_url: &str) {
    studio.copyright = COPYRIGHT_REPRINT;
    studio.source = resolve_source(template.copyright_source.as_deref(), live_url);
    if let Some(extra) = studio.extra_fields.as_mut() {
        extra.remove("copyright");
        extra.remove("source");
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    #[error("{0}")]
    Invalid(String),
    #[error("数据库出错：{0}")]
    Db(#[from] sqlx::Error),
}

/// 切片要用的上传模板：切片选了用切片的，否则用场次所属主播绑定的。
pub async fn template_for(
    pool: &ConnectionPool,
    session_id: i64,
    template_id: Option<i64>,
) -> Result<UploadStreamer, ResolveError> {
    let id = match template_id {
        Some(id) => Some(id),
        None => sqlx::query_scalar::<_, Option<i64>>(
            "SELECT l.upload_streamers_id FROM stream_sessions s
             JOIN livestreamers l ON l.id = s.streamer_id WHERE s.id = ?",
        )
        .bind(session_id)
        .fetch_optional(pool)
        .await?
        .flatten(),
    };
    let Some(id) = id else {
        return Err(ResolveError::Invalid(
            "这个主播没有绑定上传模板：在发布设置里选一个模板".into(),
        ));
    };
    match UploadStreamer::fetch_one(id, pool).await {
        Ok(template) => Ok(template),
        Err(ormlite::Error::SqlxError(sqlx::Error::RowNotFound)) => Err(ResolveError::Invalid(
            "选的上传模板已经被删掉了，换一个模板".into(),
        )),
        Err(ormlite::Error::SqlxError(e)) => Err(e.into()),
        Err(e) => Err(ResolveError::Invalid(format!("读上传模板出错：{e}"))),
    }
}

/// 场次的开播信息（主播名、直播间地址、标题、开播时间）。
pub async fn session_info(
    pool: &ConnectionPool,
    session_id: i64,
) -> Result<StreamerInfo, ResolveError> {
    match StreamerInfo::fetch_one(session_id, pool).await {
        Ok(info) => Ok(info),
        Err(ormlite::Error::SqlxError(sqlx::Error::RowNotFound)) => {
            Err(ResolveError::Invalid("场次不存在".into()))
        }
        Err(ormlite::Error::SqlxError(e)) => Err(e.into()),
        Err(e) => Err(ResolveError::Invalid(format!("读场次出错：{e}"))),
    }
}

#[cfg(test)]
mod tests;
