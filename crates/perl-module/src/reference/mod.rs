//! Cursor-aware Perl module reference extraction.
//!
//! Given source text and a cursor offset, identify module references used
//! by `use`/`require` statements.

use crate::name::normalize_package_separator;
use crate::token_parser::parse_module_token;
use perl_parser_core::text_line::{is_keyword_boundary, line_bounds_at, skip_ascii_whitespace};

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
#[must_use]
pub fn extract_module_reference(text: &str, cursor_pos: usize) -> Option<String> {
    find_module_reference(text, cursor_pos).map(|reference| reference.canonical_module_name())
}

/// Extract a module reference at `cursor_pos` as a canonical module name,
/// including `use parent`/`use base` argument modules.
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
    find_in_line_for_keyword(
        line,
        line_offset,
        cursor_in_line,
        "use",
        ModuleReferenceKind::Use,
    )
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

    let args_area = after_keyword;
    let args_start_in_line = leading_ws + "use ".len() + (rest.len() - after_keyword.len());

    let bytes = args_area.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];

        if !is_module_start_byte(b) {
            i += 1;
            continue;
        }

        let token_start_in_args = i;
        let token_end_in_args = scan_canonical_module_token(bytes, i);
        let token_start_in_line = args_start_in_line + token_start_in_args;
        let token_end_in_line = args_start_in_line + token_end_in_args;
        let module_name = &args_area[token_start_in_args..token_end_in_args];

        let is_module_like = module_name.contains("::")
            || module_name
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_uppercase);

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

fn scan_canonical_module_token(bytes: &[u8], start: usize) -> usize {
    let mut i = start;

    loop {
        while i < bytes.len() && is_identifier_byte(bytes[i]) {
            i += 1;
        }

        if i + 1 < bytes.len()
            && bytes[i] == b':'
            && bytes[i + 1] == b':'
            && i + 2 < bytes.len()
            && is_module_start_byte(bytes[i + 2])
        {
            i += 2;
        } else {
            break;
        }
    }

    i
}

fn is_module_start_byte(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

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
