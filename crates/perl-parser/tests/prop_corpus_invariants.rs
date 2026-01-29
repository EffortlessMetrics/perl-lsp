//! Property tests using corpus generators for comprehensive parser coverage
//!
//! These tests use the specialized Perl code generators from perl-corpus
//! instead of generic regex patterns, providing more realistic and targeted testing.

use perl_corpus::r#gen::{
    builtins::builtin_in_context,
    control_flow::loop_with_control,
    declarations::declaration_in_context,
    expressions::expression_in_context,
    qw::qw_in_context,
    tie::tie_in_context,
};
use perl_parser::{
    ast::{Node, NodeKind},
    Parser,
};
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, FileFailurePersistence};
use std::collections::HashSet;

const REGRESS_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/_proptest-regressions/prop_corpus_invariants"
);

/// Helper to visit all children of a node
fn visit_children<F>(node: &Node, mut f: F) -> Result<(), String>
where
    F: FnMut(&Node) -> Result<(), String>,
{
    use NodeKind::*;
    match &node.kind {
        Program { statements } => {
            for stmt in statements {
                f(stmt)?;
            }
        }
        VariableDeclaration {
            variable,
            initializer,
            ..
        } => {
            f(variable)?;
            if let Some(init) = initializer {
                f(init)?;
            }
        }
        VariableListDeclaration {
            variables,
            initializer,
            ..
        } => {
            for var in variables {
                f(var)?;
            }
            if let Some(init) = initializer {
                f(init)?;
            }
        }
        Assignment { lhs, rhs, .. } => {
            f(lhs)?;
            f(rhs)?;
        }
        Binary { left, right, .. } => {
            f(left)?;
            f(right)?;
        }
        Unary { operand, .. } => {
            f(operand)?;
        }
        Ternary {
            condition,
            then_expr,
            else_expr,
        } => {
            f(condition)?;
            f(then_expr)?;
            f(else_expr)?;
        }
        Block { statements } => {
            for stmt in statements {
                f(stmt)?;
            }
        }
        If {
            condition,
            then_branch,
            elsif_branches,
            else_branch,
        } => {
            f(condition)?;
            f(then_branch)?;
            for (cond, branch) in elsif_branches {
                f(cond)?;
                f(branch)?;
            }
            if let Some(else_br) = else_branch {
                f(else_br)?;
            }
        }
        While { condition, body, .. } => {
            f(condition)?;
            f(body)?;
        }
        Foreach {
            variable,
            list,
            body,
        } => {
            f(variable)?;
            f(list)?;
            f(body)?;
        }
        Subroutine { body, .. } => {
            f(body)?;
        }
        FunctionCall { args, .. } => {
            for arg in args {
                f(arg)?;
            }
        }
        MethodCall { object, args, .. } => {
            f(object)?;
            for arg in args {
                f(arg)?;
            }
        }
        ArrayLiteral { elements } => {
            for elem in elements {
                f(elem)?;
            }
        }
        HashLiteral { pairs } => {
            for (key, value) in pairs {
                f(key)?;
                f(value)?;
            }
        }
        _ => {} // Leaf nodes
    }
    Ok(())
}

/// Check that the AST has no cycles
fn check_no_cycles(root: &Node) -> Result<(), String> {
    let mut visited = HashSet::new();
    check_no_cycles_rec(root, &mut visited)
}

fn check_no_cycles_rec(node: &Node, visited: &mut HashSet<*const Node>) -> Result<(), String> {
    let ptr = node as *const Node;
    if visited.contains(&ptr) {
        return Err("Cycle detected in AST".to_string());
    }
    visited.insert(ptr);
    visit_children(node, |child| check_no_cycles_rec(child, visited))?;
    visited.remove(&ptr);
    Ok(())
}

/// Count total nodes in the AST
fn count_nodes(node: &Node) -> usize {
    let mut count = 1;
    let _ = visit_children(node, |child| {
        count += count_nodes(child);
        Ok(())
    });
    count
}

/// Measure maximum depth of the AST
fn max_depth(node: &Node, current: usize) -> usize {
    let mut max = current;
    let _ = visit_children(node, |child| {
        let child_depth = max_depth(child, current + 1);
        if child_depth > max {
            max = child_depth;
        }
        Ok(())
    });
    max
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
    // Control flow invariants
    // ==========================================================================

    #[test]
    fn control_flow_parses_without_panic(code in loop_with_control()) {
        let mut parser = Parser::new(&code);
        let _ = parser.parse();
    }

    #[test]
    fn control_flow_no_cycles(code in loop_with_control()) {
        let mut parser = Parser::new(&code);
        if let Ok(ast) = parser.parse() {
            let result = check_no_cycles(&ast);
            prop_assert!(result.is_ok(), "Cycle in control flow AST: {:?}", result);
        }
    }

    #[test]
    fn control_flow_bounded_depth(code in loop_with_control()) {
        let mut parser = Parser::new(&code);
        if let Ok(ast) = parser.parse() {
            let depth = max_depth(&ast, 0);
            prop_assert!(depth < 100, "Control flow AST too deep: {}", depth);
        }
    }

    // ==========================================================================
    // Declaration invariants
    // ==========================================================================

    #[test]
    fn declarations_parse_without_panic(code in declaration_in_context()) {
        let mut parser = Parser::new(&code);
        let _ = parser.parse();
    }

    #[test]
    fn declarations_no_cycles(code in declaration_in_context()) {
        let mut parser = Parser::new(&code);
        if let Ok(ast) = parser.parse() {
            let result = check_no_cycles(&ast);
            prop_assert!(result.is_ok(), "Cycle in declaration AST: {:?}", result);
        }
    }

    #[test]
    fn declarations_bounded_nodes(code in declaration_in_context()) {
        let mut parser = Parser::new(&code);
        if let Ok(ast) = parser.parse() {
            let count = count_nodes(&ast);
            // Node count should be reasonable relative to input size
            prop_assert!(
                count <= code.len() * 5 + 100,
                "Too many nodes ({}) for declaration of size {}",
                count,
                code.len()
            );
        }
    }

    // ==========================================================================
    // Expression invariants
    // ==========================================================================

    #[test]
    fn expressions_parse_without_panic(code in expression_in_context()) {
        let mut parser = Parser::new(&code);
        let _ = parser.parse();
    }

    #[test]
    fn expressions_no_cycles(code in expression_in_context()) {
        let mut parser = Parser::new(&code);
        if let Ok(ast) = parser.parse() {
            let result = check_no_cycles(&ast);
            prop_assert!(result.is_ok(), "Cycle in expression AST: {:?}", result);
        }
    }

    #[test]
    fn expressions_successful_parse(code in expression_in_context()) {
        let mut parser = Parser::new(&code);
        let result = parser.parse();
        // Expressions from our generator should parse successfully
        prop_assert!(
            result.is_ok(),
            "Expression failed to parse: {}\nError: {:?}",
            code,
            result.err()
        );
    }

    // ==========================================================================
    // Builtin function invariants
    // ==========================================================================

    #[test]
    fn builtins_parse_without_panic(code in builtin_in_context()) {
        let mut parser = Parser::new(&code);
        let _ = parser.parse();
    }

    #[test]
    fn builtins_no_cycles(code in builtin_in_context()) {
        let mut parser = Parser::new(&code);
        if let Ok(ast) = parser.parse() {
            let result = check_no_cycles(&ast);
            prop_assert!(result.is_ok(), "Cycle in builtin AST: {:?}", result);
        }
    }

    // ==========================================================================
    // QW expression invariants
    // ==========================================================================

    #[test]
    fn qw_parses_without_panic(code in qw_in_context()) {
        let mut parser = Parser::new(&code);
        let _ = parser.parse();
    }

    #[test]
    fn qw_no_cycles(code in qw_in_context()) {
        let mut parser = Parser::new(&code);
        if let Ok(ast) = parser.parse() {
            let result = check_no_cycles(&ast);
            prop_assert!(result.is_ok(), "Cycle in qw AST: {:?}", result);
        }
    }

    // ==========================================================================
    // Tie/untie invariants
    // ==========================================================================

    #[test]
    fn tie_parses_without_panic(code in tie_in_context()) {
        let mut parser = Parser::new(&code);
        let _ = parser.parse();
    }

    #[test]
    fn tie_no_cycles(code in tie_in_context()) {
        let mut parser = Parser::new(&code);
        if let Ok(ast) = parser.parse() {
            let result = check_no_cycles(&ast);
            prop_assert!(result.is_ok(), "Cycle in tie AST: {:?}", result);
        }
    }

    // ==========================================================================
    // Combined/composite invariants
    // ==========================================================================

    #[test]
    fn any_generated_code_no_panic(
        code in prop_oneof![
            loop_with_control(),
            declaration_in_context(),
            expression_in_context(),
            builtin_in_context(),
            qw_in_context(),
            tie_in_context(),
        ]
    ) {
        let mut parser = Parser::new(&code);
        // Parser should never panic, even on edge cases
        let _ = parser.parse();
    }

    #[test]
    fn any_generated_code_deterministic(
        code in prop_oneof![
            loop_with_control(),
            declaration_in_context(),
            expression_in_context(),
        ]
    ) {
        // Parse twice and compare results
        let mut parser1 = Parser::new(&code);
        let mut parser2 = Parser::new(&code);

        let result1 = parser1.parse();
        let result2 = parser2.parse();

        // Both should succeed or both should fail
        prop_assert_eq!(
            result1.is_ok(),
            result2.is_ok(),
            "Non-deterministic parsing for: {}",
            code
        );
    }
}
