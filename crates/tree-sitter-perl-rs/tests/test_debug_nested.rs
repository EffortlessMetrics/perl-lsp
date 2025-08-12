//! Debug nested heredoc

use tree_sitter_perl::perl_lexer::{PerlLexer, TokenType};

#[test]
fn test_nested_heredoc() {
    let input = r#"
my $outer = 'EOF';
my $inner = $outer;
my $doc = <<${inner};
Nested content
EOF
"#;

    let mut lexer = PerlLexer::new(input);

    println!("=== Tokenizing nested heredoc ===");
    let mut tokens = Vec::new();
    while let Some(token) = lexer.next_token() {
        println!("Token: {:?}", token);
        if matches!(&token.token_type, TokenType::Error(msg) if msg.contains("heredoc")) {
            println!("  ^ Heredoc error detected");
        }
        if token.text.contains("inner") || token.text.contains("outer") {
            println!("  ^ Variable-related token");
        }
        tokens.push(token);
    }

    // Check if we found a HeredocStart token
    let has_heredoc = tokens
        .iter()
        .any(|t| matches!(t.token_type, TokenType::HeredocStart));
    println!("\nFound HeredocStart: {}", has_heredoc);
}
