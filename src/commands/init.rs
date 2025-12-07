use crate::cli::args::InitArgs;
use crate::cli::ui;
use crate::core;
use crate::error::ReviusError;
use crate::fs;

pub fn run(args: InitArgs) -> Result<(), ReviusError> {
    let canonical_path = fs::io::canonicalize(&args.path)
        .map_err(|e| ReviusError::Io(args.path.clone(), e))?;
    let display_path = fs::io::clean_path_display(&canonical_path);
    
    core::init::create_repository(&canonical_path)?;

    ui::print_init_success(&display_path);
    Ok(())
}