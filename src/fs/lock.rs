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

        // Use OpenOptions to atomically create the file.
        // create_new(true) fails if the file already exists.
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    ReviusError::Usage(format!(
                        "Repository is locked. Another process is running.\nIf not, try removing the lock file manually: '{}'",
                        lock_path.display()
                    ))
                } else {
                    ReviusError::Io(lock_path.clone(), e)
                }
            })?;

        Ok(Self { path: lock_path })
    }
}

impl Drop for LockFile {
    fn drop(&mut self) {
        // Best-effort removal.
        let _ = fs::remove_file(&self.path);
    }
}