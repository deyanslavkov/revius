use crate::error::ReviusError;
use std::fs;
use std::fs::OpenOptions;
use std::io::{Write, Read};
use std::path::{Path, PathBuf};
use std::process;
use time::OffsetDateTime;

/// Simple repo lock that writes pid + timestamp, and checks liveness on Unix.
/// Removes lock on Drop.
pub struct RepoLock {
    path: PathBuf,
}

impl RepoLock {
    /// Acquire the lock at path. If lock exists and is active -> error.
    /// If lock exists and is stale -> remove and create new lock.
    pub fn acquire<P: AsRef<Path>>(path: P) -> Result<Self, ReviusError> {
        let lock_path = path.as_ref().to_path_buf();

        // Ensure parent exists
        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent)?;
        }

        if lock_path.exists() {
            if Self::is_stale(&lock_path)? {
                // attempt removal
                fs::remove_file(&lock_path)?;
            } else {
                // read pid for user-friendly message
                let mut content = String::new();
                if let Ok(mut f) = fs::File::open(&lock_path) {
                    let _ = f.read_to_string(&mut content);
                }
                return Err(ReviusError::Lock(format!("Lock exists: {} (content: {})", lock_path.display(), content)));
            }
        }

        let pid = process::id();
        let ts = OffsetDateTime::now_utc().unix_timestamp();

        // create new lock file atomically
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
            .map_err(|e| ReviusError::Lock(format!("failed to create lock file: {}", e)))?;

        writeln!(file, "pid={}", pid)?;
        writeln!(file, "ts={}", ts)?;

        Ok(Self { path: lock_path })
    }

    /// Force release lock file (utility)
    pub fn force_release<P: AsRef<Path>>(path: P) -> Result<(), ReviusError> {
        let p = path.as_ref();
        if p.exists() {
            fs::remove_file(p)?;
        }
        Ok(())
    }

    /// Stale detection: on Unix check process exists; on non-unix fallback to timestamp age.
    fn is_stale(path: &Path) -> Result<bool, ReviusError> {
        let content = fs::read_to_string(path)?;
        let mut pid_opt: Option<u32> = None;
        let mut ts_opt: Option<i64> = None;

        for line in content.lines() {
            if let Some(rest) = line.strip_prefix("pid=") {
                if let Ok(val) = rest.trim().parse::<u32>() {
                    pid_opt = Some(val);
                }
            } else if let Some(rest) = line.strip_prefix("ts=") {
                if let Ok(val) = rest.trim().parse::<i64>() {
                    ts_opt = Some(val);
                }
            }
        }

        // If no pid found, consider stale
        let _pid = match pid_opt {
            Some(p) => p,
            None => return Ok(true),
        };

        #[cfg(unix)]
        {
            // use libc::kill(pid, 0)
            unsafe {
                let pid_t = _pid as libc::pid_t;
                let res = libc::kill(pid_t, 0);
                if res == 0 {
                    return Ok(false); // process exists
                } else {
                    let errno = *libc::__errno_location();
                    if errno == libc::ESRCH {
                        return Ok(true); // process doesn't exist
                    } else {
                        // EPERM or other: assume alive
                        return Ok(false);
                    }
                }
            }
        }

        // non-unix: fallback to timestamp-based policy
        #[cfg(not(unix))]
        {
            if let Some(ts) = ts_opt {
                let now = OffsetDateTime::now_utc().unix_timestamp();
                // stale if older than 24 hours
                return Ok(now - ts > 24 * 3600);
            }
            // no timestamp: be conservative
            Ok(false)
        }
    }
}

impl Drop for RepoLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
