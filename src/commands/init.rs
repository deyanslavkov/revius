use crate::core::repository::init as repo_init;
use crate::error::ReviusError;
use std::path::Path;

/// CLI command wrapper for init
pub fn run(root: &Path) -> Result<(), ReviusError> {
    match repo_init::init(root) {
        Ok(repo) => {
            // prefer canonical path for final message
            let root_path = repo.root;
            crate::cli::ui::print_init_success(&root_path);
            Ok(())
        }
        Err(e) => Err(e),
    }
}
