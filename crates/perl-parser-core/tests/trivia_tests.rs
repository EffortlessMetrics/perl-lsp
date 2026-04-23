use perl_parser_core::{
    ast_v2::NodeKind as V2NodeKind,
    // Trivia
    trivia::{NodeWithTrivia, Trivia, TriviaPreservingParser, TriviaToken},
    trivia_parser::format_with_trivia,
};

#[test]
fn trivia_whitespace_variant() -> Result<(), Box<dyn std::error::Error>> {
    let trivia = Trivia::Whitespace("  ".to_string());
    assert_eq!(trivia.as_str(), "  ");
    assert_eq!(trivia.kind_name(), "whitespace");
    Ok(())
}

#[test]
fn trivia_comment_variant() -> Result<(), Box<dyn std::error::Error>> {
    let trivia = Trivia::LineComment("# hello".to_string());
    assert_eq!(trivia.as_str(), "# hello");
    assert_eq!(trivia.kind_name(), "comment");
    Ok(())
}

#[test]
fn trivia_newline_variant() -> Result<(), Box<dyn std::error::Error>> {
    let trivia = Trivia::Newline;
    assert_eq!(trivia.as_str(), "\n");
    assert_eq!(trivia.kind_name(), "newline");
    Ok(())
}

#[test]
fn trivia_token_construction() -> Result<(), Box<dyn std::error::Error>> {
    let range = perl_position_tracking::Range::new(
        perl_position_tracking::Position::new(0, 1, 1),
        perl_position_tracking::Position::new(2, 1, 3),
    );
    let tt = TriviaToken::new(Trivia::Whitespace("  ".to_string()), range);
    assert_eq!(tt.trivia.as_str(), "  ");
    Ok(())
}

#[test]
fn trivia_preserving_parser_returns_node_with_trivia() -> Result<(), Box<dyn std::error::Error>> {
    let parser = TriviaPreservingParser::new("  # comment\nmy $x;".to_string());
    let result: NodeWithTrivia = parser.parse();

    // The parser should produce a Program node
    match &result.node.kind {
        V2NodeKind::Program { .. } => { /* ok */ }
        other => return Err(format!("expected Program, got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn format_with_trivia_includes_trivia_text() -> Result<(), Box<dyn std::error::Error>> {
    let range = perl_position_tracking::Range::new(
        perl_position_tracking::Position::new(0, 1, 1),
        perl_position_tracking::Position::new(0, 1, 1),
    );
    let node = perl_ast_v2::Node::new(
        perl_ast_v2::NodeIdGenerator::new().next_id(),
        V2NodeKind::Program { statements: vec![] },
        range,
    );

    let leading = vec![TriviaToken::new(Trivia::Whitespace("  ".to_string()), range)];
    let nwt = NodeWithTrivia { node, leading_trivia: leading, trailing_trivia: vec![] };

    let formatted = format_with_trivia(&nwt);
    assert!(formatted.contains("  "), "should include leading whitespace trivia");
    Ok(())
}
