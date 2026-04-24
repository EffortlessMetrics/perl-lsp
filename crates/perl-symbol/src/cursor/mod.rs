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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TokenSpan {
    start: usize,
    name_start: usize,
    end: usize,
    sigil: Option<CursorSymbolKind>,
}

#[inline]
fn is_identifier_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

#[inline]
fn is_symbol_sigil(byte: u8) -> bool {
    matches!(byte, b'$' | b'@' | b'%' | b'&')
}

#[inline]
fn sigil_to_kind(byte: u8) -> Option<CursorSymbolKind> {
    match byte {
        b'$' => Some(CursorSymbolKind::Scalar),
        b'@' => Some(CursorSymbolKind::Array),
        b'%' => Some(CursorSymbolKind::Hash),
        b'&' => Some(CursorSymbolKind::Subroutine),
        _ => None,
    }
}

fn token_span_at_byte(source: &str, position: usize) -> Option<TokenSpan> {
    let bytes = source.as_bytes();
    let current = *bytes.get(position)?;

    if is_symbol_sigil(current) {
        let name_start = position + 1;
        let mut end = name_start;
        while end < bytes.len() && is_identifier_char(bytes[end]) {
            end += 1;
        }

        if end == name_start {
            return None;
        }

        return Some(TokenSpan { start: position, name_start, end, sigil: sigil_to_kind(current) });
    }

    if !is_identifier_char(current) {
        return None;
    }

    let mut name_start = position;
    while name_start > 0 && is_identifier_char(bytes[name_start - 1]) {
        name_start -= 1;
    }

    let mut end = position;
    while end < bytes.len() && is_identifier_char(bytes[end]) {
        end += 1;
    }

    let (start, sigil) = if name_start > 0 {
        let before = bytes[name_start - 1];
        if is_symbol_sigil(before) {
            (name_start - 1, sigil_to_kind(before))
        } else {
            (name_start, None)
        }
    } else {
        (name_start, None)
    };

    Some(TokenSpan { start, name_start, end, sigil })
}

fn extract_token_at_byte(source: &str, position: usize) -> Option<(String, CursorSymbolKind)> {
    let span = token_span_at_byte(source, position)?;
    let name = source.get(span.name_start..span.end)?.to_string();
    let kind = span.sigil.unwrap_or(CursorSymbolKind::Subroutine);
    Some((name, kind))
}

/// Extract a symbol and its kind from `source` at `position`.
pub fn extract_symbol_from_source(
    position: usize,
    source: &str,
) -> Option<(String, CursorSymbolKind)> {
    extract_token_at_byte(source, position)
}

/// Get symbol range at `position`, including a leading sigil when present.
pub fn get_symbol_range_at_position(position: usize, source: &str) -> Option<(usize, usize)> {
    let span = token_span_at_byte(source, position)?;
    Some((span.start, span.end))
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

fn token_span_with_rules<F>(text: &str, position: usize, is_body_char: F) -> Option<(usize, usize)>
where
    F: Fn(u8) -> bool,
{
    let bytes = text.as_bytes();
    let current = *bytes.get(position)?;

    if !is_body_char(current) && !matches!(current, b'$' | b'@' | b'%' | b'&' | b'*') {
        return None;
    }

    let mut start = position;
    if is_body_char(current) {
        while start > 0 && is_body_char(bytes[start - 1]) {
            start -= 1;
        }
    }

    if start > 0 && matches!(bytes[start - 1], b'$' | b'@' | b'%' | b'&' | b'*') {
        start -= 1;
    }

    let mut end = if is_body_char(current) { position } else { position + 1 };
    while end < bytes.len() && is_body_char(bytes[end]) {
        end += 1;
    }

    if end <= start {
        return None;
    }

    Some((start, end))
}

/// Extract the module/symbol token under the cursor (UTF-16 aware).
pub fn token_under_cursor(text: &str, line: usize, col_utf16: usize) -> Option<String> {
    let line_text = text.lines().nth(line)?;
    let byte_pos = byte_offset_utf16(line_text, col_utf16);
    let (start, end) = token_span_with_rules(line_text, byte_pos, is_modchar)?;
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
