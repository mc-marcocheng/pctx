//! Exit codes for the CLI application.
//!
//! These are part of the API contract and should be documented for users.
//! Agents rely on these codes to determine program flow.

/// Exit codes - THESE ARE PART OF THE PUBLIC API CONTRACT
///
/// When modifying these codes, update:
/// 1. The --help text in cli.rs
/// 2. The README.md documentation
/// 3. Any dependent scripts or tools
pub mod exit {
    /// Operation completed successfully
    pub const SUCCESS: i32 = 0;

    /// General/unspecified failure
    pub const FAILURE: i32 = 1;

    /// Usage error (invalid arguments, bad flag combinations)
    pub const USAGE_ERROR: i32 = 2;

    /// Resource not found (file, directory, config file)
    pub const NOT_FOUND: i32 = 3;

    /// Permission denied (cannot read file or directory)
    pub const PERMISSION_DENIED: i32 = 4;

    /// Conflict (e.g., output file exists without --force)
    pub const CONFLICT: i32 = 5;

    /// No files matched the specified filters
    pub const NO_MATCH: i32 = 6;

    /// Partial success (some files processed, some failed)
    pub const PARTIAL: i32 = 7;
}