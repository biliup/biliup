//! A checked complete-file commit with an exclusive, same-directory temp name.
//! It guarantees no overwrite; it does not promise directory power durability
//! or one strictly atomic publish/temporary-unlink operation on every OS.
use crate::{ConvertError, ErrorKind};
use std::io::{self, BufWriter, Write};
use std::path::Path;
use tempfile::NamedTempFile;

pub(crate) const TEMP_PREFIX: &str = ".biliup-danmaku-";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Create,
    Write,
    Flush,
    RecoverBuffer,
    Sync,
    Publish,
    Cleanup,
}

fn error(kind: ErrorKind, stage: &str, path: &Path, source: io::Error) -> ConvertError {
    let mut os_error = source.raw_os_error();
    let mut cause = std::error::Error::source(&source);
    while os_error.is_none() {
        let Some(current) = cause else { break };
        os_error = current
            .downcast_ref::<io::Error>()
            .and_then(io::Error::raw_os_error);
        cause = current.source();
    }
    ConvertError {
        kind,
        upstream_code: 0,
        os_error: if cfg!(unix) { os_error } else { None },
        detail: format!("{stage} {}: {source}", path.display()),
    }
}

fn cleanup(
    temp: NamedTempFile,
    mut primary: ConvertError,
    hook: &mut impl FnMut(Stage) -> io::Result<()>,
) -> ConvertError {
    let path = temp.path().to_owned();
    let result = match hook(Stage::Cleanup) {
        Ok(()) => temp.close(),
        Err(failure) => {
            // Only a test-injected cleanup failure reaches this branch. Keep
            // the actual residue so the test proves reporting and recovery.
            if let Err(keep) = temp.keep() {
                primary
                    .detail
                    .push_str(&format!("; retain residue: {}", keep.error));
            }
            Err(failure)
        }
    };
    if let Err(failure) = result {
        primary
            .detail
            .push_str(&format!("; cleanup {}: {failure}", path.display()));
    }
    primary
}

pub(crate) fn publish(
    path: &Path,
    write: impl FnOnce(&mut BufWriter<&mut std::fs::File>) -> io::Result<()>,
) -> Result<(), ConvertError> {
    publish_with(path, write, |_| Ok(()))
}

fn publish_with(
    path: &Path,
    write: impl FnOnce(&mut BufWriter<&mut std::fs::File>) -> io::Result<()>,
    mut hook: impl FnMut(Stage) -> io::Result<()>,
) -> Result<(), ConvertError> {
    if std::fs::symlink_metadata(path).is_ok() {
        return Err(ConvertError::new(
            ErrorKind::OutputExists,
            path.display().to_string(),
        ));
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    // Rust's file APIs handle long Windows paths, but tempfile's publication
    // calls Win32 directly. Canonicalize the existing directory so both its
    // temporary name and the not-yet-existing destination use verbatim paths.
    #[cfg(windows)]
    let parent = parent
        .canonicalize()
        .map_err(|e| error(ErrorKind::OutputCreate, "resolve parent", path, e))?;
    #[cfg(windows)]
    let destination = parent.join(path.file_name().ok_or_else(|| {
        error(
            ErrorKind::OutputCreate,
            "resolve filename",
            path,
            io::Error::new(io::ErrorKind::InvalidInput, "output has no filename"),
        )
    })?);
    #[cfg(windows)]
    let path = destination.as_path();
    hook(Stage::Create).map_err(|e| error(ErrorKind::OutputCreate, "create temporary", path, e))?;
    // tempfile 3.27 wraps creation errors in a private PathError which drops
    // raw_os_error from its public source chain. Capture the real create_new
    // error at the operation, preserving the existing public errno contract.
    let mut create_os_error = None;
    let mut temp = tempfile::Builder::new()
        .prefix(TEMP_PREFIX)
        .make_in(parent, |name| {
            let mut options = std::fs::OpenOptions::new();
            options.read(true).write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            options.open(name).inspect_err(|failure| {
                create_os_error = failure.raw_os_error();
            })
        })
        .map_err(|e| {
            let mut failure = error(ErrorKind::OutputCreate, "create temporary", path, e);
            if cfg!(unix) {
                failure.os_error = create_os_error;
            }
            failure
        })?;
    let written = (|| {
        let mut writer = BufWriter::new(temp.as_file_mut());
        hook(Stage::Write)
            .and_then(|()| write(&mut writer))
            .map_err(|e| error(ErrorKind::OutputWrite, "write", path, e))?;
        hook(Stage::Flush)
            .and_then(|()| writer.flush())
            .map_err(|e| error(ErrorKind::OutputWrite, "flush", path, e))?;
        hook(Stage::RecoverBuffer)
            .map_err(|e| error(ErrorKind::OutputWrite, "recover buffer", path, e))?;
        let file = writer.into_inner().map_err(|e| {
            error(
                ErrorKind::OutputWrite,
                "recover buffer",
                path,
                e.into_error(),
            )
        })?;
        hook(Stage::Sync)
            .and_then(|()| file.sync_all())
            .map_err(|e| error(ErrorKind::OutputWrite, "sync", path, e))?;
        Ok(())
    })();
    if let Err(primary) = written {
        return Err(cleanup(temp, primary, &mut hook));
    }
    if let Err(failure) = hook(Stage::Publish) {
        return Err(cleanup(
            temp,
            error(ErrorKind::OutputCreate, "publish", path, failure),
            &mut hook,
        ));
    }
    match temp.persist_noclobber(path) {
        Ok(file) => {
            drop(file);
            Ok(())
        }
        Err(failure) => {
            let kind = if failure.error.kind() == io::ErrorKind::AlreadyExists
                || std::fs::symlink_metadata(path).is_ok()
            {
                ErrorKind::OutputExists
            } else {
                ErrorKind::OutputCreate
            };
            Err(cleanup(
                failure.file,
                error(kind, "publish", path, failure.error),
                &mut hook,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_failure_stage_preserves_primary_error_and_removes_only_own_temp() {
        for stage in [
            Stage::Create,
            Stage::Write,
            Stage::Flush,
            Stage::RecoverBuffer,
            Stage::Sync,
            Stage::Publish,
        ] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("out.ass");
            let failure = publish_with(
                &path,
                |w| w.write_all(b"complete"),
                |at| {
                    if at == stage {
                        Err(io::Error::other(format!("injected {stage:?}")))
                    } else {
                        Ok(())
                    }
                },
            )
            .unwrap_err();
            assert_eq!(
                failure.kind,
                if matches!(stage, Stage::Create | Stage::Publish) {
                    ErrorKind::OutputCreate
                } else {
                    ErrorKind::OutputWrite
                }
            );
            assert!(failure.detail.contains(&format!("injected {stage:?}")));
            assert!(!path.exists());
            assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.ass");
        let failure = publish_with(
            &path,
            |_| Err(io::Error::other("primary write")),
            |at| {
                if at == Stage::Cleanup {
                    Err(io::Error::other("delete denied"))
                } else {
                    Ok(())
                }
            },
        )
        .unwrap_err();
        assert_eq!(failure.kind, ErrorKind::OutputWrite);
        assert!(
            failure.detail.contains("primary write")
                && failure.detail.contains("cleanup")
                && failure.detail.contains("delete denied")
        );
        let residue = std::fs::read_dir(dir.path())
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        // Windows diagnostics name the verbatim directory used for publication,
        // which may also expand the runner's short (8.3) temporary-directory name.
        #[cfg(windows)]
        let residue = residue.canonicalize().unwrap();
        assert!(failure.detail.contains(&residue.display().to_string()));
        std::fs::remove_file(residue).unwrap();
    }

    #[test]
    fn no_clobber_race_has_one_complete_winner_and_no_residue() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.ass");
        let barrier = std::sync::Barrier::new(8);
        let results = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..8)
                .map(|_| {
                    scope.spawn(|| {
                        publish_with(
                            &path,
                            |w| w.write_all(&vec![b'x'; 20000]),
                            |stage| {
                                if stage == Stage::Publish {
                                    barrier.wait();
                                }
                                Ok(())
                            },
                        )
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().unwrap())
                .collect::<Vec<_>>()
        });
        assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
        assert!(
            results
                .iter()
                .filter_map(|r| r.as_ref().err())
                .all(|e| e.kind == ErrorKind::OutputExists)
        );
        assert_eq!(std::fs::read(&path).unwrap(), vec![b'x'; 20000]);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
        std::fs::rename(&path, dir.path().join("renamed.ass")).unwrap();
    }

    #[test]
    fn existing_file_directory_and_symlink_are_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("existing.ass");
        std::fs::write(&file, b"keep").unwrap();
        assert_eq!(
            publish(&file, |_| panic!("must not write"))
                .unwrap_err()
                .kind,
            ErrorKind::OutputExists
        );
        assert_eq!(std::fs::read(&file).unwrap(), b"keep");
        assert_eq!(
            publish(dir.path(), |_| panic!("must not write"))
                .unwrap_err()
                .kind,
            ErrorKind::OutputExists
        );
        #[cfg(unix)]
        {
            let link = dir.path().join("broken-link");
            std::os::unix::fs::symlink("missing", &link).unwrap();
            assert_eq!(
                publish(&link, |_| panic!("must not write"))
                    .unwrap_err()
                    .kind,
                ErrorKind::OutputExists
            );
            assert!(
                std::fs::symlink_metadata(link)
                    .unwrap()
                    .file_type()
                    .is_symlink()
            );
        }
    }

    #[test]
    fn killed_process_never_exposes_partial_final_content() {
        const CHILD: &str = "BILIUP_NATIVE_TRANSACTION_CHILD";
        const PHASE: &str = "BILIUP_NATIVE_TRANSACTION_PHASE";
        if let Some(dir) = std::env::var_os(CHILD) {
            let dir = Path::new(&dir);
            let phase = std::env::var(PHASE).unwrap();
            let pause = || {
                std::fs::write(dir.join("ready"), b"ready").unwrap();
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            };
            publish_with(
                &dir.join("out.ass"),
                |w| w.write_all(&vec![b'x'; 20000]),
                |at| {
                    if (phase == "write" && at == Stage::Flush)
                        || (phase == "publish" && at == Stage::Publish)
                    {
                        pause();
                    }
                    Ok(())
                },
            )
            .unwrap();
            pause();
        }
        for phase in ["write", "publish", "committed"] {
            let dir = tempfile::tempdir().unwrap();
            let mut child=std::process::Command::new(std::env::current_exe().unwrap()).args(["--exact","native::transaction::tests::killed_process_never_exposes_partial_final_content","--test-threads=1"]).env(CHILD,dir.path()).env(PHASE,phase).stdout(std::process::Stdio::null()).spawn().unwrap();
            let started = std::time::Instant::now();
            while !dir.path().join("ready").exists() {
                assert!(
                    started.elapsed() < std::time::Duration::from_secs(10),
                    "child readiness timed out"
                );
                assert!(
                    child.try_wait().unwrap().is_none(),
                    "child exited before readiness"
                );
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            child.kill().unwrap();
            child.wait().unwrap();
            if phase == "committed" {
                assert_eq!(
                    std::fs::read(dir.path().join("out.ass")).unwrap(),
                    vec![b'x'; 20000]
                );
            } else {
                assert!(!dir.path().join("out.ass").exists());
                assert!(std::fs::read_dir(dir.path()).unwrap().any(|e| {
                    e.unwrap()
                        .file_name()
                        .to_string_lossy()
                        .starts_with(TEMP_PREFIX)
                }));
            }
            dir.close().unwrap();
        }
    }
}
