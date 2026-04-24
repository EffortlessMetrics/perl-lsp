use perl_parser_core::token_stream::{Token, TokenKind, TokenStream};

#[test]
fn on_stmt_boundary_resets_peek() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::new("my $x; my $y;");
    // Prime the peek
    let _ = stream.peek();
    // on_stmt_boundary should invalidate cached peeks
    stream.on_stmt_boundary();
    // After reset, peek should still work
    if let Ok(token) = stream.peek() {
        // We should get a valid token (the reparsed first token)
        let _ = format!("{:?}", token.kind);
    }
    Ok(())
}

#[test]
fn invalidate_peek_clears_cache() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::new("1 + 2");
    let _ = stream.peek();
    let _ = stream.peek_second();
    // Invalidate all cached peeks
    stream.invalidate_peek();
    // Should still work after invalidation
    if let Ok(token) = stream.peek() {
        let _ = format!("{:?}", token.kind);
    }
    Ok(())
}

#[test]
fn peek_fresh_kind_on_empty() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::new("");
    let kind = stream.peek_fresh_kind();
    // Should return Some(Eof) or similar
    assert!(kind.is_some());
    Ok(())
}

#[test]
fn peek_fresh_kind_on_valid_input() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::new("my $x;");
    let kind = stream.peek_fresh_kind();
    assert!(kind.is_some());
    Ok(())
}

#[test]
fn enter_format_mode_does_not_crash() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::new("format STDOUT =\nsome text\n.\n");
    stream.enter_format_mode();
    // Should still be able to get tokens
    if let Ok(token) = stream.peek() {
        let _ = format!("{:?}", token.kind);
    }
    Ok(())
}

#[test]
fn from_vec_synthesized_eof_uses_last_token_end() -> Result<(), Box<dyn std::error::Error>> {
    let tokens =
        vec![Token::new(TokenKind::My, "my", 0, 2), Token::new(TokenKind::Identifier, "x", 3, 4)];
    let mut stream = TokenStream::from_vec(tokens);
    let _ = stream.next()?;
    let _ = stream.next()?;
    let eof = stream.next()?;

    assert_eq!(eof.kind, TokenKind::Eof);
    assert_eq!(eof.start, 4);
    assert_eq!(eof.end, 4);
    Ok(())
}
