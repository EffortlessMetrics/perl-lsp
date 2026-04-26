use perl_parser_core::token_stream::{Token, TokenKind, TokenStream};
use perl_tdd_support::must;

#[test]
fn empty_stream_is_eof() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::new("");
    assert!(stream.is_eof());
    Ok(())
}

#[test]
fn peek_returns_token() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::new("42");
    let token = must(stream.peek());
    assert!(!token.text.is_empty());
    Ok(())
}

#[test]
fn next_consumes_token() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::new("42");
    let token = must(stream.next());
    assert!(!token.text.is_empty());
    assert!(stream.is_eof());
    Ok(())
}

#[test]
fn peek_second_looks_ahead() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::new("my $x");
    let _first = must(stream.peek());
    let second = stream.peek_second();
    assert!(second.is_ok(), "should be able to peek second token");
    Ok(())
}

#[test]
fn peek_third_looks_further() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::new("my $x = 42;");
    let third = stream.peek_third();
    assert!(third.is_ok(), "should be able to peek third token");
    Ok(())
}

#[test]
fn stream_processes_multiple_tokens() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::new("my $x = 42;");
    let mut count = 0;
    while !stream.is_eof() {
        let _tok = must(stream.next());
        count += 1;
        if count > 100 {
            return Err("infinite loop in token stream".into());
        }
    }
    assert!(count >= 4, "should have at least 4 tokens, got {}", count);
    Ok(())
}

#[test]
fn buffered_stream_synthesizes_eof_at_last_token_end() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::from_vec(vec![
        Token::new(TokenKind::My, "my", 0, 2),
        Token::new(TokenKind::Identifier, "x", 3, 4),
    ]);

    let _ = must(stream.next());
    let _ = must(stream.next());
    let eof = must(stream.next());
    assert_eq!(eof.kind, TokenKind::Eof);
    assert_eq!(eof.start, 4);
    assert_eq!(eof.end, 4);
    Ok(())
}
