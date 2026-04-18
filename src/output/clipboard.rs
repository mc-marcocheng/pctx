//! Clipboard output support.

use crate::error::PctxError;

/// Write content to system clipboard
#[cfg(feature = "clipboard")]
pub fn write(content: &str) -> Result<(), PctxError> {
    use arboard::Clipboard;

    let mut clipboard =
        Clipboard::new().map_err(|e| PctxError::ClipboardError(e.to_string()))?;

    clipboard
        .set_text(content)
        .map_err(|e| PctxError::ClipboardError(e.to_string()))?;

    Ok(())
}

/// Stub when clipboard feature is disabled
#[cfg(not(feature = "clipboard"))]
pub fn write(_content: &str) -> Result<(), PctxError> {
    Err(PctxError::ClipboardError(
        "Clipboard support not compiled. Rebuild with --features clipboard".to_string(),
    ))
}