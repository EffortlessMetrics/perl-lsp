#[cfg(feature = "workspace")]
use std::path::Path;

/// Read source text from disk with basic encoding fallbacks.
#[cfg(feature = "workspace")]
pub(super) fn read_text_with_encoding_fallback(path: &Path) -> std::io::Result<String> {
    let bytes = std::fs::read(path)?;
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return Ok(String::from_utf8_lossy(&bytes[3..]).into_owned());
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        let payload = &bytes[2..];
        if !payload.len().is_multiple_of(2) {
            return Ok(String::from_utf8_lossy(&bytes).into_owned());
        }
        let units: Vec<u16> =
            payload.chunks_exact(2).map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]])).collect();
        return Ok(String::from_utf16_lossy(&units));
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let payload = &bytes[2..];
        if !payload.len().is_multiple_of(2) {
            return Ok(String::from_utf8_lossy(&bytes).into_owned());
        }
        let units: Vec<u16> =
            payload.chunks_exact(2).map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]])).collect();
        return Ok(String::from_utf16_lossy(&units));
    }
    match String::from_utf8(bytes) {
        Ok(text) => Ok(text),
        Err(err) => Ok(String::from_utf8_lossy(&err.into_bytes()).into_owned()),
    }
}
