use perl_parser_core::{
    Node as V1Node,
    NodeKind as V1NodeKind,
    // Error types and recovery
    ParseError as CatastrophicParseError,
    ParseOutput,
    SourceLocation,
};

#[test]
fn success_output_has_no_diagnostics() -> Result<(), Box<dyn std::error::Error>> {
    let ast = V1Node::new(
        V1NodeKind::Program { statements: vec![] },
        SourceLocation { start: 0, end: 0 },
    );
    let output = ParseOutput::success(ast);
    assert!(output.diagnostics.is_empty());
    assert!(!output.terminated_early);
    assert_eq!(output.budget_usage.errors_emitted, 0);
    Ok(())
}

#[test]
fn with_errors_output_tracks_diagnostics() -> Result<(), Box<dyn std::error::Error>> {
    let ast = V1Node::new(
        V1NodeKind::Program { statements: vec![] },
        SourceLocation { start: 0, end: 0 },
    );
    let diags = vec![CatastrophicParseError::UnexpectedEof];
    let output = ParseOutput::with_errors(ast, diags);
    assert_eq!(output.diagnostics.len(), 1);
    assert_eq!(output.budget_usage.errors_emitted, 1);
    Ok(())
}
