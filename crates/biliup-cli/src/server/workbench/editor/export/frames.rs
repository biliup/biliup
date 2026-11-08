//! Direct still composition adapted from Uncut export/exporter/frames.ts (MIT).
//! The bundled planner yields bounded render/FFmpeg operations; no model-generated code runs here.
use super::*;
use crate::server::workbench::{editor, retention::MEDIA_LIFECYCLE};
use editor::ConnectionPool;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

struct Prepared {
    document: editor::Document,
    paths: BTreeMap<String, String>,
    graphics: BTreeMap<String, String>,
    assets: editor::fonts::RenderAssets,
    untagged: BTreeMap<String, u32>,
    _effects: tempfile::TempDir,
}
pub struct Picture {
    pub jpeg: Vec<u8>,
    pub evidence: Value,
}
pub async fn sheet(
    cells: &Value,
    fps: &Value,
    grid: &Value,
    cancel: &CancellationToken,
) -> Result<Vec<u8>, String> {
    let mut browser = browser::Browser::launch(cancel).await?;
    let result = async {
        let width = grid["width"]
            .as_u64()
            .filter(|n| *n <= 1600)
            .ok_or("联系表宽度无效")? as u32;
        let height = grid["height"]
            .as_u64()
            .filter(|n| *n <= 1600)
            .ok_or("联系表高度无效")? as u32;
        browser.viewport(width, height).await?;
        browser
            .eval(&format!(
                "BiliupExport.draw(BiliupExport.agentSheet({cells},{fps},{grid}),{})",
                fonts()
            ))
            .await?;
        browser.jpeg().await
    }
    .await;
    browser.close().await;
    result
}
pub struct Session {
    directory: tempfile::TempDir,
    prepared: BTreeMap<String, Prepared>,
    cancel: CancellationToken,
    deadline: tokio::task::JoinHandle<()>,
}
pub struct FrameRequest<'a> {
    pub document: editor::Document,
    pub frames: &'a [u64],
    pub width: u32,
    pub motion_blur: bool,
    pub census: bool,
}

impl Session {
    pub async fn rehearse(
        &mut self,
        pool: &ConnectionPool,
        root: &Path,
        project: i64,
        document: &editor::Document,
    ) -> Result<Value, String> {
        let graphics = document.project["tracks"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|track| track["hidden"] != true)
            .flat_map(|track| track["items"].as_array().into_iter().flatten())
            .any(|item| item["type"] == "graphic");
        if !graphics {
            return Ok(json!({"covered":0,"total":0,"reached":0}));
        }
        let cancel = &self.cancel;
        let _lifecycle = MEDIA_LIFECYCLE.read().await;
        let brand = editor::fonts::snapshot(pool, root, project)
            .await
            .map_err(|e| e.to_string())?;
        let key = format!(
            "{:x}",
            Sha256::digest(
                serde_json::to_vec(&json!({"document":document,"brand":brand}))
                    .map_err(|e| e.to_string())?
            )
        );
        let prepared = self
            .prepared
            .get(&key)
            .ok_or("工程或字体已改变，请重新取帧后预演")?;
        let _permit = crate::server::common::processing::transcode::acquire(cancel).await?;
        let mut browser = browser::Browser::launch(cancel).await?;
        let result = async {
            browser.media(&prepared.document.project, &prepared.paths);
            let plan = browser
                .eval(&format!(
                    "BiliupExport.rehearsalPlan({},{},{},{})",
                    prepared.document.project,
                    json!(prepared.paths),
                    json!(prepared.graphics),
                    serde_json::to_string(&prepared.assets).map_err(|e| e.to_string())?
                ))
                .await?;
            let width = plan["width"].as_u64().ok_or("预演宽度无效")? as u32;
            let height = plan["height"].as_u64().ok_or("预演高度无效")? as u32;
            if width == 0 || height == 0 || width as u64 * height as u64 > 16_777_216 {
                return Err("预演画布超限".into());
            }
            browser.viewport(width, height).await?;
            let mut evidence = json!({"covered":0,"total":plan["total"],"reached":0});
            let started = std::time::Instant::now();
            let budget = Duration::from_secs(30);
            let mut images = BTreeMap::new();
            for layer in plan["layers"].as_array().ok_or("图形预演计划无效")? {
                if started.elapsed() >= budget {
                    return Ok(evidence);
                }
                let doc = embed_images(
                    layer["doc"].as_str().ok_or("图形内容无效")?,
                    &prepared.document.project,
                    &prepared.paths,
                    &mut images,
                )
                .await?;
                browser
                    .eval(&format!(
                        "BiliupExport.loadGraphicDocument({},{width},{height})",
                        json!(doc)
                    ))
                    .await?;
                let calls = layer["calls"].as_array().ok_or("图形帧序列无效")?;
                let mut at = 0;
                while at < calls.len() {
                    if cancel.is_cancelled() {
                        return Err("预演已取消".into());
                    }
                    let remaining = budget.saturating_sub(started.elapsed());
                    if remaining.is_zero() {
                        return Ok(evidence);
                    }
                    let mut end = at + 1;
                    let mut count = calls[at]["ts"].as_array().ok_or("预演采样无效")?.len();
                    while end < calls.len()
                        && end - at < 60
                        && count + calls[end]["ts"].as_array().ok_or("预演采样无效")?.len() <= 120
                    {
                        count += calls[end]["ts"].as_array().unwrap().len();
                        end += 1;
                    }
                    let expression = format!(
                        "BiliupExport.rehearsalChunk({},{},{},{})",
                        json!(&calls[at..end]),
                        layer["info"],
                        layer["name"],
                        remaining.as_millis()
                    );
                    let next =
                        match tokio::time::timeout(remaining, browser.eval(&expression)).await {
                            Ok(value) => value?,
                            Err(_) => {
                                evidence["stalled"] = json!(remaining >= Duration::from_secs(30));
                                return Ok(evidence);
                            }
                        };
                    evidence["covered"] = json!(
                        evidence["covered"].as_u64().unwrap()
                            + next["covered"].as_u64().ok_or("预演计数无效")?
                    );
                    evidence["reached"] = next["reached"].clone();
                    if next["failure"].is_string() {
                        evidence["failure"] = next["failure"].clone();
                        evidence["frame"] = next["frame"].clone();
                        return Ok(evidence);
                    }
                    if next["partial"] == true {
                        return Ok(evidence);
                    }
                    at = end;
                }
            }
            Ok(evidence)
        }
        .await;
        browser.close().await;
        result
    }
    pub fn new(parent: &CancellationToken) -> Result<Self, String> {
        let directory = tempfile::Builder::new()
            .prefix("editor-agent-frames-")
            .tempdir()
            .map_err(|e| e.to_string())?;
        let cancel = parent.child_token();
        let timed = cancel.clone();
        let deadline = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(600)).await;
            timed.cancel();
        });
        Ok(Self {
            directory,
            prepared: BTreeMap::new(),
            cancel,
            deadline,
        })
    }
    pub async fn jpeg(
        &mut self,
        pool: &ConnectionPool,
        root: &Path,
        project: i64,
        document: editor::Document,
        frame: u64,
        short_side: u32,
    ) -> Result<Vec<u8>, String> {
        let width = document.project["canvas"]["width"]
            .as_f64()
            .ok_or("画布无效")?;
        let height = document.project["canvas"]["height"]
            .as_f64()
            .ok_or("画布无效")?;
        let out = (width * (short_side as f64 / width.min(height)).min(1.)).round() as u32;
        let jpeg = self
            .render(
                pool,
                root,
                project,
                FrameRequest {
                    document,
                    frames: &[frame],
                    width: out,
                    motion_blur: false,
                    census: false,
                },
            )
            .await?
            .into_iter()
            .next()
            .map(|p| p.jpeg)
            .ok_or("未生成分镜画面")?;
        if jpeg.len() > 2 * 1024 * 1024 {
            return Err("分镜画面超过 2 MiB".into());
        }
        Ok(jpeg)
    }
    pub async fn render(
        &mut self,
        pool: &ConnectionPool,
        root: &Path,
        project: i64,
        FrameRequest {
            document,
            frames,
            width,
            motion_blur,
            census,
        }: FrameRequest<'_>,
    ) -> Result<Vec<Picture>, String> {
        self.render_budget(
            pool,
            root,
            project,
            FrameRequest {
                document,
                frames,
                width,
                motion_blur,
                census,
            },
            None,
        )
        .await
    }
    pub async fn render_budget(
        &mut self,
        pool: &ConnectionPool,
        root: &Path,
        project: i64,
        FrameRequest {
            document,
            frames,
            width,
            motion_blur,
            census,
        }: FrameRequest<'_>,
        budget_ms: Option<u64>,
    ) -> Result<Vec<Picture>, String> {
        if frames.is_empty() || frames.len() > 24 || !(2..=8192).contains(&width) {
            return Err("取帧数量或尺寸无效".into());
        }
        let cancel = &self.cancel;
        if cancel.is_cancelled() {
            return Err("画面渲染已取消或超时".into());
        }
        let _lifecycle = MEDIA_LIFECYCLE.read().await;
        let brand = editor::fonts::snapshot(pool, root, project)
            .await
            .map_err(|e| e.to_string())?;
        let key = format!(
            "{:x}",
            Sha256::digest(
                serde_json::to_vec(&json!({"document":document,"brand":brand}))
                    .map_err(|e| e.to_string())?
            )
        );
        if !self.prepared.contains_key(&key) {
            if self.prepared.len() >= 8 {
                return Err("一次画面会话最多读取八个工程状态".into());
            }
            let value = prepare(pool, root, project, document, brand, cancel).await?;
            self.prepared.insert(key.clone(), value);
        }
        let prepared = self.prepared.get(&key).unwrap();
        let _permit = crate::server::common::processing::transcode::acquire(cancel).await?;
        let mut browser = browser::Browser::launch(cancel).await?;
        let result=async{
            browser.media(&prepared.document.project,&prepared.paths);
            browser.eval(&format!("globalThis.biliupFonts={};BiliupExport.frameAssets({});true",fonts(),serde_json::to_string(&prepared.assets).map_err(|e|e.to_string())?)).await?;
            let mut input=json!({"project":prepared.document.project,"paths":prepared.paths,"graphics":prepared.graphics,"assets":prepared.assets,"directory":self.directory.path().to_string_lossy(),"untagged":prepared.untagged,"frames":frames,"width":width,"motionBlur":motion_blur,"census":census});
            if let Some(ms)=budget_ms{input["budgetMs"]=json!(ms);}
            let mut state=browser.eval(&format!("BiliupExport.frameStart({input})")).await?;
            let mut images=BTreeMap::new();let mut bytes=0u64;
            for _ in 0..20000 {
                if cancel.is_cancelled(){return Err("画面渲染已取消或超时".into());}
                if let Some(error)=state["error"].as_str(){return Err(error.to_string());}
                if let Some(done)=state["done"].as_array(){
                    let mut pictures=vec![];
                    for frame in done {
                        let path=local(self.directory.path(),frame["jpeg"].as_str().ok_or("取帧结果无效")?)?;
                        let jpeg=read(&path,5*1024*1024).await?;
                        let mut evidence=frame.clone();evidence.as_object_mut().unwrap().remove("jpeg");
                        pictures.push(Picture{jpeg,evidence});
                    }
                    return Ok(pictures);
                }
                let step=&state["step"];
                let result=execute(&mut browser,prepared,self.directory.path(),step,cancel,&mut images,&mut bytes).await;
                // Every failing operation rejects the planner's pending promise and releases it.
                let reply=match result{Ok(value)=>json!({"value":value}),Err(error)=>json!({"error":error})};
                state=browser.eval(&format!("BiliupExport.frameNext({reply})")).await?;
            }
            Err("取帧操作超过上限".into())
        }.await;
        browser.close().await;
        result
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.cancel.cancel();
        self.deadline.abort();
    }
}
async fn prepare(
    pool: &ConnectionPool,
    root: &Path,
    project: i64,
    mut document: editor::Document,
    brand: editor::fonts::Snapshot,
    cancel: &CancellationToken,
) -> Result<Prepared, String> {
    let temp = tempfile::Builder::new()
        .prefix("editor-agent-effects-")
        .tempdir()
        .map_err(|e| e.to_string())?;
    let (mut browser, mut paths) =
        super::inputs::prepare(pool, &mut document, &Value::Null, "picture", cancel).await?;
    let effects = async {
        recording_effects::prepare_mask_images(
            &mut browser,
            &document.project,
            &Value::Null,
            &mut paths,
            temp.path(),
        )
        .await?;
        recording_effects::prepare(
            pool,
            &mut browser,
            recording_effects::Preparation {
                document: &mut document,
                range: &Value::Null,
                paths: &mut paths,
                directory: temp.path(),
            },
            cancel,
            |_, _| {},
        )
        .await
    }
    .await;
    browser.close().await;
    effects?;
    let mut untagged = BTreeMap::new();
    {
        let _permit = crate::server::common::processing::transcode::acquire(cancel).await?;
        let visible: std::collections::BTreeSet<&str> = document.project["tracks"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|track| track["hidden"] != true && track["kind"] != "audio")
            .flat_map(|track| track["items"].as_array().into_iter().flatten())
            .filter(|item| {
                item["type"] == "clip"
                    || (item["type"] == "danmaku" && item["danmaku"]["settings"]["enabled"] == true)
            })
            .filter_map(|item| item["media"].as_str())
            .collect();
        for media in document.project["media"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|m| {
                m["kind"] == "video" && m["id"].as_str().is_some_and(|id| visible.contains(id))
            })
        {
            if let Some(path) = media["id"].as_str().and_then(|id| paths.get(id))
                && let Some(height) =
                    editor::graphic_media::colour_hint(Path::new(path), cancel).await?
            {
                untagged.insert(path.clone(), height);
            }
        }
    }
    let graphics = editor::graphics::load_for_render(pool, project, &document.project, None)
        .await
        .map_err(|e| e.to_string())?;
    let assets = editor::fonts::materialize(root, &brand).await;
    Ok(Prepared {
        document,
        paths,
        graphics,
        assets,
        untagged,
        _effects: temp,
    })
}
fn local(directory: &Path, path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(path);
    if path.parent() != Some(directory) || path.file_name().is_none() {
        return Err("取帧临时文件路径无效".into());
    }
    Ok(path)
}
async fn read(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    if tokio::fs::metadata(path)
        .await
        .map_err(|e| e.to_string())?
        .len()
        > limit
    {
        return Err("取帧临时文件超出大小限制".into());
    }
    tokio::fs::read(path).await.map_err(|e| e.to_string())
}
async fn execute(
    browser: &mut browser::Browser,
    prepared: &Prepared,
    directory: &Path,
    step: &Value,
    cancel: &CancellationToken,
    images: &mut BTreeMap<String, String>,
    bytes: &mut u64,
) -> Result<Value, String> {
    match step["kind"].as_str() {
        Some("ffmpeg") => {
            let args: Vec<String> =
                serde_json::from_value(step["args"].clone()).map_err(|e| e.to_string())?;
            if args.len() > 10000 || args.iter().map(|s| s.len()).sum::<usize>() > 2 * 1024 * 1024 {
                return Err("取帧命令超出限制".into());
            }
            let output = local(directory, args.last().ok_or("取帧缺少输出")?)?;
            let mut command = crate::tools::ffmpeg_command();
            command
                .args(["-filter_complex_threads", "1", "-threads", "1"])
                .args(&args)
                .current_dir(directory);
            engine::run_process(command, cancel, Duration::from_secs(60), |_, _| {}).await?;
            *bytes += tokio::fs::metadata(output)
                .await
                .map_err(|e| e.to_string())?
                .len();
            if *bytes > 2 * 1024 * 1024 * 1024 {
                return Err("取帧临时数据超过 2 GiB".into());
            }
            Ok(json!({"code":0,"stderr":""}))
        }
        Some("write") => {
            let path = local(directory, step["path"].as_str().ok_or("查找表路径无效")?)?;
            let text = step["text"]
                .as_str()
                .filter(|s| s.len() <= 16 * 1024 * 1024)
                .ok_or("查找表过大")?;
            *bytes += text.len() as u64;
            if *bytes > 2 * 1024 * 1024 * 1024 {
                return Err("取帧临时数据超过 2 GiB".into());
            }
            tokio::fs::write(path, text)
                .await
                .map_err(|e| e.to_string())?;
            Ok(Value::Null)
        }
        Some("raster" | "graphic" | "shader") => {
            let width = step["width"].as_u64().ok_or("取帧宽度无效")? as u32;
            let height = step["height"].as_u64().ok_or("取帧高度无效")? as u32;
            if width == 0 || height == 0 || width as u64 * height as u64 > 16_777_216 {
                return Err("取帧画布超出范围".into());
            }
            let output = local(directory, step["out"].as_str().ok_or("取帧输出无效")?)?;
            if step["kind"] == "shader" {
                let input = local(directory, step["input"].as_str().ok_or("着色器输入无效")?)?;
                let rgba = read(&input, 16_777_216 * 4).await?;
                if rgba.len() != width as usize * height as usize * 4 {
                    return Err("着色器源帧大小不匹配".into());
                }
                let pixels = browser
                    .eval(&format!(
                        "BiliupExport.frameShade({width},{height},{},{},{},{})",
                        step["code"],
                        step["frame"],
                        step["params"],
                        json!(STANDARD.encode(rgba))
                    ))
                    .await?;
                let pixels = STANDARD
                    .decode(pixels.as_str().ok_or("着色器输出无效")?)
                    .map_err(|e| e.to_string())?;
                if pixels.len() != width as usize * height as usize * 4 {
                    return Err("着色器输出大小不匹配".into());
                }
                *bytes += pixels.len() as u64;
                if *bytes > 2 * 1024 * 1024 * 1024 {
                    return Err("取帧临时数据超过 2 GiB".into());
                }
                tokio::fs::write(output, pixels)
                    .await
                    .map_err(|e| e.to_string())?;
                return Ok(Value::Null);
            }
            browser.viewport(width, height).await?;
            let mut evidence = Value::Null;
            if step["kind"] == "graphic" {
                let doc = embed_images(
                    step["doc"].as_str().ok_or("图形文档无效")?,
                    &prepared.document.project,
                    &prepared.paths,
                    images,
                )
                .await?;
                browser
                    .eval(&format!(
                        "BiliupExport.loadGraphicDocument({},{width},{height})",
                        json!(doc)
                    ))
                    .await?;
                evidence = browser
                    .eval(&format!(
                        "BiliupExport.drawGraphic({},{},{})",
                        step["t"], step["info"], step["census"]
                    ))
                    .await?;
                let samples: Vec<f64> = step["ts"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_f64)
                    .collect();
                if samples.len() > 1 {
                    if samples.len() > 64 {
                        return Err("运动模糊采样过多".into());
                    }
                    browser
                        .eval(&format!("BiliupExport.resetGraphicBlur({width},{height})"))
                        .await?;
                    let mut times = samples;
                    let start = std::time::Instant::now();
                    loop {
                        browser.eval("BiliupExport.beginGraphicSamples();BiliupExport.showGraphicSource()").await?;
                        for time in &times {
                            browser
                                .eval(&format!(
                                    "BiliupExport.drawGraphic({time},{})",
                                    step["info"]
                                ))
                                .await?;
                            let png = browser.png().await?;
                            browser
                                .eval(&format!(
                                    "BiliupExport.addGraphicSample({})",
                                    json!(STANDARD.encode(png))
                                ))
                                .await?;
                        }
                        browser.eval("BiliupExport.finishGraphicSamples()").await?;
                        let next: Vec<f64> = serde_json::from_value(
                            browser
                                .eval(&format!(
                                    "BiliupExport.nextGraphicSamples({})",
                                    json!(times)
                                ))
                                .await?,
                        )
                        .map_err(|e| e.to_string())?;
                        if next.is_empty() || start.elapsed().as_secs_f64() > 8. || next.len() > 64
                        {
                            break;
                        }
                        times = next;
                    }
                    browser.eval("BiliupExport.showGraphicMean()").await?;
                }
            } else {
                let html = embed_images(
                    step["html"].as_str().ok_or("静态图层无效")?,
                    &prepared.document.project,
                    &prepared.paths,
                    images,
                )
                .await?;
                browser
                    .eval(&format!(
                        "BiliupExport.draw({},globalThis.biliupFonts)",
                        json!(html)
                    ))
                    .await?;
            }
            let png = browser.png().await?;
            *bytes += png.len() as u64;
            if *bytes > 2 * 1024 * 1024 * 1024 {
                return Err("取帧临时数据超过 2 GiB".into());
            }
            tokio::fs::write(output, png)
                .await
                .map_err(|e| e.to_string())?;
            Ok(evidence)
        }
        _ => Err("未知取帧步骤".into()),
    }
}
