//! JSON output types for structured responses.

use dunce;
use serde::Serialize;
use std::path::Path;

use crate::content::FileEntry;
use crate::stats::Stats;

/// Error codes as string constants for consistency
pub mod error_codes {
    pub const FILE_NOT_FOUND: &str = "file_not_found";
    pub const PERMISSION_DENIED: &str = "permission_denied";
    pub const BINARY_FILE: &str = "binary_file";
    pub const FILE_TOO_LARGE: &str = "file_too_large";
    pub const ENCODING_ERROR: &str = "encoding_error";
    pub const INVALID_PATTERN: &str = "invalid_pattern";
    pub const NO_FILES_MATCHED: &str = "no_files_matched";
    pub const OUTPUT_EXISTS: &str = "output_exists";
    pub const GIT_ERROR: &str = "git_error";
    pub const CONFIG_ERROR: &str = "config_error";
    pub const CLIPBOARD_ERROR: &str = "clipboard_error";
    pub const IO_ERROR: &str = "io_error";
    pub const JSON_ERROR: &str = "json_error";
    pub const WALK_ERROR: &str = "walk_error";
    pub const IGNORE_ERROR: &str = "ignore_error";
}

/// Top-level JSON response wrapper
#[derive(Serialize, Debug)]
#[serde(tag = "status")]
pub enum JsonResponse {
    #[serde(rename = "success")]
    Success(SuccessResponse),

    #[serde(rename = "error")]
    Error(ErrorResponse),

    #[serde(rename = "partial")]
    Partial(PartialResponse),
}

/// Successful operation response
#[derive(Serialize, Debug)]
pub struct SuccessResponse {
    pub data: ResponseData,
    pub stats: StatsJson,
}

/// Response data variants
#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum ResponseData {
    Context(ContextOutput),
    FileList(Vec<FileInfo>),
    Tree(TreeOutput),
}

/// Generated context output
#[derive(Serialize, Debug)]
pub struct ContextOutput {
    /// The formatted context content
    pub content: String,
    /// Format used (markdown, xml, plain)
    pub format: String,
    /// List of included files
    pub files: Vec<FileInfo>,
}

/// Tree structure output
#[derive(Serialize, Debug)]
pub struct TreeOutput {
    pub tree: String,
}

/// Helper function for serde skip_serializing_if
fn is_zero(n: &usize) -> bool {
    *n == 0
}

/// Flat file information structure (agent-friendly)
#[derive(Serialize, Debug, Clone)]
pub struct FileInfo {
    pub path: String,
    pub extension: String,
    pub size_bytes: u64,
    /// Line count (0 if not calculated, e.g., in `files list`)
    #[serde(skip_serializing_if = "is_zero")]
    pub line_count: usize,
    pub truncated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated_lines: Option<usize>,
}

impl FileInfo {
    /// Try to create FileInfo from a path without reading full content
    ///
    /// Note: `line_count` will be 0 as content is not read.
    pub fn try_from_path(path: &Path) -> Result<Self, std::io::Error> {
        let metadata = std::fs::metadata(path)?;

        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_string();

        let canonical = dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        let relative_path = std::env::current_dir()
            .ok()
            .and_then(|cwd| canonical.strip_prefix(&cwd).ok().map(|p| p.to_path_buf()))
            .unwrap_or(canonical);

        Ok(Self {
            path: relative_path.to_string_lossy().to_string(),
            extension,
            size_bytes: metadata.len(),
            line_count: 0, // Not calculated without reading content
            truncated: false,
            truncated_lines: None,
        })
    }
}

impl From<&FileEntry> for FileInfo {
    fn from(entry: &FileEntry) -> Self {
        Self::from_entry(entry, false)
    }
}

impl FileInfo {
    /// Create FileInfo from FileEntry with path style preference
    pub fn from_entry(entry: &FileEntry, absolute: bool) -> Self {
        Self {
            path: if absolute {
                entry.absolute_path.to_string_lossy().to_string()
            } else {
                entry.relative_path.clone()
            },
            extension: entry.extension.clone(),
            size_bytes: entry.original_bytes as u64,
            line_count: entry.original_lines,
            truncated: entry.truncated,
            truncated_lines: if entry.truncated {
                Some(entry.truncated_lines)
            } else {
                None
            },
        }
    }
}

/// Statistics in JSON format
#[derive(Serialize, Debug, Clone)]
pub struct StatsJson {
    pub file_count: usize,
    pub total_lines: usize,
    pub total_bytes: usize,
    pub truncated_count: usize,
    pub skipped_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_estimate: Option<usize>,
    pub duration_ms: u64,
}

impl StatsJson {
    /// Create a new StatsJson with minimal required fields
    pub fn new(file_count: usize) -> Self {
        Self {
            file_count,
            total_lines: 0,
            total_bytes: 0,
            truncated_count: 0,
            skipped_count: 0,
            token_estimate: None,
            duration_ms: 0,
        }
    }
}

impl From<&Stats> for StatsJson {
    fn from(stats: &Stats) -> Self {
        Self {
            file_count: stats.file_count,
            total_lines: stats.total_lines,
            total_bytes: stats.total_bytes,
            truncated_count: stats.truncated_count,
            skipped_count: stats.skipped_count,
            token_estimate: stats.token_estimate,
            duration_ms: stats.duration_ms,
        }
    }
}

/// Structured error response
#[derive(Serialize, Debug)]
pub struct ErrorResponse {
    /// Machine-readable error code
    pub code: String,
    /// Human-readable message
    pub message: String,
    /// The input that caused the error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<serde_json::Value>,
    /// Suggested fix or next action
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    /// Is this error transient (worth retrying)?
    pub transient: bool,
    /// Exit code that will be returned
    pub exit_code: i32,
}

/// Partial success response (some files processed, some failed)
#[derive(Serialize, Debug)]
pub struct PartialResponse {
    pub data: ResponseData,
    pub stats: StatsJson,
    pub errors: Vec<FileError>,
}

/// Individual file error
#[derive(Serialize, Debug, Clone)]
pub struct FileError {
    pub path: String,
    pub code: String,
    pub message: String,
    pub transient: bool,
}
