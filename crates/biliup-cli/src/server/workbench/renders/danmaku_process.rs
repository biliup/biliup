//! The conversion supervisor outlives its requesting future. Its private
//! files are reclaimed only after the child has definitely exited.
use super::{ManagedRoot, REQUEST_FILE, WorkerOutput, describe};
use crate::danmaku_worker::{self, Request, Response};
#[cfg(test)]
use std::io;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::process::Child;
use tokio_util::sync::CancellationToken;

const MAX_DIAGNOSTICS: usize = 16 * 1024;

struct CancelOnDrop(CancellationToken);
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.cancel();
    }
}

pub(super) async fn run(
    root: &Arc<ManagedRoot>,
    xml: Vec<u8>,
    request: &Request,
    cancel: &CancellationToken,
    timeout: Duration,
) -> Result<WorkerOutput, String> {
    let owned_cancel = cancel.child_token();
    let _guard = CancelOnDrop(owned_cancel.clone());
    let root = root.clone();
    let request = request.clone();
    tokio::spawn(async move { supervise(root, xml, request, owned_cancel, timeout).await })
        .await
        .map_err(|e| format!("弹幕转换监督任务失败：{e}"))?
}

async fn supervise(
    root: Arc<ManagedRoot>,
    xml: Vec<u8>,
    request: Request,
    cancel: CancellationToken,
    timeout: Duration,
) -> Result<WorkerOutput, String> {
    let lease = match danmaku_worker::acquire_root_lease(root.path()) {
        Ok(lease) => lease,
        Err(error) => return finish_root(root, Err(format!("创建弹幕转换目录租约失败：{error}"))),
    };
    let attempt = match tempfile::Builder::new()
        .prefix(".danmaku-attempt-")
        .tempdir_in(root.path())
    {
        Ok(attempt) => attempt,
        Err(error) => {
            drop(lease);
            return finish_root(root, Err(format!("创建弹幕转换独占目录失败：{error}")));
        }
    };
    let result = execute(root.path(), attempt.path(), xml, &request, &cancel, timeout).await;
    let residual = attempt.path().to_owned();
    #[cfg(test)]
    let cleanup = if matches!(
        request.test_hook,
        Some(crate::danmaku_worker::tests::Hook::CleanupFailure)
    ) {
        let _ = attempt.keep();
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "injected attempt cleanup failure",
        ))
    } else {
        attempt.close()
    };
    #[cfg(not(test))]
    let cleanup = attempt.close();
    let result = match cleanup {
        Ok(()) => result,
        Err(error) => {
            tracing::warn!(%error, path = %residual.display(), "清理弹幕转换独占目录失败，恢复时重试");
            root.record_cleanup_failure(&format!("cleanup {}: {error}", residual.display()));
            append_error(result, format!("cleanup {}: {error}", residual.display()))
        }
    };
    drop(lease);
    let result = finish_root(root, result);
    if cancel.is_cancelled() {
        return match result {
            Err(error) if error.contains("cleanup ") => Err(format!("合成已取消；{error}")),
            _ => Err("合成已取消".into()),
        };
    }
    result
}

fn append_error(
    result: Result<WorkerOutput, String>,
    error: String,
) -> Result<WorkerOutput, String> {
    Err(match result {
        Ok(_) => error,
        Err(primary) => format!("{primary}; {error}"),
    })
}

fn finish_root(
    root: Arc<ManagedRoot>,
    result: Result<WorkerOutput, String>,
) -> Result<WorkerOutput, String> {
    ManagedRoot::finish(root, result)
}

async fn execute(
    root: &Path,
    work: &Path,
    xml: Vec<u8>,
    request: &Request,
    cancel: &CancellationToken,
    timeout: Duration,
) -> Result<WorkerOutput, String> {
    if cancel.is_cancelled() {
        return Err("合成已取消".into());
    }
    danmaku_worker::mark_attempt(root, work)
        .map_err(|e| format!("创建弹幕转换目录标记失败：{e}"))?;
    // Move the normalized allocation into the blocking writer. tokio::fs::write
    // would copy the complete AsRef byte slice before its blocking operation.
    let input = work.join(&request.input);
    tokio::task::spawn_blocking(move || std::fs::write(input, xml))
        .await
        .map_err(|e| format!("创建弹幕转换文件的写入任务失败：{e}"))?
        .map_err(|e| format!("创建弹幕转换文件失败：{e}"))?;
    let json = serde_json::to_vec(request).map_err(|e| format!("创建弹幕转换请求失败：{e}"))?;
    tokio::fs::write(work.join(REQUEST_FILE), json)
        .await
        .map_err(|e| format!("创建弹幕转换请求失败：{e}"))?;
    if cancel.is_cancelled() {
        return Err("合成已取消".into());
    }
    let mut command = danmaku_worker::command(work, Path::new(REQUEST_FILE))?;
    command
        .env(danmaku_worker::MANAGED_WORKER_ENV, "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = command
        .spawn()
        .map_err(|e| format!("弹幕转换失败：启动处理程序失败：{e}"))?;
    let stderr = child.stderr.take().expect("piped worker stderr");
    let mut diagnostics = tokio::spawn(async move {
        let mut stderr = stderr;
        let mut tail = Vec::new();
        let mut buffer = [0u8; 4096];
        while let Ok(n) = stderr.read(&mut buffer).await {
            if n == 0 {
                break;
            }
            tail.extend_from_slice(&buffer[..n]);
            if tail.len() > MAX_DIAGNOSTICS {
                tail.drain(..tail.len() - MAX_DIAGNOSTICS);
            }
        }
        String::from_utf8_lossy(&tail).into_owned()
    });
    let outcome = tokio::select! {
        biased;
        _ = cancel.cancelled() => Err("合成已取消".to_owned()),
        _ = tokio::time::sleep(timeout) => Err(format!("弹幕转换失败：处理程序连续 {} 秒没有推进，任务已停止；可重试", timeout.as_secs())),
        status = child.wait() => status.map_err(|e| format!("弹幕转换失败：等待处理程序失败：{e}")),
    };
    if outcome.is_err() {
        // A failed wait is not proof of exit. Keep ownership and retry reaping;
        // no cleanup path is reached while process state remains uncertain.
        reap(&mut child).await;
    }
    let stderr = match tokio::time::timeout(Duration::from_secs(1), &mut diagnostics).await {
        Ok(Ok(text)) => text,
        _ => {
            diagnostics.abort();
            String::new()
        }
    };
    if cancel.is_cancelled() {
        return Err("合成已取消".into());
    }
    let response = danmaku_worker::read_response(work, &request.result);
    let status = match outcome {
        Ok(status) => status,
        Err(error) => return Err(format!("{error}：{}", stderr.trim())),
    };
    let response = match response {
        Some(Ok(response)) => response,
        Some(Err(error)) => return Err(error),
        None if !status.success() => {
            return Err(format!(
                "弹幕转换失败：处理程序失败（{status}）：{}",
                stderr.trim()
            ));
        }
        None => return Err("弹幕转换进程没有返回结果".into()),
    };
    match response {
        Response::Failed { error, .. } => Err(describe(&error)),
        Response::Converted { stats, .. } if status.success() => {
            let ass = tokio::fs::read(work.join(&request.output))
                .await
                .map_err(|e| format!("弹幕转换没有生成可读取的 ASS：{e}"))?;
            #[cfg(test)]
            if let Some(crate::danmaku_worker::tests::Hook::PauseBeforeAccept { ready }) =
                &request.test_hook
            {
                std::fs::write(
                    ready,
                    serde_json::to_vec(&serde_json::json!({"pid": 0, "work": work})).unwrap(),
                )
                .unwrap();
                cancel.cancelled().await;
            }
            if cancel.is_cancelled() {
                return Err("合成已取消".into());
            }
            Ok(WorkerOutput { stats, ass })
        }
        Response::Converted { .. } => Err(format!(
            "弹幕转换失败：处理程序失败（{status}）：{}",
            stderr.trim()
        )),
    }
}

async fn reap(child: &mut Child) {
    if let Err(error) = child.start_kill() {
        tracing::warn!(%error, "终止弹幕转换进程失败，继续等待进程退出");
    }
    loop {
        match child.wait().await {
            Ok(_) => break,
            Err(error) => {
                tracing::error!(%error, "回收弹幕转换进程失败，保留目录并重试");
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
    }
}
