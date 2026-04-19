//! Comprehensive unit tests for perl-module-token-core public API.

use perl_module::token_core::{
    ModuleTokenSpan, has_standalone_module_token_boundaries, is_module_identifier_char,
    is_module_token_char, parse_module_token,
};

// ---------------------------------------------------------------------------
// parse_module_token — basic parsing
// ---------------------------------------------------------------------------

#[test]
fn parse_single_bare_identifier() -> Result<(), String> {
    let span = parse_module_token("Foo", 0).ok_or("expected Some for single bare identifier")?;
    if span != (ModuleTokenSpan { start: 0, end: 3 }) {
        return Err(format!("unexpected span: {span:?}"));
    }
    Ok(())
}

#[test]
fn parse_canonical_two_segments() -> Result<(), String> {
    let span =
        parse_module_token("Foo::Bar", 0).ok_or("expected Some for canonical two-segment")?;
    if span != (ModuleTokenSpan { start: 0, end: 8 }) {
        return Err(format!("unexpected span: {span:?}"));
    }
    Ok(())
}

#[test]
fn parse_canonical_three_segments() -> Result<(), String> {
    let span = parse_module_token("Foo::Bar::Baz", 0)
        .ok_or("expected Some for canonical three-segment")?;
    if span != (ModuleTokenSpan { start: 0, end: 13 }) {
        return Err(format!("unexpected span: {span:?}"));
    }
    Ok(())
}

#[test]
fn parse_legacy_two_segments() -> Result<(), String> {
    let span = parse_module_token("Foo'Bar", 0).ok_or("expected Some for legacy two-segment")?;
    if span != (ModuleTokenSpan { start: 0, end: 7 }) {
        return Err(format!("unexpected span: {span:?}"));
    }
    Ok(())
}

#[test]
fn parse_mixed_separators() -> Result<(), String> {
    let span = parse_module_token("Foo::Bar'Baz", 0).ok_or("expected Some for mixed separators")?;
    if span != (ModuleTokenSpan { start: 0, end: 12 }) {
        return Err(format!("unexpected span: {span:?}"));
    }
    Ok(())
}

#[test]
fn parse_mixed_separators_reversed() -> Result<(), String> {
    let span = parse_module_token("Foo'Bar::Baz", 0)
        .ok_or("expected Some for reversed mixed separators")?;
    if span != (ModuleTokenSpan { start: 0, end: 12 }) {
        return Err(format!("unexpected span: {span:?}"));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// parse_module_token — offsets
// ---------------------------------------------------------------------------

#[test]
fn parse_at_nonzero_offset() -> Result<(), String> {
    let span = parse_module_token("use Foo::Bar;", 4).ok_or("expected Some at offset 4")?;
    if span != (ModuleTokenSpan { start: 4, end: 12 }) {
        return Err(format!("unexpected span: {span:?}"));
    }
    Ok(())
}

#[test]
fn parse_token_stops_at_semicolon() -> Result<(), String> {
    let span =
        parse_module_token("Foo::Bar;more", 0).ok_or("expected Some stopping at semicolon")?;
    if span != (ModuleTokenSpan { start: 0, end: 8 }) {
        return Err(format!("unexpected span: {span:?}"));
    }
    Ok(())
}

#[test]
fn parse_token_stops_at_whitespace() -> Result<(), String> {
    let span =
        parse_module_token("Foo::Bar rest", 0).ok_or("expected Some stopping at whitespace")?;
    if span != (ModuleTokenSpan { start: 0, end: 8 }) {
        return Err(format!("unexpected span: {span:?}"));
    }
    Ok(())
}

#[test]
fn parse_token_stops_at_paren() -> Result<(), String> {
    let span = parse_module_token("Foo::Bar()", 0).ok_or("expected Some stopping at paren")?;
    if span != (ModuleTokenSpan { start: 0, end: 8 }) {
        return Err(format!("unexpected span: {span:?}"));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// parse_module_token — underscore and digit in identifiers
// ---------------------------------------------------------------------------

#[test]
fn parse_underscore_identifier() -> Result<(), String> {
    let span = parse_module_token("_Foo::_Bar", 0)
        .ok_or("expected Some for underscore-prefixed identifiers")?;
    if span != (ModuleTokenSpan { start: 0, end: 10 }) {
        return Err(format!("unexpected span: {span:?}"));
    }
    Ok(())
}

#[test]
fn parse_underscore_only_segment() -> Result<(), String> {
    let span = parse_module_token("_::_", 0).ok_or("expected Some for underscore-only segments")?;
    if span != (ModuleTokenSpan { start: 0, end: 4 }) {
        return Err(format!("unexpected span: {span:?}"));
    }
    Ok(())
}

#[test]
fn parse_identifier_with_digits() -> Result<(), String> {
    let span =
        parse_module_token("Foo2::Bar3", 0).ok_or("expected Some for identifiers with digits")?;
    if span != (ModuleTokenSpan { start: 0, end: 10 }) {
        return Err(format!("unexpected span: {span:?}"));
    }
    Ok(())
}

#[test]
fn parse_single_char_segments() -> Result<(), String> {
    let span = parse_module_token("A::B::C", 0).ok_or("expected Some for single-char segments")?;
    if span != (ModuleTokenSpan { start: 0, end: 7 }) {
        return Err(format!("unexpected span: {span:?}"));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// parse_module_token — None cases
// ---------------------------------------------------------------------------

#[test]
fn parse_empty_string_returns_none() {
    assert!(parse_module_token("", 0).is_none());
}

#[test]
fn parse_start_beyond_length_returns_none() {
    assert!(parse_module_token("Foo", 10).is_none());
}

#[test]
fn parse_start_at_exact_length_returns_none() {
    assert!(parse_module_token("Foo", 3).is_none());
}

#[test]
fn parse_start_at_digit_returns_none() {
    assert!(parse_module_token("123", 0).is_none());
}

#[test]
fn parse_start_at_space_returns_none() {
    assert!(parse_module_token(" Foo", 0).is_none());
}

#[test]
fn parse_start_at_colon_returns_none() {
    assert!(parse_module_token("::Foo", 0).is_none());
}

#[test]
fn parse_start_at_semicolon_returns_none() {
    assert!(parse_module_token(";Foo", 0).is_none());
}

#[test]
fn parse_start_at_apostrophe_returns_none() {
    assert!(parse_module_token("'Foo", 0).is_none());
}

// ---------------------------------------------------------------------------
// parse_module_token — trailing separator edge cases
// ---------------------------------------------------------------------------

#[test]
fn parse_trailing_canonical_separator_returns_none() {
    // "Foo::" — separator found but no valid segment follows, so returns None
    assert!(parse_module_token("Foo::", 0).is_none());
}

#[test]
fn parse_trailing_legacy_separator_returns_none() {
    // "Foo'" — separator found but no valid segment follows, so returns None
    assert!(parse_module_token("Foo'", 0).is_none());
}

#[test]
fn parse_separator_followed_by_digit_returns_none() {
    // "Foo::2bar" — digit is not a valid identifier start after separator, returns None
    assert!(parse_module_token("Foo::2bar", 0).is_none());
}

#[test]
fn parse_legacy_separator_followed_by_digit_returns_none() {
    // "Foo'2bar" — digit after legacy separator is not valid, returns None
    assert!(parse_module_token("Foo'2bar", 0).is_none());
}

// ---------------------------------------------------------------------------
// parse_module_token — deeply nested
// ---------------------------------------------------------------------------

#[test]
fn parse_four_segment_canonical() -> Result<(), String> {
    let span =
        parse_module_token("A::B::C::D", 0).ok_or("expected Some for four-segment canonical")?;
    if span != (ModuleTokenSpan { start: 0, end: 10 }) {
        return Err(format!("unexpected span: {span:?}"));
    }
    Ok(())
}

#[test]
fn parse_long_real_world_module() -> Result<(), String> {
    let input = "Moose::Meta::Role::Application::ToClass";
    let span = parse_module_token(input, 0).ok_or("expected Some for long module")?;
    if span
        != (ModuleTokenSpan {
            start: 0,
            end: input.len(),
        })
    {
        return Err(format!("unexpected span: {span:?}"));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// parse_module_token — extracted substring matches
// ---------------------------------------------------------------------------

#[test]
fn extracted_text_matches_expected() -> Result<(), String> {
    let input = "use Foo::Bar::Baz;";
    let span = parse_module_token(input, 4).ok_or("expected Some")?;
    let extracted = &input[span.start..span.end];
    if extracted != "Foo::Bar::Baz" {
        return Err(format!("unexpected extracted text: {extracted}"));
    }
    Ok(())
}

#[test]
fn extracted_legacy_text_matches() -> Result<(), String> {
    let input = "use Foo'Bar;";
    let span = parse_module_token(input, 4).ok_or("expected Some")?;
    let extracted = &input[span.start..span.end];
    if extracted != "Foo'Bar" {
        return Err(format!("unexpected extracted text: {extracted}"));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// has_standalone_module_token_boundaries
// ---------------------------------------------------------------------------

#[test]
fn standalone_at_line_start_and_end() {
    assert!(has_standalone_module_token_boundaries("Foo::Bar", 0, 8));
}

#[test]
fn standalone_surrounded_by_non_module_chars() {
    assert!(has_standalone_module_token_boundaries("(Foo::Bar)", 1, 9));
}

#[test]
fn standalone_after_space() {
    assert!(has_standalone_module_token_boundaries(
        "use Foo::Bar;",
        4,
        12
    ));
}

#[test]
fn not_standalone_when_preceded_by_alpha() {
    // "xFoo" at position 1..4 — 'x' is a module token char
    assert!(!has_standalone_module_token_boundaries("xFoo", 1, 4));
}

#[test]
fn not_standalone_when_followed_by_alpha() {
    assert!(!has_standalone_module_token_boundaries("Foox", 0, 3));
}

#[test]
fn not_standalone_when_preceded_by_colon() {
    assert!(!has_standalone_module_token_boundaries(":Foo", 1, 4));
}

#[test]
fn not_standalone_when_followed_by_colon() {
    assert!(!has_standalone_module_token_boundaries("Foo:", 0, 3));
}

#[test]
fn not_standalone_when_preceded_by_underscore() {
    assert!(!has_standalone_module_token_boundaries("_Foo", 1, 4));
}

#[test]
fn not_standalone_when_followed_by_underscore() {
    assert!(!has_standalone_module_token_boundaries("Foo_", 0, 3));
}

#[test]
fn not_standalone_when_followed_by_digit() {
    assert!(!has_standalone_module_token_boundaries("Foo1", 0, 3));
}

#[test]
fn not_standalone_when_preceded_by_digit() {
    assert!(!has_standalone_module_token_boundaries("1Foo", 1, 4));
}

#[test]
fn standalone_when_preceded_by_double_colon_beyond_span() {
    // "::Foo" — the left context at position 2 has ':' which is a module_token_char
    assert!(!has_standalone_module_token_boundaries("::Foo", 2, 5));
}

// ---------------------------------------------------------------------------
// has_standalone_module_token_boundaries — apostrophe edge cases
// ---------------------------------------------------------------------------

#[test]
fn not_standalone_when_left_context_has_apostrophe_with_identifier() {
    // "X'Foo" — apostrophe at position 1, preceded by 'X' (identifier char)
    // so left context of Foo (position 2) sees apostrophe + identifier = module char
    assert!(!has_standalone_module_token_boundaries("X'Foo", 2, 5));
}

#[test]
fn standalone_when_left_context_has_lone_apostrophe() {
    // "'Foo" — apostrophe at position 0 with nothing before it
    assert!(has_standalone_module_token_boundaries("'Foo", 1, 4));
}

#[test]
fn not_standalone_when_right_context_has_apostrophe_with_identifier() {
    // "Foo'X" — Foo at 0..3, right context sees ' then X
    assert!(!has_standalone_module_token_boundaries("Foo'X", 0, 3));
}

#[test]
fn standalone_when_right_context_has_apostrophe_without_identifier() {
    // "Foo'" — Foo at 0..3, right context sees ' then end-of-string
    assert!(has_standalone_module_token_boundaries("Foo'", 0, 3));
}

#[test]
fn standalone_when_right_apostrophe_followed_by_non_identifier() {
    // "Foo' " — Foo at 0..3, right context sees ' then space (not identifier)
    assert!(has_standalone_module_token_boundaries("Foo' ", 0, 3));
}

// ---------------------------------------------------------------------------
// has_standalone_module_token_boundaries — edge positions
// ---------------------------------------------------------------------------

#[test]
fn standalone_empty_string_start_eq_end() {
    // Degenerate case: start == 0, end == 0 on an empty-ish line
    assert!(has_standalone_module_token_boundaries("", 0, 0));
}

#[test]
fn standalone_end_at_line_length() {
    assert!(has_standalone_module_token_boundaries("Foo", 0, 3));
}

#[test]
fn standalone_end_beyond_line_length() {
    // end > line.len() is treated as no right context
    assert!(has_standalone_module_token_boundaries("Foo", 0, 10));
}

#[test]
fn standalone_after_semicolon() {
    assert!(has_standalone_module_token_boundaries(";Foo;", 1, 4));
}

#[test]
fn standalone_after_open_paren() {
    assert!(has_standalone_module_token_boundaries("(Foo)", 1, 4));
}

#[test]
fn standalone_after_newline_char() {
    assert!(has_standalone_module_token_boundaries("\nFoo\n", 1, 4));
}

// ---------------------------------------------------------------------------
// is_module_token_char — comprehensive character class
// ---------------------------------------------------------------------------

#[test]
fn module_token_char_accepts_ascii_letters() {
    for ch in 'a'..='z' {
        assert!(is_module_token_char(ch), "expected true for '{ch}'");
    }
    for ch in 'A'..='Z' {
        assert!(is_module_token_char(ch), "expected true for '{ch}'");
    }
}

#[test]
fn module_token_char_accepts_digits() {
    for ch in '0'..='9' {
        assert!(is_module_token_char(ch), "expected true for '{ch}'");
    }
}

#[test]
fn module_token_char_accepts_underscore() {
    assert!(is_module_token_char('_'));
}

#[test]
fn module_token_char_accepts_colon() {
    assert!(is_module_token_char(':'));
}

#[test]
fn module_token_char_rejects_common_non_module_chars() {
    let rejects = [
        ' ', '\t', '\n', ';', '(', ')', '[', ']', '{', '}', ',', '.', '!', '@', '#', '$', '%', '^',
        '&', '*', '-', '+', '=', '<', '>', '/', '\\', '|', '~', '`', '"', '\'', '?',
    ];
    for ch in rejects {
        assert!(!is_module_token_char(ch), "expected false for '{ch}'");
    }
}

// ---------------------------------------------------------------------------
// is_module_identifier_char — comprehensive character class
// ---------------------------------------------------------------------------

#[test]
fn module_identifier_char_accepts_ascii_letters() {
    for ch in 'a'..='z' {
        assert!(is_module_identifier_char(ch), "expected true for '{ch}'");
    }
    for ch in 'A'..='Z' {
        assert!(is_module_identifier_char(ch), "expected true for '{ch}'");
    }
}

#[test]
fn module_identifier_char_accepts_digits() {
    for ch in '0'..='9' {
        assert!(is_module_identifier_char(ch), "expected true for '{ch}'");
    }
}

#[test]
fn module_identifier_char_accepts_underscore() {
    assert!(is_module_identifier_char('_'));
}

#[test]
fn module_identifier_char_rejects_colon() {
    assert!(!is_module_identifier_char(':'));
}

#[test]
fn module_identifier_char_rejects_apostrophe() {
    assert!(!is_module_identifier_char('\''));
}

#[test]
fn module_identifier_char_rejects_special_chars() {
    let rejects = [' ', '\t', ';', '(', ')', '-', '.', '@', '$', '%'];
    for ch in rejects {
        assert!(!is_module_identifier_char(ch), "expected false for '{ch}'");
    }
}

// ---------------------------------------------------------------------------
// ModuleTokenSpan — struct behavior
// ---------------------------------------------------------------------------

#[test]
fn module_token_span_debug_impl() {
    let span = ModuleTokenSpan { start: 1, end: 5 };
    let debug = format!("{span:?}");
    assert!(!debug.is_empty());
}

#[test]
fn module_token_span_clone() {
    let span = ModuleTokenSpan { start: 0, end: 3 };
    let cloned = span;
    assert_eq!(span, cloned);
}

#[test]
fn module_token_span_copy() {
    let span = ModuleTokenSpan { start: 2, end: 7 };
    let copied = span;
    // Both should be usable since Copy
    assert_eq!(span.start, copied.start);
    assert_eq!(span.end, copied.end);
}

#[test]
fn module_token_span_eq() {
    let a = ModuleTokenSpan { start: 0, end: 5 };
    let b = ModuleTokenSpan { start: 0, end: 5 };
    let c = ModuleTokenSpan { start: 0, end: 6 };
    assert_eq!(a, b);
    assert_ne!(a, c);
}

// ---------------------------------------------------------------------------
// parse_module_token — real-world Perl patterns
// ---------------------------------------------------------------------------

#[test]
fn parse_use_statement() -> Result<(), String> {
    let input = "use strict;";
    let span = parse_module_token(input, 4).ok_or("expected Some")?;
    let extracted = &input[span.start..span.end];
    if extracted != "strict" {
        return Err(format!("unexpected: {extracted}"));
    }
    Ok(())
}

#[test]
fn parse_require_statement() -> Result<(), String> {
    let input = "require Carp::Heavy;";
    let span = parse_module_token(input, 8).ok_or("expected Some")?;
    let extracted = &input[span.start..span.end];
    if extracted != "Carp::Heavy" {
        return Err(format!("unexpected: {extracted}"));
    }
    Ok(())
}

#[test]
fn parse_method_call_context() -> Result<(), String> {
    let input = "Foo::Bar->new()";
    let span = parse_module_token(input, 0).ok_or("expected Some")?;
    let extracted = &input[span.start..span.end];
    if extracted != "Foo::Bar" {
        return Err(format!("unexpected: {extracted}"));
    }
    Ok(())
}

#[test]
fn parse_isa_check_context_with_quotes() {
    // "$obj->isa('Some::Class')" — the closing ' is seen as a legacy separator
    // followed by ')' which is not a valid identifier start, so returns None
    let input = "$obj->isa('Some::Class')";
    assert!(parse_module_token(input, 11).is_none());
}

#[test]
fn parse_isa_check_without_trailing_quote() -> Result<(), String> {
    // Without surrounding quotes, module parses fine
    let input = "$obj->isa(Some::Class)";
    let span = parse_module_token(input, 10).ok_or("expected Some")?;
    let extracted = &input[span.start..span.end];
    if extracted != "Some::Class" {
        return Err(format!("unexpected: {extracted}"));
    }
    Ok(())
}

#[test]
fn parse_function_call_context() -> Result<(), String> {
    let input = "Foo::Bar::baz()";
    let span = parse_module_token(input, 0).ok_or("expected Some")?;
    // Should parse entire qualified name including function
    let extracted = &input[span.start..span.end];
    if extracted != "Foo::Bar::baz" {
        return Err(format!("unexpected: {extracted}"));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// parse_module_token — boundary combinations with standalone check
// ---------------------------------------------------------------------------

#[test]
fn parsed_token_is_standalone_in_use_statement() -> Result<(), String> {
    let input = "use Moose::Role;";
    let span = parse_module_token(input, 4).ok_or("expected Some")?;
    if !has_standalone_module_token_boundaries(input, span.start, span.end) {
        return Err("expected standalone boundaries".to_string());
    }
    Ok(())
}

#[test]
fn parsed_token_not_standalone_when_part_of_longer() {
    let input = "use Foo::Bar::Baz;";
    // Parse from offset 4 gets the full "Foo::Bar::Baz"
    // Check if just "Foo::Bar" (4..12) is standalone — it shouldn't be
    assert!(!has_standalone_module_token_boundaries(input, 4, 12));
}

#[test]
fn parse_at_each_segment_start() -> Result<(), String> {
    let input = "Foo::Bar::Baz";
    // Parse starting at "Bar::Baz" (offset 5)
    let span = parse_module_token(input, 5).ok_or("expected Some at Bar")?;
    let extracted = &input[span.start..span.end];
    if extracted != "Bar::Baz" {
        return Err(format!("unexpected: {extracted}"));
    }
    // Parse starting at "Baz" (offset 10)
    let span2 = parse_module_token(input, 10).ok_or("expected Some at Baz")?;
    let extracted2 = &input[span2.start..span2.end];
    if extracted2 != "Baz" {
        return Err(format!("unexpected: {extracted2}"));
    }
    Ok(())
}
