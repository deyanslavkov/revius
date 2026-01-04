use crate::cli::args::SwitchArgs;
use crate::cli::ui;
use crate::core;
use crate::error::ReviusError;
use crate::fs;

pub fn run(args: SwitchArgs) -> Result<(), ReviusError> {
    // Open repository
    let current_dir = fs::paths::get_current_dir()?;
    let repo = core::open::open_repository(&current_dir)?;

    // Perform switch operation (handles all flags: create, force)
    let result = core::switch::switch_to_target(&repo, &args.target, args.create, args.force)?;
    
    // Print success message
    ui::print_switch_success(
        &result.previous_head,
        &result.new_head,
        result.files_changed,
        result.files_deleted,
    );
    
    Ok(())
}