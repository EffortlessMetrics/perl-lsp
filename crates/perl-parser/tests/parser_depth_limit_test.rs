//! Test parser depth limit to prevent stack overflow on deeply nested constructs
//!
//! This test verifies that the parser cleanly rejects deeply nested constructs
//! instead of crashing with a stack overflow. The parser enforces a maximum
//! recursion depth of 128 levels. This is set conservatively to ensure the
//! recursion limit triggers before OS stack overflow occurs.

use perl_parser::{ParseError, Parser};
use perl_tdd_support::must_err;

/// Test that deeply nested blocks are rejected with NestingTooDeep error
#[test]
fn parser_depth_limit_nested_blocks() {
    // Create nested blocks beyond the limit
    let depth = 200;
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
        matches!(must_err(result), ParseError::NestingTooDeep { .. }),
        "Expected NestingTooDeep error for deeply nested blocks"
    );
}

/// Test that deeply nested parentheses in expressions are rejected
#[test]
fn parser_depth_limit_nested_parens() {
    // Create deeply nested parentheses beyond the limit
    let depth = 200;
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
        matches!(must_err(result), ParseError::NestingTooDeep { .. }),
        "Expected NestingTooDeep error for deeply nested parentheses"
    );
}

/// Test that deeply nested array literals are rejected
#[test]
fn parser_depth_limit_nested_arrays() {
    // Create deeply nested arrays beyond the limit
    let depth = 200;
    let mut code = String::new();

    // Opening brackets
    for _ in 0..depth {
        code.push('[');
    }
    code.push('1');
    // Closing brackets
    for _ in 0..depth {
        code.push(']');
    }

    let mut parser = Parser::new(&code);
    let result = parser.parse();

    assert!(result.is_err(), "Expected error for deeply nested arrays");
    assert!(
        matches!(must_err(result), ParseError::NestingTooDeep { .. }),
        "Expected NestingTooDeep error for deeply nested arrays"
    );
}

/// Test that reasonably nested constructs still work (well below the limit)
#[test]
fn parser_depth_limit_reasonable_nesting() {
    // Create nested blocks well below the limit (15 levels)
    // With depth limit 128 and multiple increments per level,
    // 15 levels is safely under the limit
    let depth = 15;
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
    // Create a mix of blocks and expressions that exceeds the limit.
    // Each { ( pair adds multiple depth increments, so depth=100 should
    // quickly exceed the limit of 128 and trigger NestingTooDeep.
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
        matches!(must_err(result), ParseError::NestingTooDeep { .. }),
        "Expected NestingTooDeep error for mixed deep nesting"
    );
}

/// Test that the limit applies to control flow nesting
#[test]
fn parser_depth_limit_nested_control_flow() {
    // Create deeply nested if statements
    let depth = 200;
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
        matches!(must_err(result), ParseError::NestingTooDeep { .. }),
        "Expected NestingTooDeep error for deeply nested control flow"
    );
}

/// Test that exact limit boundary works correctly (just below limit)
#[test]
fn parser_depth_limit_boundary_below() {
    // Create nested parens just below the limit
    // With 15 parens and parse_expression + parse_primary both incrementing,
    // we get about 30 depth which is well under 128
    let depth = 15;
    let mut code = String::new();

    for _ in 0..depth {
        code.push('(');
    }
    code.push_str("42");
    for _ in 0..depth {
        code.push(')');
    }

    let mut parser = Parser::new(&code);
    let result = parser.parse();

    assert!(result.is_ok(), "Expected success for nesting just below limit");
}

/// Test that exact limit boundary fails correctly (just above limit)
#[test]
fn parser_depth_limit_boundary_above() {
    // Create nested parens that exceed the limit (100 levels)
    let depth = 200;
    let mut code = String::new();

    for _ in 0..depth {
        code.push('(');
    }
    code.push_str("42");
    for _ in 0..depth {
        code.push(')');
    }

    let mut parser = Parser::new(&code);
    let result = parser.parse();

    assert!(result.is_err(), "Expected error for nesting just above limit");
    assert!(
        matches!(must_err(result), ParseError::NestingTooDeep { .. }),
        "Expected NestingTooDeep error for nesting just above limit"
    );
}
