use crate::cli::args::ReflogArgs;
use crate::cli::ui;
use crate::core;
use crate::core::open;
use crate::error::ReviusError;
use crate::fs;

pub fn run(args: ReflogArgs) -> Result<(), ReviusError> {
    let current_dir = fs::paths::get_current_dir()?;
    let repo = open::open_repository(&current_dir)?;

    let entries = core::reflog::get_reflog(&repo, args.ref_name.as_deref(), args.limit)?;

    ui::print_reflog(&entries);

    Ok(())
}