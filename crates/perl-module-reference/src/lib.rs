//! Cursor-aware Perl module reference extraction.
//!
//! This crate has one responsibility: given source text and a cursor offset,
//! identify module references used by `use`/`require` statements.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use perl_module_name::normalize_package_separator;

/// Statement kind for a parsed module reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleReferenceKind {
    /// `use Module::Name;`
    Use,
    /// `require Module::Name;`
    Require,
}

/// Module reference found at a cursor location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleReference<'a> {
    /// Statement kind (`use` or `require`).
    pub kind: ModuleReferenceKind,
    /// Raw module token text as written in source.
    pub module_name: &'a str,
    /// Inclusive byte start offset of `module_name` in the input text.
    pub module_start: usize,
    /// Exclusive byte end offset of `module_name` in the input text.
    pub module_end: usize,
}

impl ModuleReference<'_> {
    /// Return the module name normalized to canonical `::` separators.
    #[must_use]
    pub fn canonical_module_name(&self) -> String {
        normalize_package_separator(self.module_name).into_owned()
    }
}

/// Find a `use`/`require` module reference at `cursor_pos`.
///
/// Returns [`None`] if the cursor is not over a direct module token in a
/// `use` or `require` statement.
#[must_use]
pub fn find_module_reference(text: &str, cursor_pos: usize) -> Option<ModuleReference<'_>> {
    if text.is_empty() || cursor_pos > text.len() {
        return None;
    }

    let (line_start, line_end) = line_bounds_at(text, cursor_pos);
    let line = &text[line_start..line_end];
    let cursor_in_line = cursor_pos.saturating_sub(line_start);

    find_in_line(line, line_start, cursor_in_line)
}

/// Extract a module reference at `cursor_pos` as a canonical module name.
///
/// Returns canonical `::` separators even when source uses legacy `'`
/// separators.
#[must_use]
pub fn extract_module_reference(text: &str, cursor_pos: usize) -> Option<String> {
    find_module_reference(text, cursor_pos).map(|reference| reference.canonical_module_name())
}

fn line_bounds_at(text: &str, cursor_pos: usize) -> (usize, usize) {
    let cursor = cursor_pos.min(text.len());
    let start = text[..cursor].rfind('\n').map_or(0, |idx| idx + 1);
    let end = text[cursor..].find('\n').map_or(text.len(), |idx| cursor + idx);
    (start, end)
}

fn find_in_line(
    line: &str,
    line_offset: usize,
    cursor_in_line: usize,
) -> Option<ModuleReference<'_>> {
    find_in_line_for_keyword(line, line_offset, cursor_in_line, "use", ModuleReferenceKind::Use)
        .or_else(|| {
            find_in_line_for_keyword(
                line,
                line_offset,
                cursor_in_line,
                "require",
                ModuleReferenceKind::Require,
            )
        })
}

fn find_in_line_for_keyword<'a>(
    line: &'a str,
    line_offset: usize,
    cursor_in_line: usize,
    keyword: &'static str,
    kind: ModuleReferenceKind,
) -> Option<ModuleReference<'a>> {
    let keyword_len = keyword.len();
    let bytes = line.as_bytes();
    let mut idx = 0usize;

    while idx + keyword_len <= bytes.len() {
        if !line[idx..].starts_with(keyword) {
            idx += 1;
            continue;
        }

        if !is_keyword_boundary(bytes, idx, keyword_len) {
            idx += 1;
            continue;
        }

        let after_keyword = idx + keyword_len;
        if after_keyword >= bytes.len() || !bytes[after_keyword].is_ascii_whitespace() {
            idx += 1;
            continue;
        }

        let module_start = skip_ascii_whitespace(bytes, after_keyword);
        if module_start >= bytes.len() {
            idx += 1;
            continue;
        }

        if let Some(module_end) = parse_module_token(line, module_start)
            && cursor_in_line >= module_start
            && cursor_in_line <= module_end
        {
            return Some(ModuleReference {
                kind,
                module_name: &line[module_start..module_end],
                module_start: line_offset + module_start,
                module_end: line_offset + module_end,
            });
        }

        idx += 1;
    }

    None
}

fn is_keyword_boundary(bytes: &[u8], start: usize, len: usize) -> bool {
    if start > 0 && is_identifier_byte(bytes[start - 1]) {
        return false;
    }

    let end = start + len;
    if end < bytes.len() && is_identifier_byte(bytes[end]) {
        return false;
    }

    true
}

fn skip_ascii_whitespace(bytes: &[u8], mut idx: usize) -> usize {
    while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
        idx += 1;
    }
    idx
}

fn parse_module_token(line: &str, start: usize) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut idx = parse_identifier_segment(bytes, start)?;

    loop {
        if line[idx..].starts_with("::") {
            idx += 2;
            idx = parse_identifier_segment(bytes, idx)?;
            continue;
        }

        if idx < bytes.len() && bytes[idx] == b'\'' {
            idx += 1;
            idx = parse_identifier_segment(bytes, idx)?;
            continue;
        }

        break;
    }

    Some(idx)
}

fn parse_identifier_segment(bytes: &[u8], start: usize) -> Option<usize> {
    if start >= bytes.len() || !is_identifier_start(bytes[start]) {
        return None;
    }

    let mut idx = start + 1;
    while idx < bytes.len() && is_identifier_byte(bytes[idx]) {
        idx += 1;
    }
    Some(idx)
}

fn is_identifier_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

#[cfg(test)]
mod tests {
    use super::{
        ModuleReferenceKind, extract_module_reference, find_module_reference, parse_module_token,
    };

    #[test]
    fn finds_use_module_reference() {
        let text = "use Foo::Bar;";
        let cursor = text.find("Bar").unwrap_or(0);

        let reference = find_module_reference(text, cursor);
        assert!(reference.is_some());
        if let Some(reference) = reference {
            assert_eq!(reference.kind, ModuleReferenceKind::Use);
            assert_eq!(reference.module_name, "Foo::Bar");
            assert_eq!(reference.module_start, 4);
            assert_eq!(reference.module_end, 12);
        }
    }

    #[test]
    fn finds_require_module_reference() {
        let text = "require Foo::Bar;";
        let cursor = text.find("Foo").unwrap_or(0);

        let reference = find_module_reference(text, cursor);
        assert!(reference.is_some());
        if let Some(reference) = reference {
            assert_eq!(reference.kind, ModuleReferenceKind::Require);
            assert_eq!(reference.module_name, "Foo::Bar");
        }
    }

    #[test]
    fn canonicalizes_legacy_separator() {
        let text = "use Foo'Bar;";
        let cursor = text.find("Bar").unwrap_or(0);

        assert_eq!(extract_module_reference(text, cursor), Some("Foo::Bar".to_string()));
    }

    #[test]
    fn rejects_non_direct_import_forms() {
        assert_eq!(find_module_reference("use parent 'Foo::Bar';", 15), None);
        assert_eq!(find_module_reference("require 'Foo/Bar.pm';", 10), None);
    }

    #[test]
    fn cursor_at_token_end_is_accepted() {
        let text = "use Foo::Bar;";
        let token_end = "use Foo::Bar".len();
        assert_eq!(extract_module_reference(text, token_end), Some("Foo::Bar".to_string()));
    }

    #[test]
    fn parse_module_token_rejects_invalid_segments() {
        assert_eq!(parse_module_token("Foo::", 0), None);
        assert_eq!(parse_module_token("Foo'", 0), None);
        assert_eq!(parse_module_token("5_10", 0), None);
    }
}
