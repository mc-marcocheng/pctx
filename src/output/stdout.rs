//! Standard output writing.

use std::io::{self, Write};

use crate::error::PctxError;

/// Write content to stdout
pub fn write(content: &str) -> Result<(), PctxError> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    handle.write_all(content.as_bytes())?;
    handle.flush()?;
    Ok(())
}
