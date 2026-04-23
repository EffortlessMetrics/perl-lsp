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

/// Extract a symbol and its kind from `source` at `position`.
pub fn extract_symbol_from_source(
    position: usize,
    source: &str,
) -> Option<(String, CursorSymbolKind)> {
    let chars: Vec<char> = source.chars().collect();
    if position >= chars.len() {
        return None;
    }

    let (sigil, name_start) = if position > 0 {
        match chars.get(position - 1) {
            Some('$') => (Some(CursorSymbolKind::Scalar), position),
            Some('@') => (Some(CursorSymbolKind::Array), position),
            Some('%') => (Some(CursorSymbolKind::Hash), position),
            Some('&') => (Some(CursorSymbolKind::Subroutine), position),
            _ => (None, position),
        }
    } else {
        (None, position)
    };

    let (sigil, name_start) = if sigil.is_none() && position < chars.len() {
        match chars[position] {
            '$' => (Some(CursorSymbolKind::Scalar), position + 1),
            '@' => (Some(CursorSymbolKind::Array), position + 1),
            '%' => (Some(CursorSymbolKind::Hash), position + 1),
            '&' => (Some(CursorSymbolKind::Subroutine), position + 1),
            _ => (sigil, name_start),
        }
    } else {
        (sigil, name_start)
    };

    let mut end = name_start;
    while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
        end += 1;
    }

    if end > name_start {
        let name: String = chars[name_start..end].iter().collect();
        let kind = sigil.unwrap_or(CursorSymbolKind::Subroutine);
        Some((name, kind))
    } else {
        None
    }
}

/// Get symbol range at `position`, including a leading sigil when present.
pub fn get_symbol_range_at_position(position: usize, source: &str) -> Option<(usize, usize)> {
    let chars: Vec<char> = source.chars().collect();
    if position >= chars.len() {
        return None;
    }

    let mut start = position;
    if start > 0 && matches!(chars[start - 1], '$' | '@' | '%' | '&') {
        start -= 1;
    }

    let mut end = position;
    while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
        end += 1;
    }

    while start < position
        && start < chars.len()
        && (chars[start].is_alphanumeric() || chars[start] == '_')
    {
        start -= 1;
    }

    Some((start, end))
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
        if units == col_utf16 {
            return i;
        }
        units += if ch as u32 >= 0x10000 { 2 } else { 1 };
    }
    line_text.len()
}

/// Extract the module/symbol token under the cursor (UTF-16 aware).
pub fn token_under_cursor(text: &str, line: usize, col_utf16: usize) -> Option<String> {
    let line_text = text.lines().nth(line)?;
    let byte_pos = byte_offset_utf16(line_text, col_utf16);
    let bytes = line_text.as_bytes();

    if bytes.is_empty() {
        return None;
    }

    // LSP cursors are often positioned *between* characters. Support both:
    // - on-token cursors: `foo<cursor>`
    // - boundary cursors: `foo<cursor>;`
    let anchor = if byte_pos < bytes.len() && is_modchar(bytes[byte_pos]) {
        byte_pos
    } else if byte_pos > 0 && is_modchar(bytes[byte_pos - 1]) {
        byte_pos - 1
    } else {
        return None;
    };

    let mut start = anchor;
    let mut end = anchor;

    while start > 0 && is_modchar(bytes[start - 1]) {
        start -= 1;
    }
    if start > 0 && matches!(bytes[start - 1], b'$' | b'@' | b'%' | b'&' | b'*') {
        start -= 1;
    }

    while end < bytes.len() && is_modchar(bytes[end]) {
        end += 1;
    }

    if start == end {
        return None;
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
    fn token_under_cursor_supports_cursor_after_token() {
        let text = "use Demo::Worker;\n";
        // Cursor between "r" and ";"
        assert_eq!(token_under_cursor(text, 0, 16), Some("Demo::Worker".to_string()));
    }

    #[test]
    fn token_under_cursor_returns_none_on_punctuation() {
        let text = "use Demo::Worker;\n";
        assert_eq!(token_under_cursor(text, 0, 17), None);
    }

    #[test]
    fn utf16_col_to_byte_offset_handles_surrogate_pairs() {
        let line = "A😀B";
        assert_eq!(byte_offset_utf16(line, 0), 0);
        assert_eq!(byte_offset_utf16(line, 1), 1);
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
