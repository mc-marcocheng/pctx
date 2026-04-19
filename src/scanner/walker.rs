//! Directory walking for file discovery.

use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

use crate::config::Config;
use crate::error::PctxError;
use crate::filter::binary;

/// Scan a directory for files using the ignore crate's walker
pub fn scan_directory(dir: &Path, config: &Config) -> Result<Vec<PathBuf>, PctxError> {
    let mut files = Vec::new();

    let walker = WalkBuilder::new(dir)
        .standard_filters(false)
        .hidden(!config.include_hidden)
        .git_ignore(config.use_gitignore)
        .git_global(config.use_gitignore)
        .git_exclude(config.use_gitignore)
        .ignore(false)
        .parents(config.use_gitignore)
        .max_depth(config.max_depth)
        .build();

    for entry in walker.flatten() {
        let path = entry.path();
        if entry.file_type().is_some_and(|ft| ft.is_file()) {
            // Skip binary files early
            if binary::is_binary(path) {
                continue;
            }
            files.push(path.to_path_buf());
        }
    }

    Ok(files)
}
