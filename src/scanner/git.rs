//! Git-aware file scanning.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::Config;
use crate::error::PctxError;
use crate::filter::binary;

/// Check if a path is inside a git repository by walking up the directory tree.
pub fn is_inside_git_repo(path: &Path) -> bool {
    let mut current = if path.is_file() {
        match path.parent() {
            Some(p) => p.to_path_buf(),
            None => return false,
        }
    } else {
        path.to_path_buf()
    };

    loop {
        if current.join(".git").exists() {
            return true;
        }
        if !current.pop() {
            return false;
        }
    }
}

/// Scan a git repository using git ls-files
pub fn scan_git_repo(dir: &Path, config: &Config) -> Result<Vec<PathBuf>, PctxError> {
    let output = Command::new("git")
        .arg("ls-files")
        .arg("--cached")
        .arg("--others")
        .arg("--exclude-standard")
        .current_dir(dir)
        .output()
        .map_err(|e| PctxError::GitError(format!("Failed to run git: {}", e)))?;

    if !output.status.success() {
        return Err(PctxError::GitError(format!(
            "git ls-files failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut files = Vec::new();

    for line in stdout.lines() {
        if line.is_empty() {
            continue;
        }

        let path = dir.join(line);

        // Apply hidden filter
        if !config.include_hidden {
            let is_hidden = path
                .components()
                .any(|c| c.as_os_str().to_string_lossy().starts_with('.'));
            if is_hidden {
                continue;
            }
        }

        // Apply depth filter
        if let Some(max_depth) = config.max_depth {
            let depth = path
                .strip_prefix(dir)
                .map(|p| p.components().count())
                .unwrap_or(0);
            if depth > max_depth {
                continue;
            }
        }

        if path.is_file() {
            // Skip binary files early
            if binary::is_binary(&path) {
                continue;
            }
            files.push(path);
        }
    }

    Ok(files)
}
