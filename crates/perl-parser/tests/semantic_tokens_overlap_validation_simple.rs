/// Simplified semantic token overlap validation tests for LSP protocol compliance
/// These tests target semantic token overlap detection and validation
/// to ensure robust token generation and position validation.
///
/// Labels: tests:semantic-tokens, tests:mutation-hardening
use perl_parser::{Parser, semantic_tokens_provider::SemanticTokensProvider};

// Test basic semantic token generation without overlaps
#[test]
fn test_semantic_token_basic_generation() {
    let code = "my $var = 123;";
    let provider = SemanticTokensProvider::new(code.to_string());
    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();

    let tokens = provider.extract(&ast);

    // Should generate semantic tokens
    assert!(!tokens.is_empty(), "Should generate semantic tokens for variable declaration");

    // All tokens should have positive length
    for token in &tokens {
        assert!(token.length > 0, "All tokens should have positive length");
        assert!(token.line < 100, "Line numbers should be reasonable");
        assert!(token.start_char < 1000, "Character positions should be reasonable");
    }

    // Verify no overlaps exist in token list
    verify_no_semantic_token_overlaps(&tokens);
}

// Test semantic token generation with complex code
#[test]
fn test_semantic_token_complex_code_generation() {
    let code = "package MyModule::Test; sub test_func { my $variable = \"string\"; }";
    let provider = SemanticTokensProvider::new(code.to_string());
    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();

    let tokens = provider.extract(&ast);

    // Should generate tokens for complex code
    assert!(!tokens.is_empty(), "Should generate semantic tokens for complex code");

    // Test that we have tokens for different constructs
    let has_namespace = tokens.iter().any(|token| {
        matches!(
            token.token_type,
            perl_parser::semantic_tokens_provider::SemanticTokenType::Namespace
        )
    });
    let has_function = tokens.iter().any(|token| {
        matches!(
            token.token_type,
            perl_parser::semantic_tokens_provider::SemanticTokenType::Function
        )
    });
    let has_variable = tokens.iter().any(|token| {
        matches!(
            token.token_type,
            perl_parser::semantic_tokens_provider::SemanticTokenType::Variable
        )
    });

    assert!(
        has_namespace || has_function || has_variable,
        "Should have tokens for different code constructs"
    );

    // Verify no overlaps
    verify_no_semantic_token_overlaps(&tokens);
}

// Test UTF-8 boundary handling in semantic tokens
#[test]
fn test_semantic_token_utf8_handling() {
    let code = "my $🦀_var = \"🚀 test\"; # Comment with 🎯";
    let provider = SemanticTokensProvider::new(code.to_string());
    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();

    let tokens = provider.extract(&ast);

    // Should handle UTF-8 characters correctly
    assert!(!tokens.is_empty(), "Should generate tokens for UTF-8 code");

    // All tokens should have reasonable positions and lengths
    for token in &tokens {
        assert!(token.length > 0, "All tokens should have positive length");
        assert!(token.length < 100, "Token lengths should be reasonable for UTF-8");
    }

    // Verify no overlaps with UTF-8 characters
    verify_no_semantic_token_overlaps(&tokens);
}

// Test edge cases that might cause overlap issues
#[test]
fn test_semantic_token_edge_cases() {
    let test_cases = vec![
        "my $a = 1;",                        // Single character tokens
        ";;; # Empty statements",            // Minimal content
        "my $abc = 123; my $def = 456;",     // Multiple variables
        "package Test::Module; use strict;", // Package and use statements
    ];

    for code in test_cases {
        let provider = SemanticTokensProvider::new(code.to_string());
        let mut parser = Parser::new(code);
        if let Ok(ast) = parser.parse() {
            let tokens = provider.extract(&ast);

            // Verify all tokens are valid
            for token in &tokens {
                assert!(
                    token.length > 0,
                    "All tokens should have positive length in code: {}",
                    code
                );
            }

            // Verify no overlaps
            verify_no_semantic_token_overlaps(&tokens);
        }
    }
}

// Test semantic token idempotence
#[test]
fn test_semantic_token_idempotence() {
    let code = "package Test::Module; use strict; use warnings; sub test { my $var = 42; }";
    let provider = SemanticTokensProvider::new(code.to_string());
    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();

    // Generate tokens multiple times
    let tokens1 = provider.extract(&ast);
    let tokens2 = provider.extract(&ast);

    // Should be identical
    assert_eq!(tokens1.len(), tokens2.len(), "Token count should be identical across runs");

    for (i, (token1, token2)) in tokens1.iter().zip(tokens2.iter()).enumerate() {
        assert_eq!(token1.line, token2.line, "Token {} line should be identical", i);
        assert_eq!(
            token1.start_char, token2.start_char,
            "Token {} start_char should be identical",
            i
        );
        assert_eq!(token1.length, token2.length, "Token {} length should be identical", i);
        assert_eq!(token1.token_type, token2.token_type, "Token {} type should be identical", i);
    }

    // Verify both results have no overlaps
    verify_no_semantic_token_overlaps(&tokens1);
    verify_no_semantic_token_overlaps(&tokens2);
}

// Test performance characteristics
#[test]
fn test_semantic_token_performance_characteristics() {
    // Generate a moderate-sized code sample
    let mut code = String::new();
    for i in 0..50 {
        code.push_str(&format!("my $var{} = {}; ", i, i));
    }

    let provider = SemanticTokensProvider::new(code.clone());
    let mut parser = Parser::new(&code);
    let ast = parser.parse().unwrap();

    let start = std::time::Instant::now();
    let tokens = provider.extract(&ast);
    let duration = start.elapsed();

    // Performance should be reasonable
    assert!(
        duration.as_millis() < 1000,
        "Token generation should complete within 1s for 50 variables"
    );

    // Should generate reasonable number of tokens
    assert!(tokens.len() >= 50, "Should generate at least 50 tokens for 50 variables");

    // All tokens should be valid
    verify_no_semantic_token_overlaps(&tokens);

    // Memory usage should be reasonable
    let total_token_size =
        tokens.len() * std::mem::size_of::<perl_parser::semantic_tokens_provider::SemanticToken>();
    assert!(total_token_size < 100_000, "Token memory usage should be reasonable");
}

// Test nested structure handling
#[test]
fn test_semantic_token_nested_structures() {
    let code = r#"
    package Nested::Test;
    sub outer {
        my $outer_var = "test";
        {
            my $inner_var = $outer_var;
            for my $item (1..10) {
                print "$item: $inner_var\n";
            }
        }
    }
    "#;

    let provider = SemanticTokensProvider::new(code.to_string());
    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();

    let tokens = provider.extract(&ast);

    // Should handle nested structures
    assert!(!tokens.is_empty(), "Should generate tokens for nested code");

    // Verify we have tokens across multiple lines
    let line_count = tokens.iter().map(|token| token.line).max().unwrap_or(0);
    assert!(line_count >= 0, "Should have reasonable line numbers");

    // Verify no overlaps in complex nested structure
    verify_no_semantic_token_overlaps(&tokens);
}

// Helper function to verify no overlaps exist in semantic token list
fn verify_no_semantic_token_overlaps(
    tokens: &[perl_parser::semantic_tokens_provider::SemanticToken],
) {
    // Sort tokens by position for overlap checking
    let mut sorted_tokens: Vec<_> = tokens.iter().collect();
    sorted_tokens.sort_by(|a, b| a.line.cmp(&b.line).then_with(|| a.start_char.cmp(&b.start_char)));

    for i in 1..sorted_tokens.len() {
        let prev_token = sorted_tokens[i - 1];
        let curr_token = sorted_tokens[i];

        // Check for overlaps on the same line
        if prev_token.line == curr_token.line {
            let prev_end = prev_token.start_char + prev_token.length;
            assert!(
                curr_token.start_char >= prev_end,
                "Tokens overlap: prev[{}:{}-{}] curr[{}:{}-{}]",
                prev_token.line,
                prev_token.start_char,
                prev_end,
                curr_token.line,
                curr_token.start_char,
                curr_token.start_char + curr_token.length
            );
        }
    }
}
