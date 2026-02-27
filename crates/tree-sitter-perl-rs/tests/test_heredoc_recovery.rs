//! Tests for dynamic heredoc delimiter recovery

use perl_tdd_support::must_some;
use tree_sitter_perl::heredoc_recovery::{HeredocRecovery, RecoveryConfig};
use tree_sitter_perl::perl_lexer::{PerlLexer, TokenType};

#[test]
fn test_static_delimiter_recovery() {
    // Test case where delimiter is assigned earlier
    let code = r#"
my $delimiter = "EOF";
my $text = <<$delimiter;
This is heredoc content
EOF
"#;

    let mut lexer = PerlLexer::new(code);
    let mut tokens = Vec::new();

    // Collect all tokens
    while let Some(token) = lexer.next_token() {
        if matches!(token.token_type, TokenType::EOF) {
            break;
        }
        tokens.push(token);
    }

    // Find the heredoc token
    let heredoc_token =
        must_some(tokens.iter().find(|t| matches!(t.token_type, TokenType::HeredocStart)));

    // Should have recovered to <<EOF
    assert_eq!(heredoc_token.text.as_ref(), "<<EOF");
}

#[test]
fn test_heuristic_delimiter_recovery() {
    // Test case with common delimiter patterns
    let code = r#"
my $end_delimiter = get_delimiter();
my $text = <<$end_delimiter;
Content here
END
"#;

    let mut lexer = PerlLexer::new(code);
    let mut found_heredoc = false;
    let mut found_error = false;

    while let Some(token) = lexer.next_token() {
        match &token.token_type {
            TokenType::HeredocStart => {
                found_heredoc = true;
                // Should recover with heuristic
                assert!(token.text.contains("<<"));
            }
            TokenType::Error(msg) => {
                if msg.contains("heredoc") {
                    found_error = true;
                    // Should provide alternatives
                    assert!(msg.contains("possible delimiters") || msg.contains("END"));
                }
            }
            TokenType::EOF => break,
            _ => {}
        }
    }

    // Should either recover or generate error
    assert!(found_heredoc || found_error);
}

#[test]
fn test_complex_expression_recovery() {
    // Test with method call
    let code = r#"
my $text = <<$config->delimiter();
Some content
EOF
"#;

    let mut lexer = PerlLexer::new(code);
    let mut found_token = false;

    while let Some(token) = lexer.next_token() {
        match &token.token_type {
            TokenType::HeredocStart | TokenType::Error(_) => {
                found_token = true;
                // Should handle method call expression
                assert!(token.text.contains("<<") || token.text.contains("delimiter"));
            }
            TokenType::EOF => break,
            _ => {}
        }
    }

    assert!(found_token);
}

#[test]
fn test_recovery_confidence_levels() {
    let mut recovery =
        HeredocRecovery::new(RecoveryConfig { confidence_threshold: 0.7, ..Default::default() });

    // Test various expressions and their recovery confidence
    let test_cases = vec![
        ("$eof", vec!["EOF"]),
        ("$end_marker", vec!["END", "EOF"]),
        ("$sql_delimiter", vec!["SQL"]),
        ("$unknown_var", vec!["EOF", "END", "EOT"]),
    ];

    for (expr, expected_alternatives) in test_cases {
        let input = format!("<<{}", expr);
        let result = recovery.recover_dynamic_heredoc(&input, 0, &[]);

        // Check alternatives are suggested
        for expected in expected_alternatives {
            assert!(
                result.alternatives.iter().any(|a| a.as_ref() == expected)
                    || result.delimiter.as_ref().map(|d| d.as_ref() == expected).unwrap_or(false),
                "Expected '{}' in alternatives for expression '{}'",
                expected,
                expr
            );
        }
    }
}

#[test]
fn test_error_token_generation() {
    // Test that proper error tokens are generated
    let code = "my $text = <<$unknown_runtime_var;";

    let mut lexer = PerlLexer::new(code);
    let mut error_found = false;

    while let Some(token) = lexer.next_token() {
        if let TokenType::Error(msg) = &token.token_type
            && msg.contains("heredoc")
        {
            error_found = true;
            // Should provide helpful error message
            assert!(msg.contains("Unresolved") || msg.contains("dynamic"));
            // Should suggest alternatives
            assert!(msg.contains("possible") || msg.contains("EOF"));
        }
        if matches!(token.token_type, TokenType::EOF) {
            break;
        }
    }

    assert!(error_found, "Should generate error token for unresolvable delimiter");
}

#[test]
fn test_cached_recovery() {
    // Test that repeated occurrences use cache
    let code = r#"
my $delim = "END";
my $text1 = <<$delim;
First heredoc
END

my $text2 = <<$delim;
Second heredoc
END
"#;

    let mut lexer = PerlLexer::new(code);
    let mut heredoc_count = 0;

    while let Some(token) = lexer.next_token() {
        if matches!(token.token_type, TokenType::HeredocStart) {
            heredoc_count += 1;
            // Both should recover successfully
            assert!(token.text.contains("<<"));
        }
        if matches!(token.token_type, TokenType::EOF) {
            break;
        }
    }

    assert_eq!(heredoc_count, 2, "Should find both heredocs");
}
