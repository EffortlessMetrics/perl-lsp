use perl_parser_core::error_recovery::ParseError as RecoveryParseError;

#[test]
fn parse_error_builder_chain() -> Result<(), Box<dyn std::error::Error>> {
    let range = perl_position_tracking::Range::new(
        perl_position_tracking::Position::new(0, 1, 1),
        perl_position_tracking::Position::new(5, 1, 6),
    );

    let err = RecoveryParseError::new("test error".to_string(), range)
        .with_expected(vec!["semicolon".to_string()])
        .with_found("brace".to_string())
        .with_hint("add a semicolon".to_string());

    assert_eq!(err.message, "test error");
    assert_eq!(err.expected, vec!["semicolon"]);
    assert_eq!(err.found, "brace");
    assert_eq!(err.recovery_hint, Some("add a semicolon".to_string()));
    Ok(())
}

#[test]
fn parse_error_default_fields() -> Result<(), Box<dyn std::error::Error>> {
    let range = perl_position_tracking::Range::new(
        perl_position_tracking::Position::new(0, 1, 1),
        perl_position_tracking::Position::new(0, 1, 1),
    );

    let err = RecoveryParseError::new("msg".to_string(), range);
    assert!(err.expected.is_empty());
    assert!(err.found.is_empty());
    assert!(err.recovery_hint.is_none());
    Ok(())
}
