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
struct ScanOptions {
    include_leading_identifier: bool,
    allow_cursor_on_sigil: bool,
}

#[inline]
fn is_symbol_sigil(byte: u8) -> bool {
    matches!(byte, b'$' | b'@' | b'%' | b'&')
}

#[inline]
fn is_cursor_ident_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

#[inline]
fn is_module_sigil(byte: u8) -> bool {
    is_symbol_sigil(byte) || byte == b'*'
}

#[inline]
fn token_span_at_byte(
    bytes: &[u8],
    byte_pos: usize,
    is_token_char: fn(u8) -> bool,
    is_sigil: fn(u8) -> bool,
    options: ScanOptions,
) -> Option<(usize, usize, usize)> {
    if byte_pos >= bytes.len() {
        return None;
    }

    let byte = bytes[byte_pos];

    if is_sigil(byte) {
        if !options.allow_cursor_on_sigil || (byte_pos > 0 && is_sigil(bytes[byte_pos - 1])) {
            return None;
        }

        let name_start = byte_pos + 1;
        let mut end = name_start;
        while end < bytes.len() && is_token_char(bytes[end]) {
            end += 1;
        }

        if end == name_start {
            return None;
        }

        return Some((byte_pos, name_start, end));
    }

    if !is_token_char(byte) {
        return None;
    }

    let mut name_start = byte_pos;
    if options.include_leading_identifier {
        while name_start > 0 && is_token_char(bytes[name_start - 1]) {
            name_start -= 1;
        }
    }

    let mut start = name_start;
    if start > 0 && is_sigil(bytes[start - 1]) {
        start -= 1;
    }

    let mut end = byte_pos;
    while end < bytes.len() && is_token_char(bytes[end]) {
        end += 1;
    }

    Some((start, name_start, end))
}

#[inline]
fn extract_token_at_byte(
    source: &str,
    byte_pos: usize,
    is_token_char: fn(u8) -> bool,
    is_sigil: fn(u8) -> bool,
    options: ScanOptions,
) -> Option<(usize, &str)> {
    let bytes = source.as_bytes();
    let (_start, name_start, end) = token_span_at_byte(bytes, byte_pos, is_token_char, is_sigil, options)?;
    Some((name_start, &source[name_start..end]))
}

/// Extract a symbol and its kind from `source` at `position`.
///
/// `position` uses byte offsets.
pub fn extract_symbol_from_source(
    position: usize,
    source: &str,
) -> Option<(String, CursorSymbolKind)> {
    let bytes = source.as_bytes();
    let options = ScanOptions {
        include_leading_identifier: false,
        allow_cursor_on_sigil: true,
    };
    let (start, name_start, _end) =
        token_span_at_byte(bytes, position, is_cursor_ident_char, is_symbol_sigil, options)?;
    let (_, name) =
        extract_token_at_byte(source, position, is_cursor_ident_char, is_symbol_sigil, options)?;

    let kind = if start < name_start {
        match bytes[start] {
            b'$' => CursorSymbolKind::Scalar,
            b'@' => CursorSymbolKind::Array,
            b'%' => CursorSymbolKind::Hash,
            _ => CursorSymbolKind::Subroutine,
        }
    } else {
        CursorSymbolKind::Subroutine
    };

    Some((name.to_string(), kind))
}

/// Get symbol range at `position`, including a leading sigil when present.
///
/// `position` uses byte offsets.
pub fn get_symbol_range_at_position(position: usize, source: &str) -> Option<(usize, usize)> {
    if position >= source.len() {
        return None;
    }

    let bytes = source.as_bytes();
    let options = ScanOptions {
        include_leading_identifier: true,
        allow_cursor_on_sigil: false,
    };
    if let Some((start, _name_start, end)) =
        token_span_at_byte(bytes, position, is_cursor_ident_char, is_symbol_sigil, options)
    {
        return Some((start, end));
    }

    Some((position, position))
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
    let bytes = line_text.as_bytes();

    let options = ScanOptions {
        include_leading_identifier: true,
        allow_cursor_on_sigil: true,
    };
    let (start, _name_start, end) = token_span_at_byte(bytes, byte_pos, is_modchar, is_module_sigil, options)?;
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
        let text = "use Demo::Worker;
";
        assert_eq!(token_under_cursor(text, 0, 8), Some("Demo::Worker".to_string()));
    }

    #[test]
    fn token_under_cursor_supports_sigils() {
        let text = "my $value = 1;
";
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
