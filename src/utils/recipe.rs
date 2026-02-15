pub fn parse_recipe(recipe: &[u8]) -> Result<Vec<[u8; 32]>, String> {
    if !recipe.len().is_multiple_of(32) {
        return Err(format!("Invalid recipe length: {} (not multiple of 32)", recipe.len()));
    }
    
    Ok(recipe.chunks_exact(32)
        .map(|chunk| {
            let mut hash = [0u8; 32];
            hash.copy_from_slice(chunk);
            hash
        })
        .collect())
}

pub fn build_recipe(hashes: &[[u8; 32]]) -> Vec<u8> {
    hashes.iter().flat_map(|h| h.iter().copied()).collect()
}