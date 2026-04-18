//! Git-aware file scanning using the `ignore` crate.

use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

use crate::config::Config;
use crate::error::PctxError;

/// Check if a directory is inside a git repository
pub fn is_git_repo(path: &Path) -> bool {
    let start = if path.is_file() {
        path.parent().map(|p| p.to_path_buf())
    } else {
        Some(path.to_path_buf())
    };

    let mut current = start;

    while let Some(dir) = current {
        if dir.join(".git").exists() {
            return true;
        }
        current = dir.parent().map(|p| p.to_path_buf());
    }

    false
}

/// Scan a git repository, respecting .gitignore rules
pub fn scan_git_repo(path: &Path, config: &Config) -> Result<Vec<PathBuf>, PctxError> {
    let mut builder = WalkBuilder::new(path);

    builder
        .hidden(!config.include_hidden)
        .git_ignore(config.use_gitignore)
        .git_global(config.use_gitignore)
        .git_exclude(config.use_gitignore)
        .ignore(true)
        .parents(true)
        .follow_links(false) // Explicit: don't follow symlinks to prevent loops
        .same_file_system(true); // Don't cross filesystem boundaries

    if let Some(max_depth) = config.max_depth {
        builder.max_depth(Some(max_depth));
    }

    let mut files = Vec::new();

    for entry in builder.build() {
        let entry = entry?;
        let path = entry.path();

        // Only include files (not directories)
        if path.is_file() {
            files.push(path.to_path_buf());
        }
    }

    Ok(files)
}