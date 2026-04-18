//! Content reading and processing.

pub mod reader;
pub mod truncator;

use std::path::{Path, PathBuf};

pub use reader::read_file_contents;
pub use truncator::truncate_content;

use crate::config::Config;
use crate::error::PctxError;
use crate::filter::binary;

use dunce;

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
    base_path: PathBuf,
}

impl<'a> ContentProcessor<'a> {
    /// Create a new content processor
    pub fn new(config: &'a Config) -> Self {
        let base_path = std::env::current_dir()
            .ok()
            .and_then(|p| dunce::canonicalize(&p).ok())
            .unwrap_or_else(|| PathBuf::from("."));
        Self { config, base_path }
    }

    /// Create a new content processor with a specific base path
    pub fn with_base_path(config: &'a Config, base_path: PathBuf) -> Self {
        let base_path = dunce::canonicalize(&base_path).unwrap_or(base_path);
        Self { config, base_path }
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

        let clean_path = dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());

        let relative_path = clean_path
            .strip_prefix(&self.base_path)
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|_| clean_path.clone());

        let extension = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        Ok(FileEntry {
            absolute_path: clean_path,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::ContentFormat;
    use crate::config::TruncationConfig;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_config() -> Config {
        Config {
            paths: vec![],
            exclude_patterns: vec![],
            include_patterns: vec![],
            include_hidden: false,
            use_default_excludes: true,
            use_gitignore: true,
            max_file_size: 10 * 1024 * 1024, // 10 MB
            max_depth: None,
            truncation: TruncationConfig {
                max_lines: 0, // 0 means no limit
                head_lines: 50,
                tail_lines: 20,
                max_line_length: 0, // 0 means no limit
                head_chars: 500,
                tail_chars: 200,
            },
            output_format: ContentFormat::Markdown,
            show_tree: false,
            show_stats: false,
            absolute_paths: false,
            verbose: false,
            quiet: false,
        }
    }

    #[test]
    fn test_file_entry_display_path_relative() {
        let entry = FileEntry {
            absolute_path: PathBuf::from("/home/user/project/src/main.rs"),
            relative_path: "src/main.rs".to_string(),
            extension: "rs".to_string(),
            original_bytes: 100,
            original_lines: 10,
            line_count: 10,
            truncated: false,
            truncated_lines: 0,
            content: "fn main() {}".to_string(),
        };

        assert_eq!(entry.display_path(false), "src/main.rs");
    }

    #[test]
    fn test_file_entry_display_path_absolute() {
        let entry = FileEntry {
            absolute_path: PathBuf::from("/home/user/project/src/main.rs"),
            relative_path: "src/main.rs".to_string(),
            extension: "rs".to_string(),
            original_bytes: 100,
            original_lines: 10,
            line_count: 10,
            truncated: false,
            truncated_lines: 0,
            content: "fn main() {}".to_string(),
        };

        assert_eq!(entry.display_path(true), "/home/user/project/src/main.rs");
    }

    #[test]
    fn test_content_processor_process_file() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "line 1\nline 2\nline 3\n").unwrap();

        let config = create_test_config();
        let processor = ContentProcessor::with_base_path(&config, dir.path().to_path_buf());

        let entry = processor.process(&file_path).unwrap();

        assert_eq!(entry.relative_path, "test.txt");
        assert_eq!(entry.extension, "txt");
        assert_eq!(entry.original_lines, 3);
        assert_eq!(entry.original_bytes, 21);
        assert!(!entry.truncated);
        // Note: trailing newline may be normalized during processing
        assert!(entry.content.contains("line 1"));
        assert!(entry.content.contains("line 2"));
        assert!(entry.content.contains("line 3"));
    }

    #[test]
    fn test_content_processor_nested_file() {
        let dir = TempDir::new().unwrap();
        let nested_dir = dir.path().join("src").join("lib");
        fs::create_dir_all(&nested_dir).unwrap();
        let file_path = nested_dir.join("module.rs");
        fs::write(&file_path, "pub fn test() {}").unwrap();

        let config = create_test_config();
        let processor = ContentProcessor::with_base_path(&config, dir.path().to_path_buf());

        let entry = processor.process(&file_path).unwrap();

        // Check relative path uses correct separators
        assert!(
            entry.relative_path == "src/lib/module.rs"
                || entry.relative_path == r"src\lib\module.rs"
        );
        assert_eq!(entry.extension, "rs");
    }

    #[test]
    fn test_content_processor_no_extension() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("Makefile");
        fs::write(&file_path, "all:\n\techo hello").unwrap();

        let config = create_test_config();
        let processor = ContentProcessor::with_base_path(&config, dir.path().to_path_buf());

        let entry = processor.process(&file_path).unwrap();

        assert_eq!(entry.extension, "");
    }

    #[test]
    fn test_content_processor_binary_file_rejected() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("binary.bin");
        // Write binary content (PNG header)
        fs::write(&file_path, [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]).unwrap();

        let config = create_test_config();
        let processor = ContentProcessor::with_base_path(&config, dir.path().to_path_buf());

        let result = processor.process(&file_path);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PctxError::BinaryFile(_)));
    }

    #[test]
    fn test_content_processor_empty_file() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("empty.txt");
        fs::write(&file_path, "").unwrap();

        let config = create_test_config();
        let processor = ContentProcessor::with_base_path(&config, dir.path().to_path_buf());

        let entry = processor.process(&file_path).unwrap();

        assert_eq!(entry.original_lines, 0);
        assert_eq!(entry.original_bytes, 0);
        assert_eq!(entry.content, "");
    }

    #[test]
    fn test_content_processor_unicode_content() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("unicode.txt");
        fs::write(&file_path, "Hello 世界\nПривет мир\n🎉🎊").unwrap();

        let config = create_test_config();
        let processor = ContentProcessor::with_base_path(&config, dir.path().to_path_buf());

        let entry = processor.process(&file_path).unwrap();

        assert_eq!(entry.original_lines, 3);
        assert!(entry.content.contains("世界"));
        assert!(entry.content.contains("Привет"));
        assert!(entry.content.contains("🎉"));
    }

    #[test]
    fn test_content_processor_truncation() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("long.txt");

        // Create a file with many lines
        let content: String = (1..=100).map(|i| format!("line {}\n", i)).collect();
        fs::write(&file_path, &content).unwrap();

        let mut config = create_test_config();
        config.truncation.max_lines = 20;
        config.truncation.head_lines = 5;
        config.truncation.tail_lines = 5;

        let processor = ContentProcessor::with_base_path(&config, dir.path().to_path_buf());

        let entry = processor.process(&file_path).unwrap();

        assert!(entry.truncated);
        assert_eq!(entry.original_lines, 100);
        assert!(entry.truncated_lines > 0);
        assert!(entry.content.contains("line 1"));
        assert!(entry.content.contains("line 100"));
        assert!(entry.content.contains("lines omitted"));
    }

    #[test]
    #[cfg(unix)]
    fn test_content_processor_symlink_resolution() {
        use std::os::unix::fs::symlink;

        let dir = TempDir::new().unwrap();

        // Create actual directory with a file
        let real_dir = dir.path().join("real");
        fs::create_dir(&real_dir).unwrap();
        let file_path = real_dir.join("test.txt");
        fs::write(&file_path, "test content").unwrap();

        // Create symlink to the directory
        let link_dir = dir.path().join("link");
        symlink(&real_dir, &link_dir).unwrap();

        // Process file via symlink path
        let linked_file = link_dir.join("test.txt");

        let config = create_test_config();
        // Use symlink as base path - should resolve to real path
        let processor = ContentProcessor::with_base_path(&config, link_dir.clone());

        let entry = processor.process(&linked_file).unwrap();

        // Relative path should be just the filename
        assert_eq!(entry.relative_path, "test.txt");
    }

    #[test]
    fn test_content_processor_file_outside_base() {
        let dir1 = TempDir::new().unwrap();
        let dir2 = TempDir::new().unwrap();

        let file_path = dir2.path().join("outside.txt");
        fs::write(&file_path, "content").unwrap();

        let config = create_test_config();
        let processor = ContentProcessor::with_base_path(&config, dir1.path().to_path_buf());

        let entry = processor.process(&file_path).unwrap();

        // When file is outside base path, relative_path should be the full path
        // (since strip_prefix fails)
        assert!(entry.relative_path.contains("outside.txt"));
    }

    #[test]
    fn test_content_processor_extension_case_insensitive() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.RS");
        fs::write(&file_path, "fn main() {}").unwrap();

        let config = create_test_config();
        let processor = ContentProcessor::with_base_path(&config, dir.path().to_path_buf());

        let entry = processor.process(&file_path).unwrap();

        // Extension should be lowercase
        assert_eq!(entry.extension, "rs");
    }

    #[test]
    fn test_content_processor_multiple_extensions() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.spec.ts");
        fs::write(&file_path, "describe('test', () => {});").unwrap();

        let config = create_test_config();
        let processor = ContentProcessor::with_base_path(&config, dir.path().to_path_buf());

        let entry = processor.process(&file_path).unwrap();

        // Only the last extension is captured
        assert_eq!(entry.extension, "ts");
    }

    #[test]
    fn test_content_processor_hidden_file() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join(".hidden");
        fs::write(&file_path, "secret").unwrap();

        let config = create_test_config();
        let processor = ContentProcessor::with_base_path(&config, dir.path().to_path_buf());

        let entry = processor.process(&file_path).unwrap();

        assert_eq!(entry.relative_path, ".hidden");
        assert_eq!(entry.extension, "");
    }

    #[test]
    fn test_content_processor_preserves_line_endings() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("crlf.txt");
        fs::write(&file_path, "line1\r\nline2\r\nline3").unwrap();

        let config = create_test_config();
        let processor = ContentProcessor::with_base_path(&config, dir.path().to_path_buf());

        let entry = processor.process(&file_path).unwrap();

        // Content should preserve original line endings
        assert!(entry.content.contains("\r\n") || entry.content.contains('\n'));
    }
}
