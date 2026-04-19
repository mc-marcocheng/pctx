//! File content reading with encoding detection.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::error::PctxError;

/// Read file contents, attempting to handle encoding issues gracefully
pub fn read_file_contents(
    path: &Path,
    max_size: u64,
    _encoding: Option<&str>,
) -> Result<String, PctxError> {
    // Open the file first to get a stable file descriptor
    let file = File::open(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            PctxError::FileNotFound(path.to_path_buf())
        } else if e.kind() == std::io::ErrorKind::PermissionDenied {
            PctxError::PermissionDenied(path.to_path_buf())
        } else {
            PctxError::Io(e)
        }
    })?;

    // Check size on the open file descriptor
    let metadata = file.metadata().map_err(PctxError::Io)?;
    if metadata.len() > max_size {
        return Err(PctxError::FileTooLarge {
            path: path.to_path_buf(),
            size: metadata.len(),
            max: max_size,
        });
    }

    // Read with a hard limit to prevent OOM if the file is being actively appended to
    let mut bytes = Vec::new();
    // Read up to max_size + 1 to detect if it exceeds the limit during the read
    file.take(max_size + 1)
        .read_to_end(&mut bytes)
        .map_err(PctxError::Io)?;

    if bytes.len() as u64 > max_size {
        return Err(PctxError::FileTooLarge {
            path: path.to_path_buf(),
            size: bytes.len() as u64, // This will be max_size + 1
            max: max_size,
        });
    }

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

    // Use a 10MB limit for standard tests
    const MAX_SIZE: u64 = 10 * 1024 * 1024;

    #[test]
    fn test_read_utf8_file() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "Hello, world!").unwrap();
        writeln!(file, "This is a test.").unwrap();

        let content = read_file_contents(file.path(), MAX_SIZE, None).unwrap();
        assert!(content.contains("Hello, world!"));
        assert!(content.contains("This is a test."));
    }

    #[test]
    fn test_read_file_with_unicode() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "Hello, 世界!").unwrap();
        writeln!(file, "Привет мир!").unwrap();
        writeln!(file, "🎉🎊🎈").unwrap();

        let content = read_file_contents(file.path(), MAX_SIZE, None).unwrap();
        assert!(content.contains("世界"));
        assert!(content.contains("Привет"));
        assert!(content.contains("🎉"));
    }

    #[test]
    fn test_read_empty_file() {
        let file = NamedTempFile::new().unwrap();
        let content = read_file_contents(file.path(), MAX_SIZE, None).unwrap();
        assert!(content.is_empty());
    }

    #[test]
    fn test_read_nonexistent_file() {
        let result = read_file_contents(Path::new("/nonexistent/file.txt"), MAX_SIZE, None);
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
        let content = read_file_contents(file.path(), MAX_SIZE, None).unwrap();
        assert!(content.contains("Valid text"));
        assert!(content.contains("more text"));
        // Invalid bytes should be replaced with replacement character
        assert!(content.contains('\u{FFFD}'));
    }

    #[test]
    fn test_read_file_too_large() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"1234567890").unwrap(); // 10 bytes

        // Try to read with max size 5
        let result = read_file_contents(file.path(), 5, None);
        assert!(result.is_err());
        match result.unwrap_err() {
            PctxError::FileTooLarge { size, max, .. } => {
                assert_eq!(size, 10);
                assert_eq!(max, 5);
            }
            e => panic!("Expected FileTooLarge, got {:?}", e),
        }
    }
}
