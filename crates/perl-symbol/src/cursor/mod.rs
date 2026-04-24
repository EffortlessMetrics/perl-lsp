//! Cursor-oriented symbol extraction for Perl source text.
//!
//! This module focuses on a single responsibility: extracting symbol names
//! and ranges around a cursor position.

/// Symbol sigil categories used for cursor extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorSymbolKind {
    /// Scalar variable (`$foo`)
    Scalar,
    /// Array variable (`@foo`)
    Array,
    /// Hash variable (`%foo`)
    Hash,
    /// Subroutine reference (`&foo`)
    Subroutine,
}

#[inline]
fn sigil_kind(byte: u8) -> Option<CursorSymbolKind> {
    match byte {
        b'$' => Some(CursorSymbolKind::Scalar),
        b'@' => Some(CursorSymbolKind::Array),
        b'%' => Some(CursorSymbolKind::Hash),
        b'&' => Some(CursorSymbolKind::Subroutine),
        _ => None,
    }
}

#[inline]
fn is_identifier_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TokenSpan {
    token_start: usize,
    token_end: usize,
    name_start: usize,
    name_end: usize,
    kind: CursorSymbolKind,
}

fn token_span_at_byte(position: usize, source: &str) -> Option<TokenSpan> {
    let bytes = source.as_bytes();
    if position >= bytes.len() {
        return None;
    }

    let mut name_start = position;
    let mut kind = CursorSymbolKind::Subroutine;
    let mut has_sigil = false;

    if let Some(sigil) = sigil_kind(bytes[position]) {
        has_sigil = true;
        kind = sigil;
        name_start = position + 1;
    } else if position > 0 {
        if let Some(sigil) = sigil_kind(bytes[position - 1]) {
            has_sigil = true;
            kind = sigil;
            name_start = position;
        } else if is_identifier_char(bytes[position]) {
            while name_start > 0 && is_identifier_char(bytes[name_start - 1]) {
                name_start -= 1;
            }
            if name_start > 0 {
                if let Some(sigil) = sigil_kind(bytes[name_start - 1]) {
                    has_sigil = true;
                    kind = sigil;
                }
            }
        }
    }

    let mut name_end = name_start;
    while name_end < bytes.len() && is_identifier_char(bytes[name_end]) {
        name_end += 1;
    }

    if name_start == name_end {
        return Some(TokenSpan {
            token_start: position,
            token_end: position,
            name_start,
            name_end,
            kind,
        });
    }

    let token_start = if has_sigil { name_start - 1 } else { name_start };
    Some(TokenSpan {
        token_start,
        token_end: name_end,
        name_start,
        name_end,
        kind,
    })
}

fn extract_token_at_byte(position: usize, source: &str) -> Option<(String, CursorSymbolKind)> {
    let span = token_span_at_byte(position, source)?;
    if span.name_start == span.name_end {
        return None;
    }

    let name = source.get(span.name_start..span.name_end)?.to_string();
    Some((name, span.kind))
}

fn module_token_span_at_byte(position: usize, source: &str) -> Option<(usize, usize)> {
    let bytes = source.as_bytes();
    if position >= bytes.len() {
        return None;
    }

    let mut start = position;
    while start > 0 && is_modchar(bytes[start - 1]) {
        start -= 1;
    }

    if start > 0 && matches!(bytes[start - 1], b'$' | b'@' | b'%' | b'&' | b'*') {
        start -= 1;
    }

    let mut end = position;
    while end < bytes.len() && is_modchar(bytes[end]) {
        end += 1;
    }

    Some((start, end))
}

/// Extract a symbol and its kind from `source` at `position`.
pub fn extract_symbol_from_source(
    position: usize,
    source: &str,
) -> Option<(String, CursorSymbolKind)> {
    extract_token_at_byte(position, source)
}

/// Get symbol range at `position`, including a leading sigil when present.
pub fn get_symbol_range_at_position(position: usize, source: &str) -> Option<(usize, usize)> {
    let span = token_span_at_byte(position, source)?;
    Some((span.token_start, span.token_end))
}

/// Return true when `byte` is a module/name character (`[A-Za-z0-9_:]`).
#[inline]
pub fn is_modchar(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b':' || byte == b'_'
}

/// Convert a UTF-16 column index to a byte offset for a single line.
#[inline]
pub fn byte_offset_utf16(line_text: &str, col_utf16: usize) -> usize {
    let mut units = 0;
    for (i, ch) in line_text.char_indices() {
        if units >= col_utf16 {
            return i;
        }
        let ch_units = if ch as u32 >= 0x10000 { 2 } else { 1 };
        units += ch_units;
        if units > col_utf16 {
            return i;
        }
    }
    line_text.len()
}

/// Extract the module/symbol token under the cursor (UTF-16 aware).
pub fn token_under_cursor(text: &str, line: usize, col_utf16: usize) -> Option<String> {
    let line_text = text.lines().nth(line)?;
    let byte_pos = byte_offset_utf16(line_text, col_utf16);
    let (start, end) = module_token_span_at_byte(byte_pos, line_text)?;
    Some(line_text[start..end].to_string())
}

/// Check if a match at `pos..pos+word_len` is bounded by non-word chars.
pub fn is_word_boundary(text: &[u8], pos: usize, word_len: usize) -> bool {
    if pos > 0 && is_modchar(text[pos - 1]) {
        return false;
    }

    let end_pos = pos + word_len;
    if end_pos < text.len() && is_modchar(text[end_pos]) {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::{byte_offset_utf16, is_word_boundary, token_under_cursor};

    #[test]
    fn token_under_cursor_extracts_perl_module_token() {
        let text = "use Demo::Worker;\n";
        assert_eq!(token_under_cursor(text, 0, 8), Some("Demo::Worker".to_string()));
    }

    #[test]
    fn token_under_cursor_supports_sigils() {
        let text = "my $value = 1;\n";
        assert_eq!(token_under_cursor(text, 0, 5), Some("$value".to_string()));
    }

    #[test]
    fn utf16_col_to_byte_offset_handles_surrogate_pairs() {
        let line = "A😀B";
        assert_eq!(byte_offset_utf16(line, 0), 0);
        assert_eq!(byte_offset_utf16(line, 1), 1);
        assert_eq!(byte_offset_utf16(line, 2), 1);
        assert_eq!(byte_offset_utf16(line, 3), 5);
        assert_eq!(byte_offset_utf16(line, 4), 6);
    }

    #[test]
    fn word_boundary_detects_embedded_word() {
        let text = b"fooDemo::Workerbar";
        assert!(!is_word_boundary(text, 3, "Demo::Worker".len()));
        assert!(is_word_boundary(b" Demo::Worker ", 1, "Demo::Worker".len()));
    }
}
