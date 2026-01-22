//! Test parser depth limit to prevent stack overflow on deeply nested constructs
//!
//! This test verifies that the parser cleanly rejects deeply nested constructs
//! instead of crashing with a stack overflow. The parser enforces a maximum
//! recursion depth of 256 levels.

use perl_parser::{ParseError, Parser};

/// Test that deeply nested blocks are rejected with RecursionLimit error
#[test]
fn parser_depth_limit_nested_blocks() {
    // Create nested blocks beyond the limit (256 + some margin)
    let depth = 300;
    let mut code = String::new();

    // Opening braces
    for _ in 0..depth {
        code.push_str("{ ");
    }

    // Closing braces
    for _ in 0..depth {
        code.push_str("} ");
    }

    let mut parser = Parser::new(&code);
    let result = parser.parse();

    assert!(result.is_err(), "Expected error for deeply nested blocks");
    assert!(
        matches!(result.unwrap_err(), ParseError::RecursionLimit),
        "Expected RecursionLimit error for deeply nested blocks"
    );
}

/// Test that deeply nested parentheses in expressions are rejected
#[test]
fn parser_depth_limit_nested_parens() {
    // Create deeply nested parentheses beyond the limit
    let depth = 300;
    let mut code = String::new();

    // Opening parens
    for _ in 0..depth {
        code.push('(');
    }
    code.push_str("42");
    // Closing parens
    for _ in 0..depth {
        code.push(')');
    }

    let mut parser = Parser::new(&code);
    let result = parser.parse();

    assert!(result.is_err(), "Expected error for deeply nested parentheses");
    assert!(
        matches!(result.unwrap_err(), ParseError::RecursionLimit),
        "Expected RecursionLimit error for deeply nested parentheses"
    );
}

/// Test that deeply nested array literals are rejected
#[test]
fn parser_depth_limit_nested_arrays() {
    // Create deeply nested arrays beyond the limit
    let depth = 300;
    let mut code = String::new();

    // Opening brackets
    for _ in 0..depth {
        code.push('[');
    }
    code.push_str("1");
    // Closing brackets
    for _ in 0..depth {
        code.push(']');
    }

    let mut parser = Parser::new(&code);
    let result = parser.parse();

    assert!(result.is_err(), "Expected error for deeply nested arrays");
    assert!(
        matches!(result.unwrap_err(), ParseError::RecursionLimit),
        "Expected RecursionLimit error for deeply nested arrays"
    );
}

/// Test that deeply nested subroutine calls are rejected
#[test]
fn parser_depth_limit_nested_calls() {
    // Create deeply nested function calls beyond the limit
    let depth = 300;
    let mut code = String::new();

    for i in 0..depth {
        code.push_str(&format!("foo{}(", i));
    }
    code.push_str("42");
    for _ in 0..depth {
        code.push(')');
    }

    let mut parser = Parser::new(&code);
    let result = parser.parse();

    assert!(result.is_err(), "Expected error for deeply nested calls");
    assert!(
        matches!(result.unwrap_err(), ParseError::RecursionLimit),
        "Expected RecursionLimit error for deeply nested calls"
    );
}

/// Test that reasonably nested constructs still work (well below the limit)
#[test]
fn parser_depth_limit_reasonable_nesting() {
    // Create nested blocks well below the limit (50 levels)
    let depth = 50;
    let mut code = String::new();

    // Opening braces
    for _ in 0..depth {
        code.push_str("{ ");
    }

    // Closing braces
    for _ in 0..depth {
        code.push_str("} ");
    }

    let mut parser = Parser::new(&code);
    let result = parser.parse();

    assert!(result.is_ok(), "Expected success for reasonable nesting depth");
}

/// Test mixed nesting types (blocks + expressions)
#[test]
fn parser_depth_limit_mixed_nesting() {
    // Create a mix of blocks and expressions that exceeds the limit
    // Use a more conservative depth to avoid OS stack limits while still exceeding parser limit
    // Simple nested blocks with parenthesized expressions
    let depth = 100;
    let mut code = String::new();

    for _ in 0..depth {
        code.push_str("{ (");
    }

    code.push_str("42");

    for _ in 0..depth {
        code.push_str(") }");
    }

    let mut parser = Parser::new(&code);
    let result = parser.parse();

    assert!(result.is_err(), "Expected error for mixed deep nesting");
    assert!(
        matches!(result.unwrap_err(), ParseError::RecursionLimit),
        "Expected RecursionLimit error for mixed deep nesting"
    );
}

/// Test that the limit applies to control flow nesting
#[test]
fn parser_depth_limit_nested_control_flow() {
    // Create deeply nested if statements
    let depth = 300;
    let mut code = String::new();

    for _ in 0..depth {
        code.push_str("if (1) { ");
    }

    code.push_str("my $x = 1;");

    for _ in 0..depth {
        code.push_str(" }");
    }

    let mut parser = Parser::new(&code);
    let result = parser.parse();

    assert!(result.is_err(), "Expected error for deeply nested control flow");
    assert!(
        matches!(result.unwrap_err(), ParseError::RecursionLimit),
        "Expected RecursionLimit error for deeply nested control flow"
    );
}

/// Test that exact limit boundary works correctly (just below limit)
#[test]
fn parser_depth_limit_boundary_below() {
    // Create nested blocks just below the limit (250 levels)
    let depth = 250;
    let mut code = String::new();

    // Opening braces
    for _ in 0..depth {
        code.push_str("{ ");
    }

    // Closing braces
    for _ in 0..depth {
        code.push_str("} ");
    }

    let mut parser = Parser::new(&code);
    let result = parser.parse();

    assert!(result.is_ok(), "Expected success for nesting just below limit");
}

/// Test that exact limit boundary fails correctly (just above limit)
#[test]
fn parser_depth_limit_boundary_above() {
    // Create nested blocks just above the limit (260 levels)
    let depth = 260;
    let mut code = String::new();

    // Opening braces
    for _ in 0..depth {
        code.push_str("{ ");
    }

    // Closing braces
    for _ in 0..depth {
        code.push_str("} ");
    }

    let mut parser = Parser::new(&code);
    let result = parser.parse();

    assert!(result.is_err(), "Expected error for nesting just above limit");
    assert!(
        matches!(result.unwrap_err(), ParseError::RecursionLimit),
        "Expected RecursionLimit error for nesting just above limit"
    );
}
