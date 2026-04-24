//! Extended unit tests for the `perl-token` crate.
//!
//! Supplements `comprehensive_unit_tests.rs` with additional coverage for:
//! - Token span arithmetic and consistency
//! - Field mutation (pub fields)
//! - Arc sharing across multiple clones and independent allocations
//! - TokenKind memory layout and size guarantees
//! - TokenKind Debug format specifics per variant
//! - Category-based grouping helpers
//! - Complex Perl idiom token sequences
//! - Edge cases: whitespace variants, escapes, high-bit ASCII, long text
//! - Iterator / collection patterns
//! - Reflexivity, symmetry, transitivity of PartialEq

use perl_token::{Token, TokenKind};
use std::sync::Arc;

// ===========================================================================
// Token span arithmetic
// ===========================================================================

#[test]
fn token_span_length_matches_text_len() {
    let cases: &[(&str, TokenKind)] = &[
        ("my", TokenKind::My),
        ("$", TokenKind::ScalarSigil),
        ("some_long_identifier_name", TokenKind::Identifier),
        ("s/foo/bar/g", TokenKind::Substitution),
        ("", TokenKind::Eof),
    ];
    for (text, kind) in cases {
        let tok = Token::new(*kind, *text, 10, 10 + text.len());
        assert_eq!(tok.end - tok.start, tok.text.len(), "span mismatch for {text:?}");
    }
}

#[test]
fn token_span_adjacent_tokens_no_gap() {
    let a = Token::new(TokenKind::My, "my", 0, 2);
    let b = Token::new(TokenKind::ScalarSigil, "$", 2, 3);
    assert_eq!(a.end, b.start);
}

#[test]
fn token_span_with_gap_for_whitespace() {
    let a = Token::new(TokenKind::My, "my", 0, 2);
    let b = Token::new(TokenKind::ScalarSigil, "$", 3, 4);
    assert_eq!(b.start - a.end, 1);
}

// ===========================================================================
// Pub field mutation
// ===========================================================================

#[test]
fn token_kind_field_is_mutable() {
    let mut tok = Token::new(TokenKind::Identifier, "eval", 0, 4);
    tok.kind = TokenKind::Eval;
    assert_eq!(tok.kind, TokenKind::Eval);
}

#[test]
fn token_start_end_fields_are_mutable() {
    let mut tok = Token::new(TokenKind::Number, "42", 0, 2);
    tok.start = 100;
    tok.end = 102;
    assert_eq!(tok.start, 100);
    assert_eq!(tok.end, 102);
}

#[test]
fn token_text_field_is_replaceable() {
    let mut tok = Token::new(TokenKind::String, "old", 0, 3);
    tok.text = Arc::from("new");
    assert_eq!(&*tok.text, "new");
}

// ===========================================================================
// Arc sharing — multiple clones
// ===========================================================================

#[test]
fn arc_strong_count_with_three_clones() {
    let original = Token::new(TokenKind::Identifier, "shared_text", 0, 11);
    let c1 = original.clone();
    let c2 = original.clone();
    let c3 = original.clone();
    assert_eq!(Arc::strong_count(&original.text), 4);
    assert!(Arc::ptr_eq(&c1.text, &c2.text));
    assert!(Arc::ptr_eq(&c2.text, &c3.text));
}

#[test]
fn arc_count_decreases_on_drop() {
    let original = Token::new(TokenKind::Sub, "sub", 0, 3);
    let arc_ref = original.text.clone();
    assert_eq!(Arc::strong_count(&arc_ref), 2);
    drop(original);
    assert_eq!(Arc::strong_count(&arc_ref), 1);
}

#[test]
fn independent_arc_allocations_are_equal_but_not_ptr_eq() {
    let a = Token::new(TokenKind::Identifier, "foo", 0, 3);
    let b = Token::new(TokenKind::Identifier, "foo", 0, 3);
    assert_eq!(a, b);
    // Different allocations: ptr_eq should be false
    assert!(!Arc::ptr_eq(&a.text, &b.text));
}

// ===========================================================================
// TokenKind — memory layout
// ===========================================================================

#[test]
fn token_kind_size_is_one_byte() {
    // Enum with < 256 variants should fit in a single byte
    assert_eq!(std::mem::size_of::<TokenKind>(), 1);
}

#[test]
fn token_kind_alignment_is_one() {
    assert_eq!(std::mem::align_of::<TokenKind>(), 1);
}

#[test]
fn option_token_kind_is_one_byte() {
    // With niche optimization, Option<TokenKind> should still be 1 byte
    assert_eq!(std::mem::size_of::<Option<TokenKind>>(), 1);
}

// ===========================================================================
// TokenKind — Debug format specifics
// ===========================================================================

#[test]
fn token_kind_debug_keywords() {
    let cases: &[(TokenKind, &str)] = &[
        (TokenKind::My, "My"),
        (TokenKind::Our, "Our"),
        (TokenKind::Local, "Local"),
        (TokenKind::State, "State"),
        (TokenKind::Sub, "Sub"),
        (TokenKind::If, "If"),
        (TokenKind::Elsif, "Elsif"),
        (TokenKind::Else, "Else"),
        (TokenKind::Unless, "Unless"),
        (TokenKind::While, "While"),
        (TokenKind::Until, "Until"),
        (TokenKind::For, "For"),
        (TokenKind::Foreach, "Foreach"),
        (TokenKind::Return, "Return"),
        (TokenKind::Package, "Package"),
        (TokenKind::Use, "Use"),
        (TokenKind::No, "No"),
        (TokenKind::Begin, "Begin"),
        (TokenKind::End, "End"),
        (TokenKind::Check, "Check"),
        (TokenKind::Init, "Init"),
        (TokenKind::Unitcheck, "Unitcheck"),
        (TokenKind::Eval, "Eval"),
        (TokenKind::Do, "Do"),
        (TokenKind::Given, "Given"),
        (TokenKind::When, "When"),
        (TokenKind::Default, "Default"),
        (TokenKind::Try, "Try"),
        (TokenKind::Catch, "Catch"),
        (TokenKind::Finally, "Finally"),
        (TokenKind::Continue, "Continue"),
        (TokenKind::Next, "Next"),
        (TokenKind::Last, "Last"),
        (TokenKind::Redo, "Redo"),
        (TokenKind::Goto, "Goto"),
        (TokenKind::Class, "Class"),
        (TokenKind::Method, "Method"),
        (TokenKind::Field, "Field"),
        (TokenKind::Format, "Format"),
        (TokenKind::Undef, "Undef"),
        (TokenKind::Defer, "Defer"),
    ];
    for (kind, label) in cases {
        assert_eq!(format!("{kind:?}"), *label);
    }
}

#[test]
fn token_kind_debug_operators() {
    let cases: &[(TokenKind, &str)] = &[
        (TokenKind::Assign, "Assign"),
        (TokenKind::Plus, "Plus"),
        (TokenKind::Minus, "Minus"),
        (TokenKind::Star, "Star"),
        (TokenKind::Slash, "Slash"),
        (TokenKind::Percent, "Percent"),
        (TokenKind::Power, "Power"),
        (TokenKind::Arrow, "Arrow"),
        (TokenKind::FatArrow, "FatArrow"),
        (TokenKind::Dot, "Dot"),
        (TokenKind::Range, "Range"),
        (TokenKind::Ellipsis, "Ellipsis"),
        (TokenKind::Spaceship, "Spaceship"),
        (TokenKind::SmartMatch, "SmartMatch"),
        (TokenKind::DefinedOr, "DefinedOr"),
        (TokenKind::Increment, "Increment"),
        (TokenKind::Decrement, "Decrement"),
    ];
    for (kind, label) in cases {
        assert_eq!(format!("{kind:?}"), *label);
    }
}

#[test]
fn token_kind_debug_delimiters() {
    let cases: &[(TokenKind, &str)] = &[
        (TokenKind::LeftParen, "LeftParen"),
        (TokenKind::RightParen, "RightParen"),
        (TokenKind::LeftBrace, "LeftBrace"),
        (TokenKind::RightBrace, "RightBrace"),
        (TokenKind::LeftBracket, "LeftBracket"),
        (TokenKind::RightBracket, "RightBracket"),
        (TokenKind::Semicolon, "Semicolon"),
        (TokenKind::Comma, "Comma"),
    ];
    for (kind, label) in cases {
        assert_eq!(format!("{kind:?}"), *label);
    }
}

#[test]
fn token_kind_debug_literals() {
    let cases: &[(TokenKind, &str)] = &[
        (TokenKind::Number, "Number"),
        (TokenKind::String, "String"),
        (TokenKind::Regex, "Regex"),
        (TokenKind::Substitution, "Substitution"),
        (TokenKind::Transliteration, "Transliteration"),
        (TokenKind::QuoteSingle, "QuoteSingle"),
        (TokenKind::QuoteDouble, "QuoteDouble"),
        (TokenKind::QuoteWords, "QuoteWords"),
        (TokenKind::QuoteCommand, "QuoteCommand"),
        (TokenKind::HeredocStart, "HeredocStart"),
        (TokenKind::HeredocBody, "HeredocBody"),
        (TokenKind::FormatBody, "FormatBody"),
        (TokenKind::DataMarker, "DataMarker"),
        (TokenKind::DataBody, "DataBody"),
        (TokenKind::VString, "VString"),
        (TokenKind::UnknownRest, "UnknownRest"),
        (TokenKind::HeredocDepthLimit, "HeredocDepthLimit"),
    ];
    for (kind, label) in cases {
        assert_eq!(format!("{kind:?}"), *label);
    }
}

#[test]
fn token_kind_debug_identifiers_and_special() {
    let cases: &[(TokenKind, &str)] = &[
        (TokenKind::Identifier, "Identifier"),
        (TokenKind::ScalarSigil, "ScalarSigil"),
        (TokenKind::ArraySigil, "ArraySigil"),
        (TokenKind::HashSigil, "HashSigil"),
        (TokenKind::SubSigil, "SubSigil"),
        (TokenKind::GlobSigil, "GlobSigil"),
        (TokenKind::Eof, "Eof"),
        (TokenKind::Unknown, "Unknown"),
    ];
    for (kind, label) in cases {
        assert_eq!(format!("{kind:?}"), *label);
    }
}

// ===========================================================================
// Token Debug format
// ===========================================================================

#[test]
fn token_debug_contains_all_fields() {
    let t = Token::new(TokenKind::Number, "42", 10, 12);
    let dbg = format!("{t:?}");
    assert!(dbg.contains("Number"), "missing kind in: {dbg}");
    assert!(dbg.contains("42"), "missing text in: {dbg}");
    assert!(dbg.contains("10"), "missing start in: {dbg}");
    assert!(dbg.contains("12"), "missing end in: {dbg}");
}

#[test]
fn token_debug_alternate_format() {
    let t = Token::new(TokenKind::My, "my", 0, 2);
    let dbg = format!("{t:#?}");
    // Alternate format should be multi-line
    assert!(dbg.contains('\n'), "alternate debug should be multi-line: {dbg}");
    assert!(dbg.contains("My"), "missing kind in: {dbg}");
}

// ===========================================================================
// PartialEq — transitivity and edge cases
// ===========================================================================

#[test]
fn token_eq_transitivity() {
    let a = Token::new(TokenKind::Plus, "+", 5, 6);
    let b = Token::new(TokenKind::Plus, "+", 5, 6);
    let c = Token::new(TokenKind::Plus, "+", 5, 6);
    assert_eq!(a, b);
    assert_eq!(b, c);
    assert_eq!(a, c);
}

#[test]
fn token_eq_reflexive_for_all_kinds() {
    for kind in TokenKind::all() {
        let tok = Token::new(*kind, "x", 0, 1);
        assert_eq!(tok, tok);
    }
}

#[test]
fn token_ne_only_text_differs() {
    let a = Token::new(TokenKind::Identifier, "foo", 0, 3);
    let b = Token::new(TokenKind::Identifier, "bar", 0, 3);
    assert_ne!(a, b);
}

// ===========================================================================
// TokenKind — category grouping tests
// ===========================================================================

fn keyword_kinds() -> Vec<TokenKind> {
    vec![
        TokenKind::My,
        TokenKind::Our,
        TokenKind::Local,
        TokenKind::State,
        TokenKind::Sub,
        TokenKind::If,
        TokenKind::Elsif,
        TokenKind::Else,
        TokenKind::Unless,
        TokenKind::While,
        TokenKind::Until,
        TokenKind::For,
        TokenKind::Foreach,
        TokenKind::Return,
        TokenKind::Package,
        TokenKind::Use,
        TokenKind::No,
        TokenKind::Begin,
        TokenKind::End,
        TokenKind::Check,
        TokenKind::Init,
        TokenKind::Unitcheck,
        TokenKind::Eval,
        TokenKind::Do,
        TokenKind::Given,
        TokenKind::When,
        TokenKind::Default,
        TokenKind::Try,
        TokenKind::Catch,
        TokenKind::Finally,
        TokenKind::Continue,
        TokenKind::Next,
        TokenKind::Last,
        TokenKind::Redo,
        TokenKind::Goto,
        TokenKind::Class,
        TokenKind::Method,
        TokenKind::Field,
        TokenKind::Format,
        TokenKind::Undef,
        TokenKind::Defer,
    ]
}

fn delimiter_kinds() -> Vec<TokenKind> {
    vec![
        TokenKind::LeftParen,
        TokenKind::RightParen,
        TokenKind::LeftBrace,
        TokenKind::RightBrace,
        TokenKind::LeftBracket,
        TokenKind::RightBracket,
        TokenKind::Semicolon,
        TokenKind::Comma,
    ]
}

fn sigil_kinds() -> Vec<TokenKind> {
    vec![
        TokenKind::ScalarSigil,
        TokenKind::ArraySigil,
        TokenKind::HashSigil,
        TokenKind::SubSigil,
        TokenKind::GlobSigil,
    ]
}

#[test]
fn keyword_count_is_41() {
    assert_eq!(keyword_kinds().len(), 41);
}

#[test]
fn delimiter_count_is_8() {
    assert_eq!(delimiter_kinds().len(), 8);
}

#[test]
fn sigil_count_is_5() {
    assert_eq!(sigil_kinds().len(), 5);
}

#[test]
fn keywords_are_disjoint_from_delimiters() {
    for kw in keyword_kinds() {
        for delim in delimiter_kinds() {
            assert_ne!(kw, delim);
        }
    }
}

#[test]
fn sigils_are_disjoint_from_keywords() {
    for sigil in sigil_kinds() {
        for kw in keyword_kinds() {
            assert_ne!(sigil, kw);
        }
    }
}

// ===========================================================================
// TokenKind Copy chain
// ===========================================================================

#[test]
fn token_kind_copy_chain() {
    let a = TokenKind::Arrow;
    let b = a;
    let c = b;
    let d = c;
    assert_eq!(a, d);
}

#[test]
fn token_kind_copy_into_closure() {
    let kind = TokenKind::Sub;
    let f = move || kind;
    assert_eq!(f(), TokenKind::Sub);
    // Original is still usable (Copy)
    assert_eq!(kind, TokenKind::Sub);
}

// ===========================================================================
// Token::new — various Into<Arc<str>> conversions
// ===========================================================================

#[test]
fn token_new_from_static_str() {
    let t = Token::new(TokenKind::Identifier, "static", 0, 6);
    assert_eq!(&*t.text, "static");
}

#[test]
fn token_new_from_owned_string() {
    let s = "dynamic".to_string();
    let t = Token::new(TokenKind::Identifier, s, 0, 7);
    assert_eq!(&*t.text, "dynamic");
}

#[test]
fn token_new_from_boxed_str() {
    let boxed: Box<str> = "boxed".into();
    let arc: Arc<str> = Arc::from(boxed);
    let t = Token::new(TokenKind::String, arc, 0, 5);
    assert_eq!(&*t.text, "boxed");
}

// ===========================================================================
// Complex Perl idiom sequences
// ===========================================================================

#[test]
fn token_sequence_hash_access() {
    // $hash{key}
    let tokens = [
        Token::new(TokenKind::ScalarSigil, "$", 0, 1),
        Token::new(TokenKind::Identifier, "hash", 1, 5),
        Token::new(TokenKind::LeftBrace, "{", 5, 6),
        Token::new(TokenKind::Identifier, "key", 6, 9),
        Token::new(TokenKind::RightBrace, "}", 9, 10),
    ];
    assert_eq!(tokens.len(), 5);
    assert_eq!(tokens[0].kind, TokenKind::ScalarSigil);
    assert_eq!(tokens[2].kind, TokenKind::LeftBrace);
    assert_eq!(tokens[4].kind, TokenKind::RightBrace);
}

#[test]
fn token_sequence_array_slice() {
    // @array[0..2]
    let tokens = [
        Token::new(TokenKind::ArraySigil, "@", 0, 1),
        Token::new(TokenKind::Identifier, "array", 1, 6),
        Token::new(TokenKind::LeftBracket, "[", 6, 7),
        Token::new(TokenKind::Number, "0", 7, 8),
        Token::new(TokenKind::Range, "..", 8, 10),
        Token::new(TokenKind::Number, "2", 10, 11),
        Token::new(TokenKind::RightBracket, "]", 11, 12),
    ];
    assert_eq!(tokens.len(), 7);
    assert_eq!(tokens[4].kind, TokenKind::Range);
}

#[test]
fn token_sequence_if_elsif_else() {
    // if (...) { } elsif (...) { } else { }
    let tokens = vec![
        Token::new(TokenKind::If, "if", 0, 2),
        Token::new(TokenKind::LeftParen, "(", 3, 4),
        Token::new(TokenKind::RightParen, ")", 4, 5),
        Token::new(TokenKind::LeftBrace, "{", 6, 7),
        Token::new(TokenKind::RightBrace, "}", 8, 9),
        Token::new(TokenKind::Elsif, "elsif", 10, 15),
        Token::new(TokenKind::LeftParen, "(", 16, 17),
        Token::new(TokenKind::RightParen, ")", 17, 18),
        Token::new(TokenKind::LeftBrace, "{", 19, 20),
        Token::new(TokenKind::RightBrace, "}", 21, 22),
        Token::new(TokenKind::Else, "else", 23, 27),
        Token::new(TokenKind::LeftBrace, "{", 28, 29),
        Token::new(TokenKind::RightBrace, "}", 30, 31),
    ];
    assert_eq!(tokens.len(), 13);
    assert_eq!(tokens[0].kind, TokenKind::If);
    assert_eq!(tokens[5].kind, TokenKind::Elsif);
    assert_eq!(tokens[10].kind, TokenKind::Else);
}

#[test]
fn token_sequence_use_module() {
    // use strict;
    let tokens = [
        Token::new(TokenKind::Use, "use", 0, 3),
        Token::new(TokenKind::Identifier, "strict", 4, 10),
        Token::new(TokenKind::Semicolon, ";", 10, 11),
    ];
    assert_eq!(tokens[0].kind, TokenKind::Use);
    assert_eq!(&*tokens[1].text, "strict");
}

#[test]
fn token_sequence_fat_comma_pair() {
    // key => "value"
    let tokens = [
        Token::new(TokenKind::Identifier, "key", 0, 3),
        Token::new(TokenKind::FatArrow, "=>", 4, 6),
        Token::new(TokenKind::String, "\"value\"", 7, 14),
    ];
    assert_eq!(tokens[1].kind, TokenKind::FatArrow);
}

#[test]
fn token_sequence_ternary_operator() {
    // $x ? 1 : 0
    let tokens = [
        Token::new(TokenKind::ScalarSigil, "$", 0, 1),
        Token::new(TokenKind::Identifier, "x", 1, 2),
        Token::new(TokenKind::Question, "?", 3, 4),
        Token::new(TokenKind::Number, "1", 5, 6),
        Token::new(TokenKind::Colon, ":", 7, 8),
        Token::new(TokenKind::Number, "0", 9, 10),
    ];
    assert_eq!(tokens[2].kind, TokenKind::Question);
    assert_eq!(tokens[4].kind, TokenKind::Colon);
}

#[test]
fn token_sequence_regex_match() {
    // $str =~ /pattern/i
    let tokens = [
        Token::new(TokenKind::ScalarSigil, "$", 0, 1),
        Token::new(TokenKind::Identifier, "str", 1, 4),
        Token::new(TokenKind::Match, "=~", 5, 7),
        Token::new(TokenKind::Regex, "/pattern/i", 8, 18),
    ];
    assert_eq!(tokens[2].kind, TokenKind::Match);
    assert_eq!(tokens[3].kind, TokenKind::Regex);
}

#[test]
fn token_sequence_chained_arrow_deref() {
    // $obj->method->field
    let tokens = [
        Token::new(TokenKind::ScalarSigil, "$", 0, 1),
        Token::new(TokenKind::Identifier, "obj", 1, 4),
        Token::new(TokenKind::Arrow, "->", 4, 6),
        Token::new(TokenKind::Identifier, "method", 6, 12),
        Token::new(TokenKind::Arrow, "->", 12, 14),
        Token::new(TokenKind::Identifier, "field", 14, 19),
    ];
    let arrows: Vec<_> = tokens.iter().filter(|t| t.kind == TokenKind::Arrow).collect();
    assert_eq!(arrows.len(), 2);
}

#[test]
fn token_sequence_defined_or_assign() {
    // $x //= "default"
    let tokens = [
        Token::new(TokenKind::ScalarSigil, "$", 0, 1),
        Token::new(TokenKind::Identifier, "x", 1, 2),
        Token::new(TokenKind::DefinedOrAssign, "//=", 3, 6),
        Token::new(TokenKind::String, "\"default\"", 7, 16),
    ];
    assert_eq!(tokens[2].kind, TokenKind::DefinedOrAssign);
}

#[test]
fn token_sequence_try_catch_finally() {
    // try { } catch ($e) { } finally { }
    let tokens = vec![
        Token::new(TokenKind::Try, "try", 0, 3),
        Token::new(TokenKind::LeftBrace, "{", 4, 5),
        Token::new(TokenKind::RightBrace, "}", 6, 7),
        Token::new(TokenKind::Catch, "catch", 8, 13),
        Token::new(TokenKind::LeftParen, "(", 14, 15),
        Token::new(TokenKind::ScalarSigil, "$", 15, 16),
        Token::new(TokenKind::Identifier, "e", 16, 17),
        Token::new(TokenKind::RightParen, ")", 17, 18),
        Token::new(TokenKind::LeftBrace, "{", 19, 20),
        Token::new(TokenKind::RightBrace, "}", 21, 22),
        Token::new(TokenKind::Finally, "finally", 23, 30),
        Token::new(TokenKind::LeftBrace, "{", 31, 32),
        Token::new(TokenKind::RightBrace, "}", 33, 34),
    ];
    assert_eq!(tokens[0].kind, TokenKind::Try);
    assert_eq!(tokens[3].kind, TokenKind::Catch);
    assert_eq!(tokens[10].kind, TokenKind::Finally);
}

#[test]
fn token_sequence_class_method_perl538() {
    // class Foo { method bar { } }
    let tokens = [
        Token::new(TokenKind::Class, "class", 0, 5),
        Token::new(TokenKind::Identifier, "Foo", 6, 9),
        Token::new(TokenKind::LeftBrace, "{", 10, 11),
        Token::new(TokenKind::Method, "method", 12, 18),
        Token::new(TokenKind::Identifier, "bar", 19, 22),
        Token::new(TokenKind::LeftBrace, "{", 23, 24),
        Token::new(TokenKind::RightBrace, "}", 25, 26),
        Token::new(TokenKind::RightBrace, "}", 27, 28),
    ];
    assert_eq!(tokens[0].kind, TokenKind::Class);
    assert_eq!(tokens[3].kind, TokenKind::Method);
}

#[test]
fn token_sequence_heredoc() {
    // <<EOF\ncontent\nEOF
    let tokens = [
        Token::new(TokenKind::HeredocStart, "<<EOF", 0, 5),
        Token::new(TokenKind::HeredocBody, "content\n", 6, 14),
    ];
    assert_eq!(tokens[0].kind, TokenKind::HeredocStart);
    assert_eq!(tokens[1].kind, TokenKind::HeredocBody);
    assert!(tokens[1].text.contains('\n'));
}

#[test]
fn token_sequence_for_loop() {
    // for (my $i = 0; $i < 10; $i++) { }
    let tokens = vec![
        Token::new(TokenKind::For, "for", 0, 3),
        Token::new(TokenKind::LeftParen, "(", 4, 5),
        Token::new(TokenKind::My, "my", 5, 7),
        Token::new(TokenKind::ScalarSigil, "$", 8, 9),
        Token::new(TokenKind::Identifier, "i", 9, 10),
        Token::new(TokenKind::Assign, "=", 11, 12),
        Token::new(TokenKind::Number, "0", 13, 14),
        Token::new(TokenKind::Semicolon, ";", 14, 15),
        Token::new(TokenKind::ScalarSigil, "$", 16, 17),
        Token::new(TokenKind::Identifier, "i", 17, 18),
        Token::new(TokenKind::Less, "<", 19, 20),
        Token::new(TokenKind::Number, "10", 21, 23),
        Token::new(TokenKind::Semicolon, ";", 23, 24),
        Token::new(TokenKind::ScalarSigil, "$", 25, 26),
        Token::new(TokenKind::Identifier, "i", 26, 27),
        Token::new(TokenKind::Increment, "++", 27, 29),
        Token::new(TokenKind::RightParen, ")", 29, 30),
        Token::new(TokenKind::LeftBrace, "{", 31, 32),
        Token::new(TokenKind::RightBrace, "}", 33, 34),
    ];
    assert_eq!(tokens[0].kind, TokenKind::For);
    assert_eq!(tokens[15].kind, TokenKind::Increment);
}

#[test]
fn token_sequence_while_with_loop_control() {
    // while (1) { next if $skip; last; }
    let tokens = vec![
        Token::new(TokenKind::While, "while", 0, 5),
        Token::new(TokenKind::LeftParen, "(", 6, 7),
        Token::new(TokenKind::Number, "1", 7, 8),
        Token::new(TokenKind::RightParen, ")", 8, 9),
        Token::new(TokenKind::LeftBrace, "{", 10, 11),
        Token::new(TokenKind::Next, "next", 12, 16),
        Token::new(TokenKind::If, "if", 17, 19),
        Token::new(TokenKind::ScalarSigil, "$", 20, 21),
        Token::new(TokenKind::Identifier, "skip", 21, 25),
        Token::new(TokenKind::Semicolon, ";", 25, 26),
        Token::new(TokenKind::Last, "last", 27, 31),
        Token::new(TokenKind::Semicolon, ";", 31, 32),
        Token::new(TokenKind::RightBrace, "}", 33, 34),
    ];
    assert_eq!(tokens[5].kind, TokenKind::Next);
    assert_eq!(tokens[10].kind, TokenKind::Last);
}

#[test]
fn token_sequence_package_declaration() {
    // package Foo::Bar;
    let tokens = [
        Token::new(TokenKind::Package, "package", 0, 7),
        Token::new(TokenKind::Identifier, "Foo", 8, 11),
        Token::new(TokenKind::DoubleColon, "::", 11, 13),
        Token::new(TokenKind::Identifier, "Bar", 13, 16),
        Token::new(TokenKind::Semicolon, ";", 16, 17),
    ];
    assert_eq!(tokens[0].kind, TokenKind::Package);
    assert_eq!(tokens[2].kind, TokenKind::DoubleColon);
}

// ===========================================================================
// Edge cases: whitespace, escapes, special chars
// ===========================================================================

#[test]
fn token_text_with_tab_characters() {
    let tok = Token::new(TokenKind::String, "col1\tcol2\tcol3", 0, 14);
    assert!(tok.text.contains('\t'));
}

#[test]
fn token_text_with_carriage_return() {
    let tok = Token::new(TokenKind::String, "line\r\n", 0, 6);
    assert!(tok.text.contains('\r'));
}

#[test]
fn token_text_with_only_whitespace() {
    let tok = Token::new(TokenKind::String, "   \t\n  ", 0, 7);
    assert_eq!(tok.text.trim(), "");
}

#[test]
fn token_text_with_backslash_sequences() {
    let tok = Token::new(TokenKind::String, r#"hello\nworld\t!"#, 0, 15);
    assert!(tok.text.contains('\\'));
}

#[test]
fn token_text_with_high_bit_ascii() {
    let tok = Token::new(TokenKind::String, "\u{80}\u{FF}", 0, 4);
    assert_eq!(tok.text.len(), 4); // 2 two-byte codepoints
}

#[test]
fn token_text_with_cjk_characters() {
    let text = "日本語テスト";
    let tok = Token::new(TokenKind::String, text, 0, text.len());
    assert_eq!(&*tok.text, text);
    assert_eq!(tok.text.chars().count(), 6);
}

#[test]
fn token_text_with_emoji() {
    let text = "🦀🐪💎";
    let tok = Token::new(TokenKind::String, text, 0, text.len());
    assert_eq!(&*tok.text, text);
}

#[test]
fn token_text_very_long_string() {
    let long = "x".repeat(100_000);
    let tok = Token::new(TokenKind::String, long.as_str(), 0, 100_000);
    assert_eq!(tok.text.len(), 100_000);
}

#[test]
fn token_with_single_char_text() {
    for ch in ['a', 'Z', '0', '_', '$', '@'] {
        let s = ch.to_string();
        let tok = Token::new(TokenKind::Identifier, s.as_str(), 0, 1);
        assert_eq!(tok.text.len(), 1);
    }
}

// ===========================================================================
// Iterator / collection patterns
// ===========================================================================

#[test]
fn filter_tokens_by_kind() {
    let tokens = [
        Token::new(TokenKind::My, "my", 0, 2),
        Token::new(TokenKind::ScalarSigil, "$", 3, 4),
        Token::new(TokenKind::Identifier, "x", 4, 5),
        Token::new(TokenKind::Assign, "=", 6, 7),
        Token::new(TokenKind::My, "my", 8, 10),
        Token::new(TokenKind::ScalarSigil, "$", 11, 12),
        Token::new(TokenKind::Identifier, "y", 12, 13),
    ];
    let my_tokens: Vec<_> = tokens.iter().filter(|t| t.kind == TokenKind::My).collect();
    assert_eq!(my_tokens.len(), 2);
}

#[test]
fn map_tokens_to_text() {
    let tokens = [
        Token::new(TokenKind::Identifier, "foo", 0, 3),
        Token::new(TokenKind::DoubleColon, "::", 3, 5),
        Token::new(TokenKind::Identifier, "bar", 5, 8),
    ];
    let joined: String = tokens.iter().map(|t| t.text.as_ref()).collect();
    assert_eq!(joined, "foo::bar");
}

#[test]
fn partition_tokens_by_category() {
    let tokens = [
        Token::new(TokenKind::My, "my", 0, 2),
        Token::new(TokenKind::Plus, "+", 3, 4),
        Token::new(TokenKind::If, "if", 5, 7),
        Token::new(TokenKind::Minus, "-", 8, 9),
    ];
    let keywords = keyword_kinds();
    let (kw, non_kw): (Vec<_>, Vec<_>) = tokens.iter().partition(|t| keywords.contains(&t.kind));
    assert_eq!(kw.len(), 2);
    assert_eq!(non_kw.len(), 2);
}

#[test]
fn collect_token_spans() {
    let tokens = [
        Token::new(TokenKind::My, "my", 0, 2),
        Token::new(TokenKind::ScalarSigil, "$", 3, 4),
        Token::new(TokenKind::Identifier, "x", 4, 5),
    ];
    let spans: Vec<(usize, usize)> = tokens.iter().map(|t| (t.start, t.end)).collect();
    assert_eq!(spans, vec![(0, 2), (3, 4), (4, 5)]);
}

#[test]
fn find_first_identifier() {
    let tokens = [
        Token::new(TokenKind::My, "my", 0, 2),
        Token::new(TokenKind::ScalarSigil, "$", 3, 4),
        Token::new(TokenKind::Identifier, "first_id", 4, 12),
        Token::new(TokenKind::Identifier, "second_id", 13, 22),
    ];
    let first = tokens.iter().find(|t| t.kind == TokenKind::Identifier).map(|t| t.text.as_ref());
    assert_eq!(first, Some("first_id"), "should have found identifier 'first_id'");
}

#[test]
fn count_semicolons() {
    let tokens = [
        Token::new(TokenKind::Identifier, "a", 0, 1),
        Token::new(TokenKind::Semicolon, ";", 1, 2),
        Token::new(TokenKind::Identifier, "b", 3, 4),
        Token::new(TokenKind::Semicolon, ";", 4, 5),
        Token::new(TokenKind::Identifier, "c", 6, 7),
        Token::new(TokenKind::Semicolon, ";", 7, 8),
    ];
    let count = tokens.iter().filter(|t| t.kind == TokenKind::Semicolon).count();
    assert_eq!(count, 3);
}

// ===========================================================================
// Compound assignment operators
// ===========================================================================

#[test]
fn compound_assign_operators_are_distinct() {
    let compounds = [
        TokenKind::PlusAssign,
        TokenKind::MinusAssign,
        TokenKind::StarAssign,
        TokenKind::SlashAssign,
        TokenKind::PercentAssign,
        TokenKind::DotAssign,
        TokenKind::AndAssign,
        TokenKind::OrAssign,
        TokenKind::XorAssign,
        TokenKind::PowerAssign,
        TokenKind::LeftShiftAssign,
        TokenKind::RightShiftAssign,
        TokenKind::LogicalAndAssign,
        TokenKind::LogicalOrAssign,
        TokenKind::DefinedOrAssign,
    ];
    for (i, a) in compounds.iter().enumerate() {
        for b in &compounds[i + 1..] {
            assert_ne!(a, b);
        }
    }
    assert_eq!(compounds.len(), 15);
}

// ===========================================================================
// Word operators vs symbolic operators
// ===========================================================================

#[test]
fn word_operators_are_distinct_from_symbolic() {
    assert_ne!(TokenKind::WordAnd, TokenKind::And);
    assert_ne!(TokenKind::WordOr, TokenKind::Or);
    assert_ne!(TokenKind::WordNot, TokenKind::Not);
}

#[test]
fn word_xor_has_no_symbolic_counterpart() {
    // BitwiseXor is ^, WordXor is xor — they're different concepts
    assert_ne!(TokenKind::WordXor, TokenKind::BitwiseXor);
}

// ===========================================================================
// Phase block tokens
// ===========================================================================

#[test]
fn phase_blocks_are_all_distinct() {
    let phases =
        [TokenKind::Begin, TokenKind::End, TokenKind::Check, TokenKind::Init, TokenKind::Unitcheck];
    for (i, a) in phases.iter().enumerate() {
        for b in &phases[i + 1..] {
            assert_ne!(a, b);
        }
    }
}

// ===========================================================================
// Special / error tokens
// ===========================================================================

#[test]
fn unknown_rest_token_for_budget_exceeded() {
    let tok = Token::new(TokenKind::UnknownRest, "...remainder...", 500, 515);
    assert_eq!(tok.kind, TokenKind::UnknownRest);
    assert_eq!(&*tok.text, "...remainder...");
}

#[test]
fn heredoc_depth_limit_token() {
    let tok = Token::new(TokenKind::HeredocDepthLimit, "<<DEEP", 0, 6);
    assert_eq!(tok.kind, TokenKind::HeredocDepthLimit);
}

#[test]
fn data_marker_and_body() {
    let marker = Token::new(TokenKind::DataMarker, "__END__", 0, 7);
    let body = Token::new(TokenKind::DataBody, "some trailing data", 8, 26);
    assert_eq!(marker.kind, TokenKind::DataMarker);
    assert_eq!(body.kind, TokenKind::DataBody);
    assert_ne!(marker.kind, body.kind);
}

#[test]
fn format_body_token() {
    let tok = Token::new(TokenKind::FormatBody, "@<<<< $name\n.", 0, 13);
    assert_eq!(tok.kind, TokenKind::FormatBody);
}

// ===========================================================================
// Token in Option and Result contexts
// ===========================================================================

#[test]
fn option_token_some() {
    let maybe: Option<Token> = Some(Token::new(TokenKind::Number, "42", 0, 2));
    assert!(maybe.is_some(), "expected Some");
    assert_eq!(maybe.as_ref().map(|t| t.kind), Some(TokenKind::Number), "expected Number token");
}

#[test]
fn option_token_none() {
    let maybe: Option<Token> = None;
    assert!(maybe.is_none());
}

#[test]
fn vec_first_and_last() {
    let tokens = [
        Token::new(TokenKind::Use, "use", 0, 3),
        Token::new(TokenKind::Identifier, "strict", 4, 10),
        Token::new(TokenKind::Semicolon, ";", 10, 11),
    ];
    if let Some(first) = tokens.first() {
        assert_eq!(first.kind, TokenKind::Use);
    }
    if let Some(last) = tokens.last() {
        assert_eq!(last.kind, TokenKind::Semicolon);
    }
}

// ===========================================================================
// Regression-style: ensure no accidental overlap between similar variants
// ===========================================================================

#[test]
fn match_vs_smart_match() {
    assert_ne!(TokenKind::Match, TokenKind::SmartMatch);
}

#[test]
fn not_vs_bitwise_not() {
    assert_ne!(TokenKind::Not, TokenKind::BitwiseNot);
}

#[test]
fn star_vs_glob_sigil() {
    assert_ne!(TokenKind::Star, TokenKind::GlobSigil);
}

#[test]
fn percent_vs_hash_sigil() {
    assert_ne!(TokenKind::Percent, TokenKind::HashSigil);
}

#[test]
fn bitwise_and_vs_sub_sigil() {
    assert_ne!(TokenKind::BitwiseAnd, TokenKind::SubSigil);
}

#[test]
fn slash_vs_defined_or() {
    assert_ne!(TokenKind::Slash, TokenKind::DefinedOr);
}

#[test]
fn less_vs_left_shift() {
    assert_ne!(TokenKind::Less, TokenKind::LeftShift);
}

#[test]
fn greater_vs_right_shift() {
    assert_ne!(TokenKind::Greater, TokenKind::RightShift);
}
