//! Tests for enhanced error recovery mechanisms

use super::*;
use perl_parser_core::engine::parser::enhanced_recovery::{RecoveryConfig, EnhancedErrorRecovery, ErrorContext};
use perl_tdd_support::must;

#[test]
fn test_enhanced_recovery_with_timeout() {
    // Test that parser respects timeout limits
    let config = RecoveryConfig {
        max_parse_time: std::time::Duration::from_millis(1), // Very short timeout
        ..Default::default()
    };
    
    let code = "my $x = ; " . &"print $x;\n".repeat(1000); // Large input
    let mut parser = Parser::new_with_recovery_config(code, config);
    
    let result = parser.parse();
    
    // Should fail due to timeout
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ParseError::RecursionLimit));
}

#[test]
fn test_enhanced_recovery_with_memory_limit() {
    // Test that parser respects memory limits
    let config = RecoveryConfig {
        max_ast_nodes: 10, // Very low limit
        ..Default::default()
    };
    
    let code = "my $x = 1; my $y = 2; my $z = 3;"; // Multiple statements
    let mut parser = Parser::new_with_recovery_config(code, config);
    
    let result = parser.parse();
    
    // Should fail due to memory limit
    assert!(result.is_err());
}

#[test]
fn test_context_aware_suggestions() {
    // Test that error messages include helpful suggestions
    let code = "my $x = ;"; // Missing expression
    let mut parser = Parser::new(code);
    
    let result = parser.parse();
    assert!(result.is_ok());
    
    let ast = must(result);
    if let NodeKind::Program { statements } = &ast.kind {
        if let NodeKind::Error { message, .. } = &statements[0].kind {
            // Should contain suggestion about missing expression
            assert!(message.contains("Suggestion:") || message.contains("Missing expression"));
        }
    }
}

#[test]
fn test_adaptive_recovery_unclosed_delimiter() {
    // Test adaptive recovery for unclosed delimiters
    let code = "if ($x) { print 1;"; // Missing closing brace
    let mut parser = Parser::new(code);
    
    let result = parser.parse();
    assert!(result.is_ok());
    
    let ast = must(result);
    // Should recover and create a valid AST structure
    assert!(matches!(ast.kind, NodeKind::Program { .. }));
}

#[test]
fn test_adaptive_recovery_unexpected_token() {
    // Test adaptive recovery for unexpected tokens
    let code = "my $x = ; ; ;"; // Multiple semicolons
    let mut parser = Parser::new(code);
    
    let result = parser.parse();
    assert!(result.is_ok());
    
    // Should recover and continue parsing
    let ast = must(result);
    if let NodeKind::Program { statements } = &ast.kind {
        // Should have at least one statement (the error might be skipped)
        assert!(!statements.is_empty());
    }
}

#[test]
fn test_enhanced_error_node_with_suggestions() {
    // Test that error nodes contain suggestions
    let code = "my $x = ;";
    let mut parser = Parser::new(code);
    
    let result = parser.parse();
    assert!(result.is_ok());
    
    let ast = must(result);
    if let NodeKind::Program { statements } = &ast.kind {
        if let NodeKind::Error { partial, .. } = &statements[0].kind {
            // Should have suggestions
            assert!(partial.is_some());
            if let Some(suggestions) = partial {
                assert!(!suggestions.is_empty());
            }
        }
    }
}

#[test]
fn test_recovery_with_heuristics() {
    // Test heuristic recovery for common mistakes
    let test_cases = vec![
        ("my $x =", "missing_expression"),
        ("if ($x) {", "unclosed_delimiter"),
        ("print $x", "statement_without_semicolon"),
        ("sub foo {", "unclosed_delimiter"),
    ];
    
    for (code, error_type) in test_cases {
        let mut parser = Parser::new(code);
        let result = parser.parse();
        assert!(result.is_ok(), "Failed to recover from: {}", code);
        
        // Should have errors recorded
        assert!(!parser.errors().is_empty(), "No errors recorded for: {}", code);
    }
}

#[test]
fn test_resource_monitoring() {
    // Test that resource monitoring works correctly
    let config = RecoveryConfig {
        max_ast_nodes: 100,
        max_memory_bytes: 100_000, // 100KB
        ..Default::default()
    };
    
    let code = "my $x = 1;\n".repeat(50); // 50 statements
    let mut parser = Parser::new_with_recovery_config(&code, config);
    
    let result = parser.parse();
    
    // Check that node tracking worked
    assert!(parser.recovery_state().node_count > 0);
    assert!(parser.recovery_state().memory_estimate > 0);
}

#[test]
fn test_enhanced_recovery_with_complex_constructs() {
    // Test recovery with complex Perl constructs
    let code = r#"
        package My::Package;
        
        sub my_sub {
            my $x = ;
            my %hash = (
                key1 => 'value1',
                key2 => 
            );
            
            if ($x) {
                print "Hello";
            }
            
            foreach my $item (@array) {
                process($item
            }
        }
    "#;
    
    let mut parser = Parser::new(code);
    let result = parser.parse();
    assert!(result.is_ok());
    
    // Should recover from multiple errors
    assert!(!parser.errors().is_empty());
    
    let ast = must(result);
    // Should still extract valid structure
    assert!(matches!(ast.kind, NodeKind::Program { .. }));
}

#[test]
fn test_error_context_analysis() {
    // Test that error context is properly analyzed
    let code = "my $x = ; if ($x) { print $x; }";
    let mut parser = Parser::new(code);
    
    let result = parser.parse();
    assert!(result.is_ok());
    
    // Should have context-aware recovery
    let errors = parser.errors();
    assert!(!errors.is_empty());
}

#[test]
fn test_recovery_performance_overhead() {
    // Test that enhanced recovery doesn't significantly impact performance
    let code = "my $x = 1; my $y = 2; my $z = 3;";
    let start = std::time::Instant::now();
    
    let mut parser = Parser::new(code);
    let result = parser.parse();
    let duration = start.elapsed();
    
    assert!(result.is_ok());
    // Should complete quickly (under 10ms for simple code)
    assert!(duration.as_millis() < 10);
}

#[test]
fn test_multiple_error_recovery_strategies() {
    // Test that multiple recovery strategies can be applied
    let code = r#"
        my $x = ;
        my %hash = (
            key1 => 'value1',
            key2 => 
        );
        sub foo { print "hello"
    "#;
    
    let mut parser = Parser::new(code);
    let result = parser.parse();
    assert!(result.is_ok());
    
    let ast = must(result);
    if let NodeKind::Program { statements } = &ast.kind {
        // Should have recovered from multiple errors
        assert!(statements.len() > 0);
        
        // Should have error nodes mixed with valid nodes
        let has_errors = statements.iter().any(|s| matches!(s.kind, NodeKind::Error { .. }));
        let has_valid = statements.iter().any(|s| !matches!(s.kind, NodeKind::Error { .. }));
        
        assert!(has_errors);
        assert!(has_valid);
    }
}

#[test]
fn test_recovery_with_unicode_and_special_chars() {
    // Test recovery with Unicode and special characters
    let code = r#"
        my $emoji = "🚀";
        my $unicode = "café";
        my $broken = ;
        my $special = "special\@chars";
    "#;
    
    let mut parser = Parser::new(code);
    let result = parser.parse();
    assert!(result.is_ok());
    
    // Should handle Unicode correctly while recovering from errors
    assert!(!parser.errors().is_empty());
}

#[test]
fn test_nested_structure_recovery() {
    // Test recovery in deeply nested structures
    let code = r#"
        if ($condition1) {
            if ($condition2) {
                if ($condition3) {
                    my $x = ;
                    for (my $i = 0; $i < 10; $i++) {
                        while ($j < 5) {
                            do_something($i, $j
                        }
                    }
                }
            }
        }
    "#;
    
    let mut parser = Parser::new(code);
    let result = parser.parse();
    assert!(result.is_ok());
    
    // Should recover from nested errors
    let ast = must(result);
    assert!(matches!(ast.kind, NodeKind::Program { .. }));
}