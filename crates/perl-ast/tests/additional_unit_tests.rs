//! Additional unit tests for the `perl-ast` crate.
//!
//! Covers areas not exercised by the existing comprehensive_unit_tests.rs:
//! - Debug/Display trait output
//! - Clone/PartialEq trait behavior on complex trees
//! - for_each_child_mut on more node kinds
//! - for_each_child immutable on additional kinds
//! - count_nodes on complex nested structures
//! - Operator formatting edge cases (binary & unary)
//! - to_sexp edge cases (empty containers, special chars, heredoc variants)
//! - v2 module additional coverage
//! - ALL_KIND_NAMES / RECOVERY_KIND_NAMES properties

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

fn block_node(statements: Vec<Node>) -> Node {
    Node::new(NodeKind::Block { statements }, loc(0, 1))
}

// ===========================================================================
// 1. Debug trait output
// ===========================================================================

#[test]
fn debug_output_for_number_node() {
    let node = num_node("42");
    let dbg = format!("{:?}", node);
    assert!(dbg.contains("Number"), "Debug should contain variant name, got: {dbg}");
    assert!(dbg.contains("42"), "Debug should contain value, got: {dbg}");
}

#[test]
fn debug_output_for_variable_node() {
    let node = var_node("$", "x");
    let dbg = format!("{:?}", node);
    assert!(dbg.contains("Variable"), "got: {dbg}");
    assert!(dbg.contains("$"), "got: {dbg}");
}

#[test]
fn debug_output_for_program_node() {
    let prog = Node::new(NodeKind::Program { statements: vec![num_node("1")] }, loc(0, 10));
    let dbg = format!("{:?}", prog);
    assert!(dbg.contains("Program"), "got: {dbg}");
}

#[test]
fn debug_output_for_error_node() {
    let err = Node::new(
        NodeKind::Error {
            message: "oops".to_string(),
            expected: vec![],
            found: None,
            partial: None,
        },
        loc(0, 1),
    );
    let dbg = format!("{:?}", err);
    assert!(dbg.contains("Error"), "got: {dbg}");
    assert!(dbg.contains("oops"), "got: {dbg}");
}

#[test]
fn debug_output_for_unit_variants() {
    let variants = [
        Node::new(NodeKind::Diamond, loc(0, 2)),
        Node::new(NodeKind::Ellipsis, loc(0, 3)),
        Node::new(NodeKind::Undef, loc(0, 5)),
        Node::new(NodeKind::MissingExpression, loc(0, 0)),
        Node::new(NodeKind::UnknownRest, loc(0, 0)),
    ];
    let names = ["Diamond", "Ellipsis", "Undef", "MissingExpression", "UnknownRest"];
    for (node, name) in variants.iter().zip(names.iter()) {
        let dbg = format!("{:?}", node);
        assert!(dbg.contains(name), "expected {name} in debug output, got: {dbg}");
    }
}

// ===========================================================================
// 2. Clone / PartialEq on complex trees
// ===========================================================================

#[test]
fn clone_deep_tree_preserves_equality() {
    let inner = Node::new(
        NodeKind::Binary {
            op: "+".to_string(),
            left: Box::new(num_node("1")),
            right: Box::new(num_node("2")),
        },
        loc(0, 5),
    );
    let outer =
        Node::new(NodeKind::Unary { op: "-".to_string(), operand: Box::new(inner) }, loc(0, 6));
    let cloned = outer.clone();
    assert_eq!(outer, cloned);
}

#[test]
fn cloned_tree_is_independent() {
    let mut original = Node::new(NodeKind::Program { statements: vec![num_node("1")] }, loc(0, 10));
    let cloned = original.clone();
    // Mutate original
    if let NodeKind::Program { statements } = &mut original.kind {
        statements.push(num_node("2"));
    }
    // Clone should not be affected
    if let NodeKind::Program { statements } = &cloned.kind {
        assert_eq!(statements.len(), 1);
    }
}

#[test]
fn partial_eq_returns_false_for_different_locations() {
    let a = Node::new(NodeKind::Number { value: "1".to_string() }, loc(0, 1));
    let b = Node::new(NodeKind::Number { value: "1".to_string() }, loc(5, 6));
    assert_ne!(a, b);
}

#[test]
fn partial_eq_returns_false_for_different_kinds() {
    let a = Node::new(NodeKind::Number { value: "1".to_string() }, loc(0, 1));
    let b = Node::new(NodeKind::String { value: "1".to_string(), interpolated: false }, loc(0, 1));
    assert_ne!(a, b);
}

// ===========================================================================
// 3. for_each_child_mut on more node kinds
// ===========================================================================

#[test]
fn for_each_child_mut_visits_while_continue() {
    let mut node = Node::new(
        NodeKind::While {
            condition: Box::new(num_node("1")),
            body: Box::new(block_node(vec![])),
            continue_block: Some(Box::new(block_node(vec![num_node("99")]))),
        },
        loc(0, 20),
    );
    let mut count = 0;
    node.for_each_child_mut(|_| count += 1);
    // condition + body + continue_block = 3
    assert_eq!(count, 3);
}

#[test]
fn for_each_child_mut_visits_given_when_default() {
    let mut given = Node::new(
        NodeKind::Given { expr: Box::new(var_node("$", "x")), body: Box::new(block_node(vec![])) },
        loc(0, 20),
    );
    let mut count = 0;
    given.for_each_child_mut(|_| count += 1);
    assert_eq!(count, 2);

    let mut when = Node::new(
        NodeKind::When { condition: Box::new(num_node("1")), body: Box::new(block_node(vec![])) },
        loc(0, 10),
    );
    count = 0;
    when.for_each_child_mut(|_| count += 1);
    assert_eq!(count, 2);

    let mut default =
        Node::new(NodeKind::Default { body: Box::new(block_node(vec![])) }, loc(0, 10));
    count = 0;
    default.for_each_child_mut(|_| count += 1);
    assert_eq!(count, 1);
}

#[test]
fn for_each_child_mut_visits_statement_modifier() {
    let mut node = Node::new(
        NodeKind::StatementModifier {
            statement: Box::new(num_node("42")),
            modifier: "if".to_string(),
            condition: Box::new(num_node("1")),
        },
        loc(0, 10),
    );
    let mut count = 0;
    node.for_each_child_mut(|_| count += 1);
    assert_eq!(count, 2);
}

#[test]
fn for_each_child_mut_visits_labeled_statement() {
    let mut node = Node::new(
        NodeKind::LabeledStatement {
            label: "OUTER".to_string(),
            statement: Box::new(num_node("1")),
        },
        loc(0, 10),
    );
    let mut count = 0;
    node.for_each_child_mut(|_| count += 1);
    assert_eq!(count, 1);
}

#[test]
fn for_each_child_mut_visits_eval_and_do() {
    let mut eval = Node::new(NodeKind::Eval { block: Box::new(block_node(vec![])) }, loc(0, 10));
    let mut count = 0;
    eval.for_each_child_mut(|_| count += 1);
    assert_eq!(count, 1);

    let mut do_node = Node::new(NodeKind::Do { block: Box::new(block_node(vec![])) }, loc(0, 10));
    count = 0;
    do_node.for_each_child_mut(|_| count += 1);
    assert_eq!(count, 1);
}

#[test]
fn for_each_child_mut_visits_expression_statement() {
    let mut es =
        Node::new(NodeKind::ExpressionStatement { expression: Box::new(num_node("7")) }, loc(0, 1));
    let mut count = 0;
    es.for_each_child_mut(|_| count += 1);
    assert_eq!(count, 1);
}

#[test]
fn for_each_child_mut_visits_variable_with_attributes() {
    let mut node = Node::new(
        NodeKind::VariableWithAttributes {
            variable: Box::new(var_node("$", "x")),
            attributes: vec!["lvalue".to_string()],
        },
        loc(0, 5),
    );
    let mut count = 0;
    node.for_each_child_mut(|_| count += 1);
    assert_eq!(count, 1);
}

#[test]
fn for_each_child_mut_visits_assignment() {
    let mut node = Node::new(
        NodeKind::Assignment {
            lhs: Box::new(var_node("$", "x")),
            rhs: Box::new(num_node("1")),
            op: "=".to_string(),
        },
        loc(0, 6),
    );
    let mut count = 0;
    node.for_each_child_mut(|_| count += 1);
    assert_eq!(count, 2);
}

#[test]
fn for_each_child_mut_visits_ternary() {
    let mut node = Node::new(
        NodeKind::Ternary {
            condition: Box::new(num_node("1")),
            then_expr: Box::new(num_node("2")),
            else_expr: Box::new(num_node("3")),
        },
        loc(0, 10),
    );
    let mut count = 0;
    node.for_each_child_mut(|_| count += 1);
    assert_eq!(count, 3);
}

#[test]
fn for_each_child_mut_visits_function_call_args() {
    let mut node = Node::new(
        NodeKind::FunctionCall {
            name: "foo".to_string(),
            args: vec![num_node("1"), num_node("2"), num_node("3")],
        },
        loc(0, 10),
    );
    let mut count = 0;
    node.for_each_child_mut(|_| count += 1);
    assert_eq!(count, 3);
}

#[test]
fn for_each_child_mut_visits_indirect_call() {
    let mut node = Node::new(
        NodeKind::IndirectCall {
            method: "new".to_string(),
            object: Box::new(Node::new(
                NodeKind::Identifier { name: "Foo".to_string() },
                loc(0, 3),
            )),
            args: vec![num_node("1")],
        },
        loc(0, 10),
    );
    let mut count = 0;
    node.for_each_child_mut(|_| count += 1);
    // object + 1 arg = 2
    assert_eq!(count, 2);
}

#[test]
fn for_each_child_mut_visits_return_with_value() {
    let mut ret = Node::new(NodeKind::Return { value: Some(Box::new(num_node("42"))) }, loc(0, 10));
    let mut count = 0;
    ret.for_each_child_mut(|_| count += 1);
    assert_eq!(count, 1);
}

#[test]
fn for_each_child_mut_visits_return_without_value() {
    let mut ret = Node::new(NodeKind::Return { value: None }, loc(0, 6));
    let mut count = 0;
    ret.for_each_child_mut(|_| count += 1);
    assert_eq!(count, 0);
}

#[test]
fn for_each_child_mut_visits_match_node() {
    let mut node = Node::new(
        NodeKind::Match {
            expr: Box::new(var_node("$", "s")),
            pattern: "foo".to_string(),
            modifiers: "i".to_string(),
            has_embedded_code: false,
            negated: false,
        },
        loc(0, 10),
    );
    let mut count = 0;
    node.for_each_child_mut(|_| count += 1);
    assert_eq!(count, 1);
}

#[test]
fn for_each_child_mut_visits_substitution_node() {
    let mut node = Node::new(
        NodeKind::Substitution {
            expr: Box::new(var_node("$", "s")),
            pattern: "foo".to_string(),
            replacement: "bar".to_string(),
            modifiers: "g".to_string(),
            has_embedded_code: false,
            negated: false,
        },
        loc(0, 15),
    );
    let mut count = 0;
    node.for_each_child_mut(|_| count += 1);
    assert_eq!(count, 1);
}

#[test]
fn for_each_child_mut_visits_transliteration_node() {
    let mut node = Node::new(
        NodeKind::Transliteration {
            expr: Box::new(var_node("$", "s")),
            search: "a-z".to_string(),
            replace: "A-Z".to_string(),
            modifiers: "".to_string(),
            negated: false,
        },
        loc(0, 15),
    );
    let mut count = 0;
    node.for_each_child_mut(|_| count += 1);
    assert_eq!(count, 1);
}

#[test]
fn for_each_child_mut_visits_package_with_block() {
    let mut node = Node::new(
        NodeKind::Package {
            name: "Foo".to_string(),
            name_span: loc(8, 11),
            block: Some(Box::new(block_node(vec![]))),
        },
        loc(0, 20),
    );
    let mut count = 0;
    node.for_each_child_mut(|_| count += 1);
    assert_eq!(count, 1);
}

#[test]
fn for_each_child_mut_package_without_block() {
    let mut node = Node::new(
        NodeKind::Package { name: "Foo".to_string(), name_span: loc(8, 11), block: None },
        loc(0, 12),
    );
    let mut count = 0;
    node.for_each_child_mut(|_| count += 1);
    assert_eq!(count, 0);
}

#[test]
fn for_each_child_mut_visits_phase_block() {
    let mut node = Node::new(
        NodeKind::PhaseBlock {
            phase: "BEGIN".to_string(),
            phase_span: None,
            block: Box::new(block_node(vec![])),
        },
        loc(0, 10),
    );
    let mut count = 0;
    node.for_each_child_mut(|_| count += 1);
    assert_eq!(count, 1);
}

#[test]
fn for_each_child_mut_visits_class() {
    let mut node = Node::new(
        NodeKind::Class {
            name: "Point".to_string(),
            parents: vec![],
            body: Box::new(block_node(vec![])),
        },
        loc(0, 15),
    );
    let mut count = 0;
    node.for_each_child_mut(|_| count += 1);
    assert_eq!(count, 1);
}

#[test]
fn for_each_child_mut_visits_method_with_signature() {
    let sig = Node::new(NodeKind::Signature { parameters: vec![] }, loc(0, 2));
    let mut node = Node::new(
        NodeKind::Method {
            name: "greet".to_string(),
            signature: Some(Box::new(sig)),
            attributes: vec![],
            body: Box::new(block_node(vec![])),
        },
        loc(0, 20),
    );
    let mut count = 0;
    node.for_each_child_mut(|_| count += 1);
    // signature + body = 2
    assert_eq!(count, 2);
}

#[test]
fn for_each_child_mut_visits_optional_parameter() {
    let mut node = Node::new(
        NodeKind::OptionalParameter {
            variable: Box::new(var_node("$", "x")),
            default_value: Box::new(num_node("0")),
        },
        loc(0, 5),
    );
    let mut count = 0;
    node.for_each_child_mut(|_| count += 1);
    assert_eq!(count, 2);
}

#[test]
fn for_each_child_mut_visits_mandatory_parameter() {
    let mut node = Node::new(
        NodeKind::MandatoryParameter { variable: Box::new(var_node("$", "x")) },
        loc(0, 2),
    );
    let mut count = 0;
    node.for_each_child_mut(|_| count += 1);
    assert_eq!(count, 1);
}

#[test]
fn for_each_child_mut_visits_slurpy_and_named_parameter() {
    let mut slurpy = Node::new(
        NodeKind::SlurpyParameter { variable: Box::new(var_node("@", "rest")) },
        loc(0, 5),
    );
    let mut count = 0;
    slurpy.for_each_child_mut(|_| count += 1);
    assert_eq!(count, 1);

    let mut named =
        Node::new(NodeKind::NamedParameter { variable: Box::new(var_node("$", "k")) }, loc(0, 2));
    count = 0;
    named.for_each_child_mut(|_| count += 1);
    assert_eq!(count, 1);
}

#[test]
fn for_each_child_mut_visits_untie() {
    let mut node =
        Node::new(NodeKind::Untie { variable: Box::new(var_node("%", "h")) }, loc(0, 10));
    let mut count = 0;
    node.for_each_child_mut(|_| count += 1);
    assert_eq!(count, 1);
}

#[test]
fn for_each_child_mut_visits_variable_list_declaration() {
    let mut node = Node::new(
        NodeKind::VariableListDeclaration {
            declarator: "my".to_string(),
            variables: vec![var_node("$", "a"), var_node("$", "b")],
            attributes: vec![],
            initializer: Some(Box::new(num_node("0"))),
        },
        loc(0, 15),
    );
    let mut count = 0;
    node.for_each_child_mut(|_| count += 1);
    // 2 vars + 1 init = 3
    assert_eq!(count, 3);
}

// ===========================================================================
// 4. count_nodes on complex structures
// ===========================================================================

#[test]
fn count_nodes_if_with_branches() {
    let node = Node::new(
        NodeKind::If {
            condition: Box::new(num_node("1")),
            then_branch: Box::new(block_node(vec![num_node("2")])),
            elsif_branches: vec![(
                Box::new(num_node("3")),
                Box::new(block_node(vec![num_node("4")])),
            )],
            else_branch: Some(Box::new(block_node(vec![num_node("5")]))),
        },
        loc(0, 50),
    );
    // if(1) + condition(1) + then_block(1) + stmt_in_then(1)
    // + elsif_cond(1) + elsif_block(1) + stmt_in_elsif(1)
    // + else_block(1) + stmt_in_else(1) = 9
    assert_eq!(node.count_nodes(), 9);
}

#[test]
fn count_nodes_foreach_with_continue() {
    let node = Node::new(
        NodeKind::Foreach {
            variable: Box::new(var_node("$", "i")),
            list: Box::new(Node::new(
                NodeKind::ArrayLiteral { elements: vec![num_node("1"), num_node("2")] },
                loc(0, 5),
            )),
            body: Box::new(block_node(vec![])),
            continue_block: Some(Box::new(block_node(vec![]))),
        },
        loc(0, 30),
    );
    // foreach(1) + var(1) + array(1) + elem1(1) + elem2(1) + body(1) + continue(1) = 7
    assert_eq!(node.count_nodes(), 7);
}

#[test]
fn count_nodes_subroutine_with_signature() {
    let sig = Node::new(
        NodeKind::Signature {
            parameters: vec![Node::new(
                NodeKind::MandatoryParameter { variable: Box::new(var_node("$", "x")) },
                loc(0, 2),
            )],
        },
        loc(0, 4),
    );
    let node = Node::new(
        NodeKind::Subroutine {
            name: Some("add".to_string()),
            name_span: Some(loc(4, 7)),
            prototype: None,
            signature: Some(Box::new(sig)),
            attributes: vec![],
            body: Box::new(block_node(vec![num_node("42")])),
        },
        loc(0, 30),
    );
    // sub(1) + sig(1) + mandatory_param(1) + var(1) + body_block(1) + num(1) = 6
    assert_eq!(node.count_nodes(), 6);
}

// ===========================================================================
// 5. to_sexp edge cases
// ===========================================================================

#[test]
fn sexp_binary_operators_comprehensive() {
    let ops_and_expected = [
        ("==", "binary_=="),
        ("!=", "binary_!="),
        ("<=", "binary_<="),
        (">=", "binary_>="),
        ("<=>", "binary_<=>"),
        ("eq", "binary_eq"),
        ("ne", "binary_ne"),
        ("&&", "binary_&&"),
        ("||", "binary_||"),
        ("and", "binary_and"),
        ("or", "binary_or"),
        (".", "binary_."),
        ("..", "binary_.."),
        ("//", "binary_//"),
        ("~~", "binary_~~"),
        ("isa", "binary_isa"),
        ("->", "binary_->"),
        ("=~", "binary_=~"),
        ("!~", "binary_!~"),
    ];
    for (op, expected_prefix) in &ops_and_expected {
        let node = Node::new(
            NodeKind::Binary {
                op: op.to_string(),
                left: Box::new(num_node("1")),
                right: Box::new(num_node("2")),
            },
            loc(0, 5),
        );
        let sexp = node.to_sexp();
        assert!(
            sexp.starts_with(&format!("({}", expected_prefix)),
            "op={op}, expected prefix '({expected_prefix}', got: {sexp}"
        );
    }
}

#[test]
fn sexp_unary_operators_comprehensive() {
    let ops_and_expected = [
        ("+", "unary_+"),
        ("-", "unary_-"),
        ("!", "unary_not"),
        ("not", "unary_not"),
        ("~", "unary_complement"),
        ("\\", "unary_ref"),
        ("++", "unary_++"),
        ("--", "unary_--"),
        ("-f", "unary_-f"),
        ("-d", "unary_-d"),
        ("-e", "unary_-e"),
        ("defined", "unary_defined"),
    ];
    for (op, expected_prefix) in &ops_and_expected {
        let node = Node::new(
            NodeKind::Unary { op: op.to_string(), operand: Box::new(var_node("$", "x")) },
            loc(0, 5),
        );
        let sexp = node.to_sexp();
        assert!(
            sexp.starts_with(&format!("({}", expected_prefix)),
            "op={op}, expected prefix '({expected_prefix}', got: {sexp}"
        );
    }
}

#[test]
fn sexp_unary_file_test_operators() {
    let file_tests = [
        "-r", "-w", "-x", "-o", "-R", "-W", "-X", "-O", "-s", "-p", "-S", "-b", "-c", "-t", "-u",
        "-g", "-k", "-T", "-B", "-M", "-A", "-C", "-l", "-z",
    ];
    for op in &file_tests {
        let node = Node::new(
            NodeKind::Unary { op: op.to_string(), operand: Box::new(var_node("$", "f")) },
            loc(0, 5),
        );
        let sexp = node.to_sexp();
        assert!(sexp.starts_with(&format!("(unary_{}", op)), "op={op}, got: {sexp}");
    }
}

#[test]
fn sexp_unary_postfix_deref_operators() {
    let ops = ["->@*", "->%*", "->$*", "->&*", "->**"];
    for op in &ops {
        let node = Node::new(
            NodeKind::Unary { op: op.to_string(), operand: Box::new(var_node("$", "r")) },
            loc(0, 5),
        );
        let sexp = node.to_sexp();
        assert!(sexp.starts_with(&format!("(unary_{}", op)), "op={op}, got: {sexp}");
    }
}

#[test]
fn sexp_binary_assignment_operators() {
    let ops = [
        "+=", "-=", "*=", "/=", "%=", "**=", ".=", "&=", "|=", "^=", "<<=", ">>=", "&&=", "||=",
        "//=",
    ];
    for op in &ops {
        let node = Node::new(
            NodeKind::Binary {
                op: op.to_string(),
                left: Box::new(var_node("$", "x")),
                right: Box::new(num_node("1")),
            },
            loc(0, 5),
        );
        let sexp = node.to_sexp();
        assert!(sexp.starts_with(&format!("(binary_{}", op)), "op={op}, got: {sexp}");
    }
}

#[test]
fn sexp_string_escapes_quotes() {
    let node = Node::new(
        NodeKind::String { value: "he said \"hello\"".to_string(), interpolated: false },
        loc(0, 20),
    );
    let sexp = node.to_sexp();
    assert!(sexp.contains("\\\""), "quotes should be escaped in sexp, got: {sexp}");
}

#[test]
fn sexp_string_escapes_backslashes() {
    let node = Node::new(
        NodeKind::String { value: "path\\to\\file".to_string(), interpolated: false },
        loc(0, 15),
    );
    let sexp = node.to_sexp();
    assert!(sexp.contains("\\\\"), "backslashes should be escaped, got: {sexp}");
}

#[test]
fn sexp_heredoc_command_variant() {
    let node = Node::new(
        NodeKind::Heredoc {
            delimiter: "CMD".to_string(),
            content: "ls -la".to_string(),
            interpolated: false,
            indented: false,
            command: true,
            body_span: None,
        },
        loc(0, 20),
    );
    let sexp = node.to_sexp();
    assert!(sexp.starts_with("(heredoc_command"), "got: {sexp}");
}

#[test]
fn sexp_heredoc_indented_interpolated_variant() {
    let node = Node::new(
        NodeKind::Heredoc {
            delimiter: "HTML".to_string(),
            content: "  <p>$name</p>".to_string(),
            interpolated: true,
            indented: true,
            command: false,
            body_span: None,
        },
        loc(0, 30),
    );
    let sexp = node.to_sexp();
    assert!(sexp.starts_with("(heredoc_indented_interpolated"), "got: {sexp}");
}

#[test]
fn sexp_heredoc_indented_non_interpolated() {
    let node = Node::new(
        NodeKind::Heredoc {
            delimiter: "TEXT".to_string(),
            content: "  literal".to_string(),
            interpolated: false,
            indented: true,
            command: false,
            body_span: None,
        },
        loc(0, 20),
    );
    let sexp = node.to_sexp();
    assert!(sexp.starts_with("(heredoc_indented "), "got: {sexp}");
}

#[test]
fn sexp_function_call_builtin_no_args() {
    let node =
        Node::new(NodeKind::FunctionCall { name: "shift".to_string(), args: vec![] }, loc(0, 5));
    let sexp = node.to_sexp();
    assert_eq!(sexp, "(call shift ())");
}

#[test]
fn sexp_function_call_user_no_args() {
    let node =
        Node::new(NodeKind::FunctionCall { name: "my_func".to_string(), args: vec![] }, loc(0, 7));
    let sexp = node.to_sexp();
    assert_eq!(sexp, "(function_call_expression (function))");
}

#[test]
fn sexp_function_call_user_with_args() {
    let node = Node::new(
        NodeKind::FunctionCall { name: "my_func".to_string(), args: vec![num_node("1")] },
        loc(0, 10),
    );
    let sexp = node.to_sexp();
    assert!(sexp.contains("ambiguous_function_call_expression"), "got: {sexp}");
}

#[test]
fn sexp_variable_with_attributes() {
    let node = Node::new(
        NodeKind::VariableWithAttributes {
            variable: Box::new(var_node("$", "x")),
            attributes: vec!["lvalue".to_string()],
        },
        loc(0, 5),
    );
    let sexp = node.to_sexp();
    assert!(sexp.contains("attributes"), "got: {sexp}");
    assert!(sexp.contains("lvalue"), "got: {sexp}");
}

#[test]
fn sexp_assignment_operator() {
    let node = Node::new(
        NodeKind::Assignment {
            lhs: Box::new(var_node("$", "x")),
            rhs: Box::new(num_node("1")),
            op: "=".to_string(),
        },
        loc(0, 6),
    );
    let sexp = node.to_sexp();
    assert!(sexp.starts_with("(assignment_"), "got: {sexp}");
}

#[test]
fn sexp_catch_with_variable() {
    let node = Node::new(
        NodeKind::Try {
            body: Box::new(block_node(vec![])),
            catch_blocks: vec![(Some("$e".to_string()), Box::new(block_node(vec![])))],
            finally_block: None,
        },
        loc(0, 30),
    );
    let sexp = node.to_sexp();
    assert!(sexp.contains("(catch $e"), "catch with var, got: {sexp}");
}

#[test]
fn sexp_catch_without_variable() {
    let node = Node::new(
        NodeKind::Try {
            body: Box::new(block_node(vec![])),
            catch_blocks: vec![(None, Box::new(block_node(vec![])))],
            finally_block: None,
        },
        loc(0, 20),
    );
    let sexp = node.to_sexp();
    assert!(sexp.contains("(catch (block"), "catch without var, got: {sexp}");
}

#[test]
fn sexp_for_loop_with_continue() {
    let node = Node::new(
        NodeKind::For {
            init: Some(Box::new(num_node("0"))),
            condition: Some(Box::new(num_node("10"))),
            update: Some(Box::new(num_node("1"))),
            body: Box::new(block_node(vec![])),
            continue_block: Some(Box::new(block_node(vec![]))),
        },
        loc(0, 30),
    );
    let sexp = node.to_sexp();
    assert!(sexp.contains("(continue"), "got: {sexp}");
}

#[test]
fn sexp_foreach_with_continue() {
    let node = Node::new(
        NodeKind::Foreach {
            variable: Box::new(var_node("$", "i")),
            list: Box::new(Node::new(NodeKind::ArrayLiteral { elements: vec![] }, loc(0, 2))),
            body: Box::new(block_node(vec![])),
            continue_block: Some(Box::new(block_node(vec![]))),
        },
        loc(0, 30),
    );
    let sexp = node.to_sexp();
    assert!(sexp.starts_with("(foreach"), "got: {sexp}");
}

#[test]
fn sexp_data_section_with_body() {
    let node = Node::new(
        NodeKind::DataSection {
            marker: "__DATA__".to_string(),
            body: Some("line1\nline2".to_string()),
        },
        loc(0, 30),
    );
    let sexp = node.to_sexp();
    assert!(sexp.contains("data_section"), "got: {sexp}");
    assert!(sexp.contains("__DATA__"), "got: {sexp}");
}

#[test]
fn sexp_no_with_args_and_filter_risk() {
    let node = Node::new(
        NodeKind::No {
            module: "strict".to_string(),
            args: vec!["refs".to_string()],
            has_filter_risk: true,
        },
        loc(0, 20),
    );
    let sexp = node.to_sexp();
    assert!(sexp.contains("(no strict"), "got: {sexp}");
    assert!(sexp.contains("(risk:filter)"), "got: {sexp}");
}

#[test]
fn sexp_method_declaration_with_attributes() {
    let node = Node::new(
        NodeKind::Method {
            name: "greet".to_string(),
            signature: None,
            attributes: vec!["lvalue".to_string()],
            body: Box::new(block_node(vec![])),
        },
        loc(0, 25),
    );
    let sexp = node.to_sexp();
    assert!(sexp.contains("method_declaration_statement"), "got: {sexp}");
    assert!(sexp.contains("attrlist"), "got: {sexp}");
}

#[test]
fn sexp_anonymous_subroutine() {
    let node = Node::new(
        NodeKind::Subroutine {
            name: None,
            name_span: None,
            prototype: None,
            signature: None,
            attributes: vec![],
            body: Box::new(block_node(vec![num_node("1")])),
        },
        loc(0, 15),
    );
    let sexp = node.to_sexp();
    assert!(sexp.contains("anonymous_subroutine_expression"), "got: {sexp}");
}

#[test]
fn sexp_anonymous_subroutine_with_attributes() {
    let node = Node::new(
        NodeKind::Subroutine {
            name: None,
            name_span: None,
            prototype: None,
            signature: None,
            attributes: vec!["lvalue".to_string()],
            body: Box::new(block_node(vec![])),
        },
        loc(0, 15),
    );
    let sexp = node.to_sexp();
    assert!(sexp.contains("anonymous_subroutine_expression"), "got: {sexp}");
    assert!(sexp.contains("attrlist"), "got: {sexp}");
}

#[test]
fn sexp_named_subroutine_with_attributes() {
    let node = Node::new(
        NodeKind::Subroutine {
            name: Some("greet".to_string()),
            name_span: Some(loc(4, 9)),
            prototype: None,
            signature: None,
            attributes: vec!["lvalue".to_string()],
            body: Box::new(block_node(vec![])),
        },
        loc(0, 20),
    );
    let sexp = node.to_sexp();
    assert!(sexp.contains("(sub greet"), "got: {sexp}");
    assert!(sexp.contains(":lvalue"), "got: {sexp}");
}

#[test]
fn sexp_error_with_partial_and_escape() {
    let node = Node::new(
        NodeKind::Error {
            message: "unexpected \"token\"".to_string(),
            expected: vec![],
            found: None,
            partial: Some(Box::new(num_node("1"))),
        },
        loc(0, 10),
    );
    let sexp = node.to_sexp();
    assert!(sexp.contains("ERROR"), "got: {sexp}");
    assert!(sexp.contains("(number 1)"), "got: {sexp}");
}

// ===========================================================================
// 6. to_sexp_inner behavior
// ===========================================================================

#[test]
fn sexp_inner_unwraps_non_anon_expression_statement() {
    let es = Node::new(
        NodeKind::ExpressionStatement { expression: Box::new(num_node("42")) },
        loc(0, 2),
    );
    let sexp = es.to_sexp_inner();
    assert_eq!(sexp, "(number 42)");
}

#[test]
fn sexp_inner_keeps_anon_subroutine_wrapped() {
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
    let es = Node::new(NodeKind::ExpressionStatement { expression: Box::new(anon) }, loc(0, 10));
    let sexp = es.to_sexp_inner();
    assert!(sexp.contains("expression_statement"), "anon sub should stay wrapped, got: {sexp}");
}

#[test]
fn sexp_inner_non_expression_statement() {
    let num = num_node("7");
    let sexp = num.to_sexp_inner();
    assert_eq!(sexp, "(number 7)");
}

// ===========================================================================
// 7. children() and first_child() edge cases
// ===========================================================================

#[test]
fn children_of_assignment() {
    let node = Node::new(
        NodeKind::Assignment {
            lhs: Box::new(var_node("$", "x")),
            rhs: Box::new(num_node("1")),
            op: "=".to_string(),
        },
        loc(0, 6),
    );
    assert_eq!(node.children().len(), 2);
}

#[test]
fn children_of_ternary() {
    let node = Node::new(
        NodeKind::Ternary {
            condition: Box::new(num_node("1")),
            then_expr: Box::new(num_node("2")),
            else_expr: Box::new(num_node("3")),
        },
        loc(0, 10),
    );
    assert_eq!(node.children().len(), 3);
}

#[test]
fn first_child_of_error_with_partial() {
    let node = Node::new(
        NodeKind::Error {
            message: "err".to_string(),
            expected: vec![],
            found: None,
            partial: Some(Box::new(num_node("1"))),
        },
        loc(0, 5),
    );
    let first = node.first_child();
    assert!(first.is_some());
}

#[test]
fn first_child_of_error_without_partial() {
    let node = Node::new(
        NodeKind::Error {
            message: "err".to_string(),
            expected: vec![],
            found: None,
            partial: None,
        },
        loc(0, 5),
    );
    let first = node.first_child();
    assert!(first.is_none());
}

// ===========================================================================
// 8. ALL_KIND_NAMES / RECOVERY_KIND_NAMES properties
// ===========================================================================

#[test]
fn all_kind_names_is_sorted() {
    let names = NodeKind::ALL_KIND_NAMES;
    for window in names.windows(2) {
        assert!(
            window[0] < window[1],
            "ALL_KIND_NAMES not sorted: {:?} >= {:?}",
            window[0],
            window[1]
        );
    }
}

#[test]
fn recovery_kind_names_is_sorted() {
    let names = NodeKind::RECOVERY_KIND_NAMES;
    for window in names.windows(2) {
        assert!(
            window[0] < window[1],
            "RECOVERY_KIND_NAMES not sorted: {:?} >= {:?}",
            window[0],
            window[1]
        );
    }
}

#[test]
fn all_kind_names_has_expected_count() {
    // There are 67 NodeKind variants per the codebase
    assert!(
        NodeKind::ALL_KIND_NAMES.len() >= 60,
        "Expected at least 60 kind names, got {}",
        NodeKind::ALL_KIND_NAMES.len()
    );
}

#[test]
fn recovery_kind_names_count() {
    assert_eq!(NodeKind::RECOVERY_KIND_NAMES.len(), 6);
}

// ===========================================================================
// 9. v2 module additional tests
// ===========================================================================

#[test]
fn v2_node_clone_and_eq() {
    use perl_ast::v2::{Node as V2Node, NodeIdGenerator, NodeKind as V2Kind};
    use perl_position_tracking::{Position, Range};

    let mut id_gen = NodeIdGenerator::new();
    let range = Range::new(Position::new(0, 1, 1), Position::new(5, 1, 6));
    let node = V2Node::new(id_gen.next_id(), V2Kind::Number { value: "42".to_string() }, range);
    let cloned = node.clone();
    assert_eq!(node, cloned);
}

#[test]
fn v2_missing_kind_copy_and_debug() {
    use perl_ast::v2::MissingKind;

    let mk = MissingKind::Expression;
    let copied = mk;
    assert_eq!(mk, copied);
    let dbg = format!("{:?}", mk);
    assert!(dbg.contains("Expression"), "got: {dbg}");
}

#[test]
fn v2_node_id_generator_many_ids() {
    use perl_ast::v2::NodeIdGenerator;

    let mut id_gen = NodeIdGenerator::new();
    for i in 0..100 {
        assert_eq!(id_gen.next_id(), i);
    }
}

#[test]
fn v2_block_sexp() {
    use perl_ast::v2::{Node as V2Node, NodeIdGenerator, NodeKind as V2Kind};
    use perl_position_tracking::{Position, Range};

    let mut id_gen = NodeIdGenerator::new();
    let range = Range::new(Position::new(0, 1, 1), Position::new(10, 1, 11));
    let child = V2Node::new(id_gen.next_id(), V2Kind::Number { value: "1".to_string() }, range);
    let block = V2Kind::Block { statements: vec![child] };
    let sexp = block.to_sexp();
    assert!(sexp.starts_with("(block"), "got: {sexp}");
    assert!(sexp.contains("(number 1)"), "got: {sexp}");
}

#[test]
fn v2_missing_kind_all_variants_debug() {
    use perl_ast::v2::MissingKind;

    let variants = [
        MissingKind::Expression,
        MissingKind::Statement,
        MissingKind::Identifier,
        MissingKind::Block,
        MissingKind::ClosingDelimiter('}'),
        MissingKind::Semicolon,
        MissingKind::Condition,
        MissingKind::Argument,
        MissingKind::Operator,
    ];
    for v in &variants {
        let dbg = format!("{:?}", v);
        assert!(!dbg.is_empty(), "Debug output should not be empty");
    }
}

#[test]
fn v2_variable_declaration_sexp_fallthrough() {
    use perl_ast::v2::NodeKind as V2Kind;

    // Test the catch-all branch in v2 to_sexp for unsupported variants
    let kind = V2Kind::VariableDeclaration {
        declarator: "my".to_string(),
        variable: Box::new(perl_ast::v2::Node::new(
            0,
            V2Kind::Variable { sigil: "$".to_string(), name: "x".to_string() },
            perl_position_tracking::Range::new(
                perl_position_tracking::Position::new(0, 1, 1),
                perl_position_tracking::Position::new(4, 1, 5),
            ),
        )),
        attributes: vec![],
        initializer: None,
    };
    let sexp = kind.to_sexp();
    // The catch-all uses Debug format
    assert!(!sexp.is_empty(), "sexp should not be empty");
}

// ===========================================================================
// 10. SourceLocation re-export
// ===========================================================================

#[test]
fn source_location_reexported() {
    use perl_ast::SourceLocation;
    let sl = SourceLocation { start: 0, end: 10 };
    assert_eq!(sl.start, 0);
    assert_eq!(sl.end, 10);
}

#[test]
fn source_location_clone_and_copy() {
    use perl_ast::SourceLocation;
    let sl = SourceLocation { start: 5, end: 15 };
    let copied = sl;
    assert_eq!(sl, copied);
}
