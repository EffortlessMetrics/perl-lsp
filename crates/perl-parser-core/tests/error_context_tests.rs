use perl_parser_core::ParseError as CatastrophicParseError;
use perl_parser_core::error::get_error_contexts;

#[test]
fn error_context_single_line() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 42";
    let errors = vec![CatastrophicParseError::syntax("missing semicolon", 10)];
    let contexts = get_error_contexts(&errors, source);
    assert_eq!(contexts.len(), 1);
    assert_eq!(contexts[0].line, 0);
    assert_eq!(contexts[0].source_line, "my $x = 42");
    Ok(())
}

#[test]
fn error_context_multiline() -> Result<(), Box<dyn std::error::Error>> {
    let source = "line1;\nline2;\nline3;";
    // byte offset 7 is start of "line2;"
    let errors = vec![CatastrophicParseError::syntax("bad", 7)];
    let contexts = get_error_contexts(&errors, source);
    assert_eq!(contexts.len(), 1);
    assert_eq!(contexts[0].line, 1);
    assert_eq!(contexts[0].source_line, "line2;");
    Ok(())
}

#[test]
fn error_context_at_eof() -> Result<(), Box<dyn std::error::Error>> {
    let source = "short";
    let errors = vec![CatastrophicParseError::UnexpectedEof];
    let contexts = get_error_contexts(&errors, source);
    assert_eq!(contexts.len(), 1);
    // UnexpectedEof has no location, defaults to source.len()
    Ok(())
}

#[test]
fn error_context_with_suggestion() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1";
    let errors = vec![CatastrophicParseError::unexpected("';'", "EOF", 9)];
    let contexts = get_error_contexts(&errors, source);
    assert_eq!(contexts.len(), 1);
    // The suggestion should be present since expected contains "';'"
    let suggestion = contexts[0].suggestion.as_deref().unwrap_or("");
    assert!(suggestion.contains("semicolon"));
    Ok(())
}

#[test]
fn error_context_empty_source() -> Result<(), Box<dyn std::error::Error>> {
    let source = "";
    let errors = vec![CatastrophicParseError::UnexpectedEof];
    let contexts = get_error_contexts(&errors, source);
    assert_eq!(contexts.len(), 1);
    assert_eq!(contexts[0].line, 0);
    Ok(())
}

#[test]
fn error_context_multiple_errors() -> Result<(), Box<dyn std::error::Error>> {
    let source = "a;\nb;\nc;";
    let errors = vec![
        CatastrophicParseError::syntax("err1", 0),
        CatastrophicParseError::syntax("err2", 3),
        CatastrophicParseError::syntax("err3", 6),
    ];
    let contexts = get_error_contexts(&errors, source);
    assert_eq!(contexts.len(), 3);
    assert_eq!(contexts[0].line, 0);
    assert_eq!(contexts[1].line, 1);
    assert_eq!(contexts[2].line, 2);
    Ok(())
}
