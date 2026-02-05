use crate::core::models::objects;
use std::fs;
use std::io;
use std::path::Path;

pub fn create_dir(path: &Path) -> io::Result<()> {
    fs::create_dir_all(path)
}

pub fn write_file(path: &Path, content: &str) -> io::Result<()> {
    fs::write(path, content)
}

pub fn write_binary(path: &Path, content: &[u8]) -> io::Result<()> {
    fs::write(path, content)
}

pub fn read_file(path: &Path) -> io::Result<Vec<u8>> {
    fs::read(path)
}

pub fn delete_file(path: &Path) -> io::Result<()> {
    fs::remove_file(path)
}

pub fn get_file_modified_time(path: &Path) -> io::Result<i64> {
    let metadata = fs::metadata(path)?;
    let mtime = metadata.modified()?;
    let duration = mtime.duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    Ok(duration.as_secs() as i64)
}

/// Create directory and all parent directories
pub fn create_dir_all(path: &Path) -> io::Result<()> {
    fs::create_dir_all(path)
}

#[cfg(unix)]
pub fn get_file_mode(path: &Path) -> io::Result<u32> {
    use std::os::unix::fs::PermissionsExt;
    let metadata = fs::metadata(path)?;
    let permissions = metadata.permissions();
    // 0o111 checks for execution bit on user, group, or other
    Ok(if permissions.mode() & 0o111 != 0 {
        use crate::core::models::objects::MODE_EXEC;
        objects::MODE_EXEC
    } else {
        objects::MODE_FILE
    })
}

#[cfg(not(unix))]
pub fn get_file_mode(_: &Path) -> io::Result<u32> {
    Ok(objects::MODE_FILE)
}

#[cfg(unix)]
pub fn set_file_mode(path: &Path, mode: u32) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let permissions = fs::Permissions::from_mode(mode);
    fs::set_permissions(path, permissions)
}

#[cfg(not(unix))]
pub fn set_file_mode(_path: &Path, _mode: u32) -> io::Result<()> {
    Ok(())
}

/// Set file as executable (Unix only, no-op on Windows)
pub fn set_executable(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // 0o755 is standard rwxr-xr-x
        let permissions = fs::Permissions::from_mode(0o755);
        fs::set_permissions(path, permissions)?;
    }
    
    #[cfg(not(unix))]
    {
        let _ = path; // Suppress unused warning on Windows
    }
    
    Ok(())
}