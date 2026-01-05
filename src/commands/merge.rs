use crate::cli::args::MergeArgs;
use crate::cli::ui;
use crate::core;
use crate::error::ReviusError;
use crate::fs;
use crate::core::switch::resolve_target;

pub fn run(args: MergeArgs) -> Result<(), ReviusError> {
    let current_dir = fs::paths::get_current_dir()?;
    let repo = core::open::open_repository(&current_dir)?;

    // Resolve target to commit hash
    let (_, target_commit) = resolve_target(&repo.conn, &args.target)?;
    
    // Perform the merge
    match core::merge::perform_merge(&repo, target_commit)? {
        core::merge::MergeResult::FastForward { from, to } => {
            ui::print_merge_fast_forward(&from, &to);
        }
        core::merge::MergeResult::AlreadyUpToDate => {
            ui::print_merge_already_up_to_date();
        }
        core::merge::MergeResult::MergeCommit { commit_hash, files_changed } => {
            ui::print_merge_success(&commit_hash, files_changed);
        }
        core::merge::MergeResult::Conflicts(conflicts) => {
            ui::print_merge_conflicts(&conflicts);
            return Err(ReviusError::MergeError("Merge conflicts detected".to_string()));
        }
    }

    Ok(())
}