//! Default configuration values and exclusion patterns.

/// Default patterns to exclude from context generation.
///
/// These patterns match common directories and files that typically
/// should not be included in LLM context:
/// - Version control directories
/// - Dependencies and package directories
/// - Build outputs and caches
/// - IDE and editor files
/// - Environment and secret files
/// - Binary and media files
pub const DEFAULT_EXCLUDES: &[&str] = &[
    // Version control
    ".git",
    ".svn",
    ".hg",
    ".bzr",
    // Dependencies
    "node_modules",
    "vendor",
    "bower_components",
    ".pnpm",
    "jspm_packages",
    // Python
    "__pycache__",
    "*.pyc",
    "*.pyo",
    "*.pyd",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    ".tox",
    "*.egg-info",
    ".eggs",
    "venv",
    ".venv",
    "env",
    ".env",
    // Rust
    "target",
    // Go
    "go.sum",
    // Java/JVM
    "*.class",
    "*.jar",
    "*.war",
    "*.ear",
    // .NET
    "bin",
    "obj",
    "*.dll",
    "*.exe",
    // Build outputs
    "dist",
    "build",
    "out",
    "_build",
    ".build",
    // IDE and editors
    ".idea",
    ".vscode",
    "*.swp",
    "*.swo",
    "*~",
    ".project",
    ".classpath",
    ".settings",
    "*.sublime-workspace",
    "*.sublime-project",
    // Environment and secrets
    ".env.*",
    "*.local",
    ".secrets",
    ".secret",
    "secrets.yml",
    "secrets.yaml",
    "*.pem",
    "*.key",
    // Logs
    "*.log",
    "logs",
    "npm-debug.log*",
    "yarn-debug.log*",
    "yarn-error.log*",
    // OS files
    ".DS_Store",
    "Thumbs.db",
    "desktop.ini",
    "*.lnk",
    // Package locks (can be very large)
    "package-lock.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "bun.lockb",
    "Cargo.lock",
    "poetry.lock",
    "Gemfile.lock",
    "composer.lock",
    "Pipfile.lock",
    "uv.lock",
    "pdm.lock",
    "pubspec.lock",
    "mix.lock",
    "Package.resolved",
    "packages.lock.json",
    "Podfile.lock",
    "flake.lock",
    // Coverage and testing
    "coverage",
    ".coverage",
    "htmlcov",
    ".nyc_output",
    "*.lcov",
    // Caches
    ".cache",
    ".parcel-cache",
    ".turbo",
    ".next",
    ".nuxt",
    ".output",
    ".vercel",
    ".netlify",
    // Documentation builds
    "_site",
    ".docusaurus",
    // Temporary files
    "*.tmp",
    "*.temp",
    "*.bak",
    "*.backup",
    // Images and media (binary)
    "*.png",
    "*.jpg",
    "*.jpeg",
    "*.gif",
    "*.ico",
    "*.svg",
    "*.webp",
    "*.bmp",
    "*.tiff",
    "*.mp3",
    "*.mp4",
    "*.avi",
    "*.mov",
    "*.wmv",
    "*.flv",
    "*.webm",
    "*.wav",
    "*.ogg",
    "*.flac",
    // Documents (binary)
    "*.pdf",
    "*.doc",
    "*.docx",
    "*.xls",
    "*.xlsx",
    "*.ppt",
    "*.pptx",
    // Archives
    "*.zip",
    "*.tar",
    "*.gz",
    "*.rar",
    "*.7z",
    "*.bz2",
    "*.xz",
    // Fonts
    "*.woff",
    "*.woff2",
    "*.ttf",
    "*.eot",
    "*.otf",
    // Compiled/Binary
    "*.so",
    "*.dylib",
    "*.a",
    "*.o",
    "*.obj",
    "*.lib",
    // Database
    "*.sqlite",
    "*.sqlite3",
    "*.db",
    "*.mdb",
    // Maps and generated
    "*.map",
    "*.min.js",
    "*.min.css",
    "*.bundle.js",
    "*.chunk.js",
    // pctx output
    "llm_context.txt",
    "context.md",
    "context.txt",
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn default_excludes_non_empty_and_unique() {
        assert!(!DEFAULT_EXCLUDES.is_empty());
        let set: HashSet<&&str> = DEFAULT_EXCLUDES.iter().collect();
        assert_eq!(
            set.len(),
            DEFAULT_EXCLUDES.len(),
            "DEFAULT_EXCLUDES contains duplicates"
        );
    }

    #[test]
    fn default_excludes_contains_canonical_entries() {
        // Common expectations users and docs rely on.
        for expected in &[".git", "node_modules", "target", ".DS_Store"] {
            assert!(
                DEFAULT_EXCLUDES.contains(expected),
                "missing canonical default: {expected}"
            );
        }
    }
}
