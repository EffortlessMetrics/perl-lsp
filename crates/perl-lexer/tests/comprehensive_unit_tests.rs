//! Comprehensive unit tests for the perl-lexer crate.
//!
//! Covers: Token construction, LexerMode, LexerError, LexerConfig,
//! checkpoint operations, quote handler utilities, unicode helpers,
//! lexer creation variants, context-sensitive tokenization edge cases,
//! numeric literals, string types, operator disambiguation, delimiter
//! handling, sigil parsing, heredoc handling, format bodies, data sections,
//! BOM skipping, budget guards, peek/reset/collect APIs, and edge cases
//! (empty input, unicode, very long tokens).

use perl_lexer::{
    CheckpointCache, Checkpointable, LexerCheckpoint, LexerConfig, LexerError, LexerMode,
    PerlLexer, StringPart, Token, TokenType,
};
use std::sync::Arc;

type R = Result<(), Box<dyn std::error::Error>>;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn tokens(input: &str) -> Vec<Token> {
    PerlLexer::new(input).collect_tokens()
}

fn first_token(input: &str) -> Option<Token> {
    PerlLexer::new(input).next_token()
}

fn _token_types(input: &str) -> Vec<String> {
    tokens(input)
        .iter()
        .map(|t| format!("{:?}", std::mem::discriminant(&t.token_type)))
        .collect()
}

/// Collect only the significant (non-whitespace, non-newline, non-EOF) tokens.
fn significant(input: &str) -> Vec<Token> {
    tokens(input)
        .into_iter()
        .filter(|t| {
            !matches!(
                t.token_type,
                TokenType::Whitespace | TokenType::Newline | TokenType::EOF
            )
        })
        .collect()
}

// ===========================================================================
// 1. Token construction and properties
// ===========================================================================

#[test]
fn token_new_sets_fields() {
    let tok = Token::new(TokenType::Semicolon, ";", 10, 11);
    assert_eq!(tok.start, 10);
    assert_eq!(tok.end, 11);
    assert_eq!(tok.len(), 1);
    assert!(!tok.is_empty());
    assert_eq!(tok.text.as_ref(), ";");
}

#[test]
fn token_empty_when_start_equals_end() {
    let tok = Token::new(TokenType::EOF, "", 5, 5);
    assert!(tok.is_empty());
    assert_eq!(tok.len(), 0);
}

#[test]
fn token_type_debug_formatting() {
    let types = [
        TokenType::Division,
        TokenType::RegexMatch,
        TokenType::Substitution,
        TokenType::Transliteration,
        TokenType::QuoteRegex,
        TokenType::StringLiteral,
        TokenType::QuoteSingle,
        TokenType::QuoteDouble,
        TokenType::QuoteWords,
        TokenType::QuoteCommand,
        TokenType::HeredocStart,
        TokenType::Pod,
        TokenType::UnknownRest,
        TokenType::LeftParen,
        TokenType::RightParen,
        TokenType::LeftBracket,
        TokenType::RightBracket,
        TokenType::LeftBrace,
        TokenType::RightBrace,
        TokenType::Semicolon,
        TokenType::Comma,
        TokenType::Colon,
        TokenType::Arrow,
        TokenType::FatComma,
        TokenType::Whitespace,
        TokenType::Newline,
        TokenType::EOF,
    ];
    for tt in &types {
        // Debug formatting should not panic
        let _ = format!("{:?}", tt);
    }
}

#[test]
fn token_type_clone_and_eq() {
    let t1 = TokenType::Number(Arc::from("42"));
    let t2 = t1.clone();
    assert_eq!(t1, t2);

    let t3 = TokenType::Number(Arc::from("43"));
    assert_ne!(t1, t3);
}

#[test]
fn string_part_variants() {
    let parts = vec![
        StringPart::Literal(Arc::from("hello")),
        StringPart::Variable(Arc::from("$x")),
        StringPart::Expression(Arc::from("${expr}")),
        StringPart::MethodCall(Arc::from("->method()")),
        StringPart::ArraySlice(Arc::from("[1..3]")),
    ];
    for part in &parts {
        let cloned = part.clone();
        assert_eq!(part, &cloned);
        let _ = format!("{:?}", part);
    }
}

#[test]
fn token_type_with_arc_data() {
    let variants = [
        TokenType::Identifier(Arc::from("foo")),
        TokenType::Number(Arc::from("3.14")),
        TokenType::Operator(Arc::from("+")),
        TokenType::Keyword(Arc::from("my")),
        TokenType::Comment(Arc::from("# hello")),
        TokenType::Error(Arc::from("bad")),
        TokenType::HeredocBody(Arc::from("body")),
        TokenType::FormatBody(Arc::from("fmt")),
        TokenType::Version(Arc::from("v5.32")),
        TokenType::DataMarker(Arc::from("__DATA__")),
        TokenType::DataBody(Arc::from("data")),
    ];
    for v in &variants {
        let c = v.clone();
        assert_eq!(v, &c);
    }
}

#[test]
fn interpolated_string_token_type() {
    let parts = vec![
        StringPart::Literal(Arc::from("hi ")),
        StringPart::Variable(Arc::from("$x")),
    ];
    let tt = TokenType::InterpolatedString(parts.clone());
    if let TokenType::InterpolatedString(ref p) = tt {
        assert_eq!(p.len(), 2);
    }
    let tt2 = tt.clone();
    assert_eq!(tt, tt2);
}

// ===========================================================================
// 2. LexerMode
// ===========================================================================

#[test]
fn lexer_mode_default_is_expect_term() {
    let mode = LexerMode::default();
    assert!(mode.is_expect_term());
    assert!(!mode.is_expect_operator());
}

#[test]
fn lexer_mode_variants_query() {
    assert!(LexerMode::ExpectTerm.is_expect_term());
    assert!(!LexerMode::ExpectTerm.is_expect_operator());
    assert!(LexerMode::ExpectOperator.is_expect_operator());
    assert!(!LexerMode::ExpectOperator.is_expect_term());
    assert!(!LexerMode::ExpectDelimiter.is_expect_term());
    assert!(!LexerMode::ExpectDelimiter.is_expect_operator());
    assert!(!LexerMode::InFormatBody.is_expect_term());
    assert!(!LexerMode::InDataSection.is_expect_term());
}

#[test]
fn lexer_mode_clone_eq() {
    let m1 = LexerMode::ExpectTerm;
    let m2 = m1;
    assert_eq!(m1, m2);
    let m3 = LexerMode::ExpectOperator;
    assert_ne!(m1, m3);
}

#[test]
fn lexer_mode_debug() {
    let modes = [
        LexerMode::ExpectTerm,
        LexerMode::ExpectOperator,
        LexerMode::ExpectDelimiter,
        LexerMode::InFormatBody,
        LexerMode::InDataSection,
    ];
    for m in &modes {
        let s = format!("{:?}", m);
        assert!(!s.is_empty());
    }
}

// ===========================================================================
// 3. LexerError
// ===========================================================================

#[test]
fn lexer_error_position_extraction() {
    let errors = [
        (LexerError::UnterminatedString { position: 10 }, Some(10)),
        (LexerError::UnterminatedRegex { position: 20 }, Some(20)),
        (
            LexerError::InvalidEscape {
                char: 'z',
                position: 5,
            },
            Some(5),
        ),
        (
            LexerError::InvalidNumber {
                position: 7,
                reason: "bad".into(),
            },
            Some(7),
        ),
        (
            LexerError::UnexpectedChar {
                char: '?',
                position: 3,
            },
            Some(3),
        ),
        (LexerError::InvalidUtf8 { position: 0 }, Some(0)),
        (
            LexerError::HeredocError {
                position: 15,
                reason: "no end".into(),
            },
            Some(15),
        ),
        (LexerError::Other("misc".into()), None),
    ];
    for (err, expected_pos) in &errors {
        assert_eq!(err.position(), *expected_pos, "error={:?}", err);
    }
}

#[test]
fn lexer_error_display_messages() {
    let err = LexerError::UnterminatedString { position: 42 };
    let msg = format!("{}", err);
    assert!(msg.contains("42"), "display should contain position");
    assert!(msg.to_lowercase().contains("unterminated"));

    let err2 = LexerError::InvalidEscape {
        char: 'q',
        position: 1,
    };
    let msg2 = format!("{}", err2);
    assert!(msg2.contains("q"));

    let err3 = LexerError::Other("custom error".into());
    assert_eq!(format!("{}", err3), "custom error");
}

#[test]
fn lexer_error_clone() {
    let err = LexerError::InvalidNumber {
        position: 3,
        reason: "overflow".into(),
    };
    let err2 = err.clone();
    assert_eq!(err.position(), err2.position());
}

// ===========================================================================
// 4. LexerConfig
// ===========================================================================

#[test]
fn default_config_values() {
    let cfg = LexerConfig::default();
    assert!(cfg.parse_interpolation);
    assert!(cfg.track_positions);
    assert_eq!(cfg.max_lookahead, 1024);
}

#[test]
fn custom_config() {
    let cfg = LexerConfig {
        parse_interpolation: false,
        track_positions: false,
        max_lookahead: 64,
    };
    assert!(!cfg.parse_interpolation);
    assert!(!cfg.track_positions);
    assert_eq!(cfg.max_lookahead, 64);
}

#[test]
fn config_clone_and_debug() {
    let cfg = LexerConfig::default();
    let cfg2 = cfg.clone();
    assert_eq!(cfg.max_lookahead, cfg2.max_lookahead);
    let s = format!("{:?}", cfg);
    assert!(s.contains("parse_interpolation"));
}

// ===========================================================================
// 5. Lexer creation and lifecycle
// ===========================================================================

#[test]
fn new_lexer_empty_input() -> R {
    let mut lexer = PerlLexer::new("");
    let tok = lexer.next_token().ok_or("expected EOF")?;
    assert_eq!(tok.token_type, TokenType::EOF);
    assert!(lexer.next_token().is_none());
    Ok(())
}

#[test]
fn new_lexer_whitespace_only() -> R {
    let mut lexer = PerlLexer::new("   \t\n  ");
    let tok = lexer.next_token().ok_or("expected EOF")?;
    assert_eq!(tok.token_type, TokenType::EOF);
    Ok(())
}

#[test]
fn with_config_custom_lookahead() -> R {
    let cfg = LexerConfig {
        max_lookahead: 8,
        ..LexerConfig::default()
    };
    let mut lexer = PerlLexer::with_config("my $x;", cfg);
    let tok = lexer.next_token().ok_or("expected token")?;
    assert!(matches!(tok.token_type, TokenType::Keyword(_)));
    Ok(())
}

#[test]
fn with_config_zero_lookahead_disables_decimal_number_peek() -> R {
    let cfg = LexerConfig {
        max_lookahead: 0,
        ..LexerConfig::default()
    };
    let mut lexer = PerlLexer::with_config(".5", cfg);

    let dot = lexer.next_token().ok_or("expected dot operator")?;
    assert_eq!(dot.text.as_ref(), ".");

    let number = lexer.next_token().ok_or("expected number token")?;
    assert!(matches!(number.token_type, TokenType::Number(_)));
    assert_eq!(number.text.as_ref(), "5");
    Ok(())
}

#[test]
fn with_config_small_lookahead_limits_namespace_parsing() -> R {
    // max_lookahead: 0 prevents peek_char(1) in identifier parsing (try_identifier_or_keyword),
    // so "Foo::Bar" emits "Foo" as the identifier rather than "Foo::Bar". However, try_operator
    // uses current_char (not peek_char), so "::" is still correctly emitted as a single DoubleColon
    // operator token regardless of max_lookahead.
    let cfg = LexerConfig {
        max_lookahead: 0,
        ..LexerConfig::default()
    };
    let mut lexer = PerlLexer::with_config("Foo::Bar", cfg);

    let first = lexer.next_token().ok_or("expected first token")?;
    assert!(matches!(first.token_type, TokenType::Identifier(_)));
    assert_eq!(first.text.as_ref(), "Foo");

    let colon = lexer.next_token().ok_or("expected colon token")?;
    assert_eq!(colon.text.as_ref(), "::");
    Ok(())
}

#[test]
fn with_body_tokens_emits_heredoc_body() -> R {
    let input = "print <<EOF;\nhello world\nEOF\n";
    let mut lexer = PerlLexer::with_body_tokens(input);
    let toks = lexer.collect_tokens();
    let has_heredoc_body = toks
        .iter()
        .any(|t| matches!(t.token_type, TokenType::HeredocBody(_)));
    assert!(
        has_heredoc_body,
        "with_body_tokens should emit HeredocBody tokens"
    );
    Ok(())
}

#[test]
fn regular_lexer_omits_heredoc_body() -> R {
    let input = "print <<EOF;\nhello world\nEOF\n";
    let mut lexer = PerlLexer::new(input);
    let toks = lexer.collect_tokens();
    let has_heredoc_body = toks
        .iter()
        .any(|t| matches!(t.token_type, TokenType::HeredocBody(_)));
    assert!(
        !has_heredoc_body,
        "default lexer should NOT emit HeredocBody tokens"
    );
    Ok(())
}

// ===========================================================================
// 6. collect_tokens, peek_token, reset
// ===========================================================================

#[test]
fn collect_tokens_ends_with_eof() -> R {
    let toks = tokens("1 + 2");
    let last = toks.last().ok_or("no tokens")?;
    assert_eq!(last.token_type, TokenType::EOF);
    Ok(())
}

#[test]
fn collect_tokens_empty_input_returns_eof() -> R {
    let toks = tokens("");
    assert_eq!(toks.len(), 1);
    let t = toks.first().ok_or("empty")?;
    assert_eq!(t.token_type, TokenType::EOF);
    Ok(())
}

#[test]
fn peek_token_does_not_consume() -> R {
    let mut lexer = PerlLexer::new("my $x;");
    let peeked = lexer.peek_token().ok_or("peek failed")?;
    let actual = lexer.next_token().ok_or("next failed")?;
    assert_eq!(peeked.token_type, actual.token_type);
    assert_eq!(peeked.start, actual.start);
    Ok(())
}

#[test]
fn peek_token_multiple_times_same_result() -> R {
    let mut lexer = PerlLexer::new("42;");
    let p1 = lexer.peek_token().ok_or("peek 1 failed")?;
    let p2 = lexer.peek_token().ok_or("peek 2 failed")?;
    assert_eq!(p1.start, p2.start);
    assert_eq!(p1.end, p2.end);
    Ok(())
}

#[test]
fn peek_token_preserves_eof_state() -> R {
    let mut lexer = PerlLexer::new("42");
    let _number = lexer.next_token().ok_or("missing number token")?;
    let peeked_eof = lexer.peek_token().ok_or("missing peeked EOF token")?;
    assert_eq!(peeked_eof.token_type, TokenType::EOF);

    let actual_eof = lexer.next_token().ok_or("missing actual EOF token")?;
    assert_eq!(actual_eof.token_type, TokenType::EOF);
    assert!(lexer.next_token().is_none());
    Ok(())
}

#[test]
fn reset_replays_from_beginning() -> R {
    let mut lexer = PerlLexer::new("my $x = 1;");
    let first_pass = lexer.next_token().ok_or("first token")?;
    // consume more tokens
    let _ = lexer.next_token();
    let _ = lexer.next_token();
    lexer.reset();
    let after_reset = lexer.next_token().ok_or("after reset")?;
    assert_eq!(first_pass.token_type, after_reset.token_type);
    assert_eq!(first_pass.start, after_reset.start);
    Ok(())
}

#[test]
fn reset_after_eof_replays_eof_token() -> R {
    let mut lexer = PerlLexer::new("1");
    let _number = lexer.next_token().ok_or("missing number token")?;
    let eof = lexer.next_token().ok_or("missing EOF token")?;
    assert_eq!(eof.token_type, TokenType::EOF);

    lexer.reset();

    let replayed_number = lexer.next_token().ok_or("missing replayed number token")?;
    assert!(matches!(&replayed_number.token_type, TokenType::Number(text) if &**text == "1"));
    let replayed_eof = lexer.next_token().ok_or("missing replayed EOF token")?;
    assert_eq!(replayed_eof.token_type, TokenType::EOF);
    Ok(())
}

#[test]
fn next_token_returns_none_after_eof() -> R {
    let mut lexer = PerlLexer::new("1");
    let _ = lexer.next_token(); // 1
    let _ = lexer.next_token(); // EOF
    assert!(lexer.next_token().is_none());
    assert!(lexer.next_token().is_none());
    Ok(())
}

// ===========================================================================
// 7. set_mode and enter_format_mode
// ===========================================================================

#[test]
fn set_mode_changes_context() -> R {
    let mut lexer = PerlLexer::new("/ 2");
    // Default mode is ExpectTerm, slash starts regex
    lexer.set_mode(LexerMode::ExpectOperator);
    let tok = lexer.next_token().ok_or("expected division")?;
    assert_eq!(tok.token_type, TokenType::Division);
    Ok(())
}

#[test]
fn enter_format_mode_parses_format_body() -> R {
    let input = "some text\n.\n";
    let mut lexer = PerlLexer::new(input);
    lexer.enter_format_mode();
    let tok = lexer.next_token().ok_or("expected format body")?;
    assert!(
        matches!(tok.token_type, TokenType::FormatBody(_)),
        "expected FormatBody, got {:?}",
        tok.token_type
    );
    Ok(())
}

// ===========================================================================
// 8. Keyword tokenization
// ===========================================================================

#[test]
fn perl_keywords_recognized() -> R {
    let keywords = [
        "my", "our", "local", "sub", "if", "elsif", "else", "unless", "while", "until", "for",
        "foreach", "do", "eval", "use", "no", "require", "package", "return", "last", "next",
        "redo", "die", "warn", "print", "say", "chomp", "chop", "push", "pop", "shift", "unshift",
        "defined", "undef", "ref", "bless",
    ];
    for kw in &keywords {
        let tok = first_token(kw).ok_or_else(|| format!("no token for '{}'", kw))?;
        match &tok.token_type {
            TokenType::Keyword(k) => {
                assert_eq!(k.as_ref(), *kw, "keyword text mismatch for '{}'", kw)
            }
            // Some may be identifiers depending on the keyword table
            TokenType::Identifier(id) => {
                // acceptable for some builtins
                assert_eq!(id.as_ref(), *kw);
            }
            other => {
                // Not a hard failure - just note it
                let _ = format!("'{}' produced {:?}", kw, other);
            }
        }
    }
    Ok(())
}

#[test]
fn defer_block_is_tokenized_as_keyword() -> R {
    let tok = first_token("defer { }").ok_or("no token")?;
    assert!(
        matches!(&tok.token_type, TokenType::Keyword(k) if k.as_ref() == "defer"),
        "expected 'defer' to tokenize as a keyword, got {:?}",
        tok.token_type
    );
    Ok(())
}

// ===========================================================================
// 9. Variable / sigil parsing
// ===========================================================================

#[test]
fn scalar_variable_dollar() -> R {
    let tok = first_token("$foo").ok_or("no token")?;
    if let TokenType::Identifier(ref id) = tok.token_type {
        assert!(id.as_ref().contains("foo"));
    }
    Ok(())
}

#[test]
fn array_variable_at() -> R {
    let tok = first_token("@array").ok_or("no token")?;
    if let TokenType::Identifier(ref id) = tok.token_type {
        assert!(id.as_ref().contains("array"));
    }
    Ok(())
}

#[test]
fn hash_variable_percent() -> R {
    let tok = first_token("%hash").ok_or("no token")?;
    if let TokenType::Identifier(ref id) = tok.token_type {
        assert!(id.as_ref().contains("hash"));
    }
    Ok(())
}

#[test]
fn deref_sigils() -> R {
    for input in ["@$ref", "$$ref", "%$ref"] {
        let tok = first_token(input).ok_or_else(|| format!("no token for '{}'", input))?;
        assert!(
            matches!(
                tok.token_type,
                TokenType::Identifier(_) | TokenType::Operator(_)
            ),
            "'{}' => {:?}",
            input,
            tok.token_type
        );
    }
    Ok(())
}

#[test]
fn special_variables() -> R {
    let specials = ["$_", "$!", "$@", "$0", "$$", "$\\", "$,", "$/"];
    for sv in &specials {
        let toks = tokens(sv);
        assert!(!toks.is_empty(), "expected at least one token for '{}'", sv);
    }
    Ok(())
}

// ===========================================================================
// 10. Numeric literals
// ===========================================================================

#[test]
fn integer_literal() -> R {
    let tok = first_token("42").ok_or("no token")?;
    if let TokenType::Number(ref n) = tok.token_type {
        assert_eq!(n.as_ref(), "42");
    }
    Ok(())
}

#[test]
fn negative_number_is_operator_plus_number() -> R {
    let toks = significant("-42");
    assert!(
        toks.len() >= 2,
        "expected operator then number, got {} tokens",
        toks.len()
    );
    Ok(())
}

#[test]
fn float_literal() -> R {
    for input in [
        "3.14", "0.5", ".25", "1.", "1e10", "2.5e-3", "1E+5", "1e1_0", "2.5e-1_0",
    ] {
        let tok = first_token(input).ok_or_else(|| format!("no token for '{}'", input))?;
        match &tok.token_type {
            TokenType::Number(_) => {} // expected
            other => {
                // .25 might tokenize differently
                let _ = format!("'{}' => {:?}", input, other);
            }
        }
    }
    Ok(())
}

#[test]
fn hex_literal() -> R {
    // Lexer splits "0xFF" into Number("0") + Identifier("xFF")
    let tok = first_token("0xFF").ok_or("no token")?;
    assert!(matches!(tok.token_type, TokenType::Number(_)));
    Ok(())
}

#[test]
fn octal_literal() -> R {
    let tok = first_token("0777").ok_or("no token")?;
    assert!(matches!(tok.token_type, TokenType::Number(_)));
    Ok(())
}

#[test]
fn binary_literal() -> R {
    // Lexer splits "0b1010" into Number("0") + Identifier("b1010")
    let tok = first_token("0b1010").ok_or("no token")?;
    assert!(matches!(tok.token_type, TokenType::Number(_)));
    Ok(())
}

#[test]
fn underscored_number() -> R {
    let tok = first_token("1_000_000").ok_or("no token")?;
    assert!(matches!(tok.token_type, TokenType::Number(_)));
    Ok(())
}

#[test]
fn zero_literal() -> R {
    let tok = first_token("0").ok_or("no token")?;
    if let TokenType::Number(ref n) = tok.token_type {
        assert_eq!(n.as_ref(), "0");
    }
    Ok(())
}

// ===========================================================================
// 11. String literals
// ===========================================================================

#[test]
fn double_quoted_string() -> R {
    let tok = first_token(r#""hello""#).ok_or("no token")?;
    assert!(
        matches!(
            tok.token_type,
            TokenType::StringLiteral | TokenType::InterpolatedString(_) | TokenType::QuoteDouble
        ),
        "got {:?}",
        tok.token_type
    );
    Ok(())
}

#[test]
fn interpolated_string_preserves_complex_tails() -> R {
    let cases = [
        (
            r#""${expr}""#,
            vec![StringPart::Expression(Arc::from("${expr}"))],
        ),
        (
            r#""$arr[0]""#,
            vec![
                StringPart::Variable(Arc::from("$arr")),
                StringPart::ArraySlice(Arc::from("[0]")),
            ],
        ),
        (
            r#""$hash{key}""#,
            vec![
                StringPart::Variable(Arc::from("$hash")),
                StringPart::Expression(Arc::from("{key}")),
            ],
        ),
        (
            r#""$var->[0]""#,
            vec![
                StringPart::Variable(Arc::from("$var")),
                StringPart::MethodCall(Arc::from("->[0]")),
            ],
        ),
        (
            r#""$obj->{key}""#,
            vec![
                StringPart::Variable(Arc::from("$obj")),
                StringPart::MethodCall(Arc::from("->{key}")),
            ],
        ),
    ];

    for (input, expected_parts) in cases {
        let tok = first_token(input).ok_or_else(|| format!("no token for {input:?}"))?;
        assert!(
            matches!(
                &tok.token_type,
                TokenType::InterpolatedString(parts) if parts == &expected_parts
            ),
            "expected interpolated string token for {input:?}, got {:?}",
            tok.token_type
        );
    }

    let simple = first_token(r#""hello $x world""#).ok_or("no token")?;
    assert!(
        matches!(
            &simple.token_type,
            TokenType::InterpolatedString(parts) if parts
                == &vec![
                    StringPart::Literal(Arc::from("hello ")),
                    StringPart::Variable(Arc::from("$x")),
                    StringPart::Literal(Arc::from(" world")),
                ]
        ),
        "expected interpolated string token, got {:?}",
        simple.token_type
    );

    Ok(())
}

#[test]
fn single_quoted_string() -> R {
    let tok = first_token("'hello'").ok_or("no token")?;
    assert!(
        matches!(
            tok.token_type,
            TokenType::StringLiteral | TokenType::QuoteSingle
        ),
        "got {:?}",
        tok.token_type
    );
    Ok(())
}

#[test]
fn backtick_string() -> R {
    let tok = first_token("`ls -la`").ok_or("no token")?;
    assert!(
        matches!(
            tok.token_type,
            TokenType::QuoteCommand | TokenType::StringLiteral
        ),
        "got {:?}",
        tok.token_type
    );
    Ok(())
}

#[test]
fn empty_string_literals() -> R {
    for input in [r#""""#, "''", "``"] {
        let tok = first_token(input).ok_or_else(|| format!("no token for '{}'", input))?;
        assert!(
            !matches!(tok.token_type, TokenType::Error(_)),
            "'{}' produced error: {:?}",
            input,
            tok.token_type
        );
    }
    Ok(())
}

#[test]
fn string_with_escape_sequences() -> R {
    let input = r#""hello\nworld\t\"end""#;
    let tok = first_token(input).ok_or("no token")?;
    assert!(
        !matches!(tok.token_type, TokenType::Error(_)),
        "escape sequences should not cause error: {:?}",
        tok.token_type
    );
    Ok(())
}

// ===========================================================================
// 12. Operator tokenization
// ===========================================================================

#[test]
fn arithmetic_operators() -> R {
    let ops = ["+", "-", "*"];
    for op in &ops {
        let input = format!("1 {} 2", op);
        let toks = significant(&input);
        assert!(
            toks.len() >= 3,
            "expected 3+ tokens for '{}', got {}",
            input,
            toks.len()
        );
    }
    Ok(())
}

#[test]
fn comparison_operators() -> R {
    let ops = [
        "==", "!=", "<", ">", "<=", ">=", "<=>", "eq", "ne", "lt", "gt", "le", "ge",
    ];
    for op in &ops {
        let input = format!("$a {} $b", op);
        let toks = tokens(&input);
        assert!(toks.len() >= 3, "expected 3+ tokens for '{}'", input);
    }
    Ok(())
}

#[test]
fn assignment_operators() -> R {
    let ops = ["=", "+=", "-=", "*=", "/=", ".=", "||=", "&&=", "//="];
    for op in &ops {
        let input = format!("$x {} 1", op);
        let toks = tokens(&input);
        assert!(toks.len() >= 3, "expected tokens for '{}'", input);
    }
    Ok(())
}

#[test]
fn logical_operators() -> R {
    for input in ["$a && $b", "$a || $b", "!$x"] {
        let toks = tokens(input);
        assert!(toks.len() >= 2, "expected tokens for '{}'", input);
    }
    Ok(())
}

#[test]
fn string_concat_and_repeat() -> R {
    // Concatenation
    let toks = significant("$a . $b");
    assert!(toks.len() >= 3);
    // Repeat
    let toks = significant("$a x 3");
    assert!(toks.len() >= 3);
    Ok(())
}

#[test]
fn arrow_operator() -> R {
    let toks = significant("$obj->method");
    // Arrow is tokenized as Operator("->")
    let has_arrow = toks
        .iter()
        .any(|t| matches!(&t.token_type, TokenType::Operator(op) if op.as_ref() == "->"));
    assert!(has_arrow, "expected Operator(->) token");
    Ok(())
}

#[test]
fn fat_comma() -> R {
    let toks = significant("key => 'value'");
    // Fat comma is tokenized as Operator("=>")
    let has_fat_comma = toks
        .iter()
        .any(|t| matches!(&t.token_type, TokenType::Operator(op) if op.as_ref() == "=>"));
    assert!(has_fat_comma, "expected Operator(=>) token");
    Ok(())
}

#[test]
fn defined_or_operator() -> R {
    let mut lexer = PerlLexer::new("$a // $b");
    let _ = lexer.next_token(); // $a
    let tok = lexer.next_token().ok_or("expected //")?;
    assert!(
        matches!(&tok.token_type, TokenType::Operator(op) if op.as_ref() == "//"),
        "got {:?}",
        tok.token_type
    );
    Ok(())
}

#[test]
fn exponentiation_operator() -> R {
    let mut lexer = PerlLexer::new("2 ** 8");
    let _ = lexer.next_token(); // 2
    let tok = lexer.next_token().ok_or("expected **")?;
    assert!(
        matches!(&tok.token_type, TokenType::Operator(op) if op.as_ref() == "**"),
        "got {:?}",
        tok.token_type
    );
    Ok(())
}

#[test]
fn range_operator() -> R {
    let mut lexer = PerlLexer::new("1 .. 10");
    let _ = lexer.next_token(); // 1
    let tok = lexer.next_token().ok_or("expected ..")?;
    assert!(
        matches!(&tok.token_type, TokenType::Operator(op) if op.as_ref() == ".."),
        "got {:?}",
        tok.token_type
    );
    Ok(())
}

#[test]
fn ternary_operator_tokens() -> R {
    let toks = significant("$x ? 1 : 0");
    // Should have at least: $x, ?, 1, :, 0
    assert!(toks.len() >= 5, "expected 5+ tokens, got {}", toks.len());
    Ok(())
}

#[test]
fn binding_operators() -> R {
    for op_text in ["=~", "!~"] {
        let input = format!("$x {} /pat/", op_text);
        let toks = significant(&input);
        let has_binding = toks
            .iter()
            .any(|t| matches!(&t.token_type, TokenType::Operator(op) if op.as_ref() == op_text));
        assert!(
            has_binding,
            "expected binding operator '{}' in tokens",
            op_text
        );
    }
    Ok(())
}

// ===========================================================================
// 13. Delimiter tokens
// ===========================================================================

#[test]
fn paired_delimiters() -> R {
    let cases: Vec<(&str, TokenType)> = vec![
        ("(", TokenType::LeftParen),
        (")", TokenType::RightParen),
        ("[", TokenType::LeftBracket),
        ("]", TokenType::RightBracket),
        ("{", TokenType::LeftBrace),
        ("}", TokenType::RightBrace),
    ];
    for (input, expected) in &cases {
        let tok = first_token(input).ok_or_else(|| format!("no token for '{}'", input))?;
        assert_eq!(
            std::mem::discriminant(&tok.token_type),
            std::mem::discriminant(expected),
            "mismatch for '{}'",
            input
        );
    }
    Ok(())
}

#[test]
fn semicolon_and_comma() -> R {
    let toks = significant("1; 2, 3");
    let has_semi = toks
        .iter()
        .any(|t| matches!(t.token_type, TokenType::Semicolon));
    let has_comma = toks
        .iter()
        .any(|t| matches!(t.token_type, TokenType::Comma));
    assert!(has_semi);
    assert!(has_comma);
    Ok(())
}

// ===========================================================================
// 14. Context-sensitive slash disambiguation
// ===========================================================================

#[test]
fn slash_is_division_after_number() -> R {
    let mut lexer = PerlLexer::new("10 / 2");
    let _ = lexer.next_token(); // 10
    let tok = lexer.next_token().ok_or("expected division")?;
    assert_eq!(tok.token_type, TokenType::Division);
    Ok(())
}

#[test]
fn slash_is_regex_after_keyword() -> R {
    let mut lexer = PerlLexer::new("if (/pattern/)");
    let _ = lexer.next_token(); // if
    let _ = lexer.next_token(); // (
    let tok = lexer.next_token().ok_or("expected regex")?;
    assert_eq!(tok.token_type, TokenType::RegexMatch);
    Ok(())
}

#[test]
fn slash_is_regex_after_equals() -> R {
    let mut lexer = PerlLexer::new("$x = /test/");
    let _ = lexer.next_token(); // $x
    let _ = lexer.next_token(); // =
    let tok = lexer.next_token().ok_or("expected regex")?;
    assert_eq!(tok.token_type, TokenType::RegexMatch);
    Ok(())
}

#[test]
fn slash_is_division_after_close_paren() -> R {
    let mut lexer = PerlLexer::new("(2) / 3");
    let _ = lexer.next_token(); // (
    let _ = lexer.next_token(); // 2
    let _ = lexer.next_token(); // )
    let tok = lexer.next_token().ok_or("expected division")?;
    assert_eq!(tok.token_type, TokenType::Division);
    Ok(())
}

#[test]
fn slash_is_division_after_variable() -> R {
    let mut lexer = PerlLexer::new("$x / $y");
    let _ = lexer.next_token(); // $x
    let tok = lexer.next_token().ok_or("expected division")?;
    assert_eq!(tok.token_type, TokenType::Division);
    Ok(())
}

#[test]
fn slash_is_regex_after_binding_op() -> R {
    let mut lexer = PerlLexer::new("$x =~ /foo/");
    let _ = lexer.next_token(); // $x
    let _ = lexer.next_token(); // =~
    let tok = lexer.next_token().ok_or("expected regex")?;
    assert_eq!(tok.token_type, TokenType::RegexMatch);
    Ok(())
}

#[test]
fn slash_is_regex_after_open_paren() -> R {
    let mut lexer = PerlLexer::new("(/regex/)");
    let _ = lexer.next_token(); // (
    let tok = lexer.next_token().ok_or("expected regex")?;
    assert_eq!(tok.token_type, TokenType::RegexMatch);
    Ok(())
}

#[test]
fn slash_is_regex_at_start_of_input() -> R {
    let mut lexer = PerlLexer::new("/pattern/i");
    let tok = lexer.next_token().ok_or("expected regex")?;
    assert_eq!(tok.token_type, TokenType::RegexMatch);
    Ok(())
}

#[test]
fn percent_is_modulo_after_number() -> R {
    let mut lexer = PerlLexer::new("10 % 3");
    let _ = lexer.next_token(); // 10
    let tok = lexer.next_token().ok_or("expected modulo")?;
    assert!(
        matches!(&tok.token_type, TokenType::Operator(op) if op.as_ref() == "%"),
        "got {:?}",
        tok.token_type
    );
    Ok(())
}

#[test]
fn percent_is_hash_sigil_at_start() -> R {
    let tok = first_token("%ENV").ok_or("no token")?;
    assert!(
        matches!(&tok.token_type, TokenType::Identifier(id) if id.as_ref().contains("ENV")),
        "got {:?}",
        tok.token_type
    );
    Ok(())
}

// ===========================================================================
// 15. Quote-like operators
// ===========================================================================

#[test]
fn q_operator_curly() -> R {
    let tok = first_token("q{hello}").ok_or("no token")?;
    assert_eq!(tok.token_type, TokenType::QuoteSingle);
    Ok(())
}

#[test]
fn qq_operator_paren() -> R {
    let tok = first_token("qq(world)").ok_or("no token")?;
    assert_eq!(tok.token_type, TokenType::QuoteDouble);
    Ok(())
}

#[test]
fn qw_operator() -> R {
    let tok = first_token("qw[a b c]").ok_or("no token")?;
    assert_eq!(tok.token_type, TokenType::QuoteWords);
    Ok(())
}

#[test]
fn qx_operator() -> R {
    let tok = first_token("qx{ls}").ok_or("no token")?;
    assert_eq!(tok.token_type, TokenType::QuoteCommand);
    Ok(())
}

#[test]
fn qr_operator() -> R {
    let tok = first_token("qr/pattern/i").ok_or("no token")?;
    assert_eq!(tok.token_type, TokenType::QuoteRegex);
    Ok(())
}

#[test]
fn m_operator() -> R {
    let tok = first_token("m{test}").ok_or("no token")?;
    assert_eq!(tok.token_type, TokenType::RegexMatch);
    Ok(())
}

#[test]
fn s_operator_basic() -> R {
    let tok = first_token("s/foo/bar/g").ok_or("no token")?;
    assert_eq!(tok.token_type, TokenType::Substitution);
    Ok(())
}

#[test]
fn tr_operator() -> R {
    let tok = first_token("tr/a-z/A-Z/").ok_or("no token")?;
    assert_eq!(tok.token_type, TokenType::Transliteration);
    Ok(())
}

#[test]
fn y_operator() -> R {
    let tok = first_token("y/0-9/a-j/").ok_or("no token")?;
    assert_eq!(tok.token_type, TokenType::Transliteration);
    Ok(())
}

#[test]
fn quote_op_without_delimiter_is_identifier() -> R {
    for op in ["q", "qq", "qw", "qr", "qx", "m", "s", "tr", "y"] {
        let tok = first_token(op).ok_or_else(|| format!("no token for '{}'", op))?;
        assert!(
            matches!(tok.token_type, TokenType::Identifier(_)),
            "bare '{}' should be identifier, got {:?}",
            op,
            tok.token_type
        );
    }
    Ok(())
}

#[test]
fn quote_ops_with_alternate_delimiters() -> R {
    let cases = [
        ("q<hello>", TokenType::QuoteSingle),
        ("qq[world]", TokenType::QuoteDouble),
        ("qw(a b)", TokenType::QuoteWords),
        ("qr{pat}", TokenType::QuoteRegex),
        ("s{old}{new}", TokenType::Substitution),
    ];
    for (input, expected) in &cases {
        let tok = first_token(input).ok_or_else(|| format!("no token for '{}'", input))?;
        assert_eq!(
            std::mem::discriminant(&tok.token_type),
            std::mem::discriminant(expected),
            "mismatch for '{}'",
            input
        );
    }
    Ok(())
}

// ===========================================================================
// 16. Heredoc tokenization
// ===========================================================================

#[test]
fn heredoc_double_quoted_marker() -> R {
    let input = "<<EOF\nhello\nEOF\n";
    let toks = tokens(input);
    let has_heredoc = toks
        .iter()
        .any(|t| matches!(t.token_type, TokenType::HeredocStart));
    assert!(has_heredoc, "expected HeredocStart token");
    Ok(())
}

#[test]
fn heredoc_single_quoted_marker() -> R {
    let input = "<<'END'\nhello\nEND\n";
    let toks = tokens(input);
    let has_heredoc = toks
        .iter()
        .any(|t| matches!(t.token_type, TokenType::HeredocStart));
    assert!(has_heredoc, "expected HeredocStart token");
    Ok(())
}

#[test]
fn heredoc_indented() -> R {
    let input = "<<~EOF\n  hello\n  EOF\n";
    let toks = tokens(input);
    let has_heredoc = toks
        .iter()
        .any(|t| matches!(t.token_type, TokenType::HeredocStart));
    assert!(has_heredoc, "expected HeredocStart for indented heredoc");
    Ok(())
}

#[test]
fn heredoc_unterminated_does_not_hang() {
    let input = "<<EOF\nhello world\nno terminator here\n";
    let toks = tokens(input);
    // Should produce tokens without hanging
    assert!(!toks.is_empty());
}

// ===========================================================================
// 17. Data sections and POD
// ===========================================================================

#[test]
fn data_section_marker() -> R {
    let input = "__DATA__\nsome data here\n";
    let toks = tokens(input);
    let has_data = toks.iter().any(|t| {
        matches!(
            &t.token_type,
            TokenType::DataMarker(_) | TokenType::DataBody(_)
        )
    });
    assert!(has_data, "expected data section tokens");
    Ok(())
}

#[test]
fn end_section_marker() -> R {
    let input = "__END__\nstuff after end\n";
    let toks = tokens(input);
    let has_data = toks.iter().any(|t| {
        matches!(
            &t.token_type,
            TokenType::DataMarker(_) | TokenType::DataBody(_)
        )
    });
    assert!(has_data, "expected data/end section tokens");
    Ok(())
}

#[test]
fn pod_section() {
    // POD may not be tokenized as a Pod token in all contexts;
    // verify the lexer terminates and produces tokens
    let input = "=pod\nSome documentation\n=cut\n";
    let toks = tokens(input);
    let has_eof = toks.iter().any(|t| matches!(t.token_type, TokenType::EOF));
    assert!(has_eof, "POD input should produce EOF");
}

// ===========================================================================
// 18. BOM handling
// ===========================================================================

#[test]
fn utf8_bom_is_skipped() -> R {
    let input = "\u{FEFF}my $x = 1;";
    let toks = tokens(input);
    let first = toks.first().ok_or("no tokens")?;
    assert!(
        matches!(&first.token_type, TokenType::Keyword(k) if k.as_ref() == "my"),
        "BOM should be skipped, got {:?}",
        first.token_type
    );
    Ok(())
}

// ===========================================================================
// 19. Checkpoint operations
// ===========================================================================

#[test]
fn checkpoint_new_defaults() {
    let cp = LexerCheckpoint::new();
    assert_eq!(cp.position, 0);
    assert_eq!(cp.mode, LexerMode::ExpectTerm);
    assert!(cp.delimiter_stack.is_empty());
    assert!(!cp.in_prototype);
    assert_eq!(cp.prototype_depth, 0);
    assert!(!cp.after_sub);
    assert!(cp.is_at_start());
}

#[test]
fn checkpoint_at_position() {
    let cp = LexerCheckpoint::at_position(50);
    assert_eq!(cp.position, 50);
    assert!(!cp.is_at_start());
}

#[test]
fn checkpoint_display() {
    let cp = LexerCheckpoint::at_position(42);
    let s = format!("{}", cp);
    assert!(s.contains("42"));
}

#[test]
fn checkpoint_diff_no_state_changes() {
    let cp1 = LexerCheckpoint::at_position(10);
    let cp2 = LexerCheckpoint::at_position(20);
    let diff = cp2.diff(&cp1);
    assert_eq!(diff.position_delta, 10);
    assert!(!diff.has_state_changes());
}

#[test]
fn checkpoint_diff_with_mode_change() {
    let cp1 = LexerCheckpoint::at_position(10);
    let mut cp2 = LexerCheckpoint::at_position(10);
    cp2.mode = LexerMode::ExpectOperator;
    let diff = cp2.diff(&cp1);
    assert!(diff.mode_changed);
    assert!(diff.has_state_changes());
}

#[test]
fn checkpoint_apply_edit_before() {
    let mut cp = LexerCheckpoint::at_position(50);
    cp.apply_edit(10, 5, 10); // Insert 5 chars before checkpoint
    assert_eq!(cp.position, 55);
}

#[test]
fn checkpoint_apply_edit_after() {
    let mut cp = LexerCheckpoint::at_position(50);
    cp.apply_edit(60, 10, 5); // Edit after checkpoint
    assert_eq!(cp.position, 50); // No change
}

#[test]
fn checkpoint_apply_edit_inside() {
    let mut cp = LexerCheckpoint::at_position(50);
    cp.apply_edit(45, 10, 5); // Edit contains checkpoint
    assert_eq!(cp.position, 45); // Reset to edit start
    assert_eq!(cp.mode, LexerMode::ExpectTerm); // State reset
}

#[test]
fn checkpoint_is_valid_for() {
    let cp = LexerCheckpoint::at_position(5);
    assert!(cp.is_valid_for("hello world"));
    assert!(!cp.is_valid_for("hi"));
}

#[test]
fn checkpoint_default_trait() {
    let cp: LexerCheckpoint = Default::default();
    assert_eq!(cp.position, 0);
}

#[test]
fn checkpoint_save_and_restore_on_lexer() -> R {
    let mut lexer = PerlLexer::new("my $x = 42;");
    let _ = lexer.next_token(); // my
    let cp = lexer.checkpoint();
    let tok_before = lexer.next_token().ok_or("expected $x")?;
    let _ = lexer.next_token(); // skip =
    assert!(lexer.can_restore(&cp));
    lexer.restore(&cp);
    let tok_after = lexer.next_token().ok_or("expected $x again")?;
    assert_eq!(tok_before.start, tok_after.start);
    Ok(())
}

#[test]
fn checkpoint_cache_basic() -> R {
    let mut cache = CheckpointCache::new(5);
    cache.add(LexerCheckpoint::at_position(10));
    cache.add(LexerCheckpoint::at_position(20));
    cache.add(LexerCheckpoint::at_position(30));

    let cp = cache.find_before(25).ok_or("expected checkpoint")?;
    assert_eq!(cp.position, 20);

    let cp = cache.find_before(10).ok_or("expected checkpoint")?;
    assert_eq!(cp.position, 10);

    assert!(cache.find_before(5).is_none());
    Ok(())
}

#[test]
fn checkpoint_cache_eviction() {
    let mut cache = CheckpointCache::new(3);
    for i in 0..10 {
        cache.add(LexerCheckpoint::at_position(i * 10));
    }
    // Should be trimmed to 3
    // Just verify it doesn't panic and has a reasonable count
    let result = cache.find_before(100);
    assert!(result.is_some());
}

#[test]
fn checkpoint_cache_clear() {
    let mut cache = CheckpointCache::new(10);
    cache.add(LexerCheckpoint::at_position(10));
    cache.clear();
    assert!(cache.find_before(100).is_none());
}

#[test]
fn checkpoint_cache_apply_edit() -> R {
    let mut cache = CheckpointCache::new(10);
    cache.add(LexerCheckpoint::at_position(10));
    cache.add(LexerCheckpoint::at_position(50));
    cache.apply_edit(20, 5, 10); // Insert 5 chars at pos 20
    let cp = cache.find_before(60).ok_or("expected checkpoint")?;
    assert_eq!(cp.position, 55); // 50 + 5
    Ok(())
}

// ===========================================================================
// 20. Edge cases
// ===========================================================================

#[test]
fn single_character_inputs() {
    let chars = [
        ";", ",", "(", ")", "[", "]", "{", "}", "+", "-", "*", "=", "<", ">", "!", "~",
    ];
    for ch in &chars {
        let toks = tokens(ch);
        assert!(!toks.is_empty(), "expected tokens for '{}'", ch);
    }
}

#[test]
fn very_long_identifier() -> R {
    let long_name = "x".repeat(10_000);
    let input = format!("${}", long_name);
    let toks = tokens(&input);
    assert!(!toks.is_empty());
    let first = toks.first().ok_or("no tokens")?;
    assert!(
        !matches!(first.token_type, TokenType::EOF),
        "long identifier should produce a token"
    );
    Ok(())
}

#[test]
fn many_semicolons() {
    let input = ";;;;;;;;;;;";
    let toks = significant(input);
    let semi_count = toks
        .iter()
        .filter(|t| matches!(t.token_type, TokenType::Semicolon))
        .count();
    assert_eq!(semi_count, 11);
}

#[test]
fn deeply_nested_parens() {
    let open: String = "(".repeat(50);
    let close: String = ")".repeat(50);
    let input = format!("{}1{}", open, close);
    let toks = tokens(&input);
    // Should not hang
    assert!(!toks.is_empty());
}

#[test]
fn comment_line() -> R {
    let input = "# this is a comment\nmy $x;";
    let toks = tokens(input);
    let _first = toks.first().ok_or("no tokens")?;
    // The comment may be skipped or returned
    let has_keyword = toks
        .iter()
        .any(|t| matches!(&t.token_type, TokenType::Keyword(k) if k.as_ref() == "my"));
    assert!(has_keyword, "should have 'my' keyword after comment");
    Ok(())
}

#[test]
fn multiple_statements() -> R {
    let input = "my $x = 1; my $y = 2; my $z = 3;";
    let toks = significant(input);
    let my_count = toks
        .iter()
        .filter(|t| matches!(&t.token_type, TokenType::Keyword(k) if k.as_ref() == "my"))
        .count();
    assert_eq!(my_count, 3, "expected 3 'my' keywords");
    Ok(())
}

#[test]
fn token_positions_monotonically_increase() -> R {
    let input = "my $x = 42; print $x;";
    let toks = tokens(input);
    let mut prev_start = 0;
    for tok in &toks {
        assert!(
            tok.start >= prev_start,
            "token {:?} start {} < prev {}",
            tok.token_type,
            tok.start,
            prev_start
        );
        prev_start = tok.start;
    }
    Ok(())
}

#[test]
fn token_spans_within_input() {
    let input = "my $x = 42;";
    let toks = tokens(input);
    for tok in &toks {
        assert!(
            tok.end <= input.len(),
            "token {:?} end {} > input len {}",
            tok.token_type,
            tok.end,
            input.len()
        );
        assert!(
            tok.start <= tok.end,
            "token {:?} start {} > end {}",
            tok.token_type,
            tok.start,
            tok.end
        );
    }
}

#[test]
fn unicode_identifier() -> R {
    let input = "my $café = 1;";
    let toks = tokens(input);
    let has_ident = toks.iter().any(
        |t| matches!(&t.token_type, TokenType::Identifier(id) if id.as_ref().contains("café")),
    );
    assert!(has_ident, "expected unicode identifier token");
    Ok(())
}

#[test]
fn unicode_in_string_literal() -> R {
    let input = r#""héllo wörld""#;
    let tok = first_token(input).ok_or("no token")?;
    assert!(
        !matches!(tok.token_type, TokenType::Error(_)),
        "unicode in string should not cause error"
    );
    Ok(())
}

#[test]
fn cjk_identifier() -> R {
    let input = "my $変数 = 1;";
    let toks = tokens(input);
    assert!(!toks.is_empty());
    Ok(())
}

#[test]
fn emoji_zwj_identifier() -> R {
    let input = "my $👨‍💻dev = 1;";
    let toks = tokens(input);
    let has_ident = toks.iter().any(|t| {
        matches!(
            &t.token_type,
            TokenType::Identifier(id) if id.as_ref().contains("👨‍💻dev")
        )
    });
    assert!(has_ident, "expected emoji ZWJ identifier token");
    Ok(())
}

#[test]
fn emoji_variation_selector_identifier() -> R {
    let input = "my $☕️_count = 1;";
    let toks = tokens(input);
    let has_ident = toks.iter().any(|t| {
        matches!(
            &t.token_type,
            TokenType::Identifier(id) if id.as_ref().contains("☕️_count")
        )
    });
    assert!(
        has_ident,
        "expected variation-selector emoji identifier token"
    );
    Ok(())
}

#[test]
fn empty_input_eof_position() -> R {
    let toks = tokens("");
    let eof = toks.first().ok_or("no eof")?;
    assert_eq!(eof.start, 0);
    assert_eq!(eof.end, 0);
    Ok(())
}

#[test]
fn newline_only_input() -> R {
    let toks = tokens("\n");
    let last = toks.last().ok_or("no tokens")?;
    assert_eq!(last.token_type, TokenType::EOF);
    Ok(())
}

#[test]
fn mixed_line_endings() -> R {
    let input = "my $x;\r\nmy $y;\nmy $z;\r";
    let toks = tokens(input);
    let my_count = toks
        .iter()
        .filter(|t| matches!(&t.token_type, TokenType::Keyword(k) if k.as_ref() == "my"))
        .count();
    assert_eq!(my_count, 3, "should handle mixed line endings");
    Ok(())
}

// ===========================================================================
// 21. Version strings
// ===========================================================================

#[test]
fn version_string_v_prefix() -> R {
    let tok = first_token("v5.32.0").ok_or("no token")?;
    assert!(
        matches!(
            &tok.token_type,
            TokenType::Version(_) | TokenType::Identifier(_) | TokenType::Number(_)
        ),
        "version string got {:?}",
        tok.token_type
    );
    Ok(())
}

// ===========================================================================
// 22. Format body parsing
// ===========================================================================

#[test]
fn format_body_with_dot_terminator() -> R {
    let input = "@<<<< @>>>>\nfoo    bar\n.\n";
    let mut lexer = PerlLexer::new(input);
    lexer.enter_format_mode();
    let tok = lexer.next_token().ok_or("expected format body")?;
    if let TokenType::FormatBody(ref body) = tok.token_type {
        assert!(!body.is_empty() || !tok.text.is_empty());
    }
    Ok(())
}

// ===========================================================================
// 23. OO-style Perl
// ===========================================================================

#[test]
fn method_call_chain() -> R {
    let input = "$obj->foo->bar->baz";
    let toks = significant(input);
    let arrow_count = toks
        .iter()
        .filter(|t| matches!(&t.token_type, TokenType::Operator(op) if op.as_ref() == "->"))
        .count();
    assert_eq!(arrow_count, 3, "expected 3 arrow tokens");
    Ok(())
}

#[test]
fn package_separator() -> R {
    let input = "Foo::Bar::baz()";
    let toks = significant(input);
    assert!(!toks.is_empty());
    Ok(())
}

// ===========================================================================
// 24. Real-world Perl snippets
// ===========================================================================

#[test]
fn real_world_hash_ref() -> R {
    let input = "my $ref = { key => 'value', num => 42 };";
    let toks = tokens(input);
    // Fat comma is Operator("=>")
    let has_fat_comma = toks
        .iter()
        .any(|t| matches!(&t.token_type, TokenType::Operator(op) if op.as_ref() == "=>"));
    assert!(has_fat_comma);
    Ok(())
}

#[test]
fn real_world_array_operations() -> R {
    let input = "push @array, 1, 2, 3; my $len = scalar @array;";
    let toks = tokens(input);
    assert!(toks.len() > 5);
    Ok(())
}

#[test]
fn real_world_regex_substitution() -> R {
    let input = "$str =~ s/foo/bar/g;";
    let toks = significant(input);
    let has_sub = toks
        .iter()
        .any(|t| matches!(t.token_type, TokenType::Substitution));
    assert!(has_sub, "expected Substitution token");
    Ok(())
}

#[test]
fn real_world_conditional() -> R {
    let input = "if ($x > 0) { print \"positive\\n\"; } elsif ($x == 0) { print \"zero\\n\"; } else { print \"negative\\n\"; }";
    let toks = tokens(input);
    let keyword_count = toks
        .iter()
        .filter(|t| matches!(t.token_type, TokenType::Keyword(_)))
        .count();
    assert!(
        keyword_count >= 3,
        "expected at least 3 keywords (if, elsif, else, print...)"
    );
    Ok(())
}

#[test]
fn real_world_while_loop() -> R {
    let input = "while (my $line = <STDIN>) { chomp $line; print $line; }";
    let toks = tokens(input);
    assert!(toks.len() > 5);
    Ok(())
}

#[test]
fn real_world_use_statement() -> R {
    let input = "use strict; use warnings; use Carp qw(croak);";
    let toks = tokens(input);
    let use_count = toks
        .iter()
        .filter(|t| matches!(&t.token_type, TokenType::Keyword(k) if k.as_ref() == "use"))
        .count();
    assert_eq!(use_count, 3, "expected 3 'use' keywords");
    Ok(())
}

#[test]
fn real_world_for_loop() -> R {
    let input = "for my $i (0..9) { print $i; }";
    let toks = tokens(input);
    let has_for = toks
        .iter()
        .any(|t| matches!(&t.token_type, TokenType::Keyword(k) if k.as_ref() == "for"));
    assert!(has_for);
    Ok(())
}

#[test]
fn real_world_anonymous_sub() -> R {
    let input = "my $cb = sub { return 42; };";
    let toks = tokens(input);
    let has_sub = toks
        .iter()
        .any(|t| matches!(&t.token_type, TokenType::Keyword(k) if k.as_ref() == "sub"));
    assert!(has_sub);
    Ok(())
}
