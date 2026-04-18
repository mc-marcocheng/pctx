//! File content reading with encoding detection.

use std::fs;
use std::path::Path;

use crate::error::PctxError;

/// Read file contents, attempting to handle encoding issues gracefully
pub fn read_file_contents(path: &Path, _encoding: Option<&str>) -> Result<String, PctxError> {
    // Read raw bytes
    let bytes = fs::read(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            PctxError::FileNotFound(path.to_path_buf())
        } else if e.kind() == std::io::ErrorKind::PermissionDenied {
            PctxError::PermissionDenied(path.to_path_buf())
        } else {
            PctxError::Io(e)
        }
    })?;

    // Try UTF-8 first (most common case)
    match String::from_utf8(bytes) {
        Ok(content) => Ok(content),
        Err(e) => {
            // Fall back to lossy UTF-8 conversion
            // This handles files with mixed encodings or invalid UTF-8 sequences
            Ok(String::from_utf8_lossy(e.as_bytes()).into_owned())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_read_utf8_file() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "Hello, world!").unwrap();
        writeln!(file, "This is a test.").unwrap();

        let content = read_file_contents(file.path(), None).unwrap();
        assert!(content.contains("Hello, world!"));
        assert!(content.contains("This is a test."));
    }

    #[test]
    fn test_read_file_with_unicode() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "Hello, 世界!").unwrap();
        writeln!(file, "Привет мир!").unwrap();
        writeln!(file, "🎉🎊🎈").unwrap();

        let content = read_file_contents(file.path(), None).unwrap();
        assert!(content.contains("世界"));
        assert!(content.contains("Привет"));
        assert!(content.contains("🎉"));
    }

    #[test]
    fn test_read_empty_file() {
        let file = NamedTempFile::new().unwrap();
        let content = read_file_contents(file.path(), None).unwrap();
        assert!(content.is_empty());
    }

    #[test]
    fn test_read_nonexistent_file() {
        let result = read_file_contents(Path::new("/nonexistent/file.txt"), None);
        assert!(result.is_err());
        match result.unwrap_err() {
            PctxError::FileNotFound(_) => {}
            e => panic!("Expected FileNotFound, got {:?}", e),
        }
    }

    #[test]
    fn test_read_file_with_invalid_utf8() {
        let mut file = NamedTempFile::new().unwrap();
        // Write some valid UTF-8 followed by invalid bytes
        file.write_all(b"Valid text ").unwrap();
        file.write_all(&[0xFF, 0xFE]).unwrap(); // Invalid UTF-8
        file.write_all(b" more text").unwrap();

        // Should succeed with lossy conversion
        let content = read_file_contents(file.path(), None).unwrap();
        assert!(content.contains("Valid text"));
        assert!(content.contains("more text"));
        // Invalid bytes should be replaced with replacement character
        assert!(content.contains('\u{FFFD}'));
    }
}
