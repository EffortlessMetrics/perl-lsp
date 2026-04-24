use perl_symbol::cursor::{
    CursorSymbolKind, extract_symbol_from_source, get_symbol_range_at_position, token_under_cursor,
};
use perl_tdd_support::must_some;

type Result<T> = std::result::Result<T, String>;

fn char_pos_of(haystack: &str, needle: &str) -> Result<usize> {
    let byte = haystack
        .find(needle)
        .ok_or_else(|| format!("missing {needle:?} in fixture"))?;
    Ok(haystack[..byte].chars().count())
}

fn char_slice(text: &str, start: usize, end: usize) -> String {
    text.chars().skip(start).take(end - start).collect()
}

fn utf16_col_of(haystack: &str, needle: &str) -> Result<usize> {
    let byte = haystack
        .find(needle)
        .ok_or_else(|| format!("missing {needle:?} in fixture"))?;
    Ok(haystack[..byte].encode_utf16().count())
}

#[test]
fn byte_api_multibyte_near_cursor_extracts_scalar() -> Result<()> {
    let source = "my 😀 $value = 1;";
    let pos = char_pos_of(source, "value")?;
    let (name, kind) = must_some(extract_symbol_from_source(pos, source));
    assert_eq!(name, "value");
    assert_eq!(kind, CursorSymbolKind::Scalar);
    Ok(())
}

#[test]
fn byte_api_cursor_in_middle_of_bareword_extracts_suffix_from_cursor() {
    let source = "invoke_handler();";
    let pos = 8;
    let (name, kind) = must_some(extract_symbol_from_source(pos, source));
    assert_eq!(name, "andler");
    assert_eq!(kind, CursorSymbolKind::Subroutine);
}

#[test]
fn byte_api_cursor_on_sigil_and_after_sigil_match() {
    let source = "$counter";
    let on_sigil = must_some(extract_symbol_from_source(0, source));
    let after_sigil = must_some(extract_symbol_from_source(1, source));
    assert_eq!(on_sigil, after_sigil);
}

#[test]
fn byte_api_qualified_name_extract_stops_at_colon() -> Result<()> {
    let source = "My::Pkg::run";
    let (left, left_kind) = must_some(extract_symbol_from_source(0, source));
    let right_pos = char_pos_of(source, "run")?;
    let (right, right_kind) = must_some(extract_symbol_from_source(right_pos, source));
    assert_eq!(left, "My");
    assert_eq!(right, "run");
    assert_eq!(left_kind, CursorSymbolKind::Subroutine);
    assert_eq!(right_kind, CursorSymbolKind::Subroutine);
    Ok(())
}

#[test]
fn byte_api_typeglob_like_case_degrades_conservatively() {
    let source = "*glob{CODE}";
    let pos = 1;
    let (start, end) = must_some(get_symbol_range_at_position(pos, source));
    assert_eq!(char_slice(source, start, end), "glob");
}

#[test]
fn byte_api_deref_like_double_sigil_returns_none_on_second_sigil() {
    let source = "$$ref";
    assert!(extract_symbol_from_source(1, source).is_none());
}

#[test]
fn byte_api_extract_and_range_are_symmetric_for_scalar() -> Result<()> {
    let source = "my $total = 42;";
    let pos = char_pos_of(source, "total")?;
    let (name, kind) = must_some(extract_symbol_from_source(pos, source));
    let (start, end) = must_some(get_symbol_range_at_position(pos, source));

    assert_eq!(kind, CursorSymbolKind::Scalar);
    assert_eq!(char_slice(source, start, end), format!("${name}"));
    Ok(())
}

#[test]
fn utf16_api_multibyte_near_cursor_extracts_same_token() -> Result<()> {
    let text = "my 😀 $value = 1;\n";
    let col = utf16_col_of(text, "value")?;
    assert_eq!(token_under_cursor(text, 0, col), Some("$value".to_string()));
    Ok(())
}

#[test]
fn utf16_api_package_qualified_name_includes_colons() -> Result<()> {
    let text = "use Demo::Worker;\n";
    let col = utf16_col_of(text, "Worker")?;
    assert_eq!(token_under_cursor(text, 0, col), Some("Demo::Worker".to_string()));
    Ok(())
}

#[test]
fn utf16_api_crlf_line_handling_extracts_second_line_token() -> Result<()> {
    let text = "my $x = 1;\r\nmy $second = 2;\r\n";
    let second_line = "my $second = 2;";
    let col = utf16_col_of(second_line, "second")?;
    assert_eq!(token_under_cursor(text, 1, col), Some("$second".to_string()));
    Ok(())
}

#[test]
fn parity_ascii_scalar_span_matches_between_byte_and_utf16_apis() -> Result<()> {
    let source = "my $value = 1;";
    let byte_pos = char_pos_of(source, "value")?;

    let (name, kind) = must_some(extract_symbol_from_source(byte_pos, source));
    let (start, end) = must_some(get_symbol_range_at_position(byte_pos, source));
    let from_range = char_slice(source, start, end);

    let col_utf16 = utf16_col_of(source, "value")?;
    let from_utf16 = must_some(token_under_cursor(source, 0, col_utf16));

    assert_eq!(kind, CursorSymbolKind::Scalar);
    assert_eq!(from_range, format!("${name}"));
    assert_eq!(from_range, from_utf16);
    Ok(())
}
