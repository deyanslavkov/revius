use crate::cli::args::CommitArgs;
use crate::cli::ui;
use crate::core;
use crate::error::ReviusError;
use crate::core::refs;
use crate::fs::paths;

pub fn run(args: CommitArgs) -> Result<(), ReviusError> {
    let start_path = paths::get_current_dir()?;

    let repo = core::open::open_repository(&start_path)?;

    if args.message.trim().is_empty() {
        return Err(ReviusError::Usage("Commit message cannot be empty".to_string()));
    }

    if repo.config.user_name.is_none() || repo.config.user_email.is_none() {
        ui::print_no_user_configured();
        return Err(ReviusError::Config(
            "User name and email must be configured before committing".to_string()
        ));
    }

    let (commit_hash, files_changed) = core::commit::create_commit(&repo, &args.message)?;

    ui::print_commit_success(&commit_hash, &args.message, files_changed);

    Ok(())
}