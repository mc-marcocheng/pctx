//! File scanning and discovery.

pub mod git;
pub mod walker;

use std::path::PathBuf;

use crate::config::Config;
use crate::error::PctxError;
use crate::filter::patterns::PatternMatcher;

/// Result of a file scan operation
pub struct ScanResult {
    pub files: Vec<PathBuf>,
    pub errors: Vec<(PathBuf, PctxError)>,
}

/// Scanner for discovering files to include in context
pub struct Scanner<'a> {
    config: &'a Config,
    user_pattern_matcher: PatternMatcher,
    base_paths: Vec<PathBuf>,
}

impl<'a> Scanner<'a> {
    /// Create a new scanner with the given configuration
    pub fn new(config: &'a Config) -> Self {
        let user_pattern_matcher =
            PatternMatcher::new(&config.exclude_patterns, &config.include_patterns);

        let base_paths: Vec<PathBuf> = config
            .paths
            .iter()
            .filter_map(|p| dunce::canonicalize(p).ok())
            .collect();

        Self {
            config,
            user_pattern_matcher,
            base_paths,
        }
    }

    /// Scan configured paths and return list of files to process
    pub fn scan(&self) -> Result<ScanResult, PctxError> {
        let mut all_files = Vec::new();
        let mut errors = Vec::new();

        for path in &self.config.paths {
            if !path.exists() {
                let err = PctxError::FileNotFound(path.clone());
                if self.config.verbose {
                    eprintln!("Warning: {}", err);
                }
                errors.push((path.clone(), err));
                continue;
            }

            let canonical = dunce::canonicalize(path).unwrap_or_else(|_| path.clone());

            if canonical.is_file() {
                if self.should_include(&canonical) {
                    all_files.push(canonical);
                }
            } else if canonical.is_dir() {
                let files = if self.config.use_gitignore && git::is_inside_git_repo(&canonical) {
                    match git::scan_git_repo(&canonical, self.config) {
                        Ok(files) => files,
                        Err(e) => {
                            if self.config.verbose {
                                eprintln!(
                                    "Warning: git scan failed ({}), falling back to directory walk",
                                    e
                                );
                            }
                            walker::scan_directory(&canonical, self.config)?
                        }
                    }
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

        Ok(ScanResult {
            files: all_files,
            errors,
        })
    }

    /// Scan from a list of explicit file paths (for --stdin mode)
    pub fn scan_paths(&self, paths: Vec<PathBuf>) -> Result<ScanResult, PctxError> {
        let mut all_files = Vec::new();
        let mut errors = Vec::new();

        for path in paths {
            if !path.exists() {
                // Skip non-existent files in stdin mode with a warning
                // (they may have been deleted since the list was generated)
                if self.config.verbose {
                    eprintln!("Warning: file not found, skipping: {}", path.display());
                }
                errors.push((path.clone(), PctxError::FileNotFound(path.clone())));
                continue;
            }

            let canonical = dunce::canonicalize(&path).unwrap_or_else(|_| path.clone());

            if canonical.is_file() {
                if self.should_include(&canonical) {
                    all_files.push(canonical);
                }
            } else if canonical.is_dir() {
                // For directories in stdin mode, we expand them
                let files = if self.config.use_gitignore && git::is_inside_git_repo(&canonical) {
                    match git::scan_git_repo(&canonical, self.config) {
                        Ok(files) => files,
                        Err(e) => {
                            if self.config.verbose {
                                eprintln!(
                                    "Warning: git scan failed ({}), falling back to directory walk",
                                    e
                                );
                            }
                            walker::scan_directory(&canonical, self.config)?
                        }
                    }
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

        Ok(ScanResult {
            files: all_files,
            errors,
        })
    }

    /// Check if a file should be included based on patterns and size
    fn should_include(&self, path: &PathBuf) -> bool {
        // Convert to relative path for pattern matching, preferring scan base paths
        let relative = self
            .base_paths
            .iter()
            .find_map(|base| path.strip_prefix(base).ok().map(|p| p.to_path_buf()))
            .or_else(|| {
                std::env::current_dir()
                    .ok()
                    .and_then(|cwd| path.strip_prefix(&cwd).ok().map(|p| p.to_path_buf()))
            })
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
