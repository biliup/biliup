//! Portable whisper.cpp model management. Catalog: Uncut analysis/whisperModels.ts (MIT).
use super::lock;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::sync::CancellationToken;
#[derive(Clone, Copy)]
pub struct Model {
    pub id: &'static str,
    pub name: &'static str,
    pub bytes: u64,
    pub sha256: &'static str,
    pub multilingual: bool,
    pub offered: bool,
}
pub const DEFAULT: &str = "large-v3-turbo-q5_0";
pub const MODELS: &[Model] = &[
    Model {
        id: DEFAULT,
        name: "Large v3 Turbo",
        bytes: 574041195,
        sha256: "394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2",
        multilingual: true,
        offered: true,
    },
    Model {
        id: "small",
        name: "Small",
        bytes: 487601967,
        sha256: "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b",
        multilingual: true,
        offered: true,
    },
    Model {
        id: "base",
        name: "Base",
        bytes: 147951465,
        sha256: "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe",
        multilingual: true,
        offered: true,
    },
    Model {
        id: "base.en",
        name: "Base (English only)",
        bytes: 147964211,
        sha256: "a03779c86df3323075f5e796cb2ce5029f00ec8869eee3fdfb897afe36c6d002",
        multilingual: false,
        offered: false,
    },
    Model {
        id: "tiny.en",
        name: "Tiny (English only)",
        bytes: 77704715,
        sha256: "921e4cf8686fdd993dcd081a5da5b6c365bfde1162e72b08d75ac75289920b1f",
        multilingual: false,
        offered: false,
    },
];
pub fn model(id: &str) -> Result<&'static Model, String> {
    MODELS
        .iter()
        .find(|model| model.id == id)
        .ok_or_else(|| "未知的语音模型".into())
}
pub fn binary() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("BILIUP_WHISPER_BIN") {
        let path = PathBuf::from(path);
        return path.is_file().then_some(path);
    }
    let name = if cfg!(windows) {
        "whisper-cli.exe"
    } else {
        "whisper-cli"
    };
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|path| path.join(name))
            .find(|path| path.is_file())
    })
}
struct Download {
    cancel: CancellationToken,
    state: Value,
}
pub struct Models {
    root: PathBuf,
    downloads: Mutex<HashMap<String, Download>>,
    settings: tokio::sync::Mutex<()>,
    stop: CancellationToken,
}
impl Models {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.join("editor-models"),
            downloads: Mutex::new(HashMap::new()),
            settings: tokio::sync::Mutex::new(()),
            stop: CancellationToken::new(),
        }
    }
    pub fn path(&self, id: &str) -> Result<PathBuf, String> {
        Ok(self.root.join(format!("ggml-{}.bin", model(id)?.id)))
    }
    pub async fn selected(&self) -> String {
        tokio::fs::read(self.root.join("selected.json"))
            .await
            .ok()
            .and_then(|data| serde_json::from_slice::<String>(&data).ok())
            .filter(|id| model(id).is_ok())
            .unwrap_or_else(|| DEFAULT.into())
    }
    pub async fn select(&self, id: &str) -> Result<(), String> {
        model(id)?;
        let _guard = self.settings.lock().await;
        tokio::fs::create_dir_all(&self.root)
            .await
            .map_err(|e| e.to_string())?;
        let mut file = tempfile::NamedTempFile::new_in(&self.root).map_err(|e| e.to_string())?;
        std::io::Write::write_all(&mut file, &serde_json::to_vec(id).unwrap())
            .map_err(|e| e.to_string())?;
        file.persist(self.root.join("selected.json"))
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    async fn installed(&self, info: &Model) -> bool {
        tokio::fs::metadata(self.path(info.id).unwrap())
            .await
            .is_ok_and(|meta| meta.is_file() && meta.len() == info.bytes)
    }
    pub async fn state(&self) -> Value {
        let mut models = vec![];
        for info in MODELS {
            models.push(json!({"id":info.id,"name":info.name,"bytes":info.bytes,"multilingual":info.multilingual,"offered":info.offered,"installed":self.installed(info).await}));
        }
        let downloads: Vec<_> = lock(&self.downloads)
            .values()
            .map(|d| d.state.clone())
            .collect();
        json!({"selected":self.selected().await,"dir":"editor-models","models":models,"downloads":downloads})
    }
    pub async fn ready(&self, id: &str) -> Result<(PathBuf, PathBuf), String> {
        let bin = binary().ok_or(
            "未找到 whisper-cli，请在服务器安装 whisper.cpp 并加入 PATH，或设置 BILIUP_WHISPER_BIN",
        )?;
        if !self.installed(model(id)?).await {
            return Err("请先在设置中下载所选语音模型".into());
        }
        Ok((bin, self.path(id)?))
    }
    pub fn cancel(&self, id: &str) -> Result<(), String> {
        model(id)?;
        if let Some(download) = lock(&self.downloads).get(id) {
            download.cancel.cancel();
        }
        Ok(())
    }
    pub fn stop(&self) {
        self.stop.cancel();
    }
    /// A transcription joins the existing verified model download. Cancelling a waiting job
    /// leaves a shared download available to other jobs; Settings can cancel that download.
    pub async fn ensure(
        self: &Arc<Self>,
        id: &str,
        cancel: &CancellationToken,
        progress: impl Fn(f64) + Sync,
    ) -> Result<(), String> {
        if cancel.is_cancelled() {
            return Err("分析已取消".into());
        }
        if self.ready(id).await.is_ok() {
            return Ok(());
        }
        if binary().is_none() {
            return Err("未找到 whisper-cli，请先配置服务器上的 whisper.cpp".into());
        }
        self.download(id.to_string()).await?;
        loop {
            if cancel.is_cancelled() {
                return Err("分析已取消".into());
            }
            let state = lock(&self.downloads).get(id).map(|d| d.state.clone());
            match state.as_ref().and_then(|s| s["phase"].as_str()) {
                Some("done") | None => {
                    self.ready(id).await?;
                    return Ok(());
                }
                Some("failed") => {
                    return Err(state.as_ref().unwrap()["error"]
                        .as_str()
                        .unwrap_or("语音模型下载失败")
                        .to_string());
                }
                Some("cancelled") => return Err("语音模型下载已取消".into()),
                _ => {
                    let state = state.as_ref().unwrap();
                    progress(
                        (state["received"].as_f64().unwrap_or(0.)
                            / state["total"].as_f64().unwrap_or(1.).max(1.))
                        .clamp(0., 1.),
                    );
                }
            }
            tokio::select! {_=cancel.cancelled()=>return Err("分析已取消".into()),_=tokio::time::sleep(std::time::Duration::from_millis(400))=>{}}
        }
    }
    pub async fn download(self: &Arc<Self>, id: String) -> Result<(), String> {
        if self.stop.is_cancelled() {
            return Err("服务正在停止".into());
        }
        let info = *model(&id)?;
        if self.installed(&info).await {
            return Ok(());
        }
        let cancel = self.stop.child_token();
        {
            let mut downloads = lock(&self.downloads);
            if downloads
                .get(&id)
                .is_some_and(|d| d.state["phase"] == "downloading")
            {
                return Ok(());
            }
            downloads.insert(id.clone(),Download{cancel:cancel.clone(),state:json!({"model":id,"phase":"downloading","received":0,"total":info.bytes})});
        }
        let service = self.clone();
        tokio::spawn(async move {
            let result = service.receive(&info, &cancel).await;
            let mut downloads = lock(&service.downloads);
            if let Some(download) = downloads.get_mut(&id) {
                download.state["phase"] = json!(if cancel.is_cancelled() {
                    "cancelled"
                } else if result.is_ok() {
                    "done"
                } else {
                    "failed"
                });
                if let Err(error) = result {
                    download.state["error"] = json!(error);
                }
            }
        });
        Ok(())
    }
    async fn receive(&self, info: &Model, cancel: &CancellationToken) -> Result<(), String> {
        let dest = self.path(info.id)?;
        tokio::fs::create_dir_all(&self.root)
            .await
            .map_err(|e| e.to_string())?;
        let part = dest.with_extension("bin.part");
        let result = async {
            let mut have = tokio::fs::metadata(&part)
                .await
                .map(|m| m.len())
                .unwrap_or(0);
            if have >= info.bytes {
                tokio::fs::remove_file(&part)
                    .await
                    .map_err(|e| e.to_string())?;
                have = 0;
            }
            let client = reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(30))
                .read_timeout(Duration::from_secs(60))
                .build()
                .map_err(|e| e.to_string())?;
            let url = format!(
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{}.bin",
                info.id
            );
            let mut request = client.get(url);
            if have > 0 {
                request = request.header(reqwest::header::RANGE, format!("bytes={have}-"));
            }
            let mut response = request
                .send()
                .await
                .map_err(|e| format!("模型下载失败：{e}"))?
                .error_for_status()
                .map_err(|e| e.to_string())?;
            if response.status() == reqwest::StatusCode::PARTIAL_CONTENT {
                let start = response
                    .headers()
                    .get(reqwest::header::CONTENT_RANGE)
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.strip_prefix("bytes "))
                    .and_then(|value| value.split('-').next())
                    .and_then(|value| value.parse::<u64>().ok());
                if start != Some(have) {
                    let _ = tokio::fs::remove_file(&part).await;
                    return Err("模型续传位置不匹配，请重试".into());
                }
            } else {
                have = 0;
            }
            let mut hash = Sha256::new();
            if have > 0 {
                let mut existing = tokio::fs::File::open(&part)
                    .await
                    .map_err(|e| e.to_string())?;
                let mut bytes = vec![0; 65536];
                loop {
                    let size = existing.read(&mut bytes).await.map_err(|e| e.to_string())?;
                    if size == 0 {
                        break;
                    }
                    hash.update(&bytes[..size]);
                }
            }
            let mut file = tokio::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(have == 0)
                .append(have > 0)
                .open(&part)
                .await
                .map_err(|e| e.to_string())?;
            while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
                have += chunk.len() as u64;
                if have > info.bytes {
                    return Err("模型大小超过预期".into());
                }
                file.write_all(&chunk).await.map_err(|e| e.to_string())?;
                hash.update(&chunk);
                if let Some(download) = lock(&self.downloads).get_mut(info.id) {
                    download.state["received"] = json!(have);
                }
            }
            file.flush().await.map_err(|e| e.to_string())?;
            drop(file);
            if have != info.bytes || format!("{:x}", hash.finalize()) != info.sha256 {
                let _ = tokio::fs::remove_file(&part).await;
                return Err("模型校验失败，请重新下载".into());
            }
            tokio::fs::rename(&part, &dest)
                .await
                .map_err(|e| e.to_string())?;
            Ok(())
        };
        tokio::select! {biased;_=cancel.cancelled()=>{let _=tokio::fs::remove_file(&part).await;Err("模型下载已取消".into())},result=result=>result}
    }
}
