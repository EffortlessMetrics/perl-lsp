//! Regression tests for Unicode, BOM, and span robustness.

use perl_lexer::{PerlLexer, Token, TokenType};

fn collect_all_tokens(input: &str) -> Vec<Token> {
    let mut lexer = PerlLexer::new(input);
    let mut out = Vec::new();
    while let Some(token) = lexer.next_token() {
        let is_eof = token.token_type == TokenType::EOF;
        out.push(token);
        if is_eof {
            break;
        }
    }
    out
}

#[test]
fn test_unicode_heredoc_fixes() {
    let test_cases = [
        "¡<<'",        // The specific failing case - should not panic
        "<<'END'",     // Valid heredoc for comparison
        "¡test",       // Unicode identifier
        "¡ << 'test'", // Unicode with spacing
    ];

    for input in test_cases {
        let tokens = collect_all_tokens(input);
        assert!(
            tokens.iter().any(|t| t.token_type == TokenType::EOF),
            "Expected EOF token for input {input:?}"
        );
    }
}

#[test]
fn test_unicode_regression_case() {
    // The specific failing case from the proptest regression
    let input = "¡<<'";

    let mut lexer = PerlLexer::new(input);

    // This should not panic anymore
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| lexer.next_token()));

    assert!(result.is_ok(), "Lexer should not panic on Unicode input: {:?}", input);
}

#[test]
fn test_bom_is_skipped_and_spans_stay_byte_exact() {
    let input = "\u{FEFF}my $x = 1;";
    let tokens = collect_all_tokens(input);

    let first_non_ws = tokens
        .iter()
        .find(|t| {
            !matches!(t.token_type, TokenType::Whitespace | TokenType::Newline | TokenType::EOF)
        })
        .expect("expected at least one non-whitespace token");

    assert_eq!(first_non_ws.text.as_ref(), "my");
    assert_eq!(first_non_ws.start, 3, "first token should start after UTF-8 BOM bytes");
    assert_eq!(&input[first_non_ws.start..first_non_ws.end], first_non_ws.text.as_ref());
}

#[test]
fn test_multibyte_spans_and_emoji_joiner_continuations() {
    let input = "my $👨‍👩‍👧‍👦️ = 1;";
    let tokens = collect_all_tokens(input);

    for token in &tokens {
        assert!(token.start <= token.end, "Invalid span ordering for {:?}", token.token_type);
        assert!(token.end <= input.len(), "Token end out of bounds for {:?}", token.token_type);
        assert!(
            input.is_char_boundary(token.start),
            "Token start is not char boundary for {:?}",
            token.token_type
        );
        assert!(
            input.is_char_boundary(token.end),
            "Token end is not char boundary for {:?}",
            token.token_type
        );
        assert_eq!(
            &input[token.start..token.end],
            token.text.as_ref(),
            "Token text must match input slice for {:?}",
            token.token_type
        );
    }

    assert!(
        tokens.iter().any(
            |t| matches!(&t.token_type, TokenType::Identifier(name) if name.as_ref() == "$👨‍👩‍👧‍👦️")
        ),
        "Expected emoji ZWJ identifier token"
    );
}

#[test]
fn test_weird_unicode_prefixes_do_not_panic() {
    let cases =
        ["🙂<<'END'\nbody\nEND\n", "🙂q'abc'", "🙂m/abc/", "🙂tr/a/b/", "🙂<<\"EOF\"\nX\nEOF\n"];

    for input in cases {
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| collect_all_tokens(input)));
        assert!(result.is_ok(), "Lexer panicked for input {input:?}");
    }
}
