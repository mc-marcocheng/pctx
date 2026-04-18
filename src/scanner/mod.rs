//! File scanning and discovery.

pub mod git;
pub mod walker;

use std::path::PathBuf;

use crate::config::Config;
use crate::error::PctxError;
use crate::filter::patterns::PatternMatcher;

/// Scanner for discovering files to include in context
pub struct Scanner<'a> {
    config: &'a Config,
    user_pattern_matcher: PatternMatcher,
}

impl<'a> Scanner<'a> {
    /// Create a new scanner with the given configuration
    pub fn new(config: &'a Config) -> Self {
        // Pattern matcher for all exclude/include filtering
        // (gitignore and hidden files are handled by the walker/git scanner,
        // but default excludes and user patterns are handled here)
        let user_pattern_matcher =
            PatternMatcher::new(&config.exclude_patterns, &config.include_patterns);

        Self {
            config,
            user_pattern_matcher,
        }
    }

    /// Scan configured paths and return list of files to process
    pub fn scan(&self) -> Result<Vec<PathBuf>, PctxError> {
        let mut all_files = Vec::new();

        for path in &self.config.paths {
            let canonical = if path.exists() {
                path.canonicalize().unwrap_or_else(|_| path.clone())
            } else {
                // Check if it's a file that doesn't exist
                if !path.is_dir() && path.extension().is_some() {
                    return Err(PctxError::FileNotFound(path.clone()));
                }
                return Err(PctxError::DirectoryNotFound(path.clone()));
            };

            if canonical.is_file() {
                // Direct file path
                if self.should_include(&canonical) {
                    all_files.push(canonical);
                }
            } else if canonical.is_dir() {
                // Directory - check if it's a git repo
                let files = if self.config.use_gitignore && git::is_git_repo(&canonical) {
                    git::scan_git_repo(&canonical, self.config)?
                } else {
                    walker::scan_directory(&canonical, self.config)?
                };

                for file in files {
                    if self.should_include(&file) {
                        all_files.push(file);
                    }
                }
            }
        }

        // Sort for consistent output
        all_files.sort();
        all_files.dedup();

        Ok(all_files)
    }

    /// Check if a file should be included based on patterns and size
    fn should_include(&self, path: &PathBuf) -> bool {
        // Convert to relative path for pattern matching
        let relative = std::env::current_dir()
            .ok()
            .and_then(|cwd| path.strip_prefix(&cwd).ok().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| path.clone());

        // Check exclude patterns
        if self.user_pattern_matcher.is_excluded(&relative) {
            return false;
        }

        // Check include patterns (if any specified)
        if !self.user_pattern_matcher.is_included(&relative) {
            return false;
        }

        // Check file size
        if let Ok(metadata) = std::fs::metadata(path) {
            if metadata.len() > self.config.max_file_size {
                return false;
            }
        }

        true
    }
}