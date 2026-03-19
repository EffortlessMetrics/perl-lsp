use perl_parser_core::{
    Node as V1Node, NodeKind as V1NodeKind, SourceLocation, ast_v2::NodeKind as V2NodeKind,
};

#[test]
fn v1_node_creation() -> Result<(), Box<dyn std::error::Error>> {
    let node = V1Node::new(
        V1NodeKind::Number { value: "42".to_string() },
        SourceLocation { start: 0, end: 2 },
    );
    assert_eq!(node.location.start, 0);
    assert_eq!(node.location.end, 2);
    assert_eq!(node.kind.kind_name(), "Number");
    Ok(())
}

#[test]
fn v1_node_to_sexp() -> Result<(), Box<dyn std::error::Error>> {
    let node = V1Node::new(
        V1NodeKind::Number { value: "42".to_string() },
        SourceLocation { start: 0, end: 2 },
    );
    let sexp = node.to_sexp();
    assert!(!sexp.is_empty(), "S-expression should not be empty");
    Ok(())
}

#[test]
fn v1_program_node() -> Result<(), Box<dyn std::error::Error>> {
    let child = V1Node::new(
        V1NodeKind::Number { value: "1".to_string() },
        SourceLocation { start: 0, end: 1 },
    );
    let program = V1Node::new(
        V1NodeKind::Program { statements: vec![child] },
        SourceLocation { start: 0, end: 1 },
    );
    match &program.kind {
        V1NodeKind::Program { statements } => assert_eq!(statements.len(), 1),
        _ => return Err("expected Program".into()),
    }
    Ok(())
}

#[test]
fn v2_node_creation() -> Result<(), Box<dyn std::error::Error>> {
    let range = perl_position_tracking::Range::new(
        perl_position_tracking::Position::new(0, 1, 1),
        perl_position_tracking::Position::new(2, 1, 3),
    );
    let mut id_gen = perl_ast_v2::NodeIdGenerator::new();
    let node = perl_ast_v2::Node::new(
        id_gen.next_id(),
        V2NodeKind::Number { value: "99".to_string() },
        range,
    );
    assert_eq!(node.to_sexp(), node.kind.to_sexp());
    Ok(())
}

#[test]
fn v2_error_node() -> Result<(), Box<dyn std::error::Error>> {
    let range = perl_position_tracking::Range::new(
        perl_position_tracking::Position::new(0, 1, 1),
        perl_position_tracking::Position::new(0, 1, 1),
    );
    let mut id_gen = perl_ast_v2::NodeIdGenerator::new();
    let node = perl_ast_v2::Node::new(
        id_gen.next_id(),
        V2NodeKind::Error {
            message: "test".to_string(),
            expected: vec!["foo".to_string()],
            partial: None,
        },
        range,
    );
    match &node.kind {
        V2NodeKind::Error { message, expected, partial } => {
            assert_eq!(message, "test");
            assert_eq!(expected.len(), 1);
            assert!(partial.is_none());
        }
        _ => return Err("expected Error node".into()),
    }
    Ok(())
}
