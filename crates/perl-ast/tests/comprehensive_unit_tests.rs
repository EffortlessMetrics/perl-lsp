//! Comprehensive unit tests for the `perl-ast` crate.
//!
//! Covers node construction, tree building, to_sexp() formatting,
//! node traversal, NodeKind enum coverage, and edge cases.

use perl_ast::ast::{Node, NodeKind, SourceLocation};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn loc(start: usize, end: usize) -> SourceLocation {
    SourceLocation { start, end }
}

fn num_node(value: &str) -> Node {
    Node::new(NodeKind::Number { value: value.to_string() }, loc(0, value.len()))
}

fn var_node(sigil: &str, name: &str) -> Node {
    Node::new(
        NodeKind::Variable { sigil: sigil.to_string(), name: name.to_string() },
        loc(0, sigil.len() + name.len()),
    )
}

fn ident_node(name: &str) -> Node {
    Node::new(NodeKind::Identifier { name: name.to_string() }, loc(0, name.len()))
}

fn block_node(statements: Vec<Node>) -> Node {
    Node::new(NodeKind::Block { statements }, loc(0, 1))
}

fn program_node(statements: Vec<Node>) -> Node {
    Node::new(NodeKind::Program { statements }, loc(0, 100))
}

// ===========================================================================
// 1. Node construction
// ===========================================================================

#[test]
fn node_new_preserves_kind_and_location() -> Result<(), Box<dyn std::error::Error>> {
    let l = loc(5, 10);
    let node = Node::new(NodeKind::Number { value: "42".to_string() }, l);
    assert_eq!(node.location.start, 5);
    assert_eq!(node.location.end, 10);
    assert_eq!(node.kind.kind_name(), "Number");
    Ok(())
}

#[test]
fn variable_node_stores_sigil_and_name() -> Result<(), Box<dyn std::error::Error>> {
    let node = var_node("$", "foo");
    match &node.kind {
        NodeKind::Variable { sigil, name } => {
            assert_eq!(sigil, "$");
            assert_eq!(name, "foo");
        }
        other => return Err(format!("expected Variable, got {:?}", other.kind_name()).into()),
    }
    Ok(())
}

#[test]
fn string_node_interpolation_flag() -> Result<(), Box<dyn std::error::Error>> {
    let interp =
        Node::new(NodeKind::String { value: "hi".to_string(), interpolated: true }, loc(0, 4));
    let literal =
        Node::new(NodeKind::String { value: "hi".to_string(), interpolated: false }, loc(0, 4));
    match (&interp.kind, &literal.kind) {
        (
            NodeKind::String { interpolated: true, .. },
            NodeKind::String { interpolated: false, .. },
        ) => {}
        _ => return Err("interpolation flags wrong".into()),
    }
    Ok(())
}

// ===========================================================================
// 2. Tree building
// ===========================================================================

#[test]
fn program_with_multiple_statements() -> Result<(), Box<dyn std::error::Error>> {
    let prog = program_node(vec![num_node("1"), num_node("2"), num_node("3")]);
    match &prog.kind {
        NodeKind::Program { statements } => assert_eq!(statements.len(), 3),
        other => return Err(format!("expected Program, got {}", other.kind_name()).into()),
    }
    Ok(())
}

#[test]
fn variable_declaration_with_initializer() -> Result<(), Box<dyn std::error::Error>> {
    let decl = Node::new(
        NodeKind::VariableDeclaration {
            declarator: "my".to_string(),
            variable: Box::new(var_node("$", "x")),
            attributes: vec![],
            initializer: Some(Box::new(num_node("42"))),
        },
        loc(0, 11),
    );
    match &decl.kind {
        NodeKind::VariableDeclaration { declarator, initializer, .. } => {
            assert_eq!(declarator, "my");
            assert!(initializer.is_some());
        }
        other => {
            return Err(format!("expected VariableDeclaration, got {}", other.kind_name()).into());
        }
    }
    Ok(())
}

#[test]
fn variable_list_declaration() -> Result<(), Box<dyn std::error::Error>> {
    let decl = Node::new(
        NodeKind::VariableListDeclaration {
            declarator: "my".to_string(),
            variables: vec![var_node("$", "a"), var_node("$", "b")],
            attributes: vec![],
            initializer: None,
        },
        loc(0, 12),
    );
    match &decl.kind {
        NodeKind::VariableListDeclaration { variables, .. } => {
            assert_eq!(variables.len(), 2);
        }
        other => {
            return Err(
                format!("expected VariableListDeclaration, got {}", other.kind_name()).into()
            );
        }
    }
    Ok(())
}

#[test]
fn binary_expression_tree() -> Result<(), Box<dyn std::error::Error>> {
    let expr = Node::new(
        NodeKind::Binary {
            op: "+".to_string(),
            left: Box::new(num_node("1")),
            right: Box::new(num_node("2")),
        },
        loc(0, 5),
    );
    assert_eq!(expr.kind.kind_name(), "Binary");
    Ok(())
}

#[test]
fn ternary_expression() -> Result<(), Box<dyn std::error::Error>> {
    let expr = Node::new(
        NodeKind::Ternary {
            condition: Box::new(var_node("$", "x")),
            then_expr: Box::new(num_node("1")),
            else_expr: Box::new(num_node("0")),
        },
        loc(0, 10),
    );
    assert_eq!(expr.kind.kind_name(), "Ternary");
    Ok(())
}

#[test]
fn unary_expression() -> Result<(), Box<dyn std::error::Error>> {
    let expr = Node::new(
        NodeKind::Unary { op: "-".to_string(), operand: Box::new(num_node("5")) },
        loc(0, 2),
    );
    assert_eq!(expr.kind.kind_name(), "Unary");
    Ok(())
}

#[test]
fn if_with_elsif_and_else() -> Result<(), Box<dyn std::error::Error>> {
    let if_node = Node::new(
        NodeKind::If {
            condition: Box::new(var_node("$", "cond")),
            then_branch: Box::new(block_node(vec![num_node("1")])),
            elsif_branches: vec![(
                Box::new(var_node("$", "other")),
                Box::new(block_node(vec![num_node("2")])),
            )],
            else_branch: Some(Box::new(block_node(vec![num_node("3")]))),
        },
        loc(0, 50),
    );
    assert_eq!(if_node.kind.kind_name(), "If");
    Ok(())
}

#[test]
fn while_with_continue() -> Result<(), Box<dyn std::error::Error>> {
    let w = Node::new(
        NodeKind::While {
            condition: Box::new(num_node("1")),
            body: Box::new(block_node(vec![])),
            continue_block: Some(Box::new(block_node(vec![]))),
        },
        loc(0, 20),
    );
    assert_eq!(w.kind.kind_name(), "While");
    Ok(())
}

#[test]
fn for_loop_all_clauses() -> Result<(), Box<dyn std::error::Error>> {
    let f = Node::new(
        NodeKind::For {
            init: Some(Box::new(num_node("0"))),
            condition: Some(Box::new(num_node("10"))),
            update: Some(Box::new(num_node("1"))),
            body: Box::new(block_node(vec![])),
            continue_block: None,
        },
        loc(0, 30),
    );
    assert_eq!(f.kind.kind_name(), "For");
    Ok(())
}

#[test]
fn foreach_loop() -> Result<(), Box<dyn std::error::Error>> {
    let fe = Node::new(
        NodeKind::Foreach {
            variable: Box::new(var_node("$", "item")),
            list: Box::new(var_node("@", "items")),
            body: Box::new(block_node(vec![])),
            continue_block: None,
        },
        loc(0, 30),
    );
    assert_eq!(fe.kind.kind_name(), "Foreach");
    Ok(())
}

#[test]
fn subroutine_named_and_anonymous() -> Result<(), Box<dyn std::error::Error>> {
    let named = Node::new(
        NodeKind::Subroutine {
            name: Some("greet".to_string()),
            name_span: Some(loc(4, 9)),
            prototype: None,
            signature: None,
            attributes: vec![],
            body: Box::new(block_node(vec![])),
        },
        loc(0, 20),
    );
    let anon = Node::new(
        NodeKind::Subroutine {
            name: None,
            name_span: None,
            prototype: None,
            signature: None,
            attributes: vec![],
            body: Box::new(block_node(vec![])),
        },
        loc(0, 10),
    );
    assert_eq!(named.kind.kind_name(), "Subroutine");
    assert_eq!(anon.kind.kind_name(), "Subroutine");
    Ok(())
}

#[test]
fn try_catch_finally() -> Result<(), Box<dyn std::error::Error>> {
    let t = Node::new(
        NodeKind::Try {
            body: Box::new(block_node(vec![])),
            catch_blocks: vec![
                (Some("$e".to_string()), Box::new(block_node(vec![]))),
                (None, Box::new(block_node(vec![]))),
            ],
            finally_block: Some(Box::new(block_node(vec![]))),
        },
        loc(0, 40),
    );
    assert_eq!(t.kind.kind_name(), "Try");
    Ok(())
}

#[test]
fn package_with_and_without_block() -> Result<(), Box<dyn std::error::Error>> {
    let with_block = Node::new(
        NodeKind::Package {
            name: "Foo::Bar".to_string(),
            name_span: loc(8, 16),
            block: Some(Box::new(block_node(vec![]))),
        },
        loc(0, 30),
    );
    let without = Node::new(
        NodeKind::Package { name: "Foo".to_string(), name_span: loc(8, 11), block: None },
        loc(0, 12),
    );
    assert_eq!(with_block.kind.kind_name(), "Package");
    assert_eq!(without.kind.kind_name(), "Package");
    Ok(())
}

// ===========================================================================
// 3. to_sexp() formatting
// ===========================================================================

#[test]
fn sexp_number() -> Result<(), Box<dyn std::error::Error>> {
    let n = num_node("42");
    assert_eq!(n.to_sexp(), "(number 42)");
    Ok(())
}

#[test]
fn sexp_variable() -> Result<(), Box<dyn std::error::Error>> {
    let v = var_node("$", "name");
    assert_eq!(v.to_sexp(), "(variable $ name)");
    Ok(())
}

#[test]
fn sexp_string_plain() -> Result<(), Box<dyn std::error::Error>> {
    let s =
        Node::new(NodeKind::String { value: "hello".to_string(), interpolated: false }, loc(0, 7));
    assert_eq!(s.to_sexp(), "(string \"hello\")");
    Ok(())
}

#[test]
fn sexp_string_interpolated() -> Result<(), Box<dyn std::error::Error>> {
    let s =
        Node::new(NodeKind::String { value: "hi $x".to_string(), interpolated: true }, loc(0, 7));
    assert_eq!(s.to_sexp(), "(string_interpolated \"hi $x\")");
    Ok(())
}

#[test]
fn sexp_program_wraps_source_file() -> Result<(), Box<dyn std::error::Error>> {
    let prog = program_node(vec![num_node("1")]);
    let sexp = prog.to_sexp();
    assert!(sexp.starts_with("(source_file "), "got: {sexp}");
    assert!(sexp.contains("(number 1)"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_block() -> Result<(), Box<dyn std::error::Error>> {
    let b = block_node(vec![num_node("99")]);
    assert_eq!(b.to_sexp(), "(block (number 99))");
    Ok(())
}

#[test]
fn sexp_binary_addition() -> Result<(), Box<dyn std::error::Error>> {
    let expr = Node::new(
        NodeKind::Binary {
            op: "+".to_string(),
            left: Box::new(num_node("1")),
            right: Box::new(num_node("2")),
        },
        loc(0, 5),
    );
    assert_eq!(expr.to_sexp(), "(binary_+ (number 1) (number 2))");
    Ok(())
}

#[test]
fn sexp_unary_negation() -> Result<(), Box<dyn std::error::Error>> {
    let expr = Node::new(
        NodeKind::Unary { op: "-".to_string(), operand: Box::new(num_node("5")) },
        loc(0, 2),
    );
    assert_eq!(expr.to_sexp(), "(unary_- (number 5))");
    Ok(())
}

#[test]
fn sexp_unary_not() -> Result<(), Box<dyn std::error::Error>> {
    let expr = Node::new(
        NodeKind::Unary { op: "!".to_string(), operand: Box::new(var_node("$", "x")) },
        loc(0, 3),
    );
    assert_eq!(expr.to_sexp(), "(unary_not (variable $ x))");
    Ok(())
}

#[test]
fn sexp_identifier() -> Result<(), Box<dyn std::error::Error>> {
    let id = ident_node("foo");
    assert_eq!(id.to_sexp(), "(identifier foo)");
    Ok(())
}

#[test]
fn sexp_my_declaration_no_init() -> Result<(), Box<dyn std::error::Error>> {
    let decl = Node::new(
        NodeKind::VariableDeclaration {
            declarator: "my".to_string(),
            variable: Box::new(var_node("$", "x")),
            attributes: vec![],
            initializer: None,
        },
        loc(0, 5),
    );
    assert_eq!(decl.to_sexp(), "(my_declaration (variable $ x))");
    Ok(())
}

#[test]
fn sexp_my_declaration_with_init() -> Result<(), Box<dyn std::error::Error>> {
    let decl = Node::new(
        NodeKind::VariableDeclaration {
            declarator: "my".to_string(),
            variable: Box::new(var_node("$", "x")),
            attributes: vec![],
            initializer: Some(Box::new(num_node("10"))),
        },
        loc(0, 11),
    );
    let sexp = decl.to_sexp();
    assert!(sexp.starts_with("(my_declaration"), "got: {sexp}");
    assert!(sexp.contains("(variable $ x)"), "got: {sexp}");
    assert!(sexp.contains("(number 10)"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_our_declaration_with_attributes() -> Result<(), Box<dyn std::error::Error>> {
    let decl = Node::new(
        NodeKind::VariableDeclaration {
            declarator: "our".to_string(),
            variable: Box::new(var_node("$", "x")),
            attributes: vec!["shared".to_string()],
            initializer: None,
        },
        loc(0, 20),
    );
    let sexp = decl.to_sexp();
    assert!(sexp.contains("(attributes shared)"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_variable_list_declaration() -> Result<(), Box<dyn std::error::Error>> {
    let decl = Node::new(
        NodeKind::VariableListDeclaration {
            declarator: "my".to_string(),
            variables: vec![var_node("$", "a"), var_node("$", "b")],
            attributes: vec![],
            initializer: None,
        },
        loc(0, 15),
    );
    let sexp = decl.to_sexp();
    assert!(sexp.starts_with("(my_declaration"), "got: {sexp}");
    assert!(sexp.contains("(variable $ a)"), "got: {sexp}");
    assert!(sexp.contains("(variable $ b)"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_if_elsif_else() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::If {
            condition: Box::new(var_node("$", "c")),
            then_branch: Box::new(block_node(vec![])),
            elsif_branches: vec![(Box::new(var_node("$", "d")), Box::new(block_node(vec![])))],
            else_branch: Some(Box::new(block_node(vec![]))),
        },
        loc(0, 50),
    );
    let sexp = node.to_sexp();
    assert!(sexp.contains("(if"), "got: {sexp}");
    assert!(sexp.contains("(elsif"), "got: {sexp}");
    assert!(sexp.contains("(else"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_while() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::While {
            condition: Box::new(num_node("1")),
            body: Box::new(block_node(vec![])),
            continue_block: None,
        },
        loc(0, 15),
    );
    let sexp = node.to_sexp();
    assert!(sexp.starts_with("(while"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_while_with_continue() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::While {
            condition: Box::new(num_node("1")),
            body: Box::new(block_node(vec![])),
            continue_block: Some(Box::new(block_node(vec![]))),
        },
        loc(0, 25),
    );
    let sexp = node.to_sexp();
    assert!(sexp.contains("(continue"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_for_loop() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::For {
            init: Some(Box::new(num_node("0"))),
            condition: Some(Box::new(num_node("10"))),
            update: Some(Box::new(num_node("1"))),
            body: Box::new(block_node(vec![])),
            continue_block: None,
        },
        loc(0, 30),
    );
    let sexp = node.to_sexp();
    assert!(sexp.starts_with("(for"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_foreach() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::Foreach {
            variable: Box::new(var_node("$", "i")),
            list: Box::new(var_node("@", "items")),
            body: Box::new(block_node(vec![])),
            continue_block: None,
        },
        loc(0, 30),
    );
    let sexp = node.to_sexp();
    assert!(sexp.starts_with("(foreach"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_error_nodes() -> Result<(), Box<dyn std::error::Error>> {
    let err = Node::new(
        NodeKind::Error {
            message: "unexpected".to_string(),
            expected: vec![],
            found: None,
            partial: None,
        },
        loc(0, 1),
    );
    let sexp = err.to_sexp();
    assert!(sexp.contains("ERROR"), "got: {sexp}");
    assert!(sexp.contains("unexpected"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_missing_variants() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(Node::new(NodeKind::MissingExpression, loc(0, 0)).to_sexp(), "(missing_expression)");
    assert_eq!(Node::new(NodeKind::MissingStatement, loc(0, 0)).to_sexp(), "(missing_statement)");
    assert_eq!(Node::new(NodeKind::MissingIdentifier, loc(0, 0)).to_sexp(), "(missing_identifier)");
    assert_eq!(Node::new(NodeKind::MissingBlock, loc(0, 0)).to_sexp(), "(missing_block)");
    Ok(())
}

#[test]
fn sexp_unknown_rest() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(Node::new(NodeKind::UnknownRest, loc(0, 0)).to_sexp(), "(UNKNOWN_REST)");
    Ok(())
}

#[test]
fn sexp_diamond_ellipsis_undef() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(Node::new(NodeKind::Diamond, loc(0, 2)).to_sexp(), "(diamond)");
    assert_eq!(Node::new(NodeKind::Ellipsis, loc(0, 3)).to_sexp(), "(ellipsis)");
    assert_eq!(Node::new(NodeKind::Undef, loc(0, 5)).to_sexp(), "(undef)");
    Ok(())
}

#[test]
fn sexp_readline_with_and_without_fh() -> Result<(), Box<dyn std::error::Error>> {
    let with_fh =
        Node::new(NodeKind::Readline { filehandle: Some("STDIN".to_string()) }, loc(0, 7));
    let without = Node::new(NodeKind::Readline { filehandle: None }, loc(0, 2));
    assert_eq!(with_fh.to_sexp(), "(readline STDIN)");
    assert_eq!(without.to_sexp(), "(readline)");
    Ok(())
}

#[test]
fn sexp_glob_and_typeglob() -> Result<(), Box<dyn std::error::Error>> {
    let g = Node::new(NodeKind::Glob { pattern: "*.pl".to_string() }, loc(0, 6));
    let tg = Node::new(NodeKind::Typeglob { name: "foo".to_string() }, loc(0, 4));
    assert_eq!(g.to_sexp(), "(glob *.pl)");
    assert_eq!(tg.to_sexp(), "(typeglob foo)");
    Ok(())
}

#[test]
fn sexp_array_literal() -> Result<(), Box<dyn std::error::Error>> {
    let arr = Node::new(
        NodeKind::ArrayLiteral { elements: vec![num_node("1"), num_node("2")] },
        loc(0, 6),
    );
    assert_eq!(arr.to_sexp(), "(array (number 1) (number 2))");
    Ok(())
}

#[test]
fn sexp_hash_literal() -> Result<(), Box<dyn std::error::Error>> {
    let h = Node::new(
        NodeKind::HashLiteral { pairs: vec![(ident_node("key"), num_node("1"))] },
        loc(0, 10),
    );
    let sexp = h.to_sexp();
    assert!(sexp.starts_with("(hash"), "got: {sexp}");
    assert!(sexp.contains("(identifier key)"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_assignment() -> Result<(), Box<dyn std::error::Error>> {
    let a = Node::new(
        NodeKind::Assignment {
            lhs: Box::new(var_node("$", "x")),
            rhs: Box::new(num_node("5")),
            op: "=".to_string(),
        },
        loc(0, 7),
    );
    let sexp = a.to_sexp();
    assert!(sexp.starts_with("(assignment_"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_ternary() -> Result<(), Box<dyn std::error::Error>> {
    let t = Node::new(
        NodeKind::Ternary {
            condition: Box::new(var_node("$", "x")),
            then_expr: Box::new(num_node("1")),
            else_expr: Box::new(num_node("0")),
        },
        loc(0, 10),
    );
    let sexp = t.to_sexp();
    assert!(sexp.starts_with("(ternary"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_return_with_and_without_value() -> Result<(), Box<dyn std::error::Error>> {
    let with_val =
        Node::new(NodeKind::Return { value: Some(Box::new(num_node("42"))) }, loc(0, 10));
    let bare = Node::new(NodeKind::Return { value: None }, loc(0, 6));
    assert_eq!(with_val.to_sexp(), "(return (number 42))");
    assert_eq!(bare.to_sexp(), "(return)");
    Ok(())
}

#[test]
fn sexp_loop_control() -> Result<(), Box<dyn std::error::Error>> {
    let next = Node::new(NodeKind::LoopControl { op: "next".to_string(), label: None }, loc(0, 4));
    let last_labeled = Node::new(
        NodeKind::LoopControl { op: "last".to_string(), label: Some("OUTER".to_string()) },
        loc(0, 10),
    );
    assert_eq!(next.to_sexp(), "(next)");
    assert_eq!(last_labeled.to_sexp(), "(last OUTER)");
    Ok(())
}

#[test]
fn sexp_use_and_no() -> Result<(), Box<dyn std::error::Error>> {
    let use_stmt = Node::new(
        NodeKind::Use {
            module: "strict".to_string(),
            args: vec![],
            has_filter_risk: false,
            has_explicit_import_list: false,
        },
        loc(0, 11),
    );
    let no_stmt = Node::new(
        NodeKind::No {
            module: "warnings".to_string(),
            args: vec!["all".to_string()],
            has_filter_risk: false,
        },
        loc(0, 15),
    );
    assert_eq!(use_stmt.to_sexp(), "(use strict)");
    assert_eq!(no_stmt.to_sexp(), "(no warnings (all))");
    Ok(())
}

#[test]
fn sexp_use_with_filter_risk() -> Result<(), Box<dyn std::error::Error>> {
    let risky = Node::new(
        NodeKind::Use {
            module: "Filter::Simple".to_string(),
            args: vec![],
            has_filter_risk: true,
            has_explicit_import_list: false,
        },
        loc(0, 20),
    );
    let sexp = risky.to_sexp();
    assert!(sexp.contains("(risk:filter)"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_package() -> Result<(), Box<dyn std::error::Error>> {
    let pkg = Node::new(
        NodeKind::Package { name: "Foo".to_string(), name_span: loc(8, 11), block: None },
        loc(0, 12),
    );
    assert_eq!(pkg.to_sexp(), "(package Foo)");
    Ok(())
}

#[test]
fn sexp_package_with_block() -> Result<(), Box<dyn std::error::Error>> {
    let pkg = Node::new(
        NodeKind::Package {
            name: "Bar".to_string(),
            name_span: loc(8, 11),
            block: Some(Box::new(block_node(vec![]))),
        },
        loc(0, 20),
    );
    let sexp = pkg.to_sexp();
    assert!(sexp.contains("(package Bar"), "got: {sexp}");
    assert!(sexp.contains("(block"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_eval_and_do() -> Result<(), Box<dyn std::error::Error>> {
    let eval = Node::new(NodeKind::Eval { block: Box::new(block_node(vec![])) }, loc(0, 10));
    let do_node = Node::new(NodeKind::Do { block: Box::new(block_node(vec![])) }, loc(0, 10));
    assert_eq!(eval.to_sexp(), "(eval (block ))");
    assert_eq!(do_node.to_sexp(), "(do (block ))");
    Ok(())
}

#[test]
fn sexp_try_catch_finally() -> Result<(), Box<dyn std::error::Error>> {
    let t = Node::new(
        NodeKind::Try {
            body: Box::new(block_node(vec![])),
            catch_blocks: vec![(Some("$e".to_string()), Box::new(block_node(vec![])))],
            finally_block: Some(Box::new(block_node(vec![]))),
        },
        loc(0, 40),
    );
    let sexp = t.to_sexp();
    assert!(sexp.contains("(try"), "got: {sexp}");
    assert!(sexp.contains("(catch $e"), "got: {sexp}");
    assert!(sexp.contains("(finally"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_function_call_builtin() -> Result<(), Box<dyn std::error::Error>> {
    let call = Node::new(
        NodeKind::FunctionCall {
            name: "print".to_string(),
            args: vec![Node::new(
                NodeKind::String { value: "hi".to_string(), interpolated: false },
                loc(0, 4),
            )],
        },
        loc(0, 12),
    );
    let sexp = call.to_sexp();
    assert!(sexp.contains("(call print"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_function_call_user() -> Result<(), Box<dyn std::error::Error>> {
    let call = Node::new(
        NodeKind::FunctionCall { name: "my_func".to_string(), args: vec![num_node("1")] },
        loc(0, 12),
    );
    let sexp = call.to_sexp();
    assert!(sexp.contains("function"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_method_call() -> Result<(), Box<dyn std::error::Error>> {
    let mc = Node::new(
        NodeKind::MethodCall {
            object: Box::new(var_node("$", "obj")),
            method: "run".to_string(),
            args: vec![],
        },
        loc(0, 10),
    );
    let sexp = mc.to_sexp();
    assert!(sexp.starts_with("(method_call"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_regex() -> Result<(), Box<dyn std::error::Error>> {
    let r = Node::new(
        NodeKind::Regex {
            pattern: "foo".to_string(),
            replacement: None,
            modifiers: "gi".to_string(),
            has_embedded_code: false,
        },
        loc(0, 8),
    );
    let sexp = r.to_sexp();
    assert!(sexp.contains("regex"), "got: {sexp}");
    assert!(sexp.contains("foo"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_regex_with_embedded_code() -> Result<(), Box<dyn std::error::Error>> {
    let r = Node::new(
        NodeKind::Regex {
            pattern: "(?{1})".to_string(),
            replacement: None,
            modifiers: "".to_string(),
            has_embedded_code: true,
        },
        loc(0, 10),
    );
    let sexp = r.to_sexp();
    assert!(sexp.contains("(risk:code)"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_heredoc_variants() -> Result<(), Box<dyn std::error::Error>> {
    let plain = Node::new(
        NodeKind::Heredoc {
            delimiter: "EOF".to_string(),
            content: "hello".to_string(),
            interpolated: false,
            indented: false,
            command: false,
            body_span: None,
        },
        loc(0, 20),
    );
    let indented_interp = Node::new(
        NodeKind::Heredoc {
            delimiter: "END".to_string(),
            content: "world".to_string(),
            interpolated: true,
            indented: true,
            command: false,
            body_span: None,
        },
        loc(0, 20),
    );
    let cmd = Node::new(
        NodeKind::Heredoc {
            delimiter: "CMD".to_string(),
            content: "ls".to_string(),
            interpolated: false,
            indented: false,
            command: true,
            body_span: None,
        },
        loc(0, 10),
    );
    assert!(plain.to_sexp().starts_with("(heredoc "), "got: {}", plain.to_sexp());
    assert!(
        indented_interp.to_sexp().starts_with("(heredoc_indented_interpolated"),
        "got: {}",
        indented_interp.to_sexp()
    );
    assert!(cmd.to_sexp().starts_with("(heredoc_command"), "got: {}", cmd.to_sexp());
    Ok(())
}

#[test]
fn sexp_data_section() -> Result<(), Box<dyn std::error::Error>> {
    let ds = Node::new(
        NodeKind::DataSection { marker: "__DATA__".to_string(), body: Some("stuff".to_string()) },
        loc(0, 20),
    );
    let sexp = ds.to_sexp();
    assert!(sexp.contains("data_section"), "got: {sexp}");
    assert!(sexp.contains("__DATA__"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_class() -> Result<(), Box<dyn std::error::Error>> {
    let c = Node::new(
        NodeKind::Class {
            name: "MyClass".to_string(),
            parents: vec![],
            body: Box::new(block_node(vec![])),
        },
        loc(0, 20),
    );
    assert_eq!(c.to_sexp(), "(class MyClass (block ))");
    Ok(())
}

#[test]
fn sexp_statement_modifier() -> Result<(), Box<dyn std::error::Error>> {
    let sm = Node::new(
        NodeKind::StatementModifier {
            statement: Box::new(num_node("1")),
            modifier: "if".to_string(),
            condition: Box::new(var_node("$", "x")),
        },
        loc(0, 15),
    );
    let sexp = sm.to_sexp();
    assert!(sexp.starts_with("(statement_modifier_if"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_labeled_statement() -> Result<(), Box<dyn std::error::Error>> {
    let ls = Node::new(
        NodeKind::LabeledStatement {
            label: "LOOP".to_string(),
            statement: Box::new(Node::new(
                NodeKind::While {
                    condition: Box::new(num_node("1")),
                    body: Box::new(block_node(vec![])),
                    continue_block: None,
                },
                loc(0, 15),
            )),
        },
        loc(0, 20),
    );
    let sexp = ls.to_sexp();
    assert!(sexp.contains("labeled_statement"), "got: {sexp}");
    assert!(sexp.contains("LOOP"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_given_when_default() -> Result<(), Box<dyn std::error::Error>> {
    let g = Node::new(
        NodeKind::Given { expr: Box::new(var_node("$", "x")), body: Box::new(block_node(vec![])) },
        loc(0, 20),
    );
    let w = Node::new(
        NodeKind::When { condition: Box::new(num_node("1")), body: Box::new(block_node(vec![])) },
        loc(0, 15),
    );
    let d = Node::new(NodeKind::Default { body: Box::new(block_node(vec![])) }, loc(0, 10));
    assert!(g.to_sexp().starts_with("(given"), "got: {}", g.to_sexp());
    assert!(w.to_sexp().starts_with("(when"), "got: {}", w.to_sexp());
    assert!(d.to_sexp().starts_with("(default"), "got: {}", d.to_sexp());
    Ok(())
}

#[test]
fn sexp_expression_statement() -> Result<(), Box<dyn std::error::Error>> {
    let es = Node::new(
        NodeKind::ExpressionStatement { expression: Box::new(num_node("42")) },
        loc(0, 3),
    );
    assert_eq!(es.to_sexp(), "(expression_statement (number 42))");
    Ok(())
}

#[test]
fn sexp_named_subroutine() -> Result<(), Box<dyn std::error::Error>> {
    let sub = Node::new(
        NodeKind::Subroutine {
            name: Some("hello".to_string()),
            name_span: Some(loc(4, 9)),
            prototype: None,
            signature: None,
            attributes: vec![],
            body: Box::new(block_node(vec![])),
        },
        loc(0, 20),
    );
    let sexp = sub.to_sexp();
    assert!(sexp.contains("(sub hello"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_subroutine_with_attributes() -> Result<(), Box<dyn std::error::Error>> {
    let sub = Node::new(
        NodeKind::Subroutine {
            name: Some("foo".to_string()),
            name_span: Some(loc(4, 7)),
            prototype: None,
            signature: None,
            attributes: vec!["lvalue".to_string()],
            body: Box::new(block_node(vec![])),
        },
        loc(0, 20),
    );
    let sexp = sub.to_sexp();
    assert!(sexp.contains(":lvalue"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_tie_and_untie() -> Result<(), Box<dyn std::error::Error>> {
    let tie = Node::new(
        NodeKind::Tie {
            variable: Box::new(var_node("%", "h")),
            package: Box::new(ident_node("DB_File")),
            args: vec![],
        },
        loc(0, 20),
    );
    let untie = Node::new(NodeKind::Untie { variable: Box::new(var_node("%", "h")) }, loc(0, 10));
    assert!(tie.to_sexp().starts_with("(tie"), "got: {}", tie.to_sexp());
    assert!(untie.to_sexp().starts_with("(untie"), "got: {}", untie.to_sexp());
    Ok(())
}

#[test]
fn sexp_format() -> Result<(), Box<dyn std::error::Error>> {
    let f = Node::new(
        NodeKind::Format { name: "STDOUT".to_string(), body: "@<<<".to_string() },
        loc(0, 20),
    );
    let sexp = f.to_sexp();
    assert!(sexp.contains("format"), "got: {sexp}");
    assert!(sexp.contains("STDOUT"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_phase_block() -> Result<(), Box<dyn std::error::Error>> {
    let pb = Node::new(
        NodeKind::PhaseBlock {
            phase: "BEGIN".to_string(),
            phase_span: None,
            block: Box::new(block_node(vec![])),
        },
        loc(0, 15),
    );
    assert_eq!(pb.to_sexp(), "(BEGIN (block ))");
    Ok(())
}

#[test]
fn sexp_match_and_substitution() -> Result<(), Box<dyn std::error::Error>> {
    let m = Node::new(
        NodeKind::Match {
            expr: Box::new(var_node("$", "str")),
            pattern: "foo".to_string(),
            modifiers: "i".to_string(),
            has_embedded_code: false,
            negated: false,
        },
        loc(0, 15),
    );
    let s = Node::new(
        NodeKind::Substitution {
            expr: Box::new(var_node("$", "str")),
            pattern: "old".to_string(),
            replacement: "new".to_string(),
            modifiers: "g".to_string(),
            has_embedded_code: false,
            negated: false,
        },
        loc(0, 20),
    );
    assert!(m.to_sexp().contains("match"), "got: {}", m.to_sexp());
    assert!(s.to_sexp().contains("substitution"), "got: {}", s.to_sexp());
    Ok(())
}

#[test]
fn sexp_transliteration() -> Result<(), Box<dyn std::error::Error>> {
    let tr = Node::new(
        NodeKind::Transliteration {
            expr: Box::new(var_node("$", "s")),
            search: "a-z".to_string(),
            replace: "A-Z".to_string(),
            modifiers: "".to_string(),
            negated: false,
        },
        loc(0, 15),
    );
    let sexp = tr.to_sexp();
    assert!(sexp.contains("transliteration"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_indirect_call() -> Result<(), Box<dyn std::error::Error>> {
    let ic = Node::new(
        NodeKind::IndirectCall {
            method: "new".to_string(),
            object: Box::new(ident_node("Foo")),
            args: vec![],
        },
        loc(0, 10),
    );
    let sexp = ic.to_sexp();
    assert!(sexp.contains("indirect_call"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_signature_parameters() -> Result<(), Box<dyn std::error::Error>> {
    let sig = Node::new(
        NodeKind::Signature {
            parameters: vec![
                Node::new(
                    NodeKind::MandatoryParameter { variable: Box::new(var_node("$", "x")) },
                    loc(0, 2),
                ),
                Node::new(
                    NodeKind::OptionalParameter {
                        variable: Box::new(var_node("$", "y")),
                        default_value: Box::new(num_node("0")),
                    },
                    loc(0, 6),
                ),
                Node::new(
                    NodeKind::SlurpyParameter { variable: Box::new(var_node("@", "rest")) },
                    loc(0, 5),
                ),
            ],
        },
        loc(0, 20),
    );
    let sexp = sig.to_sexp();
    assert!(sexp.contains("(signature"), "got: {sexp}");
    assert!(sexp.contains("(mandatory_parameter"), "got: {sexp}");
    assert!(sexp.contains("(optional_parameter"), "got: {sexp}");
    assert!(sexp.contains("(slurpy_parameter"), "got: {sexp}");
    Ok(())
}

// ===========================================================================
// 4. to_sexp_inner() behavior
// ===========================================================================

#[test]
fn sexp_inner_unwraps_expression_statement() -> Result<(), Box<dyn std::error::Error>> {
    let es =
        Node::new(NodeKind::ExpressionStatement { expression: Box::new(num_node("7")) }, loc(0, 2));
    // to_sexp_inner should unwrap non-anon-sub expression statements
    let inner = es.to_sexp_inner();
    assert_eq!(inner, "(number 7)");
    Ok(())
}

#[test]
fn sexp_inner_keeps_anon_sub_wrapped() -> Result<(), Box<dyn std::error::Error>> {
    let anon_sub = Node::new(
        NodeKind::Subroutine {
            name: None,
            name_span: None,
            prototype: None,
            signature: None,
            attributes: vec![],
            body: Box::new(block_node(vec![])),
        },
        loc(0, 10),
    );
    let es =
        Node::new(NodeKind::ExpressionStatement { expression: Box::new(anon_sub) }, loc(0, 10));
    let inner = es.to_sexp_inner();
    assert!(inner.contains("expression_statement"), "got: {inner}");
    Ok(())
}

#[test]
fn sexp_inner_non_expression_statement_falls_through() -> Result<(), Box<dyn std::error::Error>> {
    let n = num_node("42");
    assert_eq!(n.to_sexp_inner(), n.to_sexp());
    Ok(())
}

// ===========================================================================
// 5. Node traversal (children, for_each_child, for_each_child_mut)
// ===========================================================================

#[test]
fn children_of_leaf_node_is_empty() -> Result<(), Box<dyn std::error::Error>> {
    let leaf = num_node("1");
    assert!(leaf.children().is_empty());
    Ok(())
}

#[test]
fn children_of_program() -> Result<(), Box<dyn std::error::Error>> {
    let prog = program_node(vec![num_node("1"), num_node("2")]);
    assert_eq!(prog.children().len(), 2);
    Ok(())
}

#[test]
fn children_of_block() -> Result<(), Box<dyn std::error::Error>> {
    let b = block_node(vec![num_node("1"), num_node("2"), num_node("3")]);
    assert_eq!(b.children().len(), 3);
    Ok(())
}

#[test]
fn children_of_binary() -> Result<(), Box<dyn std::error::Error>> {
    let bin = Node::new(
        NodeKind::Binary {
            op: "+".to_string(),
            left: Box::new(num_node("1")),
            right: Box::new(num_node("2")),
        },
        loc(0, 5),
    );
    assert_eq!(bin.children().len(), 2);
    Ok(())
}

#[test]
fn children_of_unary() -> Result<(), Box<dyn std::error::Error>> {
    let u = Node::new(
        NodeKind::Unary { op: "-".to_string(), operand: Box::new(num_node("3")) },
        loc(0, 2),
    );
    assert_eq!(u.children().len(), 1);
    Ok(())
}

#[test]
fn children_of_if_with_branches() -> Result<(), Box<dyn std::error::Error>> {
    let if_node = Node::new(
        NodeKind::If {
            condition: Box::new(num_node("1")),
            then_branch: Box::new(block_node(vec![])),
            elsif_branches: vec![(Box::new(num_node("2")), Box::new(block_node(vec![])))],
            else_branch: Some(Box::new(block_node(vec![]))),
        },
        loc(0, 30),
    );
    // condition + then + elsif_cond + elsif_body + else = 5
    assert_eq!(if_node.children().len(), 5);
    Ok(())
}

#[test]
fn children_of_variable_declaration() -> Result<(), Box<dyn std::error::Error>> {
    let decl = Node::new(
        NodeKind::VariableDeclaration {
            declarator: "my".to_string(),
            variable: Box::new(var_node("$", "x")),
            attributes: vec![],
            initializer: Some(Box::new(num_node("10"))),
        },
        loc(0, 11),
    );
    // variable + initializer = 2
    assert_eq!(decl.children().len(), 2);
    Ok(())
}

#[test]
fn children_of_variable_declaration_no_init() -> Result<(), Box<dyn std::error::Error>> {
    let decl = Node::new(
        NodeKind::VariableDeclaration {
            declarator: "my".to_string(),
            variable: Box::new(var_node("$", "x")),
            attributes: vec![],
            initializer: None,
        },
        loc(0, 5),
    );
    // variable only = 1
    assert_eq!(decl.children().len(), 1);
    Ok(())
}

#[test]
fn first_child_returns_first_or_none() -> Result<(), Box<dyn std::error::Error>> {
    let prog = program_node(vec![num_node("10"), num_node("20")]);
    let first = prog.first_child();
    assert!(first.is_some());
    assert_eq!(first.map(|n| n.kind.kind_name()), Some("Number"));

    let leaf = num_node("1");
    assert!(leaf.first_child().is_none());
    Ok(())
}

#[test]
fn for_each_child_mut_can_modify() -> Result<(), Box<dyn std::error::Error>> {
    let mut prog = program_node(vec![num_node("1")]);
    prog.for_each_child_mut(|child| {
        child.location = loc(99, 100);
    });
    match &prog.kind {
        NodeKind::Program { statements } => {
            assert_eq!(statements.first().map(|s| s.location.start), Some(99));
        }
        other => return Err(format!("expected Program, got {}", other.kind_name()).into()),
    }
    Ok(())
}

#[test]
fn for_each_child_visits_try_catch() -> Result<(), Box<dyn std::error::Error>> {
    let t = Node::new(
        NodeKind::Try {
            body: Box::new(block_node(vec![])),
            catch_blocks: vec![(None, Box::new(block_node(vec![])))],
            finally_block: Some(Box::new(block_node(vec![]))),
        },
        loc(0, 30),
    );
    // body + 1 catch body + finally = 3
    assert_eq!(t.children().len(), 3);
    Ok(())
}

#[test]
fn for_each_child_visits_foreach() -> Result<(), Box<dyn std::error::Error>> {
    let fe = Node::new(
        NodeKind::Foreach {
            variable: Box::new(var_node("$", "i")),
            list: Box::new(var_node("@", "arr")),
            body: Box::new(block_node(vec![])),
            continue_block: Some(Box::new(block_node(vec![]))),
        },
        loc(0, 30),
    );
    // variable + list + body + continue = 4
    assert_eq!(fe.children().len(), 4);
    Ok(())
}

#[test]
fn for_each_child_visits_for_loop() -> Result<(), Box<dyn std::error::Error>> {
    let f = Node::new(
        NodeKind::For {
            init: Some(Box::new(num_node("0"))),
            condition: Some(Box::new(num_node("10"))),
            update: Some(Box::new(num_node("1"))),
            body: Box::new(block_node(vec![])),
            continue_block: None,
        },
        loc(0, 30),
    );
    // init + condition + update + body = 4
    assert_eq!(f.children().len(), 4);
    Ok(())
}

#[test]
fn for_each_child_visits_error_partial() -> Result<(), Box<dyn std::error::Error>> {
    let err = Node::new(
        NodeKind::Error {
            message: "bad".to_string(),
            expected: vec![],
            found: None,
            partial: Some(Box::new(num_node("1"))),
        },
        loc(0, 5),
    );
    assert_eq!(err.children().len(), 1);

    let err_no_partial = Node::new(
        NodeKind::Error {
            message: "bad".to_string(),
            expected: vec![],
            found: None,
            partial: None,
        },
        loc(0, 5),
    );
    assert_eq!(err_no_partial.children().len(), 0);
    Ok(())
}

#[test]
fn for_each_child_visits_subroutine_body() -> Result<(), Box<dyn std::error::Error>> {
    let sub = Node::new(
        NodeKind::Subroutine {
            name: Some("foo".to_string()),
            name_span: None,
            prototype: None,
            signature: Some(Box::new(Node::new(
                NodeKind::Signature { parameters: vec![] },
                loc(0, 2),
            ))),
            attributes: vec![],
            body: Box::new(block_node(vec![])),
        },
        loc(0, 20),
    );
    // signature + body = 2
    assert_eq!(sub.children().len(), 2);
    Ok(())
}

#[test]
fn for_each_child_visits_method_call() -> Result<(), Box<dyn std::error::Error>> {
    let mc = Node::new(
        NodeKind::MethodCall {
            object: Box::new(var_node("$", "obj")),
            method: "do_thing".to_string(),
            args: vec![num_node("1"), num_node("2")],
        },
        loc(0, 20),
    );
    // object + 2 args = 3
    assert_eq!(mc.children().len(), 3);
    Ok(())
}

#[test]
fn for_each_child_visits_hash_literal() -> Result<(), Box<dyn std::error::Error>> {
    let h = Node::new(
        NodeKind::HashLiteral {
            pairs: vec![(ident_node("a"), num_node("1")), (ident_node("b"), num_node("2"))],
        },
        loc(0, 20),
    );
    // 2 keys + 2 values = 4
    assert_eq!(h.children().len(), 4);
    Ok(())
}

#[test]
fn for_each_child_visits_signature_params() -> Result<(), Box<dyn std::error::Error>> {
    let sig = Node::new(
        NodeKind::Signature {
            parameters: vec![
                Node::new(
                    NodeKind::MandatoryParameter { variable: Box::new(var_node("$", "a")) },
                    loc(0, 2),
                ),
                Node::new(
                    NodeKind::SlurpyParameter { variable: Box::new(var_node("@", "rest")) },
                    loc(0, 5),
                ),
            ],
        },
        loc(0, 10),
    );
    assert_eq!(sig.children().len(), 2);
    Ok(())
}

#[test]
fn for_each_child_visits_tie() -> Result<(), Box<dyn std::error::Error>> {
    let tie = Node::new(
        NodeKind::Tie {
            variable: Box::new(var_node("%", "h")),
            package: Box::new(ident_node("DB_File")),
            args: vec![num_node("1")],
        },
        loc(0, 25),
    );
    // variable + package + 1 arg = 3
    assert_eq!(tie.children().len(), 3);
    Ok(())
}

// ===========================================================================
// 6. count_nodes()
// ===========================================================================

#[test]
fn count_nodes_leaf() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(num_node("1").count_nodes(), 1);
    Ok(())
}

#[test]
fn count_nodes_program_with_statements() -> Result<(), Box<dyn std::error::Error>> {
    let prog = program_node(vec![num_node("1"), num_node("2")]);
    // program(1) + 2 leaves = 3
    assert_eq!(prog.count_nodes(), 3);
    Ok(())
}

#[test]
fn count_nodes_nested_tree() -> Result<(), Box<dyn std::error::Error>> {
    let inner = Node::new(
        NodeKind::Binary {
            op: "+".to_string(),
            left: Box::new(num_node("1")),
            right: Box::new(num_node("2")),
        },
        loc(0, 5),
    );
    let outer = program_node(vec![inner]);
    // program(1) + binary(1) + 2 numbers = 4
    assert_eq!(outer.count_nodes(), 4);
    Ok(())
}

// ===========================================================================
// 7. kind_name() coverage
// ===========================================================================

#[test]
fn kind_name_covers_all_variants() -> Result<(), Box<dyn std::error::Error>> {
    // Verify ALL_KIND_NAMES is populated and sorted
    assert!(!NodeKind::ALL_KIND_NAMES.is_empty());
    let sorted: Vec<&str> = {
        let mut v: Vec<&str> = NodeKind::ALL_KIND_NAMES.to_vec();
        v.sort();
        v
    };
    assert_eq!(NodeKind::ALL_KIND_NAMES, sorted.as_slice(), "ALL_KIND_NAMES should be sorted");
    Ok(())
}

#[test]
fn kind_name_specific_variants() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(NodeKind::Diamond.kind_name(), "Diamond");
    assert_eq!(NodeKind::Ellipsis.kind_name(), "Ellipsis");
    assert_eq!(NodeKind::Undef.kind_name(), "Undef");
    assert_eq!(NodeKind::MissingExpression.kind_name(), "MissingExpression");
    assert_eq!(NodeKind::MissingStatement.kind_name(), "MissingStatement");
    assert_eq!(NodeKind::MissingIdentifier.kind_name(), "MissingIdentifier");
    assert_eq!(NodeKind::MissingBlock.kind_name(), "MissingBlock");
    assert_eq!(NodeKind::UnknownRest.kind_name(), "UnknownRest");
    assert_eq!(NodeKind::Number { value: "1".to_string() }.kind_name(), "Number");
    assert_eq!(
        NodeKind::String { value: "".to_string(), interpolated: false }.kind_name(),
        "String"
    );
    assert_eq!(NodeKind::Readline { filehandle: None }.kind_name(), "Readline");
    assert_eq!(NodeKind::Glob { pattern: "".to_string() }.kind_name(), "Glob");
    assert_eq!(NodeKind::Typeglob { name: "".to_string() }.kind_name(), "Typeglob");
    Ok(())
}

#[test]
fn recovery_kind_names_is_subset() -> Result<(), Box<dyn std::error::Error>> {
    for name in NodeKind::RECOVERY_KIND_NAMES {
        assert!(
            NodeKind::ALL_KIND_NAMES.contains(name),
            "RECOVERY_KIND_NAMES entry '{name}' not in ALL_KIND_NAMES"
        );
    }
    Ok(())
}

// ===========================================================================
// 8. Edge cases
// ===========================================================================

#[test]
fn empty_program() -> Result<(), Box<dyn std::error::Error>> {
    let prog = program_node(vec![]);
    assert_eq!(prog.to_sexp(), "(source_file )");
    assert_eq!(prog.count_nodes(), 1);
    assert!(prog.children().is_empty());
    Ok(())
}

#[test]
fn empty_block() -> Result<(), Box<dyn std::error::Error>> {
    let b = block_node(vec![]);
    assert_eq!(b.to_sexp(), "(block )");
    assert_eq!(b.count_nodes(), 1);
    Ok(())
}

#[test]
fn empty_array_literal() -> Result<(), Box<dyn std::error::Error>> {
    let arr = Node::new(NodeKind::ArrayLiteral { elements: vec![] }, loc(0, 2));
    assert_eq!(arr.to_sexp(), "(array )");
    assert_eq!(arr.count_nodes(), 1);
    Ok(())
}

#[test]
fn empty_hash_literal() -> Result<(), Box<dyn std::error::Error>> {
    let h = Node::new(NodeKind::HashLiteral { pairs: vec![] }, loc(0, 2));
    assert_eq!(h.to_sexp(), "(hash )");
    assert_eq!(h.count_nodes(), 1);
    Ok(())
}

#[test]
fn deep_nesting() -> Result<(), Box<dyn std::error::Error>> {
    // Build a deeply nested tree: program > block > block > ... > number
    let depth = 50;
    let mut current = num_node("leaf");
    for _ in 0..depth {
        current = block_node(vec![current]);
    }
    let prog = program_node(vec![current]);
    // program(1) + 50 blocks + 1 leaf = 52
    assert_eq!(prog.count_nodes(), depth + 2);
    Ok(())
}

#[test]
fn node_clone_preserves_equality() -> Result<(), Box<dyn std::error::Error>> {
    let original = Node::new(
        NodeKind::Binary {
            op: "+".to_string(),
            left: Box::new(num_node("1")),
            right: Box::new(num_node("2")),
        },
        loc(0, 5),
    );
    let cloned = original.clone();
    assert_eq!(original, cloned);
    Ok(())
}

#[test]
fn different_nodes_are_not_equal() -> Result<(), Box<dyn std::error::Error>> {
    let a = num_node("1");
    let b = num_node("2");
    assert_ne!(a, b);
    Ok(())
}

#[test]
fn zero_length_location() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(NodeKind::MissingExpression, loc(5, 5));
    assert_eq!(node.location.start, 5);
    assert_eq!(node.location.end, 5);
    Ok(())
}

#[test]
fn string_with_special_chars_in_sexp() -> Result<(), Box<dyn std::error::Error>> {
    let s = Node::new(
        NodeKind::String { value: "he said \"hi\"".to_string(), interpolated: false },
        loc(0, 14),
    );
    let sexp = s.to_sexp();
    // Escaping should handle quotes
    assert!(sexp.contains("string"), "got: {sexp}");
    assert!(sexp.contains("\\\""), "quotes should be escaped, got: {sexp}");
    Ok(())
}

#[test]
fn error_with_partial_node() -> Result<(), Box<dyn std::error::Error>> {
    let err = Node::new(
        NodeKind::Error {
            message: "parse error".to_string(),
            expected: vec![],
            found: None,
            partial: Some(Box::new(num_node("42"))),
        },
        loc(0, 10),
    );
    let sexp = err.to_sexp();
    assert!(sexp.contains("ERROR"), "got: {sexp}");
    assert!(sexp.contains("(number 42)"), "got: {sexp}");
    Ok(())
}

#[test]
fn leaf_nodes_have_no_children() -> Result<(), Box<dyn std::error::Error>> {
    let leaves: Vec<Node> = vec![
        num_node("1"),
        var_node("$", "x"),
        ident_node("foo"),
        Node::new(NodeKind::Diamond, loc(0, 2)),
        Node::new(NodeKind::Ellipsis, loc(0, 3)),
        Node::new(NodeKind::Undef, loc(0, 5)),
        Node::new(NodeKind::MissingExpression, loc(0, 0)),
        Node::new(NodeKind::MissingStatement, loc(0, 0)),
        Node::new(NodeKind::MissingIdentifier, loc(0, 0)),
        Node::new(NodeKind::MissingBlock, loc(0, 0)),
        Node::new(NodeKind::UnknownRest, loc(0, 0)),
        Node::new(NodeKind::Readline { filehandle: None }, loc(0, 2)),
        Node::new(NodeKind::Glob { pattern: "*.pl".to_string() }, loc(0, 6)),
        Node::new(NodeKind::Typeglob { name: "foo".to_string() }, loc(0, 4)),
        Node::new(NodeKind::String { value: "hi".to_string(), interpolated: false }, loc(0, 4)),
        Node::new(
            NodeKind::Heredoc {
                delimiter: "EOF".to_string(),
                content: "".to_string(),
                interpolated: false,
                indented: false,
                command: false,
                body_span: None,
            },
            loc(0, 10),
        ),
        Node::new(
            NodeKind::Regex {
                pattern: "a".to_string(),
                replacement: None,
                modifiers: "".to_string(),
                has_embedded_code: false,
            },
            loc(0, 3),
        ),
        Node::new(
            NodeKind::Use {
                module: "strict".to_string(),
                args: vec![],
                has_filter_risk: false,
                has_explicit_import_list: false,
            },
            loc(0, 11),
        ),
        Node::new(
            NodeKind::No { module: "warnings".to_string(), args: vec![], has_filter_risk: false },
            loc(0, 13),
        ),
        Node::new(NodeKind::Prototype { content: "$".to_string() }, loc(0, 3)),
        Node::new(NodeKind::DataSection { marker: "__END__".to_string(), body: None }, loc(0, 7)),
        Node::new(
            NodeKind::Format { name: "STDOUT".to_string(), body: "@<<<".to_string() },
            loc(0, 15),
        ),
        Node::new(NodeKind::LoopControl { op: "next".to_string(), label: None }, loc(0, 4)),
    ];

    for leaf in &leaves {
        assert!(leaf.children().is_empty(), "{} should have no children", leaf.kind.kind_name());
    }
    Ok(())
}

// ===========================================================================
// 9. v2 module coverage
// ===========================================================================

#[test]
fn v2_node_creation() -> Result<(), Box<dyn std::error::Error>> {
    use perl_ast::v2::{Node as V2Node, NodeIdGenerator, NodeKind as V2Kind};
    use perl_position_tracking::{Position, Range};

    let mut id_gen = NodeIdGenerator::new();
    let range = Range::new(Position::new(0, 1, 1), Position::new(5, 1, 6));
    let node = V2Node::new(id_gen.next_id(), V2Kind::Number { value: "42".to_string() }, range);
    assert_eq!(node.id, 0);
    assert_eq!(node.to_sexp(), "(number 42)");
    Ok(())
}

#[test]
fn v2_node_id_generator_increments() -> Result<(), Box<dyn std::error::Error>> {
    use perl_ast::v2::NodeIdGenerator;

    let mut id_gen = NodeIdGenerator::new();
    assert_eq!(id_gen.next_id(), 0);
    assert_eq!(id_gen.next_id(), 1);
    assert_eq!(id_gen.next_id(), 2);
    Ok(())
}

#[test]
fn v2_node_id_generator_default() -> Result<(), Box<dyn std::error::Error>> {
    use perl_ast::v2::NodeIdGenerator;

    let mut id_gen = NodeIdGenerator::default();
    assert_eq!(id_gen.next_id(), 0);
    Ok(())
}

#[test]
fn v2_missing_kind_sexp() -> Result<(), Box<dyn std::error::Error>> {
    use perl_ast::v2::{MissingKind, NodeKind as V2Kind};

    assert_eq!(V2Kind::MissingExpression.to_sexp(), "(MISSING_EXPRESSION)");
    assert_eq!(V2Kind::MissingStatement.to_sexp(), "(MISSING_STATEMENT)");
    assert_eq!(V2Kind::MissingIdentifier.to_sexp(), "(MISSING_IDENTIFIER)");
    assert_eq!(V2Kind::MissingBlock.to_sexp(), "(MISSING_BLOCK)");
    assert_eq!(V2Kind::Missing(MissingKind::Semicolon).to_sexp(), "(MISSING Semicolon)");
    assert_eq!(
        V2Kind::Missing(MissingKind::ClosingDelimiter(')')).to_sexp(),
        "(MISSING ClosingDelimiter(')'))"
    );
    Ok(())
}

#[test]
fn v2_error_ref_sexp() -> Result<(), Box<dyn std::error::Error>> {
    use perl_ast::v2::NodeKind as V2Kind;

    assert_eq!(V2Kind::ErrorRef { diag_id: 7 }.to_sexp(), "(ERROR_REF #7)");
    Ok(())
}

#[test]
fn v2_program_sexp() -> Result<(), Box<dyn std::error::Error>> {
    use perl_ast::v2::{Node as V2Node, NodeIdGenerator, NodeKind as V2Kind};
    use perl_position_tracking::{Position, Range};

    let mut id_gen = NodeIdGenerator::new();
    let range = Range::new(Position::new(0, 1, 1), Position::new(10, 1, 11));
    let child = V2Node::new(id_gen.next_id(), V2Kind::Number { value: "1".to_string() }, range);
    let prog = V2Node::new(id_gen.next_id(), V2Kind::Program { statements: vec![child] }, range);
    let sexp = prog.to_sexp();
    assert!(sexp.starts_with("(source_file"), "got: {sexp}");
    assert!(sexp.contains("(number 1)"), "got: {sexp}");
    Ok(())
}

#[test]
fn v2_variable_sexp() -> Result<(), Box<dyn std::error::Error>> {
    use perl_ast::v2::NodeKind as V2Kind;

    let sexp = V2Kind::Variable { sigil: "$".to_string(), name: "foo".to_string() }.to_sexp();
    assert_eq!(sexp, "(variable $ foo)");
    Ok(())
}

#[test]
fn v2_string_sexp() -> Result<(), Box<dyn std::error::Error>> {
    use perl_ast::v2::NodeKind as V2Kind;

    assert_eq!(
        V2Kind::String { value: "hi".to_string(), interpolated: false }.to_sexp(),
        "(string \"hi\")"
    );
    assert_eq!(
        V2Kind::String { value: "hi".to_string(), interpolated: true }.to_sexp(),
        "(string_interpolated \"hi\")"
    );
    Ok(())
}

#[test]
fn v2_binary_sexp() -> Result<(), Box<dyn std::error::Error>> {
    use perl_ast::v2::{Node as V2Node, NodeIdGenerator, NodeKind as V2Kind};
    use perl_position_tracking::{Position, Range};

    let mut id_gen = NodeIdGenerator::new();
    let range = Range::new(Position::new(0, 1, 1), Position::new(5, 1, 6));
    let left = V2Node::new(id_gen.next_id(), V2Kind::Number { value: "1".to_string() }, range);
    let right = V2Node::new(id_gen.next_id(), V2Kind::Number { value: "2".to_string() }, range);
    let bin = V2Kind::Binary { op: "+".to_string(), left: Box::new(left), right: Box::new(right) };
    assert_eq!(bin.to_sexp(), "(binary_+ (number 1) (number 2))");
    Ok(())
}

#[test]
fn v2_error_sexp() -> Result<(), Box<dyn std::error::Error>> {
    use perl_ast::v2::NodeKind as V2Kind;

    let sexp = V2Kind::Error {
        message: "bad token".to_string(),
        expected: vec!["ident".to_string()],
        partial: None,
    }
    .to_sexp();
    assert!(sexp.contains("ERROR"), "got: {sexp}");
    assert!(sexp.contains("bad token"), "got: {sexp}");
    Ok(())
}

#[test]
fn v2_missing_kind_enum_coverage() -> Result<(), Box<dyn std::error::Error>> {
    use perl_ast::v2::MissingKind;

    // Ensure all MissingKind variants can be constructed and compared
    let variants = [
        MissingKind::Expression,
        MissingKind::Statement,
        MissingKind::Identifier,
        MissingKind::Block,
        MissingKind::ClosingDelimiter(')'),
        MissingKind::Semicolon,
        MissingKind::Condition,
        MissingKind::Argument,
        MissingKind::Operator,
    ];
    for (i, v) in variants.iter().enumerate() {
        // Each variant should equal itself (Clone + PartialEq)
        let cloned = *v;
        assert_eq!(*v, cloned, "MissingKind variant {i} should equal its clone");
    }
    Ok(())
}
