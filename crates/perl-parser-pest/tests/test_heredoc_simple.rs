//! Simple test to debug heredoc recovery

use tree_sitter_perl::perl_lexer::PerlLexer;

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

#[test]
fn test_braced_dynamic_heredoc() {
    let input = r#"my $var = "END"; my $doc = <<${var};
content
END
"#;

    let mut lexer = PerlLexer::new(input);
    let mut count = 0;

    while let Some(_token) = lexer.next_token() {
        count += 1;
        if count > 100 {
            panic!("Too many tokens - likely infinite loop");
        }
    }
}

#[test]
fn test_concatenated_dynamic_heredoc() {
    let input = r#"my $base = "E"; my $doc = <<($base . "ND");
content
END
"#;

    let mut lexer = PerlLexer::new(input);
    let mut count = 0;

    while let Some(_token) = lexer.next_token() {
        count += 1;
        if count > 100 {
            panic!("Too many tokens - likely infinite loop");
        }
    }
}
