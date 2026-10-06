//! Where the desktop app keeps the server's data (`data/`, `ds_update.log`,
//! `cookies.json` and recordings).
//!
//! On Windows the data stays next to the executable, as with the old sidecar,
//! unless the executable is on the system drive. There the user picks a
//! directory on first start (the app data directory by default) and the choice
//! is saved in the app config directory. Other platforms have no drive letters
//! and install into read-only locations, so they use the app data directory
//! (or a directory saved in the same settings file).

use std::path::{Path, PathBuf};
use std::{fs, io};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, WebviewWindow};
use tauri_plugin_dialog::{
    DialogExt, MessageDialogButtons, MessageDialogKind, MessageDialogResult,
};

const SETTINGS_FILE: &str = "desktop.json";
const DIALOG_TITLE: &str = "选择 biliup 数据目录";
const USE_DEFAULT: &str = "使用默认目录";
const USE_INSTALL_DIR: &str = "使用安装目录";
const PICK_OTHER: &str = "选择其他目录…";

#[derive(Default, Serialize, Deserialize)]
struct Settings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    data_dir: Option<PathBuf>,
}

#[derive(Debug, PartialEq)]
enum Plan {
    Use(PathBuf),
    Ask,
}

fn plan(
    drive_rules: bool,
    install_on_system_drive: bool,
    install_dir: &Path,
    saved: Option<PathBuf>,
    app_data_dir: &Path,
) -> Plan {
    if drive_rules && !install_on_system_drive {
        return Plan::Use(install_dir.to_path_buf());
    }
    match saved {
        Some(dir) => Plan::Use(dir),
        None if drive_rules => Plan::Ask,
        None => Plan::Use(app_data_dir.to_path_buf()),
    }
}

/// Returns the data directory, asking the user when needed. `Ok(None)` means
/// the dialog was closed without a choice. `legacy_roots` are only used to
/// tell the user about old data that will be copied.
pub fn resolve(
    app: &AppHandle,
    window: &WebviewWindow,
    install_dir: &Path,
    legacy_roots: &[PathBuf],
) -> Result<Option<PathBuf>, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|err| format!("无法确定应用数据目录：{err}"))?;
    let settings_file = app
        .path()
        .app_config_dir()
        .map_err(|err| format!("无法确定应用配置目录：{err}"))?
        .join(SETTINGS_FILE);
    let system_drive = std::env::var("SystemDrive").ok();
    let saved = load_settings(&settings_file).data_dir;

    match plan(
        cfg!(windows),
        on_system_drive(install_dir, system_drive.as_deref()),
        install_dir,
        saved.clone(),
        &app_data_dir,
    ) {
        Plan::Use(dir) if saved.as_ref() == Some(&dir) => {
            // A saved directory may be on a drive that is currently missing;
            // report it instead of silently creating an empty one elsewhere.
            ensure_writable(&dir).map_err(|err| {
                format!(
                    "无法使用数据目录 {}：{err}\n如需重新选择，请删除 {} 后重新打开。",
                    dir.display(),
                    settings_file.display()
                )
            })?;
            Ok(Some(dir))
        }
        Plan::Use(dir) => Ok(Some(dir)),
        Plan::Ask => {
            let legacy = find_legacy_root(legacy_roots);
            let Some(dir) = ask(app, window, &app_data_dir, install_dir, legacy) else {
                return Ok(None);
            };
            let settings = Settings {
                data_dir: Some(dir.clone()),
            };
            if let Err(err) = save_settings(&settings_file, &settings) {
                app.dialog()
                    .message(format!(
                        "无法保存数据目录的选择到 {}：{err}\n本次仍使用 {}，下次启动会再次询问。",
                        settings_file.display(),
                        dir.display()
                    ))
                    .title(DIALOG_TITLE)
                    .kind(MessageDialogKind::Warning)
                    .parent(window)
                    .blocking_show();
            }
            Ok(Some(dir))
        }
    }
}

fn ask(
    app: &AppHandle,
    window: &WebviewWindow,
    default_dir: &Path,
    install_dir: &Path,
    legacy_root: Option<&Path>,
) -> Option<PathBuf> {
    let mut message = format!(
        "biliup 安装在系统盘上，请选择数据目录。数据库、登录信息、日志和录像默认都保存在这里。\n\n\
         默认目录：{}\n安装目录：{}",
        default_dir.display(),
        install_dir.display()
    );
    match legacy_root {
        Some(root) if root == install_dir => message.push_str(
            "\n\n安装目录下有旧版数据（data）：选安装目录会直接使用，选其他目录会复制过去，原处保留。",
        ),
        Some(root) => message.push_str(&format!(
            "\n\n发现旧版数据 {}，会复制到所选目录，原处保留。",
            root.join("data").display()
        )),
        None => {}
    }
    message.push_str("\n\n选择会被记住，之后不再询问。");

    loop {
        let choice = app
            .dialog()
            .message(&message)
            .title(DIALOG_TITLE)
            .kind(MessageDialogKind::Info)
            .parent(window)
            // The third button is what closing the dialog maps to on Linux,
            // so it must not commit to a directory.
            .buttons(MessageDialogButtons::YesNoCancelCustom(
                USE_DEFAULT.into(),
                USE_INSTALL_DIR.into(),
                PICK_OTHER.into(),
            ))
            .blocking_show_with_result();
        let dir = match choice {
            MessageDialogResult::Custom(label) if label == USE_DEFAULT => default_dir.to_path_buf(),
            MessageDialogResult::Custom(label) if label == USE_INSTALL_DIR => {
                install_dir.to_path_buf()
            }
            MessageDialogResult::Custom(_) => {
                let picked = app
                    .dialog()
                    .file()
                    .set_title(DIALOG_TITLE)
                    .set_directory(default_dir.parent().unwrap_or(default_dir))
                    .set_parent(window)
                    .blocking_pick_folder();
                match picked.and_then(|path| path.into_path().ok()) {
                    Some(dir) => dir,
                    None => continue,
                }
            }
            _ => return None,
        };
        match ensure_writable(&dir) {
            Ok(()) => return Some(dir),
            Err(err) => {
                app.dialog()
                    .message(format!("无法使用 {}：{err}\n请换一个目录。", dir.display()))
                    .title(DIALOG_TITLE)
                    .kind(MessageDialogKind::Error)
                    .parent(window)
                    .blocking_show();
            }
        }
    }
}

fn load_settings(path: &Path) -> Settings {
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn save_settings(path: &Path, settings: &Settings) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(settings)?)
}

fn ensure_writable(dir: &Path) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let probe = dir.join(".biliup-write-test");
    fs::write(&probe, b"")?;
    fs::remove_file(probe)
}

/// `system_drive` is `%SystemDrive%` (`C:` when unset).
fn on_system_drive(dir: &Path, system_drive: Option<&str>) -> bool {
    let system = system_drive.and_then(drive_letter).unwrap_or('C');
    dir.to_str().and_then(drive_letter) == Some(system)
}

fn drive_letter(path: &str) -> Option<char> {
    let path = path.strip_prefix(r"\\?\").unwrap_or(path);
    let mut chars = path.chars();
    match (chars.next(), chars.next()) {
        (Some(letter), Some(':')) if letter.is_ascii_alphabetic() => {
            Some(letter.to_ascii_uppercase())
        }
        _ => None,
    }
}

/// Directories an older desktop build may have left `data/` in, most relevant
/// first: the current install directory, then where the old `bbup-app`
/// installers put the app (a different directory now that the product name
/// changed).
pub fn legacy_roots(install_dir: &Path) -> Vec<PathBuf> {
    std::iter::once(install_dir.to_path_buf())
        .chain(bbup_app_install_dirs())
        .collect()
}

pub fn find_legacy_root(roots: &[PathBuf]) -> Option<&Path> {
    roots
        .iter()
        .find(|root| root.join("data").is_dir())
        .map(PathBuf::as_path)
}

#[cfg(windows)]
fn bbup_app_install_dirs() -> Vec<PathBuf> {
    use windows_registry::{CURRENT_USER, LOCAL_MACHINE};
    // Written by the old NSIS (default value) and MSI (`InstallDir`)
    // installers; the manufacturer came from the old identifier
    // `com.tauri-app.app`.
    const KEY: &str = r"Software\tauri-app\bbup-app";
    let mut dirs: Vec<PathBuf> = [
        (CURRENT_USER, ""),
        (CURRENT_USER, "InstallDir"),
        (LOCAL_MACHINE, ""),
    ]
    .into_iter()
    .filter_map(|(root, name)| root.open(KEY).and_then(|key| key.get_string(name)).ok())
    .filter(|dir| !dir.is_empty())
    .map(|dir| PathBuf::from(dir.trim_matches('"')))
    .collect();
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        dirs.push(PathBuf::from(local).join("bbup-app"));
    }
    dirs
}

#[cfg(not(windows))]
fn bbup_app_install_dirs() -> Vec<PathBuf> {
    Vec::new()
}

/// Copies `legacy_root/data` to `data_root/data` once, when the former exists
/// and the latter does not. The original is left in place. Returns whether a
/// copy was made.
pub fn migrate_legacy_data(legacy_root: &Path, data_root: &Path) -> io::Result<bool> {
    let from = legacy_root.join("data");
    let to = data_root.join("data");
    if !from.is_dir() || to.exists() || same_dir(&from, &to) {
        return Ok(false);
    }
    // Copy to a temporary name first so an interrupted copy is retried on the
    // next start instead of being mistaken for migrated data.
    let partial = data_root.join("data.migrating");
    if partial.exists() {
        fs::remove_dir_all(&partial)?;
    }
    copy_dir(&from, &partial)
        .and_then(|()| fs::rename(&partial, &to))
        .inspect_err(|_| {
            let _ = fs::remove_dir_all(&partial);
        })?;
    Ok(true)
}

fn same_dir(a: &Path, b: &Path) -> bool {
    matches!((a.canonicalize(), b.canonicalize()), (Ok(a), Ok(b)) if a == b)
}

fn copy_dir(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_off_the_system_drive_keeps_data_in_the_install_dir() {
        let install = Path::new(r"D:\biliup");
        let saved = Some(PathBuf::from(r"E:\elsewhere"));
        assert_eq!(
            plan(true, false, install, saved, Path::new("appdata")),
            Plan::Use(install.to_path_buf())
        );
    }

    #[test]
    fn windows_on_the_system_drive_asks_once() {
        let install = Path::new(r"C:\biliup");
        assert_eq!(
            plan(true, true, install, None, Path::new("appdata")),
            Plan::Ask
        );
        assert_eq!(
            plan(
                true,
                true,
                install,
                Some(PathBuf::from(r"C:\data")),
                Path::new("appdata")
            ),
            Plan::Use(PathBuf::from(r"C:\data"))
        );
    }

    #[test]
    fn other_platforms_use_the_app_data_dir_unless_saved() {
        let install = Path::new("/usr/bin");
        assert_eq!(
            plan(
                false,
                false,
                install,
                None,
                Path::new("/home/u/.local/share/x")
            ),
            Plan::Use(PathBuf::from("/home/u/.local/share/x"))
        );
        assert_eq!(
            plan(
                false,
                false,
                install,
                Some(PathBuf::from("/srv/biliup")),
                Path::new("x")
            ),
            Plan::Use(PathBuf::from("/srv/biliup"))
        );
    }

    #[test]
    fn system_drive_comes_from_the_environment() {
        assert!(on_system_drive(Path::new(r"C:\Program Files\biliup"), None));
        assert!(on_system_drive(Path::new(r"c:\biliup"), Some("C:")));
        assert!(on_system_drive(Path::new(r"\\?\D:\biliup"), Some("D:")));
        assert!(!on_system_drive(Path::new(r"C:\biliup"), Some("D:")));
        assert!(!on_system_drive(Path::new(r"D:\biliup"), None));
        assert!(!on_system_drive(Path::new(r"\\server\share\biliup"), None));
        assert!(!on_system_drive(Path::new("/usr/bin"), None));
    }

    #[test]
    fn settings_round_trip() {
        let dir = tempfile_dir("settings");
        let file = dir.join("nested").join(SETTINGS_FILE);
        assert!(load_settings(&file).data_dir.is_none());
        let settings = Settings {
            data_dir: Some(dir.join("data root")),
        };
        save_settings(&file, &settings).unwrap();
        assert_eq!(load_settings(&file).data_dir, Some(dir.join("data root")));
        fs::write(&file, b"not json").unwrap();
        assert!(load_settings(&file).data_dir.is_none());
    }

    #[test]
    fn finds_the_first_root_with_data() {
        let empty = tempfile_dir("roots-empty");
        let old = tempfile_dir("roots-old");
        fs::create_dir_all(old.join("data")).unwrap();
        let roots = vec![empty.clone(), old.clone()];
        assert_eq!(find_legacy_root(&roots), Some(old.as_path()));
        assert_eq!(find_legacy_root(&roots[..1]), None);
    }

    #[test]
    fn migrates_legacy_data_once_and_keeps_the_original() {
        let legacy = tempfile_dir("legacy");
        let data = tempfile_dir("appdata");
        fs::create_dir_all(legacy.join("data/nested")).unwrap();
        fs::write(legacy.join("data/data.sqlite3"), b"db").unwrap();
        fs::write(legacy.join("data/nested/42.json"), b"{}").unwrap();

        assert!(migrate_legacy_data(&legacy, &data).unwrap());
        assert_eq!(fs::read(data.join("data/data.sqlite3")).unwrap(), b"db");
        assert_eq!(fs::read(data.join("data/nested/42.json")).unwrap(), b"{}");
        assert!(legacy.join("data/data.sqlite3").exists());
        assert!(!data.join("data.migrating").exists());

        fs::write(legacy.join("data/data.sqlite3"), b"newer").unwrap();
        assert!(!migrate_legacy_data(&legacy, &data).unwrap());
        assert_eq!(fs::read(data.join("data/data.sqlite3")).unwrap(), b"db");
    }

    #[test]
    fn skips_migration_without_legacy_data_or_into_itself() {
        let legacy = tempfile_dir("empty-legacy");
        let data = tempfile_dir("empty-appdata");
        assert!(!migrate_legacy_data(&legacy, &data).unwrap());
        assert!(!data.join("data").exists());

        fs::create_dir_all(legacy.join("data")).unwrap();
        assert!(!migrate_legacy_data(&legacy, &legacy).unwrap());
    }

    fn tempfile_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("biliup-desktop-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
