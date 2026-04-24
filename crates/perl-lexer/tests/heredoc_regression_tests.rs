use perl_lexer::{PerlLexer, TokenType};

#[test]
fn lexer_terminates_on_backtick_heredoc_with_cr() {
    let mut lx = PerlLexer::new("``<<a\r");

    // Try to consume up to 16 tokens - should not spin forever
    for i in 0..16 {
        if let Some(token) = lx.next_token() {
            // Just consume tokens, we're checking for termination
            if matches!(token.token_type, perl_lexer::TokenType::EOF) {
                // Found EOF, lexer terminated properly
                break;
            }
        } else {
            // No more tokens
            break;
        }

        // Safety check - if we're still going after 15 iterations, something's wrong
        assert!(i < 15, "Lexer appears to be in infinite loop");
    }

    // If we got here, the lexer terminated properly
    // Test passed - lexer terminated without infinite loop
}

#[test]
fn lexer_handles_heredoc_with_various_line_endings() {
    // Test with LF
    let mut lx = PerlLexer::new("<<EOF\nHello\nEOF\n");
    let mut token_count = 0;
    while let Some(token) = lx.next_token() {
        token_count += 1;
        if matches!(token.token_type, perl_lexer::TokenType::EOF) {
            break;
        }
        assert!(token_count < 20, "Too many tokens, possible infinite loop");
    }

    // Test with CRLF
    let mut lx = PerlLexer::new("<<EOF\r\nHello\r\nEOF\r\n");
    let mut token_count = 0;
    while let Some(token) = lx.next_token() {
        token_count += 1;
        if matches!(token.token_type, perl_lexer::TokenType::EOF) {
            break;
        }
        assert!(token_count < 20, "Too many tokens, possible infinite loop");
    }

    // Test with just CR (old Mac style)
    let mut lx = PerlLexer::new("<<EOF\rHello\rEOF\r");
    let mut token_count = 0;
    while let Some(token) = lx.next_token() {
        token_count += 1;
        if matches!(token.token_type, perl_lexer::TokenType::EOF) {
            break;
        }
        assert!(token_count < 20, "Too many tokens, possible infinite loop");
    }
}

#[test]
fn lexer_handles_malformed_heredoc_gracefully() {
    // Heredoc without terminator
    let mut lx = PerlLexer::new("<<EOF\nThis heredoc never ends");
    let mut token_count = 0;
    while let Some(token) = lx.next_token() {
        token_count += 1;
        if matches!(token.token_type, perl_lexer::TokenType::EOF) {
            break;
        }
        assert!(token_count < 30, "Too many tokens, possible infinite loop");
    }

    // Empty heredoc delimiter
    let mut lx = PerlLexer::new("<<\nContent\n");
    let mut token_count = 0;
    while let Some(token) = lx.next_token() {
        token_count += 1;
        if matches!(token.token_type, perl_lexer::TokenType::EOF) {
            break;
        }
        assert!(token_count < 20, "Too many tokens, possible infinite loop");
    }
}

#[test]
fn lexer_rejects_unclosed_backtick_heredoc_label() {
    let input = "<<`CMD\rprint 1;\r";
    let toks = PerlLexer::new(input).collect_tokens();
    assert!(
        toks.iter().all(|t| !matches!(t.token_type, TokenType::HeredocStart)),
        "unterminated backtick heredoc label must not be accepted"
    );
    assert!(
        toks.iter().any(|t| matches!(t.token_type, TokenType::EOF)),
        "lexer should still terminate after malformed heredoc label"
    );
}

#[test]
fn data_marker_line_with_crlf_transitions_cleanly() {
    let input = "my $x = 1;\r\n__DATA__\r\npayload\r\n";
    let toks = PerlLexer::new(input).collect_tokens();
    let marker = toks.iter().find(|t| matches!(t.token_type, TokenType::DataMarker(_)));
    let body = toks.iter().find(|t| matches!(t.token_type, TokenType::DataBody(_)));

    assert!(marker.is_some(), "expected __DATA__ marker token");
    assert!(body.is_some(), "expected data body token");
    if let Some(marker) = marker {
        assert_eq!(marker.text.as_ref(), "__DATA__");
    }
    if let Some(body) = body {
        assert_eq!(body.text.as_ref(), "payload\r\n");
    }
}
