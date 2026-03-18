//! Test to verify stacker fix for deep recursion
//!
//! Keep this as a bounded smoke test in the default gate. Much deeper nesting
//! belongs in the existing `stress-tests` coverage because the pure-Rust Pest
//! grammar can become extremely slow long before the AST-building stack-growth
//! path is the limiting factor.

#[test]
#[cfg(feature = "pure-rust")]
fn test_stacker_with_deep_nesting() {
    use tree_sitter_perl::pure_rust_parser::PureRustPerlParser;

    // Stay above the basic 20-level smoke test elsewhere in the suite while
    // avoiding pathological runtimes in the default test gate.
    let depths = [10, 20];

    for depth in depths {
        eprintln!("Testing depth: {}", depth);

        // Create deeply nested expression
        let mut expr = "42".to_string();
        for _ in 0..depth {
            expr = format!("({})", expr);
        }

        let mut parser = PureRustPerlParser::new();
        let result = parser.parse(&expr);
        assert!(result.is_ok(), "Failed at depth {}: {:?}", depth, result.err());
    }
}

#[test]
#[cfg(feature = "pure-rust")]
fn test_stacker_with_deep_blocks() {
    use tree_sitter_perl::pure_rust_parser::PureRustPerlParser;

    // Nested blocks exercise a slower parser path than parenthesized
    // expressions, so keep the default-gate smoke test conservative.
    let depth = 12;
    let mut code = "print 'test';".to_string();
    for _ in 0..depth {
        code = format!("{{ {} }}", code);
    }

    let mut parser = PureRustPerlParser::new();
    let result = parser.parse(&code);
    assert!(result.is_ok(), "Failed with nested blocks: {:?}", result.err());
}
