//! Test to verify stacker fix for deep recursion

#[test]
#[cfg(feature = "pure-rust")]
fn test_stacker_with_deep_nesting() {
    use tree_sitter_perl::pure_rust_parser::parse_perl;
    
    // Test progressively deeper nesting
    let depths = [100, 500, 1000];
    
    for depth in depths {
        eprintln!("Testing depth: {}", depth);
        
        // Create deeply nested expression
        let mut expr = "42".to_string();
        for _ in 0..depth {
            expr = format!("({})", expr);
        }
        
        let result = parse_perl(&expr);
        assert!(result.is_ok(), "Failed at depth {}: {:?}", depth, result.err());
    }
}

#[test]
#[cfg(feature = "pure-rust")]
fn test_stacker_with_deep_blocks() {
    use tree_sitter_perl::pure_rust_parser::parse_perl;
    
    // Test with nested blocks
    let depth = 500;
    let mut code = "print 'test';".to_string();
    for _ in 0..depth {
        code = format!("{{ {} }}", code);
    }
    
    let result = parse_perl(&code);
    assert!(result.is_ok(), "Failed with nested blocks: {:?}", result.err());
}