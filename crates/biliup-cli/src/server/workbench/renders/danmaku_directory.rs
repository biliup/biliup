//! Explicit ownership of a leased conversion root, including its last drop.
use crate::danmaku_worker;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

#[derive(Debug)]
pub struct ManagedRoot {
    path: PathBuf,
    lease: Option<File>,
    cleanup_failure: Mutex<Option<String>>,
    closed: bool,
}

impl ManagedRoot {
    pub fn new(directory: tempfile::TempDir) -> io::Result<Self> {
        let lease = danmaku_worker::acquire_root_lease(directory.path())?;
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
    /// A detached supervisor may still own a cancelled root; its final drop
    /// retains and records failures through the same guarded recovery path.
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
        assert!(path.join(danmaku_worker::ROOT_LEASE_FILE).exists());
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
}
