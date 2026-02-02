use crate::cli::args::GcArgs;
use crate::cli::ui;
use crate::core::gc;
use crate::core::open;
use crate::error::ReviusError;
use crate::fs;

pub fn run(args: GcArgs) -> Result<(), ReviusError> {
    let current_dir = fs::paths::get_current_dir()?;
    let repo = open::open_repository(&current_dir)?;

    ui::print_gc_start(args.dry_run);

    let stats = gc::run_garbage_collection(&repo, args.dry_run)?;

    ui::print_gc_stats(&stats);

    Ok(())
}