//! Gitignore-style pattern matching.
//!
//! This module provides pattern matching similar to gitignore, with some limitations:
//! - Negation patterns (starting with `!`) are not supported
//! - Character classes `[abc]` depend on glob crate support
//! - Some edge cases with `**/` patterns may differ from git behavior

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

/// Normalize path separators to forward slashes for consistent matching
fn normalize_separators(s: &str) -> String {
    s.replace('\\', "/")
}

/// Check if any accumulated prefix of the path matches the pattern.
///
/// For a path like `a/b/c/file.txt`, this tests:
///   `a`, `a/b`, `a/b/c`
/// (i.e. every proper prefix, not the full path itself).
///
/// This implements gitignore directory semantics: matching a directory name
/// means everything inside it is also matched.
fn matches_any_prefix(path: &Path, pattern: &Pattern) -> bool {
    let normalized = normalize_separators(&path.to_string_lossy());
    let parts: Vec<&str> = normalized.split('/').collect();

    // Check all prefixes except the full path (caller already checks that)
    for i in 1..parts.len() {
        let prefix = parts[..i].join("/");
        if pattern.matches(&prefix) {
            return true;
        }
    }

    false
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
        let path_str = normalize_separators(&path.to_string_lossy());

        for pattern in &self.excludes {
            if pattern.anchored {
                if pattern.pattern.matches(&path_str) {
                    return true;
                }
                // Check path prefixes for directory matching
                if matches_any_prefix(path, &pattern.pattern) {
                    return true;
                }
            } else {
                // Check each individual component of the path (handles simple
                // patterns like "node_modules" matching any component anywhere)
                for component in path.components() {
                    let component_str = component.as_os_str().to_string_lossy();
                    if pattern.pattern.matches(&component_str) {
                        return true;
                    }
                }

                // Check full normalized path
                if pattern.pattern.matches(&path_str) {
                    return true;
                }

                // Also check just the filename
                if let Some(filename) = path.file_name() {
                    let filename_str = normalize_separators(&filename.to_string_lossy());
                    if pattern.pattern.matches(&filename_str) {
                        return true;
                    }
                }

                // Check accumulated path prefixes for directory matching
                // (handles multi-component patterns like "models/huggingface_cache")
                if matches_any_prefix(path, &pattern.pattern) {
                    return true;
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

        let path_str = normalize_separators(&path.to_string_lossy());

        for pattern in &self.includes {
            if pattern.anchored {
                if pattern.pattern.matches(&path_str) {
                    return true;
                }
                if matches_any_prefix(path, &pattern.pattern) {
                    return true;
                }
            } else {
                if pattern.pattern.matches(&path_str) {
                    return true;
                }
                // Also check just the filename
                if let Some(filename) = path.file_name() {
                    let filename_str = normalize_separators(&filename.to_string_lossy());
                    if pattern.pattern.matches(&filename_str) {
                        return true;
                    }
                }

                // Check each individual component (handles simple patterns)
                for component in path.components() {
                    let component_str = component.as_os_str().to_string_lossy();
                    if pattern.pattern.matches(&component_str) {
                        return true;
                    }
                }

                // Check accumulated path prefixes for directory matching
                if matches_any_prefix(path, &pattern.pattern) {
                    return true;
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

    // Handle negation (not supported - warn user)
    if trimmed.starts_with('!') {
        eprintln!(
            "Warning: negation patterns are not supported, ignoring: {}",
            trimmed
        );
        return None;
    }

    let is_dir_pattern = trimmed.ends_with('/');
    let clean = trimmed.trim_end_matches('/');

    // Handle root-anchored patterns (starting with /)
    let (anchored, pattern_body) = if let Some(stripped) = clean.strip_prefix('/') {
        (true, stripped)
    } else {
        (false, clean)
    };

    // Normalize separators in the pattern itself
    let pattern_body = normalize_separators(pattern_body);

    // Convert gitignore patterns to glob patterns
    let glob_pattern = if anchored {
        // Anchored patterns match from root only
        // Per gitignore spec: / at start anchors to directory where .gitignore is
        pattern_body.to_string()
    } else if pattern_body.contains('/') && !pattern_body.starts_with("**") {
        // Pattern with path separator but not anchored - match anywhere in tree
        // Per gitignore spec: patterns without leading / match at any level
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

    #[test]
    fn test_double_star_pattern() {
        let matcher = PatternMatcher::new(&["**/test/**".to_string()], &[]);
        assert!(matcher.is_excluded(&PathBuf::from("src/test/file.rs")));
        assert!(matcher.is_excluded(&PathBuf::from("test/file.rs")));
    }

    #[test]
    fn test_extension_with_path() {
        let matcher = PatternMatcher::new(&["**/*.test.ts".to_string()], &[]);
        assert!(matcher.is_excluded(&PathBuf::from("src/app.test.ts")));
        assert!(matcher.is_excluded(&PathBuf::from("app.test.ts")));
        assert!(!matcher.is_excluded(&PathBuf::from("app.ts")));
    }

    #[test]
    fn test_multi_component_directory_exclude() {
        // Excluding a path like "src/config" should exclude everything inside it.
        let matcher = PatternMatcher::new(&["src/config".to_string()], &[]);

        // The directory itself
        assert!(matcher.is_excluded(&PathBuf::from("src/config")));

        // Files inside the directory
        assert!(matcher.is_excluded(&PathBuf::from("src/config/mod.rs")));
        assert!(matcher.is_excluded(&PathBuf::from("src/config/defaults.rs")));
        assert!(matcher.is_excluded(&PathBuf::from("src/config/file.rs")));

        // Files NOT inside the directory should not be excluded
        assert!(!matcher.is_excluded(&PathBuf::from("src/main.rs")));
        assert!(!matcher.is_excluded(&PathBuf::from("src/content/mod.rs")));
        assert!(!matcher.is_excluded(&PathBuf::from("README.md")));
    }

    #[test]
    fn test_multi_component_directory_exclude_with_trailing_slash() {
        let matcher = PatternMatcher::new(&["src/output/".to_string()], &[]);
        assert!(matcher.is_excluded(&PathBuf::from("src/output/formatter.rs")));
        assert!(matcher.is_excluded(&PathBuf::from("src/output/json_types.rs")));
        assert!(!matcher.is_excluded(&PathBuf::from("src/scanner/mod.rs")));
    }

    #[test]
    fn test_multi_component_anchored_directory_exclude() {
        let matcher = PatternMatcher::new(&["/src/filter".to_string()], &[]);
        assert!(matcher.is_excluded(&PathBuf::from("src/filter/binary.rs")));
        assert!(matcher.is_excluded(&PathBuf::from("src/filter/patterns.rs")));
        // Anchored: should not match if nested under another prefix
        assert!(!matcher.is_excluded(&PathBuf::from("other/src/filter/binary.rs")));
    }

    #[test]
    fn test_multi_component_include() {
        let matcher = PatternMatcher::new(&[], &["src/output".to_string()]);
        assert!(matcher.is_included(&PathBuf::from("src/output/formatter.rs")));
        assert!(matcher.is_included(&PathBuf::from("src/output/tree.rs")));
        assert!(!matcher.is_included(&PathBuf::from("tests/integration_test.rs")));
    }

    #[test]
    fn test_backslash_separator_excluded() {
        // Simulate Windows-style paths
        let matcher = PatternMatcher::new(&["src/config".to_string()], &[]);
        assert!(matcher.is_excluded(&PathBuf::from("src\\config\\defaults.rs")));
    }
}
