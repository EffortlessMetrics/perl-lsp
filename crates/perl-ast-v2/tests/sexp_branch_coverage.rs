use perl_ast_v2::{MissingKind, Node, NodeIdGenerator, NodeKind};
use perl_position_tracking::{Position, Range};

fn range(start: usize, end: usize, start_column: u32, end_column: u32) -> Range {
    Range::new(Position::new(start, 1, start_column), Position::new(end, 1, end_column))
}

fn node(id_gen: &mut NodeIdGenerator, kind: NodeKind) -> Node {
    Node::new(id_gen.next_id(), kind, range(0, 0, 1, 1))
}

#[test]
fn nested_program_sexp_preserves_statement_order() -> Result<(), Box<dyn std::error::Error>> {
    let mut id_gen = NodeIdGenerator::new();
    let first = node(&mut id_gen, NodeKind::Number { value: "1".into() });
    let second = node(&mut id_gen, NodeKind::String { value: "two".into(), interpolated: false });
    let third = node(&mut id_gen, NodeKind::Variable { sigil: "$".into(), name: "three".into() });
    let program = node(&mut id_gen, NodeKind::Program { statements: vec![first, second, third] });

    assert_eq!(program.to_sexp(), r#"(source_file (number 1) (string "two") (variable $ three))"#);
    Ok(())
}

#[test]
fn binary_sexp_recurses_through_nested_operands() -> Result<(), Box<dyn std::error::Error>> {
    let mut id_gen = NodeIdGenerator::new();
    let left = node(&mut id_gen, NodeKind::Number { value: "1".into() });
    let inner_left = node(&mut id_gen, NodeKind::Number { value: "2".into() });
    let inner_right = node(&mut id_gen, NodeKind::Number { value: "3".into() });
    let right = node(
        &mut id_gen,
        NodeKind::Binary {
            op: "*".into(),
            left: Box::new(inner_left),
            right: Box::new(inner_right),
        },
    );
    let expr = node(
        &mut id_gen,
        NodeKind::Binary { op: "+".into(), left: Box::new(left), right: Box::new(right) },
    );

    assert_eq!(expr.to_sexp(), "(binary_+ (number 1) (binary_* (number 2) (number 3)))");
    Ok(())
}

#[test]
fn missing_kind_sexp_covers_all_specific_variants() -> Result<(), Box<dyn std::error::Error>> {
    let cases = [
        (MissingKind::Expression, "(MISSING Expression)"),
        (MissingKind::Statement, "(MISSING Statement)"),
        (MissingKind::Identifier, "(MISSING Identifier)"),
        (MissingKind::Block, "(MISSING Block)"),
        (MissingKind::ClosingDelimiter(')'), "(MISSING ClosingDelimiter(')'))"),
        (MissingKind::Semicolon, "(MISSING Semicolon)"),
        (MissingKind::Condition, "(MISSING Condition)"),
        (MissingKind::Argument, "(MISSING Argument)"),
        (MissingKind::Operator, "(MISSING Operator)"),
    ];

    for (kind, expected) in cases {
        assert_eq!(NodeKind::Missing(kind).to_sexp(), expected);
    }
    Ok(())
}

#[test]
fn error_sexp_uses_message_and_ignores_recovery_payload() -> Result<(), Box<dyn std::error::Error>>
{
    let mut id_gen = NodeIdGenerator::new();
    let partial = node(&mut id_gen, NodeKind::Identifier { name: "partial".into() });
    let error = NodeKind::Error {
        message: "Unexpected keyword".into(),
        expected: vec!["identifier".into(), "sub".into()],
        partial: Some(Box::new(partial)),
    };

    assert_eq!(error.to_sexp(), "(ERROR Unexpected keyword)");
    Ok(())
}

#[test]
fn node_new_preserves_id_kind_and_range() -> Result<(), Box<dyn std::error::Error>> {
    let source_range = range(3, 9, 4, 10);
    let node = Node::new(42, NodeKind::Identifier { name: "answer".into() }, source_range);

    assert_eq!(node.id, 42);
    assert_eq!(node.kind, NodeKind::Identifier { name: "answer".into() });
    assert_eq!(node.range, source_range);
    Ok(())
}

#[test]
fn id_generator_sequences_are_independent() -> Result<(), Box<dyn std::error::Error>> {
    let mut first = NodeIdGenerator::new();
    let mut second = NodeIdGenerator::new();

    assert_eq!(first.next_id(), 0);
    assert_eq!(first.next_id(), 1);
    assert_eq!(second.next_id(), 0);
    assert_eq!(first.next_id(), 2);
    assert_eq!(second.next_id(), 1);
    Ok(())
}
