//! 节点上报给控制面的事件（F4）：录制出错、投稿失败。
//!
//! 录制与投稿流程里各有一行调用（`common/download.rs`、`common/upload.rs`）。事件先进进程内的
//! 广播通道，由节点代理转成 [`NodeMessage::Event`](super::protocol::NodeMessage::Event) 发出去；
//! 断线期间最多攒 [`CAPACITY`] 条，更早的丢掉。
//!
//! 通道只在节点代理启动时创建。单机与没启用「本机」节点（[`super::local`]）的控制面进程里 [`SINK`]
//! 一直是空的，两处调用只读一次 `OnceLock`，不分配、不记日志、不改任何状态。启用之后与节点相同：
//! 本进程里所有房间（含控制面自己的主播）的事件都记在「本机」名下。

use super::now_ms;
use super::protocol::{EVENT_RECORDING_ERROR, EVENT_UPLOAD_FAILED, Event, RoomEvent};
use crate::server::core::downloader::DownloadStatus;
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::context::Context;
use std::borrow::Cow;
use std::sync::OnceLock;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

/// 断线期间最多攒多少条
pub const CAPACITY: usize = 64;
/// 错误信息最多带多少个字符
pub const MAX_ERROR_CHARS: usize = 500;

static SINK: OnceLock<broadcast::Sender<Event>> = OnceLock::new();

/// 节点代理启动时调用；之后录制与投稿流程里的事件才会被收下
pub fn subscribe() -> broadcast::Receiver<Event> {
    SINK.get_or_init(|| broadcast::channel(CAPACITY).0)
        .subscribe()
}

/// 通道是整个测试进程共用的，正在跑的节点代理会把别的测试发的事件也转给自己的控制面。
/// 往通道里发事件的测试与数告警条数的测试都先拿这把锁，互不串扰
#[cfg(test)]
pub(crate) async fn test_guard() -> tokio::sync::MutexGuard<'static, ()> {
    static LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    LOCK.lock().await
}

/// 测试里绕过录制与投稿流程直接发一条
#[cfg(test)]
pub(crate) fn inject(event: Event) {
    if let Some(sink) = SINK.get() {
        let _ = sink.send(event);
    }
}

fn emit(kind: &str, ctx: &Context, error: String) {
    let streamer = ctx.live_streamer();
    emit_room(kind, &streamer.url, &streamer.remark, &error);
}

fn emit_room(kind: &str, url: &str, remark: &str, error: &str) {
    let Some(sink) = SINK.get() else {
        return;
    };
    let detail = RoomEvent {
        url: url.to_string(),
        remark: remark.to_string(),
        error: scrub(error),
    };
    let _ = sink.send(Event {
        kind: kind.to_string(),
        at: now_ms(),
        detail: serde_json::to_value(detail).unwrap_or_default(),
    });
}

/// 一次拉流结束（`common/download.rs`）：出错才上报；用户停止、迁移等取消不算
pub fn recording_finished(
    ctx: &Context,
    result: &AppResult<DownloadStatus>,
    token: &CancellationToken,
) {
    if SINK.get().is_none() || token.is_cancelled() {
        return;
    }
    let error = match result {
        Ok(DownloadStatus::Error(message)) => message.clone(),
        Err(report) => format!("{report:#}"),
        Ok(_) => return,
    };
    emit(EVENT_RECORDING_ERROR, ctx, error);
}

/// 一场投稿流程失败（`common/upload.rs`）。没有上传模板或用 `Noop` 上传器时失败的只会是后处理，不算投稿失败
pub fn upload_failed(ctx: &Context, report: &error_stack::Report<AppError>) {
    if SINK.get().is_none() {
        return;
    }
    match ctx.upload_config() {
        Some(config) if !config.is_noop_uploader() => {
            emit(EVENT_UPLOAD_FAILED, ctx, format!("{report:#}"))
        }
        _ => {}
    }
}

/// 一主一备里要人看一眼的场次（待人工处理、备机自己投稿失败）：备机的投稿不经过投稿流程，按投稿失败上报
pub fn ha_attention(url: &str, remark: &str, message: &str) {
    emit_room(EVENT_UPLOAD_FAILED, url, remark, message);
}

/// 去掉链接里的查询串（直链常带签名），本机的绝对路径只留文件名，合并空白，截到 [`MAX_ERROR_CHARS`]
pub fn scrub(text: &str) -> String {
    let mut out = String::with_capacity(text.len().min(MAX_ERROR_CHARS));
    let mut rest = text;
    while let Some(start) = rest.find("://") {
        let scheme_start = rest[..start]
            .rfind(|c: char| !c.is_ascii_alphanumeric() && c != '+' && c != '-' && c != '.')
            .map_or(0, |i| i + 1);
        let end = rest[start..]
            .find(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | ')' | ']'))
            .map_or(rest.len(), |i| start + i);
        out.push_str(&rest[..scheme_start]);
        let link = &rest[scheme_start..end];
        match link.find(['?', '#']) {
            Some(cut) => {
                out.push_str(&link[..cut]);
                out.push_str("?…");
            }
            None => out.push_str(link),
        }
        rest = &rest[end..];
    }
    out.push_str(rest);
    let collapsed = out
        .split_whitespace()
        .map(shorten_path)
        .collect::<Vec<_>>()
        .join(" ");
    if collapsed.chars().count() <= MAX_ERROR_CHARS {
        return collapsed;
    }
    let mut cut: String = collapsed.chars().take(MAX_ERROR_CHARS - 1).collect();
    cut.push('…');
    cut
}

/// 目录结构和凭据文件的位置不出节点：投稿失败的原因里常带 Cookie 文件的完整路径，
/// 而心跳里的房间也不带凭据路径
fn shorten_path(word: &str) -> Cow<'_, str> {
    let start = word.len() - word.trim_start_matches(['"', '\'', '(', '[', '<']).len();
    let body = word[start..].trim_end_matches(['"', '\'', ')', ']', '>', ':', ',', ';', '.']);
    let end = start + body.len();
    let bytes = body.as_bytes();
    let absolute = body.starts_with('/')
        || body.starts_with("\\\\")
        || (bytes.len() > 2
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && matches!(bytes[2], b'\\' | b'/'));
    match body.rfind(['/', '\\']) {
        Some(cut) if absolute && cut > 0 && cut + 1 < body.len() => Cow::Owned(format!(
            "{}…/{}{}",
            &word[..start],
            &body[cut + 1..],
            &word[end..]
        )),
        _ => Cow::Borrowed(word),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::config::Config;
    use crate::server::errors::AppError;
    use crate::server::infrastructure::context::Worker;
    use serde_json::json;
    use std::sync::{Arc, RwLock};

    fn context(url: &str, uploader: Option<&str>) -> Context {
        let streamer =
            serde_json::from_value(json!({ "id": 1, "url": url, "remark": "房间" })).unwrap();
        let upload = uploader.map(|uploader| {
            serde_json::from_value(json!({
                "id": 1, "template_name": "t", "tags": [], "uploader": uploader,
            }))
            .unwrap()
        });
        let worker = Arc::new(Worker::new(
            streamer,
            upload,
            Arc::new(RwLock::new(Config::default())),
            Default::default(),
        ));
        let stream = serde_json::from_value(json!({
            "name": "n", "url": url, "title": "t", "date": "2026-01-01T00:00:00Z",
            "live_cover_url": "", "raw_stream_url": "http://127.0.0.1:9/x.flv", "platform": "p",
            "stream_headers": {}, "suffix": "flv", "danmaku": null, "downloader_hint": "StreamGears",
            "runtime_options": null,
        }))
        .unwrap();
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect_lazy("sqlite::memory:")
            .unwrap();
        Context::new(1, worker, pool, stream)
    }

    /// 同一进程里别的测试（带节点代理的端到端测试）也可能往通道里发，按地址挑自己的
    fn drain(events: &mut broadcast::Receiver<Event>, url: &str) -> Vec<(String, RoomEvent)> {
        let mut mine = Vec::new();
        while let Ok(event) = events.try_recv() {
            let detail: RoomEvent = serde_json::from_value(event.detail).unwrap_or_default();
            if detail.url == url {
                mine.push((event.kind, detail));
            }
        }
        mine
    }

    #[tokio::test]
    async fn hooks_report_errors_but_not_cancellations_or_postprocessing() {
        let _guard = test_guard().await;
        let mut events = subscribe();
        let url = "https://events.example/hooks";
        let token = CancellationToken::new();
        let ctx = context(url, Some("bili_web"));

        recording_finished(&ctx, &Ok(DownloadStatus::StreamEnded), &token);
        recording_finished(&ctx, &Ok(DownloadStatus::SegmentCompleted), &token);
        assert!(drain(&mut events, url).is_empty());
        recording_finished(
            &ctx,
            &Ok(DownloadStatus::Error(
                "mesio error: https://cdn/x?sign=1".into(),
            )),
            &token,
        );
        let failed: AppResult<DownloadStatus> = Err(error_stack::Report::new(AppError::Custom(
            "no template".into(),
        )));
        recording_finished(&ctx, &failed, &token);
        let got = drain(&mut events, url);
        assert_eq!(got.len(), 2, "{got:?}");
        assert_eq!(got[0].0, EVENT_RECORDING_ERROR);
        assert_eq!(got[0].1.remark, "房间");
        assert_eq!(got[0].1.error, "mesio error: https://cdn/x?…");
        assert!(got[1].1.error.contains("no template"), "{got:?}");

        token.cancel();
        recording_finished(&ctx, &Ok(DownloadStatus::Error("killed".into())), &token);
        assert!(drain(&mut events, url).is_empty());

        let report = error_stack::Report::new(AppError::Custom("open cookies file: x.json".into()))
            .change_context(AppError::Custom("upload".into()));
        upload_failed(&ctx, &report);
        let got = drain(&mut events, url);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].0, EVENT_UPLOAD_FAILED);
        assert_eq!(got[0].1.error, "upload: open cookies file: x.json");
        // 没有上传模板、Noop 上传器：失败的只会是后处理
        upload_failed(&context(url, None), &report);
        upload_failed(&context(url, Some("Noop")), &report);
        assert!(drain(&mut events, url).is_empty());
    }

    #[test]
    fn signed_links_lose_their_query() {
        assert_eq!(
            scrub("mesio error: GET https://cdn.example/live/a.flv?sign=abc&expire=1 failed"),
            "mesio error: GET https://cdn.example/live/a.flv?… failed"
        );
        assert_eq!(
            scrub("x(\"http://h/p#frag\") y http://plain/path"),
            "x(\"http://h/p?…\") y http://plain/path"
        );
        assert_eq!(scrub("no links here"), "no links here");
        assert_eq!(scrub("a://"), "a://");
    }

    #[test]
    fn local_paths_keep_only_the_file_name() {
        assert_eq!(
            scrub("login by cookies file failed: /home/u/biliup/cookies/1001.json: dns error"),
            "login by cookies file failed: …/1001.json: dns error"
        );
        assert_eq!(
            scrub("open \"C:\\biliup\\cookies.json\" failed"),
            "open \"…/cookies.json\" failed"
        );
        assert_eq!(scrub("(/data/rec/a.flv), / and /x"), "(…/a.flv), / and /x");
        assert_eq!(
            scrub("url (http://h:1/live/a.m3u8?t=1) cookies.json a/b"),
            "url (http://h:1/live/a.m3u8?…) cookies.json a/b"
        );
    }

    #[test]
    fn whitespace_is_collapsed_and_long_errors_are_cut() {
        assert_eq!(scrub("a\n\n  b\tc"), "a b c");
        let long = "错".repeat(MAX_ERROR_CHARS + 10);
        let cut = scrub(&long);
        assert_eq!(cut.chars().count(), MAX_ERROR_CHARS);
        assert!(cut.ends_with('…'));
    }
}
