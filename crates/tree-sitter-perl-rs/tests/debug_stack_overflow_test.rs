//! Test case to reproduce stack overflow in debug builds
//! This creates deeply nested Perl structures to identify recursion issues

#[cfg(feature = "pure-rust")]
use tree_sitter_perl::PureRustPerlParser;

#[test]
#[cfg(feature = "pure-rust")]
#[ignore = "This test is designed to fail in debug builds to demonstrate stack overflow"]
fn test_deep_nested_expression() {
    // Create a deeply nested arithmetic expression
    // Each level adds parentheses: (((((1)))))
    let depth = 1500;
    let mut expr = "1".to_string();
    for _ in 0..depth {
        expr = format!("({})", expr);
    }

    println!("Testing expression with depth: {}", depth);
    println!("Expression length: {} bytes", expr.len());

    // This should overflow in debug builds
    let mut parser = PureRustPerlParser::new();
    let result = parser.parse(&expr);

    match result {
        Ok(ast) => {
            println!("Successfully parsed!");
            // Should not reach here in debug builds
        }
        Err(e) => {
            println!("Parse error: {:?}", e);
        }
    }
}

#[test]
#[cfg(feature = "pure-rust")]
#[ignore = "This test is designed to fail in debug builds to demonstrate stack overflow"]
fn test_deep_nested_blocks() {
    // Create deeply nested blocks: { { { { code } } } }
    let depth = 1000;
    let mut code = "print 'deep';".to_string();
    for _ in 0..depth {
        code = format!("{{ {} }}", code);
    }

    println!("Testing nested blocks with depth: {}", depth);

    let mut parser = PureRustPerlParser::new();
    let result = parser.parse(&code);

    match result {
        Ok(ast) => {
            println!("Successfully parsed!");
        }
        Err(e) => {
            println!("Parse error: {:?}", e);
        }
    }
}

#[test]
#[cfg(feature = "pure-rust")]
#[ignore = "This test is designed to fail in debug builds to demonstrate stack overflow"]
fn test_deep_nested_arrays() {
    // Create deeply nested array refs: [[[[1]]]]
    let depth = 1200;
    let mut expr = "42".to_string();
    for _ in 0..depth {
        expr = format!("[{}]", expr);
    }

    println!("Testing nested arrays with depth: {}", depth);

    let mut parser = PureRustPerlParser::new();
    let result = parser.parse(&expr);

    match result {
        Ok(ast) => {
            println!("Successfully parsed!");
        }
        Err(e) => {
            println!("Parse error: {:?}", e);
        }
    }
}

#[test]
#[cfg(feature = "pure-rust")]
#[ignore = "This test is designed to fail in debug builds to demonstrate stack overflow"]
fn test_deep_method_chain() {
    // Create a deep method chain: $obj->method1()->method2()->...
    let depth = 800;
    let mut expr = "$obj".to_string();
    for i in 0..depth {
        expr = format!("{}->method{}()", expr, i);
    }

    println!("Testing method chain with depth: {}", depth);

    let mut parser = PureRustPerlParser::new();
    let result = parser.parse(&expr);

    match result {
        Ok(ast) => {
            println!("Successfully parsed!");
        }
        Err(e) => {
            println!("Parse error: {:?}", e);
        }
    }
}

// Helper to run a specific test with custom stack trace
#[cfg(test)]
mod helpers {
    use std::env;

    pub fn run_with_backtrace<F: FnOnce()>(test_fn: F) {
        unsafe {
            env::set_var("RUST_BACKTRACE", "1");
        }
        test_fn();
    }
}
