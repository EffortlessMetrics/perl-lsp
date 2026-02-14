//! Property tests for complete Perl programs
//!
//! These tests use the program composition generators to test parsing
//! of realistic, complete Perl programs.

use perl_corpus::r#gen::program::{
    any_program, complex_program, program_with_control_flow, program_with_declarations,
    program_with_imports, program_with_subs, program_with_tie, simple_program,
};
use perl_parser::{
    Parser,
    ast::{Node, NodeKind},
};
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, FileFailurePersistence};
use std::collections::HashSet;

const REGRESS_DIR: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/_proptest-regressions/prop_complete_programs");

/// Check for cycles in the AST
fn check_no_cycles(root: &Node) -> Result<(), String> {
    let mut visited = HashSet::new();
    check_no_cycles_rec(root, &mut visited)
}

fn check_no_cycles_rec(node: &Node, visited: &mut HashSet<*const Node>) -> Result<(), String> {
    let ptr = node as *const Node;
    if visited.contains(&ptr) {
        return Err("Cycle detected".to_string());
    }
    visited.insert(ptr);
    visit_children(node, |child| check_no_cycles_rec(child, visited))?;
    visited.remove(&ptr);
    Ok(())
}

/// Count total AST nodes
fn count_nodes(node: &Node) -> usize {
    let mut count = 1;
    let _ = visit_children(node, |child| {
        count += count_nodes(child);
        Ok(())
    });
    count
}

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
        VariableDeclaration { variable, initializer, .. } => {
            f(variable)?;
            if let Some(init) = initializer {
                f(init)?;
            }
        }
        VariableListDeclaration { variables, initializer, .. } => {
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
        Ternary { condition, then_expr, else_expr } => {
            f(condition)?;
            f(then_expr)?;
            f(else_expr)?;
        }
        Block { statements } => {
            for stmt in statements {
                f(stmt)?;
            }
        }
        If { condition, then_branch, elsif_branches, else_branch } => {
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
        While { condition, body, continue_block, .. } => {
            f(condition)?;
            f(body)?;
            if let Some(cont) = continue_block {
                f(cont)?;
            }
        }
        Foreach { variable, list, body, continue_block } => {
            f(variable)?;
            f(list)?;
            f(body)?;
        }
        For { init, condition, update, body, continue_block } => {
            if let Some(i) = init {
                f(i)?;
            }
            if let Some(c) = condition {
                f(c)?;
            }
            if let Some(u) = update {
                f(u)?;
            }
            f(body)?;
            if let Some(cont) = continue_block {
                f(cont)?;
            }
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

proptest! {
    #![proptest_config(ProptestConfig {
        cases: std::env::var("PROPTEST_CASES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(32),
        failure_persistence: Some(Box::new(
            FileFailurePersistence::Direct(REGRESS_DIR)
        )),
        ..ProptestConfig::default()
    })]

    // ==========================================================================
    // Simple program tests
    // ==========================================================================

    #[test]
    fn simple_programs_parse_without_panic(code in simple_program()) {
        let mut parser = Parser::new(&code);
        let _ = parser.parse();
    }

    #[test]
    fn simple_programs_no_cycles(code in simple_program()) {
        let mut parser = Parser::new(&code);
        if let Ok(ast) = parser.parse() {
            let result = check_no_cycles(&ast);
            prop_assert!(result.is_ok(), "Cycle detected in simple program AST");
        }
    }

    // ==========================================================================
    // Programs with subroutines
    // ==========================================================================

    #[test]
    fn programs_with_subs_parse(code in program_with_subs()) {
        let mut parser = Parser::new(&code);
        let _ = parser.parse();
    }

    #[test]
    fn programs_with_subs_no_cycles(code in program_with_subs()) {
        let mut parser = Parser::new(&code);
        if let Ok(ast) = parser.parse() {
            let result = check_no_cycles(&ast);
            prop_assert!(result.is_ok(), "Cycle detected in program with subs");
        }
    }

    // ==========================================================================
    // Programs with control flow
    // ==========================================================================

    #[test]
    fn programs_with_control_flow_parse(code in program_with_control_flow()) {
        let mut parser = Parser::new(&code);
        let _ = parser.parse();
    }

    #[test]
    fn programs_with_control_flow_no_cycles(code in program_with_control_flow()) {
        let mut parser = Parser::new(&code);
        if let Ok(ast) = parser.parse() {
            let result = check_no_cycles(&ast);
            prop_assert!(result.is_ok(), "Cycle detected in control flow program");
        }
    }

    // ==========================================================================
    // Programs with declarations
    // ==========================================================================

    #[test]
    fn programs_with_declarations_parse(code in program_with_declarations()) {
        let mut parser = Parser::new(&code);
        let _ = parser.parse();
    }

    // ==========================================================================
    // Programs with imports
    // ==========================================================================

    #[test]
    fn programs_with_imports_parse(code in program_with_imports()) {
        let mut parser = Parser::new(&code);
        let _ = parser.parse();
    }

    // ==========================================================================
    // Programs with tie
    // ==========================================================================

    #[test]
    fn programs_with_tie_parse(code in program_with_tie()) {
        let mut parser = Parser::new(&code);
        let _ = parser.parse();
    }

    // ==========================================================================
    // Any program tests
    // ==========================================================================

    #[test]
    fn any_program_parses(code in any_program()) {
        let mut parser = Parser::new(&code);
        let _ = parser.parse();
    }

    #[test]
    fn any_program_no_cycles(code in any_program()) {
        let mut parser = Parser::new(&code);
        if let Ok(ast) = parser.parse() {
            let result = check_no_cycles(&ast);
            prop_assert!(result.is_ok(), "Cycle detected in any_program AST");
        }
    }

    #[test]
    fn any_program_bounded_nodes(code in any_program()) {
        let mut parser = Parser::new(&code);
        if let Ok(ast) = parser.parse() {
            let count = count_nodes(&ast);
            // Should have reasonable node count for input size
            prop_assert!(
                count <= code.len() * 10 + 100,
                "Too many nodes ({}) for program of size {}",
                count,
                code.len()
            );
        }
    }

    // ==========================================================================
    // Complex program tests
    // ==========================================================================

    #[test]
    fn complex_programs_parse(code in complex_program()) {
        let mut parser = Parser::new(&code);
        let _ = parser.parse();
    }

    #[test]
    fn complex_programs_no_cycles(code in complex_program()) {
        let mut parser = Parser::new(&code);
        if let Ok(ast) = parser.parse() {
            let result = check_no_cycles(&ast);
            prop_assert!(result.is_ok(), "Cycle detected in complex program AST");
        }
    }

    #[test]
    fn complex_programs_deterministic(code in complex_program()) {
        let mut parser1 = Parser::new(&code);
        let mut parser2 = Parser::new(&code);

        let result1 = parser1.parse();
        let result2 = parser2.parse();

        prop_assert_eq!(result1.is_ok(), result2.is_ok());
    }

    // ==========================================================================
    // Root node invariants
    // ==========================================================================

    #[test]
    fn all_programs_have_program_root(code in any_program()) {
        let mut parser = Parser::new(&code);
        if let Ok(ast) = parser.parse() {
            prop_assert!(
                matches!(ast.kind, NodeKind::Program { .. }),
                "Expected Program root node"
            );
        }
    }
}
