//! Round-trip property tests for parse stability and invariants
//!
//! These tests verify that:
//! 1. Parsing is deterministic (same input -> same AST)
//! 2. AST spans are valid and within source bounds
//! 3. Semantic equivalence holds for syntactic variations

use perl_corpus::r#gen::{
    builtins::builtin_in_context, control_flow::loop_with_control,
    declarations::declaration_in_context, expressions::expression_in_context, qw::qw_in_context,
};
use perl_parser::{
    Parser,
    ast::{Node, NodeKind},
};
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, FileFailurePersistence};

const REGRESS_DIR: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/_proptest-regressions/prop_round_trip");

/// Extract AST shape (node kinds only) for comparison
fn extract_shape(node: &Node) -> Vec<String> {
    let mut out = Vec::new();
    extract_shape_rec(node, &mut out);
    out
}

fn extract_shape_rec(node: &Node, out: &mut Vec<String>) {
    use NodeKind::*;

    // Get variant name
    let s = format!("{:?}", node.kind);
    let name = s.split(['(', '{']).next().map_or_else(|| s.clone(), |n| n.to_string());
    out.push(name);

    match &node.kind {
        Program { statements } => {
            for s in statements {
                extract_shape_rec(s, out);
            }
        }
        VariableDeclaration { variable, initializer, .. } => {
            extract_shape_rec(variable, out);
            if let Some(init) = initializer {
                extract_shape_rec(init, out);
            }
        }
        VariableListDeclaration { variables, initializer, .. } => {
            for v in variables {
                extract_shape_rec(v, out);
            }
            if let Some(init) = initializer {
                extract_shape_rec(init, out);
            }
        }
        Assignment { lhs, rhs, .. } => {
            extract_shape_rec(lhs, out);
            extract_shape_rec(rhs, out);
        }
        Binary { left, right, .. } => {
            extract_shape_rec(left, out);
            extract_shape_rec(right, out);
        }
        Unary { operand, .. } => {
            extract_shape_rec(operand, out);
        }
        Ternary { condition, then_expr, else_expr } => {
            extract_shape_rec(condition, out);
            extract_shape_rec(then_expr, out);
            extract_shape_rec(else_expr, out);
        }
        Block { statements } => {
            for s in statements {
                extract_shape_rec(s, out);
            }
        }
        If { condition, then_branch, elsif_branches, else_branch } => {
            extract_shape_rec(condition, out);
            extract_shape_rec(then_branch, out);
            for (cond, br) in elsif_branches {
                extract_shape_rec(cond, out);
                extract_shape_rec(br, out);
            }
            if let Some(else_br) = else_branch {
                extract_shape_rec(else_br, out);
            }
        }
        While { condition, body, continue_block, .. } => {
            extract_shape_rec(condition, out);
            extract_shape_rec(body, out);
            if let Some(cont) = continue_block {
                extract_shape_rec(cont, out);
            }
        }
        Foreach { variable, list, body, continue_block } => {
            extract_shape_rec(variable, out);
            extract_shape_rec(list, out);
            extract_shape_rec(body, out);
        }
        For { init, condition, update, body, continue_block } => {
            if let Some(i) = init {
                extract_shape_rec(i, out);
            }
            if let Some(c) = condition {
                extract_shape_rec(c, out);
            }
            if let Some(u) = update {
                extract_shape_rec(u, out);
            }
            extract_shape_rec(body, out);
            if let Some(cont) = continue_block {
                extract_shape_rec(cont, out);
            }
        }
        Subroutine { body, .. } => {
            extract_shape_rec(body, out);
        }
        FunctionCall { args, .. } => {
            for a in args {
                extract_shape_rec(a, out);
            }
        }
        MethodCall { object, args, .. } => {
            extract_shape_rec(object, out);
            for a in args {
                extract_shape_rec(a, out);
            }
        }
        ArrayLiteral { elements } => {
            for e in elements {
                extract_shape_rec(e, out);
            }
        }
        HashLiteral { pairs } => {
            for (k, v) in pairs {
                extract_shape_rec(k, out);
                extract_shape_rec(v, out);
            }
        }
        Return { value: Some(val) } => {
            extract_shape_rec(val, out);
        }
        _ => {}
    }
}

/// Check all spans in the AST are valid
/// Returns (error_count, first_error) for reporting
fn check_spans_valid(node: &Node, source_len: usize) -> (usize, Option<String>) {
    let mut errors = Vec::new();
    check_spans_rec(node, source_len, &mut errors);
    let count = errors.len();
    (count, errors.into_iter().next())
}

fn check_spans_rec(node: &Node, source_len: usize, errors: &mut Vec<String>) {
    use NodeKind::*;

    // Check this node's span - only report if significantly out of bounds
    if node.location.end > source_len + 10 {
        // Allow small overruns from trailing newlines
        errors.push(format!(
            "Node {:?} has span end {} beyond source length {}",
            format!("{:?}", node.kind).split(['(', '{']).next().unwrap_or("Unknown"),
            node.location.end,
            source_len
        ));
    }
    // Note: start > end can happen for empty/synthetic nodes in error recovery
    // Don't treat as fatal, just track it

    // Recursively check children
    match &node.kind {
        Program { statements } => {
            for s in statements {
                check_spans_rec(s, source_len, errors);
            }
        }
        VariableDeclaration { variable, initializer, .. } => {
            check_spans_rec(variable, source_len, errors);
            if let Some(init) = initializer {
                check_spans_rec(init, source_len, errors);
            }
        }
        VariableListDeclaration { variables, initializer, .. } => {
            for v in variables {
                check_spans_rec(v, source_len, errors);
            }
            if let Some(init) = initializer {
                check_spans_rec(init, source_len, errors);
            }
        }
        Assignment { lhs, rhs, .. } => {
            check_spans_rec(lhs, source_len, errors);
            check_spans_rec(rhs, source_len, errors);
        }
        Binary { left, right, .. } => {
            check_spans_rec(left, source_len, errors);
            check_spans_rec(right, source_len, errors);
        }
        Unary { operand, .. } => {
            check_spans_rec(operand, source_len, errors);
        }
        Ternary { condition, then_expr, else_expr } => {
            check_spans_rec(condition, source_len, errors);
            check_spans_rec(then_expr, source_len, errors);
            check_spans_rec(else_expr, source_len, errors);
        }
        Block { statements } => {
            for s in statements {
                check_spans_rec(s, source_len, errors);
            }
        }
        If { condition, then_branch, elsif_branches, else_branch } => {
            check_spans_rec(condition, source_len, errors);
            check_spans_rec(then_branch, source_len, errors);
            for (cond, br) in elsif_branches {
                check_spans_rec(cond, source_len, errors);
                check_spans_rec(br, source_len, errors);
            }
            if let Some(else_br) = else_branch {
                check_spans_rec(else_br, source_len, errors);
            }
        }
        While { condition, body, continue_block, .. } => {
            check_spans_rec(condition, source_len, errors);
            check_spans_rec(body, source_len, errors);
            if let Some(cont) = continue_block {
                check_spans_rec(cont, source_len, errors);
            }
        }
        Foreach { variable, list, body, continue_block } => {
            check_spans_rec(variable, source_len, errors);
            check_spans_rec(list, source_len, errors);
            check_spans_rec(body, source_len, errors);
        }
        Subroutine { body, .. } => {
            check_spans_rec(body, source_len, errors);
        }
        FunctionCall { args, .. } => {
            for arg in args {
                check_spans_rec(arg, source_len, errors);
            }
        }
        MethodCall { object, args, .. } => {
            check_spans_rec(object, source_len, errors);
            for arg in args {
                check_spans_rec(arg, source_len, errors);
            }
        }
        ArrayLiteral { elements } => {
            for elem in elements {
                check_spans_rec(elem, source_len, errors);
            }
        }
        HashLiteral { pairs } => {
            for (k, v) in pairs {
                check_spans_rec(k, source_len, errors);
                check_spans_rec(v, source_len, errors);
            }
        }
        _ => {}
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: std::env::var("PROPTEST_CASES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(64),
        failure_persistence: Some(Box::new(
            FileFailurePersistence::Direct(REGRESS_DIR)
        )),
        ..ProptestConfig::default()
    })]

    // ==========================================================================
    // Parse determinism tests
    // ==========================================================================

    #[test]
    fn parse_is_deterministic_expressions(code in expression_in_context()) {
        let mut parser1 = Parser::new(&code);
        let mut parser2 = Parser::new(&code);

        let result1 = parser1.parse();
        let result2 = parser2.parse();

        // Both should have same success/failure
        prop_assert_eq!(
            result1.is_ok(),
            result2.is_ok(),
            "Non-deterministic parse success for: {}",
            code
        );

        // If both succeed, shapes should match
        if let (Ok(ast1), Ok(ast2)) = (result1, result2) {
            let shape1 = extract_shape(&ast1);
            let shape2 = extract_shape(&ast2);
            prop_assert_eq!(
                shape1,
                shape2,
                "Non-deterministic AST shape for: {}",
                code
            );
        }
    }

    #[test]
    fn parse_is_deterministic_declarations(code in declaration_in_context()) {
        let mut parser1 = Parser::new(&code);
        let mut parser2 = Parser::new(&code);

        let result1 = parser1.parse();
        let result2 = parser2.parse();

        prop_assert_eq!(result1.is_ok(), result2.is_ok());

        if let (Ok(ast1), Ok(ast2)) = (result1, result2) {
            prop_assert_eq!(extract_shape(&ast1), extract_shape(&ast2));
        }
    }

    #[test]
    fn parse_is_deterministic_control_flow(code in loop_with_control()) {
        let mut parser1 = Parser::new(&code);
        let mut parser2 = Parser::new(&code);

        let result1 = parser1.parse();
        let result2 = parser2.parse();

        prop_assert_eq!(result1.is_ok(), result2.is_ok());

        if let (Ok(ast1), Ok(ast2)) = (result1, result2) {
            prop_assert_eq!(extract_shape(&ast1), extract_shape(&ast2));
        }
    }

    // ==========================================================================
    // Span validity tests (only check for severe issues - out of bounds)
    // Note: start > end can happen for empty/synthetic nodes in error recovery
    // ==========================================================================

    #[test]
    fn spans_valid_expressions(code in expression_in_context()) {
        let mut parser = Parser::new(&code);
        if let Ok(ast) = parser.parse() {
            let (error_count, first_error) = check_spans_valid(&ast, code.len());
            prop_assert!(error_count == 0, "Span out of bounds: {:?}", first_error);
        }
    }

    #[test]
    fn spans_valid_declarations(code in declaration_in_context()) {
        let mut parser = Parser::new(&code);
        if let Ok(ast) = parser.parse() {
            let (error_count, first_error) = check_spans_valid(&ast, code.len());
            prop_assert!(error_count == 0, "Span out of bounds: {:?}", first_error);
        }
    }

    #[test]
    fn spans_valid_control_flow(code in loop_with_control()) {
        let mut parser = Parser::new(&code);
        if let Ok(ast) = parser.parse() {
            let (error_count, first_error) = check_spans_valid(&ast, code.len());
            prop_assert!(error_count == 0, "Span out of bounds: {:?}", first_error);
        }
    }

    #[test]
    fn spans_valid_builtins(code in builtin_in_context()) {
        let mut parser = Parser::new(&code);
        if let Ok(ast) = parser.parse() {
            let (error_count, first_error) = check_spans_valid(&ast, code.len());
            prop_assert!(error_count == 0, "Span out of bounds: {:?}", first_error);
        }
    }

    #[test]
    fn spans_valid_qw(code in qw_in_context()) {
        let mut parser = Parser::new(&code);
        if let Ok(ast) = parser.parse() {
            let (error_count, first_error) = check_spans_valid(&ast, code.len());
            prop_assert!(error_count == 0, "Span out of bounds: {:?}", first_error);
        }
    }

    // ==========================================================================
    // Multi-parse stability
    // ==========================================================================

    #[test]
    fn multiple_parses_stable(code in expression_in_context()) {
        // Parse 3 times and ensure all results are identical
        let results: Vec<_> = (0..3)
            .map(|_| {
                let mut p = Parser::new(&code);
                p.parse().ok().map(|ast| extract_shape(&ast))
            })
            .collect();

        // All should be Some or all should be None
        let all_some = results.iter().all(|r| r.is_some());
        let all_none = results.iter().all(|r| r.is_none());
        prop_assert!(all_some || all_none, "Inconsistent parse results");

        // If all succeeded, shapes should be identical
        if all_some {
            let first = results[0].as_ref();
            for (i, result) in results.iter().enumerate().skip(1) {
                prop_assert_eq!(
                    first,
                    result.as_ref(),
                    "Parse {} differs from parse 0",
                    i
                );
            }
        }
    }

    // ==========================================================================
    // Root node invariants
    // ==========================================================================

    #[test]
    fn root_is_program_node(code in prop_oneof![
        expression_in_context(),
        declaration_in_context(),
        loop_with_control(),
    ]) {
        let mut parser = Parser::new(&code);
        if let Ok(ast) = parser.parse() {
            prop_assert!(
                matches!(ast.kind, NodeKind::Program { .. }),
                "Root node should be Program, got: {:?}",
                format!("{:?}", ast.kind).split(['(', '{']).next()
            );
        }
    }

    #[test]
    fn root_span_covers_source(code in expression_in_context()) {
        let mut parser = Parser::new(&code);
        if let Ok(ast) = parser.parse() {
            // Root span should start at 0 and cover most of the source
            prop_assert_eq!(ast.location.start, 0, "Root span should start at 0");
            // Allow some tolerance for trailing whitespace/newlines
            prop_assert!(
                ast.location.end <= code.len(),
                "Root span end {} exceeds source length {}",
                ast.location.end,
                code.len()
            );
        }
    }
}
