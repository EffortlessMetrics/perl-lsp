//! Extended test coverage for `perl-ast` crate.
//!
//! Fills gaps not covered by existing test files:
//! - Immutable `for_each_child` traversal on all branching node kinds
//! - Mutable traversal that actually modifies children
//! - `children()` and `first_child()` on additional node kinds
//! - Deep/complex `count_nodes` scenarios
//! - PartialEq edge cases (nested tree equality)
//! - SourceLocation boundary conditions
//! - to_sexp for under-tested node kinds (Goto, Tie, Untie, Regex flags,
//!   negated Match/Substitution/Transliteration, Use/No with filter risk,
//!   Readline, Glob, Typeglob, LoopControl, Prototype, Signature params,
//!   Given/When/Default, While+continue, LabeledStatement, Format, Class,
//!   DataSection, Identifier, missing-node variants)
//! - Clone independence for nested Box nodes

use perl_ast::ast::{Node, NodeKind, SourceLocation};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn loc(start: usize, end: usize) -> SourceLocation {
    SourceLocation { start, end }
}

fn num(value: &str) -> Node {
    Node::new(NodeKind::Number { value: value.to_string() }, loc(0, value.len()))
}

fn var(sigil: &str, name: &str) -> Node {
    Node::new(
        NodeKind::Variable { sigil: sigil.to_string(), name: name.to_string() },
        loc(0, sigil.len() + name.len()),
    )
}

fn block(stmts: Vec<Node>) -> Node {
    Node::new(NodeKind::Block { statements: stmts }, loc(0, 1))
}

fn ident(name: &str) -> Node {
    Node::new(NodeKind::Identifier { name: name.to_string() }, loc(0, name.len()))
}

// ===========================================================================
// 1. Immutable for_each_child — verify it visits the same set as mutable
// ===========================================================================

#[test]
fn for_each_child_program() -> Result<(), Box<dyn std::error::Error>> {
    let prog =
        Node::new(NodeKind::Program { statements: vec![num("1"), num("2"), num("3")] }, loc(0, 10));
    let mut visited = Vec::new();
    prog.for_each_child(|c| visited.push(c.kind.kind_name()));
    assert_eq!(visited.len(), 3);
    Ok(())
}

#[test]
fn for_each_child_if_with_all_branches() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::If {
            condition: Box::new(num("1")),
            then_branch: Box::new(block(vec![])),
            elsif_branches: vec![
                (Box::new(num("2")), Box::new(block(vec![]))),
                (Box::new(num("3")), Box::new(block(vec![]))),
            ],
            else_branch: Some(Box::new(block(vec![]))),
        },
        loc(0, 50),
    );
    let mut count = 0usize;
    node.for_each_child(|_| count += 1);
    // condition + then + 2*(elsif_cond + elsif_body) + else = 1+1+4+1 = 7
    assert_eq!(count, 7);
    Ok(())
}

#[test]
fn for_each_child_for_all_optional_fields() -> Result<(), Box<dyn std::error::Error>> {
    // All options present
    let full = Node::new(
        NodeKind::For {
            init: Some(Box::new(num("0"))),
            condition: Some(Box::new(num("10"))),
            update: Some(Box::new(num("1"))),
            body: Box::new(block(vec![])),
            continue_block: Some(Box::new(block(vec![]))),
        },
        loc(0, 30),
    );
    let mut full_count = 0usize;
    full.for_each_child(|_| full_count += 1);
    assert_eq!(full_count, 5);

    // All options absent
    let minimal = Node::new(
        NodeKind::For {
            init: None,
            condition: None,
            update: None,
            body: Box::new(block(vec![])),
            continue_block: None,
        },
        loc(0, 10),
    );
    let mut min_count = 0usize;
    minimal.for_each_child(|_| min_count += 1);
    assert_eq!(min_count, 1); // only body
    Ok(())
}

#[test]
fn for_each_child_try_with_multiple_catches_and_finally() -> Result<(), Box<dyn std::error::Error>>
{
    let node = Node::new(
        NodeKind::Try {
            body: Box::new(block(vec![])),
            catch_blocks: vec![
                (Some("$e1".to_string()), Box::new(block(vec![]))),
                (None, Box::new(block(vec![]))),
            ],
            finally_block: Some(Box::new(block(vec![]))),
        },
        loc(0, 50),
    );
    let mut count = 0usize;
    node.for_each_child(|_| count += 1);
    // body + 2 catch bodies + finally = 4
    assert_eq!(count, 4);
    Ok(())
}

#[test]
fn for_each_child_foreach_without_continue() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::Foreach {
            variable: Box::new(var("$", "i")),
            list: Box::new(Node::new(NodeKind::ArrayLiteral { elements: vec![] }, loc(0, 2))),
            body: Box::new(block(vec![])),
            continue_block: None,
        },
        loc(0, 20),
    );
    let mut count = 0usize;
    node.for_each_child(|_| count += 1);
    assert_eq!(count, 3); // variable + list + body
    Ok(())
}

#[test]
fn for_each_child_subroutine_with_all_parts() -> Result<(), Box<dyn std::error::Error>> {
    let proto = Node::new(NodeKind::Prototype { content: "$$".to_string() }, loc(0, 4));
    let sig = Node::new(NodeKind::Signature { parameters: vec![] }, loc(0, 2));
    let node = Node::new(
        NodeKind::Subroutine {
            name: Some("test_fn".to_string()),
            name_span: Some(loc(4, 11)),
            prototype: Some(Box::new(proto)),
            signature: Some(Box::new(sig)),
            attributes: vec!["lvalue".to_string()],
            body: Box::new(block(vec![])),
        },
        loc(0, 30),
    );
    let mut count = 0usize;
    node.for_each_child(|_| count += 1);
    // prototype + signature + body = 3
    assert_eq!(count, 3);
    Ok(())
}

#[test]
fn for_each_child_subroutine_body_only() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::Subroutine {
            name: None,
            name_span: None,
            prototype: None,
            signature: None,
            attributes: vec![],
            body: Box::new(block(vec![])),
        },
        loc(0, 10),
    );
    let mut count = 0usize;
    node.for_each_child(|_| count += 1);
    assert_eq!(count, 1); // only body
    Ok(())
}

#[test]
fn for_each_child_hash_literal_pairs() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::HashLiteral { pairs: vec![(ident("a"), num("1")), (ident("b"), num("2"))] },
        loc(0, 20),
    );
    let mut count = 0usize;
    node.for_each_child(|_| count += 1);
    // 2 keys + 2 values = 4
    assert_eq!(count, 4);
    Ok(())
}

#[test]
fn for_each_child_method_call() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::MethodCall {
            object: Box::new(var("$", "self")),
            method: "run".to_string(),
            args: vec![num("1"), num("2")],
        },
        loc(0, 15),
    );
    let mut count = 0usize;
    node.for_each_child(|_| count += 1);
    // object + 2 args = 3
    assert_eq!(count, 3);
    Ok(())
}

#[test]
fn for_each_child_tie_with_args() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::Tie {
            variable: Box::new(var("%", "h")),
            package: Box::new(ident("DB_File")),
            args: vec![num("1"), num("2")],
        },
        loc(0, 20),
    );
    let mut count = 0usize;
    node.for_each_child(|_| count += 1);
    // variable + package + 2 args = 4
    assert_eq!(count, 4);
    Ok(())
}

#[test]
fn for_each_child_goto() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(NodeKind::Goto { target: Box::new(ident("DONE")) }, loc(0, 10));
    let mut count = 0usize;
    node.for_each_child(|_| count += 1);
    assert_eq!(count, 1);
    Ok(())
}

#[test]
fn for_each_child_signature_with_params() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::Signature {
            parameters: vec![
                Node::new(
                    NodeKind::MandatoryParameter { variable: Box::new(var("$", "x")) },
                    loc(0, 2),
                ),
                Node::new(
                    NodeKind::OptionalParameter {
                        variable: Box::new(var("$", "y")),
                        default_value: Box::new(num("0")),
                    },
                    loc(3, 8),
                ),
                Node::new(
                    NodeKind::SlurpyParameter { variable: Box::new(var("@", "rest")) },
                    loc(9, 14),
                ),
            ],
        },
        loc(0, 15),
    );
    let mut count = 0usize;
    node.for_each_child(|_| count += 1);
    assert_eq!(count, 3); // 3 parameter nodes
    Ok(())
}

#[test]
fn for_each_child_leaf_nodes_visit_nothing() -> Result<(), Box<dyn std::error::Error>> {
    let leaves: Vec<Node> = vec![
        Node::new(NodeKind::Variable { sigil: "$".to_string(), name: "x".to_string() }, loc(0, 2)),
        Node::new(NodeKind::Identifier { name: "foo".to_string() }, loc(0, 3)),
        Node::new(NodeKind::Number { value: "42".to_string() }, loc(0, 2)),
        Node::new(NodeKind::String { value: "hi".to_string(), interpolated: false }, loc(0, 4)),
        Node::new(
            NodeKind::Heredoc {
                delimiter: "EOF".to_string(),
                content: "text".to_string(),
                interpolated: false,
                indented: false,
                command: false,
                body_span: None,
            },
            loc(0, 10),
        ),
        Node::new(
            NodeKind::Regex {
                pattern: "abc".to_string(),
                replacement: None,
                modifiers: "".to_string(),
                has_embedded_code: false,
            },
            loc(0, 5),
        ),
        Node::new(NodeKind::Readline { filehandle: Some("STDIN".to_string()) }, loc(0, 7)),
        Node::new(NodeKind::Glob { pattern: "*.pl".to_string() }, loc(0, 5)),
        Node::new(NodeKind::Typeglob { name: "foo".to_string() }, loc(0, 4)),
        Node::new(NodeKind::Diamond, loc(0, 2)),
        Node::new(NodeKind::Ellipsis, loc(0, 3)),
        Node::new(NodeKind::Undef, loc(0, 5)),
        Node::new(
            NodeKind::Use { module: "strict".to_string(), args: vec![], has_filter_risk: false },
            loc(0, 11),
        ),
        Node::new(
            NodeKind::No { module: "warnings".to_string(), args: vec![], has_filter_risk: false },
            loc(0, 13),
        ),
        Node::new(NodeKind::Prototype { content: "$@".to_string() }, loc(0, 4)),
        Node::new(NodeKind::DataSection { marker: "__DATA__".to_string(), body: None }, loc(0, 8)),
        Node::new(
            NodeKind::Format { name: "STDOUT".to_string(), body: "fmt".to_string() },
            loc(0, 10),
        ),
        Node::new(NodeKind::LoopControl { op: "next".to_string(), label: None }, loc(0, 4)),
        Node::new(NodeKind::MissingExpression, loc(0, 0)),
        Node::new(NodeKind::MissingStatement, loc(0, 0)),
        Node::new(NodeKind::MissingIdentifier, loc(0, 0)),
        Node::new(NodeKind::MissingBlock, loc(0, 0)),
        Node::new(NodeKind::UnknownRest, loc(0, 0)),
    ];
    for leaf in &leaves {
        let mut count = 0usize;
        leaf.for_each_child(|_| count += 1);
        assert_eq!(count, 0, "leaf {:?} should have zero children", leaf.kind.kind_name());
    }
    Ok(())
}

// ===========================================================================
// 2. for_each_child_mut — actually mutate children
// ===========================================================================

#[test]
fn for_each_child_mut_can_modify_locations() -> Result<(), Box<dyn std::error::Error>> {
    let mut prog =
        Node::new(NodeKind::Program { statements: vec![num("1"), num("2")] }, loc(0, 10));
    prog.for_each_child_mut(|child| {
        child.location = loc(99, 100);
    });
    if let NodeKind::Program { statements } = &prog.kind {
        for stmt in statements {
            assert_eq!(stmt.location.start, 99);
            assert_eq!(stmt.location.end, 100);
        }
    }
    Ok(())
}

#[test]
fn for_each_child_mut_can_modify_number_values() -> Result<(), Box<dyn std::error::Error>> {
    let mut binary = Node::new(
        NodeKind::Binary {
            op: "+".to_string(),
            left: Box::new(num("10")),
            right: Box::new(num("20")),
        },
        loc(0, 10),
    );
    binary.for_each_child_mut(|child| {
        if let NodeKind::Number { value } = &mut child.kind {
            *value = "0".to_string();
        }
    });
    if let NodeKind::Binary { left, right, .. } = &binary.kind {
        if let NodeKind::Number { value } = &left.kind {
            assert_eq!(value, "0");
        }
        if let NodeKind::Number { value } = &right.kind {
            assert_eq!(value, "0");
        }
    }
    Ok(())
}

#[test]
fn for_each_child_mut_modifies_if_branches() -> Result<(), Box<dyn std::error::Error>> {
    let mut node = Node::new(
        NodeKind::If {
            condition: Box::new(num("1")),
            then_branch: Box::new(block(vec![])),
            elsif_branches: vec![(Box::new(num("2")), Box::new(block(vec![])))],
            else_branch: Some(Box::new(block(vec![]))),
        },
        loc(0, 50),
    );
    let mut visited_names = Vec::new();
    node.for_each_child_mut(|child| {
        visited_names.push(child.kind.kind_name().to_string());
        child.location = loc(42, 43);
    });
    // Should visit: condition, then_branch, elsif_cond, elsif_body, else_branch
    assert_eq!(visited_names.len(), 5);
    Ok(())
}

// ===========================================================================
// 3. children() and first_child() on additional kinds
// ===========================================================================

#[test]
fn children_of_while_with_continue() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::While {
            condition: Box::new(num("1")),
            body: Box::new(block(vec![])),
            continue_block: Some(Box::new(block(vec![]))),
        },
        loc(0, 20),
    );
    let kids = node.children();
    assert_eq!(kids.len(), 3);
    assert_eq!(kids.first().map(|n| n.kind.kind_name()), Some("Number"));
    Ok(())
}

#[test]
fn children_of_while_without_continue() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::While {
            condition: Box::new(num("1")),
            body: Box::new(block(vec![])),
            continue_block: None,
        },
        loc(0, 15),
    );
    let kids = node.children();
    assert_eq!(kids.len(), 2);
    Ok(())
}

#[test]
fn first_child_of_expression_statement() -> Result<(), Box<dyn std::error::Error>> {
    let node =
        Node::new(NodeKind::ExpressionStatement { expression: Box::new(num("7")) }, loc(0, 1));
    let first = node.first_child();
    assert!(first.is_some());
    assert_eq!(first.map(|n| n.kind.kind_name()), Some("Number"));
    Ok(())
}

#[test]
fn first_child_of_unary() -> Result<(), Box<dyn std::error::Error>> {
    let node =
        Node::new(NodeKind::Unary { op: "-".to_string(), operand: Box::new(num("5")) }, loc(0, 2));
    let first = node.first_child();
    assert!(first.is_some());
    assert_eq!(first.map(|n| n.kind.kind_name()), Some("Number"));
    Ok(())
}

#[test]
fn first_child_of_leaf_is_none() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(NodeKind::Diamond, loc(0, 2));
    assert!(node.first_child().is_none());
    Ok(())
}

#[test]
fn children_of_return_with_value() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(NodeKind::Return { value: Some(Box::new(num("42"))) }, loc(0, 10));
    assert_eq!(node.children().len(), 1);
    Ok(())
}

#[test]
fn children_of_return_without_value() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(NodeKind::Return { value: None }, loc(0, 6));
    assert_eq!(node.children().len(), 0);
    assert!(node.first_child().is_none());
    Ok(())
}

#[test]
fn children_of_goto() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(NodeKind::Goto { target: Box::new(ident("DONE")) }, loc(0, 10));
    assert_eq!(node.children().len(), 1);
    assert_eq!(node.first_child().map(|n| n.kind.kind_name()), Some("Identifier"));
    Ok(())
}

#[test]
fn children_of_tie() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::Tie {
            variable: Box::new(var("%", "h")),
            package: Box::new(ident("DB_File")),
            args: vec![],
        },
        loc(0, 15),
    );
    // variable + package = 2
    assert_eq!(node.children().len(), 2);
    Ok(())
}

#[test]
fn children_of_untie() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(NodeKind::Untie { variable: Box::new(var("%", "h")) }, loc(0, 10));
    assert_eq!(node.children().len(), 1);
    Ok(())
}

// ===========================================================================
// 4. count_nodes — deeper nesting
// ===========================================================================

#[test]
fn count_nodes_single_leaf() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(num("42").count_nodes(), 1);
    assert_eq!(Node::new(NodeKind::Diamond, loc(0, 2)).count_nodes(), 1);
    assert_eq!(Node::new(NodeKind::MissingExpression, loc(0, 0)).count_nodes(), 1);
    Ok(())
}

#[test]
fn count_nodes_deeply_nested_unary() -> Result<(), Box<dyn std::error::Error>> {
    // Build a chain: -(-(-(-42)))
    let mut node = num("42");
    for _ in 0..4 {
        node =
            Node::new(NodeKind::Unary { op: "-".to_string(), operand: Box::new(node) }, loc(0, 5));
    }
    // 4 unary wrappers + 1 leaf = 5
    assert_eq!(node.count_nodes(), 5);
    Ok(())
}

#[test]
fn count_nodes_program_with_nested_blocks() -> Result<(), Box<dyn std::error::Error>> {
    let inner_block = block(vec![num("1"), num("2")]);
    let outer_block = block(vec![inner_block, num("3")]);
    let prog = Node::new(NodeKind::Program { statements: vec![outer_block] }, loc(0, 50));
    // prog(1) + outer_block(1) + inner_block(1) + num1(1) + num2(1) + num3(1) = 6
    assert_eq!(prog.count_nodes(), 6);
    Ok(())
}

#[test]
fn count_nodes_hash_literal() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::HashLiteral { pairs: vec![(ident("a"), num("1")), (ident("b"), num("2"))] },
        loc(0, 20),
    );
    // hash(1) + 2 keys(2) + 2 values(2) = 5
    assert_eq!(node.count_nodes(), 5);
    Ok(())
}

#[test]
fn count_nodes_try_catch_finally() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::Try {
            body: Box::new(block(vec![num("1")])),
            catch_blocks: vec![(Some("$e".to_string()), Box::new(block(vec![num("2")])))],
            finally_block: Some(Box::new(block(vec![num("3")]))),
        },
        loc(0, 50),
    );
    // try(1) + body_block(1) + num1(1) + catch_block(1) + num2(1) + finally_block(1) + num3(1) = 7
    assert_eq!(node.count_nodes(), 7);
    Ok(())
}

// ===========================================================================
// 5. PartialEq — nested tree equality edge cases
// ===========================================================================

#[test]
fn eq_identical_nested_trees() -> Result<(), Box<dyn std::error::Error>> {
    let make_tree = || {
        Node::new(
            NodeKind::Binary {
                op: "+".to_string(),
                left: Box::new(Node::new(
                    NodeKind::Unary { op: "-".to_string(), operand: Box::new(num("1")) },
                    loc(0, 2),
                )),
                right: Box::new(num("2")),
            },
            loc(0, 5),
        )
    };
    assert_eq!(make_tree(), make_tree());
    Ok(())
}

#[test]
fn ne_different_operators_same_structure() -> Result<(), Box<dyn std::error::Error>> {
    let a = Node::new(
        NodeKind::Binary {
            op: "+".to_string(),
            left: Box::new(num("1")),
            right: Box::new(num("2")),
        },
        loc(0, 5),
    );
    let b = Node::new(
        NodeKind::Binary {
            op: "-".to_string(),
            left: Box::new(num("1")),
            right: Box::new(num("2")),
        },
        loc(0, 5),
    );
    assert_ne!(a, b);
    Ok(())
}

#[test]
fn ne_different_children_same_kind() -> Result<(), Box<dyn std::error::Error>> {
    let a = Node::new(NodeKind::Program { statements: vec![num("1")] }, loc(0, 10));
    let b = Node::new(NodeKind::Program { statements: vec![num("1"), num("2")] }, loc(0, 10));
    assert_ne!(a, b);
    Ok(())
}

#[test]
fn eq_empty_programs() -> Result<(), Box<dyn std::error::Error>> {
    let a = Node::new(NodeKind::Program { statements: vec![] }, loc(0, 0));
    let b = Node::new(NodeKind::Program { statements: vec![] }, loc(0, 0));
    assert_eq!(a, b);
    Ok(())
}

#[test]
fn ne_missing_variants_differ() -> Result<(), Box<dyn std::error::Error>> {
    let a = Node::new(NodeKind::MissingExpression, loc(0, 0));
    let b = Node::new(NodeKind::MissingStatement, loc(0, 0));
    assert_ne!(a, b);
    Ok(())
}

// ===========================================================================
// 6. SourceLocation boundary conditions
// ===========================================================================

#[test]
fn source_location_zero_length_span() -> Result<(), Box<dyn std::error::Error>> {
    let sl = loc(5, 5);
    assert_eq!(sl.start, 5);
    assert_eq!(sl.end, 5);
    let node = Node::new(NodeKind::MissingExpression, sl);
    assert_eq!(node.location.start, node.location.end);
    Ok(())
}

#[test]
fn source_location_large_offsets() -> Result<(), Box<dyn std::error::Error>> {
    let sl = loc(1_000_000, 1_000_042);
    let node = Node::new(NodeKind::Number { value: "42".to_string() }, sl);
    assert_eq!(node.location.start, 1_000_000);
    assert_eq!(node.location.end, 1_000_042);
    Ok(())
}

#[test]
fn source_location_copy_semantics() -> Result<(), Box<dyn std::error::Error>> {
    let a = loc(10, 20);
    let b = a; // Copy
    assert_eq!(a, b);
    assert_eq!(a.start, b.start);
    assert_eq!(a.end, b.end);
    Ok(())
}

// ===========================================================================
// 7. to_sexp coverage for under-tested node kinds
// ===========================================================================

#[test]
fn sexp_goto() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(NodeKind::Goto { target: Box::new(ident("DONE")) }, loc(0, 10));
    let sexp = node.to_sexp();
    assert!(sexp.starts_with("(goto"), "got: {sexp}");
    assert!(sexp.contains("identifier"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_tie_with_args() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::Tie {
            variable: Box::new(var("%", "h")),
            package: Box::new(ident("DB_File")),
            args: vec![num("1")],
        },
        loc(0, 20),
    );
    let sexp = node.to_sexp();
    assert!(sexp.starts_with("(tie"), "got: {sexp}");
    assert!(sexp.contains("variable"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_tie_without_args() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::Tie {
            variable: Box::new(var("%", "h")),
            package: Box::new(ident("DB_File")),
            args: vec![],
        },
        loc(0, 15),
    );
    let sexp = node.to_sexp();
    assert!(sexp.starts_with("(tie"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_untie() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(NodeKind::Untie { variable: Box::new(var("%", "h")) }, loc(0, 10));
    let sexp = node.to_sexp();
    assert!(sexp.starts_with("(untie"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_regex_with_embedded_code() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::Regex {
            pattern: "(?{ code })".to_string(),
            replacement: None,
            modifiers: "x".to_string(),
            has_embedded_code: true,
        },
        loc(0, 20),
    );
    let sexp = node.to_sexp();
    assert!(sexp.contains("risk:code"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_regex_without_embedded_code() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::Regex {
            pattern: "abc".to_string(),
            replacement: None,
            modifiers: "i".to_string(),
            has_embedded_code: false,
        },
        loc(0, 7),
    );
    let sexp = node.to_sexp();
    assert!(!sexp.contains("risk:code"), "got: {sexp}");
    assert!(sexp.starts_with("(regex"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_match_negated() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::Match {
            expr: Box::new(var("$", "s")),
            pattern: "foo".to_string(),
            modifiers: "".to_string(),
            has_embedded_code: false,
            negated: true,
        },
        loc(0, 15),
    );
    let sexp = node.to_sexp();
    assert!(sexp.starts_with("(not_match"), "negated match should use not_match, got: {sexp}");
    Ok(())
}

#[test]
fn sexp_match_non_negated() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::Match {
            expr: Box::new(var("$", "s")),
            pattern: "foo".to_string(),
            modifiers: "i".to_string(),
            has_embedded_code: false,
            negated: false,
        },
        loc(0, 15),
    );
    let sexp = node.to_sexp();
    assert!(sexp.starts_with("(match"), "got: {sexp}");
    assert!(!sexp.contains("not_match"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_match_with_embedded_code() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::Match {
            expr: Box::new(var("$", "s")),
            pattern: "(?{ code })".to_string(),
            modifiers: "".to_string(),
            has_embedded_code: true,
            negated: false,
        },
        loc(0, 20),
    );
    let sexp = node.to_sexp();
    assert!(sexp.contains("risk:code"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_substitution_negated() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::Substitution {
            expr: Box::new(var("$", "s")),
            pattern: "old".to_string(),
            replacement: "new".to_string(),
            modifiers: "g".to_string(),
            has_embedded_code: false,
            negated: true,
        },
        loc(0, 20),
    );
    let sexp = node.to_sexp();
    assert!(sexp.contains("(negated)"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_substitution_with_embedded_code() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::Substitution {
            expr: Box::new(var("$", "s")),
            pattern: "foo".to_string(),
            replacement: "bar".to_string(),
            modifiers: "e".to_string(),
            has_embedded_code: true,
            negated: false,
        },
        loc(0, 20),
    );
    let sexp = node.to_sexp();
    assert!(sexp.contains("risk:code"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_transliteration_negated() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::Transliteration {
            expr: Box::new(var("$", "s")),
            search: "a-z".to_string(),
            replace: "A-Z".to_string(),
            modifiers: "".to_string(),
            negated: true,
        },
        loc(0, 20),
    );
    let sexp = node.to_sexp();
    assert!(sexp.contains("(negated)"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_use_with_filter_risk() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::Use { module: "Filter::Simple".to_string(), args: vec![], has_filter_risk: true },
        loc(0, 20),
    );
    let sexp = node.to_sexp();
    assert!(sexp.contains("(risk:filter)"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_use_with_args() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::Use {
            module: "strict".to_string(),
            args: vec!["refs".to_string(), "subs".to_string()],
            has_filter_risk: false,
        },
        loc(0, 20),
    );
    let sexp = node.to_sexp();
    assert!(sexp.contains("(use strict (refs subs))"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_use_no_args() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::Use { module: "strict".to_string(), args: vec![], has_filter_risk: false },
        loc(0, 11),
    );
    let sexp = node.to_sexp();
    assert_eq!(sexp, "(use strict)");
    Ok(())
}

#[test]
fn sexp_no_no_args() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::No { module: "strict".to_string(), args: vec![], has_filter_risk: false },
        loc(0, 10),
    );
    let sexp = node.to_sexp();
    assert_eq!(sexp, "(no strict)");
    Ok(())
}

#[test]
fn sexp_no_with_args() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::No {
            module: "warnings".to_string(),
            args: vec!["once".to_string()],
            has_filter_risk: false,
        },
        loc(0, 20),
    );
    let sexp = node.to_sexp();
    assert!(sexp.contains("(no warnings (once))"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_readline_with_filehandle() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(NodeKind::Readline { filehandle: Some("STDIN".to_string()) }, loc(0, 7));
    let sexp = node.to_sexp();
    assert_eq!(sexp, "(readline STDIN)");
    Ok(())
}

#[test]
fn sexp_readline_without_filehandle() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(NodeKind::Readline { filehandle: None }, loc(0, 2));
    let sexp = node.to_sexp();
    assert_eq!(sexp, "(readline)");
    Ok(())
}

#[test]
fn sexp_glob_pattern() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(NodeKind::Glob { pattern: "*.pl".to_string() }, loc(0, 6));
    let sexp = node.to_sexp();
    assert_eq!(sexp, "(glob *.pl)");
    Ok(())
}

#[test]
fn sexp_typeglob() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(NodeKind::Typeglob { name: "main::foo".to_string() }, loc(0, 10));
    let sexp = node.to_sexp();
    assert_eq!(sexp, "(typeglob main::foo)");
    Ok(())
}

#[test]
fn sexp_loop_control_with_label() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::LoopControl { op: "last".to_string(), label: Some("OUTER".to_string()) },
        loc(0, 11),
    );
    let sexp = node.to_sexp();
    assert_eq!(sexp, "(last OUTER)");
    Ok(())
}

#[test]
fn sexp_loop_control_without_label() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(NodeKind::LoopControl { op: "next".to_string(), label: None }, loc(0, 4));
    let sexp = node.to_sexp();
    assert_eq!(sexp, "(next)");
    Ok(())
}

#[test]
fn sexp_loop_control_redo() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(NodeKind::LoopControl { op: "redo".to_string(), label: None }, loc(0, 4));
    let sexp = node.to_sexp();
    assert_eq!(sexp, "(redo)");
    Ok(())
}

#[test]
fn sexp_prototype() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(NodeKind::Prototype { content: "$@%".to_string() }, loc(0, 5));
    let sexp = node.to_sexp();
    assert_eq!(sexp, "(prototype)");
    Ok(())
}

#[test]
fn sexp_signature_with_params() -> Result<(), Box<dyn std::error::Error>> {
    let mand =
        Node::new(NodeKind::MandatoryParameter { variable: Box::new(var("$", "x")) }, loc(0, 2));
    let opt = Node::new(
        NodeKind::OptionalParameter {
            variable: Box::new(var("$", "y")),
            default_value: Box::new(num("0")),
        },
        loc(3, 8),
    );
    let slurpy =
        Node::new(NodeKind::SlurpyParameter { variable: Box::new(var("@", "rest")) }, loc(9, 14));
    let named =
        Node::new(NodeKind::NamedParameter { variable: Box::new(var("$", "k")) }, loc(15, 17));
    let sig =
        Node::new(NodeKind::Signature { parameters: vec![mand, opt, slurpy, named] }, loc(0, 17));
    let sexp = sig.to_sexp();
    assert!(sexp.starts_with("(signature"), "got: {sexp}");
    assert!(sexp.contains("mandatory_parameter"), "got: {sexp}");
    assert!(sexp.contains("optional_parameter"), "got: {sexp}");
    assert!(sexp.contains("slurpy_parameter"), "got: {sexp}");
    assert!(sexp.contains("named_parameter"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_given_when_default() -> Result<(), Box<dyn std::error::Error>> {
    let given = Node::new(
        NodeKind::Given { expr: Box::new(var("$", "x")), body: Box::new(block(vec![])) },
        loc(0, 20),
    );
    assert!(given.to_sexp().starts_with("(given"), "got: {}", given.to_sexp());

    let when = Node::new(
        NodeKind::When { condition: Box::new(num("1")), body: Box::new(block(vec![])) },
        loc(0, 10),
    );
    assert!(when.to_sexp().starts_with("(when"), "got: {}", when.to_sexp());

    let default = Node::new(NodeKind::Default { body: Box::new(block(vec![])) }, loc(0, 10));
    assert!(default.to_sexp().starts_with("(default"), "got: {}", default.to_sexp());
    Ok(())
}

#[test]
fn sexp_while_with_continue() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::While {
            condition: Box::new(num("1")),
            body: Box::new(block(vec![])),
            continue_block: Some(Box::new(block(vec![num("2")]))),
        },
        loc(0, 30),
    );
    let sexp = node.to_sexp();
    assert!(sexp.starts_with("(while"), "got: {sexp}");
    assert!(sexp.contains("(continue"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_labeled_statement() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::LabeledStatement { label: "LOOP".to_string(), statement: Box::new(num("1")) },
        loc(0, 10),
    );
    let sexp = node.to_sexp();
    assert_eq!(sexp, "(labeled_statement LOOP (number 1))");
    Ok(())
}

#[test]
fn sexp_format() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::Format { name: "STDOUT".to_string(), body: "@<<<< @>>>>".to_string() },
        loc(0, 30),
    );
    let sexp = node.to_sexp();
    assert!(sexp.starts_with("(format STDOUT"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_class() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::Class {
            name: "Point".to_string(),
            parents: vec![],
            body: Box::new(block(vec![])),
        },
        loc(0, 20),
    );
    let sexp = node.to_sexp();
    assert!(sexp.starts_with("(class Point"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_data_section_without_body() -> Result<(), Box<dyn std::error::Error>> {
    let node =
        Node::new(NodeKind::DataSection { marker: "__END__".to_string(), body: None }, loc(0, 7));
    let sexp = node.to_sexp();
    assert_eq!(sexp, "(data_section __END__)");
    Ok(())
}

#[test]
fn sexp_identifier() -> Result<(), Box<dyn std::error::Error>> {
    let node = ident("foo_bar");
    let sexp = node.to_sexp();
    assert_eq!(sexp, "(identifier foo_bar)");
    Ok(())
}

#[test]
fn sexp_missing_nodes() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(Node::new(NodeKind::MissingExpression, loc(0, 0)).to_sexp(), "(missing_expression)");
    assert_eq!(Node::new(NodeKind::MissingStatement, loc(0, 0)).to_sexp(), "(missing_statement)");
    assert_eq!(Node::new(NodeKind::MissingIdentifier, loc(0, 0)).to_sexp(), "(missing_identifier)");
    assert_eq!(Node::new(NodeKind::MissingBlock, loc(0, 0)).to_sexp(), "(missing_block)");
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
fn sexp_variable_declaration_without_initializer() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::VariableDeclaration {
            declarator: "my".to_string(),
            variable: Box::new(var("$", "x")),
            attributes: vec![],
            initializer: None,
        },
        loc(0, 5),
    );
    let sexp = node.to_sexp();
    assert!(sexp.starts_with("(my_declaration"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_variable_declaration_with_initializer() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::VariableDeclaration {
            declarator: "my".to_string(),
            variable: Box::new(var("$", "x")),
            attributes: vec![],
            initializer: Some(Box::new(num("42"))),
        },
        loc(0, 10),
    );
    let sexp = node.to_sexp();
    assert!(sexp.starts_with("(my_declaration"), "got: {sexp}");
    assert!(sexp.contains("number 42"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_variable_declaration_with_attributes() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::VariableDeclaration {
            declarator: "our".to_string(),
            variable: Box::new(var("$", "x")),
            attributes: vec!["shared".to_string(), "locked".to_string()],
            initializer: None,
        },
        loc(0, 15),
    );
    let sexp = node.to_sexp();
    assert!(sexp.contains("attributes"), "got: {sexp}");
    assert!(sexp.contains("shared"), "got: {sexp}");
    assert!(sexp.contains("locked"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_variable_list_declaration() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::VariableListDeclaration {
            declarator: "my".to_string(),
            variables: vec![var("$", "a"), var("$", "b")],
            attributes: vec![],
            initializer: None,
        },
        loc(0, 15),
    );
    let sexp = node.to_sexp();
    assert!(sexp.starts_with("(my_declaration ("), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_variable_list_declaration_with_attrs_and_init() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::VariableListDeclaration {
            declarator: "my".to_string(),
            variables: vec![var("$", "a")],
            attributes: vec!["shared".to_string()],
            initializer: Some(Box::new(num("0"))),
        },
        loc(0, 20),
    );
    let sexp = node.to_sexp();
    assert!(sexp.contains("attributes"), "got: {sexp}");
    assert!(sexp.contains("number 0"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_package_with_block() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::Package {
            name: "Foo::Bar".to_string(),
            name_span: loc(8, 16),
            block: Some(Box::new(block(vec![num("1")]))),
        },
        loc(0, 30),
    );
    let sexp = node.to_sexp();
    assert!(sexp.starts_with("(package Foo::Bar"), "got: {sexp}");
    assert!(sexp.contains("block"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_package_without_block() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::Package { name: "Foo".to_string(), name_span: loc(8, 11), block: None },
        loc(0, 12),
    );
    let sexp = node.to_sexp();
    assert_eq!(sexp, "(package Foo)");
    Ok(())
}

#[test]
fn sexp_phase_block() -> Result<(), Box<dyn std::error::Error>> {
    for phase in &["BEGIN", "END", "CHECK", "INIT", "UNITCHECK"] {
        let node = Node::new(
            NodeKind::PhaseBlock {
                phase: phase.to_string(),
                phase_span: Some(loc(0, phase.len())),
                block: Box::new(block(vec![])),
            },
            loc(0, 20),
        );
        let sexp = node.to_sexp();
        assert!(sexp.starts_with(&format!("({phase}")), "phase={phase}, got: {sexp}");
    }
    Ok(())
}

#[test]
fn sexp_return_with_and_without_value() -> Result<(), Box<dyn std::error::Error>> {
    let with = Node::new(NodeKind::Return { value: Some(Box::new(num("42"))) }, loc(0, 10));
    assert_eq!(with.to_sexp(), "(return (number 42))");

    let without = Node::new(NodeKind::Return { value: None }, loc(0, 6));
    assert_eq!(without.to_sexp(), "(return)");
    Ok(())
}

#[test]
fn sexp_method_call() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::MethodCall {
            object: Box::new(var("$", "obj")),
            method: "run".to_string(),
            args: vec![num("1"), num("2")],
        },
        loc(0, 20),
    );
    let sexp = node.to_sexp();
    assert!(sexp.starts_with("(method_call"), "got: {sexp}");
    assert!(sexp.contains("run"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_indirect_call() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::IndirectCall {
            method: "new".to_string(),
            object: Box::new(ident("Foo")),
            args: vec![num("1")],
        },
        loc(0, 10),
    );
    let sexp = node.to_sexp();
    assert!(sexp.starts_with("(indirect_call"), "got: {sexp}");
    assert!(sexp.contains("new"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_error_without_partial() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::Error {
            message: "unexpected end".to_string(),
            expected: vec![],
            found: None,
            partial: None,
        },
        loc(0, 5),
    );
    let sexp = node.to_sexp();
    assert!(sexp.starts_with("(ERROR"), "got: {sexp}");
    assert!(sexp.contains("unexpected end"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_string_interpolated_vs_literal() -> Result<(), Box<dyn std::error::Error>> {
    let interp = Node::new(
        NodeKind::String { value: "hello $name".to_string(), interpolated: true },
        loc(0, 15),
    );
    assert!(interp.to_sexp().contains("string_interpolated"), "got: {}", interp.to_sexp());

    let literal =
        Node::new(NodeKind::String { value: "hello".to_string(), interpolated: false }, loc(0, 7));
    let sexp = literal.to_sexp();
    assert!(sexp.starts_with("(string"), "got: {sexp}");
    assert!(!sexp.contains("interpolated"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_heredoc_plain() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::Heredoc {
            delimiter: "EOF".to_string(),
            content: "hello world".to_string(),
            interpolated: true,
            indented: false,
            command: false,
            body_span: None,
        },
        loc(0, 25),
    );
    let sexp = node.to_sexp();
    assert!(sexp.starts_with("(heredoc_interpolated"), "got: {sexp}");
    Ok(())
}

#[test]
fn sexp_heredoc_non_interpolated() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::Heredoc {
            delimiter: "EOF".to_string(),
            content: "literal text".to_string(),
            interpolated: false,
            indented: false,
            command: false,
            body_span: None,
        },
        loc(0, 25),
    );
    let sexp = node.to_sexp();
    assert!(sexp.starts_with("(heredoc "), "plain heredoc, got: {sexp}");
    Ok(())
}

// ===========================================================================
// 8. Clone independence for Box nodes
// ===========================================================================

#[test]
fn clone_binary_is_independent() -> Result<(), Box<dyn std::error::Error>> {
    let original = Node::new(
        NodeKind::Binary {
            op: "+".to_string(),
            left: Box::new(num("1")),
            right: Box::new(num("2")),
        },
        loc(0, 5),
    );
    let mut cloned = original.clone();
    if let NodeKind::Binary { left, .. } = &mut cloned.kind {
        if let NodeKind::Number { value } = &mut left.kind {
            *value = "999".to_string();
        }
    }
    // Original should be unchanged
    if let NodeKind::Binary { left, .. } = &original.kind {
        if let NodeKind::Number { value } = &left.kind {
            assert_eq!(value, "1", "original should be unmodified after clone mutation");
        }
    }
    Ok(())
}

#[test]
fn clone_if_is_independent() -> Result<(), Box<dyn std::error::Error>> {
    let original = Node::new(
        NodeKind::If {
            condition: Box::new(num("1")),
            then_branch: Box::new(block(vec![num("2")])),
            elsif_branches: vec![],
            else_branch: None,
        },
        loc(0, 20),
    );
    let mut cloned = original.clone();
    if let NodeKind::If { condition, .. } = &mut cloned.kind {
        condition.location = loc(99, 100);
    }
    if let NodeKind::If { condition, .. } = &original.kind {
        assert_eq!(condition.location.start, 0, "original should be unmodified");
    }
    Ok(())
}

// ===========================================================================
// 9. kind_name exhaustiveness — verify all variant names appear in ALL_KIND_NAMES
// ===========================================================================

#[test]
fn all_kind_names_contains_every_variant() -> Result<(), Box<dyn std::error::Error>> {
    // Build one instance of every variant and check its kind_name is in ALL_KIND_NAMES
    let all_variants: Vec<NodeKind> = vec![
        NodeKind::Program { statements: vec![] },
        NodeKind::ExpressionStatement { expression: Box::new(num("1")) },
        NodeKind::VariableDeclaration {
            declarator: "my".to_string(),
            variable: Box::new(var("$", "x")),
            attributes: vec![],
            initializer: None,
        },
        NodeKind::VariableListDeclaration {
            declarator: "my".to_string(),
            variables: vec![],
            attributes: vec![],
            initializer: None,
        },
        NodeKind::Variable { sigil: "$".to_string(), name: "x".to_string() },
        NodeKind::VariableWithAttributes { variable: Box::new(var("$", "x")), attributes: vec![] },
        NodeKind::Assignment {
            lhs: Box::new(var("$", "x")),
            rhs: Box::new(num("1")),
            op: "=".to_string(),
        },
        NodeKind::Binary {
            op: "+".to_string(),
            left: Box::new(num("1")),
            right: Box::new(num("2")),
        },
        NodeKind::Ternary {
            condition: Box::new(num("1")),
            then_expr: Box::new(num("2")),
            else_expr: Box::new(num("3")),
        },
        NodeKind::Unary { op: "-".to_string(), operand: Box::new(num("1")) },
        NodeKind::Diamond,
        NodeKind::Ellipsis,
        NodeKind::Undef,
        NodeKind::Readline { filehandle: None },
        NodeKind::Glob { pattern: "*.pl".to_string() },
        NodeKind::Typeglob { name: "foo".to_string() },
        NodeKind::Number { value: "1".to_string() },
        NodeKind::String { value: "s".to_string(), interpolated: false },
        NodeKind::Heredoc {
            delimiter: "E".to_string(),
            content: "c".to_string(),
            interpolated: false,
            indented: false,
            command: false,
            body_span: None,
        },
        NodeKind::ArrayLiteral { elements: vec![] },
        NodeKind::HashLiteral { pairs: vec![] },
        NodeKind::Block { statements: vec![] },
        NodeKind::Eval { block: Box::new(num("1")) },
        NodeKind::Do { block: Box::new(num("1")) },
        NodeKind::Defer { block: Box::new(num("1")) },
        NodeKind::Try { body: Box::new(block(vec![])), catch_blocks: vec![], finally_block: None },
        NodeKind::If {
            condition: Box::new(num("1")),
            then_branch: Box::new(block(vec![])),
            elsif_branches: vec![],
            else_branch: None,
        },
        NodeKind::LabeledStatement { label: "L".to_string(), statement: Box::new(num("1")) },
        NodeKind::While {
            condition: Box::new(num("1")),
            body: Box::new(block(vec![])),
            continue_block: None,
        },
        NodeKind::Tie {
            variable: Box::new(var("%", "h")),
            package: Box::new(ident("Pkg")),
            args: vec![],
        },
        NodeKind::Untie { variable: Box::new(var("%", "h")) },
        NodeKind::For {
            init: None,
            condition: None,
            update: None,
            body: Box::new(block(vec![])),
            continue_block: None,
        },
        NodeKind::Foreach {
            variable: Box::new(var("$", "i")),
            list: Box::new(Node::new(NodeKind::ArrayLiteral { elements: vec![] }, loc(0, 2))),
            body: Box::new(block(vec![])),
            continue_block: None,
        },
        NodeKind::Given { expr: Box::new(var("$", "x")), body: Box::new(block(vec![])) },
        NodeKind::When { condition: Box::new(num("1")), body: Box::new(block(vec![])) },
        NodeKind::Default { body: Box::new(block(vec![])) },
        NodeKind::StatementModifier {
            statement: Box::new(num("1")),
            modifier: "if".to_string(),
            condition: Box::new(num("1")),
        },
        NodeKind::Subroutine {
            name: None,
            name_span: None,
            prototype: None,
            signature: None,
            attributes: vec![],
            body: Box::new(block(vec![])),
        },
        NodeKind::Prototype { content: "$".to_string() },
        NodeKind::Signature { parameters: vec![] },
        NodeKind::MandatoryParameter { variable: Box::new(var("$", "x")) },
        NodeKind::OptionalParameter {
            variable: Box::new(var("$", "x")),
            default_value: Box::new(num("0")),
        },
        NodeKind::SlurpyParameter { variable: Box::new(var("@", "r")) },
        NodeKind::NamedParameter { variable: Box::new(var("$", "k")) },
        NodeKind::Method {
            name: "m".to_string(),
            signature: None,
            attributes: vec![],
            body: Box::new(block(vec![])),
        },
        NodeKind::Return { value: None },
        NodeKind::LoopControl { op: "next".to_string(), label: None },
        NodeKind::Goto { target: Box::new(ident("L")) },
        NodeKind::MethodCall {
            object: Box::new(var("$", "o")),
            method: "m".to_string(),
            args: vec![],
        },
        NodeKind::FunctionCall { name: "f".to_string(), args: vec![] },
        NodeKind::IndirectCall {
            method: "new".to_string(),
            object: Box::new(ident("C")),
            args: vec![],
        },
        NodeKind::Regex {
            pattern: "p".to_string(),
            replacement: None,
            modifiers: "".to_string(),
            has_embedded_code: false,
        },
        NodeKind::Match {
            expr: Box::new(var("$", "s")),
            pattern: "p".to_string(),
            modifiers: "".to_string(),
            has_embedded_code: false,
            negated: false,
        },
        NodeKind::Substitution {
            expr: Box::new(var("$", "s")),
            pattern: "p".to_string(),
            replacement: "r".to_string(),
            modifiers: "".to_string(),
            has_embedded_code: false,
            negated: false,
        },
        NodeKind::Transliteration {
            expr: Box::new(var("$", "s")),
            search: "a".to_string(),
            replace: "b".to_string(),
            modifiers: "".to_string(),
            negated: false,
        },
        NodeKind::Package { name: "P".to_string(), name_span: loc(0, 1), block: None },
        NodeKind::Use { module: "M".to_string(), args: vec![], has_filter_risk: false },
        NodeKind::No { module: "M".to_string(), args: vec![], has_filter_risk: false },
        NodeKind::PhaseBlock {
            phase: "BEGIN".to_string(),
            phase_span: None,
            block: Box::new(block(vec![])),
        },
        NodeKind::DataSection { marker: "__DATA__".to_string(), body: None },
        NodeKind::Class { name: "C".to_string(), parents: vec![], body: Box::new(block(vec![])) },
        NodeKind::Format { name: "F".to_string(), body: "b".to_string() },
        NodeKind::Identifier { name: "id".to_string() },
        NodeKind::Error { message: "e".to_string(), expected: vec![], found: None, partial: None },
        NodeKind::MissingExpression,
        NodeKind::MissingStatement,
        NodeKind::MissingIdentifier,
        NodeKind::MissingBlock,
        NodeKind::UnknownRest,
    ];

    let all_names: std::collections::HashSet<&str> =
        NodeKind::ALL_KIND_NAMES.iter().copied().collect();

    for variant in &all_variants {
        let name = variant.kind_name();
        assert!(all_names.contains(name), "kind_name {:?} not found in ALL_KIND_NAMES", name);
    }

    // Verify completeness: variant count matches ALL_KIND_NAMES count
    assert_eq!(
        all_variants.len(),
        NodeKind::ALL_KIND_NAMES.len(),
        "variant count ({}) should match ALL_KIND_NAMES count ({})",
        all_variants.len(),
        NodeKind::ALL_KIND_NAMES.len(),
    );
    Ok(())
}

// ===========================================================================
// 10. RECOVERY_KIND_NAMES is a subset of ALL_KIND_NAMES
// ===========================================================================

#[test]
fn recovery_kind_names_is_subset_of_all() -> Result<(), Box<dyn std::error::Error>> {
    let all: std::collections::HashSet<&str> = NodeKind::ALL_KIND_NAMES.iter().copied().collect();
    for name in NodeKind::RECOVERY_KIND_NAMES {
        assert!(all.contains(name), "recovery kind {:?} not in ALL_KIND_NAMES", name);
    }
    Ok(())
}

// ===========================================================================
// 11. Statement modifier sexp with various modifier keywords
// ===========================================================================

#[test]
fn sexp_statement_modifier_variants() -> Result<(), Box<dyn std::error::Error>> {
    for modifier in &["if", "unless", "while", "until", "for", "foreach"] {
        let node = Node::new(
            NodeKind::StatementModifier {
                statement: Box::new(num("1")),
                modifier: modifier.to_string(),
                condition: Box::new(num("1")),
            },
            loc(0, 15),
        );
        let sexp = node.to_sexp();
        assert!(
            sexp.starts_with(&format!("(statement_modifier_{modifier}")),
            "modifier={modifier}, got: {sexp}"
        );
    }
    Ok(())
}

// ===========================================================================
// 12. Builtin function call sexp variants
// ===========================================================================

#[test]
fn sexp_builtin_function_calls() -> Result<(), Box<dyn std::error::Error>> {
    let builtins = [
        "bless", "shift", "unshift", "open", "die", "warn", "print", "printf", "say", "push",
        "pop", "map", "sort", "grep", "keys", "values", "each", "defined", "scalar", "ref",
    ];
    for name in &builtins {
        let node = Node::new(
            NodeKind::FunctionCall { name: name.to_string(), args: vec![num("1")] },
            loc(0, 10),
        );
        let sexp = node.to_sexp();
        assert!(sexp.starts_with(&format!("(call {name}")), "builtin={name}, got: {sexp}");
    }
    Ok(())
}

// ===========================================================================
// 13. for_each_child on error with and without partial
// ===========================================================================

#[test]
fn for_each_child_error_with_partial() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::Error {
            message: "err".to_string(),
            expected: vec![],
            found: None,
            partial: Some(Box::new(num("1"))),
        },
        loc(0, 5),
    );
    let mut count = 0usize;
    node.for_each_child(|_| count += 1);
    assert_eq!(count, 1);
    Ok(())
}

#[test]
fn for_each_child_error_without_partial() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::Error {
            message: "err".to_string(),
            expected: vec![],
            found: None,
            partial: None,
        },
        loc(0, 5),
    );
    let mut count = 0usize;
    node.for_each_child(|_| count += 1);
    assert_eq!(count, 0);
    Ok(())
}

// ===========================================================================
// 14. for/foreach with no optional parts
// ===========================================================================

#[test]
fn sexp_for_no_init_no_condition_no_update() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::For {
            init: None,
            condition: None,
            update: None,
            body: Box::new(block(vec![])),
            continue_block: None,
        },
        loc(0, 10),
    );
    let sexp = node.to_sexp();
    // Should have empty () placeholders
    assert!(sexp.contains("(for () () ()"), "got: {sexp}");
    Ok(())
}

// ===========================================================================
// 15. Named subroutine with prototype node (not just string)
// ===========================================================================

#[test]
fn sexp_named_subroutine_with_prototype() -> Result<(), Box<dyn std::error::Error>> {
    let proto = Node::new(NodeKind::Prototype { content: "$$".to_string() }, loc(0, 4));
    let node = Node::new(
        NodeKind::Subroutine {
            name: Some("test_fn".to_string()),
            name_span: Some(loc(4, 11)),
            prototype: Some(Box::new(proto)),
            signature: None,
            attributes: vec![],
            body: Box::new(block(vec![])),
        },
        loc(0, 30),
    );
    let sexp = node.to_sexp();
    assert!(sexp.contains("(sub test_fn"), "got: {sexp}");
    assert!(sexp.contains("prototype"), "got: {sexp}");
    Ok(())
}

// ===========================================================================
// 16. Anonymous subroutine with signature
// ===========================================================================

#[test]
fn sexp_anonymous_subroutine_with_signature() -> Result<(), Box<dyn std::error::Error>> {
    let sig = Node::new(
        NodeKind::Signature {
            parameters: vec![Node::new(
                NodeKind::MandatoryParameter { variable: Box::new(var("$", "x")) },
                loc(0, 2),
            )],
        },
        loc(0, 4),
    );
    let node = Node::new(
        NodeKind::Subroutine {
            name: None,
            name_span: None,
            prototype: None,
            signature: Some(Box::new(sig)),
            attributes: vec![],
            body: Box::new(block(vec![])),
        },
        loc(0, 15),
    );
    let sexp = node.to_sexp();
    assert!(sexp.contains("anonymous_subroutine_expression"), "got: {sexp}");
    assert!(sexp.contains("signature"), "got: {sexp}");
    Ok(())
}

// ===========================================================================
// 17. Regex with replacement
// ===========================================================================

#[test]
fn sexp_regex_with_replacement() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::Regex {
            pattern: "foo".to_string(),
            replacement: Some("bar".to_string()),
            modifiers: "g".to_string(),
            has_embedded_code: false,
        },
        loc(0, 15),
    );
    let sexp = node.to_sexp();
    assert!(sexp.contains("regex"), "got: {sexp}");
    assert!(sexp.contains("foo"), "got: {sexp}");
    assert!(sexp.contains("bar"), "got: {sexp}");
    Ok(())
}

// ===========================================================================
// 18. Transliteration sexp (non-negated)
// ===========================================================================

#[test]
fn sexp_transliteration_non_negated() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(
        NodeKind::Transliteration {
            expr: Box::new(var("$", "s")),
            search: "a-z".to_string(),
            replace: "A-Z".to_string(),
            modifiers: "cd".to_string(),
            negated: false,
        },
        loc(0, 20),
    );
    let sexp = node.to_sexp();
    assert!(sexp.starts_with("(transliteration"), "got: {sexp}");
    assert!(!sexp.contains("negated"), "got: {sexp}");
    Ok(())
}

// ===========================================================================
// 19. to_sexp_inner for non-ExpressionStatement nodes
// ===========================================================================

#[test]
fn sexp_inner_delegates_to_sexp_for_non_expression_statement()
-> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(NodeKind::Number { value: "42".to_string() }, loc(0, 2));
    assert_eq!(node.to_sexp_inner(), node.to_sexp());

    let block_node = block(vec![num("1")]);
    assert_eq!(block_node.to_sexp_inner(), block_node.to_sexp());
    Ok(())
}

// ===========================================================================
// 20. Debug output contains struct fields
// ===========================================================================

#[test]
fn debug_node_contains_location_info() -> Result<(), Box<dyn std::error::Error>> {
    let node = Node::new(NodeKind::Number { value: "42".to_string() }, loc(5, 7));
    let dbg = format!("{:?}", node);
    assert!(dbg.contains("5"), "debug should show start offset, got: {dbg}");
    assert!(dbg.contains("7"), "debug should show end offset, got: {dbg}");
    Ok(())
}

#[test]
fn debug_nodekind_shows_variant_fields() -> Result<(), Box<dyn std::error::Error>> {
    let kind = NodeKind::Variable { sigil: "@".to_string(), name: "arr".to_string() };
    let dbg = format!("{:?}", kind);
    assert!(dbg.contains("Variable"), "got: {dbg}");
    assert!(dbg.contains("@"), "got: {dbg}");
    assert!(dbg.contains("arr"), "got: {dbg}");
    Ok(())
}
