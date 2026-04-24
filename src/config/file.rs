//! Configuration file handling (.pctx.toml)

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::PctxError;

/// Configuration loaded from .pctx.toml file
#[derive(Debug, Deserialize, Serialize, Default, Clone)]
pub struct FileConfig {
    /// Additional patterns to exclude
    #[serde(default)]
    pub exclude: Vec<String>,

    /// Patterns to include (if non-empty, only matching files are included)
    #[serde(default)]
    pub include: Vec<String>,

    /// Override: max lines before truncation
    #[serde(default)]
    pub max_lines: Option<usize>,

    /// Override: lines to show at start when truncating
    #[serde(default)]
    pub head_lines: Option<usize>,

    /// Override: lines to show at end when truncating
    #[serde(default)]
    pub tail_lines: Option<usize>,

    /// Override: max line length before truncation
    #[serde(default)]
    pub max_line_length: Option<usize>,

    /// Override: chars to show at line start when truncating
    #[serde(default)]
    pub head_chars: Option<usize>,

    /// Override: chars to show at line end when truncating
    #[serde(default)]
    pub tail_chars: Option<usize>,
}

/// Search for .pctx.toml in current directory and parents
pub fn find_config_file() -> Option<PathBuf> {
    let mut current = std::env::current_dir().ok()?;

    loop {
        let config_path = current.join(".pctx.toml");
        if config_path.exists() && config_path.is_file() {
            return Some(config_path);
        }

        if !current.pop() {
            break;
        }
    }

    None
}

/// Find and load configuration file
pub fn find_and_load() -> Result<FileConfig, PctxError> {
    match find_config_file() {
        Some(path) => load_config(&path),
        None => Ok(FileConfig::default()),
    }
}

/// Load configuration from a specific path
pub fn load_config(path: &Path) -> Result<FileConfig, PctxError> {
    let contents = fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            PctxError::FileNotFound(path.to_path_buf())
        } else if e.kind() == std::io::ErrorKind::PermissionDenied {
            PctxError::PermissionDenied(path.to_path_buf())
        } else {
            PctxError::Io(e)
        }
    })?;

    let config: FileConfig = toml::from_str(&contents)?;
    Ok(config)
}

/// Write a template configuration file (atomic write with force option)
pub fn write_template(path: &Path, force: bool) -> Result<(), PctxError> {
    let template = r#"# pctx configuration file
# See https://github.com/mc-marcocheng/pctx for documentation

# Additional patterns to exclude (gitignore-style)
exclude = [
    "*.test.ts",
    "*.spec.js",
    "__tests__",
    "*.snap",
]

# Only include files matching these patterns (empty = include all)
# include = ["*.rs", "*.toml"]

# Truncation settings for long files
# max_lines = 500      # Max lines per file (0 = no limit)
# head_lines = 20      # Lines to keep at start when truncating
# tail_lines = 10      # Lines to keep at end when truncating

# Truncation settings for long lines
# max_line_length = 500  # Max chars per line (0 = no limit)
# head_chars = 200       # Chars to keep at line start
# tail_chars = 100       # Chars to keep at line end
"#;

    let mut options = OpenOptions::new();
    options.write(true);

    if force {
        options.create(true).truncate(true);
    } else {
        options.create_new(true);
    }

    let mut file = options.open(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::AlreadyExists {
            PctxError::OutputExists(path.to_path_buf())
        } else {
            PctxError::Io(e)
        }
    })?;

    file.write_all(template.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn load_config_round_trip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(".pctx.toml");
        fs::write(
            &path,
            r#"exclude = ["*.foo", "*.bar"]
include = ["*.rs"]
max_lines = 123
head_chars = 45
"#,
        )
        .unwrap();

        let cfg = load_config(&path).unwrap();
        assert_eq!(cfg.exclude, vec!["*.foo", "*.bar"]);
        assert_eq!(cfg.include, vec!["*.rs"]);
        assert_eq!(cfg.max_lines, Some(123));
        assert_eq!(cfg.head_chars, Some(45));
        // Unspecified fields fall back to None
        assert_eq!(cfg.tail_lines, None);
    }

    #[test]
    fn load_config_empty_file_yields_defaults() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(".pctx.toml");
        fs::write(&path, "").unwrap();

        let cfg = load_config(&path).unwrap();
        assert!(cfg.exclude.is_empty());
        assert!(cfg.include.is_empty());
        assert_eq!(cfg.max_lines, None);
    }

    #[test]
    fn load_config_missing_file_returns_file_not_found() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("does-not-exist.toml");

        match load_config(&path) {
            Err(PctxError::FileNotFound(p)) => assert_eq!(p, path),
            other => panic!("expected FileNotFound, got {other:?}"),
        }
    }

    #[test]
    fn load_config_malformed_toml_returns_toml_error() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(".pctx.toml");
        fs::write(&path, "this is = not [valid toml").unwrap();

        match load_config(&path) {
            Err(PctxError::Toml(_)) => {}
            other => panic!("expected Toml error, got {other:?}"),
        }
    }

    #[test]
    fn write_template_refuses_existing_without_force() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(".pctx.toml");
        fs::write(&path, "pre-existing").unwrap();

        match write_template(&path, false) {
            Err(PctxError::OutputExists(p)) => assert_eq!(p, path),
            other => panic!("expected OutputExists, got {other:?}"),
        }
        // File untouched
        assert_eq!(fs::read_to_string(&path).unwrap(), "pre-existing");
    }

    #[test]
    fn write_template_force_overwrites() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(".pctx.toml");
        fs::write(&path, "pre-existing").unwrap();

        write_template(&path, true).unwrap();
        let written = fs::read_to_string(&path).unwrap();
        assert!(written.contains("pctx configuration"));
        assert!(!written.contains("pre-existing"));
    }

    #[test]
    fn write_template_creates_when_absent() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(".pctx.toml");
        assert!(!path.exists());

        write_template(&path, false).unwrap();
        assert!(path.exists());
        let cfg = load_config(&path).unwrap();
        // The template's commented-out overrides should still parse (none active)
        assert!(!cfg.exclude.is_empty());
    }
}
