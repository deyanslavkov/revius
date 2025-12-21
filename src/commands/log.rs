use crate::cli::args::LogArgs;
use crate::cli::ui;
use crate::core;
use crate::core::models::objects::LogOptions;
use crate::error::ReviusError;
use crate::fs;

pub fn run(args: LogArgs) -> Result<(), ReviusError> {
    let current_dir = fs::paths::get_current_dir()?;
    let repo = core::open::open_repository(&current_dir)?;

    let options = LogOptions {
        limit: args.limit,
        show_graph: args.graph,
        oneline: args.oneline,
        first_parent: args.first_parent,
    };

    let commits = core::log::get_commit_history(&repo.conn, &options)?;

    if commits.is_empty() {
        ui::print_no_commits();
        return Ok(());
    }

    ui::print_log(&commits, &options);
    Ok(())
}