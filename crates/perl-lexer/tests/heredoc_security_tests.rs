use perl_lexer::{PerlLexer, TokenType};
use std::time::{Duration, Instant};

#[test]
fn test_heredoc_depth_limit() {
    let mut code = String::from("my @h = (");
    for i in 0..110 {
        code.push_str(&format!("<<EOF{}, ", i));
    }
    code.push_str(");");

    let mut lexer = PerlLexer::new(&code);
    let tokens = lexer.collect_tokens();

    let error_tokens: Vec<_> = tokens.iter()
        .filter(|t| matches!(t.token_type, TokenType::Error(ref msg) if msg.contains("Heredoc nesting too deep")))
        .collect();

    assert!(!error_tokens.is_empty(), "Should have found 'nesting too deep' errors");
}

#[test]
fn test_heredoc_timeout() {
    // This is hard to test deterministically without mocking time,
    // but we can try with a very large input and see if it triggers.
    // Actually, we can just check if the code compiles and runs.

    let mut code = String::from("my $x = <<EOF;\n");
    for _ in 0..100000 {
        code.push_str("some content line\n");
    }
    // No EOF terminator

    let start = Instant::now();
    let mut lexer = PerlLexer::new(&code);
    let _tokens = lexer.collect_tokens();
    let duration = start.elapsed();

    assert!(duration < Duration::from_secs(10), "Lexer should not hang for more than 10 seconds");
}
