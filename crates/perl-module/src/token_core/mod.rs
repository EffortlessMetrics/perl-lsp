//! Shared primitives for Perl module token parsing and boundary detection.

/// Byte span for a parsed module token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleTokenSpan {
    /// Inclusive byte start offset in the source text.
    pub start: usize,
    /// Exclusive byte end offset in the source text.
    pub end: usize,
}

/// Parse a module token that starts at `start` in `text`.
///
/// A module token is one or more identifier segments separated by either
/// `::` (canonical) or `'` (legacy) separators.
#[must_use]
pub fn parse_module_token(text: &str, start: usize) -> Option<ModuleTokenSpan> {
    let bytes = text.as_bytes();
    if start >= bytes.len() || !is_identifier_start(bytes[start]) {
        return None;
    }

    let token_start = start;
    let mut index = parse_identifier_segment(bytes, start)?;

    while let Some(next) = next_separator(bytes, index) {
        index = match next {
            Separator::Canonical => index + 2,
            Separator::Legacy => index + 1,
        };

        index = parse_identifier_segment(bytes, index)?;
    }

    Some(ModuleTokenSpan { start: token_start, end: index })
}

/// Check if a span from `start` to `end` is bounded as a standalone token.
#[must_use]
pub fn has_standalone_module_token_boundaries(line: &str, start: usize, end: usize) -> bool {
    let left_ok = !left_context_is_module_char(line, start);
    let right_ok = !right_context_is_module_char(line, end);

    left_ok && right_ok
}

/// Check whether `ch` belongs to the module token character class.
#[must_use]
pub fn is_module_token_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == ':'
}

/// Check whether `ch` belongs to Perl module identifier characters.
#[must_use]
pub fn is_module_identifier_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

#[derive(Debug, Clone, Copy)]
enum Separator {
    Canonical,
    Legacy,
}

fn next_separator(bytes: &[u8], index: usize) -> Option<Separator> {
    if text_starts_with(bytes, index, "::") {
        return Some(Separator::Canonical);
    }

    if index < bytes.len() && bytes[index] == b'\'' {
        return Some(Separator::Legacy);
    }

    None
}

fn parse_identifier_segment(bytes: &[u8], start: usize) -> Option<usize> {
    if start >= bytes.len() || !is_identifier_start(bytes[start]) {
        return None;
    }

    let mut index = start + 1;
    while index < bytes.len() && is_identifier_byte(bytes[index]) {
        index += 1;
    }

    Some(index)
}

fn is_identifier_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn left_context_is_module_char(line: &str, start: usize) -> bool {
    if start == 0 {
        return false;
    }

    let mut left = line[..start].char_indices();
    let Some((left_idx, ch)) = left.next_back() else {
        return false;
    };

    if ch != '\'' {
        return is_module_token_char(ch);
    }

    if left_idx == 0 {
        return false;
    }

    line[..left_idx].chars().next_back().is_some_and(is_module_identifier_char)
}

fn right_context_is_module_char(line: &str, end: usize) -> bool {
    if end >= line.len() {
        return false;
    }

    let mut right = line[end..].chars();
    let Some(ch) = right.next() else {
        return false;
    };

    if ch != '\'' {
        return is_module_token_char(ch);
    }

    right.next().is_some_and(is_module_identifier_char)
}

fn text_starts_with(bytes: &[u8], start: usize, needle: &str) -> bool {
    let bytes_len = bytes.len();
    let needle_bytes = needle.as_bytes();
    if start + needle_bytes.len() > bytes_len {
        return false;
    }

    &bytes[start..start + needle_bytes.len()] == needle_bytes
}
