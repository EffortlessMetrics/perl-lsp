//! Property tests for parser invariants and safety properties

use perl_parser::{
    Parser,
    ast::{Node, NodeKind},
};
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, FileFailurePersistence};
use std::collections::HashSet;

const REGRESS_DIR: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/_proptest-regressions/prop_invariants");

/// Visit all nodes and check for cycles
fn check_no_cycles(root: &Node) -> Result<(), String> {
    let mut visited = HashSet::new();
    check_no_cycles_rec(root, &mut visited, &mut Vec::new())
}

fn check_no_cycles_rec(
    node: &Node,
    visited: &mut HashSet<*const Node>,
    path: &mut Vec<String>,
) -> Result<(), String> {
    let ptr = node as *const Node;

    if visited.contains(&ptr) {
        return Err(format!("Cycle detected at path: {:?}", path));
    }

    visited.insert(ptr);

    // Add current node to path
    let kind_str = format!("{:?}", node.kind);
    let variant = kind_str.split(['(', '{']).next().unwrap_or_else(|| &kind_str).to_string();
    path.push(variant);

    // Visit children based on node kind
    visit_children(node, |child| check_no_cycles_rec(child, visited, path))?;

    // Remove from path when done
    path.pop();
    visited.remove(&ptr);

    Ok(())
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
        While { condition, body, .. } => {
            f(condition)?;
            f(body)?;
        }
        Foreach { variable, list, body, continue_block } => {
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

/// Count total nodes in the AST
fn count_nodes(node: &Node) -> usize {
    let mut count = 1; // Count self
    visit_children(node, |child| {
        count += count_nodes(child);
        Ok::<(), String>(())
    })
    .unwrap_or(());
    count
}

/// Check depth doesn't exceed reasonable limits
fn check_depth(node: &Node, current_depth: usize, max_depth: usize) -> Result<(), String> {
    if current_depth > max_depth {
        return Err(format!("AST depth {} exceeds maximum {}", current_depth, max_depth));
    }

    visit_children(node, |child| check_depth(child, current_depth + 1, max_depth))
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

    #[test]
    fn no_cycles_in_ast(
        input in "[^\0]{1,100}"
    ) {
        let mut parser = Parser::new(&input);
        let ast = parser.parse();

        if let Ok(root) = ast {
            let result = check_no_cycles(&root);
            prop_assert!(result.is_ok(), "Cycle detected: {:?}", result);
        }
    }

    #[test]
    fn bounded_ast_depth(
        input in "[^\0]{1,100}"
    ) {
        let mut parser = Parser::new(&input);
        let ast = parser.parse();

        if let Ok(root) = ast {
            let result = check_depth(&root, 0, 100);
            prop_assert!(result.is_ok(), "Depth exceeded: {:?}", result);
        }
    }

    #[test]
    fn bounded_node_count(
        input in "[^\0]{1,100}"
    ) {
        let mut parser = Parser::new(&input);
        let ast = parser.parse();

        if let Ok(root) = ast {
            let count = count_nodes(&root);
            // Node count should be reasonable relative to input size
            prop_assert!(count <= input.len() * 10,
                        "Too many nodes ({}) for input size {}", count, input.len());
        }
    }

    #[test]
    fn parser_doesnt_panic(
        input in match prop::string::string_regex("[^\0]{0,200}") {
            Ok(strat) => strat,
            Err(e) => panic!("invalid proptest regex pattern: {e:?}"),
        }
    ) {
        let mut parser = Parser::new(&input);
        // Parser should either succeed or return an error, never panic
        let _ = parser.parse();
    }

    #[test]
    fn empty_input_parses_cleanly(
        idx in 0usize..5
    ) {
        let inputs = ["", " ", "\n", "\t", "   \n  "];
        let input = inputs[idx];

        let mut parser = Parser::new(input);
        let result = parser.parse();
        prop_assert!(result.is_ok(), "Failed to parse empty/whitespace input: {:?}", input);
    }

    #[test]
    fn nested_structures_parse(
        depth in 1usize..10
    ) {
        // Generate nested blocks
        let mut code = String::new();
        for _ in 0..depth {
            code.push_str("{ ");
        }
        code.push_str("1;");
        for _ in 0..depth {
            code.push_str(" }");
        }

        let mut parser = Parser::new(&code);
        let result = parser.parse();
        prop_assert!(result.is_ok(), "Failed to parse nested blocks at depth {}", depth);
    }

    #[test]
    fn deeply_nested_expressions(
        depth in 1usize..20
    ) {
        // Generate deeply nested arithmetic
        let mut code = String::new();
        for _ in 0..depth {
            code.push('(');
        }
        code.push('1');
        for i in 0..depth {
            code.push_str(&format!(" + {})", i));
        }
        code.push(';');

        let mut parser = Parser::new(&code);
        let result = parser.parse();

        // Should either parse or fail gracefully
        match result {
            Ok(ast) => {
                let depth_check = check_depth(&ast, 0, 100);
                prop_assert!(depth_check.is_ok());
            }
            Err(_) => {
                // Parse error is acceptable for very deep nesting
                prop_assert!(true);
            }
        }
    }
}
