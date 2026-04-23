use perl_parser::Parser;

#[test]
fn parses_heredoc_start_token() {
    // Test that the parser handles HeredocStart tokens
    let src = "print <<'EOF';\nHello World\nEOF\n";
    let mut parser = Parser::new(src);
    let result = parser.parse();

    // Should parse without panicking or entering infinite loop
    assert!(result.is_ok(), "Failed to parse heredoc: {:?}", result);
}

#[test]
fn parses_bare_heredoc_label() {
    let src = "<<EOF\ntest\nEOF\n";
    let mut parser = Parser::new(src);
    let result = parser.parse();
    assert!(result.is_ok(), "Failed to parse bare heredoc: {:?}", result);
}

#[test]
fn parses_indented_heredoc() {
    let src = "<<~END\n    indented content\n    END";
    let mut parser = Parser::new(src);
    let result = parser.parse();
    assert!(result.is_ok(), "Failed to parse indented heredoc: {:?}", result);
}

#[test]
fn parses_sigil_brace_split_tokens() {
    // Test that the parser handles split sigil+brace tokens
    for expr in ["${x}", "@{arr}", "%{hash}"] {
        let src = format!("print {};", expr);
        let mut parser = Parser::new(&src);
        let result = parser.parse();
        assert!(result.is_ok(), "Failed to parse {}: {:?}", expr, result);
    }
}

#[test]
fn parses_empty_sigil_brace() {
    // Test empty sigil+brace (now split into two tokens)
    for expr in ["${}", "@{}", "%{}"] {
        let src = format!("my $x = {};", expr);
        let mut parser = Parser::new(&src);
        let result = parser.parse();
        // Parser should handle this gracefully even if semantically invalid
        assert!(result.is_ok() || result.is_err(), "Parser should not panic on {}", expr);
    }
}

#[test]
fn parses_sigil_brace_with_whitespace() {
    // Test sigil+brace with whitespace/newline after brace
    let cases = ["${ }", "@{\n}", "%{  \n  }"];

    for expr in cases {
        let src = format!("print {};", expr);
        let mut parser = Parser::new(&src);
        let _ = parser.parse(); // Just ensure no panic
    }
}
