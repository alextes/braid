//! global lock operations for mutation safety.

use fs2::FileExt;
use std::fs::{File, OpenOptions};
use std::path::Path;

use crate::error::Result;

/// a guard that holds an exclusive lock on the brd lock file.
/// the lock is released when this guard is dropped.
pub struct LockGuard {
    _file: File,
}

impl LockGuard {
    /// acquire an exclusive lock on the lock file.
    /// creates the lock file if it doesn't exist.
    pub fn acquire(lock_path: &Path) -> Result<Self> {
        // ensure parent directory exists
        if let Some(parent) = lock_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)?;

        file.lock_exclusive()?;

        Ok(Self { _file: file })
    }

    /// try to acquire an exclusive lock without blocking.
    /// returns None if the lock is held by another process.
    pub fn try_acquire(lock_path: &Path) -> Result<Option<Self>> {
        if let Some(parent) = lock_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)?;

        match file.try_lock_exclusive() {
            Ok(()) => Ok(Some(Self { _file: file })),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

// lock is released automatically when File is dropped (via fs2)

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::thread::sleep;
    use std::time::{Duration, Instant};

    use tempfile::tempdir;

    const READY_FILENAME: &str = "lock-ready";
    const RELEASE_FILENAME: &str = "lock-release";

    fn wait_for_file(path: &Path, timeout: Duration) -> bool {
        let start = Instant::now();
        while start.elapsed() < timeout {
            if path.exists() {
                return true;
            }
            sleep(Duration::from_millis(10));
        }
        false
    }

    #[test]
    fn test_lock_acquire_creates_file() {
        let dir = tempdir().unwrap();
        let lock_path = dir.path().join("brd.lock");
        assert!(!lock_path.exists());

        {
            let _guard = LockGuard::acquire(&lock_path).unwrap();
            assert!(lock_path.exists());
        }

        assert!(lock_path.exists());
    }

    #[test]
    fn test_try_acquire_success_and_release() {
        let dir = tempdir().unwrap();
        let lock_path = dir.path().join("brd.lock");

        let guard = LockGuard::try_acquire(&lock_path).unwrap();
        assert!(guard.is_some());
        drop(guard);

        let guard = LockGuard::try_acquire(&lock_path).unwrap();
        assert!(guard.is_some());
    }

    #[test]
    fn test_try_acquire_when_locked_returns_none() {
        let dir = tempdir().unwrap();
        let lock_path = dir.path().join("brd.lock");
        let ready_path = dir.path().join(READY_FILENAME);
        let release_path = dir.path().join(RELEASE_FILENAME);

        let exe = std::env::current_exe().unwrap();
        let mut child = Command::new(exe)
            .arg("--exact")
            .arg("lock::tests::lock_helper")
            .env(
                "BRD_LOCK_HELPER_LOCK",
                lock_path.to_string_lossy().to_string(),
            )
            .env(
                "BRD_LOCK_HELPER_READY",
                ready_path.to_string_lossy().to_string(),
            )
            .env(
                "BRD_LOCK_HELPER_RELEASE",
                release_path.to_string_lossy().to_string(),
            )
            .env("BRD_LOCK_HELPER", "1")
            .spawn()
            .unwrap();

        assert!(
            wait_for_file(&ready_path, Duration::from_secs(5)),
            "timed out waiting for helper to lock"
        );

        let guard = LockGuard::try_acquire(&lock_path).unwrap();
        assert!(guard.is_none());

        std::fs::write(&release_path, "release").unwrap();
        let status = child.wait().unwrap();
        assert!(status.success());

        let guard = LockGuard::try_acquire(&lock_path).unwrap();
        assert!(guard.is_some());
    }

    #[test]
    fn lock_helper() {
        if std::env::var("BRD_LOCK_HELPER").as_deref() != Ok("1") {
            return;
        }

        let lock_path = std::env::var("BRD_LOCK_HELPER_LOCK").expect("missing lock path");
        let ready_path =
            PathBuf::from(std::env::var("BRD_LOCK_HELPER_READY").expect("missing ready path"));
        let release_path =
            PathBuf::from(std::env::var("BRD_LOCK_HELPER_RELEASE").expect("missing release path"));

        let _guard = LockGuard::acquire(Path::new(&lock_path)).unwrap();
        std::fs::write(&ready_path, "ready").unwrap();

        assert!(
            wait_for_file(&release_path, Duration::from_secs(5)),
            "timed out waiting for release"
        );
    }
}
