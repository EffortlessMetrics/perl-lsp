use perl_ast::NodeKind;
use perl_parser_core::Parser;
use perl_tdd_support::must;

#[test]
fn class_field_declaration_parses_as_variable_declaration() {
    let code = r#"
class Example {
    field $name :param;
    field $count = 0;
}
"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let NodeKind::Program { statements } = &ast.kind else {
        unreachable!("expected program node, got {}", ast.kind.kind_name());
    };

    let class = statements
        .iter()
        .find(|statement| matches!(statement.kind, NodeKind::Class { .. }));
    assert!(
        class.is_some(),
        "expected class declaration in {}",
        ast.to_sexp()
    );
    let class = class.unwrap_or_else(|| unreachable!());

    let NodeKind::Class { body, .. } = &class.kind else {
        unreachable!("expected class node, got {}", class.kind.kind_name());
    };

    let NodeKind::Block { statements } = &body.kind else {
        unreachable!("expected class body block, got {}", body.kind.kind_name());
    };

    let field_nodes: Vec<_> = statements
        .iter()
        .filter_map(|statement| match &statement.kind {
            NodeKind::VariableDeclaration {
                declarator,
                attributes,
                initializer,
                ..
            } if declarator == "field" => Some((attributes, initializer)),
            _ => None,
        })
        .collect();

    assert_eq!(
        field_nodes.len(),
        2,
        "expected two field declarations in {}",
        ast.to_sexp()
    );
    assert_eq!(field_nodes[0].0, &vec!["param".to_string()]);
    assert!(
        field_nodes[0].1.is_none(),
        "field :param should not synthesize an initializer"
    );
    assert!(
        field_nodes[1].1.is_some(),
        "initialized field should keep its initializer"
    );
    assert!(
        parser.errors().is_empty(),
        "expected clean parse for field declarations, got {:?}",
        parser.errors()
    );
}
