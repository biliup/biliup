//! An isolated headless Chromium process for the shared preview/export layout code.
//! No Electron, user browser profile, Node runtime, or external page/network is used.
use base64::{Engine, engine::general_purpose::STANDARD};
use futures::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::{path::PathBuf, process::Stdio, time::Duration};
use tokio::{
    net::TcpStream,
    process::{Child, Command},
};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite::Message};
use tokio_util::sync::CancellationToken;

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;
pub struct Browser {
    child: Child,
    profile: tempfile::TempDir,
    socket: Socket,
    seq: u64,
    cancel: CancellationToken,
    media: Option<super::super::graphic_media::ProjectFrames>,
    context: Option<u64>,
    main_frame: Option<String>,
}

pub fn executable() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("BILIUP_EDITOR_BROWSER") {
        let path = PathBuf::from(path);
        return path
            .is_file()
            .then_some(path)
            .ok_or("BILIUP_EDITOR_BROWSER 指定的浏览器不存在".into());
    }
    let mut paths = vec![];
    for root in ["PROGRAMFILES", "PROGRAMFILES(X86)", "LOCALAPPDATA"] {
        if let Some(root) = std::env::var_os(root) {
            for suffix in [
                "Google/Chrome/Application/chrome.exe",
                "Microsoft/Edge/Application/msedge.exe",
                "Chromium/Application/chrome.exe",
            ] {
                paths.push(PathBuf::from(&root).join(suffix));
            }
        }
    }
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            for file in [
                "chromium",
                "chromium-browser",
                "google-chrome",
                "google-chrome-stable",
                "chrome.exe",
                "msedge.exe",
            ] {
                paths.push(dir.join(file));
            }
        }
    }
    paths.into_iter().find(|path| path.is_file()).ok_or(
        "工程导出需要 Chromium、Chrome 或 Edge；可通过 BILIUP_EDITOR_BROWSER 指定可执行文件".into(),
    )
}

impl Browser {
    pub async fn launch(cancel: &CancellationToken) -> Result<Self, String> {
        let profile = tempfile::Builder::new()
            .prefix("biliup-editor-render-")
            .tempdir()
            .map_err(|e| e.to_string())?;
        let mut command = Command::new(executable()?);
        command
            .args([
                "--headless=new",
                "--enable-gpu",
                "--remote-debugging-port=0",
                "--remote-debugging-address=127.0.0.1",
                "--no-first-run",
                "--no-default-browser-check",
                "--disable-background-networking",
                "--disable-component-update",
                "--disable-sync",
                "--disable-extensions",
                "--disable-dev-shm-usage",
                "--no-proxy-server",
            ])
            .arg(format!("--user-data-dir={}", profile.path().display()))
            .arg("about:blank")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        #[cfg(windows)]
        command.creation_flags(0x08000000);
        let mut child = command
            .spawn()
            .map_err(|e| format!("启动工程渲染器失败：{e}"))?;
        let connect = async {
            let client = reqwest::Client::builder()
                .no_proxy()
                .timeout(Duration::from_secs(5))
                .build()
                .map_err(|e| e.to_string())?;
            loop {
                if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
                    return Err(format!("工程渲染器提前退出：{status}"));
                }
                if let Ok(text) =
                    tokio::fs::read_to_string(profile.path().join("DevToolsActivePort")).await
                    && let Some(port) = text.lines().next().and_then(|p| p.parse::<u16>().ok())
                {
                    let targets: Value = client
                        .get(format!("http://127.0.0.1:{port}/json/list"))
                        .send()
                        .await
                        .map_err(|e| e.to_string())?
                        .json()
                        .await
                        .map_err(|e| e.to_string())?;
                    if let Some(url) = targets
                        .as_array()
                        .and_then(|pages| pages.iter().find(|p| p["type"] == "page"))
                        .and_then(|page| page["webSocketDebuggerUrl"].as_str())
                    {
                        return tokio_tungstenite::connect_async(url)
                            .await
                            .map(|(socket, _)| socket)
                            .map_err(|e| e.to_string());
                    }
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        };
        let socket = tokio::select! {
            _ = cancel.cancelled() => return Err("导出已取消".into()),
            result = tokio::time::timeout(Duration::from_secs(30), connect) => result.map_err(|_| "启动渲染器超时")??,
        };
        let mut browser = Self {
            child,
            profile,
            socket,
            seq: 0,
            cancel: cancel.clone(),
            media: None,
            context: None,
            main_frame: None,
        };
        browser.call("Page.enable", json!({})).await?;
        browser.main_frame =
            browser.call("Page.getFrameTree", json!({})).await?["frameTree"]["frame"]["id"]
                .as_str()
                .map(str::to_string);
        browser.call("Runtime.enable", json!({})).await?;
        browser
            .call(
                "Runtime.addBinding",
                json!({"name":"__biliup_graphic_media"}),
            )
            .await?;
        // Block requests at the protocol level as well as the page CSP. Only embedded data is drawn.
        browser
            .call("Fetch.enable", json!({"patterns":[{"urlPattern":"*"}]}))
            .await?;
        // A synthetic secure origin enables WebGPU without contacting a server or disabling security.
        browser
            .call(
                "Page.navigate",
                json!({"url":"https://biliup-editor.invalid/"}),
            )
            .await?;
        browser.eval("new Promise(resolve => document.readyState === 'complete' ? resolve(true) : window.addEventListener('load', () => resolve(true), {once:true}))").await?;
        browser
            .call(
                "Emulation.setDefaultBackgroundColorOverride",
                json!({"color":{"r":0,"g":0,"b":0,"a":0}}),
            )
            .await?;
        browser
            .eval(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../resources/editor/export-runtime.js"
            )))
            .await?;
        Ok(browser)
    }

    pub async fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        self.seq += 1;
        let id = self.seq;
        self.socket
            .send(Message::Text(
                json!({"id":id,"method":method,"params":params})
                    .to_string()
                    .into(),
            ))
            .await
            .map_err(|e| e.to_string())?;
        let receive = async {
            let mut media_requests = 0;
            while let Some(message) = self.socket.next().await {
                let message = message.map_err(|e| e.to_string())?;
                let Message::Text(message) = message else {
                    continue;
                };
                let value: Value = serde_json::from_str(&message).map_err(|e| e.to_string())?;
                if value["method"] == "Runtime.executionContextCreated" {
                    let context = &value["params"]["context"];
                    if context["origin"] == "https://biliup-editor.invalid"
                        && context["auxData"]["isDefault"] == true
                        && context["auxData"]["frameId"].as_str() == self.main_frame.as_deref()
                    {
                        self.context = context["id"].as_u64();
                    }
                } else if value["method"] == "Runtime.bindingCalled"
                    && value["params"]["name"] == "__biliup_graphic_media"
                {
                    if self.context.is_none()
                        || value["params"]["executionContextId"].as_u64() != self.context
                    {
                        continue;
                    }
                    let Some(payload) = value["params"]["payload"]
                        .as_str()
                        .filter(|s| s.len() <= 2048)
                    else {
                        continue;
                    };
                    let Ok(payload) = serde_json::from_str::<Value>(payload) else {
                        continue;
                    };
                    let Some(seq) = payload["seq"]
                        .as_u64()
                        .filter(|v| *v <= 9_007_199_254_740_991)
                    else {
                        continue;
                    };
                    media_requests += 1;
                    let result = async {
                        if media_requests > 64 {
                            return Err("单帧请求的素材数量超过限制".to_string());
                        }
                        let request: super::super::graphic_media::Request =
                            serde_json::from_value(payload["request"].clone())
                                .map_err(|_| "图形素材请求无效")?;
                        let media = self.media.as_ref().ok_or("此图形没有关联工程素材")?;
                        media.picture(&request, &self.cancel).await
                    }
                    .await;
                    let reply = match result {
                        Ok(p) => {
                            json!({"bytes":STANDARD.encode(p.bytes.as_slice()),"type":p.mime,"width":p.width,"height":p.height})
                        }
                        Err(error) => json!({"error":error}),
                    };
                    self.seq += 1;
                    self.socket.send(Message::Text(json!({"id":self.seq,"method":"Runtime.evaluate","params":{"expression":format!("BiliupExport.graphicMediaReply({seq},{reply})"),"awaitPromise":false,"returnByValue":true}}).to_string().into())).await.map_err(|e|e.to_string())?;
                } else if value["method"] == "Fetch.requestPaused" {
                    self.seq += 1;
                    let request = &value["params"];
                    let (method, params) = if request["request"]["url"]
                        == "https://biliup-editor.invalid/"
                        && request["resourceType"] == "Document"
                    {
                        (
                            "Fetch.fulfillRequest",
                            json!({"requestId":request["requestId"],"responseCode":200,"responseHeaders":[{"name":"Content-Type","value":"text/html; charset=utf-8"}],"body":STANDARD.encode("<!doctype html><html><head></head><body></body></html>")}),
                        )
                    } else {
                        (
                            "Fetch.failRequest",
                            json!({"requestId":request["requestId"],"errorReason":"BlockedByClient"}),
                        )
                    };
                    self.socket
                        .send(Message::Text(
                            json!({"id":self.seq,"method":method,"params":params})
                                .to_string()
                                .into(),
                        ))
                        .await
                        .map_err(|e| e.to_string())?;
                } else if value["id"] == id {
                    if !value["error"].is_null() {
                        return Err(format!("渲染器命令失败：{}", value["error"]));
                    }
                    return Ok(value["result"].clone());
                }
            }
            Err("工程渲染器已断开".into())
        };
        let cancel = self.cancel.clone();
        tokio::select! {
            _ = cancel.cancelled() => Err("导出已取消".into()),
            result = tokio::time::timeout(Duration::from_secs(60), receive) => result.map_err(|_| "工程渲染器无响应".to_string())?,
        }
    }

    pub async fn eval(&mut self, expression: &str) -> Result<Value, String> {
        let value = self
            .call(
                "Runtime.evaluate",
                json!({"expression":expression,"awaitPromise":true,"returnByValue":true}),
            )
            .await?;
        if !value["exceptionDetails"].is_null() {
            return Err(value["exceptionDetails"]["exception"]["description"]
                .as_str()
                .unwrap_or("工程渲染计算失败")
                .into());
        }
        Ok(value["result"]["value"].clone())
    }

    pub async fn viewport(&mut self, width: u32, height: u32) -> Result<(), String> {
        self.call(
            "Emulation.setDeviceMetricsOverride",
            json!({"width":width,"height":height,"deviceScaleFactor":1,"mobile":false}),
        )
        .await?;
        Ok(())
    }
    pub fn media(&mut self, project: &Value, paths: &std::collections::BTreeMap<String, String>) {
        self.media = Some(super::super::graphic_media::ProjectFrames::new(
            project, paths,
        ));
    }

    pub async fn png(&mut self) -> Result<Vec<u8>, String> {
        let value = self
            .call(
                "Page.captureScreenshot",
                json!({"format":"png","fromSurface":true,"captureBeyondViewport":false}),
            )
            .await?;
        STANDARD
            .decode(value["data"].as_str().ok_or("渲染器未返回图像")?)
            .map_err(|e| e.to_string())
    }
    pub async fn jpeg(&mut self) -> Result<Vec<u8>, String> {
        let value=self.call("Page.captureScreenshot",json!({"format":"jpeg","quality":85,"fromSurface":true,"captureBeyondViewport":false})).await?;
        STANDARD
            .decode(value["data"].as_str().ok_or("渲染器未返回图像")?)
            .map_err(|e| e.to_string())
    }

    pub async fn transparent_png(&mut self) -> Result<Vec<u8>, String> {
        self.call(
            "Emulation.setDefaultBackgroundColorOverride",
            json!({"color":{"r":0,"g":0,"b":0,"a":0}}),
        )
        .await?;
        let result = self.png().await;
        self.call("Emulation.setDefaultBackgroundColorOverride", json!({}))
            .await?;
        result
    }

    pub async fn close(mut self) {
        let _ = self.child.kill().await;
        let _ = self.child.wait().await;
        let _ = self.profile.close();
    }
}
