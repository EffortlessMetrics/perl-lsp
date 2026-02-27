//! Cursor-aware Perl module reference extraction.
//!
//! This crate has one responsibility: given source text and a cursor offset,
//! identify module references used by `use`/`require` statements.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use perl_module_name::normalize_package_separator;
use perl_module_token_parser::parse_module_token;
use perl_text_line::{is_keyword_boundary, line_bounds_at, skip_ascii_whitespace};

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

        if let Some(span) = parse_module_token(line, module_start)
            && cursor_in_line >= module_start
            && cursor_in_line <= span.end
        {
            return Some(ModuleReference {
                kind,
                module_name: &line[module_start..span.end],
                module_start: line_offset + module_start,
                module_end: line_offset + span.end,
            });
        }

        idx += 1;
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{ModuleReferenceKind, extract_module_reference, find_module_reference};

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
    fn ignores_invalid_reference_tokens() {
        assert_eq!(find_module_reference("use Foo::", 0), None);
        assert_eq!(find_module_reference("use Foo'", 0), None);
        assert_eq!(find_module_reference("5_10", 0), None);
    }
}
