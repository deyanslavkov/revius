use crate::error::ReviusError;
use crate::utils::hash;
use crate::db;
use crate::core::models::objects::HeadReference;
use rusqlite::Connection;

#[derive(Debug, PartialEq)]
pub enum ResolvedTarget {
    /// The target resolved to a specific branch (e.g., "main")
    Branch(String, [u8; 32]),
    /// The target resolved to a specific commit (detached, tag, or hash)
    Commit([u8; 32]),
}

impl ResolvedTarget {
    pub fn hash(&self) -> [u8; 32] {
        match self {
            ResolvedTarget::Branch(_, h) => *h,
            ResolvedTarget::Commit(h) => *h,
        }
    }
}

/// Resolves a user-provided string to a commit.
/// Handles:
/// - Branches: "main"
/// - Tags: "v1.0"
/// - HEAD: "HEAD"
/// - Hashes: Full or prefix
/// - Ancestry: "~n" or "^" suffix (e.g., "main~2", "HEAD^")
pub fn resolve_target(conn: &Connection, target: &str) -> Result<ResolvedTarget, ReviusError> {
    // 1. Parse Ancestry (e.g., "main~2" -> base: "main", generations: 2)
    let (base_str, generations) = parse_ancestry(target);

    // 2. Resolve the base revision to a hash and type
    let mut current_resolution = resolve_base(conn, base_str)?;
    let mut current_hash = current_resolution.hash();

    // 3. Apply ancestry traversal
    if generations > 0 {
        // If we traverse back, it's no longer a branch reference, it's a specific commit
        for _ in 0..generations {
            let parents = db::commits::get_commit_parents(conn, &current_hash)?;
            if parents.is_empty() {
                return Err(ReviusError::TargetNotFound(format!(
                    "Commit {} has no parent (reached root)",
                    hash::hash_to_short_hex(&current_hash)
                )));
            }
            // Always follow the first parent for ~N syntax
            current_hash = parents[0];
        }
        // Result is always a raw Commit after traversal
        current_resolution = ResolvedTarget::Commit(current_hash);
    }

    Ok(current_resolution)
}

/// Just get the hash (wrapper around resolve_target)
pub fn resolve_commit_hash(conn: &Connection, target: &str) -> Result<[u8; 32], ReviusError> {
    resolve_target(conn, target).map(|r| r.hash())
}

fn parse_ancestry(target: &str) -> (&str, usize) {
    if let Some(pos) = target.find('~') {
        let (base, suffix) = target.split_at(pos);
        // suffix is like "~2" or "~"
        let count_str = &suffix[1..];
        let count = if count_str.is_empty() {
            1
        } else {
            count_str.parse::<usize>().unwrap_or(1)
        };
        return (base, count);
    }
    
    if let Some(pos) = target.find('^') {
        let (base, suffix) = target.split_at(pos);
        // Count how many ^ are there
        let count = suffix.chars().filter(|c| *c == '^').count();
        return (base, count);
    }

    (target, 0)
}

fn resolve_base(conn: &Connection, base: &str) -> Result<ResolvedTarget, ReviusError> {
    // 1. Try HEAD
    if base == "HEAD" {
        // We need to know if HEAD points to a ref or is detached
        let head_state = crate::core::refs::get_head_state(conn)?;
        match head_state {
            HeadReference::Branch(ref_path) => {
                let name = ref_path.strip_prefix("refs/heads/").unwrap_or(&ref_path).to_string();
                let hash = db::refs::get_ref(conn, &ref_path)?
                    .ok_or_else(|| ReviusError::Db(format!("Ref {} not found", ref_path)))?;
                return Ok(ResolvedTarget::Branch(name, hash));
            }
            HeadReference::Detached(hash) => {
                return Ok(ResolvedTarget::Commit(hash));
            }
        }
    }

    // 2. Try as a Branch Name (refs/heads/)
    let branch_path = format!("refs/heads/{}", base);
    if let Some(hash) = db::refs::get_ref(conn, &branch_path)? {
        return Ok(ResolvedTarget::Branch(base.to_string(), hash));
    }

    // 3. Try as a Tag (refs/tags/)
    let tag_path = format!("refs/tags/{}", base);
    if let Some(hash) = db::refs::get_ref(conn, &tag_path)? {
        return Ok(ResolvedTarget::Commit(hash));
    }

    // 4. Try as a Hex Hash (Full or Prefix)
    // Only attempt if it looks like hex
    if hash::is_valid_hash_prefix(base) {
        // If it's exactly 64 chars, try direct lookup first (fastest)
        if base.len() == 64 {
            let (bytes, _) = hash::hex_prefix_to_bytes(base)
                .map_err(|_| ReviusError::InvalidHashPrefix(base.to_string()))?;
            let hash = hash::vec_to_hash(&bytes).unwrap();
            
            if db::commits::commit_exists(conn, &hash)? {
                return Ok(ResolvedTarget::Commit(hash));
            }
        }

        // Try prefix search
        let matches = db::commits::find_commits_by_prefix(conn, base)?;
        match matches.len() {
            0 => {
                // If it looked like a hash but wasn't found, and wasn't a branch/tag:
                return Err(ReviusError::CommitNotFound(base.to_string()));
            }
            1 => return Ok(ResolvedTarget::Commit(matches[0])),
            _ => return Err(ReviusError::AmbiguousHashPrefix(base.to_string())),
        }
    }

    Err(ReviusError::TargetNotFound(base.to_string()))
}