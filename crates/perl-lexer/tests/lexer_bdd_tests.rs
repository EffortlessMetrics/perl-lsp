use perl_lexer::{PerlLexer, TokenType};

type R = Result<(), Box<dyn std::error::Error>>;

fn collect_token_types(input: &str) -> Vec<TokenType> {
    let mut lexer = PerlLexer::new(input);
    let mut types = Vec::new();

    while let Some(token) = lexer.next_token() {
        types.push(token.token_type.clone());
        if matches!(types.last(), Some(TokenType::EOF)) {
            break;
        }
    }

    types
}

#[test]
fn given_empty_input_when_lexing_then_only_eof_is_emitted() -> R {
    // Given
    let input = "";

    // When
    let token_types = collect_token_types(input);

    // Then
    assert_eq!(token_types.len(), 1, "empty input should emit only EOF");
    assert!(matches!(token_types.first(), Some(TokenType::EOF)));
    Ok(())
}

#[test]
fn given_quote_operator_without_delimiter_when_lexing_then_it_is_identifier() -> R {
    // Given
    let input = "qq";

    // When
    let token_types = collect_token_types(input);

    // Then
    assert!(
        matches!(token_types.first(), Some(TokenType::Identifier(text)) if text.as_ref() == "qq")
    );
    assert!(matches!(token_types.get(1), Some(TokenType::EOF)));
    Ok(())
}

#[test]
fn given_quote_operator_with_delimiter_when_lexing_then_quote_token_is_emitted() -> R {
    // Given
    let input = "qq{hello world}";

    // When
    let token_types = collect_token_types(input);

    // Then
    assert!(matches!(token_types.first(), Some(TokenType::QuoteDouble)));
    assert!(matches!(token_types.get(1), Some(TokenType::EOF)));
    Ok(())
}

#[test]
fn given_heredoc_with_default_mode_when_lexing_then_start_is_emitted_without_body_token() -> R {
    // Given
    let input = "print <<'EOF';\nhello\nEOF\n";

    // When
    let token_types = collect_token_types(input);

    // Then
    assert!(
        token_types.iter().any(|token_type| matches!(token_type, TokenType::HeredocStart)),
        "expected HeredocStart token"
    );
    assert!(
        !token_types.iter().any(|token_type| matches!(token_type, TokenType::HeredocBody(_))),
        "default lexer mode should not emit HeredocBody tokens"
    );
    assert!(matches!(token_types.last(), Some(TokenType::EOF)));
    Ok(())
}

#[test]
fn given_heredoc_with_body_mode_when_lexing_then_body_token_is_emitted() -> R {
    // Given
    let input = "print <<EOF;\nhello world\nEOF\n";

    // When
    let token_types = PerlLexer::with_body_tokens(input)
        .collect_tokens()
        .into_iter()
        .map(|token| token.token_type)
        .collect::<Vec<_>>();

    // Then
    assert!(
        token_types.iter().any(|token_type| matches!(token_type, TokenType::HeredocBody(_))),
        "with_body_tokens mode should emit HeredocBody tokens"
    );
    assert!(matches!(token_types.last(), Some(TokenType::EOF)));
    Ok(())
}

#[test]
fn given_special_scalar_when_lexing_then_it_remains_a_single_identifier() -> R {
    // Given
    let input = "$$";

    // When
    let token_types = collect_token_types(input);

    // Then
    assert!(
        matches!(token_types.first(), Some(TokenType::Identifier(text)) if text.as_ref() == "$$")
    );
    assert!(matches!(token_types.get(1), Some(TokenType::EOF)));
    Ok(())
}
