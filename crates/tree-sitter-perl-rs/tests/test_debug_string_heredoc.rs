//! Debug heredoc in string
use tree_sitter_perl::perl_lexer::{PerlLexer, TokenType};

#[test]
fn test_string_heredoc() {
    let input = r#"
my $template = "$prefix<<$end_tag
Template content here
$end_tag";
"#;
    
    let mut lexer = PerlLexer::new(input);
    
    println!("=== Tokenizing heredoc in string ===");
    println!("Input: {}", input);
    
    while let Some(token) = lexer.next_token() {
        println!("Token: {:?}", token);
        if token.text.contains("<<") {
            println!("  ^ Found << operator");
        }
        if matches!(token.token_type, TokenType::StringLiteral) {
            println!("  ^ String content: {}", token.text);
        }
    }
}