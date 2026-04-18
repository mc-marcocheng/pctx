//! File output writing.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use crate::error::PctxError;

/// Write content to a file (atomic check-and-create to avoid TOCTOU race)
pub fn write(path: &Path, content: &str, force: bool) -> Result<(), PctxError> {
    let mut options = OpenOptions::new();
    options.write(true);

    if force {
        options.create(true).truncate(true);
    } else {
        options.create_new(true);
    }

    let mut file = options.open(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::AlreadyExists {
            PctxError::OutputExists(path.to_path_buf())
        } else if e.kind() == std::io::ErrorKind::PermissionDenied {
            PctxError::PermissionDenied(path.to_path_buf())
        } else {
            PctxError::Io(e)
        }
    })?;

    file.write_all(content.as_bytes())?;
    file.flush()?;
    Ok(())
}
