//! File reading utilities with encoding detection.

use std::fs;
use std::path::Path;

use crate::error::PctxError;

/// Threshold for replacement characters - if more than this ratio, consider it encoding error
const REPLACEMENT_CHAR_THRESHOLD: f64 = 0.3;

/// Read a file as UTF-8 text
pub fn read_file(path: &Path) -> Result<String, PctxError> {
    // Try reading as UTF-8 first
    match fs::read_to_string(path) {
        Ok(content) => Ok(content),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                Err(PctxError::FileNotFound(path.to_path_buf()))
            } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                Err(PctxError::PermissionDenied(path.to_path_buf()))
            } else {
                // Try reading as bytes and converting with lossy UTF-8
                match fs::read(path) {
                    Ok(bytes) => {
                        let content = String::from_utf8_lossy(&bytes);
                        // If there are replacement characters, it might be binary
                        if content.contains('\u{FFFD}') {
                            // Check if it's mostly replacement chars
                            let replacement_count = content.matches('\u{FFFD}').count();
                            let char_count = content.chars().count();
                            if char_count > 0
                                && (replacement_count as f64 / char_count as f64)
                                    > REPLACEMENT_CHAR_THRESHOLD
                            {
                                return Err(PctxError::EncodingError {
                                    path: path.to_path_buf(),
                                    reason:
                                        "File appears to be binary or uses unsupported encoding"
                                            .to_string(),
                                });
                            }
                        }
                        Ok(content.into_owned())
                    }
                    Err(e) => Err(PctxError::Io(e)),
                }
            }
        }
    }
}