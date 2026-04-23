use perl_parser_core::{
    NodeKind as V1NodeKind,
    ParseOutput,
    // Parser
    Parser,
};
use perl_tdd_support::must;

#[test]
fn parse_simple_assignment() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("my $var = 42;");
    let ast = must(parser.parse());

    match &ast.kind {
        V1NodeKind::Program { statements } => {
            assert!(!statements.is_empty(), "should parse at least one statement");
        }
        other => return Err(format!("expected Program, got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn parse_empty_input() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("");
    let ast = must(parser.parse());

    match &ast.kind {
        V1NodeKind::Program { statements } => {
            assert!(statements.is_empty(), "empty source should yield no statements");
        }
        other => return Err(format!("expected Program, got {:?}", other).into()),
    }
    assert!(parser.errors().is_empty());
    Ok(())
}

#[test]
fn parse_with_recovery_returns_output() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("my $x = ;");
    let output: ParseOutput = parser.parse_with_recovery();

    match &output.ast.kind {
        V1NodeKind::Program { .. } => { /* ok */ }
        other => return Err(format!("expected Program, got {:?}", other).into()),
    }
    // The output should contain diagnostics for the syntax error
    // or recovery nodes within the AST
    Ok(())
}

#[test]
fn parse_with_recovery_empty() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("");
    let output = parser.parse_with_recovery();

    match &output.ast.kind {
        V1NodeKind::Program { statements } => {
            assert!(statements.is_empty());
        }
        other => return Err(format!("expected Program, got {:?}", other).into()),
    }
    assert!(output.diagnostics.is_empty());
    Ok(())
}

#[test]
fn parser_errors_initially_empty() -> Result<(), Box<dyn std::error::Error>> {
    let parser = Parser::new("my $x = 1;");
    assert!(parser.errors().is_empty(), "should have no errors before parsing");
    Ok(())
}

#[test]
fn new_with_recovery_config_creates_parser() -> Result<(), Box<dyn std::error::Error>> {
    let parser = Parser::new_with_recovery_config("my $x = 1;", ());
    assert!(parser.errors().is_empty());
    Ok(())
}

#[test]
fn parse_subroutine_declaration() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("sub hello { print 'hi'; }");
    let ast = must(parser.parse());

    match &ast.kind {
        V1NodeKind::Program { statements } => {
            assert!(!statements.is_empty(), "should parse subroutine");
        }
        other => return Err(format!("expected Program, got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn parse_namespaced_class_declaration() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("class My::App::Service { method run { 1; } }");
    let ast = must(parser.parse());

    match &ast.kind {
        V1NodeKind::Program { statements } => {
            let Some(class_node) = statements.first() else {
                return Err("expected a class statement".into());
            };

            match &class_node.kind {
                V1NodeKind::Class { name, .. } => {
                    assert_eq!(name, "My::App::Service");
                }
                other => return Err(format!("expected Class node, got {:?}", other).into()),
            }
        }
        other => return Err(format!("expected Program, got {:?}", other).into()),
    }

    assert!(parser.errors().is_empty(), "unexpected parser errors: {:?}", parser.errors());
    Ok(())
}

#[test]
fn reject_class_declaration_with_trailing_separator() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("class My::App:: { method run { 1; } }");
    let ast = parser.parse()?;
    let rendered = format!("{ast:?}");
    assert!(
        rendered.contains("identifier after ::"),
        "expected a missing identifier error, got: {rendered}"
    );
    assert!(
        !rendered.contains("Class { name: \"My::App::\""),
        "class declaration should not be accepted with a trailing ::: {rendered}"
    );
    Ok(())
}

#[test]
fn parse_package_declaration_with_trailing_separator() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("package My::App::;");
    let ast = must(parser.parse());

    match &ast.kind {
        V1NodeKind::Program { statements } => {
            let Some(package_node) = statements.first() else {
                return Err("expected a package statement".into());
            };

            match &package_node.kind {
                V1NodeKind::Package { name, block, .. } => {
                    assert_eq!(name, "My::App::");
                    assert!(block.is_none(), "package statement form should not have a block");
                }
                other => return Err(format!("expected Package node, got {:?}", other).into()),
            }
        }
        other => return Err(format!("expected Program, got {:?}", other).into()),
    }

    assert!(parser.errors().is_empty(), "unexpected parser errors: {:?}", parser.errors());
    Ok(())
}

#[test]
fn parse_package_declaration_with_trailing_separator_and_block() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("package My::App:: { my $x = 1; }");
    let ast = must(parser.parse());

    match &ast.kind {
        V1NodeKind::Program { statements } => {
            let Some(package_node) = statements.first() else {
                return Err("expected a package statement".into());
            };

            match &package_node.kind {
                V1NodeKind::Package { name, block, .. } => {
                    assert_eq!(name, "My::App::");
                    assert!(block.is_some(), "package block form should include a block node");
                }
                other => return Err(format!("expected Package node, got {:?}", other).into()),
            }
        }
        other => return Err(format!("expected Program, got {:?}", other).into()),
    }

    assert!(parser.errors().is_empty(), "unexpected parser errors: {:?}", parser.errors());
    Ok(())
}

#[test]
fn parse_multiple_statements() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("my $x = 1; my $y = 2; my $z = 3;");
    let ast = must(parser.parse());

    match &ast.kind {
        V1NodeKind::Program { statements } => {
            assert!(
                statements.len() >= 3,
                "should parse three statements, got {}",
                statements.len()
            );
        }
        other => return Err(format!("expected Program, got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn parse_output_budget_usage_tracked() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("my $x = ;");
    let output = parser.parse_with_recovery();

    // Budget tracker should reflect diagnostics
    assert_eq!(output.budget_usage.errors_emitted, output.diagnostics.len());
    Ok(())
}
