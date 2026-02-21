//! File locking for memory system.

use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::Error;

/// Lock timeout in milliseconds.
const LOCK_TIMEOUT_MS: u64 = 5000;

/// Acquire an exclusive lock on a file.
pub fn acquire_lock(path: &Path) -> Result<LockHandle, Error> {
    let lock_path_str = format!("{}.lock", path.display());
    let lock_path = Path::new(&lock_path_str);

    // Check if lock exists and is not stale
    if lock_path.exists() {
        let lock_age = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
            - lock_path
                .metadata()?
                .modified()?
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;

        if lock_age < LOCK_TIMEOUT_MS {
            return Err(Error::Memory(format!(
                "Lock file is held: {}",
                lock_path.display()
            )));
        }

        // Stale lock, remove it
        tracing::warn!("Removing stale lock: {}", lock_path.display());
        std::fs::remove_file(&lock_path).ok();
    }

    // Create lock file
    let mut lock_file = File::create(&lock_path)?;
    lock_file.write_all(format!("{}\n", std::process::id()).as_bytes())?;
    lock_file.sync_all()?;

    tracing::debug!("Acquired lock: {}", lock_path.display());

    Ok(LockHandle {
        lock_path: lock_path.to_path_buf(),
    })
}

/// Lock handle - releases lock when dropped.
pub struct LockHandle {
    lock_path: std::path::PathBuf,
}

impl Drop for LockHandle {
    fn drop(&mut self) {
        if let Err(e) = std::fs::remove_file(&self.lock_path) {
            tracing::warn!("Failed to release lock {}: {}", self.lock_path.display(), e);
        } else {
            tracing::debug!("Released lock: {}", self.lock_path.display());
        }
    }
}

/// Acquire lock, execute function, release lock.
pub fn with_lock<T, F>(path: &Path, f: F) -> Result<T, Error>
where
    F: FnOnce() -> Result<T, Error>,
{
    let _lock = acquire_lock(path)?;
    f()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_lock() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.json");

        fs::write(&test_file, "{}").unwrap();

        let lock1 = acquire_lock(&test_file);
        assert!(lock1.is_ok());

        // Try to acquire again should fail
        let lock2 = acquire_lock(&test_file);
        assert!(lock2.is_err());

        // Drop first lock
        drop(lock1);

        // Now should work again
        let lock3 = acquire_lock(&test_file);
        assert!(lock3.is_ok());
    }
}
