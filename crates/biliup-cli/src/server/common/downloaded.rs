//! Session-end hooks consume the exact files emitted by the recording/processing
//! pipeline. Scripts can return replacements explicitly; never scan directories.
use super::util::FileValidator;
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::context::Context;
use crate::server::infrastructure::models::hook_step::HookStep;
use error_stack::{ResultExt, bail};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{
    OnceLock,
    atomic::{AtomicBool, Ordering},
};

#[derive(Debug, Default)]
pub(crate) struct SessionEnd {
    finished: OnceLock<(i64, bool)>,
    claimed: AtomicBool,
}

impl SessionEnd {
    pub(crate) fn finish(&self, timestamp: i64, checked: bool) {
        let _ = self.finished.set((timestamp, checked));
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HookFile {
    pub video: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub danmaku: Option<PathBuf>,
}

impl HookFile {
    pub(crate) fn from_paths(paths: Vec<PathBuf>) -> Self {
        Self {
            video: paths[0].clone(),
            danmaku: paths.get(1).cloned(),
        }
    }
    pub(crate) fn paths(&self) -> Vec<PathBuf> {
        std::iter::once(self.video.clone())
            .chain(self.danmaku.clone())
            .collect()
    }
}

pub(crate) fn configured(ctx: &Context) -> bool {
    ctx.live_streamer()
        .downloaded_processor
        .as_ref()
        .is_some_and(|steps| !steps.is_empty())
}

pub(crate) fn payload(files: &[HookFile], ctx: &Context) -> Vec<u8> {
    let info = ctx.streamer_info();
    let end = ctx
        .download_end
        .finished
        .get()
        .map(|(end, _)| *end)
        .unwrap_or_else(|| chrono::Utc::now().timestamp());
    serde_json::json!({
        "name": info.name, "url": info.url, "room_title": info.title,
        "start_time": info.date.timestamp(), "end_time": end.max(info.date.timestamp()),
        "file_list": files.iter().map(|file| &file.video).collect::<Vec<_>>(),
    })
    .to_string()
    .into_bytes()
}

/// Called once after the segment stream closes, including zero-file sessions.
/// A failed processing check prevents session hooks from touching unchecked media.
pub(crate) async fn run(mut files: Vec<HookFile>, ctx: &Context) -> AppResult<Vec<HookFile>> {
    if !configured(ctx)
        || ctx
            .download_end
            .finished
            .get()
            .is_some_and(|(_, checked)| !checked)
    {
        return Ok(files);
    }
    if ctx.download_end.claimed.swap(true, Ordering::AcqRel) {
        // Never upload stale inputs after an earlier invocation replaced them
        // or failed. A duplicate completion event must not publish twice.
        return Ok(Vec::new());
    }
    for step in ctx
        .live_streamer()
        .downloaded_processor
        .as_deref()
        .unwrap_or_default()
    {
        let stdin = match step {
            HookStep::Run { json: true, .. } => payload(&files, ctx),
            HookStep::Run { .. } => files
                .iter()
                .flat_map(HookFile::paths)
                .map(|path| path.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("\n")
                .into_bytes(),
            _ => bail!(AppError::Custom("downloaded_processor 仅支持 run".into())),
        };
        if let Some(output) = step.execute_downloaded(&stdin).await? {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Output {
                files: Vec<HookFile>,
            }
            files = serde_json::from_str::<Output>(&output)
                .change_context(AppError::Custom(
                    "BILIUP_OUTPUT 必须是 {\"files\":[{\"video\":\"成品.mp4\",\"danmaku\":null}]}"
                        .into(),
                ))?
                .files;
        }
        validate(&files)?;
    }
    Ok(files)
}

fn validate(files: &[HookFile]) -> AppResult<()> {
    let mut unique = std::collections::HashSet::new();
    for file in files {
        if crate::server::common::processing::masking::is_unmasked(&file.video) {
            bail!(AppError::Custom(
                "下载后脚本返回了未遮挡录像，停止上传".into()
            ));
        }
        FileValidator::validate_completed_video(&file.video).change_context(AppError::Custom(
            format!(
                "下载后脚本的输出文件不存在或格式不支持: {}；改变路径时请输出 BILIUP_OUTPUT JSON",
                file.video.display()
            ),
        ))?;
        let path = std::fs::canonicalize(&file.video).change_context(AppError::Unknown)?;
        if !unique.insert(path) {
            bail!(AppError::Custom(
                "下载后脚本返回了空文件、目录或重复视频".into()
            ));
        }
        if let Some(path) = &file.danmaku {
            if !path.is_file()
                || !path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("xml"))
            {
                bail!(AppError::Custom("下载后脚本返回的弹幕文件无效".into()));
            }
        }
    }
    Ok(())
}
