//! Content processing and transformation.

pub mod reader;
pub mod truncator;

use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::error::PctxError;
use crate::filter::binary::{check_binary, BinaryCheckResult};

/// A processed file entry ready for output
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// Absolute path to the file
    pub absolute_path: PathBuf,
    /// Path relative to working directory
    pub relative_path: String,
    /// File extension (without dot)
    pub extension: String,
    /// Processed content (possibly truncated)
    pub content: String,
    /// Original line count before truncation
    pub original_lines: usize,
    /// Original size in bytes
    pub original_bytes: usize,
    /// Whether content was truncated
    pub truncated: bool,
    /// Number of lines removed by truncation
    pub truncated_lines: usize,
}

impl FileEntry {
    /// Get the display path based on configuration
    pub fn display_path(&self, absolute: bool) -> &str {
        if absolute {
            self.absolute_path.to_str().unwrap_or(&self.relative_path)
        } else {
            &self.relative_path
        }
    }
}

/// Processor for reading and transforming file content
pub struct ContentProcessor<'a> {
    config: &'a Config,
}

impl<'a> ContentProcessor<'a> {
    /// Create a new content processor
    pub fn new(config: &'a Config) -> Self {
        Self { config }
    }

    /// Process a file and return a FileEntry
    pub fn process(&self, path: &Path) -> Result<FileEntry, PctxError> {
        // Check if binary (with proper error handling)
        match check_binary(path) {
            BinaryCheckResult::Binary => {
                return Err(PctxError::BinaryFile(path.to_path_buf()));
            }
            BinaryCheckResult::Error(e) => {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    return Err(PctxError::PermissionDenied(path.to_path_buf()));
                }
                return Err(PctxError::Io(e));
            }
            BinaryCheckResult::Text => {}
        }

        // Read content
        let raw_content = reader::read_file(path)?;
        let original_lines = raw_content.lines().count();
        let original_bytes = raw_content.len();

        // Truncate if needed
        let (content, truncated, truncated_lines) =
            truncator::truncate_content(&raw_content, &self.config.truncation);

        // Get absolute and relative paths
        let (absolute_path, relative_path) = self.get_paths(path);

        // Get extension
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_string();

        Ok(FileEntry {
            absolute_path,
            relative_path,
            extension,
            content,
            original_lines,
            original_bytes,
            truncated,
            truncated_lines,
        })
    }

    /// Get both absolute and relative paths for a file
    fn get_paths(&self, path: &Path) -> (PathBuf, String) {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        // Canonicalize cwd to handle symlinks - when cwd is accessed via a symlink,
        // file paths from the scanner will be relative to the resolved (real) path,
        // not the symlink path. We need both to resolve to compare them.
        let canonical_cwd = cwd.canonicalize().unwrap_or_else(|_| cwd.clone());

        // Try to canonicalize to get absolute path
        let absolute_path = path.canonicalize().unwrap_or_else(|_| {
            // If canonicalize fails, try to make it absolute
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                canonical_cwd.join(path)
            }
        });

        // Normalize paths using dunce to handle Windows long path format (\\?\...)
        // This ensures consistent path format for comparison
        let normalized_abs = dunce::simplified(&absolute_path);
        let normalized_cwd = dunce::simplified(&canonical_cwd);

        // Try to get relative path by stripping cwd prefix
        let relative_path = if let Ok(relative) = normalized_abs.strip_prefix(normalized_cwd) {
            relative.to_string_lossy().to_string()
        } else if !path.is_absolute() {
            // If strip_prefix fails and input was relative, use input as relative
            path.to_string_lossy().to_string()
        } else {
            // Last resort: use normalized absolute path (without \\?\ prefix)
            normalized_abs.to_string_lossy().to_string()
        };

        (absolute_path, relative_path)
    }
}