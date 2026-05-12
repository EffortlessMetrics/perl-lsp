use perl_lexer::{PerlLexer, TokenType};

type R = Result<(), Box<dyn std::error::Error>>;

fn assert_spans_are_valid(input: &str) {
    let mut lexer = PerlLexer::new(input);
    let mut token_count = 0usize;

    while let Some(token) = lexer.next_token() {
        assert!(input.is_char_boundary(token.start), "token.start must be a char boundary");
        assert!(input.is_char_boundary(token.end), "token.end must be a char boundary");
        assert!(token.start <= token.end, "token start must not exceed end");
        assert!(token.end <= input.len(), "token end must be within the input");
        assert_eq!(
            token.text.as_ref(),
            &input[token.start..token.end],
            "token text must match its byte span"
        );

        token_count += 1;
        assert!(token_count <= 64, "lexer should terminate for short regression cases");

        if matches!(token.token_type, TokenType::EOF) {
            break;
        }
    }
}

#[test]
fn unicode_and_bom_prefixes_do_not_panic_and_have_valid_spans() -> R {
    let cases = [
        "¡<<'",             // original regression shape
        "\u{FEFF}my $x=1;", // UTF-8 BOM at file start
        "👩‍💻<<'END'",        // ZWJ emoji prefix before heredoc-like bytes
        "🏳️‍🌈q'abc'",         // variation selector + joiner before quote-like bytes
        "🧑🏽‍💻\"x\"",          // skin tone modifier + ZWJ
    ];

    for case in cases {
        assert_spans_are_valid(case);
    }

    Ok(())
}

#[test]
fn unicode_regression_case_no_panic() -> R {
    let input = "¡<<'";
    let mut lexer = PerlLexer::new(input);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| lexer.next_token()));
    assert!(result.is_ok(), "Lexer should not panic on Unicode input: {input:?}");
    Ok(())
}
