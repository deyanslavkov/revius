use crate::cli::args::StatusArgs;
use crate::cli::ui;
use crate::core;
use crate::error::ReviusError;
use crate::fs;

pub fn run(_args: StatusArgs) -> Result<(), ReviusError> {
    let current_dir = fs::paths::get_current_dir()?;
    let repo = core::open::open_repository(&current_dir)?;

    let status_info = core::status::get_status_info(&repo)?;

    ui::print_status(&status_info);
    
    Ok(())
}