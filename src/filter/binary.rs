//! Binary file detection.

use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Common binary file signatures (magic bytes)
const BINARY_SIGNATURES: &[&[u8]] = &[
    // Images
    &[0x89, 0x50, 0x4E, 0x47], // PNG
    &[0xFF, 0xD8, 0xFF],       // JPEG
    &[0x47, 0x49, 0x46, 0x38], // GIF
    &[0x42, 0x4D],             // BMP
    &[0x00, 0x00, 0x01, 0x00], // ICO
    &[0x52, 0x49, 0x46, 0x46], // WEBP (RIFF)
    // Archives
    &[0x50, 0x4B, 0x03, 0x04], // ZIP/JAR/DOCX/etc
    &[0x1F, 0x8B],             // GZIP
    &[0x42, 0x5A, 0x68],       // BZIP2
    &[0xFD, 0x37, 0x7A, 0x58], // XZ
    &[0x52, 0x61, 0x72, 0x21], // RAR
    &[0x37, 0x7A, 0xBC, 0xAF], // 7Z
    // Executables
    &[0x7F, 0x45, 0x4C, 0x46], // ELF
    &[0x4D, 0x5A],             // DOS/PE executable
    &[0xCF, 0xFA, 0xED, 0xFE], // Mach-O (32-bit)
    &[0xCE, 0xFA, 0xED, 0xFE], // Mach-O (64-bit)
    &[0xCA, 0xFE, 0xBA, 0xBE], // Java class / Mach-O fat
    // Documents (binary)
    &[0x25, 0x50, 0x44, 0x46], // PDF
    &[0xD0, 0xCF, 0x11, 0xE0], // MS Office (DOC, XLS, PPT)
    // Media
    &[0x49, 0x44, 0x33],       // MP3 (ID3)
    &[0xFF, 0xFB],             // MP3 (no ID3)
    &[0x00, 0x00, 0x00],       // MP4/MOV (often starts with ftyp at offset 4)
    &[0x4F, 0x67, 0x67, 0x53], // OGG
    // Fonts
    &[0x00, 0x01, 0x00, 0x00], // TrueType font
    &[0x4F, 0x54, 0x54, 0x4F], // OpenType font
    // Database
    &[0x53, 0x51, 0x4C, 0x69], // SQLite
];

/// Extensions that are always considered binary
const BINARY_EXTENSIONS: &[&str] = &[
    // Images
    "png", "jpg", "jpeg", "gif", "bmp", "ico", "webp", "tiff", "tif", "psd", "svg",
    // Archives
    "zip", "tar", "gz", "bz2", "xz", "7z", "rar", "jar", "war", "ear", // Executables
    "exe", "dll", "so", "dylib", "bin", "o", "a", "lib", "pyc", "pyo", "class",
    // Documents
    "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "odt", "ods", "odp", // Media
    "mp3", "mp4", "avi", "mkv", "mov", "wmv", "flv", "wav", "flac", "ogg", "m4a",
    // Fonts
    "ttf", "otf", "woff", "woff2", "eot", // Other
    "db", "sqlite", "sqlite3", "pickle", "npy", "npz",
];

/// Check if a file is likely binary by examining its extension first, then content
pub fn is_binary(path: &Path) -> bool {
    // First check extension
    if let Some(ext) = path.extension() {
        let ext_lower = ext.to_string_lossy().to_lowercase();
        if BINARY_EXTENSIONS.contains(&ext_lower.as_str()) {
            return true;
        }
    }

    // Then check content
    is_binary_content(path)
}

/// Check if file content appears to be binary
pub fn is_binary_content(path: &Path) -> bool {
    let Ok(mut file) = File::open(path) else {
        return false; // Can't open, let later stages handle the error
    };

    let mut buffer = [0u8; 8192];
    let Ok(bytes_read) = file.read(&mut buffer) else {
        return false;
    };

    if bytes_read == 0 {
        return false; // Empty files are not binary
    }

    let content = &buffer[..bytes_read];

    // Check for binary signatures
    for sig in BINARY_SIGNATURES {
        if content.starts_with(sig) {
            return true;
        }
    }

    // Check for null bytes (strong indicator of binary)
    if content.contains(&0) {
        return true;
    }

    // Check ratio of non-printable characters
    let non_printable = content
        .iter()
        .filter(|&&b| {
            // Non-printable: not tab, newline, carriage return, or printable ASCII
            b < 0x09 || (b > 0x0D && b < 0x20) || b == 0x7F
        })
        .count();

    // If more than 10% non-printable, likely binary
    non_printable * 10 > bytes_read
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_text_file() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "Hello, world!").unwrap();
        writeln!(file, "This is a text file.").unwrap();
        assert!(!is_binary(file.path()));
    }

    #[test]
    fn test_binary_with_nulls() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(&[0x00, 0x01, 0x02, 0x03]).unwrap();
        assert!(is_binary(file.path()));
    }

    #[test]
    fn test_png_signature() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A])
            .unwrap();
        assert!(is_binary(file.path()));
    }

    #[test]
    fn test_empty_file_is_text() {
        let file = NamedTempFile::new().unwrap();
        assert!(!is_binary(file.path()));
    }

    #[test]
    fn test_binary_extension() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.png");
        std::fs::write(&path, "not actually png data").unwrap();
        // Should be detected as binary by extension alone
        assert!(is_binary(&path));
    }
}
