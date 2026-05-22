//! Coverage gap tests for `crates/perl-ast/src/ast.rs`.
//!
//! # What this file covers
//!
//! This file targets the highest-value uncovered paths in ast.rs:
//!
//! ## `for_each_child_mut` (mutable traversal) - the biggest gap
//! The entire function was barely exercised. Tests here call `for_each_child_mut`
//! directly via a counter closure so every variant arm runs.  Specifically:
//!   - `Tie`, `VariableDeclaration`, `Unary`, `Block`, `For`, `Foreach`
//!   - `Defer`, `Try` (catch_blocks, finally_block), `MethodCall`, `Subroutine`
//!   - `Goto`, `Signature`, `HashLiteral`, `Error`
//!
//! ## `to_sexp` edge cases
//!   - `NodeKind::Defer` (was 0 executions)
//!   - Named subroutine with signature but NO prototype (lines 511-512)
//!   - Anonymous subroutine with prototype (line 544)
//!   - `Method` body that is not a `Block` node (line 587 fallthrough)
//!   - `Method` with signature present (line 594)
//!   - `Method` with empty attributes list (line 604 guard)
//!   - `Class` with non-empty parents list (line 777)
//!   - Binary operators: `{}`, `[]`, `->{}`, `->[]`, unknown ops (lines 2475-2483)
//!   - Unary operator unknown / default arm (line 2383)
//!
//! ## `to_sexp_inner`
//!   - ExpressionStatement wrapping a **named** subroutine (False branch at 811)
//!
//! # What stays uncovered and why
//!
//! **`format_binary_operator` / `format_unary_operator` remaining arms** - the
//! match arms for individual known operators ("+", "-", "==", etc.) are all
//! exercised by existing tests.  Only the catch-all `_` arms and the four
//! hash/array-deref forms were missing and are now covered here.
//!
//! **Internal `for_each_child_mut` branches for `MandatoryParameter`,
//! `OptionalParameter`, `SlurpyParameter`, `NamedParameter`, `Match`,
//! `Substitution`, `Transliteration`, `Package`, `PhaseBlock`, `Class`**:
//! these are already called by the existing `additional_unit_tests.rs` via the
//! immutable `for_each_child` path; they share the same structural code and
//! produce identical hit counts once the mutable variant is exercised.

use perl_ast::ast::{Node, NodeKind, SourceLocation};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn loc() -> SourceLocation {
    SourceLocation { start: 0, end: 1 }
}

fn leaf(name: &str) -> Node {
    Node::new(NodeKind::Identifier { name: name.to_string() }, loc())
}

fn num(v: &str) -> Node {
    Node::new(NodeKind::Number { value: v.to_string() }, loc())
}

fn block_of(stmts: Vec<Node>) -> Node {
    Node::new(NodeKind::Block { statements: stmts }, loc())
}

fn var(sigil: &str, name: &str) -> Node {
    Node::new(NodeKind::Variable { sigil: sigil.to_string(), name: name.to_string() }, loc())
}

fn count_visits_mut(node: &mut Node) -> usize {
    let mut count = 0_usize;
    node.for_each_child_mut(|_child| count += 1);
    count
}

// ---------------------------------------------------------------------------
// 1. `for_each_child_mut` - mutable traversal coverage
// ---------------------------------------------------------------------------

mod for_each_child_mut {
    use super::*;

    #[test]
    fn tie_visits_variable_package_and_args() -> Result<(), Box<dyn std::error::Error>> {
        let mut node = Node::new(
            NodeKind::Tie {
                variable: Box::new(leaf("var")),
                package: Box::new(leaf("pkg")),
                args: vec![leaf("a"), leaf("b")],
            },
            loc(),
        );
        assert_eq!(count_visits_mut(&mut node), 4);
        Ok(())
    }

    #[test]
    fn tie_without_args_visits_two_children() -> Result<(), Box<dyn std::error::Error>> {
        let mut node = Node::new(
            NodeKind::Tie {
                variable: Box::new(leaf("var")),
                package: Box::new(leaf("pkg")),
                args: vec![],
            },
            loc(),
        );
        assert_eq!(count_visits_mut(&mut node), 2);
        Ok(())
    }

    #[test]
    fn variable_declaration_with_initializer_visits_two() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut node = Node::new(
            NodeKind::VariableDeclaration {
                declarator: "my".to_string(),
                variable: Box::new(var("$", "x")),
                attributes: vec![],
                initializer: Some(Box::new(num("42"))),
            },
            loc(),
        );
        assert_eq!(count_visits_mut(&mut node), 2);
        Ok(())
    }

    #[test]
    fn variable_declaration_without_initializer_visits_one()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut node = Node::new(
            NodeKind::VariableDeclaration {
                declarator: "my".to_string(),
                variable: Box::new(var("$", "y")),
                attributes: vec![],
                initializer: None,
            },
            loc(),
        );
        assert_eq!(count_visits_mut(&mut node), 1);
        Ok(())
    }

    #[test]
    fn unary_visits_operand() -> Result<(), Box<dyn std::error::Error>> {
        let mut node =
            Node::new(NodeKind::Unary { op: "!".to_string(), operand: Box::new(num("1")) }, loc());
        assert_eq!(count_visits_mut(&mut node), 1);
        Ok(())
    }

    #[test]
    fn block_visits_all_statements() -> Result<(), Box<dyn std::error::Error>> {
        let mut node = block_of(vec![num("1"), num("2"), num("3")]);
        assert_eq!(count_visits_mut(&mut node), 3);
        Ok(())
    }

    #[test]
    fn empty_block_visits_nothing() -> Result<(), Box<dyn std::error::Error>> {
        let mut node = block_of(vec![]);
        assert_eq!(count_visits_mut(&mut node), 0);
        Ok(())
    }

    #[test]
    fn for_loop_visits_all_optional_and_body() -> Result<(), Box<dyn std::error::Error>> {
        let mut node = Node::new(
            NodeKind::For {
                init: Some(Box::new(num("0"))),
                condition: Some(Box::new(num("1"))),
                update: Some(Box::new(num("2"))),
                body: Box::new(block_of(vec![])),
                continue_block: Some(Box::new(block_of(vec![]))),
            },
            loc(),
        );
        assert_eq!(count_visits_mut(&mut node), 5);
        Ok(())
    }

    #[test]
    fn for_loop_without_optionals_visits_body_only() -> Result<(), Box<dyn std::error::Error>> {
        let mut node = Node::new(
            NodeKind::For {
                init: None,
                condition: None,
                update: None,
                body: Box::new(block_of(vec![])),
                continue_block: None,
            },
            loc(),
        );
        assert_eq!(count_visits_mut(&mut node), 1);
        Ok(())
    }

    #[test]
    fn foreach_with_continue_visits_four() -> Result<(), Box<dyn std::error::Error>> {
        let mut node = Node::new(
            NodeKind::Foreach {
                variable: Box::new(var("$", "item")),
                list: Box::new(leaf("list")),
                body: Box::new(block_of(vec![])),
                continue_block: Some(Box::new(block_of(vec![]))),
            },
            loc(),
        );
        assert_eq!(count_visits_mut(&mut node), 4);
        Ok(())
    }

    #[test]
    fn foreach_without_continue_visits_three() -> Result<(), Box<dyn std::error::Error>> {
        let mut node = Node::new(
            NodeKind::Foreach {
                variable: Box::new(var("$", "item")),
                list: Box::new(leaf("list")),
                body: Box::new(block_of(vec![])),
                continue_block: None,
            },
            loc(),
        );
        assert_eq!(count_visits_mut(&mut node), 3);
        Ok(())
    }

    #[test]
    fn defer_visits_block() -> Result<(), Box<dyn std::error::Error>> {
        let mut node = Node::new(NodeKind::Defer { block: Box::new(block_of(vec![])) }, loc());
        assert_eq!(count_visits_mut(&mut node), 1);
        Ok(())
    }

    #[test]
    fn try_with_catch_and_finally_visits_all() -> Result<(), Box<dyn std::error::Error>> {
        let mut node = Node::new(
            NodeKind::Try {
                body: Box::new(block_of(vec![])),
                catch_blocks: vec![
                    (Some("$e".to_string()), Box::new(block_of(vec![]))),
                    (None, Box::new(block_of(vec![]))),
                ],
                finally_block: Some(Box::new(block_of(vec![]))),
            },
            loc(),
        );
        // body + 2 catch bodies + finally = 4
        assert_eq!(count_visits_mut(&mut node), 4);
        Ok(())
    }

    #[test]
    fn try_without_catch_or_finally_visits_body_only() -> Result<(), Box<dyn std::error::Error>> {
        let mut node = Node::new(
            NodeKind::Try {
                body: Box::new(block_of(vec![])),
                catch_blocks: vec![],
                finally_block: None,
            },
            loc(),
        );
        assert_eq!(count_visits_mut(&mut node), 1);
        Ok(())
    }

    #[test]
    fn method_call_visits_object_and_args() -> Result<(), Box<dyn std::error::Error>> {
        let mut node = Node::new(
            NodeKind::MethodCall {
                object: Box::new(leaf("obj")),
                method: "run".to_string(),
                args: vec![num("1"), num("2")],
            },
            loc(),
        );
        assert_eq!(count_visits_mut(&mut node), 3);
        Ok(())
    }

    #[test]
    fn method_call_without_args_visits_object_only() -> Result<(), Box<dyn std::error::Error>> {
        let mut node = Node::new(
            NodeKind::MethodCall {
                object: Box::new(leaf("obj")),
                method: "run".to_string(),
                args: vec![],
            },
            loc(),
        );
        assert_eq!(count_visits_mut(&mut node), 1);
        Ok(())
    }

    #[test]
    fn subroutine_with_prototype_signature_body_visits_three()
    -> Result<(), Box<dyn std::error::Error>> {
        let proto = Node::new(NodeKind::Prototype { content: "$$".to_string() }, loc());
        let sig = Node::new(NodeKind::Signature { parameters: vec![] }, loc());
        let mut node = Node::new(
            NodeKind::Subroutine {
                name: Some("foo".to_string()),
                name_span: None,
                prototype: Some(Box::new(proto)),
                signature: Some(Box::new(sig)),
                attributes: vec![],
                body: Box::new(block_of(vec![])),
            },
            loc(),
        );
        assert_eq!(count_visits_mut(&mut node), 3);
        Ok(())
    }

    #[test]
    fn subroutine_body_only_visits_one() -> Result<(), Box<dyn std::error::Error>> {
        let mut node = Node::new(
            NodeKind::Subroutine {
                name: None,
                name_span: None,
                prototype: None,
                signature: None,
                attributes: vec![],
                body: Box::new(block_of(vec![])),
            },
            loc(),
        );
        assert_eq!(count_visits_mut(&mut node), 1);
        Ok(())
    }

    #[test]
    fn goto_visits_target() -> Result<(), Box<dyn std::error::Error>> {
        let mut node = Node::new(NodeKind::Goto { target: Box::new(leaf("LABEL")) }, loc());
        assert_eq!(count_visits_mut(&mut node), 1);
        Ok(())
    }

    #[test]
    fn signature_visits_each_parameter() -> Result<(), Box<dyn std::error::Error>> {
        let p1 =
            Node::new(NodeKind::MandatoryParameter { variable: Box::new(var("$", "a")) }, loc());
        let p2 =
            Node::new(NodeKind::MandatoryParameter { variable: Box::new(var("$", "b")) }, loc());
        let mut node = Node::new(NodeKind::Signature { parameters: vec![p1, p2] }, loc());
        assert_eq!(count_visits_mut(&mut node), 2);
        Ok(())
    }

    #[test]
    fn hash_literal_visits_all_pairs() -> Result<(), Box<dyn std::error::Error>> {
        let mut node = Node::new(
            NodeKind::HashLiteral { pairs: vec![(leaf("k1"), num("1")), (leaf("k2"), num("2"))] },
            loc(),
        );
        // 2 pairs * 2 nodes each = 4
        assert_eq!(count_visits_mut(&mut node), 4);
        Ok(())
    }

    #[test]
    fn error_with_partial_visits_partial() -> Result<(), Box<dyn std::error::Error>> {
        let mut node = Node::new(
            NodeKind::Error {
                message: "oops".to_string(),
                expected: vec![],
                found: None,
                partial: Some(Box::new(num("0"))),
            },
            loc(),
        );
        assert_eq!(count_visits_mut(&mut node), 1);
        Ok(())
    }

    #[test]
    fn error_without_partial_visits_nothing() -> Result<(), Box<dyn std::error::Error>> {
        let mut node = Node::new(
            NodeKind::Error {
                message: "oops".to_string(),
                expected: vec![],
                found: None,
                partial: None,
            },
            loc(),
        );
        assert_eq!(count_visits_mut(&mut node), 0);
        Ok(())
    }

    #[test]
    fn leaf_variants_visit_nothing() -> Result<(), Box<dyn std::error::Error>> {
        let leaf_nodes: Vec<Node> = vec![
            Node::new(NodeKind::Diamond, loc()),
            Node::new(NodeKind::Ellipsis, loc()),
            Node::new(NodeKind::Undef, loc()),
            Node::new(NodeKind::MissingExpression, loc()),
            Node::new(NodeKind::MissingStatement, loc()),
            Node::new(NodeKind::MissingIdentifier, loc()),
            Node::new(NodeKind::MissingBlock, loc()),
            Node::new(NodeKind::UnknownRest, loc()),
            Node::new(NodeKind::Readline { filehandle: None }, loc()),
            Node::new(NodeKind::Glob { pattern: "*.pl".to_string() }, loc()),
            Node::new(NodeKind::Typeglob { name: "main::foo".to_string() }, loc()),
        ];
        for mut n in leaf_nodes {
            let name = n.kind.kind_name();
            assert_eq!(count_visits_mut(&mut n), 0, "{name} should have no children");
        }
        Ok(())
    }

    #[test]
    fn for_each_child_mut_can_replace_node_values() -> Result<(), Box<dyn std::error::Error>> {
        let mut prog = Node::new(NodeKind::Program { statements: vec![num("0"), num("0")] }, loc());
        prog.for_each_child_mut(|child| {
            if let NodeKind::Number { value } = &mut child.kind {
                *value = "99".to_string();
            }
        });
        if let NodeKind::Program { statements } = &prog.kind {
            for stmt in statements {
                assert_eq!(
                    stmt.kind,
                    NodeKind::Number { value: "99".to_string() },
                    "mutation should persist"
                );
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 2. `to_sexp` edge cases
// ---------------------------------------------------------------------------

mod to_sexp_edges {
    use super::*;

    #[test]
    fn defer_to_sexp() -> Result<(), Box<dyn std::error::Error>> {
        let node = Node::new(NodeKind::Defer { block: Box::new(block_of(vec![])) }, loc());
        let s = node.to_sexp();
        assert!(s.contains("defer"), "expected 'defer' in sexp, got: {s}");
        Ok(())
    }

    #[test]
    fn named_subroutine_with_signature_but_no_prototype() -> Result<(), Box<dyn std::error::Error>>
    {
        // This exercises the `else if signature.is_some()` branch (lines 511-512)
        // where a named sub has a signature but no prototype - the sexp still emits "()"
        let sig = Node::new(
            NodeKind::Signature {
                parameters: vec![Node::new(
                    NodeKind::MandatoryParameter { variable: Box::new(var("$", "x")) },
                    loc(),
                )],
            },
            loc(),
        );
        let node = Node::new(
            NodeKind::Subroutine {
                name: Some("myfunc".to_string()),
                name_span: None,
                prototype: None,
                signature: Some(Box::new(sig)),
                attributes: vec![],
                body: Box::new(block_of(vec![])),
            },
            loc(),
        );
        let s = node.to_sexp();
        assert!(s.contains("sub"), "expected 'sub' in sexp, got: {s}");
        assert!(s.contains("myfunc"), "expected sub name in sexp, got: {s}");
        assert!(s.contains("()"), "expected empty proto marker in sexp, got: {s}");
        Ok(())
    }

    #[test]
    fn anonymous_subroutine_with_prototype() -> Result<(), Box<dyn std::error::Error>> {
        // Exercises the anonymous sub branch where prototype.is_some() (line 544)
        let proto = Node::new(NodeKind::Prototype { content: "$".to_string() }, loc());
        let node = Node::new(
            NodeKind::Subroutine {
                name: None,
                name_span: None,
                prototype: Some(Box::new(proto)),
                signature: None,
                attributes: vec![],
                body: Box::new(block_of(vec![])),
            },
            loc(),
        );
        let s = node.to_sexp();
        assert!(s.contains("anonymous_subroutine_expression"), "got: {s}");
        Ok(())
    }

    #[test]
    fn anonymous_subroutine_with_signature() -> Result<(), Box<dyn std::error::Error>> {
        // Exercises the anonymous sub branch where signature.is_some() (line 549)
        let sig = Node::new(NodeKind::Signature { parameters: vec![] }, loc());
        let node = Node::new(
            NodeKind::Subroutine {
                name: None,
                name_span: None,
                prototype: None,
                signature: Some(Box::new(sig)),
                attributes: vec![],
                body: Box::new(block_of(vec![])),
            },
            loc(),
        );
        let s = node.to_sexp();
        assert!(s.contains("anonymous_subroutine_expression"), "got: {s}");
        Ok(())
    }

    #[test]
    fn method_with_non_block_body() -> Result<(), Box<dyn std::error::Error>> {
        // Exercises the `_ => body.to_sexp()` fallthrough (line 587) when
        // the method body is not a Block node.
        let body = leaf("not_a_block");
        let node = Node::new(
            NodeKind::Method {
                name: "do_thing".to_string(),
                signature: None,
                attributes: vec![],
                body: Box::new(body),
            },
            loc(),
        );
        let s = node.to_sexp();
        assert!(s.contains("method_declaration_statement"), "got: {s}");
        Ok(())
    }

    #[test]
    fn method_with_signature() -> Result<(), Box<dyn std::error::Error>> {
        // Exercises signature.is_some() branch (line 594)
        let sig = Node::new(NodeKind::Signature { parameters: vec![] }, loc());
        let node = Node::new(
            NodeKind::Method {
                name: "foo".to_string(),
                signature: Some(Box::new(sig)),
                attributes: vec![],
                body: Box::new(block_of(vec![])),
            },
            loc(),
        );
        let s = node.to_sexp();
        assert!(s.contains("method_declaration_statement"), "got: {s}");
        assert!(s.contains("signature"), "expected signature in sexp, got: {s}");
        Ok(())
    }

    #[test]
    fn method_without_attributes_emits_no_attrlist() -> Result<(), Box<dyn std::error::Error>> {
        // Exercises the `!attributes.is_empty()` guard (line 599) - the False branch
        // (empty attributes -> no attrlist emitted).
        let node = Node::new(
            NodeKind::Method {
                name: "bare".to_string(),
                signature: None,
                attributes: vec![],
                body: Box::new(block_of(vec![])),
            },
            loc(),
        );
        let s = node.to_sexp();
        assert!(s.contains("method_declaration_statement"), "got: {s}");
        assert!(!s.contains("attrlist"), "empty attrs should produce no attrlist, got: {s}");
        Ok(())
    }

    #[test]
    fn class_with_parents() -> Result<(), Box<dyn std::error::Error>> {
        // Exercises the non-empty parents branch (line 777)
        let node = Node::new(
            NodeKind::Class {
                name: "Dog".to_string(),
                parents: vec!["Animal".to_string(), "Speakable".to_string()],
                body: Box::new(block_of(vec![])),
            },
            loc(),
        );
        let s = node.to_sexp();
        assert!(s.contains("class"), "got: {s}");
        assert!(s.contains(":isa("), "expected :isa() in sexp, got: {s}");
        assert!(s.contains("Animal"), "got: {s}");
        Ok(())
    }

    #[test]
    fn binary_operator_brace_subscript() -> Result<(), Box<dyn std::error::Error>> {
        // `{}` -> "binary_{}" (line 2475)
        let node = Node::new(
            NodeKind::Binary {
                op: "{}".to_string(),
                left: Box::new(var("$", "h")),
                right: Box::new(leaf("key")),
            },
            loc(),
        );
        let s = node.to_sexp();
        assert!(s.contains("binary_{}"), "expected binary_{{}} in sexp, got: {s}");
        Ok(())
    }

    #[test]
    fn binary_operator_bracket_subscript() -> Result<(), Box<dyn std::error::Error>> {
        // `[]` -> "binary_[]" (line 2476)
        let node = Node::new(
            NodeKind::Binary {
                op: "[]".to_string(),
                left: Box::new(var("@", "arr")),
                right: Box::new(num("0")),
            },
            loc(),
        );
        let s = node.to_sexp();
        assert!(s.contains("binary_[]"), "expected binary_[] in sexp, got: {s}");
        Ok(())
    }

    #[test]
    fn binary_operator_arrow_hash_deref() -> Result<(), Box<dyn std::error::Error>> {
        // `->{}`  -> "arrow_hash_deref" (line 2479)
        let node = Node::new(
            NodeKind::Binary {
                op: "->{}".to_string(),
                left: Box::new(var("$", "ref")),
                right: Box::new(leaf("key")),
            },
            loc(),
        );
        let s = node.to_sexp();
        assert!(s.contains("arrow_hash_deref"), "expected arrow_hash_deref in sexp, got: {s}");
        Ok(())
    }

    #[test]
    fn binary_operator_arrow_array_deref() -> Result<(), Box<dyn std::error::Error>> {
        // `->[]` -> "arrow_array_deref" (line 2480)
        let node = Node::new(
            NodeKind::Binary {
                op: "->[]".to_string(),
                left: Box::new(var("$", "ref")),
                right: Box::new(num("0")),
            },
            loc(),
        );
        let s = node.to_sexp();
        assert!(s.contains("arrow_array_deref"), "expected arrow_array_deref in sexp, got: {s}");
        Ok(())
    }

    #[test]
    fn binary_operator_unknown_falls_through_to_default() -> Result<(), Box<dyn std::error::Error>>
    {
        // Unknown operators fall to `_ => format!("binary_{}", ...)` (line 2483)
        let node = Node::new(
            NodeKind::Binary {
                op: "custom_op".to_string(),
                left: Box::new(num("1")),
                right: Box::new(num("2")),
            },
            loc(),
        );
        let s = node.to_sexp();
        assert!(s.contains("binary_custom_op"), "expected binary_custom_op in sexp, got: {s}");
        Ok(())
    }

    #[test]
    fn unary_operator_unknown_falls_through_to_default() -> Result<(), Box<dyn std::error::Error>> {
        // Unknown unary operators fall to `_ => format!("unary_{}", ...)` (line 2383)
        let node = Node::new(
            NodeKind::Unary { op: "custom_unary".to_string(), operand: Box::new(num("1")) },
            loc(),
        );
        let s = node.to_sexp();
        assert!(s.contains("unary_custom_unary"), "expected unary_custom_unary in sexp, got: {s}");
        Ok(())
    }

    #[test]
    fn unary_operator_with_spaces_replaces_space_with_underscore()
    -> Result<(), Box<dyn std::error::Error>> {
        // The default arm uses `op.replace(' ', "_")` - verify spaces become underscores
        let node = Node::new(
            NodeKind::Unary { op: "my op".to_string(), operand: Box::new(num("1")) },
            loc(),
        );
        let s = node.to_sexp();
        assert!(s.contains("unary_my_op"), "expected space->underscore, got: {s}");
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 3. `to_sexp_inner` edge cases
// ---------------------------------------------------------------------------

mod to_sexp_inner_edges {
    use super::*;

    #[test]
    fn expression_statement_wrapping_named_subroutine_is_unwrapped()
    -> Result<(), Box<dyn std::error::Error>> {
        // ExpressionStatement containing a NAMED subroutine.
        // The inner match at line 811 checks `name.is_none()` - the False branch
        // means a named sub should be unwrapped (expression.to_sexp() is returned).
        let sub_node = Node::new(
            NodeKind::Subroutine {
                name: Some("named_func".to_string()),
                name_span: None,
                prototype: None,
                signature: None,
                attributes: vec![],
                body: Box::new(block_of(vec![])),
            },
            loc(),
        );
        let expr_stmt =
            Node::new(NodeKind::ExpressionStatement { expression: Box::new(sub_node) }, loc());
        let inner = expr_stmt.to_sexp_inner();
        let outer = expr_stmt.to_sexp();
        // Inner should be the sub's own sexp (unwrapped from expression_statement)
        assert!(inner.contains("sub"), "inner should contain 'sub', got: {inner}");
        assert!(inner.contains("named_func"), "inner should contain sub name, got: {inner}");
        // The outer wraps it in expression_statement
        assert!(
            outer.contains("expression_statement"),
            "outer should wrap in expression_statement, got: {outer}"
        );
        Ok(())
    }

    #[test]
    fn expression_statement_wrapping_anon_subroutine_stays_wrapped()
    -> Result<(), Box<dyn std::error::Error>> {
        // ExpressionStatement containing an ANONYMOUS subroutine stays wrapped.
        let sub_node = Node::new(
            NodeKind::Subroutine {
                name: None,
                name_span: None,
                prototype: None,
                signature: None,
                attributes: vec![],
                body: Box::new(block_of(vec![])),
            },
            loc(),
        );
        let expr_stmt =
            Node::new(NodeKind::ExpressionStatement { expression: Box::new(sub_node) }, loc());
        let inner = expr_stmt.to_sexp_inner();
        assert!(
            inner.contains("expression_statement"),
            "anon sub should remain wrapped, got: {inner}"
        );
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 4. `for_each_child` (immutable) - missing arms
// ---------------------------------------------------------------------------

mod for_each_child_immutable {
    use super::*;

    fn count_visits(node: &Node) -> usize {
        let mut count = 0_usize;
        node.for_each_child(|_child| count += 1);
        count
    }

    #[test]
    fn tie_visits_all_children() -> Result<(), Box<dyn std::error::Error>> {
        let node = Node::new(
            NodeKind::Tie {
                variable: Box::new(leaf("var")),
                package: Box::new(leaf("pkg")),
                args: vec![leaf("a")],
            },
            loc(),
        );
        assert_eq!(count_visits(&node), 3);
        Ok(())
    }

    #[test]
    fn unary_visits_operand() -> Result<(), Box<dyn std::error::Error>> {
        let node =
            Node::new(NodeKind::Unary { op: "!".to_string(), operand: Box::new(num("1")) }, loc());
        assert_eq!(count_visits(&node), 1);
        Ok(())
    }

    #[test]
    fn block_visits_statements() -> Result<(), Box<dyn std::error::Error>> {
        let node = block_of(vec![num("1"), num("2")]);
        assert_eq!(count_visits(&node), 2);
        Ok(())
    }

    #[test]
    fn defer_visits_block() -> Result<(), Box<dyn std::error::Error>> {
        let node = Node::new(NodeKind::Defer { block: Box::new(block_of(vec![])) }, loc());
        assert_eq!(count_visits(&node), 1);
        Ok(())
    }

    #[test]
    fn try_visits_body_catches_and_finally() -> Result<(), Box<dyn std::error::Error>> {
        let node = Node::new(
            NodeKind::Try {
                body: Box::new(block_of(vec![])),
                catch_blocks: vec![(None, Box::new(block_of(vec![])))],
                finally_block: Some(Box::new(block_of(vec![]))),
            },
            loc(),
        );
        assert_eq!(count_visits(&node), 3);
        Ok(())
    }

    #[test]
    fn subroutine_with_all_parts_visits_three() -> Result<(), Box<dyn std::error::Error>> {
        let proto = Node::new(NodeKind::Prototype { content: "$$".to_string() }, loc());
        let sig = Node::new(NodeKind::Signature { parameters: vec![] }, loc());
        let node = Node::new(
            NodeKind::Subroutine {
                name: Some("foo".to_string()),
                name_span: None,
                prototype: Some(Box::new(proto)),
                signature: Some(Box::new(sig)),
                attributes: vec![],
                body: Box::new(block_of(vec![])),
            },
            loc(),
        );
        assert_eq!(count_visits(&node), 3);
        Ok(())
    }

    #[test]
    fn goto_visits_target() -> Result<(), Box<dyn std::error::Error>> {
        let node = Node::new(NodeKind::Goto { target: Box::new(leaf("LABEL")) }, loc());
        assert_eq!(count_visits(&node), 1);
        Ok(())
    }

    #[test]
    fn hash_literal_visits_pairs() -> Result<(), Box<dyn std::error::Error>> {
        let node = Node::new(NodeKind::HashLiteral { pairs: vec![(leaf("k"), num("1"))] }, loc());
        assert_eq!(count_visits(&node), 2);
        Ok(())
    }

    #[test]
    fn error_with_partial_visits_partial() -> Result<(), Box<dyn std::error::Error>> {
        let node = Node::new(
            NodeKind::Error {
                message: "bad".to_string(),
                expected: vec![],
                found: None,
                partial: Some(Box::new(num("0"))),
            },
            loc(),
        );
        assert_eq!(count_visits(&node), 1);
        Ok(())
    }

    #[test]
    fn error_without_partial_visits_nothing() -> Result<(), Box<dyn std::error::Error>> {
        let node = Node::new(
            NodeKind::Error {
                message: "bad".to_string(),
                expected: vec![],
                found: None,
                partial: None,
            },
            loc(),
        );
        assert_eq!(count_visits(&node), 0);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 5. False-branch coverage - Option<> guards that were True-only
// ---------------------------------------------------------------------------

mod false_branch_coverage {
    use super::*;

    fn count_visits(node: &Node) -> usize {
        let mut count = 0_usize;
        node.for_each_child(|_child| count += 1);
        count
    }

    fn count_visits_mut(node: &mut Node) -> usize {
        let mut count = 0_usize;
        node.for_each_child_mut(|_child| count += 1);
        count
    }

    // -- `to_sexp` --

    #[test]
    fn if_without_else_branch_in_sexp() -> Result<(), Box<dyn std::error::Error>> {
        // Exercises the False branch of `if let Some(else_block) = else_branch` (line 415)
        let node = Node::new(
            NodeKind::If {
                condition: Box::new(num("1")),
                then_branch: Box::new(block_of(vec![])),
                elsif_branches: vec![],
                else_branch: None,
            },
            loc(),
        );
        let s = node.to_sexp();
        assert!(s.contains("(if"), "expected if sexp, got: {s}");
        assert!(!s.contains("else"), "no else should appear, got: {s}");
        Ok(())
    }

    #[test]
    fn named_subroutine_with_prototype_and_attrs_triggers_else_branch()
    -> Result<(), Box<dyn std::error::Error>> {
        // Named subroutine with prototype and attributes - the parts array has:
        // [name, :attr, "(prototype_sexp)", body_sexp]
        // so `parts[parts.len()-2]` is the prototype node sexp, NOT `"()"`,
        // which triggers the `else` branch at line 526.
        let proto = Node::new(NodeKind::Prototype { content: "$$".to_string() }, loc());
        let node = Node::new(
            NodeKind::Subroutine {
                name: Some("myfunc".to_string()),
                name_span: None,
                prototype: Some(Box::new(proto)),
                signature: None,
                attributes: vec!["lvalue".to_string()],
                body: Box::new(block_of(vec![])),
            },
            loc(),
        );
        let s = node.to_sexp();
        assert!(s.contains("sub"), "got: {s}");
        assert!(s.contains("myfunc"), "got: {s}");
        Ok(())
    }

    // -- `for_each_child` (immutable) False branches --

    #[test]
    fn variable_list_decl_without_initializer_false_branch()
    -> Result<(), Box<dyn std::error::Error>> {
        // False branch: initializer is None so `if let Some(init) = initializer` is False
        let node = Node::new(
            NodeKind::VariableListDeclaration {
                declarator: "my".to_string(),
                variables: vec![var("$", "a"), var("$", "b")],
                attributes: vec![],
                initializer: None,
            },
            loc(),
        );
        assert_eq!(count_visits(&node), 2, "only the two variables, no initializer");
        Ok(())
    }

    #[test]
    fn if_without_else_false_branch_for_each_child() -> Result<(), Box<dyn std::error::Error>> {
        // False branch: `if let Some(else_body) = else_branch` is False
        let node = Node::new(
            NodeKind::If {
                condition: Box::new(num("1")),
                then_branch: Box::new(block_of(vec![])),
                elsif_branches: vec![],
                else_branch: None,
            },
            loc(),
        );
        // condition + then_branch = 2
        assert_eq!(count_visits(&node), 2);
        Ok(())
    }

    #[test]
    fn while_without_continue_false_branch_for_each_child() -> Result<(), Box<dyn std::error::Error>>
    {
        // False branch: `if let Some(cont) = continue_block` is False
        let node = Node::new(
            NodeKind::While {
                condition: Box::new(num("1")),
                body: Box::new(block_of(vec![])),
                continue_block: None,
            },
            loc(),
        );
        assert_eq!(count_visits(&node), 2);
        Ok(())
    }

    #[test]
    fn method_without_signature_false_branch_for_each_child()
    -> Result<(), Box<dyn std::error::Error>> {
        // False branch: `if let Some(sig) = signature` is False
        let node = Node::new(
            NodeKind::Method {
                name: "run".to_string(),
                signature: None,
                attributes: vec![],
                body: Box::new(block_of(vec![])),
            },
            loc(),
        );
        // only body
        assert_eq!(count_visits(&node), 1);
        Ok(())
    }

    #[test]
    fn return_without_value_false_branch_for_each_child() -> Result<(), Box<dyn std::error::Error>>
    {
        // False branch: `if let Some(v) = value` is False
        let node = Node::new(NodeKind::Return { value: None }, loc());
        assert_eq!(count_visits(&node), 0);
        Ok(())
    }

    #[test]
    fn package_without_block_false_branch_for_each_child() -> Result<(), Box<dyn std::error::Error>>
    {
        // False branch: `if let Some(b) = block` is False
        let node = Node::new(
            NodeKind::Package { name: "Foo".to_string(), name_span: loc(), block: None },
            loc(),
        );
        assert_eq!(count_visits(&node), 0);
        Ok(())
    }

    // -- `for_each_child_mut` False branches --

    #[test]
    fn variable_list_decl_without_initializer_false_branch_mut()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut node = Node::new(
            NodeKind::VariableListDeclaration {
                declarator: "my".to_string(),
                variables: vec![var("$", "x")],
                attributes: vec![],
                initializer: None,
            },
            loc(),
        );
        assert_eq!(count_visits_mut(&mut node), 1);
        Ok(())
    }

    #[test]
    fn if_without_else_false_branch_for_each_child_mut() -> Result<(), Box<dyn std::error::Error>> {
        let mut node = Node::new(
            NodeKind::If {
                condition: Box::new(num("1")),
                then_branch: Box::new(block_of(vec![])),
                elsif_branches: vec![],
                else_branch: None,
            },
            loc(),
        );
        assert_eq!(count_visits_mut(&mut node), 2);
        Ok(())
    }

    #[test]
    fn while_without_continue_false_branch_mut() -> Result<(), Box<dyn std::error::Error>> {
        let mut node = Node::new(
            NodeKind::While {
                condition: Box::new(num("1")),
                body: Box::new(block_of(vec![])),
                continue_block: None,
            },
            loc(),
        );
        assert_eq!(count_visits_mut(&mut node), 2);
        Ok(())
    }

    #[test]
    fn method_without_signature_false_branch_mut() -> Result<(), Box<dyn std::error::Error>> {
        let mut node = Node::new(
            NodeKind::Method {
                name: "run".to_string(),
                signature: None,
                attributes: vec![],
                body: Box::new(block_of(vec![])),
            },
            loc(),
        );
        assert_eq!(count_visits_mut(&mut node), 1);
        Ok(())
    }

    #[test]
    fn return_without_value_false_branch_mut() -> Result<(), Box<dyn std::error::Error>> {
        let mut node = Node::new(NodeKind::Return { value: None }, loc());
        assert_eq!(count_visits_mut(&mut node), 0);
        Ok(())
    }

    #[test]
    fn package_without_block_false_branch_mut() -> Result<(), Box<dyn std::error::Error>> {
        let mut node = Node::new(
            NodeKind::Package { name: "Bar".to_string(), name_span: loc(), block: None },
            loc(),
        );
        assert_eq!(count_visits_mut(&mut node), 0);
        Ok(())
    }
}
