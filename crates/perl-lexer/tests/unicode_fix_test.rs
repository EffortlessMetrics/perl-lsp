//! Unicode regression tests for tokenizer safety and span correctness.

use perl_lexer::{PerlLexer, TokenType};

type R = Result<(), Box<dyn std::error::Error>>;

fn assert_token_spans_within_input(input: &str) {
    let tokens = PerlLexer::new(input).collect_tokens();

    for token in tokens {
        assert!(token.start <= token.end, "invalid token span ordering for {:?}", token.token_type);
        assert!(
            token.end <= input.len(),
            "token span out of bounds for {:?}: {}..{} > {}",
            token.token_type,
            token.start,
            token.end,
            input.len()
        );
        assert!(
            input.is_char_boundary(token.start) && input.is_char_boundary(token.end),
            "token span not on UTF-8 boundary for {:?}: {}..{}",
            token.token_type,
            token.start,
            token.end
        );
        assert_eq!(token.text.as_ref(), &input[token.start..token.end]);
    }
}

#[test]
fn unicode_prefixes_resembling_heredoc_do_not_panic() -> R {
    for input in ["¡<<'", "𝑥<<'END'", "👩\u{200D}💻<<'EOF'", "¡ << 'test'"] {
        let tokens = PerlLexer::new(input).collect_tokens();
        let has_eof = tokens.iter().any(|token| matches!(token.token_type, TokenType::EOF));
        assert!(has_eof, "lexer must always terminate with EOF for {input:?}");
        assert_token_spans_within_input(input);
    }

    Ok(())
}

#[test]
fn utf8_bom_is_skipped_only_at_file_start() -> R {
    let input = "\u{FEFF}my $x = 1;";
    let tokens = PerlLexer::new(input).collect_tokens();
    let first = tokens
        .iter()
        .find(|token| !matches!(token.token_type, TokenType::Whitespace | TokenType::Newline))
        .ok_or("expected non-whitespace token")?;

    assert!(matches!(first.token_type, TokenType::Keyword(ref kw) if kw.as_ref() == "my"));
    assert_eq!(first.start, "\u{FEFF}".len());
    assert_token_spans_within_input(input);
    Ok(())
}

#[test]
fn emoji_joiner_variation_selector_identifier_has_valid_multibyte_spans() -> R {
    let input = "👩\u{200D}💻\u{FE0F}\u{E0100}\u{E0067}";
    let tokens = PerlLexer::new(input).collect_tokens();

    let ident = tokens
        .iter()
        .find(|token| matches!(token.token_type, TokenType::Identifier(_)))
        .ok_or("identifier token should exist")?;

    assert_eq!(ident.start, 0);
    assert_eq!(ident.end, input.len());
    assert_eq!(ident.text.as_ref(), input);
    assert_token_spans_within_input(input);
    Ok(())
}
