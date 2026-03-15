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

/// Find a module reference inside `use parent`/`use base` argument lists.
///
/// When the cursor is on a quoted module name inside `use parent 'Foo::Bar'`
/// or `use base qw(Foo::Bar)`, this returns the referenced module name.
/// For direct `use`/`require` statements, delegates to [`find_module_reference`].
#[must_use]
pub fn find_module_reference_extended(
    text: &str,
    cursor_pos: usize,
) -> Option<ModuleReference<'_>> {
    // First try direct module reference (use Foo::Bar, require Foo::Bar)
    if let Some(reference) = find_module_reference(text, cursor_pos) {
        return Some(reference);
    }

    if text.is_empty() || cursor_pos > text.len() {
        return None;
    }

    let (line_start, line_end) = line_bounds_at(text, cursor_pos);
    let line = &text[line_start..line_end];
    let cursor_in_line = cursor_pos.saturating_sub(line_start);

    find_parent_base_module_in_line(line, line_start, cursor_in_line)
}

/// Extract a module reference at `cursor_pos` as a canonical module name.
///
/// Returns canonical `::` separators even when source uses legacy `'`
/// separators.
#[must_use]
pub fn extract_module_reference(text: &str, cursor_pos: usize) -> Option<String> {
    find_module_reference(text, cursor_pos).map(|reference| reference.canonical_module_name())
}

/// Extract a module reference at `cursor_pos` as a canonical module name,
/// including `use parent`/`use base` argument modules.
///
/// This is the extended version that also resolves quoted module names
/// inside `use parent 'Module::Name'` and `use base qw(Module::Name)`.
#[must_use]
pub fn extract_module_reference_extended(text: &str, cursor_pos: usize) -> Option<String> {
    find_module_reference_extended(text, cursor_pos)
        .map(|reference| reference.canonical_module_name())
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

/// Check if a line starts with `use parent` or `use base` and extract the
/// quoted module name under the cursor from the argument list.
fn find_parent_base_module_in_line<'a>(
    line: &'a str,
    line_offset: usize,
    cursor_in_line: usize,
) -> Option<ModuleReference<'a>> {
    let trimmed = line.trim_start();
    let leading_ws = line.len().saturating_sub(trimmed.len());

    let rest = trimmed.strip_prefix("use")?;
    if !rest.starts_with(|c: char| c.is_whitespace()) {
        return None;
    }
    let rest = rest.trim_start();

    // Check for parent or base keyword
    let is_parent = rest.starts_with("parent");
    let is_base = rest.starts_with("base");
    if !is_parent && !is_base {
        return None;
    }

    let keyword = if is_parent { "parent" } else { "base" };
    let after_keyword = &rest[keyword.len()..];
    if !after_keyword.is_empty() && !after_keyword.starts_with(|c: char| c.is_whitespace()) {
        return None;
    }

    // Scan the argument area for module-like tokens (Foo::Bar style).
    // These appear inside quotes ('Module::Name', "Module::Name") or qw(Module::Name).
    // We use a simple scanner that finds identifiers joined by `::`.
    let args_area = after_keyword;
    let args_start_in_line = leading_ws + "use ".len() + (rest.len() - after_keyword.len());

    let bytes = args_area.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];

        // Skip non-identifier-start characters (quotes, whitespace, parens, commas, etc.)
        if !is_module_start_byte(b) {
            i += 1;
            continue;
        }

        // Found start of a potential module token -- scan using `::` separators only
        // (not `'`, which is a string delimiter in this context)
        let token_start_in_args = i;
        let token_end_in_args = scan_canonical_module_token(bytes, i);
        let token_start_in_line = args_start_in_line + token_start_in_args;
        let token_end_in_line = args_start_in_line + token_end_in_args;
        let module_name = &args_area[token_start_in_args..token_end_in_args];

        // Only consider tokens that contain `::` (i.e., actual module names)
        // or start with an uppercase letter (single-segment module names like `Carp`)
        let is_module_like = module_name.contains("::")
            || module_name.as_bytes().first().is_some_and(u8::is_ascii_uppercase);

        if is_module_like
            && cursor_in_line >= token_start_in_line
            && cursor_in_line <= token_end_in_line
        {
            return Some(ModuleReference {
                kind: ModuleReferenceKind::Use,
                module_name,
                module_start: line_offset + token_start_in_line,
                module_end: line_offset + token_end_in_line,
            });
        }

        i = token_end_in_args;
    }

    None
}

/// Scan a module token using only canonical `::` separators (not legacy `'`).
///
/// Returns the exclusive end offset of the token in the byte slice.
fn scan_canonical_module_token(bytes: &[u8], start: usize) -> usize {
    let mut i = start;

    loop {
        // Scan one identifier segment
        while i < bytes.len() && is_identifier_byte(bytes[i]) {
            i += 1;
        }

        // Check for `::` separator followed by another identifier segment
        if i + 1 < bytes.len()
            && bytes[i] == b':'
            && bytes[i + 1] == b':'
            && i + 2 < bytes.len()
            && is_module_start_byte(bytes[i + 2])
        {
            i += 2; // skip `::`
        } else {
            break;
        }
    }

    i
}

/// Check if a byte can start a Perl module/identifier name.
fn is_module_start_byte(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

/// Check if a byte is a valid identifier continuation character.
fn is_identifier_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
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
    use super::{
        ModuleReferenceKind, extract_module_reference, extract_module_reference_extended,
        find_module_reference, find_module_reference_extended,
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
    fn ignores_invalid_reference_tokens() {
        assert_eq!(find_module_reference("use Foo::", 0), None);
        assert_eq!(find_module_reference("use Foo'", 0), None);
        assert_eq!(find_module_reference("5_10", 0), None);
    }

    // Extended reference tests for use parent / use base

    #[test]
    fn extended_finds_parent_single_quoted_module() {
        let text = "use parent 'Foo::Bar';";
        let cursor = text.find("Foo::Bar").unwrap_or(0);

        let reference = find_module_reference_extended(text, cursor);
        assert!(reference.is_some());
        if let Some(reference) = reference {
            assert_eq!(reference.kind, ModuleReferenceKind::Use);
            assert_eq!(reference.module_name, "Foo::Bar");
        }
    }

    #[test]
    fn extended_finds_base_single_quoted_module() {
        let text = "use base 'Foo::Bar';";
        let cursor = text.find("Foo::Bar").unwrap_or(0);

        let reference = find_module_reference_extended(text, cursor);
        assert!(reference.is_some());
        if let Some(reference) = reference {
            assert_eq!(reference.kind, ModuleReferenceKind::Use);
            assert_eq!(reference.module_name, "Foo::Bar");
        }
    }

    #[test]
    fn extended_finds_parent_qw_module() {
        let text = "use parent qw(Foo::Bar Baz::Qux);";
        let cursor = text.find("Baz::Qux").unwrap_or(0);

        let reference = find_module_reference_extended(text, cursor);
        assert!(reference.is_some());
        if let Some(reference) = reference {
            assert_eq!(reference.module_name, "Baz::Qux");
        }
    }

    #[test]
    fn extended_finds_parent_double_quoted_module() {
        let text = r#"use parent "Foo::Bar";"#;
        let cursor = text.find("Foo::Bar").unwrap_or(0);

        assert_eq!(extract_module_reference_extended(text, cursor), Some("Foo::Bar".to_string()));
    }

    #[test]
    fn extended_still_finds_direct_use() {
        let text = "use File::Basename;";
        let cursor = text.find("Basename").unwrap_or(0);

        assert_eq!(
            extract_module_reference_extended(text, cursor),
            Some("File::Basename".to_string())
        );
    }

    #[test]
    fn extended_returns_none_for_cursor_outside_token() {
        let text = "use parent 'Foo::Bar';";
        // cursor on the quote character
        let cursor = text.find('\'').unwrap_or(0);

        assert_eq!(find_module_reference_extended(text, cursor), None);
    }

    #[test]
    fn extended_returns_none_for_empty_text() {
        assert_eq!(find_module_reference_extended("", 0), None);
    }

    #[test]
    fn extended_does_not_match_use_parenthetical() {
        // "use parentModule" should not match (no word boundary after "parent")
        let text = "use parentModule 'Foo::Bar';";
        let cursor = text.find("Foo::Bar").unwrap_or(0);

        assert_eq!(find_module_reference_extended(text, cursor), None);
    }
}
