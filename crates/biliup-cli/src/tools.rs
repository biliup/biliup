//! External programs: where ffmpeg is, and how child processes are started.
//!
//! ffmpeg is looked up in this order: the `ffmpeg_path` setting, then the one
//! the embedding host ships (the desktop app passes the ffmpeg bundled in its
//! installer through `ServeOptions::ffmpeg`), then `ffmpeg` on `PATH`.
//!
//! Danmaku XML -> ASS conversion uses the native Rust converter built into
//! biliup (see `crate::danmaku_worker`). The render tools
//! directory only provides the font shared by the browser preview and FFmpeg.

use serde::Serialize;
use std::ffi::OsStr;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Mutex, Once, OnceLock, PoisonError, RwLock};
use std::time::Duration;

pub use biliup::tools::{command, std_command};

const FFMPEG: &str = "ffmpeg";
const RENDER_TOOLS_ENV: &str = "BILIUP_RENDER_TOOLS_DIR";
const RENDER_TOOLS_DIR: &str = "render-tools";
const RENDER_FONT: &str = "NotoSansSC-Regular.otf";
const RENDER_FONT_BYTES: &[u8] = include_bytes!("assets/NotoSansSC-Regular.otf");
const RENDER_EMOJI_FONT: &[u8] = include_bytes!("assets/NotoEmoji.ttf");
const VERSION_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FfmpegSource {
    /// The `ffmpeg_path` setting
    Config,
    /// Shipped with the embedding host (the desktop installer)
    Bundled,
    /// `ffmpeg` on `PATH`
    Path,
}

struct Locations {
    configured: Option<PathBuf>,
    bundled: Option<PathBuf>,
    bundled_render_tools: Option<PathBuf>,
}

static LOCATIONS: RwLock<Locations> = RwLock::new(Locations {
    configured: None,
    bundled: None,
    bundled_render_tools: None,
});

/// Successful `ffmpeg -version` results, keyed by the program that was run.
static VERSION_CACHE: Mutex<Option<(PathBuf, FfmpegVersion)>> = Mutex::new(None);
static EMBEDDED_RENDER_TOOLS: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Applies the `ffmpeg_path` setting; blank means unset.
pub fn set_configured_ffmpeg(path: Option<&str>) {
    let path = path
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(PathBuf::from);
    LOCATIONS
        .write()
        .unwrap_or_else(PoisonError::into_inner)
        .configured = path;
}

/// Sets the ffmpeg shipped with the embedding host.
pub fn set_bundled_ffmpeg(path: Option<PathBuf>) {
    LOCATIONS
        .write()
        .unwrap_or_else(PoisonError::into_inner)
        .bundled = path;
}

/// The legacy `danmaku_factory_path` setting is accepted but ignored: the
/// converter is built in, and a failing built-in conversion is reported
/// rather than silently handed to an external program.
pub fn warn_legacy_danmaku_factory_path(path: Option<&str>) {
    static WARNED: Once = Once::new();
    if path.is_some_and(|p| !p.trim().is_empty()) {
        WARNED.call_once(|| {
            tracing::warn!(
                "danmaku_factory_path is no longer used: XML to ASS conversion is built in; \
                 remove the setting"
            );
        });
    }
}

/// Sets the directory shipped by an embedding host. It contains a `fonts/`
/// directory.
pub fn set_bundled_render_tools(path: Option<PathBuf>) {
    LOCATIONS
        .write()
        .unwrap_or_else(PoisonError::into_inner)
        .bundled_render_tools = path;
}

fn bundled_render_tools() -> Option<PathBuf> {
    let locations = LOCATIONS.read().unwrap_or_else(PoisonError::into_inner);
    if let Some(path) = &locations.bundled_render_tools {
        return Some(path.clone());
    }
    if let Some(path) = std::env::var_os(RENDER_TOOLS_ENV).map(PathBuf::from) {
        return Some(path);
    }
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.join(RENDER_TOOLS_DIR)))
}

fn render_font_files_in(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<_> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && matches!(
                    path.extension()
                        .and_then(|e| e.to_str())
                        .map(str::to_ascii_lowercase)
                        .as_deref(),
                    Some("ttf" | "otf" | "ttc")
                )
        })
        .collect();
    files.sort_by_key(|path| {
        let name = path.file_name().and_then(OsStr::to_str).unwrap_or_default();
        if name == RENDER_FONT { 0 } else { 1 }
    });
    files
}

fn embedded_render_font_matches(path: &Path) -> bool {
    std::fs::read(path).is_ok_and(|bytes| bytes.as_slice() == RENDER_FONT_BYTES)
}

fn materialize_embedded_render_tools() -> Option<PathBuf> {
    let cache = dirs::cache_dir().unwrap_or_else(|| {
        std::env::temp_dir().join(format!("biliup-cache-{}", std::process::id()))
    });
    let root = cache.join("biliup").join("render-tools-embedded-v1");
    let fonts = root.join("fonts");
    std::fs::create_dir_all(&fonts).ok()?;
    let font = fonts.join(RENDER_FONT);
    if embedded_render_font_matches(&font) {
        return Some(root);
    }

    let mut temporary = tempfile::NamedTempFile::new_in(&fonts).ok()?;
    temporary.write_all(RENDER_FONT_BYTES).ok()?;
    if temporary.persist(&font).is_err() && !embedded_render_font_matches(&font) {
        // On platforms that cannot replace an existing file atomically, remove
        // only the invalid cached copy and retry through a fresh temporary file.
        std::fs::remove_file(&font).ok()?;
        let mut replacement = tempfile::NamedTempFile::new_in(&fonts).ok()?;
        replacement.write_all(RENDER_FONT_BYTES).ok()?;
        replacement.persist(&font).ok()?;
    }
    embedded_render_font_matches(&font).then_some(root)
}

fn embedded_render_tools() -> Option<PathBuf> {
    EMBEDDED_RENDER_TOOLS
        .get_or_init(materialize_embedded_render_tools)
        .clone()
}

/// Directory containing fonts shared by FFmpeg/libass and browser preview.
/// Shipped render-tools take precedence; the CJK font embedded in the binary
/// is materialized into the user cache when no external font is available.
pub fn render_fonts_dir() -> Option<PathBuf> {
    let bundled = bundled_render_tools()
        .map(|dir| dir.join("fonts"))
        .filter(|path| !render_font_files_in(path).is_empty());
    bundled.or_else(|| embedded_render_tools().map(|dir| dir.join("fonts")))
}

/// Returns the preferred bundled CJK font, or another supported font in the
/// shared font directory.
pub fn render_font() -> Option<PathBuf> {
    let dir = render_fonts_dir()?;
    render_font_in_with_embedded_fallback(&dir, || {
        embedded_render_tools().map(|root| root.join("fonts").join(RENDER_FONT))
    })
}

fn render_font_in_with_embedded_fallback(
    dir: &Path,
    embedded: impl FnOnce() -> Option<PathBuf>,
) -> Option<PathBuf> {
    if let Some(font) = render_font_in(dir, None) {
        return Some(font);
    }
    let embedded = embedded()?;
    render_font_in(dir, Some(&embedded))
}

fn render_font_in(dir: &Path, embedded: Option<&Path>) -> Option<PathBuf> {
    let preferred = dir.join(RENDER_FONT);
    if preferred.is_file() {
        return Some(preferred);
    }
    if let Some(path) = render_font_files_in(&dir)
        .into_iter()
        .find(|path| path.file_name().and_then(OsStr::to_str) != Some("NotoEmoji.ttf"))
    {
        return Some(path);
    }
    embedded
        .filter(|path| path.is_file())
        .map(Path::to_path_buf)
}

pub fn render_font_path() -> Result<PathBuf, String> {
    render_font()
        .ok_or_else(|| "找不到弹幕字体：请安装随包字体或设置 BILIUP_RENDER_TOOLS_DIR".into())
}

/// Font files exposed to the browser font endpoint. Paths are always under
/// the shared bundled font directory.
pub fn render_font_files() -> Vec<PathBuf> {
    let Some(dir) = render_fonts_dir() else {
        return Vec::new();
    };
    render_font_files_in(&dir)
}

/// Monochrome Noto Emoji is embedded in the binary so libass and the browser
/// preview always have a deterministic fallback for emoji missing from CJK.
pub fn render_emoji_font_bytes() -> &'static [u8] {
    RENDER_EMOJI_FONT
}

/// The ffmpeg program to run and where it came from.
pub fn locate_ffmpeg() -> (PathBuf, FfmpegSource) {
    let locations = LOCATIONS.read().unwrap_or_else(PoisonError::into_inner);
    resolve(
        locations.configured.as_deref(),
        locations.bundled.as_deref(),
    )
}

/// The ffmpeg program to run.
pub fn ffmpeg() -> PathBuf {
    locate_ffmpeg().0
}

/// A [`command`] that runs ffmpeg.
pub fn ffmpeg_command() -> tokio::process::Command {
    command(ffmpeg())
}

/// For tests that return early because a tool (ffmpeg, an encoder) is
/// missing. A skipped test still reports `ok`, and libtest captures
/// `eprintln!`, so this writes to stderr directly: CI logs then show which
/// tests did not really run.
#[cfg(test)]
pub(crate) fn note_skipped_test(reason: &str) {
    use std::io::Write;
    let thread = std::thread::current();
    let name = thread.name().unwrap_or("?");
    let _ = writeln!(std::io::stderr(), "SKIPPED {name}: {reason}");
}

/// [`ffmpeg_command`] at a lower CPU priority, for background work that must
/// not slow down recording: `nice 10` on Unix, `BELOW_NORMAL_PRIORITY_CLASS`
/// on Windows.
pub fn low_priority_ffmpeg_command() -> tokio::process::Command {
    #[cfg_attr(not(any(unix, windows)), allow(unused_mut))]
    let mut command = std_command(ffmpeg());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x0000_4000;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        // Setting the flags replaces the ones `std_command` chose, so keep its console rule.
        let no_window = if biliup::tools::hides_console_windows() {
            CREATE_NO_WINDOW
        } else {
            0
        };
        command.creation_flags(BELOW_NORMAL_PRIORITY_CLASS | no_window);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // SAFETY: nice() is async-signal-safe and only changes the child's own priority.
        unsafe {
            command.pre_exec(|| {
                libc::nice(10);
                Ok(())
            });
        }
    }
    tokio::process::Command::from(command)
}

fn resolve(configured: Option<&Path>, bundled: Option<&Path>) -> (PathBuf, FfmpegSource) {
    if let Some(path) = configured {
        (path.to_path_buf(), FfmpegSource::Config)
    } else if let Some(path) = bundled {
        (path.to_path_buf(), FfmpegSource::Bundled)
    } else {
        (PathBuf::from(FFMPEG), FfmpegSource::Path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FfmpegVersion {
    /// First line of `ffmpeg -version`
    pub version: String,
    /// SPDX id of the license the build is under, derived from its configure flags
    pub license: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FfmpegStatus {
    pub available: bool,
    pub source: FfmpegSource,
    /// The program that is run; `None` when it is not found on `PATH`
    pub path: Option<PathBuf>,
    pub version: Option<String>,
    pub license: Option<&'static str>,
    pub error: Option<String>,
}

/// Runs `ffmpeg -version` on the ffmpeg that would be used. Successful results
/// are cached per program, failures are retried on the next call.
pub async fn ffmpeg_status() -> FfmpegStatus {
    let (program, source) = locate_ffmpeg();
    let path = match source {
        FfmpegSource::Path => find_in_path(FFMPEG, std::env::var_os("PATH").as_deref()),
        _ => Some(program.clone()),
    };
    let cached = VERSION_CACHE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
        .filter(|(cached, _)| *cached == program)
        .map(|(_, version)| version.clone());
    let result = match cached {
        Some(version) => Ok(version),
        None => probe_version(&program, source).await.inspect(|version| {
            tracing::info!(
                program = %program.display(),
                ?source,
                version = %version.version,
                "ffmpeg -version succeeded"
            );
            *VERSION_CACHE.lock().unwrap_or_else(PoisonError::into_inner) =
                Some((program.clone(), version.clone()));
        }),
    };
    match result {
        Ok(version) => FfmpegStatus {
            available: true,
            source,
            path,
            version: Some(version.version),
            license: version.license,
            error: None,
        },
        Err(error) => FfmpegStatus {
            available: false,
            source,
            path,
            version: None,
            license: None,
            error: Some(error),
        },
    }
}

async fn probe_version(program: &Path, source: FfmpegSource) -> Result<FfmpegVersion, String> {
    let output = command(program)
        .arg("-version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .output();
    let output = match tokio::time::timeout(VERSION_TIMEOUT, output).await {
        Ok(Ok(output)) => output,
        Ok(Err(err)) if err.kind() == std::io::ErrorKind::NotFound => {
            let hint = match source {
                FfmpegSource::Config => "请检查配置里的 ffmpeg_path",
                _ => "请安装 FFmpeg 并加入 PATH，或在配置里填写 ffmpeg_path",
            };
            return Err(format!("找不到 {}：{hint}", program.display()));
        }
        Ok(Err(err)) => return Err(format!("无法运行 {}：{err}", program.display())),
        Err(_) => {
            return Err(format!(
                "{} -version 在 {} 秒内没有返回",
                program.display(),
                VERSION_TIMEOUT.as_secs()
            ));
        }
    };
    if !output.status.success() {
        return Err(format!(
            "{} -version 退出码 {}",
            program.display(),
            output.status
        ));
    }
    parse_version_output(&String::from_utf8_lossy(&output.stdout))
        .ok_or_else(|| format!("{} -version 的输出无法识别", program.display()))
}

fn parse_version_output(stdout: &str) -> Option<FfmpegVersion> {
    let version = stdout.lines().next()?.trim();
    if !version.starts_with("ffmpeg version") {
        return None;
    }
    let license = stdout
        .lines()
        .find_map(|line| line.trim().strip_prefix("configuration:"))
        .map(license_from_configuration);
    Some(FfmpegVersion {
        version: version.to_string(),
        license,
    })
}

/// Same rules as FFmpeg's `configure` uses to pick the license it reports.
fn license_from_configuration(configuration: &str) -> &'static str {
    let enabled = |flag: &str| configuration.split_whitespace().any(|f| f == flag);
    match (
        enabled("--enable-nonfree"),
        enabled("--enable-gpl"),
        enabled("--enable-version3"),
    ) {
        (true, _, _) => "LicenseRef-nonfree",
        (false, true, true) => "GPL-3.0-or-later",
        (false, true, false) => "GPL-2.0-or-later",
        (false, false, true) => "LGPL-3.0-or-later",
        (false, false, false) => "LGPL-2.1-or-later",
    }
}

/// Where the OS would find `program` on `path_var`, for display only.
fn find_in_path(program: &str, path_var: Option<&OsStr>) -> Option<PathBuf> {
    let names: Vec<String> = if cfg!(windows) {
        vec![format!("{program}.exe"), program.to_string()]
    } else {
        vec![program.to_string()]
    };
    std::env::split_paths(path_var?)
        .flat_map(|dir| names.iter().map(move |name| dir.join(name)))
        .find(|candidate| candidate.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_cjk_font_materializes_from_compiled_asset() {
        let tools = embedded_render_tools().expect("embedded CJK font should be extractable");
        let font = tools.join("fonts").join(RENDER_FONT);
        assert_eq!(std::fs::read(font).unwrap().as_slice(), RENDER_FONT_BYTES);
    }

    #[test]
    fn emoji_only_directory_uses_only_an_existing_embedded_cjk_font() {
        let fonts = tempfile::tempdir().unwrap();
        std::fs::write(fonts.path().join("NotoEmoji.ttf"), b"emoji").unwrap();

        let missing_fallback = fonts.path().join("missing-cjk.otf");
        assert_eq!(render_font_in(fonts.path(), Some(&missing_fallback)), None);

        let embedded = fonts.path().join("embedded-cjk.otf");
        std::fs::write(&embedded, b"cjk").unwrap();
        assert_eq!(
            render_font_in(fonts.path(), Some(&embedded)),
            Some(embedded)
        );
    }

    #[test]
    fn external_cjk_font_does_not_materialize_embedded_fallback() {
        let fonts = tempfile::tempdir().unwrap();
        let external = fonts.path().join("custom-cjk.ttf");
        std::fs::write(&external, b"cjk").unwrap();
        let embedded_was_requested = std::cell::Cell::new(false);

        let selected = render_font_in_with_embedded_fallback(fonts.path(), || {
            embedded_was_requested.set(true);
            panic!("external CJK font should skip embedded extraction");
        });

        assert_eq!(selected, Some(external));
        assert!(!embedded_was_requested.get());
    }

    #[test]
    fn emoji_only_directory_extracts_embedded_cjk_fallback_on_demand() {
        let fonts = tempfile::tempdir().unwrap();
        std::fs::write(fonts.path().join("NotoEmoji.ttf"), b"emoji").unwrap();
        let fallback = fonts.path().join("embedded-cjk.otf");
        std::fs::write(&fallback, b"cjk").unwrap();

        let selected =
            render_font_in_with_embedded_fallback(fonts.path(), || Some(fallback.clone()));

        assert_eq!(selected, Some(fallback));
    }

    const BTBN_WIN64: &str = "ffmpeg version n8.1.2-50-g1a748fe2cd-20260831 Copyright (c) 2000-2026 the FFmpeg developers\n\
built with gcc 15.2.0 (crosstool-NG 1.28.0.21_3c5cc17)\n\
configuration: --prefix=/ffbuild/prefix --pkg-config-flags=--static --enable-gpl --enable-version3 --disable-debug --enable-libx264 --extra-version=20260831\n\
libavutil      60. 26.100 / 60. 26.100\n";

    #[test]
    fn setting_wins_over_bundled_and_path() {
        let configured = Path::new("/opt/ffmpeg/bin/ffmpeg");
        let bundled = Path::new("C:/biliup/ffmpeg/ffmpeg.exe");
        assert_eq!(
            resolve(Some(configured), Some(bundled)),
            (configured.to_path_buf(), FfmpegSource::Config)
        );
        assert_eq!(
            resolve(None, Some(bundled)),
            (bundled.to_path_buf(), FfmpegSource::Bundled)
        );
        assert_eq!(
            resolve(None, None),
            (PathBuf::from("ffmpeg"), FfmpegSource::Path)
        );
    }

    #[test]
    fn version_line_and_license_are_read_from_the_output() {
        assert_eq!(
            parse_version_output(BTBN_WIN64),
            Some(FfmpegVersion {
                version: "ffmpeg version n8.1.2-50-g1a748fe2cd-20260831 Copyright (c) 2000-2026 the FFmpeg developers".into(),
                license: Some("GPL-3.0-or-later"),
            })
        );
        assert_eq!(parse_version_output("not ffmpeg\n"), None);
        assert_eq!(parse_version_output(""), None);
    }

    #[test]
    fn license_follows_configure() {
        assert_eq!(
            license_from_configuration("--enable-gpl --enable-libx264"),
            "GPL-2.0-or-later"
        );
        assert_eq!(
            license_from_configuration("--enable-version3"),
            "LGPL-3.0-or-later"
        );
        assert_eq!(
            license_from_configuration("--prefix=/usr"),
            "LGPL-2.1-or-later"
        );
        assert_eq!(
            license_from_configuration("--enable-gpl --enable-nonfree"),
            "LicenseRef-nonfree"
        );
        // A flag that merely starts with --enable-gpl is not --enable-gpl.
        assert_eq!(
            license_from_configuration("--enable-gpl-foo"),
            "LGPL-2.1-or-later"
        );
    }

    #[test]
    fn path_lookup_finds_the_first_match() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        let name = if cfg!(windows) {
            "ffmpeg.exe"
        } else {
            "ffmpeg"
        };
        std::fs::write(second.path().join(name), b"").unwrap();
        let path_var = std::env::join_paths([first.path(), second.path()]).unwrap();
        assert_eq!(
            find_in_path("ffmpeg", Some(&path_var)),
            Some(second.path().join(name))
        );
        assert_eq!(find_in_path("ffmpeg", Some(OsStr::new(""))), None);
        assert_eq!(find_in_path("ffmpeg", None), None);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn probe_runs_the_program() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let fake = dir.path().join("ffmpeg");
        std::fs::write(
            &fake,
            "#!/bin/sh\n[ \"$1\" = -version ] || exit 2\nprintf 'ffmpeg version 7.0-test\\nconfiguration: --enable-gpl\\n'\n",
        )
        .unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(
            probe_version(&fake, FfmpegSource::Config).await,
            Ok(FfmpegVersion {
                version: "ffmpeg version 7.0-test".into(),
                license: Some("GPL-2.0-or-later"),
            })
        );

        let missing = dir.path().join("missing");
        let err = probe_version(&missing, FfmpegSource::Config)
            .await
            .unwrap_err();
        assert!(
            err.contains("找不到") && err.contains("请检查配置"),
            "{err}"
        );
        let err = probe_version(Path::new("ffmpeg-not-on-path"), FfmpegSource::Path)
            .await
            .unwrap_err();
        assert!(err.contains("PATH"), "{err}");

        let failing = dir.path().join("failing");
        std::fs::write(&failing, "#!/bin/sh\nexit 3\n").unwrap();
        std::fs::set_permissions(&failing, std::fs::Permissions::from_mode(0o755)).unwrap();
        let err = probe_version(&failing, FfmpegSource::Config)
            .await
            .unwrap_err();
        assert!(err.contains("退出码"), "{err}");
    }
}
