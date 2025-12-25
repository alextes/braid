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
