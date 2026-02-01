use crate::cli::args::ResetArgs;
use crate::cli::ui;
use crate::core::{self, open};
use crate::error::ReviusError;
use crate::fs;

pub fn run(args: ResetArgs) -> Result<(), ReviusError> {
    let current_dir = fs::paths::get_current_dir()?;
    let repo = open::open_repository(&current_dir)?;

    // Default target is HEAD if not specified
    let target = args.target.as_deref().unwrap_or("HEAD");

    // Ensure mutual exclusion of modes
    let mode_count = (args.soft as u8) + (args.mixed as u8) + (args.hard as u8);
    if mode_count > 1 {
        return Err(ReviusError::Usage("Cannot specify multiple reset modes (--soft, --mixed, --hard) at once".to_string()));
    }

    // Determine mode (Mixed is default)
    let final_hash = if args.hard {
        core::reset::reset_hard(&repo, target)?
    } else if args.soft {
        core::reset::reset_soft(&repo, target)?
    } else {
        core::reset::reset_mixed(&repo, target)?
    };

    let mode_str = if args.hard { "hard" } else if args.soft { "soft" } else { "mixed" };
    ui::print_reset_success(mode_str, &final_hash);

    Ok(())
}