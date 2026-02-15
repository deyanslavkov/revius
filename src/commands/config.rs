use crate::cli::args::ConfigArgs;
use crate::cli::ui;
use crate::core::config;
use crate::error::ReviusError;

pub fn run(args: ConfigArgs) -> Result<(), ReviusError> {
    // 1. Handle --user shortcut
    if let Some(user_details) = args.user {
        if user_details.len() != 2 {
             return Err(ReviusError::Usage("The --user flag requires exactly two arguments: <name> <email>".to_string()));
        }
        let name = &user_details[0];
        let email = &user_details[1];
        
        config::set_user_identity(name, email)?;
        ui::print_user_setup_success(name, email);
        return Ok(());
    }

    // 2. Handle Key-Value pair
    if let (Some(key), Some(value)) = (args.key, args.value) {
        let scope = config::set_config_value(&key, &value)?;
        ui::print_config_set_success(&key, &value, &scope);
        return Ok(());
    }

    // 3. No valid arguments
    Err(ReviusError::Usage("Invalid arguments. Use 'rvs config <key> <value>' or 'rvs config --user <name> <email>'".to_string()))
}