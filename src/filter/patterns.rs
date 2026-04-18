//! Gitignore-style pattern matching.

use glob::Pattern;
use std::path::Path;

/// Pattern matcher for include/exclude filtering
pub struct PatternMatcher {
    excludes: Vec<CompiledPattern>,
    includes: Vec<CompiledPattern>,
}

struct CompiledPattern {
    pattern: Pattern,
    #[allow(dead_code)]
    original: String,
    /// Whether this pattern was anchored to root (started with /)
    anchored: bool,
    #[allow(dead_code)]
    is_dir_pattern: bool,
}

impl PatternMatcher {
    /// Create a new pattern matcher
    pub fn new(exclude_patterns: &[String], include_patterns: &[String]) -> Self {
        let excludes = exclude_patterns
            .iter()
            .filter_map(|p| compile_pattern(p))
            .collect();

        let includes = include_patterns
            .iter()
            .filter_map(|p| compile_pattern(p))
            .collect();

        Self { excludes, includes }
    }

    /// Check if a path should be excluded
    pub fn is_excluded(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        for pattern in &self.excludes {
            // For anchored patterns, only match from root
            if pattern.anchored {
                if pattern.pattern.matches(&path_str) {
                    return true;
                }
            } else {
                // Check each component of the path
                for component in path.components() {
                    let component_str = component.as_os_str().to_string_lossy();
                    if pattern.pattern.matches(&component_str) {
                        return true;
                    }
                }

                // Check full path
                if pattern.pattern.matches(&path_str) {
                    return true;
                }

                // Also check just the filename
                if let Some(filename) = path.file_name() {
                    if pattern.pattern.matches(&filename.to_string_lossy()) {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Check if a path should be included
    ///
    /// Returns true if:
    /// - No include patterns are specified, or
    /// - The path matches at least one include pattern
    pub fn is_included(&self, path: &Path) -> bool {
        if self.includes.is_empty() {
            return true;
        }

        let path_str = path.to_string_lossy();

        for pattern in &self.includes {
            if pattern.anchored {
                if pattern.pattern.matches(&path_str) {
                    return true;
                }
            } else {
                if pattern.pattern.matches(&path_str) {
                    return true;
                }
                // Also check just the filename
                if let Some(filename) = path.file_name() {
                    if pattern.pattern.matches(&filename.to_string_lossy()) {
                        return true;
                    }
                }
            }
        }

        false
    }
}

/// Compile a gitignore-style pattern to a glob pattern
fn compile_pattern(pattern: &str) -> Option<CompiledPattern> {
    let trimmed = pattern.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }

    // Handle negation (not fully supported yet, just skip)
    if trimmed.starts_with('!') {
        // TODO: Support negation patterns
        return None;
    }

    let is_dir_pattern = trimmed.ends_with('/');
    let clean = trimmed.trim_end_matches('/');

    // Handle root-anchored patterns (starting with /)
    let (anchored, pattern_body) = if clean.starts_with('/') {
        (true, &clean[1..])
    } else {
        (false, clean)
    };

    // Convert gitignore patterns to glob patterns
    let glob_pattern = if anchored {
        // Anchored patterns match from root only
        pattern_body.to_string()
    } else if clean.contains('/') {
        // Pattern with path separator but not anchored - match anywhere
        format!("**/{}", pattern_body)
    } else {
        // Simple pattern - match the name anywhere
        pattern_body.to_string()
    };

    Pattern::new(&glob_pattern).ok().map(|p| CompiledPattern {
        pattern: p,
        original: pattern.to_string(),
        anchored,
        is_dir_pattern,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_simple_exclude() {
        let matcher = PatternMatcher::new(&["*.log".to_string()], &[]);
        assert!(matcher.is_excluded(&PathBuf::from("app.log")));
        assert!(matcher.is_excluded(&PathBuf::from("logs/app.log")));
        assert!(!matcher.is_excluded(&PathBuf::from("app.txt")));
    }

    #[test]
    fn test_directory_exclude() {
        let matcher = PatternMatcher::new(&["node_modules".to_string()], &[]);
        assert!(matcher.is_excluded(&PathBuf::from("node_modules/package/index.js")));
        assert!(matcher.is_excluded(&PathBuf::from("src/node_modules/foo.js")));
    }

    #[test]
    fn test_anchored_pattern() {
        let matcher = PatternMatcher::new(&["/src/test".to_string()], &[]);
        assert!(matcher.is_excluded(&PathBuf::from("src/test")));
        // Note: anchored patterns in our implementation match the exact path
        assert!(!matcher.is_excluded(&PathBuf::from("foo/src/test")));
    }

    #[test]
    fn test_include_patterns() {
        let matcher = PatternMatcher::new(&[], &["*.rs".to_string()]);
        assert!(matcher.is_included(&PathBuf::from("main.rs")));
        assert!(matcher.is_included(&PathBuf::from("src/lib.rs")));
        assert!(!matcher.is_included(&PathBuf::from("main.py")));
    }

    #[test]
    fn test_empty_include_allows_all() {
        let matcher = PatternMatcher::new(&[], &[]);
        assert!(matcher.is_included(&PathBuf::from("anything.txt")));
    }

    #[test]
    fn test_comment_pattern_ignored() {
        let result = compile_pattern("# this is a comment");
        assert!(result.is_none());
    }

    #[test]
    fn test_empty_pattern_ignored() {
        let result = compile_pattern("   ");
        assert!(result.is_none());
    }
}