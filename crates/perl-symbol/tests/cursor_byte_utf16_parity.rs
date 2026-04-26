//! Focused cursor regressions that separate source-position APIs from
//! UTF-16 line/column behavior.

use perl_symbol::cursor::{
    CursorSymbolKind, byte_offset_utf16, extract_symbol_from_source, get_symbol_range_at_position,
    token_under_cursor,
};
use perl_tdd_support::must_some;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn char_span(source: &str, start: usize, end: usize) -> String {
    source.chars().skip(start).take(end - start).collect()
}

fn utf16_col_for_byte(line: &str, byte_idx: usize) -> usize {
    let mut units = 0;
    for (idx, ch) in line.char_indices() {
        if idx >= byte_idx {
            return units;
        }
        units += ch.len_utf16();
    }
    units
}

// ─── byte-oriented source-position APIs ──────────────────────────────────────

#[test]
fn byte_cursor_multibyte_prefix_extract_and_range_remain_stable() -> Result<()> {
    let source = "my 😀 $value = 1;";
    let pos = source.chars().position(|c| c == 'v').ok_or("missing value token")?;

    let (name, kind) = must_some(extract_symbol_from_source(pos, source));
    let (start, end) = must_some(get_symbol_range_at_position(pos, source));

    assert_eq!(name, "value");
    assert_eq!(kind, CursorSymbolKind::Scalar);
    assert_eq!(char_span(source, start, end), "$value");
    Ok(())
}

#[test]
fn byte_cursor_middle_of_bareword_extracts_suffix_and_range_matches_it() -> Result<()> {
    let source = "compute_value();";
    let pos = source.chars().position(|c| c == 'v').ok_or("missing v")?;

    let (name, kind) = must_some(extract_symbol_from_source(pos, source));
    let (start, end) = must_some(get_symbol_range_at_position(pos, source));

    assert_eq!(name, "value");
    assert_eq!(kind, CursorSymbolKind::Subroutine);
    // With cursor in the middle, both APIs currently return the forward suffix.
    assert_eq!(char_span(source, start, end), "value");
    Ok(())
}

#[test]
fn byte_cursor_on_sigil_extracts_name_but_range_is_empty_conservative_span() -> Result<()> {
    let source = "$item";
    let (name, kind) = must_some(extract_symbol_from_source(0, source));
    let (start, end) = must_some(get_symbol_range_at_position(0, source));

    assert_eq!(name, "item");
    assert_eq!(kind, CursorSymbolKind::Scalar);
    assert_eq!((start, end), (0, 0));
    Ok(())
}

#[test]
fn byte_cursor_after_sigil_extract_and_range_include_same_symbol() -> Result<()> {
    let source = "$item";
    let (name, kind) = must_some(extract_symbol_from_source(1, source));
    let (start, end) = must_some(get_symbol_range_at_position(1, source));

    assert_eq!(name, "item");
    assert_eq!(kind, CursorSymbolKind::Scalar);
    assert_eq!(char_span(source, start, end), "$item");
    Ok(())
}

#[test]
fn byte_cursor_package_qualified_name_extract_degrades_to_segment() -> Result<()> {
    let source = "My::Pkg::run";
    let first = must_some(extract_symbol_from_source(0, source));
    let run_pos = source.chars().position(|c| c == 'r').ok_or("missing run")?;
    let last = must_some(extract_symbol_from_source(run_pos, source));

    assert_eq!(first.0, "My");
    assert_eq!(last.0, "run");
    assert_eq!(first.1, CursorSymbolKind::Subroutine);
    assert_eq!(last.1, CursorSymbolKind::Subroutine);
    Ok(())
}

#[test]
fn byte_cursor_double_deref_on_second_sigil_returns_none() {
    let source = "$$ref";
    assert!(extract_symbol_from_source(1, source).is_none());
}

#[test]
fn byte_cursor_double_deref_on_name_returns_scalar_ref_name() -> Result<()> {
    let source = "$$ref";
    let pos = source.chars().position(|c| c == 'r').ok_or("missing ref")?;

    let (name, kind) = must_some(extract_symbol_from_source(pos, source));
    let (start, end) = must_some(get_symbol_range_at_position(pos, source));

    assert_eq!(name, "ref");
    assert_eq!(kind, CursorSymbolKind::Scalar);
    assert_eq!(char_span(source, start, end), "$ref");
    Ok(())
}

#[test]
fn byte_extract_and_range_symmetry_for_simple_scalar() -> Result<()> {
    let source = "my $count = 0;";
    let pos = source.chars().position(|c| c == 'c').ok_or("missing count")?;

    let (name, _) = must_some(extract_symbol_from_source(pos, source));
    let (start, end) = must_some(get_symbol_range_at_position(pos, source));

    assert_eq!(char_span(source, start + 1, end), name);
    assert_eq!(char_span(source, start, end), "$count");
    Ok(())
}

// ─── UTF-16 line/column API ──────────────────────────────────────────────────

#[test]
fn utf16_token_under_cursor_preserves_qualified_name() {
    let text = "use Demo::Worker;\n";
    assert_eq!(token_under_cursor(text, 0, 8), Some("Demo::Worker".to_string()));
}

#[test]
fn utf16_token_under_cursor_multibyte_prefix_column_parity() -> Result<()> {
    let text = "my 😀 Demo::Worker;\n";
    let line = "my 😀 Demo::Worker;";
    let w_byte = line.find('W').ok_or("fixture includes W")?;
    let utf16_col_on_w = utf16_col_for_byte(line, w_byte);

    let byte = byte_offset_utf16(line, utf16_col_on_w);
    assert_eq!(line.as_bytes()[byte], b'W');
    assert_eq!(token_under_cursor(text, 0, utf16_col_on_w), Some("Demo::Worker".to_string()));
    Ok(())
}

#[test]
fn utf16_token_under_cursor_handles_crlf_lines() {
    let text = "use Demo::Worker;\r\nmy $x = 1;\r\n";
    assert_eq!(token_under_cursor(text, 0, 8), Some("Demo::Worker".to_string()));
    assert_eq!(token_under_cursor(text, 1, 4), Some("$x".to_string()));
}

#[test]
fn utf16_and_source_position_apis_agree_on_symbol_text_after_multibyte_prefix() -> Result<()> {
    let line = "my 😀 $value = 1;";
    let text = format!("{line}\n");

    // UTF-16 column 8 in "my 😀 $value": m(0) y(1) ' '(2) 😀(3,4 surrogate pair) ' '(5) $(6) v(7) a(8).
    // col 8 lands on 'a', not 'v'. token_under_cursor extends backward to include the full $value token.
    let utf16_col_on_v = 8;
    let token = must_some(token_under_cursor(&text, 0, utf16_col_on_v));

    let source_pos = line.chars().position(|c| c == 'v').ok_or("missing v")?;
    let (start, end) = must_some(get_symbol_range_at_position(source_pos, line));

    assert_eq!(token, char_span(line, start, end));
    Ok(())
}
