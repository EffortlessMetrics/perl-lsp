use perl_lexer::{PerlLexer, TokenType};
use perl_tokenizer::{Token, TokenKind, TokenStream};

fn collect_raw_lexer_tokens(source: &str) -> Vec<perl_lexer::Token> {
    let mut lexer = PerlLexer::new(source);
    let mut tokens = Vec::new();

    while let Some(token) = lexer.next_token() {
        if matches!(token.token_type, TokenType::EOF) {
            break;
        }

        tokens.push(token);
    }

    tokens
}

#[test]
fn bdd_given_raw_lexer_tokens_when_converted_then_trivia_is_filtered_and_eof_is_synthesized()
-> Result<(), Box<dyn std::error::Error>> {
    let raw = collect_raw_lexer_tokens("my $x = 1; # trailing comment\n");
    let parser_tokens = TokenStream::lexer_tokens_to_parser_tokens(raw);

    assert_eq!(
        parser_tokens.iter().map(|token| token.kind).collect::<Vec<_>>(),
        vec![
            TokenKind::My,
            TokenKind::Identifier,
            TokenKind::Assign,
            TokenKind::Number,
            TokenKind::Semicolon,
        ]
    );

    let mut stream = TokenStream::from_vec(parser_tokens);
    let mut kinds = Vec::new();

    loop {
        let token = stream.next()?;
        kinds.push(token.kind);
        if token.kind == TokenKind::Eof {
            break;
        }
    }

    assert_eq!(
        kinds,
        vec![
            TokenKind::My,
            TokenKind::Identifier,
            TokenKind::Assign,
            TokenKind::Number,
            TokenKind::Semicolon,
            TokenKind::Eof,
        ]
    );
    assert!(matches!(stream.peek(), Ok(token) if token.kind == TokenKind::Eof));

    Ok(())
}

#[test]
fn bdd_given_buffered_tokens_when_peek_is_invalidated_then_cursor_advances_from_current_position()
-> Result<(), Box<dyn std::error::Error>> {
    let tokens = vec![
        Token::new(TokenKind::My, "my", 0, 2),
        Token::new(TokenKind::Identifier, "$x", 3, 5),
        Token::new(TokenKind::Semicolon, ";", 5, 6),
    ];
    let mut stream = TokenStream::from_vec(tokens);

    assert_eq!(stream.peek()?.kind, TokenKind::My);

    stream.invalidate_peek();
    assert_eq!(stream.peek()?.kind, TokenKind::Identifier);

    stream.on_stmt_boundary();
    assert_eq!(stream.peek()?.kind, TokenKind::Semicolon);
    assert_eq!(stream.next()?.kind, TokenKind::Semicolon);
    assert_eq!(stream.next()?.kind, TokenKind::Eof);

    Ok(())
}
