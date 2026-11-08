//! Portable equivalent of Uncut's shaderPass: decode -> shared WebGPU runner -> lossless FFV1.
//! Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
//! One frame in flight; pipes provide backpressure instead of retaining the clip in memory.
use super::*;
use std::process::Stdio;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Child,
    task::JoinHandle,
};

#[derive(Deserialize)]
pub(super) struct Shader {
    pub index: usize,
    pub width: u32,
    pub height: u32,
    pub frames: usize,
    pub name: String,
    pub decode: Vec<String>,
}

struct Process {
    child: Child,
    stderr: JoinHandle<String>,
}
impl Process {
    fn start(args: &[String], directory: &Path, encode: bool) -> Result<Self, String> {
        let mut command = crate::tools::ffmpeg_command();
        command
            .args(args)
            .current_dir(directory)
            .kill_on_drop(true)
            .stdin(if encode {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(if encode {
                Stdio::null()
            } else {
                Stdio::piped()
            })
            .stderr(Stdio::piped());
        let mut child = command.spawn().map_err(|e| e.to_string())?;
        let mut stderr = child.stderr.take().ok_or("读取特效渲染日志失败")?;
        let stderr = tokio::spawn(async move {
            let mut tail = Vec::new();
            let mut chunk = [0u8; 4096];
            while let Ok(n) = stderr.read(&mut chunk).await {
                if n == 0 {
                    break;
                }
                tail.extend_from_slice(&chunk[..n]);
                if tail.len() > 16384 {
                    tail.drain(..tail.len() - 16384);
                }
            }
            String::from_utf8_lossy(&tail).into_owned()
        });
        Ok(Self { child, stderr })
    }
    async fn stop(&mut self) {
        let _ = self.child.kill().await;
        let _ = self.child.wait().await;
    }
}

pub(super) async fn render(
    browser: &mut browser::Browser,
    layers: &[Shader],
    fps: f64,
    directory: &Path,
    cancel: &CancellationToken,
    mut frame_done: impl FnMut(),
) -> Result<BTreeMap<usize, String>, String> {
    let mut shaded = BTreeMap::new();
    if layers.is_empty() {
        return Ok(shaded);
    }
    browser.eval("BiliupExport.checkShaders()").await?;
    for layer in layers {
        if layer.width == 0
            || layer.height == 0
            || layer.width > 8192
            || layer.height > 8192
            || layer.frames == 0
        {
            return Err("着色器画面尺寸或帧数无效".into());
        }
        let file = format!("shader-{}.mkv", layer.index);
        let args: Vec<String> = [
            "-hide_banner",
            "-nostdin",
            "-v",
            "error",
            "-y",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgba",
            "-s",
            &format!("{}x{}", layer.width, layer.height),
            "-framerate",
            &fps.to_string(),
            "-i",
            "pipe:0",
            "-c:v",
            "ffv1",
            "-pix_fmt",
            "bgra",
            &file,
        ]
        .into_iter()
        .map(String::from)
        .collect();
        let mut encode = Process::start(&args, directory, true)?;
        let mut decode = Process::start(&layer.decode, directory, false)?;
        let mut input = decode.child.stdout.take().ok_or("无法读取素材帧")?;
        let mut output = encode.child.stdin.take().ok_or("无法写入特效帧")?;
        let size = layer.width as usize * layer.height as usize * 4;
        let mut pixels = vec![0u8; size];
        let mut ended = false;
        let mut last = None;
        let work = async {
            for frame in 0..layer.frames {
                let one = async {
                    if !ended {
                        let mut filled = 0;
                        while filled < size {
                            let n = input.read(&mut pixels[filled..]).await.map_err(|e| e.to_string())?;
                            if n == 0 {
                                if filled != 0 { return Err("素材帧不完整".into()); }
                                ended = true;
                                if !decode.child.wait().await.map_err(|e| e.to_string())?.success() {
                                    return Err("素材解码失败".into());
                                }
                                break;
                            }
                            filled += n;
                        }
                        if filled == size { last = Some(STANDARD.encode(&pixels)); }
                    }
                    let picture = last.as_ref().ok_or("素材未产生画面")?;
                    let result = browser.eval(&format!("BiliupExport.shade({},{frame},'{}')", layer.index, picture)).await?;
                    let drawn = STANDARD.decode(result.as_str().ok_or("着色器未返回画面")?).map_err(|e| e.to_string())?;
                    if drawn.len() != size { return Err("着色器输出帧大小无效".into()); }
                    output.write_all(&drawn).await.map_err(|e| e.to_string())?;
                    Ok::<_, String>(())
                };
                tokio::select! {
                    _ = cancel.cancelled() => return Err("导出已取消".into()),
                    result = tokio::time::timeout(Duration::from_secs(90), one) => result.map_err(|_| "特效帧渲染超时".to_string())??,
                }
                frame_done();
            }
            output.shutdown().await.map_err(|e| e.to_string())?;
            drop(output);
            decode.stop().await;
            let status = tokio::select! {
                _ = cancel.cancelled() => return Err("导出已取消".into()),
                result = tokio::time::timeout(Duration::from_secs(90), encode.child.wait()) => result.map_err(|_| "保存特效帧超时")?.map_err(|e| e.to_string())?,
            };
            if !status.success() { return Err("保存特效帧失败".into()); }
            Ok::<_, String>(())
        }.await;
        decode.stop().await;
        encode.stop().await;
        let logs = format!(
            "{}{}",
            decode.stderr.await.unwrap_or_default(),
            encode.stderr.await.unwrap_or_default()
        );
        work.map_err(|e| {
            format!(
                "{}：{e}{}",
                layer.name,
                if logs.trim().is_empty() {
                    String::new()
                } else {
                    format!("\n{}", logs.trim())
                }
            )
        })?;
        shaded.insert(layer.index, file);
    }
    Ok(shaded)
}
