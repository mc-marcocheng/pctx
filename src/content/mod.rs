//! Content reading and processing.

pub mod reader;
pub mod truncator;

use std::path::{Path, PathBuf};

pub use reader::read_file_contents;
pub use truncator::truncate_content;

use crate::config::Config;
use crate::error::PctxError;
use crate::filter::binary;

/// Represents a processed file with its content and metadata
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// Absolute path to the file
    pub absolute_path: PathBuf,
    /// Relative path (for display)
    pub relative_path: String,
    /// File extension (empty string if none)
    pub extension: String,
    /// Original file size in bytes
    pub original_bytes: usize,
    /// Original number of lines
    pub original_lines: usize,
    /// Number of lines in the processed content
    pub line_count: usize,
    /// Whether the content was truncated
    pub truncated: bool,
    /// Number of lines that were truncated
    pub truncated_lines: usize,
    /// The processed content
    pub content: String,
}

impl FileEntry {
    /// Get the display path as a string
    pub fn display_path(&self, absolute: bool) -> String {
        if absolute {
            self.absolute_path.to_string_lossy().to_string()
        } else {
            self.relative_path.clone()
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
        // Check if binary
        if binary::is_binary(path) {
            return Err(PctxError::BinaryFile(path.to_path_buf()));
        }

        // Read content
        let raw_content = read_file_contents(path, None)?;
        let original_bytes = raw_content.len();
        let original_lines = raw_content.lines().count();

        // Truncate if needed
        let (content, truncated, truncated_lines) =
            truncate_content(&raw_content, &self.config.truncation);

        let line_count = content.lines().count();

        // Compute relative path
        let relative_path = std::env::current_dir()
            .ok()
            .and_then(|cwd| path.strip_prefix(&cwd).ok().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| path.to_path_buf());

        let extension = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        Ok(FileEntry {
            absolute_path: path.to_path_buf(),
            relative_path: relative_path.to_string_lossy().to_string(),
            extension,
            original_bytes,
            original_lines,
            line_count,
            truncated,
            truncated_lines,
            content,
        })
    }
}
