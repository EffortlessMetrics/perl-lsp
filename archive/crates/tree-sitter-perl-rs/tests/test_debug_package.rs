//! Debug package variable heredoc

use tree_sitter_perl::perl_lexer::{PerlLexer, TokenType};

#[test]
fn test_package_variable_heredoc() {
    let input = r#"package My::Config;
our $END_MARKER = 'END_CONFIG';

package main;
my $config = <<$My::Config::END_MARKER;
Configuration data
END_CONFIG
"#;

    let mut lexer = PerlLexer::new(input);

    println!("=== Tokenizing package variable heredoc ===");
    while let Some(token) = lexer.next_token() {
        println!("Token: {:?}", token);
        if matches!(&token.token_type, TokenType::Error(msg) if msg.contains("heredoc")) {
            println!("  ^ Heredoc error detected");
        }
    }
}
