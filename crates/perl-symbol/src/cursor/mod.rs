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

#[derive(Debug, Clone, Copy)]
struct TokenSpan {
    name_start: usize,
    name_end: usize,
    sigil: Option<CursorSymbolKind>,
}

/// Return true when `byte` is an identifier character (`[A-Za-z0-9_]`).
#[inline]
fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
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
fn scan_while<F>(bytes: &[u8], mut index: usize, class: F) -> usize
where
    F: Fn(u8) -> bool,
{
    while index < bytes.len() && class(bytes[index]) {
        index += 1;
    }
    index
}

#[inline]
fn token_span_at_byte(source: &str, position: usize) -> Option<TokenSpan> {
    let bytes = source.as_bytes();
    if position >= bytes.len() {
        return None;
    }

    let (sigil, name_start) = if position > 0 {
        if let Some(kind) = sigil_kind(bytes[position - 1]) {
            (Some(kind), position)
        } else {
            (None, position)
        }
    } else {
        (None, position)
    };

    let (sigil, name_start) = if sigil.is_none() {
        if let Some(kind) = sigil_kind(bytes[position]) {
            (Some(kind), position.saturating_add(1))
        } else {
            (None, name_start)
        }
    } else {
        (sigil, name_start)
    };

    let name_end = scan_while(bytes, name_start, is_identifier_byte);

    Some(TokenSpan { name_start, name_end, sigil })
}

#[inline]
fn extract_token_at_byte<F>(source: &str, byte_pos: usize, class: F) -> Option<(usize, usize)>
where
    F: Fn(u8) -> bool,
{
    let bytes = source.as_bytes();
    if byte_pos >= bytes.len() {
        return None;
    }

    let mut start = byte_pos;
    while start > 0 && class(bytes[start - 1]) {
        start -= 1;
    }

    let mut end = byte_pos;
    while end < bytes.len() && class(bytes[end]) {
        end += 1;
    }

    if start == end {
        return None;
    }

    Some((start, end))
}

/// Extract a symbol and its kind from `source` at `position`.
pub fn extract_symbol_from_source(
    position: usize,
    source: &str,
) -> Option<(String, CursorSymbolKind)> {
    let span = token_span_at_byte(source, position)?;
    if span.name_end <= span.name_start {
        return None;
    }

    let name = source[span.name_start..span.name_end].to_string();
    let kind = span.sigil.unwrap_or(CursorSymbolKind::Subroutine);
    Some((name, kind))
}

/// Get symbol range at `position`, including a leading sigil when present.
pub fn get_symbol_range_at_position(position: usize, source: &str) -> Option<(usize, usize)> {
    let span = token_span_at_byte(source, position)?;
    let bytes = source.as_bytes();

    if sigil_kind(bytes[position]).is_some()
        && (position == 0 || sigil_kind(bytes[position - 1]).is_none())
    {
        return Some((position, position));
    }

    let mut start = span.name_start;
    while start < position && start < bytes.len() && is_identifier_byte(bytes[start]) {
        if start == 0 {
            break;
        }
        start -= 1;
    }

    if start > 0 && sigil_kind(bytes[start - 1]).is_some() {
        start -= 1;
    }

    Some((start, span.name_end))
}

/// Return true when `byte` is a module/name character (`[A-Za-z0-9_:]`).
#[inline]
pub fn is_modchar(byte: u8) -> bool {
    is_identifier_byte(byte) || byte == b':'
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
    let (mut start, end) = extract_token_at_byte(line_text, byte_pos, is_modchar)?;

    if start > 0 {
        let prev = line_text.as_bytes()[start - 1];
        if matches!(prev, b'$' | b'@' | b'%' | b'&' | b'*') {
            start -= 1;
        }
    }

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
