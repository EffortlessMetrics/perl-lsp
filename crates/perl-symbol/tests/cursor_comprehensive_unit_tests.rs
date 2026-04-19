//! Comprehensive unit tests for perl-symbol-cursor.
//!
//! Covers all public types and functions:
//! - `CursorSymbolKind` enum variants and trait impls
//! - `extract_symbol_from_source` — sigil detection, edge cases, defaults
//! - `get_symbol_range_at_position` — range extraction with sigils

use perl_symbol::cursor::{
    CursorSymbolKind, extract_symbol_from_source, get_symbol_range_at_position,
};
use perl_tdd_support::must_some;

// ─── CursorSymbolKind trait impls ────────────────────────────────────────────

#[test]
fn kind_debug_display() {
    // Verify Debug impl produces non-empty output for all variants
    let variants = [
        CursorSymbolKind::Scalar,
        CursorSymbolKind::Array,
        CursorSymbolKind::Hash,
        CursorSymbolKind::Subroutine,
    ];
    for v in &variants {
        let dbg = format!("{v:?}");
        assert!(!dbg.is_empty(), "Debug output should be non-empty");
    }
}

#[test]
fn kind_clone_and_copy() {
    let original = CursorSymbolKind::Scalar;
    let cloned: CursorSymbolKind = { original };
    let copied = original;
    assert_eq!(original, cloned);
    assert_eq!(original, copied);
}

#[test]
fn kind_eq_and_ne() {
    assert_eq!(CursorSymbolKind::Scalar, CursorSymbolKind::Scalar);
    assert_ne!(CursorSymbolKind::Scalar, CursorSymbolKind::Array);
    assert_ne!(CursorSymbolKind::Hash, CursorSymbolKind::Subroutine);
}

// ─── extract_symbol_from_source — basic sigil variants ───────────────────────

#[test]
fn extract_scalar_cursor_after_sigil() -> Result<(), String> {
    let source = "my $name = 1;";
    let pos = source
        .find('n')
        .ok_or_else(|| "missing 'n' in fixture".to_string())?;
    let (name, kind) = must_some(extract_symbol_from_source(pos, source));
    assert_eq!(name, "name");
    assert_eq!(kind, CursorSymbolKind::Scalar);
    Ok(())
}

#[test]
fn extract_array_cursor_after_sigil() -> Result<(), String> {
    let source = "my @items;";
    let pos = source
        .find('i')
        .ok_or_else(|| "missing 'i' in fixture".to_string())?;
    let (name, kind) = must_some(extract_symbol_from_source(pos, source));
    assert_eq!(name, "items");
    assert_eq!(kind, CursorSymbolKind::Array);
    Ok(())
}

#[test]
fn extract_hash_cursor_after_sigil() -> Result<(), String> {
    let source = "my %config;";
    let pos = source
        .find('c')
        .ok_or_else(|| "missing 'c' in fixture".to_string())?;
    let (name, kind) = must_some(extract_symbol_from_source(pos, source));
    assert_eq!(name, "config");
    assert_eq!(kind, CursorSymbolKind::Hash);
    Ok(())
}

#[test]
fn extract_subroutine_cursor_after_ampersand() -> Result<(), String> {
    let source = "&process();";
    let pos = source
        .find('p')
        .ok_or_else(|| "missing 'p' in fixture".to_string())?;
    let (name, kind) = must_some(extract_symbol_from_source(pos, source));
    assert_eq!(name, "process");
    assert_eq!(kind, CursorSymbolKind::Subroutine);
    Ok(())
}

// ─── extract_symbol_from_source — cursor on sigil itself ─────────────────────

#[test]
fn extract_cursor_on_dollar_sigil() -> Result<(), String> {
    let source = "$foo";
    let (name, kind) = must_some(extract_symbol_from_source(0, source));
    assert_eq!(name, "foo");
    assert_eq!(kind, CursorSymbolKind::Scalar);
    Ok(())
}

#[test]
fn extract_cursor_on_at_sigil() -> Result<(), String> {
    let source = "@arr";
    let (name, kind) = must_some(extract_symbol_from_source(0, source));
    assert_eq!(name, "arr");
    assert_eq!(kind, CursorSymbolKind::Array);
    Ok(())
}

#[test]
fn extract_cursor_on_percent_sigil() -> Result<(), String> {
    let source = "%hash";
    let (name, kind) = must_some(extract_symbol_from_source(0, source));
    assert_eq!(name, "hash");
    assert_eq!(kind, CursorSymbolKind::Hash);
    Ok(())
}

#[test]
fn extract_cursor_on_ampersand_sigil() -> Result<(), String> {
    let source = "&sub_call";
    let (name, kind) = must_some(extract_symbol_from_source(0, source));
    assert_eq!(name, "sub_call");
    assert_eq!(kind, CursorSymbolKind::Subroutine);
    Ok(())
}

// ─── extract_symbol_from_source — no sigil (bare word) ──────────────────────

#[test]
fn extract_bare_word_defaults_to_subroutine() -> Result<(), String> {
    let source = "calculate();";
    let (name, kind) = must_some(extract_symbol_from_source(0, source));
    assert_eq!(name, "calculate");
    assert_eq!(kind, CursorSymbolKind::Subroutine);
    Ok(())
}

#[test]
fn extract_bare_word_mid_position() -> Result<(), String> {
    let source = "my_func();";
    // cursor in the middle of "my_func"
    let (name, kind) = must_some(extract_symbol_from_source(3, source));
    assert_eq!(name, "func");
    assert_eq!(kind, CursorSymbolKind::Subroutine);
    Ok(())
}

// ─── extract_symbol_from_source — underscores and digits ─────────────────────

#[test]
fn extract_name_with_underscores() -> Result<(), String> {
    let source = "$my_long_name";
    let (name, kind) = must_some(extract_symbol_from_source(1, source));
    assert_eq!(name, "my_long_name");
    assert_eq!(kind, CursorSymbolKind::Scalar);
    Ok(())
}

#[test]
fn extract_name_with_digits() -> Result<(), String> {
    let source = "$var123";
    let (name, kind) = must_some(extract_symbol_from_source(1, source));
    assert_eq!(name, "var123");
    assert_eq!(kind, CursorSymbolKind::Scalar);
    Ok(())
}

#[test]
fn extract_name_starting_with_underscore() -> Result<(), String> {
    let source = "$_private";
    let (name, kind) = must_some(extract_symbol_from_source(1, source));
    assert_eq!(name, "_private");
    assert_eq!(kind, CursorSymbolKind::Scalar);
    Ok(())
}

// ─── extract_symbol_from_source — edge cases ────────────────────────────────

#[test]
fn extract_empty_source_returns_none() {
    let result = extract_symbol_from_source(0, "");
    assert_eq!(result, None);
}

#[test]
fn extract_position_beyond_length_returns_none() {
    let result = extract_symbol_from_source(100, "short");
    assert_eq!(result, None);
}

#[test]
fn extract_position_at_exact_length_returns_none() {
    let source = "abc";
    let result = extract_symbol_from_source(source.len(), source);
    assert_eq!(result, None);
}

#[test]
fn extract_cursor_on_whitespace_returns_none() {
    let source = "a b";
    // position 1 is space, no sigil before or at position, and space is not alnum/_
    let result = extract_symbol_from_source(1, source);
    assert_eq!(result, None);
}

#[test]
fn extract_cursor_on_semicolon_returns_none() {
    let source = ";";
    let result = extract_symbol_from_source(0, source);
    assert_eq!(result, None);
}

#[test]
fn extract_sigil_only_no_name_returns_none() {
    // $ at position 0 with no following alnum chars
    let source = "$ ";
    let result = extract_symbol_from_source(0, source);
    assert_eq!(result, None);
}

#[test]
fn extract_sigil_at_end_of_source_returns_none() {
    let source = "x$";
    // cursor on $, position 1, name_start would be 2 which equals chars.len()
    let result = extract_symbol_from_source(1, source);
    assert_eq!(result, None);
}

#[test]
fn extract_single_char_name() -> Result<(), String> {
    let source = "$x";
    let (name, kind) = must_some(extract_symbol_from_source(1, source));
    assert_eq!(name, "x");
    assert_eq!(kind, CursorSymbolKind::Scalar);
    Ok(())
}

#[test]
fn extract_cursor_at_position_zero_no_sigil_before() -> Result<(), String> {
    let source = "foo";
    let (name, kind) = must_some(extract_symbol_from_source(0, source));
    assert_eq!(name, "foo");
    assert_eq!(kind, CursorSymbolKind::Subroutine);
    Ok(())
}

// ─── extract_symbol_from_source — multiple symbols in line ──────────────────

#[test]
fn extract_second_symbol_in_line() -> Result<(), String> {
    let source = "$a + $b";
    // cursor on 'b' at position 6
    let pos = source.rfind('b').ok_or_else(|| "missing 'b'".to_string())?;
    let (name, kind) = must_some(extract_symbol_from_source(pos, source));
    assert_eq!(name, "b");
    assert_eq!(kind, CursorSymbolKind::Scalar);
    Ok(())
}

#[test]
fn extract_first_symbol_in_line() -> Result<(), String> {
    let source = "$a + $b";
    let (name, kind) = must_some(extract_symbol_from_source(1, source));
    assert_eq!(name, "a");
    assert_eq!(kind, CursorSymbolKind::Scalar);
    Ok(())
}

// ─── extract_symbol_from_source — qualified names stop at colon ─────────────

#[test]
fn extract_stops_at_colon_in_qualified_name() -> Result<(), String> {
    // The scanner stops at non-alnum/non-underscore chars
    let source = "Foo::bar";
    let (name, kind) = must_some(extract_symbol_from_source(0, source));
    assert_eq!(name, "Foo");
    assert_eq!(kind, CursorSymbolKind::Subroutine);
    Ok(())
}

#[test]
fn extract_qualified_after_colons() -> Result<(), String> {
    let source = "Foo::bar";
    let pos = source
        .find("bar")
        .ok_or_else(|| "missing 'bar'".to_string())?;
    let (name, kind) = must_some(extract_symbol_from_source(pos, source));
    assert_eq!(name, "bar");
    assert_eq!(kind, CursorSymbolKind::Subroutine);
    Ok(())
}

// ─── get_symbol_range_at_position — basic range extraction ──────────────────

#[test]
fn range_of_scalar_includes_sigil() -> Result<(), String> {
    let source = "print $total;";
    let pos = source
        .find("total")
        .ok_or_else(|| "missing 'total'".to_string())?;
    let (start, end) = must_some(get_symbol_range_at_position(pos, source));
    // $ is at pos-1, name runs "total" = 5 chars
    assert_eq!(start, pos - 1); // includes the $
    assert_eq!(end, pos + 5); // "total".len() == 5
    Ok(())
}

#[test]
fn range_of_array_includes_sigil() -> Result<(), String> {
    let source = "push @list, 1;";
    let pos = source
        .find("list")
        .ok_or_else(|| "missing 'list'".to_string())?;
    let (start, end) = must_some(get_symbol_range_at_position(pos, source));
    assert_eq!(start, pos - 1); // includes @
    assert_eq!(end, pos + 4);
    Ok(())
}

#[test]
fn range_of_hash_includes_sigil() -> Result<(), String> {
    let source = "keys %opts;";
    let pos = source
        .find("opts")
        .ok_or_else(|| "missing 'opts'".to_string())?;
    let (start, end) = must_some(get_symbol_range_at_position(pos, source));
    assert_eq!(start, pos - 1); // includes %
    assert_eq!(end, pos + 4);
    Ok(())
}

#[test]
fn range_of_ampersand_includes_sigil() -> Result<(), String> {
    let source = "call &func;";
    let pos = source
        .find("func")
        .ok_or_else(|| "missing 'func'".to_string())?;
    let (start, end) = must_some(get_symbol_range_at_position(pos, source));
    assert_eq!(start, pos - 1); // includes &
    assert_eq!(end, pos + 4);
    Ok(())
}

// ─── get_symbol_range_at_position — no sigil ────────────────────────────────

#[test]
fn range_bare_word_no_sigil() -> Result<(), String> {
    let source = "hello world";
    let (_start, end) = must_some(get_symbol_range_at_position(0, source));
    // No sigil before pos 0, forward scan covers "hello"
    assert_eq!(end, 5);
    Ok(())
}

// ─── get_symbol_range_at_position — edge cases ──────────────────────────────

#[test]
fn range_empty_source_returns_none() {
    let result = get_symbol_range_at_position(0, "");
    assert_eq!(result, None);
}

#[test]
fn range_position_beyond_length_returns_none() {
    let result = get_symbol_range_at_position(50, "short");
    assert_eq!(result, None);
}

#[test]
fn range_position_at_exact_length_returns_none() {
    let source = "abc";
    let result = get_symbol_range_at_position(source.len(), source);
    assert_eq!(result, None);
}

#[test]
fn range_on_whitespace_returns_some_empty_range() {
    let source = "a b";
    // position 1 is space, no sigil at position 0 (it's 'a' not a sigil)
    let result = get_symbol_range_at_position(1, source);
    // start stays at 1, end stays at 1 (space is not alnum/_)
    assert!(result.is_some());
    if let Some((start, end)) = result {
        assert_eq!(start, end);
    }
}

#[test]
fn range_cursor_at_start_of_source() -> Result<(), String> {
    let source = "$abc";
    // position 0 is '$', no sigil before (position > 0 false)
    // forward scan: '$' is not alnum/_, so end stays at 0
    // backward loop: start == position so no backward scan
    let (start, end) = must_some(get_symbol_range_at_position(0, source));
    assert_eq!(start, 0);
    assert_eq!(end, 0);
    Ok(())
}

#[test]
fn range_cursor_at_last_char() -> Result<(), String> {
    let source = "abc";
    let (_start, end) = must_some(get_symbol_range_at_position(2, source));
    // forward scan from pos 2: 'c' is alnum, end=3 (past length)
    assert_eq!(end, 3);
    Ok(())
}

// ─── get_symbol_range_at_position — sigil at position 0 with name ───────────

#[test]
fn range_sigil_at_start_cursor_on_name() -> Result<(), String> {
    let source = "$x";
    // cursor on 'x' at position 1: chars[0] is '$', so start = 0
    let (start, end) = must_some(get_symbol_range_at_position(1, source));
    assert_eq!(start, 0);
    assert_eq!(end, 2);
    Ok(())
}

// ─── Integration-style: extract + range consistency ─────────────────────────

#[test]
fn extract_and_range_agree_on_symbol_length() -> Result<(), String> {
    let source = "my $count = 42;";
    let pos = source
        .find("count")
        .ok_or_else(|| "missing 'count'".to_string())?;

    let (name, _kind) = must_some(extract_symbol_from_source(pos, source));
    let (start, end) = must_some(get_symbol_range_at_position(pos, source));

    // Range should cover sigil + name: 1 (for $) + name.len()
    assert_eq!(end - start, 1 + name.len());
    Ok(())
}

#[test]
fn extract_and_range_for_bare_word() -> Result<(), String> {
    let source = "foo";
    let (name, _kind) = must_some(extract_symbol_from_source(0, source));
    let (_start, end) = must_some(get_symbol_range_at_position(0, source));

    // Forward scan from 0 gives end = 3 = "foo".len()
    assert_eq!(end, name.len());
    Ok(())
}

// ─── Multiline source ──────────────────────────────────────────────────────

#[test]
fn extract_symbol_from_multiline_source() -> Result<(), String> {
    let source = "my $x = 1;\nmy $y = 2;\n";
    let pos = source
        .find("$y")
        .ok_or_else(|| "missing '$y'".to_string())?
        + 1; // cursor on 'y' after '$'
    let (name, kind) = must_some(extract_symbol_from_source(pos, source));
    assert_eq!(name, "y");
    assert_eq!(kind, CursorSymbolKind::Scalar);
    Ok(())
}

// ─── Numeric-only position after sigil ──────────────────────────────────────

#[test]
fn extract_numeric_name_after_sigil() -> Result<(), String> {
    // Perl special variable like $1
    let source = "$1";
    let (name, kind) = must_some(extract_symbol_from_source(1, source));
    assert_eq!(name, "1");
    assert_eq!(kind, CursorSymbolKind::Scalar);
    Ok(())
}

// ─── Consecutive sigils ─────────────────────────────────────────────────────

#[test]
fn extract_dollar_at_treats_first_as_sigil() -> Result<(), String> {
    // "$$ref" — cursor at position 1 (second $): chars[0] is '$'
    // so sigil = Scalar, name_start = 1, but chars[1] is '$' (not alnum/_)
    // end stays at 1, no name → None
    let source = "$$ref";
    let result = extract_symbol_from_source(1, source);
    assert_eq!(result, None);
    Ok(())
}

#[test]
fn extract_deref_cursor_on_name() -> Result<(), String> {
    // "$$ref" — cursor on 'r' at position 2: chars[1] is '$' → Scalar
    let source = "$$ref";
    let pos = source.find('r').ok_or_else(|| "missing 'r'".to_string())?;
    let (name, kind) = must_some(extract_symbol_from_source(pos, source));
    assert_eq!(name, "ref");
    assert_eq!(kind, CursorSymbolKind::Scalar);
    Ok(())
}
