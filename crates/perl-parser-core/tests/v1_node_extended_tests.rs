use perl_parser_core::{Node as V1Node, NodeKind as V1NodeKind, SourceLocation};

#[test]
fn node_with_children() -> Result<(), Box<dyn std::error::Error>> {
    let child =
        V1Node::new(V1NodeKind::Number { value: "42".to_string() }, SourceLocation::new(0, 2));
    let program = V1Node::new(
        V1NodeKind::Program { statements: vec![child.clone()] },
        SourceLocation::new(0, 2),
    );
    if let V1NodeKind::Program { statements } = &program.kind {
        assert_eq!(statements.len(), 1);
    } else {
        return Err("expected Program".into());
    }
    Ok(())
}

#[test]
fn variable_declaration_node() -> Result<(), Box<dyn std::error::Error>> {
    let var = V1Node::new(
        V1NodeKind::Variable { sigil: "$".to_string(), name: "x".to_string() },
        SourceLocation::new(3, 5),
    );
    let decl = V1Node::new(
        V1NodeKind::VariableDeclaration {
            declarator: "my".to_string(),
            variable: Box::new(var),
            attributes: vec![],
            initializer: None,
        },
        SourceLocation::new(0, 5),
    );
    if let V1NodeKind::VariableDeclaration { declarator, attributes, initializer, .. } = &decl.kind
    {
        assert_eq!(declarator, "my");
        assert!(attributes.is_empty());
        assert!(initializer.is_none());
    } else {
        return Err("expected VariableDeclaration".into());
    }
    Ok(())
}

#[test]
fn error_node_fields() -> Result<(), Box<dyn std::error::Error>> {
    let node = V1Node::new(
        V1NodeKind::Error {
            message: "bad stuff".to_string(),
            expected: vec![],
            found: None,
            partial: None,
        },
        SourceLocation::new(10, 15),
    );
    if let V1NodeKind::Error { message, expected, found, partial } = &node.kind {
        assert_eq!(message, "bad stuff");
        assert!(expected.is_empty());
        assert!(found.is_none());
        assert!(partial.is_none());
    } else {
        return Err("expected Error".into());
    }
    Ok(())
}

#[test]
fn to_sexp_empty_program() -> Result<(), Box<dyn std::error::Error>> {
    let node = V1Node::new(V1NodeKind::Program { statements: vec![] }, SourceLocation::new(0, 0));
    let sexp = node.to_sexp();
    assert!(sexp.contains("source_file"), "sexp should contain source_file: {}", sexp);
    Ok(())
}

#[test]
fn to_sexp_with_number() -> Result<(), Box<dyn std::error::Error>> {
    let num =
        V1Node::new(V1NodeKind::Number { value: "99".to_string() }, SourceLocation::new(0, 2));
    let prog =
        V1Node::new(V1NodeKind::Program { statements: vec![num] }, SourceLocation::new(0, 2));
    let sexp = prog.to_sexp();
    assert!(sexp.contains("number"), "sexp should contain number: {}", sexp);
    Ok(())
}

#[test]
fn node_debug_format() -> Result<(), Box<dyn std::error::Error>> {
    let node = V1Node::new(V1NodeKind::MissingExpression, SourceLocation::new(0, 0));
    let dbg = format!("{:?}", node);
    assert!(dbg.contains("MissingExpression"));
    Ok(())
}

#[test]
fn node_clone_equals() -> Result<(), Box<dyn std::error::Error>> {
    let node =
        V1Node::new(V1NodeKind::Number { value: "1".to_string() }, SourceLocation::new(0, 1));
    let cloned = node.clone();
    assert_eq!(node, cloned);
    Ok(())
}

#[test]
fn block_node() -> Result<(), Box<dyn std::error::Error>> {
    let stmt =
        V1Node::new(V1NodeKind::Number { value: "1".to_string() }, SourceLocation::new(1, 2));
    let block =
        V1Node::new(V1NodeKind::Block { statements: vec![stmt] }, SourceLocation::new(0, 3));
    if let V1NodeKind::Block { statements } = &block.kind {
        assert_eq!(statements.len(), 1);
    } else {
        return Err("expected Block".into());
    }
    Ok(())
}
