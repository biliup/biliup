use crate::UploadLine;
use crate::server::errors::{AppError, AppResult};
use crate::upload_lock::UploadLock;
use biliup::client::StatelessClient;
use biliup::error::Kind;
use biliup::uploader::bilibili::{BiliBili, Studio, Vid, Video};
use biliup::uploader::credential::{Credential, LoginInfo, save_login_info};
use biliup::uploader::line::Probe;
use biliup::uploader::util::SubmitOption;
use biliup::uploader::{VideoFile, credential, line, load_config};
use bytes::{Buf, Bytes};
use clap::ValueEnum;
use dialoguer::Input;
use dialoguer::Select;
use dialoguer::theme::ColorfulTheme;
use error_stack::ResultExt;
use futures::{Stream, StreamExt};
use image::Luma;
use indicatif::{ProgressBar, ProgressStyle};
use qrcode::QrCode;
use qrcode::render::unicode;
use reqwest::Body;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::ffi::OsStr;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::Poll;
use std::time::{Instant, UNIX_EPOCH};
use tracing::{info, warn};

// 断点续传的数据结构
#[derive(Serialize, Deserialize, Debug)]
struct UploadCheckpoint {
    manifest: UploadManifest,
    videos: Vec<Video>,
    uploaded_files: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
struct UploadManifest {
    account_id: u64,
    files: Vec<UploadFileIdentity>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
struct UploadFileIdentity {
    path: PathBuf,
    size: u64,
    modified: Option<(u64, u32)>,
}

impl UploadManifest {
    fn new(account_id: u64, paths: &[PathBuf]) -> std::io::Result<Self> {
        let files = paths
            .iter()
            .map(|path| {
                let path = path.canonicalize()?;
                let metadata = std::fs::metadata(&path)?;
                let modified = metadata
                    .modified()
                    .ok()
                    .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                    .map(|duration| (duration.as_secs(), duration.subsec_nanos()));
                Ok(UploadFileIdentity {
                    path,
                    size: metadata.len(),
                    modified,
                })
            })
            .collect::<std::io::Result<_>>()?;
        Ok(Self { account_id, files })
    }

    fn checkpoint_name(&self) -> std::io::Result<String> {
        let encoded = serde_json::to_vec(self)?;
        Ok(format!(
            "biliup_checkpoint_{:x}.json",
            Sha256::digest(encoded)
        ))
    }
}

impl UploadCheckpoint {
    fn new(manifest: UploadManifest) -> Self {
        Self {
            manifest,
            videos: Vec::new(),
            uploaded_files: Vec::new(),
        }
    }

    fn load(path: &Path, manifest: &UploadManifest, paths: &[PathBuf]) -> Option<Self> {
        let checkpoint: Self = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
        let expected_files: Vec<_> = paths
            .iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect();
        (checkpoint.manifest == *manifest
            && checkpoint.videos.len() == checkpoint.uploaded_files.len()
            && expected_files.starts_with(&checkpoint.uploaded_files))
        .then_some(checkpoint)
    }

    fn save(&self, path: &Path) -> std::io::Result<()> {
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(parent)?;
        let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
        serde_json::to_writer_pretty(&mut temporary, self)?;
        temporary.write_all(b"\n")?;
        temporary.as_file().sync_all()?;
        temporary.persist(path).map_err(|error| error.error)?;
        Ok(())
    }

    fn is_uploaded(&self, file_path: &Path) -> bool {
        let file_name = file_path.to_string_lossy().to_string();
        self.uploaded_files.contains(&file_name)
    }

    fn add_video(&mut self, file_path: &Path, video: Video) {
        self.videos.push(video);
        self.uploaded_files
            .push(file_path.to_string_lossy().to_string());
    }
}

pub async fn login(user_cookie: PathBuf, proxy: Option<&str>) -> AppResult<()> {
    let client = Credential::new(proxy);
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("选择一种登录方式")
        .default(1)
        .item("账号密码")
        .item("短信登录")
        .item("扫码登录")
        .item("浏览器登录")
        .item("网页Cookie登录1")
        .item("网页Cookie登录2")
        .interact()
        .change_context_lazy(|| AppError::Unknown)?;
    let info = match selection {
        0 => login_by_password(client).await?,
        1 => login_by_sms(client).await?,
        2 => login_by_qrcode(client).await?,
        3 => login_by_browser(client).await?,
        4 => login_by_web_cookies(client).await?,
        5 => login_by_webqr_cookies(client).await?,
        _ => panic!(),
    };
    save_login_info(&user_cookie, &info)
        .await
        .change_context_lazy(|| AppError::Unknown)?;
    info!("登录成功，凭据已保存到 {}", user_cookie.display());
    Ok(())
}

pub async fn renew(user_cookie: PathBuf, proxy: Option<&str>) -> AppResult<()> {
    credential::renew_login_info_file(&user_cookie, proxy)
        .await
        .change_context_lazy(|| AppError::Unknown)?;
    info!("登录凭据刷新成功");
    Ok(())
}

pub async fn upload_by_command(
    mut studio: Studio,
    user_cookie: PathBuf,
    video_path: Vec<PathBuf>,
    line: Option<UploadLine>,
    limit: usize,
    submit: SubmitOption,
    proxy: Option<&str>,
) -> AppResult<()> {
    if video_path.is_empty() {
        return Err(AppError::Custom(
            "No video files specified. Please provide at least one video file path.".to_string(),
        )
        .into());
    }
    let bili = login_by_cookies(user_cookie, proxy).await?;
    if studio.title.is_empty() {
        studio.title = video_path[0]
            .file_stem()
            .and_then(OsStr::to_str)
            .map(|s| s.to_string())
            .unwrap();
    }
    cover_up(&mut studio, &bili).await?;
    studio.videos = upload(&video_path, &bili, line, limit).await?;

    match submit {
        SubmitOption::BCutAndroid => bili
            .submit_by_bcut_android(&studio, proxy)
            .await
            .change_context_lazy(|| AppError::Unknown)?,
        SubmitOption::Web => bili
            .submit_by_web(&studio, proxy)
            .await
            .change_context_lazy(|| AppError::Unknown)?,
        _ => bili
            .submit_by_app(&studio, proxy)
            .await
            .change_context_lazy(|| AppError::Unknown)?,
    };

    Ok(())
}

pub async fn upload_by_config(
    config: PathBuf,
    user_cookie: PathBuf,
    submit_override: Option<SubmitOption>,
    proxy: Option<&str>,
) -> AppResult<()> {
    // println!("number of concurrent futures: {limit}");
    let bilibili = login_by_cookies(user_cookie, proxy).await?;
    let config = load_config(&config).change_context_lazy(|| AppError::Unknown)?;
    for (filename_patterns, mut studio) in config.streamers {
        let mut paths = Vec::new();
        for entry in glob::glob(&filename_patterns)
            .change_context_lazy(|| AppError::Unknown)?
            .filter_map(Result::ok)
        {
            paths.push(entry);
        }
        if paths.is_empty() {
            warn!("未搜索到匹配的视频文件：{filename_patterns}");
            continue;
        }
        cover_up(&mut studio, &bilibili).await?;

        studio.videos = upload(
            &paths,
            &bilibili,
            config
                .line
                .as_ref()
                .and_then(|l| UploadLine::from_str(l, true).ok()),
            config.limit,
        )
        .await?;
        // 命令行参数优先，如果没有提供则使用配置文件中的设置
        let submit_option = submit_override.clone().unwrap_or(config.submit.clone());
        match submit_option {
            SubmitOption::BCutAndroid => bilibili
                .submit_by_bcut_android(&studio, proxy)
                .await
                .change_context_lazy(|| AppError::Unknown)?,
            SubmitOption::Web => bilibili
                .submit_by_web(&studio, proxy)
                .await
                .change_context_lazy(|| AppError::Unknown)?,
            _ => bilibili
                .submit_by_app(&studio, proxy)
                .await
                .change_context_lazy(|| AppError::Unknown)?,
        };
    }
    Ok(())
}

pub async fn append(
    user_cookie: PathBuf,
    vid: Vid,
    video_path: Vec<PathBuf>,
    line: Option<UploadLine>,
    limit: usize,
    submit: SubmitOption,
    proxy: Option<&str>,
) -> AppResult<()> {
    if video_path.is_empty() {
        return Err(AppError::Custom(
            "No video files specified. Please provide at least one video file path.".to_string(),
        )
        .into());
    }
    let bilibili = login_by_cookies(user_cookie, proxy).await?;
    let mut uploaded_videos = upload(&video_path, &bilibili, line, limit).await?;
    let mut studio = bilibili
        .studio_data(&vid, proxy)
        .await
        .change_context_lazy(|| AppError::Unknown)?;
    studio.videos.append(&mut uploaded_videos);
    match submit {
        SubmitOption::App => bilibili
            .edit_by_app(&studio, proxy)
            .await
            .change_context_lazy(|| AppError::Unknown)?,
        _ => bilibili
            .edit_by_web(&studio)
            .await
            .change_context_lazy(|| AppError::Unknown)?,
    };
    // studio.edit(&login_info).await?;
    Ok(())
}

pub async fn show(user_cookie: PathBuf, vid: Vid, proxy: Option<&str>) -> AppResult<()> {
    let bilibili = login_by_cookies(user_cookie, proxy).await?;
    let video_info = bilibili
        .video_data(&vid, proxy)
        .await
        .change_context_lazy(|| AppError::Unknown)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&video_info).change_context_lazy(|| AppError::Unknown)?
    );
    Ok(())
}

pub async fn comments(
    user_cookie: PathBuf,
    vid: Vid,
    sort: u8,
    pn: u32,
    ps: u32,
    proxy: Option<&str>,
) -> AppResult<()> {
    let bilibili = login_by_cookies(user_cookie, proxy).await?;
    let reply_list = bilibili
        .comments(&vid, sort, pn, ps, proxy)
        .await
        .change_context_lazy(|| AppError::Unknown)?;

    for reply in reply_list.replies.unwrap_or_default() {
        println!("rpid={}  uname={}", reply.rpid, reply.member.uname);
        println!("{}", reply.content.message);
        println!();
    }

    Ok(())
}

pub async fn reply(
    user_cookie: PathBuf,
    vid: Vid,
    rpid: u64,
    message: String,
    execute: bool,
    proxy: Option<&str>,
) -> AppResult<()> {
    if !execute {
        println!("dry-run: reply to {vid} rpid={rpid}");
        println!("{message}");
        println!("use --execute to send");
        return Ok(());
    }

    let bilibili = login_by_cookies(user_cookie, proxy).await?;
    let ret = bilibili
        .reply_comment(&vid, rpid, &message, proxy)
        .await
        .change_context_lazy(|| AppError::Unknown)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&ret).change_context_lazy(|| AppError::Unknown)?
    );
    Ok(())
}

pub async fn list(
    user_cookie: PathBuf,
    is_pubing: bool,
    pubed: bool,
    not_pubed: bool,
    proxy: Option<&str>,
    from_page: u32,
    max_pages: Option<u32>,
) -> AppResult<()> {
    let status = match (is_pubing, pubed, not_pubed) {
        (true, false, false) => "is_pubing",
        (false, true, false) => "pubed",
        (false, false, true) => "not_pubed",
        (false, false, false) => "is_pubing,pubed,not_pubed",
        _ => {
            tracing::warn!("选项互斥，默认列出所有状态的稿件");
            "is_pubing,pubed,not_pubed"
        }
    };

    let bilibili = login_by_cookies(user_cookie, proxy).await?;
    bilibili
        .recent_archives(status, from_page, max_pages)
        .await
        .change_context_lazy(|| AppError::Unknown)?
        .iter()
        .for_each(|arc| println!("{}", arc.to_string_pretty()));
    Ok(())
}

pub(crate) async fn login_by_cookies(
    user_cookie: PathBuf,
    proxy: Option<&str>,
) -> AppResult<BiliBili> {
    let result = credential::login_by_cookies(&user_cookie, proxy).await;
    Ok(match result {
        Err(Kind::IO(_)) => result.change_context_lazy(|| {
            AppError::Custom(String::from("open cookies file: ") + &user_cookie.to_string_lossy())
        })?,
        _ => {
            let bili = result.change_context_lazy(|| AppError::Unknown)?;
            let info = bili
                .my_info()
                .await
                .change_context_lazy(|| AppError::Unknown)?;
            info!(
                "user: {}",
                info["data"]["name"]
                    .as_str()
                    .ok_or_else(|| AppError::Custom(format!("{info}no name")))?
            );
            bili
        }
    })
}

pub async fn cover_up(studio: &mut Studio, bili: &BiliBili) -> AppResult<()> {
    if !studio.cover.is_empty() {
        // 扩展路径中的 ~ 为用户主目录
        let expanded = shellexpand::tilde(&studio.cover);
        let cover_path = PathBuf::from(expanded.as_ref());

        let url = bili
            .cover_up(&std::fs::read(&cover_path).change_context_lazy(|| {
                AppError::Custom(format!("cover: {}", cover_path.display()))
            })?)
            .await
            .change_context_lazy(|| AppError::Unknown)?;
        info!("{url}");
        studio.cover = url;
    }
    Ok(())
}

pub async fn upload(
    video_path: &[PathBuf],
    bili: &BiliBili,
    line: Option<UploadLine>,
    limit: usize,
) -> AppResult<Vec<Video>> {
    info!("number of concurrent futures: {limit}");

    let manifest = UploadManifest::new(bili.login_info.token_info.mid, video_path)
        .change_context_lazy(|| AppError::Custom("读取上传文件状态失败".into()))?;
    let checkpoint_filename = manifest
        .checkpoint_name()
        .change_context(AppError::Unknown)?;

    // 使用平台相关的本地数据目录，Windows 下是 %LOCALAPPDATA%，Linux/macOS 下是 /tmp
    let checkpoint_path = if let Some(data_dir) = dirs::data_local_dir() {
        data_dir.join(checkpoint_filename)
    } else {
        // 如果无法获取数据目录，回退到临时目录
        std::env::temp_dir().join(checkpoint_filename)
    };

    // 尝试加载已有的断点续传数据
    let mut checkpoint = UploadCheckpoint::load(&checkpoint_path, &manifest, video_path)
        .unwrap_or_else(|| {
            info!("No checkpoint found, starting fresh upload");
            UploadCheckpoint::new(manifest)
        });

    if !checkpoint.uploaded_files.is_empty() {
        info!(
            "Found checkpoint with {} uploaded files, resuming...",
            checkpoint.uploaded_files.len()
        );
    }

    let mut videos = checkpoint.videos.clone();
    let client = StatelessClient::default();
    let line = match line {
        Some(UploadLine::Bldsa) => line::bldsa(),
        Some(UploadLine::Cnbldsa) => line::cnbldsa(),
        Some(UploadLine::Andsa) => line::andsa(),
        Some(UploadLine::Atdsa) => line::atdsa(),
        Some(UploadLine::Bda2) => line::bda2(),
        Some(UploadLine::Cnbd) => line::cnbd(),
        Some(UploadLine::Anbd) => line::anbd(),
        Some(UploadLine::Atbd) => line::atbd(),
        Some(UploadLine::Tx) => line::tx(),
        Some(UploadLine::Cntx) => line::cntx(),
        Some(UploadLine::Antx) => line::antx(),
        Some(UploadLine::Attx) => line::attx(),
        Some(UploadLine::Txa) => line::txa(),
        Some(UploadLine::Alia) => line::alia(),
        Some(UploadLine::Estx) => line::estx(),
        Some(UploadLine::Akbd) => line::akbd(),
        _ => Probe::probe(&client.client).await.unwrap_or_default(),
    };
    // let line = line::kodo();
    for video_path in video_path {
        // 检查文件是否已经上传
        if checkpoint.is_uploaded(video_path) {
            info!("Skipping already uploaded file: {}", video_path.display());
            continue;
        }

        info!("{line:?}");
        let video_file = VideoFile::new(video_path).change_context_lazy(|| {
            AppError::Custom(format!("file {}", video_path.to_string_lossy()))
        })?;
        let total_size = video_file.total_size;
        let file_name = video_file.file_name.clone();

        // 使用通用的 retry 函数处理限流错误（code: 601）
        // 配合账号级互斥锁防止多进程同时重试
        let credential_id = format!("{}", bili.login_info.token_info.mid);
        let upload_lock = Arc::new(Mutex::new(
            UploadLock::new(&credential_id)
                .map_err(|e| AppError::Custom(format!("Failed to create upload lock: {}", e)))?,
        ));

        // 在开始上传前检查是否有其他进程正在等待限流恢复
        {
            let lock = upload_lock.lock().unwrap();
            if lock.is_locked() {
                return Err(AppError::Custom(format!(
                    "另一个使用该账号 ({}) 的上传进程正在等待限流恢复，请稍后重试。\n\
                     如果确认当前没有其他上传进程在运行，可能是上次异常退出残留的锁文件，\n\
                     可手动删除以下文件后重试：\n  {}",
                    credential_id,
                    lock.path().display()
                ))
                .into());
            }
        }

        // 用于追踪是否已经尝试获取锁
        let lock_acquired = Arc::new(Mutex::new(false));

        // 执行上传，遇到限流错误时自动重试
        let uploader = {
            let upload_lock_clone = Arc::clone(&upload_lock);
            let lock_acquired_clone = Arc::clone(&lock_acquired);

            biliup::retry_with_config(
                || async {
                    let video_file_clone = VideoFile::new(video_path).map_err(|e| {
                        Kind::Custom(format!("file {}: {}", video_path.to_string_lossy(), e))
                    })?;
                    line.pre_upload(bili, video_file_clone).await
                },
                5,
                Some(move |e: &Kind| {
                    if matches!(e, Kind::RateLimit { .. }) {
                        let mut acquired = lock_acquired_clone.lock().unwrap();
                        if !*acquired {
                            // 第一次遇到限流错误，尝试获取锁
                            let mut lock = upload_lock_clone.lock().unwrap();
                            match lock.try_acquire() {
                                Ok(true) => {
                                    info!("检测到限流，成功获取上传锁，将进行重试");
                                    *acquired = true;
                                    true
                                }
                                Ok(false) => {
                                    warn!("检测到其他进程正在处理限流，本进程退出");
                                    false
                                }
                                Err(e) => {
                                    warn!("尝试获取锁时出错: {}", e);
                                    true // 出错时仍然尝试重试
                                }
                            }
                        } else {
                            // 已经获取锁，继续重试
                            true
                        }
                    } else {
                        false
                    }
                }),
            )
            .await
            .change_context_lazy(|| AppError::Custom("after retries".to_owned()))?
        };

        // 上传成功后释放锁
        if *lock_acquired.lock().unwrap() {
            let mut lock = upload_lock.lock().unwrap();
            let _ = lock.release();
        }
        //Progress bar
        let pb = ProgressBar::new(total_size);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})").change_context_lazy(|| AppError::Unknown)?);
        // pb.enable_steady_tick(Duration::from_secs(1));
        // pb.tick()

        let instant = Instant::now();

        let video = uploader
            .upload(client.clone(), limit, |vs| {
                vs.map(|chunk| {
                    let pb = pb.clone();
                    let chunk = chunk?;
                    let len = chunk.len();
                    Ok((Progressbar::new(chunk, pb), len))
                })
            })
            .await
            .change_context_lazy(|| AppError::Unknown)?;
        pb.finish_and_clear();
        let t = instant.elapsed().as_millis();
        info!(
            "Upload completed: {file_name} => cost {:.2}s, {:.2} MB/s.",
            t as f64 / 1000.,
            total_size as f64 / 1000. / t as f64
        );

        // 保存断点续传信息
        checkpoint.add_video(video_path, video.clone());
        if let Err(e) = checkpoint.save(&checkpoint_path) {
            warn!("Failed to save checkpoint: {}", e);
        } else {
            info!(
                "Checkpoint saved: {} files uploaded",
                checkpoint.uploaded_files.len()
            );
        }

        videos.push(video);
    }

    // 上传完成后删除断点续传文件
    if checkpoint_path.exists() {
        let _ = std::fs::remove_file(&checkpoint_path);
        info!("All files uploaded successfully, checkpoint removed");
    }

    Ok(videos)
}

pub async fn login_by_password(credential: Credential) -> AppResult<LoginInfo> {
    let username: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("请输入账号")
        .interact()
        .change_context_lazy(|| AppError::Unknown)?;
    let password: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("请输入密码")
        .interact()
        .change_context_lazy(|| AppError::Unknown)?;
    credential
        .login_by_password(&username, &password)
        .await
        .change_context_lazy(|| AppError::Unknown)
}

pub async fn login_by_sms(credential: Credential) -> AppResult<LoginInfo> {
    let country_code: u32 = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("请输入手机国家代码")
        .default(86)
        .interact_text()
        .change_context_lazy(|| AppError::Unknown)?;
    let phone: u64 = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("请输入手机号")
        .interact_text()
        .change_context_lazy(|| AppError::Unknown)?;
    let res = credential
        .send_sms_handle_recaptcha(phone, country_code, |url| async move {
            println!("{url}");
            println!("请复制此链接至浏览器打开并启动开发者工具，完成滑动验证后查看网络请求");

            let challenge: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("请输入get.php响应中的challenge值")
                .interact_text()
                .map_err(|e| e.to_string())?;

            let valiate: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("请输入ajax.php响应中的validate值")
                .interact_text()
                .map_err(|e| e.to_string())?;

            Ok((challenge, valiate))
        })
        .await
        .change_context_lazy(|| AppError::Unknown)?;
    let input: u32 = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("请输入验证码")
        .interact_text()
        .change_context_lazy(|| AppError::Unknown)?;
    // println!("{}", payload);
    credential
        .login_by_sms(input, res)
        .await
        .change_context_lazy(|| AppError::Unknown)
}

pub async fn login_by_qrcode(credential: Credential) -> AppResult<LoginInfo> {
    let value = credential
        .get_qrcode()
        .await
        .change_context_lazy(|| AppError::Unknown)?;
    let code = QrCode::new(
        value["data"]["url"]
            .as_str()
            .unwrap()
            .replace("https", "http"),
    )
    .unwrap();
    let image = code
        .render::<unicode::Dense1x2>()
        .dark_color(unicode::Dense1x2::Light)
        .light_color(unicode::Dense1x2::Dark)
        .build();
    println!("{}", image);
    // Render the bits into an image.
    let image = code.render::<Luma<u8>>().build();
    println!(
        "在Windows下建议使用Windows Terminal(支持utf8，可完整显示二维码)\n否则可能无法正常显示，此时请打开./qrcode.png扫码"
    );
    // Save the image.
    image.save("qrcode.png").unwrap();
    credential
        .login_by_qrcode(value)
        .await
        .change_context_lazy(|| AppError::Unknown)
}

pub async fn login_by_browser(credential: Credential) -> AppResult<LoginInfo> {
    let value = credential
        .get_qrcode()
        .await
        .change_context_lazy(|| AppError::Unknown)?;
    println!(
        "{}",
        value["data"]["url"]
            .as_str()
            .ok_or_else(|| AppError::Custom(value.to_string()))?
    );
    println!("请复制此链接至浏览器中完成登录");
    credential
        .login_by_qrcode(value)
        .await
        .change_context_lazy(|| AppError::Unknown)
}

pub async fn login_by_web_cookies(credential: Credential) -> AppResult<LoginInfo> {
    let sess_data: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("请输入SESSDATA")
        .interact_text()
        .change_context_lazy(|| AppError::Unknown)?;
    let bili_jct: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("请输入bili_jct")
        .interact_text()
        .change_context_lazy(|| AppError::Unknown)?;
    credential
        .login_by_web_cookies(&sess_data, &bili_jct)
        .await
        .change_context_lazy(|| AppError::Unknown)
}

pub async fn login_by_webqr_cookies(credential: Credential) -> AppResult<LoginInfo> {
    let sess_data: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("请输入SESSDATA")
        .interact_text()
        .change_context_lazy(|| AppError::Unknown)?;
    let dede_user_id: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("请输入DedeUserID")
        .interact_text()
        .change_context_lazy(|| AppError::Unknown)?;
    credential
        .login_by_web_qrcode(&sess_data, &dede_user_id)
        .await
        .change_context_lazy(|| AppError::Unknown)
}

impl From<Progressbar> for Body {
    fn from(async_stream: Progressbar) -> Self {
        Body::wrap_stream(async_stream)
    }
}

#[inline]
pub fn fopen_rw<P: AsRef<Path>>(path: P) -> AppResult<std::fs::File> {
    let path = path.as_ref();
    std::fs::File::options()
        .read(true)
        .write(true)
        .open(path)
        .change_context_lazy(|| {
            AppError::Custom(String::from("open cookies file: ") + &path.to_string_lossy())
        })
}

#[derive(Clone)]
struct Progressbar {
    bytes: Bytes,
    pb: ProgressBar,
}

impl Progressbar {
    pub fn new(bytes: Bytes, pb: ProgressBar) -> Self {
        Self { bytes, pb }
    }

    pub fn progress(&mut self) -> AppResult<Option<Bytes>> {
        let pb = &self.pb;

        let content_bytes = &mut self.bytes;

        let n = content_bytes.remaining();

        let pc = 4096;
        if n == 0 {
            Ok(None)
        } else if n < pc {
            pb.inc(n as u64);
            Ok(Some(content_bytes.copy_to_bytes(n)))
        } else {
            pb.inc(pc as u64);

            Ok(Some(content_bytes.copy_to_bytes(pc)))
        }
    }
}

impl Stream for Progressbar {
    type Item = AppResult<Bytes>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        match self.progress()? {
            None => Poll::Ready(None),
            Some(s) => Poll::Ready(Some(Ok(s))),
        }
    }
}

#[cfg(test)]
mod checkpoint_tests {
    use super::*;

    #[test]
    fn checkpoints_resume_only_the_same_account_and_unchanged_physical_files() {
        let dir = tempfile::tempdir().unwrap();
        let video = dir.path().join("a.mp4");
        std::fs::write(&video, b"first video").unwrap();
        let paths = vec![video.clone()];
        let manifest = UploadManifest::new(42, &paths).unwrap();
        let original_name = manifest.checkpoint_name().unwrap();
        let checkpoint_path = dir.path().join("nested/checkpoint.json");
        let mut checkpoint = UploadCheckpoint::new(manifest);
        checkpoint.add_video(&video, Video::new("uploaded-key"));
        checkpoint.save(&checkpoint_path).unwrap();

        let same = UploadManifest::new(42, &paths).unwrap();
        let resumed = UploadCheckpoint::load(&checkpoint_path, &same, &paths).unwrap();
        assert_eq!(resumed.videos[0].filename, "uploaded-key");
        assert!(resumed.is_uploaded(&video));

        let other_account = UploadManifest::new(43, &paths).unwrap();
        assert_ne!(original_name, other_account.checkpoint_name().unwrap());
        assert!(UploadCheckpoint::load(&checkpoint_path, &other_account, &paths).is_none());

        let other_dir = tempfile::tempdir().unwrap();
        let other = other_dir.path().join("a.mp4");
        std::fs::write(&other, b"first video").unwrap();
        let other_paths = vec![other];
        let other_manifest = UploadManifest::new(42, &other_paths).unwrap();
        assert_ne!(original_name, other_manifest.checkpoint_name().unwrap());
        assert!(UploadCheckpoint::load(&checkpoint_path, &other_manifest, &other_paths).is_none());

        std::fs::write(&video, b"a replacement video").unwrap();
        let replaced = UploadManifest::new(42, &paths).unwrap();
        assert_ne!(original_name, replaced.checkpoint_name().unwrap());
        assert!(UploadCheckpoint::load(&checkpoint_path, &replaced, &paths).is_none());
    }

    #[test]
    fn checkpoints_require_a_complete_ordered_prefix_and_reject_legacy_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let paths: Vec<_> = ["a.mp4", "b.mp4"].map(|name| dir.path().join(name)).into();
        for path in &paths {
            std::fs::write(path, b"video").unwrap();
        }
        let manifest = UploadManifest::new(42, &paths).unwrap();
        let path = dir.path().join("checkpoint.json");
        let mut checkpoint = UploadCheckpoint::new(UploadManifest::new(42, &paths).unwrap());
        checkpoint.add_video(&paths[0], Video::new("uploaded-key"));
        checkpoint.save(&path).unwrap();
        assert!(UploadCheckpoint::load(&path, &manifest, &paths).is_some());

        checkpoint.videos.clear();
        checkpoint.save(&path).unwrap();
        assert!(UploadCheckpoint::load(&path, &manifest, &paths).is_none());

        checkpoint.videos.push(Video::new("uploaded-key"));
        checkpoint.uploaded_files[0] = paths[1].to_string_lossy().to_string();
        checkpoint.save(&path).unwrap();
        assert!(UploadCheckpoint::load(&path, &manifest, &paths).is_none());

        std::fs::write(&path, r#"{"videos":[],"uploaded_files":[]}"#).unwrap();
        assert!(UploadCheckpoint::load(&path, &manifest, &paths).is_none());
        assert!(
            path.exists(),
            "untrusted legacy checkpoints are left untouched"
        );
        assert!(UploadManifest::new(42, &[dir.path().join("missing.mp4")]).is_err());
    }

    #[test]
    fn checkpoints_change_when_equal_sized_files_are_modified() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("video.mp4");
        std::fs::write(&path, b"first").unwrap();
        let paths = [path.clone()];
        let original = UploadManifest::new(42, &paths).unwrap();
        let original_time = original.files[0].modified.unwrap();

        std::fs::write(&path, b"other").unwrap();
        std::fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(UNIX_EPOCH + std::time::Duration::from_secs(original_time.0 + 60))
            .unwrap();

        let changed = UploadManifest::new(42, &paths).unwrap();
        assert_eq!(original.files[0].size, changed.files[0].size);
        assert_ne!(
            original.checkpoint_name().unwrap(),
            changed.checkpoint_name().unwrap()
        );
    }

    #[test]
    fn saving_a_checkpoint_atomically_replaces_the_previous_progress() {
        let dir = tempfile::tempdir().unwrap();
        let paths = [dir.path().join("a.mp4"), dir.path().join("b.mp4")];
        for path in &paths {
            std::fs::write(path, b"video").unwrap();
        }
        let manifest = UploadManifest::new(42, &paths).unwrap();
        let checkpoint_path = dir.path().join("checkpoint.json");
        let mut checkpoint = UploadCheckpoint::new(UploadManifest::new(42, &paths).unwrap());
        checkpoint.add_video(&paths[0], Video::new("first-key"));
        checkpoint.save(&checkpoint_path).unwrap();
        checkpoint.add_video(&paths[1], Video::new("second-key"));
        checkpoint.save(&checkpoint_path).unwrap();

        let resumed = UploadCheckpoint::load(&checkpoint_path, &manifest, &paths).unwrap();
        assert_eq!(resumed.videos.len(), 2);
        assert_eq!(resumed.videos[1].filename, "second-key");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 3);
    }
}
