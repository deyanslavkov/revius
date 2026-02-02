use crate::cli::args::RestoreArgs;
use crate::cli::ui;
use crate::core::{self, open};
use crate::error::ReviusError;
use crate::fs;

pub fn run(args: RestoreArgs) -> Result<(), ReviusError> {
    let current_dir = fs::paths::get_current_dir()?;
    let repo = open::open_repository(&current_dir)?;

    let source = args.source.as_deref().unwrap_or("HEAD");

    // Determine Mode
    let worktree = args.worktree || (!args.staged && !args.worktree);
    let staged = args.staged;

    let count = if staged && worktree {
        // Mixed restore
        core::restore::restore_mixed(&repo, &args.paths, source)?
    } else if staged {
        // Staged only
        core::restore::restore_staged(&repo, &args.paths, source)?
    } else {
        // Worktree only (Source arg is ignored, source is Staging)
        core::restore::restore_worktree(&repo, &args.paths)?
    };

    let mode_str = if staged && worktree { "mixed" } else if staged { "staged" } else { "worktree" };
    ui::print_restore_success(mode_str, count);

    Ok(())
}