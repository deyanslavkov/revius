use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub fn canonicalize(path: &Path) -> io::Result<PathBuf> {
    let canonical = if path.exists() {
        fs::canonicalize(path)?
    } else {
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()?.join(path)
        };
        absolute
    };

    Ok(clean_path_display(&canonical))
}

pub fn clean_path_display(path: &Path) -> PathBuf {
    let path_str = path.to_string_lossy();
    
    if cfg!(windows) && path_str.starts_with(r"\\?\") {
        PathBuf::from(&path_str[4..])
    } else {
        path.to_path_buf()
    }
}

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

pub fn get_file_modified_time(path: &Path) -> io::Result<i64> {
    let metadata = fs::metadata(path)?;
    let mtime = metadata.modified()?;
    let duration = mtime.duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    Ok(duration.as_secs() as i64)
}

#[cfg(unix)]
pub fn get_file_mode(path: &Path) -> io::Result<u32> {
    use std::os::unix::fs::PermissionsExt;
    let metadata = fs::metadata(path)?;
    let permissions = metadata.permissions();
    Ok(if permissions.mode() & 0o111 != 0 {
        0o100755
    } else {
        0o100644
    })
}

#[cfg(not(unix))]
pub fn get_file_mode(_: &Path) -> io::Result<u32> {
    Ok(0o100644)
}
