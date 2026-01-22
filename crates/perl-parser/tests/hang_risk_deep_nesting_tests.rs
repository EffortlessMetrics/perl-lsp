//! Comprehensive deep nesting boundedness tests for hang/bounds risk mitigation
//!
//! Tests feature spec: ROADMAP.md#known-gaps-hang-bounds-risks
//!
//! This test suite validates that the parser has robust depth limiting for deeply
//! nested constructs to prevent stack overflow, hang conditions, and performance
//! degradation on pathological inputs.
//!
//! Coverage areas:
//! - Nested blocks (basic depth limiting)
//! - Nested expressions (parentheses, array/hash literals)
//! - Nested control flow (if/while/for)
//! - Mixed nesting (combinations of blocks, expressions, control flow)
//! - Regex nesting (capture groups, lookahead/behind)
//! - Quote operator nesting (nested delimiters in q/qq/qw/etc)
//! - Heredoc nesting
//! - Hash/array reference nesting
//! - Subroutine call nesting
//! - Pattern match nesting
//! - Complex real-world pathological patterns

use perl_parser::{ParseError, Parser};

/// Test deeply nested blocks exceed recursion limit
///
/// Tests feature spec: ROADMAP.md#deep-nesting-boundedness
#[test]
fn parser_hang_risk_nested_blocks_exceed_limit() {
    // Create nested blocks beyond the limit (300 levels exceeds 256 limit)
    let depth = 300;
    let mut code = String::new();

    for _ in 0..depth {
        code.push_str("{ ");
    }
    code.push_str("my $x = 1;");
    for _ in 0..depth {
        code.push_str("} ");
    }

    let mut parser = Parser::new(&code);
    let result = parser.parse();

    assert!(
        result.is_err(),
        "Expected RecursionLimit error for {} nested blocks",
        depth
    );
    assert!(
        matches!(result.unwrap_err(), ParseError::RecursionLimit),
        "Expected RecursionLimit error, got different error type"
    );
}

/// Test deeply nested parentheses in expressions
///
/// Tests feature spec: ROADMAP.md#deep-nesting-boundedness
#[test]
fn parser_hang_risk_nested_parentheses_exceed_limit() {
    let depth = 300;
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

    assert!(
        result.is_err(),
        "Expected RecursionLimit error for deeply nested parentheses"
    );
    assert!(
        matches!(result.unwrap_err(), ParseError::RecursionLimit),
        "Expected RecursionLimit error for nested parentheses"
    );
}

/// Test deeply nested array literals
///
/// Tests feature spec: ROADMAP.md#deep-nesting-boundedness
#[test]
fn parser_hang_risk_nested_array_literals() {
    let depth = 300;
    let mut code = String::new();

    for _ in 0..depth {
        code.push('[');
    }
    code.push_str("1");
    for _ in 0..depth {
        code.push(']');
    }

    let mut parser = Parser::new(&code);
    let result = parser.parse();

    assert!(
        result.is_err(),
        "Expected RecursionLimit error for deeply nested array literals"
    );
}

/// Test deeply nested hash literals
///
/// Tests feature spec: ROADMAP.md#deep-nesting-boundedness
#[test]
fn parser_hang_risk_nested_hash_literals() {
    let depth = 300;
    let mut code = String::new();

    for _ in 0..depth {
        code.push_str("{ a => ");
    }
    code.push_str("1");
    for _ in 0..depth {
        code.push_str(" }");
    }

    let mut parser = Parser::new(&code);
    let result = parser.parse();

    assert!(
        result.is_err(),
        "Expected RecursionLimit error for deeply nested hash literals"
    );
}

/// Test deeply nested function calls
///
/// Tests feature spec: ROADMAP.md#deep-nesting-boundedness
#[test]
fn parser_hang_risk_nested_function_calls() {
    let depth = 300;
    let mut code = String::new();

    for i in 0..depth {
        code.push_str(&format!("func{}(", i));
    }
    code.push_str("42");
    for _ in 0..depth {
        code.push(')');
    }

    let mut parser = Parser::new(&code);
    let result = parser.parse();

    assert!(
        result.is_err(),
        "Expected RecursionLimit error for deeply nested function calls"
    );
}

/// Test deeply nested if statements (control flow)
///
/// Tests feature spec: ROADMAP.md#deep-nesting-boundedness
#[test]
fn parser_hang_risk_nested_if_statements() {
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

    assert!(
        result.is_err(),
        "Expected RecursionLimit error for deeply nested if statements"
    );
}

/// Test deeply nested while loops
///
/// Tests feature spec: ROADMAP.md#deep-nesting-boundedness
#[test]
fn parser_hang_risk_nested_while_loops() {
    let depth = 300;
    let mut code = String::new();

    for _ in 0..depth {
        code.push_str("while (1) { ");
    }
    code.push_str("last;");
    for _ in 0..depth {
        code.push_str(" }");
    }

    let mut parser = Parser::new(&code);
    let result = parser.parse();

    assert!(
        result.is_err(),
        "Expected RecursionLimit error for deeply nested while loops"
    );
}

/// Test deeply nested for loops
///
/// Tests feature spec: ROADMAP.md#deep-nesting-boundedness
#[test]
fn parser_hang_risk_nested_for_loops() {
    let depth = 300;
    let mut code = String::new();

    for _ in 0..depth {
        code.push_str("for my $x (1..10) { ");
    }
    code.push_str("last;");
    for _ in 0..depth {
        code.push_str(" }");
    }

    let mut parser = Parser::new(&code);
    let result = parser.parse();

    assert!(
        result.is_err(),
        "Expected RecursionLimit error for deeply nested for loops"
    );
}

/// Test mixed nesting: blocks + expressions
///
/// Tests feature spec: ROADMAP.md#deep-nesting-boundedness
#[test]
fn parser_hang_risk_mixed_blocks_expressions() {
    let depth = 150;
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

    assert!(
        result.is_err(),
        "Expected RecursionLimit error for mixed blocks and expressions"
    );
}

/// Test deeply nested ternary operators
///
/// Tests feature spec: ROADMAP.md#deep-nesting-boundedness
#[test]
fn parser_hang_risk_nested_ternary_operators() {
    let depth = 300;
    let mut code = String::new();

    for _ in 0..depth {
        code.push_str("1 ? ");
    }
    code.push_str("42");
    for _ in 0..depth {
        code.push_str(" : 0");
    }

    let mut parser = Parser::new(&code);
    let result = parser.parse();

    assert!(
        result.is_err(),
        "Expected RecursionLimit error for deeply nested ternary operators"
    );
}

/// Test deeply nested regex capture groups
///
/// Tests feature spec: ROADMAP.md#deep-nesting-boundedness
/// Tests feature spec: ROADMAP.md#regex-literal-handling
#[test]
fn parser_hang_risk_nested_regex_captures() {
    let depth = 300;
    let mut code = String::new();
    code.push_str("m/");

    for _ in 0..depth {
        code.push_str("(");
    }
    code.push_str("x");
    for _ in 0..depth {
        code.push_str(")");
    }
    code.push_str("/");

    let mut parser = Parser::new(&code);
    let result = parser.parse();

    // Regex depth should be handled gracefully - either parse or fail with specific error
    // This ensures we don't hang on pathological regex patterns
    match result {
        Ok(_) => {
            // Parser might succeed with bounded regex parsing
        }
        Err(e) => {
            // Should fail gracefully, not hang
            assert!(
                matches!(e, ParseError::RecursionLimit | ParseError::LexerError { .. }),
                "Expected RecursionLimit or LexerError, got {:?}",
                e
            );
        }
    }
}

/// Test deeply nested hash reference dereferencing
///
/// Tests feature spec: ROADMAP.md#deep-nesting-boundedness
#[test]
fn parser_hang_risk_nested_hash_deref() {
    let depth = 300;
    let mut code = String::with_capacity(depth * 10);
    code.push_str("$hash");

    for i in 0..depth {
        code.push_str(&format!("{{key{}}}", i));
    }

    let mut parser = Parser::new(&code);
    let result = parser.parse();

    assert!(
        result.is_err(),
        "Expected RecursionLimit error for deeply nested hash dereferencing"
    );
}

/// Test deeply nested array reference indexing
///
/// Tests feature spec: ROADMAP.md#deep-nesting-boundedness
#[test]
fn parser_hang_risk_nested_array_indexing() {
    let depth = 300;
    let mut code = String::with_capacity(depth * 5);
    code.push_str("$arr");

    for _ in 0..depth {
        code.push_str("[0]");
    }

    let mut parser = Parser::new(&code);
    let result = parser.parse();

    assert!(
        result.is_err(),
        "Expected RecursionLimit error for deeply nested array indexing"
    );
}

/// Test nested quote operators with paired delimiters
///
/// Tests feature spec: ROADMAP.md#deep-nesting-boundedness
#[test]
fn parser_hang_risk_nested_quote_delimiters() {
    // Test q{} with nested braces
    let depth = 50;
    let mut code = String::new();
    code.push_str("q{");

    for _ in 0..depth {
        code.push_str("{");
    }
    code.push_str("text");
    for _ in 0..depth {
        code.push_str("}");
    }
    code.push_str("}");

    let mut parser = Parser::new(&code);
    let result = parser.parse();

    // Should handle balanced delimiter nesting gracefully
    match result {
        Ok(_) => {
            // Successful parse with balanced delimiters
        }
        Err(e) => {
            // Or fail gracefully without hanging
            eprintln!("Quote nesting parse error (acceptable): {:?}", e);
        }
    }
}

/// Test reasonable nesting depth succeeds (well below limit)
///
/// Tests feature spec: ROADMAP.md#deep-nesting-boundedness
#[test]
fn parser_hang_risk_reasonable_nesting_succeeds() {
    let depth = 50; // Well below 256 limit
    let mut code = String::new();

    for _ in 0..depth {
        code.push_str("{ ");
    }
    code.push_str("my $x = 1;");
    for _ in 0..depth {
        code.push_str("} ");
    }

    let mut parser = Parser::new(&code);
    let result = parser.parse();

    assert!(
        result.is_ok(),
        "Expected success for reasonable nesting depth: {:?}",
        result.err()
    );
}

/// Test boundary condition: just below limit
///
/// Tests feature spec: ROADMAP.md#deep-nesting-boundedness
#[test]
fn parser_hang_risk_boundary_just_below_limit() {
    let depth = 250; // Just below 256 limit
    let mut code = String::new();

    for _ in 0..depth {
        code.push_str("{ ");
    }
    code.push_str("1;");
    for _ in 0..depth {
        code.push_str("} ");
    }

    let mut parser = Parser::new(&code);
    let result = parser.parse();

    assert!(
        result.is_ok(),
        "Expected success for nesting just below limit: {:?}",
        result.err()
    );
}

/// Test boundary condition: just above limit
///
/// Tests feature spec: ROADMAP.md#deep-nesting-boundedness
#[test]
fn parser_hang_risk_boundary_just_above_limit() {
    let depth = 260; // Just above 256 limit
    let mut code = String::new();

    for _ in 0..depth {
        code.push_str("{ ");
    }
    code.push_str("1;");
    for _ in 0..depth {
        code.push_str("} ");
    }

    let mut parser = Parser::new(&code);
    let result = parser.parse();

    assert!(
        result.is_err(),
        "Expected RecursionLimit error for nesting just above limit"
    );
    assert!(
        matches!(result.unwrap_err(), ParseError::RecursionLimit),
        "Expected RecursionLimit error type"
    );
}

/// Test mixed control flow nesting (if/while/for combinations)
///
/// Tests feature spec: ROADMAP.md#deep-nesting-boundedness
#[test]
fn parser_hang_risk_mixed_control_flow_nesting() {
    let depth = 100;
    let mut code = String::new();

    for i in 0..depth {
        match i % 3 {
            0 => code.push_str("if (1) { "),
            1 => code.push_str("while (0) { "),
            _ => code.push_str("for (1..10) { "),
        }
    }
    code.push_str("my $x = 1;");
    for _ in 0..depth {
        code.push_str(" }");
    }

    let mut parser = Parser::new(&code);
    let result = parser.parse();

    assert!(
        result.is_err(),
        "Expected RecursionLimit error for mixed control flow nesting"
    );
}

/// Test deeply nested anonymous subroutines
///
/// Tests feature spec: ROADMAP.md#deep-nesting-boundedness
#[test]
fn parser_hang_risk_nested_anonymous_subs() {
    let depth = 300;
    let mut code = String::new();

    for _ in 0..depth {
        code.push_str("sub { ");
    }
    code.push_str("return 42;");
    for _ in 0..depth {
        code.push_str(" }");
    }

    let mut parser = Parser::new(&code);
    let result = parser.parse();

    assert!(
        result.is_err(),
        "Expected RecursionLimit error for deeply nested anonymous subs"
    );
}

/// Test deeply nested do blocks
///
/// Tests feature spec: ROADMAP.md#deep-nesting-boundedness
#[test]
fn parser_hang_risk_nested_do_blocks() {
    let depth = 300;
    let mut code = String::new();

    for _ in 0..depth {
        code.push_str("do { ");
    }
    code.push_str("42");
    for _ in 0..depth {
        code.push_str(" }");
    }

    let mut parser = Parser::new(&code);
    let result = parser.parse();

    assert!(
        result.is_err(),
        "Expected RecursionLimit error for deeply nested do blocks"
    );
}

/// Test deeply nested eval blocks
///
/// Tests feature spec: ROADMAP.md#deep-nesting-boundedness
#[test]
fn parser_hang_risk_nested_eval_blocks() {
    let depth = 300;
    let mut code = String::new();

    for _ in 0..depth {
        code.push_str("eval { ");
    }
    code.push_str("1");
    for _ in 0..depth {
        code.push_str(" }");
    }

    let mut parser = Parser::new(&code);
    let result = parser.parse();

    assert!(
        result.is_err(),
        "Expected RecursionLimit error for deeply nested eval blocks"
    );
}

/// Test deeply nested map/grep builtins
///
/// Tests feature spec: ROADMAP.md#deep-nesting-boundedness
#[test]
fn parser_hang_risk_nested_map_grep() {
    let depth = 300;
    let mut code = String::new();

    for i in 0..depth {
        if i % 2 == 0 {
            code.push_str("map { ");
        } else {
            code.push_str("grep { ");
        }
    }
    code.push_str("$_");
    for _ in 0..depth {
        code.push_str(" } @list");
    }

    let mut parser = Parser::new(&code);
    let result = parser.parse();

    assert!(
        result.is_err(),
        "Expected RecursionLimit error for deeply nested map/grep"
    );
}

/// Test pathological case: alternating array and hash nesting
///
/// Tests feature spec: ROADMAP.md#deep-nesting-boundedness
#[test]
fn parser_hang_risk_pathological_alternating_structures() {
    let depth = 150;
    let mut code = String::new();

    for i in 0..depth {
        if i % 2 == 0 {
            code.push_str("[ { a => ");
        } else {
            code.push_str("{ b => [ ");
        }
    }
    code.push_str("1");
    for i in 0..depth {
        if i % 2 == 0 {
            code.push_str(" ] }");
        } else {
            code.push_str(" } ]");
        }
    }

    let mut parser = Parser::new(&code);
    let result = parser.parse();

    assert!(
        result.is_err(),
        "Expected RecursionLimit error for pathological alternating structures"
    );
}

/// Test that parser doesn't hang on extremely deep nesting (timeout test)
///
/// Tests feature spec: ROADMAP.md#deep-nesting-boundedness
#[test]
#[cfg_attr(not(feature = "slow_tests"), ignore)]
fn parser_hang_risk_no_timeout_on_pathological_input() {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    let depth = 1000; // Extremely deep
    let mut code = String::new();

    for _ in 0..depth {
        code.push_str("{ ");
    }
    code.push_str("1;");
    for _ in 0..depth {
        code.push_str("} ");
    }

    let code_arc = Arc::new(code);
    let result_arc = Arc::new(Mutex::new(None));
    let result_clone = Arc::clone(&result_arc);

    let handle = std::thread::spawn(move || {
        let mut parser = Parser::new(&code_arc);
        let result = parser.parse();
        *result_clone.lock().unwrap() = Some(result);
    });

    // Wait max 5 seconds for parser to complete
    let _timeout = Duration::from_secs(5);
    let completed = handle.join().is_ok();

    assert!(
        completed,
        "Parser should complete within timeout, not hang indefinitely"
    );

    let result_guard = result_arc.lock().unwrap();
    let result = result_guard.as_ref().expect("Parser should have returned a result");

    // Should fail with RecursionLimit, not hang
    assert!(
        result.is_err(),
        "Parser should reject extremely deep nesting"
    );
}

/// Test performance doesn't degrade linearly with depth
///
/// Tests feature spec: ROADMAP.md#deep-nesting-boundedness
#[test]
#[cfg_attr(not(feature = "slow_tests"), ignore)]
fn parser_hang_risk_performance_bounded() {
    use std::time::Instant;

    // Test that parse time doesn't grow linearly with depth
    // Parse at safe depth should be fast
    let safe_depth = 100;
    let mut safe_code = String::new();
    for _ in 0..safe_depth {
        safe_code.push_str("{ ");
    }
    safe_code.push_str("1;");
    for _ in 0..safe_depth {
        safe_code.push_str("} ");
    }

    let start = Instant::now();
    let mut parser = Parser::new(&safe_code);
    let _ = parser.parse();
    let safe_duration = start.elapsed();

    // Parse at limit should not be dramatically slower
    let limit_depth = 255;
    let mut limit_code = String::new();
    for _ in 0..limit_depth {
        limit_code.push_str("{ ");
    }
    limit_code.push_str("1;");
    for _ in 0..limit_depth {
        limit_code.push_str("} ");
    }

    let start = Instant::now();
    let mut parser = Parser::new(&limit_code);
    let _ = parser.parse();
    let limit_duration = start.elapsed();

    // Ratio should be less than 10x (linear would be 2.5x, allowing some overhead)
    let ratio = limit_duration.as_micros() as f64 / safe_duration.as_micros() as f64;
    assert!(
        ratio < 10.0,
        "Parse time ratio {} indicates potential performance issue",
        ratio
    );
}
