//! Directory walking for file discovery.

use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

use crate::config::Config;
use crate::error::PctxError;
use crate::filter::binary;

/// Scan a directory for files using the ignore crate's walker
pub fn scan_directory(dir: &Path, config: &Config) -> Result<Vec<PathBuf>, PctxError> {
    let mut files = Vec::new();

    let include_hidden_clone = config.include_hidden;

    let walker = WalkBuilder::new(dir)
        // Disable all standard filters first, then enable what we want
        .standard_filters(false)
        .hidden(!config.include_hidden)
        .git_ignore(config.use_gitignore)
        .git_global(config.use_gitignore)
        .git_exclude(config.use_gitignore)
        .ignore(false) // Don't use .ignore files
        .parents(config.use_gitignore) // Only check parent gitignores if using gitignore
        .max_depth(config.max_depth)
        .filter_entry(move |entry| {
            let path = entry.path();

            // Always allow directories to enable traversal
            if path.is_dir() {
                // But skip hidden directories if not including hidden
                if let Some(name) = path.file_name() {
                    let name_str = name.to_string_lossy();
                    if !include_hidden_clone && name_str.starts_with('.') {
                        return false;
                    }
                }
                return true;
            }

            true
        })
        .build();

    for entry in walker.flatten() {
        let path = entry.path();
        if path.is_file() {
            // Skip binary files early
            if binary::is_binary(path) {
                continue;
            }
            files.push(path.to_path_buf());
        }
    }

    Ok(files)
}
