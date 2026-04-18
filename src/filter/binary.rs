//! Binary file detection.

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

/// Sample size for binary detection (8KB)
const SAMPLE_SIZE: usize = 8192;

/// Null byte threshold - if more than this percentage of bytes are null,
/// consider the file binary
const NULL_THRESHOLD: f64 = 0.1;

/// Non-text byte threshold - if more than this percentage of bytes are
/// non-text control characters, consider the file binary
const NON_TEXT_THRESHOLD: f64 = 0.3;

/// Result of binary detection
#[derive(Debug)]
pub enum BinaryCheckResult {
    /// File is text (safe to read as UTF-8)
    Text,
    /// File appears to be binary
    Binary,
    /// Could not check (IO error)
    Error(io::Error),
}

/// Check if a file appears to be binary (not text)
///
/// Uses a heuristic approach: reads a sample of the file and checks for
/// null bytes and control characters that typically indicate binary content.
///
/// Returns a Result to properly propagate permission errors.
pub fn check_binary(path: &Path) -> BinaryCheckResult {
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(e) => return BinaryCheckResult::Error(e),
    };

    let mut buffer = vec![0u8; SAMPLE_SIZE];
    let bytes_read = match file.read(&mut buffer) {
        Ok(n) => n,
        Err(e) => return BinaryCheckResult::Error(e),
    };

    if bytes_read == 0 {
        return BinaryCheckResult::Text; // Empty files are considered text
    }

    buffer.truncate(bytes_read);

    // Count null bytes
    let null_count = buffer.iter().filter(|&&b| b == 0).count();
    let null_ratio = null_count as f64 / bytes_read as f64;

    if null_ratio > NULL_THRESHOLD {
        return BinaryCheckResult::Binary;
    }

    // Check for common binary signatures
    if has_binary_signature(&buffer) {
        return BinaryCheckResult::Binary;
    }

    // Check for high concentration of non-text bytes
    let non_text_count = buffer
        .iter()
        .filter(|&&b| {
            // Non-text bytes: control chars except common whitespace
            b < 0x09 || (b > 0x0D && b < 0x20 && b != 0x1B)
        })
        .count();

    let non_text_ratio = non_text_count as f64 / bytes_read as f64;
    if non_text_ratio > NON_TEXT_THRESHOLD {
        return BinaryCheckResult::Binary;
    }

    BinaryCheckResult::Text
}

/// Legacy function for backward compatibility - returns true if binary or error
pub fn is_binary(path: &Path) -> bool {
    matches!(
        check_binary(path),
        BinaryCheckResult::Binary | BinaryCheckResult::Error(_)
    )
}

/// Check for common binary file signatures (magic bytes)
fn has_binary_signature(buffer: &[u8]) -> bool {
    if buffer.len() < 4 {
        return false;
    }

    let signatures: &[&[u8]] = &[
        // Images
        &[0x89, 0x50, 0x4E, 0x47], // PNG
        &[0xFF, 0xD8, 0xFF],       // JPEG
        &[0x47, 0x49, 0x46, 0x38], // GIF
        &[0x42, 0x4D],             // BMP
        // Archives
        &[0x50, 0x4B, 0x03, 0x04], // ZIP/JAR/DOCX
        &[0x1F, 0x8B],             // GZIP
        &[0x52, 0x61, 0x72, 0x21], // RAR
        &[0x37, 0x7A, 0xBC, 0xAF], // 7Z
        // Executables
        &[0x7F, 0x45, 0x4C, 0x46], // ELF
        &[0x4D, 0x5A],             // DOS/PE executable
        &[0xFE, 0xED, 0xFA, 0xCE], // Mach-O 32-bit
        &[0xFE, 0xED, 0xFA, 0xCF], // Mach-O 64-bit
        &[0xCA, 0xFE, 0xBA, 0xBE], // Java class / Mach-O fat
        // Documents
        &[0x25, 0x50, 0x44, 0x46], // PDF
        // Audio/Video
        &[0x49, 0x44, 0x33],       // MP3 with ID3
        &[0x00, 0x00, 0x00],       // MP4/MOV (partial)
        // Database
        &[0x53, 0x51, 0x4C, 0x69], // SQLite
    ];

    for sig in signatures {
        if buffer.starts_with(sig) {
            return true;
        }
    }

    false
}

/// Common text file extensions
const TEXT_EXTENSIONS: &[&str] = &[
    // Documentation
    "txt", "md", "markdown", "rst", "adoc", "asciidoc", "org", "tex",
    // Programming languages
    "rs", "go", "py", "pyi", "js", "mjs", "cjs", "ts", "mts", "cts", "jsx", "tsx", "java", "kt",
    "kts", "scala", "groovy", "gradle", "c", "h", "cpp", "cc", "cxx", "hpp", "hxx", "hh", "cs",
    "fs", "fsx", "vb", "rb", "rake", "gemspec", "php", "phtml", "pl", "pm", "pod", "t", "swift",
    "m", "mm", "lua", "vim", "el", "lisp", "cl", "clj", "cljs", "cljc", "edn", "hs", "lhs", "elm",
    "purs", "erl", "hrl", "ex", "exs", "ml", "mli", "mll", "mly", "r", "R", "rmd", "Rmd", "jl",
    "dart", "v", "sv", "svh", "vhd", "vhdl", "zig", "nim", "cr", "d",
    // Shell
    "sh", "bash", "zsh", "fish", "ksh", "csh", "tcsh", "ps1", "psm1", "psd1", "bat", "cmd",
    // Web
    "html", "htm", "xhtml", "css", "scss", "sass", "less", "styl", "vue", "svelte", "astro",
    // Data & Config
    "json", "jsonc", "json5", "yaml", "yml", "toml", "ini", "cfg", "conf", "xml", "xsl", "xslt",
    "xsd", "dtd", "svg", "csv", "tsv", // Query
    "sql", "graphql", "gql", "prisma",
    // Build & CI
    "dockerfile", "containerfile", "makefile", "mk", "cmake", "jenkinsfile", "vagrantfile",
    "rakefile", "gemfile", "podfile",
    // Config files (no extension patterns)
    "gitignore", "gitattributes", "gitmodules", "dockerignore", "eslintignore", "prettierignore",
    "editorconfig", "browserslistrc", "env", "envrc", "env.example", // Infra
    "tf", "tfvars", "hcl", "proto", "thrift", "avsc", "avdl", // Lock files that are text
    "lock",
];

/// Check if a file has a known text extension
pub fn has_text_extension(path: &Path) -> bool {
    // Handle files with no extension (like Makefile, Dockerfile)
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    // Check filename directly for special files
    let special_files = [
        "makefile",
        "dockerfile",
        "containerfile",
        "jenkinsfile",
        "vagrantfile",
        "rakefile",
        "gemfile",
        "podfile",
        "cmakelists.txt",
        "build.gradle",
        "settings.gradle",
        "pom.xml",
        "cargo.toml",
        "go.mod",
        "go.sum",
        "package.json",
        "tsconfig.json",
        "license",
        "license.md",
        "license.txt",
        "readme",
        "changelog",
        "authors",
        "contributors",
    ];

    if special_files.contains(&filename.as_str()) {
        return true;
    }

    // Check extension
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| TEXT_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_empty_file_is_text() {
        let file = NamedTempFile::new().unwrap();
        assert!(matches!(check_binary(file.path()), BinaryCheckResult::Text));
    }

    #[test]
    fn test_text_file() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "Hello, world!").unwrap();
        writeln!(file, "This is a text file.").unwrap();
        assert!(matches!(check_binary(file.path()), BinaryCheckResult::Text));
    }

    #[test]
    fn test_binary_with_nulls() {
        let mut file = NamedTempFile::new().unwrap();
        // Write mostly null bytes
        file.write_all(&[0u8; 100]).unwrap();
        assert!(matches!(
            check_binary(file.path()),
            BinaryCheckResult::Binary
        ));
    }

    #[test]
    fn test_png_signature() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A])
            .unwrap();
        assert!(matches!(
            check_binary(file.path()),
            BinaryCheckResult::Binary
        ));
    }
}