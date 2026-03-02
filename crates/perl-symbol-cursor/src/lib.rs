//! Cursor-oriented symbol extraction for Perl source text.
//!
//! This microcrate focuses on a single responsibility: extracting symbol names
//! and ranges around a cursor position.

/// Symbol sigil categories used for cursor extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorSymbolKind {
    Scalar,
    Array,
    Hash,
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
