//! Simple test to debug heredoc recovery

use tree_sitter_perl::perl_lexer::{PerlLexer, TokenType};

#[test] 
fn test_simple_dynamic_heredoc() {
    let input = r#"my $doc = <<$var;
content
EOF
"#;
    
    let mut lexer = PerlLexer::new(input);
    let mut count = 0;
    
    while let Some(token) = lexer.next_token() {
        println!("Token {:?}", token);
        count += 1;
        if count > 100 {
            panic!("Too many tokens - likely infinite loop");
        }
    }
}