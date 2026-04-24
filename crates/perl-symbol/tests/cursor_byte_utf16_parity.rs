//! Focused cursor regressions for byte/char-position APIs vs UTF-16 APIs.
//!
//! This suite intentionally separates:
//! - `extract_symbol_from_source` / `get_symbol_range_at_position` (position-in-source)
//! - `token_under_cursor` (line + UTF-16 column)

use perl_symbol::cursor::{
    CursorSymbolKind, extract_symbol_from_source, get_symbol_range_at_position, token_under_cursor,
};
use perl_tdd_support::must_some;

fn char_slice(source: &str, start: usize, end: usize) -> String {
    source.chars().skip(start).take(end.saturating_sub(start)).collect()
}

fn utf16_col_for_char_index(line: &str, char_index: usize) -> usize {
    line.chars().take(char_index).map(char::len_utf16).sum()
}

fn find_char(source: &str, needle: char) -> Result<usize, String> {
    source
        .chars()
        .position(|ch| ch == needle)
        .ok_or_else(|| format!("missing {needle:?} in fixture"))
}

// ─── Byte/char-position cursor APIs ──────────────────────────────────────────

#[test]
fn byte_cursor_multibyte_prefix_extracts_scalar_name() -> Result<(), String> {
    let source = "😀 my $name = 1;";
    let pos = find_char(source, 'n')?;
    let (name, kind) = must_some(extract_symbol_from_source(pos, source));
    assert_eq!(name, "name");
    assert_eq!(kind, CursorSymbolKind::Scalar);
    Ok(())
}

#[test]
fn byte_cursor_middle_of_bareword_extracts_suffix_regression() -> Result<(), String> {
    let source = "module();";
    let pos = find_char(source, 'd')?;
    let (name, kind) = must_some(extract_symbol_from_source(pos, source));
    assert_eq!(name, "dule");
    assert_eq!(kind, CursorSymbolKind::Subroutine);
    Ok(())
}

#[test]
fn byte_cursor_middle_of_bareword_range_tracks_suffix_regression() -> Result<(), String> {
    let source = "module();";
    let pos = find_char(source, 'd')?;
    let (start, end) = must_some(get_symbol_range_at_position(pos, source));
    assert_eq!((start, end), (pos, pos + 4));
    assert_eq!(char_slice(source, start, end), "dule");
    Ok(())
}

#[test]
fn byte_cursor_on_sigil_and_after_sigil_extract_same_symbol() {
    let source = "$value";
    let on_sigil = must_some(extract_symbol_from_source(0, source));
    let after_sigil = must_some(extract_symbol_from_source(1, source));
    assert_eq!(on_sigil, after_sigil);
}

#[test]
fn byte_cursor_qualified_name_splits_at_double_colon() {
    let source = "Pkg::Name";
    let on_pkg = must_some(extract_symbol_from_source(0, source));
    let on_name = must_some(extract_symbol_from_source(5, source));
    assert_eq!(on_pkg, ("Pkg".to_string(), CursorSymbolKind::Subroutine));
    assert_eq!(on_name, ("Name".to_string(), CursorSymbolKind::Subroutine));
}

#[test]
fn byte_cursor_deref_style_on_second_dollar_degrades_to_none() {
    let source = "$$ref";
    assert!(extract_symbol_from_source(1, source).is_none());
}

#[test]
fn byte_cursor_typeglob_style_star_degrades_to_none() {
    let source = "*STDOUT";
    assert!(extract_symbol_from_source(0, source).is_none());
    assert_eq!(must_some(extract_symbol_from_source(1, source)).0, "STDOUT");
}

#[test]
fn byte_cursor_extract_and_range_are_symmetric_for_scalar() -> Result<(), String> {
    let source = "my $count = 1;";
    let pos = find_char(source, 'c')?;

    let (name, kind) = must_some(extract_symbol_from_source(pos, source));
    let (start, end) = must_some(get_symbol_range_at_position(pos, source));

    assert_eq!(kind, CursorSymbolKind::Scalar);
    assert_eq!(char_slice(source, start, end), "$count");
    assert_eq!(end - start, name.len() + 1);
    Ok(())
}

// ─── UTF-16 line/column cursor API ───────────────────────────────────────────

#[test]
fn utf16_cursor_multibyte_prefix_keeps_token_detection_stable() -> Result<(), String> {
    let line = "😀 my $value = 1;";
    let text = format!("{line}\n");
    let char_index = find_char(line, 'a')?;
    let col_utf16 = utf16_col_for_char_index(line, char_index);

    let token = must_some(token_under_cursor(&text, 0, col_utf16));
    assert_eq!(token, "$value");
    Ok(())
}

#[test]
fn utf16_cursor_on_sigil_and_after_sigil_return_same_token() {
    let line = "$value = 1;";
    let text = format!("{line}\n");
    let on_sigil = must_some(token_under_cursor(&text, 0, 0));
    let after_sigil = must_some(token_under_cursor(&text, 0, 1));
    assert_eq!(on_sigil, "");
    assert_eq!(after_sigil, "$value");
}

#[test]
fn utf16_cursor_package_qualified_name_includes_double_colon() {
    let line = "use Demo::Worker;";
    let text = format!("{line}\n");
    let col_utf16 = utf16_col_for_char_index(line, 8);
    let token = must_some(token_under_cursor(&text, 0, col_utf16));
    assert_eq!(token, "Demo::Worker");
}

#[test]
fn utf16_cursor_handles_crlf_second_line() {
    let text = "my $x = 1;\r\ncall Demo::Worker;\r\n";
    let line = "call Demo::Worker;\r";
    let col_utf16 = utf16_col_for_char_index(line, 10);
    let token = must_some(token_under_cursor(text, 1, col_utf16));
    assert_eq!(token, "Demo::Worker");
}

#[test]
fn utf16_and_byte_position_apis_agree_for_ascii_scalar_span() -> Result<(), String> {
    let line = "my $value = 1;";
    let text = format!("{line}\n");

    let pos = find_char(line, 'v')?;
    let col_utf16 = utf16_col_for_char_index(line, pos);

    let utf16_token = must_some(token_under_cursor(&text, 0, col_utf16));
    let (start, end) = must_some(get_symbol_range_at_position(pos, line));
    let range_token = char_slice(line, start, end);

    assert_eq!(utf16_token, "$value");
    assert_eq!(range_token, utf16_token);
    Ok(())
}
