use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub fn canonicalize(path: &Path) -> io::Result<PathBuf> {
    if path.exists() {
        fs::canonicalize(path)
    } else {
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()?.join(path)
        };
        Ok(absolute)
    }
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