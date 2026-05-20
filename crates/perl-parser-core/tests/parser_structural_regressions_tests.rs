use perl_parser_core::{Node, NodeKind, Parser};
use perl_tdd_support::must;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn parse_clean(source: &str) -> Result<Node, String> {
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    if parser.errors().is_empty() {
        Ok(ast)
    } else {
        Err(format!("unexpected parser errors: {:?}", parser.errors()))
    }
}

fn statements(ast: &Node) -> Result<&[Node], String> {
    match &ast.kind {
        NodeKind::Program { statements } => Ok(statements),
        other => Err(format!("expected Program, got {other:?}")),
    }
}

fn expression_statement(node: &Node) -> Result<&Node, String> {
    match &node.kind {
        NodeKind::ExpressionStatement { expression } => Ok(expression),
        other => Err(format!("expected ExpressionStatement, got {other:?}")),
    }
}

#[test]
fn data_section_captures_body_and_stops_parsing() -> TestResult {
    let ast = parse_clean("my $x = 1;\n__DATA__\nline one\nline two\n")?;
    let statements = statements(&ast)?;
    assert_eq!(statements.len(), 2, "data body should not be parsed as Perl statements");

    match &statements[1].kind {
        NodeKind::DataSection { marker, body } => {
            assert_eq!(marker, "__DATA__");
            assert_eq!(body.as_deref(), Some("line one\nline two\n"));
        }
        other => return Err(format!("expected DataSection, got {other:?}").into()),
    }

    Ok(())
}

#[test]
fn end_marker_without_body_is_data_section() -> TestResult {
    let ast = parse_clean("1;\n__END__")?;
    let statements = statements(&ast)?;
    assert_eq!(statements.len(), 2);

    match &statements[1].kind {
        NodeKind::DataSection { marker, body } => {
            assert_eq!(marker, "__END__");
            assert!(body.is_none(), "bare __END__ should not invent a body");
        }
        other => return Err(format!("expected DataSection, got {other:?}").into()),
    }

    Ok(())
}

#[test]
fn phase_block_records_phase_span_and_body() -> TestResult {
    let ast = parse_clean("BEGIN { my $x = 1; }")?;
    let statements = statements(&ast)?;
    assert_eq!(statements.len(), 1);

    match &statements[0].kind {
        NodeKind::PhaseBlock { phase, phase_span, block } => {
            assert_eq!(phase, "BEGIN");
            let Some(span) = phase_span else {
                return Err("expected phase span for BEGIN block".into());
            };
            assert_eq!(span.start, 0);
            assert_eq!(span.end, 5);
            match &block.kind {
                NodeKind::Block { statements } => {
                    assert_eq!(statements.len(), 1, "phase block body should contain declaration");
                }
                other => return Err(format!("expected phase Block, got {other:?}").into()),
            }
        }
        other => return Err(format!("expected PhaseBlock, got {other:?}").into()),
    }

    Ok(())
}

#[test]
fn phase_keyword_call_is_not_phase_block() -> TestResult {
    let ast = parse_clean("CHECK();")?;
    let statements = statements(&ast)?;
    assert_eq!(statements.len(), 1);
    let expression = expression_statement(&statements[0])?;

    match &expression.kind {
        NodeKind::FunctionCall { name, args } => {
            assert_eq!(name, "CHECK");
            assert!(args.is_empty());
        }
        other => return Err(format!("expected FunctionCall, got {other:?}").into()),
    }

    Ok(())
}

#[test]
fn phase_keyword_label_is_not_phase_block() -> TestResult {
    let ast = parse_clean("END: while (1) { last END; }")?;
    let statements = statements(&ast)?;
    assert_eq!(statements.len(), 1);

    match &statements[0].kind {
        NodeKind::LabeledStatement { label, statement } => {
            assert_eq!(label, "END");
            assert!(matches!(statement.kind, NodeKind::While { .. }));
        }
        other => return Err(format!("expected LabeledStatement, got {other:?}").into()),
    }

    Ok(())
}
