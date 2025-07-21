//! Debug array element heredoc

use tree_sitter_perl::perl_lexer::{PerlLexer, TokenType};

#[test] 
fn test_array_element_heredoc() {
    let input = r#"my @markers = ('START', 'END', 'DATA');
my $log = <<$markers[1];
Log entry
END
"#;
    
    let mut lexer = PerlLexer::new(input);
    
    println!("=== Tokenizing array element heredoc ===");
    while let Some(token) = lexer.next_token() {
        println!("Token: {:?}", token);
        if matches!(&token.token_type, TokenType::Error(msg) if msg.contains("heredoc")) {
            println!("  ^ Heredoc error detected");
        }
    }
}