use perl_parser::{
    Parser,
    ast::{Node, NodeKind},
};

fn parse_without_errors(source: &str) -> Result<Node, String> {
    let mut parser = Parser::new(source);
    let ast = parser.parse().map_err(|err| err.to_string())?;
    if parser.errors().is_empty() {
        Ok(ast)
    } else {
        Err(format!("unexpected parser diagnostics for {source:?}: {:?}", parser.errors()))
    }
}

fn program_statements(ast: &Node) -> Result<&[Node], String> {
    match &ast.kind {
        NodeKind::Program { statements } => Ok(statements),
        other => Err(format!("expected Program node, got {other:?}")),
    }
}

fn block_statements(node: &Node) -> Result<&[Node], String> {
    match &node.kind {
        NodeKind::Block { statements } => Ok(statements),
        other => Err(format!("expected Block node, got {other:?}")),
    }
}

fn variable_name(node: &Node) -> Result<String, String> {
    match &node.kind {
        NodeKind::Variable { sigil, name } => Ok(format!("{sigil}{name}")),
        other => Err(format!("expected Variable node, got {other:?}")),
    }
}

#[test]
fn try_catch_finally_preserves_handler_shapes() -> Result<(), String> {
    let ast = parse_without_errors(
        r#"try { risky($value); } catch ($err) { warn $err; } catch Other::Failure with { recover(); } finally { cleanup(); }"#,
    )?;
    let statements = program_statements(&ast)?;
    assert_eq!(statements.len(), 1);

    match &statements[0].kind {
        NodeKind::Try { body, catch_blocks, finally_block } => {
            assert_eq!(block_statements(body)?.len(), 1);
            assert_eq!(catch_blocks.len(), 2);
            assert_eq!(catch_blocks[0].0.as_deref(), Some("$err"));
            assert_eq!(block_statements(&catch_blocks[0].1)?.len(), 1);
            assert_eq!(catch_blocks[1].0, None);
            assert_eq!(block_statements(&catch_blocks[1].1)?.len(), 1);

            let finally = finally_block.as_ref().ok_or("missing finally block")?;
            assert_eq!(block_statements(finally)?.len(), 1);
        }
        other => return Err(format!("expected Try node, got {other:?}")),
    }

    Ok(())
}

#[test]
fn given_when_default_preserves_switch_body_order() -> Result<(), String> {
    let ast = parse_without_errors(
        r#"given ($value) { when (1) { say 'one'; } when ($fallback) { say 'fallback'; } default { say 'other'; } }"#,
    )?;
    let statements = program_statements(&ast)?;
    assert_eq!(statements.len(), 1);

    match &statements[0].kind {
        NodeKind::Given { expr, body } => {
            assert_eq!(variable_name(expr)?, "$value");
            let switch_arms = block_statements(body)?;
            assert_eq!(switch_arms.len(), 3);
            assert!(matches!(switch_arms[0].kind, NodeKind::When { .. }));
            assert!(matches!(switch_arms[1].kind, NodeKind::When { .. }));
            assert!(matches!(switch_arms[2].kind, NodeKind::Default { .. }));
        }
        other => return Err(format!("expected Given node, got {other:?}")),
    }

    Ok(())
}

#[test]
fn package_block_keeps_precise_name_span_and_inner_statements() -> Result<(), String> {
    let source = "package Local::Thing { our $VERSION = '1.0'; sub build { return 1; } }";
    let ast = parse_without_errors(source)?;
    let statements = program_statements(&ast)?;
    assert_eq!(statements.len(), 1);

    match &statements[0].kind {
        NodeKind::Package { name, name_span, block } => {
            assert_eq!(name, "Local::Thing");
            assert_eq!(&source[name_span.start..name_span.end], "Local::Thing");

            let package_body = block.as_ref().ok_or("missing package block")?;
            let body_statements = block_statements(package_body)?;
            assert_eq!(body_statements.len(), 2);
            assert!(matches!(body_statements[0].kind, NodeKind::VariableDeclaration { .. }));
            assert!(matches!(body_statements[1].kind, NodeKind::Subroutine { .. }));
        }
        other => return Err(format!("expected Package node, got {other:?}")),
    }

    Ok(())
}

#[test]
fn class_with_parent_and_method_signature_has_expected_ast_shape() -> Result<(), String> {
    let ast = parse_without_errors(
        "class Local::Widget :isa(Local::Base) { method render ($self, $ctx = undef) { return $ctx; } }",
    )?;
    let statements = program_statements(&ast)?;
    assert_eq!(statements.len(), 1);

    match &statements[0].kind {
        NodeKind::Class { name, parents, body } => {
            assert_eq!(name, "Local::Widget");
            assert_eq!(parents, &["Local::Base".to_string()]);

            let body_statements = block_statements(body)?;
            assert_eq!(body_statements.len(), 1);
            match &body_statements[0].kind {
                NodeKind::Method { name, signature, attributes, body } => {
                    assert_eq!(name, "render");
                    assert!(attributes.is_empty());
                    assert!(matches!(body.kind, NodeKind::Block { .. }));

                    let signature = signature.as_ref().ok_or("missing method signature")?;
                    match &signature.kind {
                        NodeKind::Signature { parameters } => {
                            assert_eq!(parameters.len(), 2);
                            assert!(matches!(
                                parameters[0].kind,
                                NodeKind::MandatoryParameter { .. }
                            ));
                            assert!(matches!(
                                parameters[1].kind,
                                NodeKind::OptionalParameter { .. }
                            ));
                        }
                        other => return Err(format!("expected Signature node, got {other:?}")),
                    }
                }
                other => return Err(format!("expected Method node, got {other:?}")),
            }
        }
        other => return Err(format!("expected Class node, got {other:?}")),
    }

    Ok(())
}

#[test]
fn phase_defer_and_data_section_cover_compile_time_and_tail_constructs() -> Result<(), String> {
    let source = "BEGIN { $seen = 1; } defer { cleanup(); }\n__DATA__\nfirst\nsecond\n";
    let ast = parse_without_errors(source)?;
    let statements = program_statements(&ast)?;

    let phase_node = statements
        .iter()
        .find(|statement| matches!(statement.kind, NodeKind::PhaseBlock { .. }))
        .ok_or("missing phase block")?;
    match &phase_node.kind {
        NodeKind::PhaseBlock { phase, phase_span, block } => {
            assert_eq!(phase, "BEGIN");
            let span = phase_span.ok_or("missing phase span")?;
            assert_eq!(&source[span.start..span.end], "BEGIN");
            assert_eq!(block_statements(block)?.len(), 1);
        }
        other => return Err(format!("expected PhaseBlock node, got {other:?}")),
    }

    let defer_node = statements
        .iter()
        .find(|statement| matches!(statement.kind, NodeKind::Defer { .. }))
        .ok_or("missing defer block")?;
    match &defer_node.kind {
        NodeKind::Defer { block } => assert_eq!(block_statements(block)?.len(), 1),
        other => return Err(format!("expected Defer node, got {other:?}")),
    }

    let data_node = statements
        .iter()
        .find(|statement| matches!(statement.kind, NodeKind::DataSection { .. }))
        .ok_or("missing data section")?;
    match &data_node.kind {
        NodeKind::DataSection { marker, body } => {
            assert_eq!(marker, "__DATA__");
            let body = body.as_ref().ok_or("missing data body")?;
            assert!(body.contains("first"));
            assert!(body.contains("second"));
        }
        other => return Err(format!("expected DataSection node, got {other:?}")),
    }

    Ok(())
}
