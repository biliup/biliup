//! Explicit ownership of a leased conversion root, including its last drop.
//!
//! A conversion writes into a private attempt directory under its managed
//! root, and the root lives in a cache directory shared by every biliup
//! process on the host. Recovery therefore cannot treat a directory as
//! abandoned just because it is old: it takes the root's exclusive lease
//! first, so a root another live process is still converting in survives.
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

/// Held open by every owner of a root. Recovery must take the exclusive lock
/// before reclaiming a root.
pub const ROOT_LEASE_FILE: &str = ".biliup-danmaku-lease";
/// Marks a root that recovery has taken over; new owners must refuse it.
pub(crate) const ROOT_RETIRED_FILE: &str = ".biliup-danmaku-retired";
/// A Windows path has at most 32,767 UTF-16 code units: valid filename
/// characters need at most three UTF-8 bytes each (escaped backslashes need
/// two), leaving room for JSON fields under 128 KiB. Unix's 4096-byte paths
/// need less than 25 KiB even with six-byte JSON control escapes.
pub const ROOT_LEASE_MAX_BYTES: u64 = 128 * 1024;

/// The only global location scanned for abandoned preview/self-check roots.
/// Export roots remain under their managed session directories.
#[doc(hidden)]
pub fn managed_preview_root_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("biliup")
        .join("danmaku-workers")
}

#[doc(hidden)]
pub fn managed_preview_root() -> io::Result<tempfile::TempDir> {
    let base = managed_preview_root_dir();
    std::fs::create_dir_all(&base)?;
    tempfile::Builder::new().prefix(".render-").tempdir_in(base)
}

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RootLease {
    root: PathBuf,
    token: String,
}

fn invalid_lease(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn require_active_root(root: &Path) -> io::Result<()> {
    match std::fs::symlink_metadata(root.join(ROOT_RETIRED_FILE)) {
        Ok(_) => Err(invalid_lease("managed root is retired for recovery")),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn read_lease(file: &File) -> io::Result<RootLease> {
    let mut file = file.try_clone()?;
    file.rewind()?;
    let mut bytes = Vec::new();
    file.take(ROOT_LEASE_MAX_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > ROOT_LEASE_MAX_BYTES {
        return Err(invalid_lease("managed root lease is too large"));
    }
    serde_json::from_slice(&bytes).map_err(|e| invalid_lease(e.to_string()))
}

/// Verifies the locked marker still names this unique, unrenamed root. A late
/// owner must fail this check after recovery renamed the directory.
pub fn validate_root_lease(root: &Path, file: &File) -> io::Result<()> {
    // Recheck after acquiring the shared lock: recovery may have retired the
    // root while a late owner was waiting for its exclusive lease to release.
    require_active_root(root)?;
    let lease = read_lease(file)?;
    if lease.root != std::fs::canonicalize(root)? || lease.token.is_empty() {
        return Err(invalid_lease("managed root lease identity changed"));
    }
    let current = read_lease(&File::open(lease.root.join(ROOT_LEASE_FILE))?)?;
    if current != lease {
        return Err(invalid_lease("managed root lease was replaced"));
    }
    Ok(())
}

/// Acquires one owner's shared lease. Concurrent conversions in the same root
/// share the marker but hold independent OS locks.
pub fn acquire_root_lease(root: &Path) -> io::Result<File> {
    let root = std::fs::canonicalize(root)?;
    require_active_root(&root)?;
    let marker = root.join(ROOT_LEASE_FILE);
    if !marker.try_exists()? {
        let mut temporary = tempfile::Builder::new()
            .prefix(".lease-")
            .tempfile_in(&root)?;
        let lease = RootLease {
            root: root.clone(),
            token: temporary
                .path()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
        };
        serde_json::to_writer(temporary.as_file_mut(), &lease).map_err(io::Error::other)?;
        temporary.flush()?;
        temporary.as_file().sync_all()?;
        if let Err(error) = temporary.persist_noclobber(&marker)
            && error.error.kind() != io::ErrorKind::AlreadyExists
        {
            return Err(error.error);
        }
    }
    if !std::fs::symlink_metadata(&marker)?.is_file() {
        return Err(invalid_lease("managed root lease is not a regular file"));
    }
    let file = OpenOptions::new().read(true).write(true).open(&marker)?;
    file.lock_shared()?;
    validate_root_lease(&root, &file)?;
    Ok(file)
}

#[derive(Debug)]
pub struct ManagedRoot {
    path: PathBuf,
    lease: Option<File>,
    cleanup_failure: Mutex<Option<String>>,
    closed: bool,
}

impl ManagedRoot {
    pub fn new(directory: tempfile::TempDir) -> io::Result<Self> {
        let lease = acquire_root_lease(directory.path())?;
        let path = directory.keep();
        Ok(Self {
            path,
            lease: Some(lease),
            cleanup_failure: Mutex::new(None),
            closed: false,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub(super) fn record_cleanup_failure(&self, error: &str) {
        *self
            .cleanup_failure
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(error.to_owned());
    }

    fn cleanup(&mut self) -> io::Result<()> {
        self.lease.take();
        if let Some(error) = self
            .cleanup_failure
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
        {
            return Err(io::Error::other(format!(
                "retained failed attempt {}: {error}",
                self.path.display()
            )));
        }
        match super::jobs::cleanup_inactive_danmaku_root(&self.path)? {
            true => {
                self.closed = true;
                Ok(())
            }
            false => Err(io::Error::other(format!(
                "root still active or ownership uncertain: {}",
                self.path.display()
            ))),
        }
    }

    /// At normal acceptance boundaries the final owner checks cleanup errors.
    /// A conversion stopped by cancellation or timeout may still own its root
    /// from a detached thread; its final drop retains and records failures
    /// through the same guarded recovery path.
    pub(super) fn finish<T>(root: Arc<Self>, result: Result<T, String>) -> Result<T, String> {
        match Arc::try_unwrap(root) {
            Ok(mut root) => {
                let cleanup = root.cleanup();
                // This explicit attempt already reports a failure to its caller;
                // don't immediately retry in Drop and hide the residual state.
                root.closed = true;
                match cleanup {
                    Ok(()) => result,
                    Err(error) => {
                        tracing::warn!(path=%root.path.display(),%error,"清理弹幕工作根目录失败，恢复时重试");
                        Err(match result {
                            Ok(_) => format!("cleanup {}: {error}", root.path.display()),
                            Err(primary) => {
                                format!("{primary}; cleanup {}: {error}", root.path.display())
                            }
                        })
                    }
                }
            }
            Err(root) => {
                if let Some(error) = root
                    .cleanup_failure
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .clone()
                {
                    return Err(match result {
                        Ok(_) => error,
                        Err(primary) => format!("{primary}; {error}"),
                    });
                }
                result
            }
        }
    }
}

impl Drop for ManagedRoot {
    fn drop(&mut self) {
        if !self.closed
            && let Err(error) = self.cleanup()
        {
            tracing::warn!(path=%self.path.display(),%error,"弹幕工作根目录残留，恢复时重试");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_owner_checks_cleanup_and_retains_a_failed_attempt_for_recovery() {
        let owner = tempfile::tempdir().unwrap();
        let root = ManagedRoot::new(
            tempfile::Builder::new()
                .prefix(".render-")
                .tempdir_in(owner.path())
                .unwrap(),
        )
        .unwrap();
        let path = root.path().to_owned();
        let root = Arc::new(root);
        root.record_cleanup_failure("cleanup private attempt: delete denied");
        let failure = ManagedRoot::finish(root, Ok(())).unwrap_err();
        assert!(failure.contains("delete denied") && failure.contains(&path.display().to_string()));
        assert!(path.join(ROOT_LEASE_FILE).exists());
        assert!(super::super::jobs::cleanup_inactive_danmaku_root(&path).unwrap());
        assert!(!path.exists());
    }

    #[test]
    fn final_drop_uses_checked_managed_recovery_after_shared_owners_leave() {
        let root = Arc::new(ManagedRoot::new(tempfile::tempdir().unwrap()).unwrap());
        let path = root.path().to_owned();
        let shared = root.clone();
        drop(root);
        assert!(path.exists());
        drop(shared);
        assert!(!path.exists());
    }

    #[test]
    fn a_retired_root_cannot_be_acquired_again() {
        let root = tempfile::tempdir().unwrap();
        drop(acquire_root_lease(root.path()).unwrap());
        std::fs::File::create_new(root.path().join(ROOT_RETIRED_FILE))
            .unwrap()
            .sync_all()
            .unwrap();
        assert!(acquire_root_lease(root.path()).is_err());
    }
}
