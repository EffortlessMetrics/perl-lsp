use perl_parser_core::{
    BudgetTracker,
    LineEnding,
    // AST (v1) types used by Parser
    Node as V1Node,
    NodeKind as V1NodeKind,
    ParseBudget,
    // Error types and recovery
    ParseError as CatastrophicParseError,
    ParseOutput,
    // Parser
    Parser,
    // Position mapping
    PositionMapper,
    SourceLocation,
    // AST (v2) types used by RecoveryParser
    ast_v2::MissingKind,
    ast_v2::NodeKind as V2NodeKind,
    builtin_signatures,
    builtin_signatures_phf,
    error::recovery_parser::RecoveryParser,
    error_recovery::{ParseError as RecoveryParseError, RecoveryResult, SyncPoint},
    line_index::LineIndex,
    // ParserContext
    parser_context::ParserContext,
    // Token stream
    token_stream::TokenStream,
    // Trivia
    trivia::{NodeWithTrivia, Trivia, TriviaPreservingParser, TriviaToken},
    trivia_parser::format_with_trivia,
};
use perl_tdd_support::{must, must_some};

#[test]
fn recovery_parser_many_errors() -> Result<(), Box<dyn std::error::Error>> {
    // Source with multiple syntax errors
    let source = "my $a = ; my $b = ; my $c = ;".to_string();
    let parser = RecoveryParser::new(source);
    let (ast, errors) = parser.parse();

    match &ast.kind {
        V2NodeKind::Program { statements } => {
            assert!(statements.len() >= 3, "should attempt to parse all three decls");
        }
        other => return Err(format!("expected Program, got {:?}", other).into()),
    }
    assert!(errors.len() >= 3, "should have at least 3 errors");
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
    assert!(parser.errors().is_empty(), "valid code should have no parse errors");
    Ok(())
}

#[test]
fn whitespace_only_input() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("   \n\n  \t  ");
    let ast = must(parser.parse());

    match &ast.kind {
        V1NodeKind::Program { statements } => {
            assert!(statements.is_empty(), "whitespace-only should yield empty program");
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
            assert!(statements.is_empty(), "comment-only should yield empty program");
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
