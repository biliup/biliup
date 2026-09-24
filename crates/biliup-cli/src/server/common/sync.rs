use crate::server::common::upload::{
    UploadContext, aid_from_submit, build_studio, complete_byte_stream, edit_to_bilibili,
    execute_postprocessor, initialize_upload_context, submit_to_bilibili, upload_byte_stream_parts,
    upload_single_file,
};
use crate::server::common::util::Recorder;
use crate::server::core::downloader::sync_downloader::{
    PumpResult, SyncDownloader, align_file_size, pump_chunks,
};
use crate::server::core::downloader::{DownloadConfig, DownloadStatus};
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::context::{Context, Stage, WorkerStatus};
use crate::server::infrastructure::models::upload_streamer::UploadStreamer;
use crate::server::workbench::recorder::{ClosedSegment, RecorderHandle};
use crate::server::workbench::retention::{self, Retention};
use biliup::bilibili::{BiliBili, Studio, Video};
use biliup::uploader::line::UploadedStream;
use bytes::Bytes;
use error_stack::ResultExt;
use futures::StreamExt;
use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

const MIN_MEDIA_BYTES: u64 = 100;
const MAX_EMPTY_RETRIES: u32 = 5;
const STREAM_QUEUE_CAPACITY: usize = 6;
const MAX_UNCOMMITTED_SEGMENTS: usize = 2;

#[derive(Debug, Clone)]
struct SegmentFile {
    path: PathBuf,
    keep: bool,
}

#[derive(Default)]
pub(crate) struct SyncSession {
    studio: Option<Studio>,
    videos: Vec<Video>,
    files: Vec<SegmentFile>,
    confirmed_parts: usize,
    cleaned_parts: usize,
    first_submit_uncertain: bool,
    postprocess_paths: Vec<PathBuf>,
    recorder: Option<RecorderHandle>,
    retention: Option<Retention>,
}

impl SyncSession {
    pub(crate) fn committed_parts(&self) -> usize {
        self.confirmed_parts
    }

    /// 把分段记进切片工作台的场次时间轴。
    pub(crate) fn set_recorder(&mut self, recorder: RecorderHandle) {
        self.recorder = Some(recorder);
    }

    /// 投稿确认后删临时分段时，按切片工作台的引用与「投稿后保留录像」推迟删除。
    pub(crate) fn set_retention(&mut self, retention: Retention) {
        self.retention = Some(retention);
    }
}

impl Drop for SyncSession {
    /// 外层放弃重试后会话随任务销毁；已入列但没确认投稿的分段文件必须留下线索，
    /// 否则用户只会看到稿件少了几 P，却找不到录好的视频在哪。
    fn drop(&mut self) {
        let unconfirmed: Vec<String> = self.files[self.cleaned_parts.min(self.files.len())..]
            .iter()
            .map(|file| file.path.display().to_string())
            .collect();
        if !unconfirmed.is_empty() {
            warn!(
                aid = self.studio.as_ref().and_then(|studio| studio.aid),
                files = ?unconfirmed,
                "边录边传会话结束时仍有分段未确认投稿，文件已保留，可手动补传"
            );
        }
    }
}

struct PendingSegment {
    seq: u64,
    file: SegmentFile,
    pump: PumpResult,
    upload: JoinHandle<AppResult<UploadedStream>>,
}

struct ReadySegment {
    seq: u64,
    file: SegmentFile,
    video: Video,
}

pub(crate) async fn run_sync_pipeline(
    downloader: &SyncDownloader,
    token: CancellationToken,
    ctx: &Context,
    download_config: DownloadConfig,
    session: Arc<Mutex<SyncSession>>,
) -> AppResult<DownloadStatus> {
    let Some(upload_config) = ctx.upload_config().clone() else {
        return Err(AppError::Custom("边录边传需要先为主播设定上传模板".into()).into());
    };
    if upload_config.is_noop_uploader() {
        return Err(AppError::Custom("边录边传不支持 Noop 上传器".into()).into());
    }

    ctx.change_status(Stage::Upload, WorkerStatus::Pending)
        .await;
    let result = run_inner(
        downloader,
        token,
        ctx,
        download_config,
        &upload_config,
        session,
    )
    .await;
    ctx.change_status(Stage::Upload, WorkerStatus::Idle).await;
    result
}

async fn run_inner(
    downloader: &SyncDownloader,
    token: CancellationToken,
    ctx: &Context,
    download_config: DownloadConfig,
    upload_config: &UploadStreamer,
    session: Arc<Mutex<SyncSession>>,
) -> AppResult<DownloadStatus> {
    let mut upload_ctx =
        initialize_upload_context(&ctx.config(), ctx.stateless_client(), upload_config).await?;
    upload_ctx.threads = 3;
    let total_size = align_file_size(download_config.file_size);
    info!(max_file_size_mb = total_size / 1024 / 1024, "启动边录边传");

    let mut recorder = ctx.recorder(ctx.streamer_info().clone());
    recorder.filename_prefix = upload_config.title.clone();
    let prefix = download_config.recorder.generate_filename("mkv");
    let save_dir = ctx.config().sync_save_dir.clone().map(PathBuf::from);
    let mut pending = VecDeque::new();
    let mut ready = BTreeMap::new();

    // 上一轮若 edit 失败，先用同一份有序 videos 快照重试，不能先录新段。
    commit_unconfirmed(
        &session,
        &upload_ctx.bilibili,
        upload_config,
        ctx.config().submit_api.as_deref(),
        &recorder,
    )
    .await?;

    let result = run_loop(
        downloader,
        &token,
        ctx,
        &download_config,
        upload_config,
        &upload_ctx,
        &recorder,
        &prefix,
        save_dir.as_ref(),
        total_size,
        &session,
        &mut pending,
        &mut ready,
    )
    .await;

    let _ = downloader.stop().await;
    // 正常结束（含取消）时 run_loop 已排空队列；这里只处理出错后残留的分段。
    abandon_pending(&mut pending, &mut ready).await;

    // 取消（停止/暂停）同样是“本场录制结束”，已投稿分段的后处理照常执行
    if result.is_ok() {
        let paths = {
            let mut state = session.lock().await;
            std::mem::take(&mut state.postprocess_paths)
        };
        if !paths.is_empty() {
            execute_postprocessor(paths, ctx).await?;
        }
    }
    result
}

#[allow(clippy::too_many_arguments)]
async fn run_loop(
    downloader: &SyncDownloader,
    token: &CancellationToken,
    ctx: &Context,
    download_config: &DownloadConfig,
    upload_config: &UploadStreamer,
    upload_ctx: &UploadContext,
    recorder: &Recorder,
    prefix: &str,
    save_dir: Option<&PathBuf>,
    total_size: u64,
    session: &Arc<Mutex<SyncSession>>,
    pending: &mut VecDeque<PendingSegment>,
    ready: &mut BTreeMap<u64, ReadySegment>,
) -> AppResult<DownloadStatus> {
    // 录制阶段出错也要先把已录分段的上传/投稿收尾，再向上报告错误；
    // 否则一次 preupload 失败就会连带丢掉前面正在上传的分段。
    let recorded = record_segments(
        downloader,
        token,
        ctx,
        download_config,
        upload_config,
        upload_ctx,
        recorder,
        prefix,
        save_dir,
        total_size,
        session,
        pending,
        ready,
    )
    .await;
    // 出错路径上 ffmpeg 可能还在跑（如 preupload 失败），收尾前先停掉
    let _ = downloader.stop().await;
    let drained = drain_pending(
        pending,
        ready,
        session,
        upload_ctx,
        upload_config,
        ctx.config().submit_api.as_deref(),
        recorder,
    )
    .await;
    recorded?;
    drained?;
    Ok(DownloadStatus::StreamEnded)
}

/// 循环录制分段直到直播结束、取消或出错。返回 `Ok` 表示正常停止（含取消），
/// 队列里剩余的分段交给 [`drain_pending`] 收尾。
#[allow(clippy::too_many_arguments)]
async fn record_segments(
    downloader: &SyncDownloader,
    token: &CancellationToken,
    ctx: &Context,
    download_config: &DownloadConfig,
    upload_config: &UploadStreamer,
    upload_ctx: &UploadContext,
    recorder: &Recorder,
    prefix: &str,
    save_dir: Option<&PathBuf>,
    total_size: u64,
    session: &Arc<Mutex<SyncSession>>,
    pending: &mut VecDeque<PendingSegment>,
    ready: &mut BTreeMap<u64, ReadySegment>,
) -> AppResult<()> {
    let mut empty_retries = 0u32;
    let mut recorded_segments = 0u32;
    let mut next_seq = session.lock().await.videos.len() as u64 + 1;

    loop {
        // 取消只表示停止拉流：已录分段由 drain_pending 上传并投稿，而不是丢弃
        if token.is_cancelled() {
            return Ok(());
        }

        collect_finished(pending, ready, upload_ctx).await?;
        commit_ready(
            ready,
            session,
            &upload_ctx.bilibili,
            upload_config,
            ctx.config().submit_api.as_deref(),
            recorder,
        )
        .await?;

        while pending.len() + ready.len() >= MAX_UNCOMMITTED_SEGMENTS {
            let work = pending
                .pop_front()
                .ok_or_else(|| AppError::Custom("边录边传待提交队列状态错误".into()))?;
            let segment = finish_pending(work, upload_ctx).await?;
            ready.insert(segment.seq, segment);
            commit_ready(
                ready,
                session,
                &upload_ctx.bilibili,
                upload_config,
                ctx.config().submit_api.as_deref(),
                recorder,
            )
            .await?;
        }

        let file_name = format!("{prefix}_{next_seq}.mkv");
        let keep = save_dir.is_some();
        let path = save_dir.map_or_else(
            || {
                std::env::temp_dir()
                    .join("biliup-sync")
                    .join(format!("worker-{}", ctx.worker_id()))
                    .join(&file_name)
            },
            |dir| dir.join(&file_name),
        );
        info!(seq = next_seq, file_name, "准备边录边传分段");

        // 先确认 ffmpeg 真的产出了数据，再向 UPOS 申请上传：直播结束或直链失效时
        // 不会每次重试都留下一个只 preupload、从未写入的孤儿上传会话。
        let Some(segment) = downloader
            .start_segment(download_config, total_size, token)
            .await?
        else {
            if token.is_cancelled() {
                return Ok(());
            }
            // 已经录出过分段，说明直链此前有效，此刻拉不到数据多半是直播结束或直链过期
            // （B 站直链约 1 小时有效）。直接交回外层用 check_stream 刷新直链，
            // 不再拿同一个失效地址重试。
            if recorded_segments > 0 {
                warn!("ffmpeg 没有输出数据，结束本轮边录边传，等待重新获取直链");
                return Ok(());
            }
            empty_retries += 1;
            warn!(empty_retries, "ffmpeg 没有输出数据，重试边录边传拉流");
            if empty_retries >= MAX_EMPTY_RETRIES {
                return Ok(());
            }
            tokio::select! {
                _ = token.cancelled() => return Ok(()),
                _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {}
            }
            continue;
        };
        empty_retries = 0;

        let parcel = upload_ctx
            .line
            .pre_upload_stream(&upload_ctx.bilibili, &file_name, total_size)
            .await
            .change_context(AppError::Unknown)?;
        let chunk_size = parcel.chunk_size();
        if chunk_size == 0 {
            return Err(AppError::Custom("preupload 未返回 chunk_size".into()).into());
        }

        let (tx, rx) = async_channel::bounded::<Bytes>(STREAM_QUEUE_CAPACITY);
        let segment_ctx = upload_ctx.clone();
        let upload = tokio::spawn(async move {
            let stream = rx.map(|chunk: Bytes| {
                let len = chunk.len();
                Ok((chunk, len))
            });
            upload_byte_stream_parts(&segment_ctx, parcel, stream).await
        });
        let workbench = session.lock().await.recorder.clone();
        if let Some(workbench) = &workbench {
            workbench.opened(&path);
        }
        let pump = pump_chunks(
            segment.stdout,
            segment.peeked,
            chunk_size,
            total_size,
            path.clone(),
            tx,
            token.clone(),
        )
        .await;
        let _ = downloader.stop().await;
        let pump = match pump {
            Ok(pump) => pump,
            Err(error) => {
                upload.abort();
                let _ = upload.await;
                retain_or_remove(&SegmentFile { path, keep }).await;
                return Err(error);
            }
        };
        info!(
            seq = next_seq,
            actual_size = pump.actual_size,
            streamed_size = pump.streamed_size,
            stream_complete = pump.stream_complete,
            "本段录制结束"
        );
        if let Some(workbench) = &workbench {
            workbench.closed(
                &path,
                ClosedSegment {
                    bytes: Some(pump.actual_size),
                    discard: pump.actual_size < MIN_MEDIA_BYTES && !keep,
                    ..Default::default()
                },
            );
        }

        if pump.actual_size < MIN_MEDIA_BYTES {
            upload.abort();
            let _ = upload.await;
            if !keep {
                let retention = Retention::without_delay(ctx.pool().clone());
                let _ = retention::remove(&retention, &[&path]).await;
            }
            return Ok(());
        }

        pending.push_back(PendingSegment {
            seq: next_seq,
            file: SegmentFile { path, keep },
            pump,
            upload,
        });
        next_seq += 1;
        recorded_segments += 1;
    }
}

/// 按录制顺序等待所有待处理分段上传完成并投稿。
async fn drain_pending(
    pending: &mut VecDeque<PendingSegment>,
    ready: &mut BTreeMap<u64, ReadySegment>,
    session: &Arc<Mutex<SyncSession>>,
    upload_ctx: &UploadContext,
    upload_config: &UploadStreamer,
    submit_api: Option<&str>,
    recorder: &Recorder,
) -> AppResult<()> {
    while let Some(work) = pending.pop_front() {
        let segment = finish_pending(work, upload_ctx).await?;
        ready.insert(segment.seq, segment);
        commit_ready(
            ready,
            session,
            &upload_ctx.bilibili,
            upload_config,
            submit_api,
            recorder,
        )
        .await?;
    }
    Ok(())
}

async fn collect_finished(
    pending: &mut VecDeque<PendingSegment>,
    ready: &mut BTreeMap<u64, ReadySegment>,
    upload_ctx: &UploadContext,
) -> AppResult<()> {
    let mut index = 0;
    while index < pending.len() {
        if pending[index].upload.is_finished() {
            let work = pending.remove(index).expect("pending index must exist");
            let segment = finish_pending(work, upload_ctx).await?;
            ready.insert(segment.seq, segment);
        } else {
            index += 1;
        }
    }
    Ok(())
}

/// 等待分段的预传结束并拿到可投稿的分 P。
///
/// 这里刻意不响应取消：取消只停止拉流，已录内容必须传完并投稿，否则停止/暂停
/// 一次就会丢掉整段视频。
async fn finish_pending(
    work: PendingSegment,
    upload_ctx: &UploadContext,
) -> AppResult<ReadySegment> {
    let uploaded = work.upload.await;

    let video = if work.pump.stream_complete {
        match uploaded {
            Ok(Ok(uploaded))
                if uploaded.uploaded_size() == uploaded.declared_size()
                    && uploaded.uploaded_size() == work.pump.actual_size =>
            {
                match complete_byte_stream(uploaded).await {
                    Ok(video) => Some(video),
                    Err(error) => {
                        warn!(seq = work.seq, ?error, "UPOS complete 失败，按临时文件重传");
                        None
                    }
                }
            }
            Ok(Ok(uploaded)) => {
                warn!(
                    seq = work.seq,
                    uploaded_size = uploaded.uploaded_size(),
                    actual_size = work.pump.actual_size,
                    "UPOS 预传长度不一致，按临时文件重传"
                );
                None
            }
            Ok(Err(error)) => {
                warn!(seq = work.seq, ?error, "UPOS 预传失败，按临时文件重传");
                None
            }
            Err(error) => {
                warn!(seq = work.seq, ?error, "UPOS 预传任务失败，按临时文件重传");
                None
            }
        }
    } else {
        if let Ok(Err(error)) = uploaded {
            warn!(seq = work.seq, ?error, "短流预传未完成，按实际长度重传");
        }
        None
    };

    let video = match video {
        Some(video) => video,
        None => match upload_single_file(&work.file.path, upload_ctx).await {
            Ok(video) => video,
            Err(error) => {
                retain_or_remove(&work.file).await;
                return Err(error);
            }
        },
    };
    Ok(ReadySegment {
        seq: work.seq,
        file: work.file,
        video,
    })
}

/// 只允许提交紧接在已入列分 P 之后的分段，晚完成的前序分段会把后序分段留在 `ready` 中。
fn next_committable(
    ready: &mut BTreeMap<u64, ReadySegment>,
    committed_videos: usize,
) -> Option<ReadySegment> {
    ready.remove(&(committed_videos as u64 + 1))
}

async fn commit_ready(
    ready: &mut BTreeMap<u64, ReadySegment>,
    session: &Arc<Mutex<SyncSession>>,
    bilibili: &BiliBili,
    upload_config: &UploadStreamer,
    submit_api: Option<&str>,
    recorder: &Recorder,
) -> AppResult<()> {
    loop {
        let committed = session.lock().await.videos.len();
        let Some(segment) = next_committable(ready, committed) else {
            break;
        };
        {
            let mut state = session.lock().await;
            state.videos.push(segment.video);
            state.files.push(segment.file);
        }
        commit_unconfirmed(session, bilibili, upload_config, submit_api, recorder).await?;
    }
    Ok(())
}

async fn commit_unconfirmed(
    session: &Arc<Mutex<SyncSession>>,
    bilibili: &BiliBili,
    upload_config: &UploadStreamer,
    submit_api: Option<&str>,
    recorder: &Recorder,
) -> AppResult<()> {
    let (temporary_files, committed, retention) = {
        let mut state = session.lock().await;
        if state.confirmed_parts == state.videos.len() {
            return Ok(());
        }
        if state.first_submit_uncertain {
            return Err(AppError::Custom(
                "首 P 投稿结果不确定，已停止自动重试以避免生成重复稿件".into(),
            )
            .into());
        }

        let videos = state.videos.clone();
        let videos_len = state.videos.len();
        if let Some(studio) = state.studio.as_mut() {
            studio.videos = videos;
            edit_to_bilibili(bilibili, studio, submit_api).await?;
            let aid = studio.aid;
            state.confirmed_parts = videos_len;
            info!(parts = videos_len, aid, "边录边传追加分P成功");
        } else {
            let mut studio = build_studio(upload_config, bilibili, videos, recorder).await?;
            let ret = match submit_to_bilibili(bilibili, &studio, submit_api).await {
                Ok(ret) => ret,
                Err(error) => {
                    state.first_submit_uncertain = true;
                    return Err(error);
                }
            };
            // 投稿已成功但解析不出 aid 时同样标记不确定：
            // 此时服务端大概率已建稿，再自动重投会产生重复稿件。
            let aid = match aid_from_submit(&ret) {
                Ok(aid) => aid,
                Err(error) => {
                    state.first_submit_uncertain = true;
                    return Err(error);
                }
            };
            studio.aid = Some(aid);
            state.confirmed_parts = videos_len;
            state.studio = Some(studio);
            info!(aid, "边录边传首P投稿成功");
        }

        let mut temporary_files = Vec::new();
        while state.cleaned_parts < state.confirmed_parts {
            let file = state.files[state.cleaned_parts].clone();
            if file.keep {
                state.postprocess_paths.push(file.path);
            } else {
                temporary_files.push(file.path);
            }
            state.cleaned_parts += 1;
        }
        (
            temporary_files,
            state.confirmed_parts,
            state.retention.clone(),
        )
    };

    for path in temporary_files {
        let removed = match &retention {
            Some(retention) => retention::remove(retention, &[&path]).await.map(|_| ()),
            None => tokio::fs::remove_file(&path).await,
        };
        if let Err(error) = removed {
            warn!(?path, ?error, "删除边录边传临时文件失败");
        }
    }
    info!(committed, "边录边传分P状态已确认");
    Ok(())
}

/// 出错后放弃尚未投稿的分段：中止上传，但保留已录出内容的文件供手动补传。
///
/// 只有空文件才删除。录好的视频因为上传/投稿失败（如 cookie 失效）就被删掉，
/// 是边录边传最不可接受的结果；普通下载器在同样情形下也会把文件留在磁盘上。
async fn abandon_pending(
    pending: &mut VecDeque<PendingSegment>,
    ready: &mut BTreeMap<u64, ReadySegment>,
) {
    while let Some(work) = pending.pop_front() {
        work.upload.abort();
        let _ = work.upload.await;
        retain_or_remove(&work.file).await;
    }
    for (_, segment) in std::mem::take(ready) {
        retain_or_remove(&segment.file).await;
    }
}

async fn retain_or_remove(file: &SegmentFile) {
    let size = tokio::fs::metadata(&file.path)
        .await
        .map(|meta| meta.len())
        .unwrap_or(0);
    if file.keep || size >= MIN_MEDIA_BYTES {
        warn!(
            path = %file.path.display(),
            size,
            "边录边传分段未能投稿，已保留本地文件，可手动补传"
        );
    } else {
        let _ = tokio::fs::remove_file(&file.path).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(seq: u64) -> ReadySegment {
        ReadySegment {
            seq,
            file: SegmentFile {
                path: PathBuf::from(format!("seg-{seq}.mkv")),
                keep: true,
            },
            video: Video::new(&format!("seg-{seq}")),
        }
    }

    fn dummy_bili() -> BiliBili {
        BiliBili {
            client: reqwest::Client::new(),
            login_info: serde_json::from_value(serde_json::json!({
                "cookie_info": {},
                "sso": [],
                "token_info": {
                    "access_token": "",
                    "expires_in": 0,
                    "mid": 0,
                    "refresh_token": ""
                },
                "platform": null
            }))
            .expect("构造测试 LoginInfo"),
        }
    }

    fn dummy_upload_config() -> UploadStreamer {
        serde_json::from_value(serde_json::json!({
            "id": 0,
            "template_name": "test",
            "tags": []
        }))
        .expect("构造测试 UploadStreamer")
    }

    #[test]
    fn later_segment_finishing_first_waits_for_recording_order() {
        let mut ready = BTreeMap::new();
        ready.insert(2, segment(2));
        assert!(
            next_committable(&mut ready, 0).is_none(),
            "P2 先上传完也不能先投稿"
        );

        ready.insert(1, segment(1));
        assert_eq!(next_committable(&mut ready, 0).unwrap().seq, 1);
        assert_eq!(next_committable(&mut ready, 1).unwrap().seq, 2);
        assert!(next_committable(&mut ready, 2).is_none());
    }

    #[tokio::test]
    async fn abandoning_segments_keeps_recorded_files_and_drops_empty_ones() {
        let temp = tempfile::tempdir().unwrap();
        let recorded = temp.path().join("recorded.mkv");
        std::fs::write(&recorded, vec![0u8; MIN_MEDIA_BYTES as usize]).unwrap();
        let empty = temp.path().join("empty.mkv");
        std::fs::write(&empty, b"").unwrap();

        let mut ready = BTreeMap::new();
        for (seq, path) in [(1, recorded.clone()), (2, empty.clone())] {
            ready.insert(
                seq,
                ReadySegment {
                    seq,
                    file: SegmentFile { path, keep: false },
                    video: Video::new("seg"),
                },
            );
        }
        abandon_pending(&mut VecDeque::new(), &mut ready).await;

        assert!(ready.is_empty());
        assert!(recorded.exists(), "上传/投稿失败不能把录好的分段一起删掉");
        assert!(!empty.exists(), "空文件没有保留价值");
    }

    #[tokio::test]
    async fn confirmed_session_skips_resubmission_across_reconnects() {
        // 重连后 session 里已确认的分 P 不应触发任何网络提交（否则早就 panic 在请求上）
        let mut confirmed = SyncSession::default();
        confirmed.videos = vec![Video::new("p1")];
        confirmed.confirmed_parts = 1;
        confirmed.cleaned_parts = 1;
        let session = Arc::new(Mutex::new(confirmed));
        commit_unconfirmed(
            &session,
            &dummy_bili(),
            &dummy_upload_config(),
            None,
            &Recorder::default(),
        )
        .await
        .expect("已确认状态必须无副作用地返回");
    }

    #[tokio::test]
    async fn uncertain_first_submit_blocks_further_retries() {
        let mut uncertain = SyncSession::default();
        uncertain.videos = vec![Video::new("p1")];
        uncertain.first_submit_uncertain = true;
        let session = Arc::new(Mutex::new(uncertain));
        let error = commit_unconfirmed(
            &session,
            &dummy_bili(),
            &dummy_upload_config(),
            None,
            &Recorder::default(),
        )
        .await
        .expect_err("首投结果不确定时禁止自动重试");
        assert!(error.to_string().contains("重复稿件"));
    }
}
