pub fn serialize_tree_entry(mode: u32, name: &str, hash: &[u8; 32]) -> Vec<u8> {
    let mut result = Vec::new();

    result.extend_from_slice(&mode.to_be_bytes());

    result.extend_from_slice(name.as_bytes());

    result.push(0);

    result.extend_from_slice(hash);

    result
}

pub fn serialize_author(name: &str, email: &str, timestamp: i64) -> Result<String, String> {
    if name.contains('\n') {
        return Err("Author name cannot contain newlines".to_string());
    }
    if email.contains('\n') {
        return Err("Author email cannot contain newlines".to_string());
    }
    if name.is_empty() {
        return Err("Author name cannot be empty".to_string());
    }
    if email.is_empty() {
        return Err("Author email cannot be empty".to_string());
    }
    
    Ok(format!("author {} {} {}\n", name, email, timestamp))
}

// tree <tree_hash_hex>\n
// parent <parent_hash_hex>\n (if exists)
// merge_parent <merge_parent_hash_hex>\n (if exists)
// author <name> <email> <timestamp>\n
// message\n
// <message_text>
pub fn serialize_commit(tree_hash: &[u8; 32], parent_hash: Option<&[u8; 32]>, merge_parent_hash: Option<&[u8; 32]>, author_name: &str, author_email: &str, timestamp: i64, message: &str) -> Result<Vec<u8>, String> {
    let mut result = String::new();

    result.push_str(&format!("tree {}\n", hex::encode(tree_hash)));

    if let Some(parent) = parent_hash {
        result.push_str(&format!("parent {}\n", hex::encode(parent)));
    }

    if let Some(merge_parent) = merge_parent_hash {
        result.push_str(&format!("merge_parent {}\n", hex::encode(merge_parent)));
    }

    let author_line = serialize_author(author_name, author_email, timestamp)?;
    result.push_str(&author_line);

    result.push_str("message\n");

    result.push_str(message);

    Ok(result.into_bytes())
}