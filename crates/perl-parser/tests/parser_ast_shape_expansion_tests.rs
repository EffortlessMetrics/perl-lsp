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

fn variable_initializer<'a>(node: &'a Node, expected_declarator: &str) -> Result<&'a Node, String> {
    match &node.kind {
        NodeKind::VariableDeclaration { declarator, initializer, .. } => {
            assert_eq!(declarator, expected_declarator);
            initializer.as_deref().ok_or("missing variable initializer".to_string())
        }
        other => Err(format!("expected VariableDeclaration node, got {other:?}")),
    }
}

fn collect_loop_controls<'a>(node: &'a Node, controls: &mut Vec<&'a Node>) {
    if matches!(node.kind, NodeKind::LoopControl { .. }) {
        controls.push(node);
    }

    match &node.kind {
        NodeKind::Program { statements } | NodeKind::Block { statements } => {
            for statement in statements {
                collect_loop_controls(statement, controls);
            }
        }
        NodeKind::ExpressionStatement { expression }
        | NodeKind::Unary { operand: expression, .. }
        | NodeKind::Goto { target: expression }
        | NodeKind::Eval { block: expression }
        | NodeKind::Do { block: expression }
        | NodeKind::Defer { block: expression } => collect_loop_controls(expression, controls),
        NodeKind::VariableDeclaration { variable, initializer, .. } => {
            collect_loop_controls(variable, controls);
            if let Some(initializer) = initializer {
                collect_loop_controls(initializer, controls);
            }
        }
        NodeKind::VariableListDeclaration { variables, initializer, .. } => {
            for variable in variables {
                collect_loop_controls(variable, controls);
            }
            if let Some(initializer) = initializer {
                collect_loop_controls(initializer, controls);
            }
        }
        NodeKind::VariableWithAttributes { variable, .. } => {
            collect_loop_controls(variable, controls)
        }
        NodeKind::Assignment { lhs, rhs, .. } | NodeKind::Binary { left: lhs, right: rhs, .. } => {
            collect_loop_controls(lhs, controls);
            collect_loop_controls(rhs, controls);
        }
        NodeKind::Ternary { condition, then_expr, else_expr } => {
            collect_loop_controls(condition, controls);
            collect_loop_controls(then_expr, controls);
            collect_loop_controls(else_expr, controls);
        }
        NodeKind::ArrayLiteral { elements } => {
            for element in elements {
                collect_loop_controls(element, controls);
            }
        }
        NodeKind::HashLiteral { pairs } => {
            for (key, value) in pairs {
                collect_loop_controls(key, controls);
                collect_loop_controls(value, controls);
            }
        }
        NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
            collect_loop_controls(condition, controls);
            collect_loop_controls(then_branch, controls);
            for (condition, branch) in elsif_branches {
                collect_loop_controls(condition, controls);
                collect_loop_controls(branch, controls);
            }
            if let Some(else_branch) = else_branch {
                collect_loop_controls(else_branch, controls);
            }
        }
        NodeKind::LabeledStatement { statement, .. } => collect_loop_controls(statement, controls),
        NodeKind::While { condition, body, continue_block } => {
            collect_loop_controls(condition, controls);
            collect_loop_controls(body, controls);
            if let Some(continue_block) = continue_block {
                collect_loop_controls(continue_block, controls);
            }
        }
        NodeKind::For { init, condition, update, body, continue_block } => {
            for node in
                [init.as_deref(), condition.as_deref(), update.as_deref()].into_iter().flatten()
            {
                collect_loop_controls(node, controls);
            }
            collect_loop_controls(body, controls);
            if let Some(continue_block) = continue_block {
                collect_loop_controls(continue_block, controls);
            }
        }
        NodeKind::Foreach { variable, list, body, continue_block } => {
            collect_loop_controls(variable, controls);
            collect_loop_controls(list, controls);
            collect_loop_controls(body, controls);
            if let Some(continue_block) = continue_block {
                collect_loop_controls(continue_block, controls);
            }
        }
        NodeKind::Given { expr, body } => {
            collect_loop_controls(expr, controls);
            collect_loop_controls(body, controls);
        }
        NodeKind::When { condition, body } => {
            collect_loop_controls(condition, controls);
            collect_loop_controls(body, controls);
        }
        NodeKind::Default { body }
        | NodeKind::Subroutine { body, .. }
        | NodeKind::Method { body, .. }
        | NodeKind::Class { body, .. }
        | NodeKind::PhaseBlock { block: body, .. } => collect_loop_controls(body, controls),
        NodeKind::Signature { parameters } => {
            for parameter in parameters {
                collect_loop_controls(parameter, controls);
            }
        }
        NodeKind::MandatoryParameter { variable }
        | NodeKind::SlurpyParameter { variable }
        | NodeKind::NamedParameter { variable } => collect_loop_controls(variable, controls),
        NodeKind::OptionalParameter { variable, default_value } => {
            collect_loop_controls(variable, controls);
            collect_loop_controls(default_value, controls);
        }
        NodeKind::Return { value: Some(value) } => collect_loop_controls(value, controls),
        NodeKind::MethodCall { object, args, .. } => {
            collect_loop_controls(object, controls);
            for arg in args {
                collect_loop_controls(arg, controls);
            }
        }
        NodeKind::FunctionCall { args, .. } => {
            for arg in args {
                collect_loop_controls(arg, controls);
            }
        }
        NodeKind::IndirectCall { object, args, .. } => {
            collect_loop_controls(object, controls);
            for arg in args {
                collect_loop_controls(arg, controls);
            }
        }
        NodeKind::Tie { variable, package, args } => {
            collect_loop_controls(variable, controls);
            collect_loop_controls(package, controls);
            for arg in args {
                collect_loop_controls(arg, controls);
            }
        }
        NodeKind::Untie { variable } => collect_loop_controls(variable, controls),
        NodeKind::Package { block: Some(block), .. } => collect_loop_controls(block, controls),
        NodeKind::StatementModifier { statement, condition, .. } => {
            collect_loop_controls(statement, controls);
            collect_loop_controls(condition, controls);
        }
        NodeKind::Match { expr, .. }
        | NodeKind::Substitution { expr, .. }
        | NodeKind::Transliteration { expr, .. } => collect_loop_controls(expr, controls),
        _ => {}
    }
}

#[test]
fn do_and_eval_initializers_preserve_distinct_ast_shapes() -> Result<(), String> {
    let ast = parse_without_errors(
        r#"my $value = do { my $tmp = compute(); $tmp + 1 };
my $ok = eval { risky(); 1; };"#,
    )?;
    let statements = program_statements(&ast)?;
    assert_eq!(statements.len(), 2);

    match &variable_initializer(&statements[0], "my")?.kind {
        NodeKind::Do { block } => assert_eq!(block_statements(block)?.len(), 2),
        other => return Err(format!("expected Do initializer, got {other:?}")),
    }

    match &variable_initializer(&statements[1], "my")?.kind {
        NodeKind::Eval { block } => assert_eq!(block_statements(block)?.len(), 2),
        other => return Err(format!("expected Eval initializer, got {other:?}")),
    }

    Ok(())
}

#[test]
fn while_and_foreach_continue_blocks_stay_attached_to_loops() -> Result<(), String> {
    let ast = parse_without_errors(
        r#"while (my $line = <$fh>) { process($line); } continue { $line_count++; }
for my $item (@items) { process_item($item); } continue { $seen{$item}++; }"#,
    )?;
    let statements = program_statements(&ast)?;
    assert_eq!(statements.len(), 2);

    match &statements[0].kind {
        NodeKind::While { continue_block, body, .. } => {
            assert_eq!(block_statements(body)?.len(), 1);
            let continue_block = continue_block.as_ref().ok_or("missing while continue block")?;
            assert_eq!(block_statements(continue_block)?.len(), 1);
        }
        other => return Err(format!("expected While node, got {other:?}")),
    }

    match &statements[1].kind {
        NodeKind::Foreach { continue_block, body, .. } => {
            assert_eq!(block_statements(body)?.len(), 1);
            let continue_block = continue_block.as_ref().ok_or("missing foreach continue block")?;
            assert_eq!(block_statements(continue_block)?.len(), 1);
        }
        other => return Err(format!("expected Foreach node, got {other:?}")),
    }

    Ok(())
}

#[test]
fn labeled_loop_control_labels_survive_statement_modifiers() -> Result<(), String> {
    let ast = parse_without_errors(
        r#"OUTER: while ($running) {
    next OUTER if should_skip();
    last OUTER if should_stop();
    redo OUTER if should_retry();
}"#,
    )?;
    let statements = program_statements(&ast)?;
    assert_eq!(statements.len(), 1);

    match &statements[0].kind {
        NodeKind::LabeledStatement { label, statement } => {
            assert_eq!(label, "OUTER");
            assert!(matches!(statement.kind, NodeKind::While { .. }));
        }
        other => return Err(format!("expected LabeledStatement node, got {other:?}")),
    }

    let mut controls = Vec::new();
    collect_loop_controls(&statements[0], &mut controls);
    assert_eq!(controls.len(), 3);

    let mut ops = Vec::new();
    let mut labels = Vec::new();
    for control in controls {
        match &control.kind {
            NodeKind::LoopControl { op, label } => {
                ops.push(op.as_str());
                labels.push(label.as_deref());
            }
            other => return Err(format!("expected LoopControl node, got {other:?}")),
        }
    }

    assert_eq!(ops, vec!["next", "last", "redo"]);
    assert_eq!(labels, vec![Some("OUTER"), Some("OUTER"), Some("OUTER")]);
    Ok(())
}

#[test]
fn use_and_no_pragmas_preserve_import_args_and_filter_risk() -> Result<(), String> {
    let ast = parse_without_errors(
        "use Filter::Simple;\nuse List::Util qw(first max);\nno warnings 'experimental';",
    )?;
    let statements = program_statements(&ast)?;
    assert_eq!(statements.len(), 3);

    match &statements[0].kind {
        NodeKind::Use { module, args, has_filter_risk } => {
            assert_eq!(module, "Filter::Simple");
            assert!(args.is_empty());
            assert!(*has_filter_risk);
        }
        other => return Err(format!("expected filter Use node, got {other:?}")),
    }

    match &statements[1].kind {
        NodeKind::Use { module, args, has_filter_risk } => {
            assert_eq!(module, "List::Util");
            assert_eq!(args, &["qw(first max)".to_string()]);
            assert!(!has_filter_risk);
        }
        other => return Err(format!("expected import Use node, got {other:?}")),
    }

    match &statements[2].kind {
        NodeKind::No { module, args, has_filter_risk } => {
            assert_eq!(module, "warnings");
            assert_eq!(args, &["'experimental'".to_string()]);
            assert!(!has_filter_risk);
        }
        other => return Err(format!("expected No node, got {other:?}")),
    }

    Ok(())
}
