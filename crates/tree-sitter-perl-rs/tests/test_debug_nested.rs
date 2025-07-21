//! Debug nested heredoc

use tree_sitter_perl::perl_lexer::{PerlLexer, TokenType};

#[test] 
fn test_nested_heredoc() {
    let input = r#"
my $outer = 'EOF';
my $inner = $outer;
my $doc = <<${${var}};
Nested content
EOF
"#;
    
    let mut lexer = PerlLexer::new(input);
    
    println!("=== Tokenizing nested heredoc ===");
    while let Some(token) = lexer.next_token() {
        println!("Token: {:?}", token);
        if matches!(&token.token_type, TokenType::Error(msg) if msg.contains("heredoc")) {
            println!("  ^ Heredoc error detected");
        }
    }
}