//! Comprehensive unit tests for the `perl-token` crate.
//!
//! Covers:
//! - Token construction and field access
//! - Token Clone semantics (Arc<str> sharing)
//! - Token PartialEq / Debug
//! - TokenKind variant exhaustiveness
//! - TokenKind Copy / Clone / Eq / Debug
//! - Edge cases: empty text, zero-length spans, unicode, large offsets

use perl_token::{Token, TokenKind};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Token construction
// ---------------------------------------------------------------------------

#[test]
fn token_new_basic_fields() {
    let t = Token::new(TokenKind::My, "my", 0, 2);
    assert_eq!(t.kind, TokenKind::My);
    assert_eq!(&*t.text, "my");
    assert_eq!(t.start, 0);
    assert_eq!(t.end, 2);
}

#[test]
fn token_new_accepts_string() {
    let s = String::from("hello");
    let t = Token::new(TokenKind::String, s, 5, 12);
    assert_eq!(&*t.text, "hello");
    assert_eq!(t.start, 5);
    assert_eq!(t.end, 12);
}

#[test]
fn token_new_accepts_arc_str() {
    let arc: Arc<str> = Arc::from("world");
    let t = Token::new(TokenKind::Identifier, arc.clone(), 0, 5);
    assert_eq!(&*t.text, "world");
    // Shared Arc: strong count should be 2
    assert_eq!(Arc::strong_count(&arc), 2);
}

#[test]
fn token_new_empty_text() {
    let t = Token::new(TokenKind::Eof, "", 100, 100);
    assert_eq!(&*t.text, "");
    assert_eq!(t.start, 100);
    assert_eq!(t.end, 100);
}

#[test]
fn token_new_zero_length_span() {
    let t = Token::new(TokenKind::Semicolon, ";", 42, 42);
    assert_eq!(t.start, t.end);
    assert!(t.is_empty());
}

#[test]
fn token_len_reports_span_width() {
    let t = Token::new(TokenKind::Identifier, "hello", 7, 12);
    assert_eq!(t.len(), 5);
    assert!(!t.is_empty());
}

#[test]
fn token_len_saturates_for_malformed_span() {
    let t = Token::new(TokenKind::Unknown, "?", 12, 7);
    assert_eq!(t.len(), 0);
    assert!(t.is_empty());
}

#[test]
fn token_new_large_offsets() {
    let big = usize::MAX - 1;
    let t = Token::new(TokenKind::Number, "99", big, usize::MAX);
    assert_eq!(t.start, big);
    assert_eq!(t.end, usize::MAX);
}

#[test]
fn token_new_unicode_text() {
    let t = Token::new(TokenKind::String, "héllo wörld 🦀", 0, 19);
    assert_eq!(&*t.text, "héllo wörld 🦀");
}

// ---------------------------------------------------------------------------
// Token Clone
// ---------------------------------------------------------------------------

#[test]
fn token_clone_is_equal() {
    let t = Token::new(TokenKind::Sub, "sub", 10, 13);
    let c = t.clone();
    assert_eq!(t, c);
}

#[test]
fn token_clone_shares_arc() {
    let t = Token::new(TokenKind::Identifier, "foo_bar", 0, 7);
    let c = t.clone();
    // Arc::ptr_eq confirms they share the same allocation
    assert!(Arc::ptr_eq(&t.text, &c.text));
}

// ---------------------------------------------------------------------------
// Token PartialEq
// ---------------------------------------------------------------------------

#[test]
fn token_eq_same_fields() {
    let a = Token::new(TokenKind::If, "if", 0, 2);
    let b = Token::new(TokenKind::If, "if", 0, 2);
    assert_eq!(a, b);
}

#[test]
fn token_ne_different_kind() {
    let a = Token::new(TokenKind::If, "if", 0, 2);
    let b = Token::new(TokenKind::Else, "if", 0, 2);
    assert_ne!(a, b);
}

#[test]
fn token_ne_different_text() {
    let a = Token::new(TokenKind::Identifier, "foo", 0, 3);
    let b = Token::new(TokenKind::Identifier, "bar", 0, 3);
    assert_ne!(a, b);
}

#[test]
fn token_ne_different_start() {
    let a = Token::new(TokenKind::Number, "1", 0, 1);
    let b = Token::new(TokenKind::Number, "1", 5, 1);
    assert_ne!(a, b);
}

#[test]
fn token_ne_different_end() {
    let a = Token::new(TokenKind::Number, "1", 0, 1);
    let b = Token::new(TokenKind::Number, "1", 0, 6);
    assert_ne!(a, b);
}

// ---------------------------------------------------------------------------
// Token Debug
// ---------------------------------------------------------------------------

#[test]
fn token_debug_contains_kind_and_text() {
    let t = Token::new(TokenKind::Return, "return", 0, 6);
    let dbg = format!("{t:?}");
    assert!(dbg.contains("Return"), "expected 'Return' in debug: {dbg}");
    assert!(dbg.contains("return"), "expected 'return' in debug: {dbg}");
}

// ---------------------------------------------------------------------------
// TokenKind — variant exhaustiveness
// ---------------------------------------------------------------------------

fn all_token_kinds() -> &'static [TokenKind] {
    TokenKind::all()
}

#[test]
fn all_variants_are_listed() {
    // If a new variant is added to TokenKind the compiler will force an update
    // to the match below, keeping the test in sync.
    let kinds = all_token_kinds();
    for kind in kinds {
        match kind {
            TokenKind::My
            | TokenKind::Our
            | TokenKind::Local
            | TokenKind::State
            | TokenKind::Sub
            | TokenKind::If
            | TokenKind::Elsif
            | TokenKind::Else
            | TokenKind::Unless
            | TokenKind::While
            | TokenKind::Until
            | TokenKind::For
            | TokenKind::Foreach
            | TokenKind::Return
            | TokenKind::Package
            | TokenKind::Use
            | TokenKind::No
            | TokenKind::Begin
            | TokenKind::End
            | TokenKind::Check
            | TokenKind::Init
            | TokenKind::Unitcheck
            | TokenKind::Eval
            | TokenKind::Do
            | TokenKind::Given
            | TokenKind::When
            | TokenKind::Default
            | TokenKind::Try
            | TokenKind::Catch
            | TokenKind::Finally
            | TokenKind::Continue
            | TokenKind::Next
            | TokenKind::Last
            | TokenKind::Redo
            | TokenKind::Goto
            | TokenKind::Class
            | TokenKind::Method
            | TokenKind::Field
            | TokenKind::Format
            | TokenKind::Undef
            | TokenKind::Defer
            | TokenKind::Assign
            | TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Star
            | TokenKind::Slash
            | TokenKind::Percent
            | TokenKind::Power
            | TokenKind::LeftShift
            | TokenKind::RightShift
            | TokenKind::BitwiseAnd
            | TokenKind::BitwiseOr
            | TokenKind::BitwiseXor
            | TokenKind::BitwiseNot
            | TokenKind::PlusAssign
            | TokenKind::MinusAssign
            | TokenKind::StarAssign
            | TokenKind::SlashAssign
            | TokenKind::PercentAssign
            | TokenKind::DotAssign
            | TokenKind::AndAssign
            | TokenKind::OrAssign
            | TokenKind::XorAssign
            | TokenKind::PowerAssign
            | TokenKind::LeftShiftAssign
            | TokenKind::RightShiftAssign
            | TokenKind::LogicalAndAssign
            | TokenKind::LogicalOrAssign
            | TokenKind::DefinedOrAssign
            | TokenKind::Equal
            | TokenKind::NotEqual
            | TokenKind::Match
            | TokenKind::NotMatch
            | TokenKind::SmartMatch
            | TokenKind::Less
            | TokenKind::Greater
            | TokenKind::LessEqual
            | TokenKind::GreaterEqual
            | TokenKind::Spaceship
            | TokenKind::StringCompare
            | TokenKind::And
            | TokenKind::Or
            | TokenKind::Not
            | TokenKind::DefinedOr
            | TokenKind::WordAnd
            | TokenKind::WordOr
            | TokenKind::WordNot
            | TokenKind::WordXor
            | TokenKind::Arrow
            | TokenKind::FatArrow
            | TokenKind::Dot
            | TokenKind::Range
            | TokenKind::Ellipsis
            | TokenKind::Increment
            | TokenKind::Decrement
            | TokenKind::DoubleColon
            | TokenKind::Question
            | TokenKind::Colon
            | TokenKind::Backslash
            | TokenKind::LeftParen
            | TokenKind::RightParen
            | TokenKind::LeftBrace
            | TokenKind::RightBrace
            | TokenKind::LeftBracket
            | TokenKind::RightBracket
            | TokenKind::Semicolon
            | TokenKind::Comma
            | TokenKind::Number
            | TokenKind::String
            | TokenKind::Regex
            | TokenKind::Substitution
            | TokenKind::Transliteration
            | TokenKind::QuoteSingle
            | TokenKind::QuoteDouble
            | TokenKind::QuoteWords
            | TokenKind::QuoteCommand
            | TokenKind::HeredocStart
            | TokenKind::HeredocBody
            | TokenKind::FormatBody
            | TokenKind::DataMarker
            | TokenKind::DataBody
            | TokenKind::VString
            | TokenKind::UnknownRest
            | TokenKind::HeredocDepthLimit
            | TokenKind::Identifier
            | TokenKind::ScalarSigil
            | TokenKind::ArraySigil
            | TokenKind::HashSigil
            | TokenKind::SubSigil
            | TokenKind::GlobSigil
            | TokenKind::Eof
            | TokenKind::Unknown => {}
        }
    }
}

#[test]
fn all_variants_unique() {
    let kinds = all_token_kinds();
    for (i, a) in kinds.iter().enumerate() {
        for b in &kinds[i + 1..] {
            assert_ne!(a, b, "duplicate variant detected");
        }
    }
}

// ---------------------------------------------------------------------------
// TokenKind — Copy / Clone / Eq / Debug
// ---------------------------------------------------------------------------

#[test]
fn token_kind_is_copy() {
    let a = TokenKind::My;
    let b = a; // Copy
    assert_eq!(a, b);
}

#[test]
fn token_kind_clone_equals_original() {
    let a = TokenKind::Regex;
    let b = a;
    assert_eq!(a, b);
}

#[test]
fn token_kind_eq_is_reflexive() {
    for kind in all_token_kinds() {
        assert_eq!(kind, kind);
    }
}

#[test]
fn token_kind_eq_is_symmetric() {
    let a = TokenKind::Arrow;
    let b = TokenKind::Arrow;
    assert_eq!(a, b);
    assert_eq!(b, a);
}

#[test]
fn token_kind_ne_across_categories() {
    assert_ne!(TokenKind::My, TokenKind::Assign);
    assert_ne!(TokenKind::LeftParen, TokenKind::Number);
    assert_ne!(TokenKind::Identifier, TokenKind::Eof);
}

#[test]
fn token_kind_debug_is_non_empty() {
    for kind in all_token_kinds() {
        let dbg = format!("{kind:?}");
        assert!(!dbg.is_empty(), "Debug for {kind:?} should be non-empty");
    }
}

// ---------------------------------------------------------------------------
// Keyword tokens
// ---------------------------------------------------------------------------

#[test]
fn keyword_token_round_trip() {
    let keywords: &[(TokenKind, &str)] = &[
        (TokenKind::My, "my"),
        (TokenKind::Our, "our"),
        (TokenKind::Local, "local"),
        (TokenKind::State, "state"),
        (TokenKind::Sub, "sub"),
        (TokenKind::If, "if"),
        (TokenKind::Elsif, "elsif"),
        (TokenKind::Else, "else"),
        (TokenKind::Unless, "unless"),
        (TokenKind::While, "while"),
        (TokenKind::Until, "until"),
        (TokenKind::For, "for"),
        (TokenKind::Foreach, "foreach"),
        (TokenKind::Return, "return"),
        (TokenKind::Package, "package"),
        (TokenKind::Use, "use"),
        (TokenKind::No, "no"),
        (TokenKind::Begin, "BEGIN"),
        (TokenKind::End, "END"),
        (TokenKind::Check, "CHECK"),
        (TokenKind::Init, "INIT"),
        (TokenKind::Unitcheck, "UNITCHECK"),
        (TokenKind::Eval, "eval"),
        (TokenKind::Do, "do"),
        (TokenKind::Given, "given"),
        (TokenKind::When, "when"),
        (TokenKind::Default, "default"),
        (TokenKind::Try, "try"),
        (TokenKind::Catch, "catch"),
        (TokenKind::Finally, "finally"),
        (TokenKind::Continue, "continue"),
        (TokenKind::Next, "next"),
        (TokenKind::Last, "last"),
        (TokenKind::Redo, "redo"),
        (TokenKind::Goto, "goto"),
        (TokenKind::Class, "class"),
        (TokenKind::Method, "method"),
        (TokenKind::Field, "field"),
        (TokenKind::Format, "format"),
        (TokenKind::Undef, "undef"),
        (TokenKind::Defer, "defer"),
    ];

    let mut offset = 0;
    for (kind, text) in keywords {
        let end = offset + text.len();
        let tok = Token::new(*kind, *text, offset, end);
        assert_eq!(tok.kind, *kind);
        assert_eq!(&*tok.text, *text);
        assert_eq!(tok.start, offset);
        assert_eq!(tok.end, end);
        offset = end + 1; // simulate whitespace gap
    }
}

// ---------------------------------------------------------------------------
// Operator tokens
// ---------------------------------------------------------------------------

#[test]
fn operator_token_round_trip() {
    let operators: &[(TokenKind, &str)] = &[
        (TokenKind::Assign, "="),
        (TokenKind::Plus, "+"),
        (TokenKind::Minus, "-"),
        (TokenKind::Star, "*"),
        (TokenKind::Slash, "/"),
        (TokenKind::Percent, "%"),
        (TokenKind::Power, "**"),
        (TokenKind::LeftShift, "<<"),
        (TokenKind::RightShift, ">>"),
        (TokenKind::BitwiseAnd, "&"),
        (TokenKind::BitwiseOr, "|"),
        (TokenKind::BitwiseXor, "^"),
        (TokenKind::BitwiseNot, "~"),
        (TokenKind::PlusAssign, "+="),
        (TokenKind::MinusAssign, "-="),
        (TokenKind::StarAssign, "*="),
        (TokenKind::SlashAssign, "/="),
        (TokenKind::PercentAssign, "%="),
        (TokenKind::DotAssign, ".="),
        (TokenKind::AndAssign, "&="),
        (TokenKind::OrAssign, "|="),
        (TokenKind::XorAssign, "^="),
        (TokenKind::PowerAssign, "**="),
        (TokenKind::LeftShiftAssign, "<<="),
        (TokenKind::RightShiftAssign, ">>="),
        (TokenKind::LogicalAndAssign, "&&="),
        (TokenKind::LogicalOrAssign, "||="),
        (TokenKind::DefinedOrAssign, "//="),
        (TokenKind::Equal, "=="),
        (TokenKind::NotEqual, "!="),
        (TokenKind::Match, "=~"),
        (TokenKind::NotMatch, "!~"),
        (TokenKind::SmartMatch, "~~"),
        (TokenKind::Less, "<"),
        (TokenKind::Greater, ">"),
        (TokenKind::LessEqual, "<="),
        (TokenKind::GreaterEqual, ">="),
        (TokenKind::Spaceship, "<=>"),
        (TokenKind::StringCompare, "cmp"),
        (TokenKind::And, "&&"),
        (TokenKind::Or, "||"),
        (TokenKind::Not, "!"),
        (TokenKind::DefinedOr, "//"),
        (TokenKind::WordAnd, "and"),
        (TokenKind::WordOr, "or"),
        (TokenKind::WordNot, "not"),
        (TokenKind::WordXor, "xor"),
        (TokenKind::Arrow, "->"),
        (TokenKind::FatArrow, "=>"),
        (TokenKind::Dot, "."),
        (TokenKind::Range, ".."),
        (TokenKind::Ellipsis, "..."),
        (TokenKind::Increment, "++"),
        (TokenKind::Decrement, "--"),
        (TokenKind::DoubleColon, "::"),
        (TokenKind::Question, "?"),
        (TokenKind::Colon, ":"),
        (TokenKind::Backslash, "\\"),
    ];

    for (kind, text) in operators {
        let tok = Token::new(*kind, *text, 0, text.len());
        assert_eq!(tok.kind, *kind, "operator {text}");
        assert_eq!(&*tok.text, *text);
    }
}

// ---------------------------------------------------------------------------
// Delimiter tokens
// ---------------------------------------------------------------------------

#[test]
fn delimiter_token_round_trip() {
    let delimiters: &[(TokenKind, &str)] = &[
        (TokenKind::LeftParen, "("),
        (TokenKind::RightParen, ")"),
        (TokenKind::LeftBrace, "{"),
        (TokenKind::RightBrace, "}"),
        (TokenKind::LeftBracket, "["),
        (TokenKind::RightBracket, "]"),
        (TokenKind::Semicolon, ";"),
        (TokenKind::Comma, ","),
    ];

    for (kind, text) in delimiters {
        let tok = Token::new(*kind, *text, 0, text.len());
        assert_eq!(tok.kind, *kind, "delimiter {text}");
        assert_eq!(&*tok.text, *text);
    }
}

// ---------------------------------------------------------------------------
// Literal tokens
// ---------------------------------------------------------------------------

#[test]
fn literal_token_round_trip() {
    let literals: &[(TokenKind, &str)] = &[
        (TokenKind::Number, "42"),
        (TokenKind::Number, "3.14"),
        (TokenKind::Number, "0xFF"),
        (TokenKind::String, "\"hello\""),
        (TokenKind::String, "'world'"),
        (TokenKind::Regex, "/pattern/i"),
        (TokenKind::Substitution, "s/foo/bar/g"),
        (TokenKind::Transliteration, "tr/a-z/A-Z/"),
        (TokenKind::QuoteSingle, "q/text/"),
        (TokenKind::QuoteDouble, "qq/text/"),
        (TokenKind::QuoteWords, "qw(foo bar)"),
        (TokenKind::QuoteCommand, "qx/ls/"),
        (TokenKind::HeredocStart, "<<EOF"),
        (TokenKind::HeredocBody, "line1\nline2\n"),
        (TokenKind::FormatBody, "@<<<< $name"),
        (TokenKind::DataMarker, "__DATA__"),
        (TokenKind::DataBody, "some data content"),
        (TokenKind::UnknownRest, "...leftover..."),
        (TokenKind::HeredocDepthLimit, "<<DEEP"),
    ];

    for (kind, text) in literals {
        let tok = Token::new(*kind, *text, 0, text.len());
        assert_eq!(tok.kind, *kind, "literal {text}");
        assert_eq!(&*tok.text, *text);
    }
}

// ---------------------------------------------------------------------------
// Identifier and sigil tokens
// ---------------------------------------------------------------------------

#[test]
fn identifier_token() {
    let tok = Token::new(TokenKind::Identifier, "some_func", 0, 9);
    assert_eq!(tok.kind, TokenKind::Identifier);
    assert_eq!(&*tok.text, "some_func");
}

#[test]
fn sigil_tokens() {
    let sigils: &[(TokenKind, &str)] = &[
        (TokenKind::ScalarSigil, "$"),
        (TokenKind::ArraySigil, "@"),
        (TokenKind::HashSigil, "%"),
        (TokenKind::SubSigil, "&"),
        (TokenKind::GlobSigil, "*"),
    ];

    for (kind, text) in sigils {
        let tok = Token::new(*kind, *text, 0, 1);
        assert_eq!(tok.kind, *kind, "sigil {text}");
        assert_eq!(&*tok.text, *text);
    }
}

// ---------------------------------------------------------------------------
// Special tokens
// ---------------------------------------------------------------------------

#[test]
fn eof_token() {
    let tok = Token::new(TokenKind::Eof, "", 500, 500);
    assert_eq!(tok.kind, TokenKind::Eof);
    assert_eq!(&*tok.text, "");
}

#[test]
fn unknown_token() {
    let tok = Token::new(TokenKind::Unknown, "\x01", 0, 1);
    assert_eq!(tok.kind, TokenKind::Unknown);
}

// ---------------------------------------------------------------------------
// Token sequence simulation
// ---------------------------------------------------------------------------

#[test]
fn token_sequence_my_x_assign_42() {
    // Simulates: my $x = 42;
    let tokens = [
        Token::new(TokenKind::My, "my", 0, 2),
        Token::new(TokenKind::ScalarSigil, "$", 3, 4),
        Token::new(TokenKind::Identifier, "x", 4, 5),
        Token::new(TokenKind::Assign, "=", 6, 7),
        Token::new(TokenKind::Number, "42", 8, 10),
        Token::new(TokenKind::Semicolon, ";", 10, 11),
    ];

    assert_eq!(tokens.len(), 6);
    assert_eq!(tokens[0].kind, TokenKind::My);
    assert_eq!(tokens[1].kind, TokenKind::ScalarSigil);
    assert_eq!(tokens[2].kind, TokenKind::Identifier);
    assert_eq!(tokens[3].kind, TokenKind::Assign);
    assert_eq!(tokens[4].kind, TokenKind::Number);
    assert_eq!(tokens[5].kind, TokenKind::Semicolon);
}

#[test]
fn token_sequence_sub_declaration() {
    // Simulates: sub greet { }
    let tokens = [
        Token::new(TokenKind::Sub, "sub", 0, 3),
        Token::new(TokenKind::Identifier, "greet", 4, 9),
        Token::new(TokenKind::LeftBrace, "{", 10, 11),
        Token::new(TokenKind::RightBrace, "}", 12, 13),
    ];

    assert_eq!(tokens.first().map(|t| t.kind), Some(TokenKind::Sub));
    assert_eq!(tokens.last().map(|t| t.kind), Some(TokenKind::RightBrace));
}

#[test]
fn token_sequence_method_call() {
    // Simulates: $obj->method()
    let tokens = [
        Token::new(TokenKind::ScalarSigil, "$", 0, 1),
        Token::new(TokenKind::Identifier, "obj", 1, 4),
        Token::new(TokenKind::Arrow, "->", 4, 6),
        Token::new(TokenKind::Identifier, "method", 6, 12),
        Token::new(TokenKind::LeftParen, "(", 12, 13),
        Token::new(TokenKind::RightParen, ")", 13, 14),
    ];

    assert_eq!(tokens[2].kind, TokenKind::Arrow);
    assert_eq!(&*tokens[2].text, "->");
}

#[test]
fn token_sequence_package_qualified() {
    // Simulates: Foo::Bar::baz
    let tokens = [
        Token::new(TokenKind::Identifier, "Foo", 0, 3),
        Token::new(TokenKind::DoubleColon, "::", 3, 5),
        Token::new(TokenKind::Identifier, "Bar", 5, 8),
        Token::new(TokenKind::DoubleColon, "::", 8, 10),
        Token::new(TokenKind::Identifier, "baz", 10, 13),
    ];

    assert_eq!(tokens.len(), 5);
    assert_eq!(tokens[1].kind, TokenKind::DoubleColon);
    assert_eq!(tokens[3].kind, TokenKind::DoubleColon);
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

#[test]
fn token_with_multiline_text() {
    let heredoc = "line1\nline2\nline3\n";
    let tok = Token::new(TokenKind::HeredocBody, heredoc, 0, heredoc.len());
    assert_eq!(&*tok.text, heredoc);
    assert!(tok.text.contains('\n'));
}

#[test]
fn token_with_null_byte() {
    let tok = Token::new(TokenKind::String, "a\0b", 0, 3);
    assert_eq!(&*tok.text, "a\0b");
}

#[test]
fn many_tokens_in_vec() {
    let tokens: Vec<Token> =
        (0..1000).map(|i| Token::new(TokenKind::Number, i.to_string(), i * 4, i * 4 + 3)).collect();

    assert_eq!(tokens.len(), 1000);
    assert_eq!(&*tokens[0].text, "0");
    assert_eq!(&*tokens[999].text, "999");
}
