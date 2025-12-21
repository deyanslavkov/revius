use crate::core::models::objects::{LogOptions, CommitInfo};
use crate::db;
use crate::error::ReviusError;
use rusqlite::Connection;
use std::collections::{HashSet, VecDeque};

/// Get commit history starting from HEAD, traversing the parent chain
pub fn get_commit_history(conn: &Connection, options: &LogOptions) -> Result<Vec<CommitInfo>, ReviusError> {
    let start_commit = match db::refs::resolve_head(conn)? {
        Some(hash) => hash,
        None => return Ok(Vec::new()),
    };

    let all_refs = db::refs::get_all_refs(conn)?;

    let mut commits = Vec::new();
    let mut to_visit = VecDeque::new();
    let mut visited = HashSet::new();

    to_visit.push_back(start_commit);

    while let Some(current_hash) = to_visit.pop_front() {
        if visited.contains(&current_hash) {
            continue;
        }

        visited.insert(current_hash);

        let commit = match db::commits::get_commit(conn, &current_hash)? {
            Some(c) => c,
            None => {
                continue;
            }
        };

        let (author_name, author_email) = db::authors::get_author_by_id(conn, commit.author_id)?;

        let refs: Vec<String> = all_refs
            .iter()
            .filter(|(_, hash)| *hash == current_hash)
            .map(|(path, _)| path.clone())
            .collect();

        commits.push(CommitInfo {
            hash: current_hash,
            parent_hash: commit.parent_hash,
            merge_parent_hash: commit.merge_parent_hash,
            tree_hash: commit.tree_hash,
            author_name,
            author_email,
            timestamp: commit.timestamp,
            message: commit.message,
            refs,
        });

        if let Some(parent) = commit.parent_hash {
            to_visit.push_back(parent);
        }

        if !options.first_parent {
            if let Some(merge_parent) = commit.merge_parent_hash {
                to_visit.push_back(merge_parent);
            }
        }

        if let Some(limit) = options.limit {
            if commits.len() >= limit {
                break;
            }
        }
    }

    Ok(commits)
}