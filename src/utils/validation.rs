use crate::error::ReviusError;

pub fn validate_branch_name(name: &str) -> Result<(), ReviusError> {
    if name.is_empty() {
        return Err(ReviusError::InvalidBranchName(
            "Branch name cannot be empty".to_string(),
        ));
    }

    if name == "HEAD" {
        return Err(ReviusError::InvalidBranchName(
            "Branch name cannot be 'HEAD'".to_string(),
        ));
    }

    if name.starts_with('.') {
        return Err(ReviusError::InvalidBranchName(
            "Branch name cannot start with '.'".to_string(),
        ));
    }

    if name.starts_with('-') {
        return Err(ReviusError::InvalidBranchName(
            "Branch name cannot start with '-'".to_string(),
        ));
    }

    if name.contains("..") {
        return Err(ReviusError::InvalidBranchName(
            "Branch name cannot contain consecutive dots (..)".to_string(),
        ));
    }

    let invalid_chars = [' ', '?', '*', '[', ']', '~', '^', ':', '@', '{', '}', '\\'];
    for ch in invalid_chars.iter() {
        if name.contains(*ch) {
            return Err(ReviusError::InvalidBranchName(format!(
                "Branch name cannot contain '{}'",
                ch
            )));
        }
    }

    if name.starts_with('/') || name.ends_with('/') {
        return Err(ReviusError::InvalidBranchName(
            "Branch name cannot start or end with '/'".to_string(),
        ));
    }

    if name.contains("//") {
        return Err(ReviusError::InvalidBranchName(
            "Branch name cannot contain consecutive slashes (//)".to_string(),
        ));
    }

    Ok(())
}