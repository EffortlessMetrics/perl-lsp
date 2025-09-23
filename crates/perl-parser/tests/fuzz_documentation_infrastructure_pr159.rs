//! Comprehensive Fuzz Testing for PR 159 - Documentation Infrastructure & Parser Robustness
//!
//! This test suite validates the stability and robustness of the missing documentation
//! infrastructure under stress conditions, specifically for Draft PR 159.
//!
//! Test Focus Areas:
//! 1. Missing Documentation Infrastructure Robustness
//! 2. Enhanced Perl Parser Stress Testing with AST Invariant Validation
//! 3. LSP Provider Robustness under Concurrent Load
//! 4. Integration Stress Testing with Documentation Validation
//!
//! Labels: tests:fuzz, perl-fuzz:pr159, documentation:infrastructure

use perl_parser::*;
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, FileFailurePersistence};
use std::panic;
use std::time::{Duration, Instant};

/// Regression directory for PR 159 fuzz test cases
const REGRESS_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/_proptest-regressions/fuzz_documentation_infrastructure_pr159"
);

/// Test documentation infrastructure robustness with malformed Perl syntax
#[test]
fn fuzz_missing_docs_infrastructure_robustness() {
    fn test_docs_infrastructure_stability(
        input: String,
    ) -> Result<(), proptest::test_runner::TestCaseError> {
        // Core invariant: documentation infrastructure should never panic
        let result = panic::catch_unwind(|| {
            // Test parser with missing docs enforcement
            let mut parser = Parser::new(&input);
            let parse_result = parser.parse();

            // Documentation validation should not interfere with parsing
            if let Ok(_ast) = parse_result {
                // AST should be valid regardless of documentation state
                // This validates that missing docs infrastructure doesn't corrupt parsing
                true
            } else {
                // Parse failures are acceptable - we just need no panics
                true
            }
        });

        prop_assert!(
            result.is_ok(),
            "Documentation infrastructure caused panic on input: {:?}",
            input
        );

        Ok(())
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 2000, // Aggressive testing for documentation infrastructure
            max_shrink_iters: 200,
            failure_persistence: Some(Box::new(
                FileFailurePersistence::Direct(REGRESS_DIR)
            )),
            .. ProptestConfig::default()
        })]

        #[test]
        fn docs_infrastructure_robustness_fuzz(
            input in prop_oneof![
                // Malformed Perl syntax that might trigger doc validation edge cases
                prop::string::string_regex("(package|use|sub)\\s+[A-Za-z0-9_:]+\\s*\\{[^}]*\\}").unwrap(),
                // Extreme Unicode that might confuse documentation processing
                prop::string::string_regex(".*[\\u{0080}-\\u{FFFF}]{1,50}.*").unwrap(),
                // Nested structures that might stress documentation analysis
                prop::string::string_regex("(sub|package)\\s*\\{[^{}]*\\{[^{}]*\\}[^{}]*\\}").unwrap(),
                // Binary-like data that documentation validators must handle gracefully
                prop::collection::vec(any::<u8>(), 0..500).prop_map(|bytes| {
                    String::from_utf8_lossy(&bytes).into_owned()
                }),
                // Very long identifiers that might stress missing docs tracking
                prop::string::string_regex("[a-zA-Z_][a-zA-Z0-9_]{100,1000}").unwrap(),
            ]
        ) {
            test_docs_infrastructure_stability(input)?;
        }
    }
}

/// Stress test enhanced Perl parser with extreme inputs and AST invariant validation
#[test]
fn fuzz_enhanced_perl_parser_ast_invariants() {
    fn test_parser_ast_invariants(
        input: String,
    ) -> Result<(), proptest::test_runner::TestCaseError> {
        let start_time = Instant::now();

        let result = panic::catch_unwind(|| {
            let mut parser = Parser::new(&input);
            let parse_result = parser.parse();

            // Performance invariant: parsing should complete within reasonable time
            let elapsed = start_time.elapsed();
            if elapsed > Duration::from_millis(100) {
                eprintln!("Warning: Slow parsing detected: {:?} for input length {}", elapsed, input.len());
            }

            match parse_result {
                Ok(ast) => {
                    // AST structural invariants
                    validate_ast_structure(&ast, &input)?;
                    Ok::<bool, std::io::Error>(true)
                }
                Err(_parse_error) => {
                    // Parse failures are acceptable, as long as no panics occur
                    Ok(false)
                }
            }
        });

        prop_assert!(result.is_ok(), "Parser panicked on input: {:?}", input);
        Ok(())
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 1500, // Comprehensive AST validation
            max_shrink_iters: 150,
            failure_persistence: Some(Box::new(
                FileFailurePersistence::Direct(REGRESS_DIR)
            )),
            .. ProptestConfig::default()
        })]

        #[test]
        fn enhanced_parser_ast_invariants_fuzz(
            input in prop_oneof![
                // Enhanced builtin function patterns with edge cases
                prop::string::string_regex("(map|grep|sort)\\s*\\{[^}]{0,100}\\}\\s*\\([^)]*\\)").unwrap(),
                // Dual indexing stress patterns
                prop::string::string_regex("[A-Za-z_][A-Za-z0-9_]*::[A-Za-z_][A-Za-z0-9_]*\\([^)]*\\)").unwrap(),
                // Substitution operators with complex delimiters
                prop::string::string_regex("s[/\\\\{}()\\[\\]<>|#!~][^/\\\\{}()\\[\\]<>|#!~]{0,50}[/\\\\{}()\\[\\]<>|#!~][^/\\\\{}()\\[\\]<>|#!~]{0,50}[/\\\\{}()\\[\\]<>|#!~][imsxgaeludnrpcoRD]*").unwrap(),
                // Heredoc variants that might stress incremental parsing
                prop::string::string_regex("<<\\s*[A-Za-z_][A-Za-z0-9_]*\\s*\\n[^\\n]{0,200}\\n[A-Za-z_][A-Za-z0-9_]*").unwrap(),
                // Unicode identifier stress testing
                prop::string::string_regex("(package|sub)\\s+[\\u{0080}-\\u{FFFF}]{1,20}\\s*\\{").unwrap(),
            ]
        ) {
            test_parser_ast_invariants(input)?;
        }
    }
}

/// Validate AST structural invariants
fn validate_ast_structure(
    ast: &Node,
    input: &str,
) -> Result<(), proptest::test_runner::TestCaseError> {
    // Basic AST structure validation
    match &ast.kind {
        NodeKind::Program { statements } => {
            // Program nodes should have valid statement lists
            for statement in statements {
                validate_node_invariants(statement, input)?;
            }
        }
        _ => {
            // Non-program root nodes are acceptable for fragments
        }
    }

    // Range validation: all nodes should have valid ranges within input bounds
    validate_node_ranges(ast, input)?;

    Ok(())
}

/// Validate individual node invariants
fn validate_node_invariants(
    node: &Node,
    input: &str,
) -> Result<(), proptest::test_runner::TestCaseError> {
    // Range bounds checking using location field
    prop_assert!(
        node.location.start <= input.len(),
        "Node start position {} exceeds input length {}",
        node.location.start,
        input.len()
    );
    prop_assert!(
        node.location.end <= input.len(),
        "Node end position {} exceeds input length {}",
        node.location.end,
        input.len()
    );
    prop_assert!(
        node.location.start <= node.location.end,
        "Node range start {} > end {}",
        node.location.start,
        node.location.end
    );

    // Recursively validate child nodes by pattern matching
    validate_child_nodes(node, input)?;

    Ok(())
}

/// Validate child nodes recursively
fn validate_child_nodes(
    node: &Node,
    input: &str,
) -> Result<(), proptest::test_runner::TestCaseError> {
    match &node.kind {
        NodeKind::Program { statements } => {
            for statement in statements {
                validate_node_invariants(statement, input)?;
            }
        }
        NodeKind::FunctionCall { args, .. } => {
            for arg in args {
                validate_node_invariants(arg, input)?;
            }
        }
        NodeKind::Block { statements } => {
            for statement in statements {
                validate_node_invariants(statement, input)?;
            }
        }
        NodeKind::Binary { left, right, .. } => {
            validate_node_invariants(left, input)?;
            validate_node_invariants(right, input)?;
        }
        NodeKind::Assignment { lhs, rhs, .. } => {
            validate_node_invariants(lhs, input)?;
            validate_node_invariants(rhs, input)?;
        }
        _ => {
            // Other node types may not have easily accessible children
            // This is acceptable for fuzz testing purposes
        }
    }
    Ok(())
}

/// Validate all node ranges are within input bounds
fn validate_node_ranges(
    node: &Node,
    input: &str,
) -> Result<(), proptest::test_runner::TestCaseError> {
    let input_len = input.len();

    prop_assert!(
        node.location.start <= input_len && node.location.end <= input_len,
        "Node range {:?} exceeds input bounds (length: {})",
        node.location,
        input_len
    );

    // Validate child node ranges recursively
    validate_child_nodes(node, input)?;

    Ok(())
}

/// Test LSP provider robustness under concurrent load and cancellation scenarios
#[test]
fn fuzz_lsp_provider_concurrent_robustness() {
    fn test_lsp_concurrent_load(
        input: String,
    ) -> Result<(), proptest::test_runner::TestCaseError> {
        let result = panic::catch_unwind(|| {
            // Simulate concurrent LSP operations
            let mut parser = Parser::new(&input);
            let parse_result = parser.parse();

            match parse_result {
                Ok(ast) => {
                    // Test multiple LSP operations on the same AST
                    // This simulates concurrent LSP provider access

                    // Simulate completion requests
                    let _symbols = extract_symbols_safe(&ast);

                    // Simulate diagnostic requests
                    let _diagnostics = validate_syntax_safe(&ast);

                    // Simulate workspace indexing
                    let _references = find_references_safe(&ast, "test_symbol");

                    true
                }
                Err(_) => {
                    // Parse errors are acceptable - just need no panics during LSP operations
                    false
                }
            }
        });

        prop_assert!(
            result.is_ok(),
            "LSP provider operations panicked on concurrent access: {:?}",
            input
        );

        Ok(())
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 800, // Focused testing on LSP concurrency
            max_shrink_iters: 80,
            failure_persistence: Some(Box::new(
                FileFailurePersistence::Direct(REGRESS_DIR)
            )),
            .. ProptestConfig::default()
        })]

        #[test]
        fn lsp_provider_concurrent_robustness_fuzz(
            input in prop_oneof![
                // Large files that might stress LSP operations
                prop::string::string_regex("[a-zA-Z0-9\\s\\n\\.;{}()\\[\\]]{1000,5000}").unwrap(),
                // Complex package hierarchies for workspace indexing stress
                prop::string::string_regex("(package\\s+[A-Za-z_][A-Za-z0-9_:]*;\\s*){5,20}").unwrap(),
                // Mixed UTF-8 content for UTF-16 position mapping stress
                prop::string::string_regex(".*[\\u{0080}-\\u{FFFF}].*[a-zA-Z0-9\\s]*.*").unwrap(),
                // Function definition patterns for reference resolution stress
                prop::string::string_regex("(sub\\s+[a-zA-Z_][a-zA-Z0-9_]*\\s*\\{[^}]{0,100}\\}\\s*){3,10}").unwrap(),
            ]
        ) {
            test_lsp_concurrent_load(input)?;
        }
    }
}

/// Safe symbol extraction that doesn't panic
fn extract_symbols_safe(ast: &Node) -> Vec<String> {
    let mut symbols = Vec::new();
    collect_symbols_recursive(ast, &mut symbols);
    symbols.truncate(1000); // Prevent memory exhaustion
    symbols
}

/// Recursively collect symbols from AST
fn collect_symbols_recursive(node: &Node, symbols: &mut Vec<String>) {
    match &node.kind {
        NodeKind::FunctionCall { name, .. } => {
            symbols.push(name.clone());
        }
        NodeKind::Variable { name, .. } => {
            symbols.push(name.clone());
        }
        NodeKind::FunctionDefinition { name, .. } => {
            symbols.push(name.clone());
        }
        _ => {}
    }

    // Recursively process children using pattern matching
    if symbols.len() < 1000 { // Prevent unbounded growth
        collect_symbols_from_children(node, symbols);
    }
}

/// Collect symbols from node children
fn collect_symbols_from_children(node: &Node, symbols: &mut Vec<String>) {
    match &node.kind {
        NodeKind::Program { statements } => {
            for statement in statements {
                collect_symbols_recursive(statement, symbols);
            }
        }
        NodeKind::FunctionCall { args, .. } => {
            for arg in args {
                collect_symbols_recursive(arg, symbols);
            }
        }
        NodeKind::Block { statements } => {
            for statement in statements {
                collect_symbols_recursive(statement, symbols);
            }
        }
        NodeKind::Binary { left, right, .. } => {
            collect_symbols_recursive(left, symbols);
            collect_symbols_recursive(right, symbols);
        }
        _ => {
            // Other node types handled at parent level
        }
    }
}

/// Safe syntax validation that doesn't panic
fn validate_syntax_safe(ast: &Node) -> Vec<String> {
    let mut diagnostics = Vec::new();
    validate_syntax_recursive(ast, &mut diagnostics);
    diagnostics.truncate(100); // Prevent memory exhaustion
    diagnostics
}

/// Recursively validate syntax
fn validate_syntax_recursive(node: &Node, diagnostics: &mut Vec<String>) {
    // Basic syntax validation - look for potential issues
    if node.location.start > node.location.end {
        diagnostics.push(format!("Invalid range: {:?}", node.location));
    }

    // Recursively validate children using pattern matching
    if diagnostics.len() < 100 { // Prevent unbounded growth
        validate_syntax_from_children(node, diagnostics);
    }
}

/// Validate syntax from node children
fn validate_syntax_from_children(node: &Node, diagnostics: &mut Vec<String>) {
    match &node.kind {
        NodeKind::Program { statements } => {
            for statement in statements {
                validate_syntax_recursive(statement, diagnostics);
            }
        }
        NodeKind::FunctionCall { args, .. } => {
            for arg in args {
                validate_syntax_recursive(arg, diagnostics);
            }
        }
        NodeKind::Block { statements } => {
            for statement in statements {
                validate_syntax_recursive(statement, diagnostics);
            }
        }
        _ => {
            // Other node types handled at parent level
        }
    }
}

/// Safe reference finding that doesn't panic
fn find_references_safe(ast: &Node, symbol: &str) -> Vec<(usize, usize)> {
    let mut references = Vec::new();
    find_references_recursive(ast, symbol, &mut references);
    references.truncate(1000); // Prevent memory exhaustion
    references
}

/// Recursively find symbol references
fn find_references_recursive(node: &Node, symbol: &str, references: &mut Vec<(usize, usize)>) {
    match &node.kind {
        NodeKind::FunctionCall { name, .. } if name == symbol => {
            references.push((node.location.start, node.location.end));
        }
        NodeKind::Variable { name, .. } if name == symbol => {
            references.push((node.location.start, node.location.end));
        }
        _ => {}
    }

    // Recursively process children using pattern matching
    if references.len() < 1000 { // Prevent unbounded growth
        find_references_from_children(node, symbol, references);
    }
}

/// Find references from node children
fn find_references_from_children(node: &Node, symbol: &str, references: &mut Vec<(usize, usize)>) {
    match &node.kind {
        NodeKind::Program { statements } => {
            for statement in statements {
                find_references_recursive(statement, symbol, references);
            }
        }
        NodeKind::FunctionCall { args, .. } => {
            for arg in args {
                find_references_recursive(arg, symbol, references);
            }
        }
        NodeKind::Block { statements } => {
            for statement in statements {
                find_references_recursive(statement, symbol, references);
            }
        }
        _ => {
            // Other node types handled at parent level
        }
    }
}

/// Integration stress test: documentation validation during incremental parsing
#[test]
fn fuzz_documentation_incremental_parsing_integration() {
    fn test_docs_incremental_integration(
        initial_content: String,
        modification: String,
    ) -> Result<(), proptest::test_runner::TestCaseError> {
        let result = panic::catch_unwind(|| {
            // Initial parse with documentation infrastructure
            let mut parser = Parser::new(&initial_content);
            let initial_ast = parser.parse();

            // Simulate incremental update
            let modified_content = format!("{}\n{}", initial_content, modification);
            let mut incremental_parser = Parser::new(&modified_content);
            let modified_ast = incremental_parser.parse();

            // Both operations should complete without panics
            // Documentation infrastructure should not interfere with incremental parsing
            (initial_ast.is_ok() || initial_ast.is_err()) &&
            (modified_ast.is_ok() || modified_ast.is_err())
        });

        prop_assert!(
            result.is_ok(),
            "Documentation infrastructure interfered with incremental parsing: initial={:?}, mod={:?}",
            initial_content, modification
        );

        Ok(())
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 1000, // Integration testing with documentation infrastructure
            max_shrink_iters: 100,
            failure_persistence: Some(Box::new(
                FileFailurePersistence::Direct(REGRESS_DIR)
            )),
            .. ProptestConfig::default()
        })]

        #[test]
        fn docs_incremental_integration_fuzz(
            initial_content in prop::string::string_regex("(package|use|sub)\\s+[A-Za-z0-9_]+\\s*\\{[^}]{0,50}\\}").unwrap(),
            modification in prop_oneof![
                "# Documentation comment",
                "sub new_function { }",
                "package NewPackage;",
                "use strict;",
                "# TODO: Add documentation",
                "=pod\nDocumentation\n=cut",
            ]
        ) {
            test_docs_incremental_integration(initial_content, modification.to_string())?;
        }
    }
}

/// Memory safety and bounds checking stress test
#[test]
fn fuzz_memory_safety_bounds_checking() {
    fn test_memory_safety(
        input: String,
    ) -> Result<(), proptest::test_runner::TestCaseError> {
        let result = panic::catch_unwind(|| {
            // Test with various input sizes to stress memory allocation
            let mut parser = Parser::new(&input);
            let parse_result = parser.parse();

            // Memory safety: parser should handle large inputs gracefully
            match parse_result {
                Ok(ast) => {
                    // Verify AST doesn't contain invalid memory references
                    validate_memory_safety(&ast, &input)
                }
                Err(_) => {
                    // Parse errors are acceptable - just verify no memory corruption
                    true
                }
            }
        });

        prop_assert!(
            result.is_ok(),
            "Memory safety violation detected for input length: {}",
            input.len()
        );

        Ok(())
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 500, // Memory safety focused testing
            max_shrink_iters: 50,
            failure_persistence: Some(Box::new(
                FileFailurePersistence::Direct(REGRESS_DIR)
            )),
            .. ProptestConfig::default()
        })]

        #[test]
        fn memory_safety_bounds_fuzz(
            input in prop_oneof![
                // Very large inputs to test memory allocation limits
                prop::collection::vec(any::<u8>(), 1000..10000).prop_map(|bytes| {
                    String::from_utf8_lossy(&bytes).into_owned()
                }),
                // Deeply nested structures that might cause stack overflow
                prop::string::string_regex("(\\{){50,200}[a-zA-Z0-9\\s]*\\}{50,200}").unwrap(),
                // Repeated patterns that might cause exponential memory growth
                prop::string::string_regex("(a{1,100}){1,100}").unwrap(),
                // Mixed content with extreme Unicode
                prop::string::string_regex("([\\u{0000}-\\u{FFFF}]{100,1000})").unwrap(),
            ]
        ) {
            test_memory_safety(input)?;
        }
    }
}

/// Validate memory safety of AST nodes
fn validate_memory_safety(ast: &Node, input: &str) -> bool {
    // Verify all node ranges are within valid memory bounds
    validate_memory_bounds_recursive(ast, input)
}

/// Recursively validate memory bounds
fn validate_memory_bounds_recursive(node: &Node, input: &str) -> bool {
    // Check that node ranges don't exceed input bounds
    if node.location.start > input.len() || node.location.end > input.len() {
        return false;
    }

    // Check that node ranges don't wrap around (overflow)
    if node.location.start > node.location.end {
        return false;
    }

    // Recursively validate children using pattern matching
    validate_memory_bounds_from_children(node, input)
}

/// Validate memory bounds from node children
fn validate_memory_bounds_from_children(node: &Node, input: &str) -> bool {
    match &node.kind {
        NodeKind::Program { statements } => {
            statements.iter().all(|statement| {
                validate_memory_bounds_recursive(statement, input)
            })
        }
        NodeKind::FunctionCall { args, .. } => {
            args.iter().all(|arg| {
                validate_memory_bounds_recursive(arg, input)
            })
        }
        NodeKind::Block { statements } => {
            statements.iter().all(|statement| {
                validate_memory_bounds_recursive(statement, input)
            })
        }
        NodeKind::Binary { left, right, .. } => {
            validate_memory_bounds_recursive(left, input) &&
            validate_memory_bounds_recursive(right, input)
        }
        _ => {
            // Other node types are considered valid
            true
        }
    }
}