//! Directory walking without git (for non-git directories).

use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::config::Config;
use crate::error::PctxError;
use crate::filter::patterns::PatternMatcher;

/// Scan a directory without git support (uses walkdir)
pub fn scan_directory(path: &Path, config: &Config) -> Result<Vec<PathBuf>, PctxError> {
    let mut walker = WalkDir::new(path).follow_links(false); // Explicit: don't follow symlinks

    if let Some(max_depth) = config.max_depth {
        walker = walker.max_depth(max_depth);
    }

    // Create a pattern matcher for early directory filtering (performance optimization)
    let exclude_matcher = PatternMatcher::new(&config.exclude_patterns, &[]);

    let mut files = Vec::new();

    for entry in walker.into_iter().filter_entry(|e| {
        let name = e.file_name().to_string_lossy();

        // Skip hidden files/directories unless configured to include them
        if !config.include_hidden && name.starts_with('.') {
            return false;
        }

        // Skip excluded directories early for performance
        // This prevents walking into node_modules, target, etc.
        if e.file_type().is_dir() && exclude_matcher.is_excluded(e.path()) {
            return false;
        }

        true
    }) {
        let entry = entry?;
        let path = entry.path();

        // Only include files
        if path.is_file() {
            files.push(path.to_path_buf());
        }
    }

    Ok(files)
}
