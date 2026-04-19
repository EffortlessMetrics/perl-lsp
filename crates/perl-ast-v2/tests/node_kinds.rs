use perl_ast_v2::{MissingKind, Node, NodeIdGenerator, NodeKind};
use perl_position_tracking::{Position, Range};

fn zero_range() -> Range {
    Range::new(Position::new(0, 1, 1), Position::new(0, 1, 1))
}

fn make_node(id_gen: &mut NodeIdGenerator, kind: NodeKind) -> Node {
    Node::new(id_gen.next_id(), kind, zero_range())
}

// ---- NodeIdGenerator -------------------------------------------------------

#[test]
fn test_id_generator_sequential() {
    let mut id_gen = NodeIdGenerator::new();
    assert_eq!(id_gen.next_id(), 0);
    assert_eq!(id_gen.next_id(), 1);
    assert_eq!(id_gen.next_id(), 2);
}

#[test]
fn test_id_generator_default() {
    let mut id_gen = NodeIdGenerator::default();
    assert_eq!(id_gen.next_id(), 0);
}

// ---- Node construction & sexp ----------------------------------------------

#[test]
fn test_variable_sexp() {
    let mut id_gen = NodeIdGenerator::new();
    let node = make_node(
        &mut id_gen,
        NodeKind::Variable {
            sigil: "$".into(),
            name: "x".into(),
        },
    );
    assert_eq!(node.to_sexp(), "(variable $ x)");
}

#[test]
fn test_number_sexp() {
    let mut id_gen = NodeIdGenerator::new();
    let node = make_node(
        &mut id_gen,
        NodeKind::Number {
            value: "3.14".into(),
        },
    );
    assert_eq!(node.to_sexp(), "(number 3.14)");
}

#[test]
fn test_string_non_interpolated_sexp() {
    let mut id_gen = NodeIdGenerator::new();
    let node = make_node(
        &mut id_gen,
        NodeKind::String {
            value: "hello".into(),
            interpolated: false,
        },
    );
    assert_eq!(node.to_sexp(), r#"(string "hello")"#);
}

#[test]
fn test_string_interpolated_sexp() {
    let mut id_gen = NodeIdGenerator::new();
    let node = make_node(
        &mut id_gen,
        NodeKind::String {
            value: "hi".into(),
            interpolated: true,
        },
    );
    assert_eq!(node.to_sexp(), r#"(string_interpolated "hi")"#);
}

#[test]
fn test_error_ref_sexp() {
    let mut id_gen = NodeIdGenerator::new();
    let node = make_node(&mut id_gen, NodeKind::ErrorRef { diag_id: 7 });
    assert_eq!(node.to_sexp(), "(ERROR_REF #7)");
}

#[test]
fn test_missing_expression_sexp() {
    let mut id_gen = NodeIdGenerator::new();
    let node = make_node(&mut id_gen, NodeKind::MissingExpression);
    assert_eq!(node.to_sexp(), "(MISSING_EXPRESSION)");
}

#[test]
fn test_missing_statement_sexp() {
    let mut id_gen = NodeIdGenerator::new();
    let node = make_node(&mut id_gen, NodeKind::MissingStatement);
    assert_eq!(node.to_sexp(), "(MISSING_STATEMENT)");
}

#[test]
fn test_missing_identifier_sexp() {
    let mut id_gen = NodeIdGenerator::new();
    let node = make_node(&mut id_gen, NodeKind::MissingIdentifier);
    assert_eq!(node.to_sexp(), "(MISSING_IDENTIFIER)");
}

#[test]
fn test_missing_block_sexp() {
    let mut id_gen = NodeIdGenerator::new();
    let node = make_node(&mut id_gen, NodeKind::MissingBlock);
    assert_eq!(node.to_sexp(), "(MISSING_BLOCK)");
}

#[test]
fn test_missing_kind_variant_sexp() {
    let mut id_gen = NodeIdGenerator::new();
    let node = make_node(&mut id_gen, NodeKind::Missing(MissingKind::Semicolon));
    let sexp = node.to_sexp();
    assert!(sexp.contains("MISSING"), "expected MISSING in {sexp}");
    assert!(
        sexp.contains("Semicolon"),
        "expected Semicolon variant in {sexp}"
    );
}

#[test]
fn test_program_sexp_empty() {
    let mut id_gen = NodeIdGenerator::new();
    let node = make_node(&mut id_gen, NodeKind::Program { statements: vec![] });
    assert_eq!(node.to_sexp(), "(source_file )");
}

#[test]
fn test_program_with_child_sexp() {
    let mut id_gen = NodeIdGenerator::new();
    let child = make_node(&mut id_gen, NodeKind::Number { value: "1".into() });
    let node = make_node(
        &mut id_gen,
        NodeKind::Program {
            statements: vec![child],
        },
    );
    assert_eq!(node.to_sexp(), "(source_file (number 1))");
}

#[test]
fn test_block_sexp() {
    let mut id_gen = NodeIdGenerator::new();
    let inner = make_node(&mut id_gen, NodeKind::Number { value: "0".into() });
    let node = make_node(
        &mut id_gen,
        NodeKind::Block {
            statements: vec![inner],
        },
    );
    assert_eq!(node.to_sexp(), "(block (number 0))");
}

#[test]
fn test_binary_sexp() {
    let mut id_gen = NodeIdGenerator::new();
    let left = make_node(&mut id_gen, NodeKind::Number { value: "1".into() });
    let right = make_node(&mut id_gen, NodeKind::Number { value: "2".into() });
    let node = make_node(
        &mut id_gen,
        NodeKind::Binary {
            op: "+".into(),
            left: Box::new(left),
            right: Box::new(right),
        },
    );
    assert_eq!(node.to_sexp(), "(binary_+ (number 1) (number 2))");
}

#[test]
fn test_node_equality() {
    let mut id_gen = NodeIdGenerator::new();
    let r = zero_range();
    let a = Node::new(id_gen.next_id(), NodeKind::Number { value: "5".into() }, r);
    let b = Node::new(id_gen.next_id(), NodeKind::Number { value: "5".into() }, r);
    // Same kind/range but different ids — kinds are equal even if ids differ
    assert_eq!(a.kind, b.kind);
    assert_ne!(a.id, b.id);
    // Full node equality considers id — nodes with different ids are not equal
    assert_ne!(a, b);
}
