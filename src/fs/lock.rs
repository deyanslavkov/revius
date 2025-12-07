use std::fs;
use std::io;
use std::path::Path;

pub fn init_lockfile(path: &Path) -> io::Result<()> {
    fs::write(path, "")
}