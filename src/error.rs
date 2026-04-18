//! Error types for the application.
//!
//! Errors include machine-readable codes and suggestions for recovery.

use std::path::PathBuf;
use thiserror::Error;

use crate::exit_codes::exit;
use crate::output::json_types::error_codes;

/// Main error type for pctx operations
#[derive(Error, Debug)]
pub enum PctxError {
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Directory not found: {0}")]
    DirectoryNotFound(PathBuf),

    #[error("Permission denied: {0}")]
    PermissionDenied(PathBuf),

    #[error("Output file already exists: {0}")]
    OutputExists(PathBuf),

    #[error("Cannot process binary file: {0}")]
    BinaryFile(PathBuf),

    #[error("File too large ({size} bytes, max {max}): {path}")]
    FileTooLarge { path: PathBuf, size: u64, max: u64 },

    #[error("Invalid pattern '{pattern}': {reason}")]
    InvalidPattern { pattern: String, reason: String },

    #[error("Encoding error reading {path}: {reason}")]
    EncodingError { path: PathBuf, reason: String },

    #[error("Git error: {0}")]
    GitError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Clipboard error: {0}")]
    ClipboardError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("TOML parsing error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("TOML serialization error: {0}")]
    TomlSer(#[from] toml::ser::Error),

    #[error("Pattern matching error: {0}")]
    Pattern(#[from] glob::PatternError),

    #[error("Directory walk error: {0}")]
    WalkDir(#[from] walkdir::Error),

    #[error("Ignore pattern error: {0}")]
    Ignore(#[from] ignore::Error),
}

impl PctxError {
    /// Returns a machine-readable error code for structured output
    pub fn code(&self) -> &'static str {
        match self {
            Self::FileNotFound(_) | Self::DirectoryNotFound(_) => error_codes::FILE_NOT_FOUND,
            Self::PermissionDenied(_) => error_codes::PERMISSION_DENIED,
            Self::OutputExists(_) => error_codes::OUTPUT_EXISTS,
            Self::BinaryFile(_) => error_codes::BINARY_FILE,
            Self::FileTooLarge { .. } => error_codes::FILE_TOO_LARGE,
            Self::InvalidPattern { .. } | Self::Pattern(_) => error_codes::INVALID_PATTERN,
            Self::EncodingError { .. } => error_codes::ENCODING_ERROR,
            Self::GitError(_) => error_codes::GIT_ERROR,
            Self::ConfigError(_) | Self::Toml(_) | Self::TomlSer(_) => error_codes::CONFIG_ERROR,
            Self::ClipboardError(_) => error_codes::CLIPBOARD_ERROR,
            Self::Io(_) => error_codes::IO_ERROR,
            Self::Json(_) => error_codes::JSON_ERROR,
            Self::WalkDir(_) => error_codes::WALK_ERROR,
            Self::Ignore(_) => error_codes::IGNORE_ERROR,
        }
    }

    /// Returns the appropriate exit code for this error
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::FileNotFound(_) | Self::DirectoryNotFound(_) => exit::NOT_FOUND,
            Self::PermissionDenied(_) => exit::PERMISSION_DENIED,
            Self::OutputExists(_) => exit::CONFLICT,
            Self::InvalidPattern { .. } | Self::Pattern(_) => exit::USAGE_ERROR,
            Self::ConfigError(_) | Self::Toml(_) | Self::TomlSer(_) => exit::USAGE_ERROR,
            _ => exit::FAILURE,
        }
    }

    /// Returns true if this error is likely transient (worth retrying)
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            Self::Io(_) | Self::ClipboardError(_) | Self::GitError(_)
        )
    }

    /// Returns a suggestion for how to fix or work around this error
    pub fn suggestion(&self) -> Option<&'static str> {
        match self {
            Self::FileNotFound(_) | Self::DirectoryNotFound(_) => {
                Some("Check that the path exists and is spelled correctly")
            }
            Self::PermissionDenied(_) => {
                Some("Check file permissions or run with appropriate privileges")
            }
            Self::OutputExists(_) => Some("Use --force to overwrite the existing file"),
            Self::BinaryFile(_) => Some("Binary files are automatically skipped"),
            Self::FileTooLarge { .. } => Some("Use --max-size to adjust the file size limit"),
            Self::InvalidPattern { .. } | Self::Pattern(_) => {
                Some("Check that the pattern follows gitignore syntax")
            }
            Self::EncodingError { .. } => Some("File may be binary or use an unsupported encoding"),
            Self::GitError(_) => Some("Ensure you're in a git repository or use --no-gitignore"),
            Self::ConfigError(_) | Self::Toml(_) => {
                Some("Check your .pctx.toml file for syntax errors")
            }
            Self::ClipboardError(_) => Some(Self::clipboard_suggestion()),
            Self::Io(_) => Some("This may be a temporary issue; try again"),
            _ => None,
        }
    }

    /// Platform-specific clipboard error suggestion
    fn clipboard_suggestion() -> &'static str {
        #[cfg(target_os = "linux")]
        {
            "Clipboard access may require a display server (X11/Wayland) on Linux"
        }
        #[cfg(target_os = "macos")]
        {
            "Check that the application has permission to access the clipboard"
        }
        #[cfg(target_os = "windows")]
        {
            "Clipboard may be locked by another application"
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            "Clipboard access failed; try writing to a file instead"
        }
    }

    /// Returns structured context about the input that caused this error
    pub fn input_context(&self) -> Option<serde_json::Value> {
        match self {
            Self::FileNotFound(p)
            | Self::DirectoryNotFound(p)
            | Self::PermissionDenied(p)
            | Self::OutputExists(p)
            | Self::BinaryFile(p) => Some(serde_json::json!({
                "path": p.to_string_lossy()
            })),
            Self::FileTooLarge { path, size, max } => Some(serde_json::json!({
                "path": path.to_string_lossy(),
                "size_bytes": size,
                "max_bytes": max
            })),
            Self::InvalidPattern { pattern, reason } => Some(serde_json::json!({
                "pattern": pattern,
                "reason": reason
            })),
            Self::EncodingError { path, reason } => Some(serde_json::json!({
                "path": path.to_string_lossy(),
                "reason": reason
            })),
            _ => None,
        }
    }
}
