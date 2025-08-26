//! Simple test to debug heredoc recovery

use tree_sitter_perl::perl_lexer::{PerlLexer, TokenType};

#[test]
fn test_simple_dynamic_heredoc() {
    let input = r#"my $doc = <<$var;
content
EOF
"#;

    let mut lexer = PerlLexer::new(input);
    let mut tokens = Vec::new();

    while let Some(token) = lexer.next_token() {
        println!("Token {:?}", token);
        tokens.push(token);
        if tokens.len() > 100 {
            panic!("Too many tokens - likely infinite loop");
        }
    }

    // Verify we got reasonable number of tokens
    assert!(tokens.len() > 5, "Expected at least 5 tokens, got {}", tokens.len());
    assert!(tokens.len() < 50, "Expected fewer than 50 tokens, got {}", tokens.len());

    // Verify we have expected keywords
    assert!(
        tokens.iter().any(|t| matches!(&t.token_type, TokenType::Keyword(k) if k.as_ref() == "my")),
        "Expected 'my' keyword in tokens"
    );
}

#[test]
fn test_braced_dynamic_heredoc() {
    let input = r#"my $var = "END"; my $doc = <<${var};
content
END
"#;

    let mut lexer = PerlLexer::new(input);
    let mut tokens = Vec::new();

    while let Some(token) = lexer.next_token() {
        tokens.push(token);
        if tokens.len() > 100 {
            panic!("Too many tokens - likely infinite loop");
        }
    }

    // Verify we got reasonable number of tokens
    assert!(tokens.len() > 8, "Expected at least 8 tokens, got {}", tokens.len());
    assert!(tokens.len() < 50, "Expected fewer than 50 tokens, got {}", tokens.len());

    // Verify we have assignment and variable
    assert!(
        tokens
            .iter()
            .any(|t| matches!(&t.token_type, TokenType::Operator(op) if op.as_ref() == "=")),
        "Expected assignment operator in tokens"
    );
    assert!(
        tokens.iter().any(|t| matches!(&t.token_type, TokenType::Identifier(_))),
        "Expected identifier (variable name) in tokens"
    );
    assert!(
        tokens.iter().any(|t| matches!(&t.token_type, TokenType::StringLiteral)),
        "Expected string literal in tokens"
    );
}

#[test]
fn test_concatenated_dynamic_heredoc() {
    let input = r#"my $base = "E"; my $doc = <<($base . "ND");
content
END
"#;

    let mut lexer = PerlLexer::new(input);
    let mut tokens = Vec::new();

    while let Some(token) = lexer.next_token() {
        tokens.push(token);
        if tokens.len() > 100 {
            panic!("Too many tokens - likely infinite loop");
        }
    }

    // Verify we got reasonable number of tokens
    assert!(tokens.len() > 10, "Expected at least 10 tokens, got {}", tokens.len());
    assert!(tokens.len() < 50, "Expected fewer than 50 tokens, got {}", tokens.len());

    // Verify we have concatenation operator
    assert!(
        tokens
            .iter()
            .any(|t| matches!(&t.token_type, TokenType::Operator(op) if op.as_ref() == ".")),
        "Expected concatenation operator (.) in tokens"
    );
    assert!(
        tokens.iter().any(|t| matches!(&t.token_type, TokenType::StringLiteral)),
        "Expected string literal in tokens"
    );
}
