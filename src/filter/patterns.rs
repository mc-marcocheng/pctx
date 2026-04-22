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
    /// Whether this pattern was anchored to root (started with /)
    anchored: bool,
    /// Whether this pattern ended with a trailing slash (matches only directories)
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

impl CompiledPattern {
    /// Evaluate if this pattern matches the given file path
    fn matches_path(&self, path: &Path, path_str: &str) -> bool {
        if self.anchored {
            // Exact match (must not be a directory-only pattern since path is a file)
            if !self.is_dir_pattern && self.pattern.matches(path_str) {
                return true;
            }
            // Check path prefixes for directory matching
            if matches_any_prefix(path, &self.pattern) {
                return true;
            }
        } else {
            // Check full normalized path
            if !self.is_dir_pattern && self.pattern.matches(path_str) {
                return true;
            }

            // Also check just the filename
            if !self.is_dir_pattern {
                if let Some(filename) = path.file_name() {
                    let filename_str = normalize_separators(&filename.to_string_lossy());
                    if self.pattern.matches(&filename_str) {
                        return true;
                    }
                }
            }

            // Check each individual component of the path (handles simple
            // patterns like "node_modules" matching any component anywhere)
            let components: Vec<_> = path.components().collect();
            for (i, component) in components.iter().enumerate() {
                let is_last = i == components.len() - 1;
                // If it's a directory pattern, it cannot match the file itself (the last component)
                if self.is_dir_pattern && is_last {
                    continue;
                }

                let component_str = component.as_os_str().to_string_lossy();
                if self.pattern.matches(&component_str) {
                    return true;
                }
            }

            // Check accumulated path prefixes for directory matching
            // (handles multi-component patterns like "models/huggingface_cache")
            if matches_any_prefix(path, &self.pattern) {
                return true;
            }
        }

        false
    }
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
        self.excludes
            .iter()
            .any(|p| p.matches_path(path, &path_str))
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
        self.includes
            .iter()
            .any(|p| p.matches_path(path, &path_str))
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

    // Normalize backslashes to forward slashes
    let normalized = normalize_separators(trimmed);

    // Detect path-like input (leading "./") and strip it with a warning.
    // This commonly occurs when users tab-complete a path on the command line
    // and pass it to --include/--exclude, which expect gitignore-style patterns.
    let cleaned = if let Some(stripped) = normalized.strip_prefix("./") {
        eprintln!(
            "Warning: '{}' looks like a file path, not a pattern. \
             Stripping leading \"./\". Consider using a positional argument instead: \
             pctx {}",
            trimmed, stripped
        );
        stripped
    } else {
        normalized.as_str()
    };

    let is_dir_pattern = cleaned.ends_with('/');
    let clean = cleaned.trim_end_matches('/');

    // Handle root-anchored patterns (starting with /)
    let (anchored, pattern_body) = if let Some(stripped) = clean.strip_prefix('/') {
        (true, stripped)
    } else {
        (false, clean)
    };

    if pattern_body.is_empty() {
        return None;
    }

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

    #[test]
    fn test_trailing_slash_semantics() {
        // "output/" should only match directories, not files named "output"
        let matcher = PatternMatcher::new(&["output/".to_string()], &[]);

        // Match files INSIDE output directory
        assert!(matcher.is_excluded(&PathBuf::from("output/file.txt")));
        assert!(matcher.is_excluded(&PathBuf::from("src/output/file.txt")));

        // Should NOT match a file named "output"
        assert!(!matcher.is_excluded(&PathBuf::from("output")));
        assert!(!matcher.is_excluded(&PathBuf::from("src/output")));

        // Without trailing slash, it matches both
        let matcher2 = PatternMatcher::new(&["output".to_string()], &[]);
        assert!(matcher2.is_excluded(&PathBuf::from("output/file.txt")));
        assert!(matcher2.is_excluded(&PathBuf::from("output")));
    }

    // --- Tests for backslash normalization ordering fix ---

    #[test]
    fn test_trailing_backslash_detected_as_dir_pattern() {
        // On Windows, tab-completion produces trailing backslashes.
        // After normalization this must be recognized as a directory pattern.
        let matcher = PatternMatcher::new(&[r"output\".to_string()], &[]);

        // Files inside the directory
        assert!(matcher.is_excluded(&PathBuf::from("output/file.txt")));
        assert!(matcher.is_excluded(&PathBuf::from("src/output/file.txt")));

        // Directory-only pattern must NOT match a bare file named "output"
        assert!(!matcher.is_excluded(&PathBuf::from("output")));
    }

    #[test]
    fn test_backslash_multi_component_include() {
        // Backslash-separated include pattern should work
        let matcher = PatternMatcher::new(&[], &[r"src\content".to_string()]);
        assert!(matcher.is_included(&PathBuf::from("src/content/reader.rs")));
        assert!(matcher.is_included(&PathBuf::from("src/content/truncator.rs")));
        assert!(!matcher.is_included(&PathBuf::from("src/filter/binary.rs")));
    }

    #[test]
    fn test_backslash_multi_component_dir_include() {
        // The original reported bug: .\path\to\dir\ should work
        let matcher = PatternMatcher::new(&[], &[r".\src\scanner\".to_string()]);
        assert!(matcher.is_included(&PathBuf::from("src/scanner/git.rs")));
        assert!(matcher.is_included(&PathBuf::from("src/scanner/walker.rs")));
        assert!(!matcher.is_included(&PathBuf::from("src/filter/binary.rs")));
    }

    // --- Tests for leading "./" handling ---

    #[test]
    fn test_dot_slash_prefix_stripped_include() {
        let matcher = PatternMatcher::new(&[], &["./src/config".to_string()]);
        assert!(matcher.is_included(&PathBuf::from("src/config/mod.rs")));
        assert!(!matcher.is_included(&PathBuf::from("tests/integration_test.rs")));
    }

    #[test]
    fn test_dot_slash_prefix_stripped_exclude() {
        let matcher = PatternMatcher::new(&["./node_modules".to_string()], &[]);
        assert!(matcher.is_excluded(&PathBuf::from("node_modules/pkg/index.js")));
    }

    #[test]
    fn test_dot_backslash_prefix_stripped() {
        // .\dir (Windows tab-completion) should work after normalization + stripping
        let matcher = PatternMatcher::new(&[r".\node_modules".to_string()], &[]);
        assert!(matcher.is_excluded(&PathBuf::from("node_modules/pkg/index.js")));
    }

    #[test]
    fn test_bare_dot_pattern_kept() {
        // "." alone is a valid glob that matches a literal "." component
        assert!(compile_pattern(".").is_some());
    }

    #[test]
    fn test_dot_slash_alone_ignored() {
        // "./" is just current dir, not a meaningful pattern
        assert!(compile_pattern("./").is_none());
    }

    #[test]
    fn test_dot_backslash_alone_ignored() {
        // ".\" normalized to "./" — same as above
        assert!(compile_pattern(r".\").is_none());
    }

    #[test]
    fn test_dot_slash_does_not_strip_from_middle() {
        // Only leading "./" is stripped. A pattern like "src/./config" is left as-is
        // (the user likely made a mistake and should see it fail or behave oddly).
        let compiled = compile_pattern("src/./config");
        assert!(compiled.is_some());
        // The glob will be "**/src/./config" which may or may not match depending
        // on glob crate behavior — but we don't silently rewrite it.
    }
}
