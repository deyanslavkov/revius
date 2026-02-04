use crate::error::ReviusError;
use std::fs;
use std::path::{Path, PathBuf};

/// A RAII guard for the repository lockfile.
/// When this struct is dropped (goes out of scope), the lockfile is removed.
pub struct LockFile {
    path: PathBuf,
}

impl LockFile {
    pub fn acquire(repo_root: &Path) -> Result<Self, ReviusError> {
        let lock_path = repo_root.join(".rvs").join("lock");

        if lock_path.exists() {
            return Err(ReviusError::Usage(format!(
                "Repository is locked. Another process is running.\nIf not, try removing the lock file manually: '{}'",
                lock_path.display()
            )));
        }

        // Create the lock file (optionally write PID, but empty is fine for now)
        fs::write(&lock_path, "")
            .map_err(|e| ReviusError::Io(lock_path.clone(), e))?;

        Ok(Self { path: lock_path })
    }
}

impl Drop for LockFile {
    fn drop(&mut self) {
        // Best-effort removal. If it fails, not much we can do in Drop,
        // but typically it succeeds.
        let _ = fs::remove_file(&self.path);
    }
}