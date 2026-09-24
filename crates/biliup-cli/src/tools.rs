//! External programs: where ffmpeg is, and how child processes are started.
//!
//! ffmpeg is looked up in this order: the `ffmpeg_path` setting, then the one
//! the embedding host ships (the desktop app passes the ffmpeg bundled in its
//! installer through `ServeOptions::ffmpeg`), then `ffmpeg` on `PATH`.

use std::path::{Path, PathBuf};
use std::sync::{PoisonError, RwLock};

pub use biliup::tools::{command, std_command};

const FFMPEG: &str = "ffmpeg";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}

static LOCATIONS: RwLock<Locations> = RwLock::new(Locations {
    configured: None,
    bundled: None,
});

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

fn resolve(configured: Option<&Path>, bundled: Option<&Path>) -> (PathBuf, FfmpegSource) {
    if let Some(path) = configured {
        (path.to_path_buf(), FfmpegSource::Config)
    } else if let Some(path) = bundled {
        (path.to_path_buf(), FfmpegSource::Bundled)
    } else {
        (PathBuf::from(FFMPEG), FfmpegSource::Path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
