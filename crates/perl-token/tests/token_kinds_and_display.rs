//! Tests for TokenKind classification, display_name coverage, and source location tracking.
//!
//! Supplements existing test files with:
//! - Exhaustive display_name() verification for every TokenKind variant
//! - Token classification by category (keyword, operator, delimiter, literal, sigil, special)
//! - Source location (span) tracking and consistency
//! - Missing variant coverage (Field, Goto in keyword lists)
//! - Token equality across different construction paths

use perl_token::{Token, TokenCategory, TokenKind};
use std::sync::Arc;

// ===========================================================================
// Helpers: classify TokenKind into categories
// ===========================================================================

fn is_keyword(kind: TokenKind) -> bool {
    kind.is_keyword()
}

fn is_operator(kind: TokenKind) -> bool {
    kind.is_operator()
}

fn is_delimiter(kind: TokenKind) -> bool {
    kind.is_delimiter()
}

fn is_literal(kind: TokenKind) -> bool {
    kind.is_literal()
}

fn is_identifier_or_sigil(kind: TokenKind) -> bool {
    kind.is_identifier_like()
}

fn is_special(kind: TokenKind) -> bool {
    kind.is_special()
}

/// Every variant in TokenKind, sourced from production metadata.
fn all_kinds() -> Vec<TokenKind> {
    TokenKind::all().to_vec()
}

// ===========================================================================
// display_name() exhaustive tests
// ===========================================================================

#[test]
fn display_name_keywords() {
    let cases: &[(TokenKind, &str)] = &[
        (TokenKind::My, "'my'"),
        (TokenKind::Our, "'our'"),
        (TokenKind::Local, "'local'"),
        (TokenKind::State, "'state'"),
        (TokenKind::Sub, "'sub'"),
        (TokenKind::If, "'if'"),
        (TokenKind::Elsif, "'elsif'"),
        (TokenKind::Else, "'else'"),
        (TokenKind::Unless, "'unless'"),
        (TokenKind::While, "'while'"),
        (TokenKind::Until, "'until'"),
        (TokenKind::For, "'for'"),
        (TokenKind::Foreach, "'foreach'"),
        (TokenKind::Return, "'return'"),
        (TokenKind::Package, "'package'"),
        (TokenKind::Use, "'use'"),
        (TokenKind::No, "'no'"),
        (TokenKind::Begin, "'BEGIN'"),
        (TokenKind::End, "'END'"),
        (TokenKind::Check, "'CHECK'"),
        (TokenKind::Init, "'INIT'"),
        (TokenKind::Unitcheck, "'UNITCHECK'"),
        (TokenKind::Eval, "'eval'"),
        (TokenKind::Do, "'do'"),
        (TokenKind::Given, "'given'"),
        (TokenKind::When, "'when'"),
        (TokenKind::Default, "'default'"),
        (TokenKind::Try, "'try'"),
        (TokenKind::Catch, "'catch'"),
        (TokenKind::Finally, "'finally'"),
        (TokenKind::Continue, "'continue'"),
        (TokenKind::Next, "'next'"),
        (TokenKind::Last, "'last'"),
        (TokenKind::Redo, "'redo'"),
        (TokenKind::Goto, "'goto'"),
        (TokenKind::Class, "'class'"),
        (TokenKind::Method, "'method'"),
        (TokenKind::Field, "'field'"),
        (TokenKind::Format, "'format'"),
        (TokenKind::Undef, "'undef'"),
        (TokenKind::Defer, "'defer'"),
    ];
    for (kind, expected) in cases {
        assert_eq!(kind.display_name(), *expected, "display_name mismatch for {kind:?}");
    }
}

#[test]
fn display_name_operators() {
    let cases: &[(TokenKind, &str)] = &[
        (TokenKind::Assign, "'='"),
        (TokenKind::Plus, "'+'"),
        (TokenKind::Minus, "'-'"),
        (TokenKind::Star, "'*'"),
        (TokenKind::Slash, "'/'"),
        (TokenKind::Percent, "'%'"),
        (TokenKind::Power, "'**'"),
        (TokenKind::LeftShift, "'<<'"),
        (TokenKind::RightShift, "'>>'"),
        (TokenKind::BitwiseAnd, "'&'"),
        (TokenKind::BitwiseOr, "'|'"),
        (TokenKind::BitwiseXor, "'^'"),
        (TokenKind::BitwiseNot, "'~'"),
        (TokenKind::PlusAssign, "'+='"),
        (TokenKind::MinusAssign, "'-='"),
        (TokenKind::StarAssign, "'*='"),
        (TokenKind::SlashAssign, "'/='"),
        (TokenKind::PercentAssign, "'%='"),
        (TokenKind::DotAssign, "'.='"),
        (TokenKind::AndAssign, "'&='"),
        (TokenKind::OrAssign, "'|='"),
        (TokenKind::XorAssign, "'^='"),
        (TokenKind::PowerAssign, "'**='"),
        (TokenKind::LeftShiftAssign, "'<<='"),
        (TokenKind::RightShiftAssign, "'>>='"),
        (TokenKind::LogicalAndAssign, "'&&='"),
        (TokenKind::LogicalOrAssign, "'||='"),
        (TokenKind::DefinedOrAssign, "'//='"),
        (TokenKind::Equal, "'=='"),
        (TokenKind::NotEqual, "'!='"),
        (TokenKind::Match, "'=~'"),
        (TokenKind::NotMatch, "'!~'"),
        (TokenKind::SmartMatch, "'~~'"),
        (TokenKind::Less, "'<'"),
        (TokenKind::Greater, "'>'"),
        (TokenKind::LessEqual, "'<='"),
        (TokenKind::GreaterEqual, "'>='"),
        (TokenKind::Spaceship, "'<=>'"),
        (TokenKind::StringCompare, "'cmp'"),
        (TokenKind::And, "'&&'"),
        (TokenKind::Or, "'||'"),
        (TokenKind::Not, "'!'"),
        (TokenKind::DefinedOr, "'//'"),
        (TokenKind::WordAnd, "'and'"),
        (TokenKind::WordOr, "'or'"),
        (TokenKind::WordNot, "'not'"),
        (TokenKind::WordXor, "'xor'"),
        (TokenKind::Arrow, "'->'"),
        (TokenKind::FatArrow, "'=>'"),
        (TokenKind::Dot, "'.'"),
        (TokenKind::Range, "'..'"),
        (TokenKind::Ellipsis, "'...'"),
        (TokenKind::Increment, "'++'"),
        (TokenKind::Decrement, "'--'"),
        (TokenKind::DoubleColon, "'::'"),
        (TokenKind::Question, "'?'"),
        (TokenKind::Colon, "':'"),
        (TokenKind::Backslash, "'\\'"),
    ];
    for (kind, expected) in cases {
        assert_eq!(kind.display_name(), *expected, "display_name mismatch for {kind:?}");
    }
}

#[test]
fn display_name_delimiters() {
    let cases: &[(TokenKind, &str)] = &[
        (TokenKind::LeftParen, "'('"),
        (TokenKind::RightParen, "')'"),
        (TokenKind::LeftBrace, "'{'"),
        (TokenKind::RightBrace, "'}'"),
        (TokenKind::LeftBracket, "'['"),
        (TokenKind::RightBracket, "']'"),
        (TokenKind::Semicolon, "';'"),
        (TokenKind::Comma, "','"),
    ];
    for (kind, expected) in cases {
        assert_eq!(kind.display_name(), *expected, "display_name mismatch for {kind:?}");
    }
}

#[test]
fn display_name_literals() {
    let cases: &[(TokenKind, &str)] = &[
        (TokenKind::Number, "number"),
        (TokenKind::String, "string"),
        (TokenKind::Regex, "regex"),
        (TokenKind::Substitution, "substitution (s///)"),
        (TokenKind::Transliteration, "transliteration (tr///)"),
        (TokenKind::QuoteSingle, "q// string"),
        (TokenKind::QuoteDouble, "qq// string"),
        (TokenKind::QuoteWords, "qw() word list"),
        (TokenKind::QuoteCommand, "qx// command"),
        (TokenKind::HeredocStart, "heredoc (<<)"),
        (TokenKind::HeredocBody, "heredoc body"),
        (TokenKind::FormatBody, "format body"),
        (TokenKind::DataMarker, "__DATA__"),
        (TokenKind::DataBody, "data section"),
        (TokenKind::VString, "version string"),
        (TokenKind::UnknownRest, "unparsed content"),
        (TokenKind::HeredocDepthLimit, "heredoc depth limit"),
    ];
    for (kind, expected) in cases {
        assert_eq!(kind.display_name(), *expected, "display_name mismatch for {kind:?}");
    }
}

#[test]
fn display_name_identifiers_and_sigils() {
    let cases: &[(TokenKind, &str)] = &[
        (TokenKind::Identifier, "identifier"),
        (TokenKind::ScalarSigil, "'$'"),
        (TokenKind::ArraySigil, "'@'"),
        (TokenKind::HashSigil, "'%'"),
        (TokenKind::SubSigil, "'&'"),
        (TokenKind::GlobSigil, "'*'"),
    ];
    for (kind, expected) in cases {
        assert_eq!(kind.display_name(), *expected, "display_name mismatch for {kind:?}");
    }
}

#[test]
fn display_name_special() {
    assert_eq!(TokenKind::Eof.display_name(), "end of input");
    assert_eq!(TokenKind::Unknown.display_name(), "unknown token");
}

#[test]
fn display_name_returns_non_empty_for_all_variants() {
    for kind in all_kinds() {
        let name = kind.display_name();
        assert!(!name.is_empty(), "display_name() returned empty for {kind:?}");
    }
}

#[test]
fn display_name_keyword_variants_are_quoted() {
    // All keyword display names should be wrapped in single quotes
    let keywords = [
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
    ];
    for kind in &keywords {
        let name = kind.display_name();
        assert!(
            name.starts_with('\'') && name.ends_with('\''),
            "keyword display_name should be quoted: {kind:?} -> {name}"
        );
    }
}

#[test]
fn display_name_delimiter_variants_are_quoted() {
    let delimiters = [
        TokenKind::LeftParen,
        TokenKind::RightParen,
        TokenKind::LeftBrace,
        TokenKind::RightBrace,
        TokenKind::LeftBracket,
        TokenKind::RightBracket,
        TokenKind::Semicolon,
        TokenKind::Comma,
    ];
    for kind in &delimiters {
        let name = kind.display_name();
        assert!(
            name.starts_with('\'') && name.ends_with('\''),
            "delimiter display_name should be quoted: {kind:?} -> {name}"
        );
    }
}

#[test]
fn display_name_operator_variants_are_quoted() {
    let operators = [
        TokenKind::Assign,
        TokenKind::Plus,
        TokenKind::Minus,
        TokenKind::Star,
        TokenKind::Slash,
        TokenKind::Percent,
        TokenKind::Power,
        TokenKind::Arrow,
        TokenKind::FatArrow,
        TokenKind::Dot,
        TokenKind::Range,
        TokenKind::Ellipsis,
        TokenKind::And,
        TokenKind::Or,
        TokenKind::Not,
        TokenKind::DefinedOr,
        TokenKind::Increment,
        TokenKind::Decrement,
        TokenKind::DoubleColon,
        TokenKind::Question,
        TokenKind::Colon,
        TokenKind::Backslash,
    ];
    for kind in &operators {
        let name = kind.display_name();
        assert!(
            name.starts_with('\'') && name.ends_with('\''),
            "operator display_name should be quoted: {kind:?} -> {name}"
        );
    }
}

#[test]
fn display_name_literal_variants_are_unquoted() {
    // Literal display names describe the kind, not the syntax — no quotes
    let literals = [
        TokenKind::Number,
        TokenKind::String,
        TokenKind::Regex,
        TokenKind::Substitution,
        TokenKind::Transliteration,
        TokenKind::QuoteSingle,
        TokenKind::QuoteDouble,
        TokenKind::QuoteWords,
        TokenKind::QuoteCommand,
        TokenKind::HeredocStart,
        TokenKind::HeredocBody,
        TokenKind::FormatBody,
        TokenKind::DataBody,
        TokenKind::VString,
        TokenKind::UnknownRest,
        TokenKind::HeredocDepthLimit,
    ];
    for kind in &literals {
        let name = kind.display_name();
        assert!(
            !name.starts_with('\''),
            "literal display_name should not start with quote: {kind:?} -> {name}"
        );
    }
}

// ===========================================================================
// Classification: every variant belongs to exactly one category
// ===========================================================================

#[test]
fn every_variant_has_exactly_one_category() {
    for kind in all_kinds() {
        let categories = [
            is_keyword(kind),
            is_operator(kind),
            is_delimiter(kind),
            is_literal(kind),
            is_identifier_or_sigil(kind),
            is_special(kind),
        ];
        let count = categories.iter().filter(|&&b| b).count();
        assert_eq!(count, 1, "{kind:?} belongs to {count} categories (expected exactly 1)");
    }
}

#[test]
fn category_method_matches_helpers() {
    for kind in all_kinds() {
        let category = kind.category();
        assert_eq!(is_keyword(kind), matches!(category, TokenCategory::Keyword));
        assert_eq!(is_operator(kind), matches!(category, TokenCategory::Operator));
        assert_eq!(is_delimiter(kind), matches!(category, TokenCategory::Delimiter));
        assert_eq!(is_literal(kind), matches!(category, TokenCategory::Literal));
        assert_eq!(is_special(kind), matches!(category, TokenCategory::Special));
    }
}

#[test]
fn keyword_classification_count() {
    let count = all_kinds().iter().filter(|k| is_keyword(**k)).count();
    assert_eq!(count, 41, "expected 41 keyword variants");
}

#[test]
fn operator_classification_count() {
    let count = all_kinds().iter().filter(|k| is_operator(**k)).count();
    assert_eq!(count, 58, "expected 58 operator variants");
}

#[test]
fn delimiter_classification_count() {
    let count = all_kinds().iter().filter(|k| is_delimiter(**k)).count();
    assert_eq!(count, 8, "expected 8 delimiter variants");
}

#[test]
fn literal_classification_count() {
    let count = all_kinds().iter().filter(|k| is_literal(**k)).count();
    assert_eq!(count, 17, "expected 17 literal variants");
}

#[test]
fn identifier_sigil_classification_count() {
    let count = all_kinds().iter().filter(|k| is_identifier_or_sigil(**k)).count();
    assert_eq!(count, 6, "expected 6 identifier/sigil variants");
}

#[test]
fn special_classification_count() {
    let count = all_kinds().iter().filter(|k| is_special(**k)).count();
    assert_eq!(count, 2, "expected 2 special variants");
}

#[test]
fn total_variant_count() {
    // 41 keywords + 58 operators + 8 delimiters + 17 literals + 6 identifier-like + 2 special = 132
    assert_eq!(all_kinds().len(), 132, "expected 132 total TokenKind variants");
}

// ===========================================================================
// Classification: specific membership checks
// ===========================================================================

#[test]
fn field_is_keyword() {
    assert!(is_keyword(TokenKind::Field));
    assert!(!is_operator(TokenKind::Field));
    assert!(!is_delimiter(TokenKind::Field));
    assert!(!is_literal(TokenKind::Field));
    assert!(!is_identifier_or_sigil(TokenKind::Field));
    assert!(!is_special(TokenKind::Field));
}

#[test]
fn goto_is_keyword() {
    assert!(is_keyword(TokenKind::Goto));
}

#[test]
fn defer_is_keyword() {
    assert!(is_keyword(TokenKind::Defer));
}

#[test]
fn eof_is_special_not_keyword() {
    assert!(is_special(TokenKind::Eof));
    assert!(!is_keyword(TokenKind::Eof));
}

#[test]
fn unknown_is_special_not_literal() {
    assert!(is_special(TokenKind::Unknown));
    assert!(!is_literal(TokenKind::Unknown));
}

#[test]
fn identifier_is_not_keyword() {
    assert!(is_identifier_or_sigil(TokenKind::Identifier));
    assert!(!is_keyword(TokenKind::Identifier));
}

#[test]
fn semicolon_is_delimiter_not_operator() {
    assert!(is_delimiter(TokenKind::Semicolon));
    assert!(!is_operator(TokenKind::Semicolon));
}

#[test]
fn comma_is_delimiter_not_operator() {
    assert!(is_delimiter(TokenKind::Comma));
    assert!(!is_operator(TokenKind::Comma));
}

#[test]
fn data_marker_is_literal() {
    assert!(is_literal(TokenKind::DataMarker));
}

#[test]
fn heredoc_depth_limit_is_literal() {
    assert!(is_literal(TokenKind::HeredocDepthLimit));
}

#[test]
fn unknown_rest_is_literal() {
    assert!(is_literal(TokenKind::UnknownRest));
}

#[test]
fn sigils_classified_correctly() {
    let sigils = [
        TokenKind::ScalarSigil,
        TokenKind::ArraySigil,
        TokenKind::HashSigil,
        TokenKind::SubSigil,
        TokenKind::GlobSigil,
    ];
    for sigil in &sigils {
        assert!(is_identifier_or_sigil(*sigil), "{sigil:?} should be identifier_or_sigil");
        assert!(!is_operator(*sigil), "{sigil:?} should not be classified as operator");
    }
}

// ===========================================================================
// Classification: keyword subcategories
// ===========================================================================

#[test]
fn declaration_keywords() {
    let decl = [TokenKind::My, TokenKind::Our, TokenKind::Local, TokenKind::State];
    for kind in &decl {
        assert!(is_keyword(*kind), "{kind:?} should be a keyword");
    }
}

#[test]
fn loop_keywords() {
    let loops = [TokenKind::While, TokenKind::Until, TokenKind::For, TokenKind::Foreach];
    for kind in &loops {
        assert!(is_keyword(*kind), "{kind:?} should be a keyword");
    }
}

#[test]
fn loop_control_keywords() {
    let controls = [TokenKind::Next, TokenKind::Last, TokenKind::Redo];
    for kind in &controls {
        assert!(is_keyword(*kind), "{kind:?} should be a keyword");
    }
}

#[test]
fn exception_handling_keywords() {
    let exc = [TokenKind::Try, TokenKind::Catch, TokenKind::Finally];
    for kind in &exc {
        assert!(is_keyword(*kind), "{kind:?} should be a keyword");
    }
}

#[test]
fn perl_538_oop_keywords() {
    let oop = [TokenKind::Class, TokenKind::Method, TokenKind::Field];
    for kind in &oop {
        assert!(is_keyword(*kind), "{kind:?} should be a keyword");
    }
}

#[test]
fn phase_block_keywords() {
    let phases =
        [TokenKind::Begin, TokenKind::End, TokenKind::Check, TokenKind::Init, TokenKind::Unitcheck];
    for kind in &phases {
        assert!(is_keyword(*kind), "{kind:?} should be a keyword");
    }
}

// ===========================================================================
// Classification: operator subcategories
// ===========================================================================

#[test]
fn comparison_operators() {
    let comps = [
        TokenKind::Equal,
        TokenKind::NotEqual,
        TokenKind::Less,
        TokenKind::Greater,
        TokenKind::LessEqual,
        TokenKind::GreaterEqual,
        TokenKind::Spaceship,
        TokenKind::StringCompare,
    ];
    for kind in &comps {
        assert!(is_operator(*kind), "{kind:?} should be an operator");
    }
}

#[test]
fn assignment_operators() {
    let assigns = [
        TokenKind::Assign,
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
    for kind in &assigns {
        assert!(is_operator(*kind), "{kind:?} should be an operator");
    }
    assert_eq!(assigns.len(), 16, "expected 16 assignment operators");
}

#[test]
fn logical_operators() {
    let logicals = [
        TokenKind::And,
        TokenKind::Or,
        TokenKind::Not,
        TokenKind::DefinedOr,
        TokenKind::WordAnd,
        TokenKind::WordOr,
        TokenKind::WordNot,
        TokenKind::WordXor,
    ];
    for kind in &logicals {
        assert!(is_operator(*kind), "{kind:?} should be an operator");
    }
}

#[test]
fn bitwise_operators() {
    let bitwise = [
        TokenKind::BitwiseAnd,
        TokenKind::BitwiseOr,
        TokenKind::BitwiseXor,
        TokenKind::BitwiseNot,
        TokenKind::LeftShift,
        TokenKind::RightShift,
    ];
    for kind in &bitwise {
        assert!(is_operator(*kind), "{kind:?} should be an operator");
    }
}

// ===========================================================================
// Source location tracking
// ===========================================================================

#[test]
fn token_span_length() {
    let tok = Token::new(TokenKind::Identifier, "hello", 10, 15);
    assert_eq!(tok.end - tok.start, 5);
    assert_eq!(tok.end - tok.start, tok.text.len());
}

#[test]
fn token_span_zero_width_at_position() {
    // EOF tokens typically have zero-width spans
    let tok = Token::new(TokenKind::Eof, "", 42, 42);
    assert_eq!(tok.start, tok.end);
    assert_eq!(tok.end - tok.start, 0);
}

#[test]
fn token_span_preserves_exact_offsets() {
    let tok = Token::new(TokenKind::String, "\"hello world\"", 100, 113);
    assert_eq!(tok.start, 100);
    assert_eq!(tok.end, 113);
    assert_eq!(tok.end - tok.start, 13);
}

#[test]
fn token_sequence_spans_are_monotonically_increasing() {
    // my $x = 42;
    let tokens = [
        Token::new(TokenKind::My, "my", 0, 2),
        Token::new(TokenKind::ScalarSigil, "$", 3, 4),
        Token::new(TokenKind::Identifier, "x", 4, 5),
        Token::new(TokenKind::Assign, "=", 6, 7),
        Token::new(TokenKind::Number, "42", 8, 10),
        Token::new(TokenKind::Semicolon, ";", 10, 11),
    ];
    for window in tokens.windows(2) {
        assert!(
            window[0].start <= window[1].start,
            "token start positions should be monotonically increasing: {} > {}",
            window[0].start,
            window[1].start
        );
        assert!(
            window[0].end <= window[1].end,
            "token end positions should be monotonically increasing: {} > {}",
            window[0].end,
            window[1].end
        );
    }
}

#[test]
fn token_sequence_non_overlapping_spans() {
    let tokens = [
        Token::new(TokenKind::Sub, "sub", 0, 3),
        Token::new(TokenKind::Identifier, "greet", 4, 9),
        Token::new(TokenKind::LeftParen, "(", 9, 10),
        Token::new(TokenKind::RightParen, ")", 10, 11),
        Token::new(TokenKind::LeftBrace, "{", 12, 13),
        Token::new(TokenKind::RightBrace, "}", 14, 15),
    ];
    for window in tokens.windows(2) {
        assert!(
            window[0].end <= window[1].start,
            "tokens should not overlap: {:?} ends at {} but {:?} starts at {}",
            window[0].kind,
            window[0].end,
            window[1].kind,
            window[1].start,
        );
    }
}

#[test]
fn token_span_with_unicode_byte_offsets() {
    // Unicode: byte offsets, not char offsets
    let text = "日本語"; // 9 bytes (3 chars * 3 bytes each)
    let tok = Token::new(TokenKind::String, text, 0, 9);
    assert_eq!(tok.end - tok.start, 9);
    assert_eq!(tok.text.len(), 9);
    assert_eq!(tok.text.chars().count(), 3);
}

#[test]
fn token_start_end_can_represent_end_of_large_file() {
    let large_offset = 10_000_000;
    let tok = Token::new(TokenKind::Eof, "", large_offset, large_offset);
    assert_eq!(tok.start, large_offset);
    assert_eq!(tok.end, large_offset);
}

// ===========================================================================
// Token equality: deeper scenarios
// ===========================================================================

#[test]
fn token_equality_ignores_arc_identity() {
    // Two tokens with same fields but different Arc allocations are equal
    let a = Token::new(TokenKind::Identifier, "foo", 0, 3);
    let b = Token::new(TokenKind::Identifier, "foo", 0, 3);
    assert_eq!(a, b);
    assert!(!Arc::ptr_eq(&a.text, &b.text)); // different allocations
}

#[test]
fn token_equality_considers_all_fields() {
    let base = Token::new(TokenKind::Number, "42", 10, 12);

    // Different kind
    let diff_kind = Token::new(TokenKind::String, "42", 10, 12);
    assert_ne!(base, diff_kind);

    // Different text
    let diff_text = Token::new(TokenKind::Number, "43", 10, 12);
    assert_ne!(base, diff_text);

    // Different start
    let diff_start = Token::new(TokenKind::Number, "42", 11, 12);
    assert_ne!(base, diff_start);

    // Different end
    let diff_end = Token::new(TokenKind::Number, "42", 10, 13);
    assert_ne!(base, diff_end);
}

#[test]
fn token_equality_with_shared_arc() {
    let shared: Arc<str> = Arc::from("shared");
    let a = Token { kind: TokenKind::Identifier, text: shared.clone(), start: 0, end: 6 };
    let b = Token { kind: TokenKind::Identifier, text: shared, start: 0, end: 6 };
    assert_eq!(a, b);
}

// ===========================================================================
// Field variant coverage (previously missing from test files)
// ===========================================================================

#[test]
fn field_token_construction() {
    let tok = Token::new(TokenKind::Field, "field", 0, 5);
    assert_eq!(tok.kind, TokenKind::Field);
    assert_eq!(&*tok.text, "field");
    assert_eq!(tok.start, 0);
    assert_eq!(tok.end, 5);
}

#[test]
fn field_display_name() {
    assert_eq!(TokenKind::Field.display_name(), "'field'");
}

#[test]
fn field_debug_format() {
    assert_eq!(format!("{:?}", TokenKind::Field), "Field");
}

#[test]
fn field_is_copy() {
    let a = TokenKind::Field;
    let b = a; // Copy
    assert_eq!(a, b);
}

#[test]
fn field_in_class_declaration_sequence() {
    // class Foo { field $name; }
    let tokens = [
        Token::new(TokenKind::Class, "class", 0, 5),
        Token::new(TokenKind::Identifier, "Foo", 6, 9),
        Token::new(TokenKind::LeftBrace, "{", 10, 11),
        Token::new(TokenKind::Field, "field", 12, 17),
        Token::new(TokenKind::ScalarSigil, "$", 18, 19),
        Token::new(TokenKind::Identifier, "name", 19, 23),
        Token::new(TokenKind::Semicolon, ";", 23, 24),
        Token::new(TokenKind::RightBrace, "}", 25, 26),
    ];
    assert_eq!(tokens[0].kind, TokenKind::Class);
    assert_eq!(tokens[3].kind, TokenKind::Field);
    assert_eq!(tokens[4].kind, TokenKind::ScalarSigil);
}

#[test]
fn goto_token_construction() {
    let tok = Token::new(TokenKind::Goto, "goto", 0, 4);
    assert_eq!(tok.kind, TokenKind::Goto);
    assert_eq!(&*tok.text, "goto");
}

#[test]
fn goto_display_name() {
    assert_eq!(TokenKind::Goto.display_name(), "'goto'");
}

#[test]
fn defer_display_name() {
    assert_eq!(TokenKind::Defer.display_name(), "'defer'");
}

// ===========================================================================
// TokenKind: Copy semantics verified across all variants
// ===========================================================================

#[test]
fn all_variants_are_copy() {
    for kind in all_kinds() {
        let copied = kind;
        assert_eq!(kind, copied, "{kind:?} should be Copy");
    }
}

// ===========================================================================
// display_name stability: same variant always returns the same pointer
// ===========================================================================

#[test]
fn display_name_returns_static_str() {
    // Calling display_name twice on the same variant yields the same &'static str
    for kind in all_kinds() {
        let first = kind.display_name();
        let second = kind.display_name();
        assert_eq!(
            first as *const str, second as *const str,
            "display_name should return the same &'static str for {kind:?}"
        );
    }
}
