use perl_parser_core::{
    NodeKind as V1NodeKind,
    // Parser
    Parser,
    // Position mapping
    PositionMapper,
};
use perl_tdd_support::must;

#[test]
fn parser_many_errors_recovers() -> Result<(), Box<dyn std::error::Error>> {
    // Source with multiple syntax errors — the production parser should recover
    // and return a program node rather than failing catastrophically.
    let source = "my $a = ; my $b = ; my $c = ;";
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    match &ast.kind {
        V1NodeKind::Program { statements } => {
            assert!(
                !statements.is_empty(),
                "should produce at least one statement"
            );
        }
        other => return Err(format!("expected Program, got {:?}", other).into()),
    }
    // Should have recorded at least one error
    assert!(
        !parser.errors().is_empty(),
        "should have errors for missing expressions"
    );
    Ok(())
}

#[test]
fn parser_parse_and_errors_consistent() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("my $x = 42; sub hello { }");
    let ast = must(parser.parse());

    match &ast.kind {
        V1NodeKind::Program { statements } => {
            assert!(statements.len() >= 2, "should parse declaration and sub");
        }
        other => return Err(format!("expected Program, got {:?}", other).into()),
    }
    // Valid code should have no errors
    assert!(
        parser.errors().is_empty(),
        "valid code should have no parse errors"
    );
    Ok(())
}

#[test]
fn whitespace_only_input() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("   \n\n  \t  ");
    let ast = must(parser.parse());

    match &ast.kind {
        V1NodeKind::Program { statements } => {
            assert!(
                statements.is_empty(),
                "whitespace-only should yield empty program"
            );
        }
        other => return Err(format!("expected Program, got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn comment_only_input() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("# just a comment\n# another one\n");
    let ast = must(parser.parse());

    match &ast.kind {
        V1NodeKind::Program { statements } => {
            assert!(
                statements.is_empty(),
                "comment-only should yield empty program"
            );
        }
        other => return Err(format!("expected Program, got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn position_mapper_with_parser() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 42;\nmy $y = 99;";
    let mapper = PositionMapper::new(source);
    let mut parser = Parser::new(source);
    let _ast = must(parser.parse());

    // Verify mapper agrees on line count
    assert_eq!(mapper.len_lines(), 2);
    // First char of second line
    let pos = mapper.byte_to_lsp_pos(12);
    assert_eq!(pos.line, 1);
    Ok(())
}
