use perl_parser_core::ParseError as CatastrophicParseError;

#[test]
fn unexpected_eof_display() -> Result<(), Box<dyn std::error::Error>> {
    let err = CatastrophicParseError::UnexpectedEof;
    let msg = format!("{}", err);
    assert!(msg.contains("Unexpected end of input"));
    Ok(())
}

#[test]
fn unexpected_token_display() -> Result<(), Box<dyn std::error::Error>> {
    let err = CatastrophicParseError::unexpected("semicolon", "comma", 42);
    let msg = format!("{}", err);
    assert!(msg.contains("semicolon"));
    assert!(msg.contains("comma"));
    assert!(msg.contains("42"));
    Ok(())
}

#[test]
fn syntax_error_display() -> Result<(), Box<dyn std::error::Error>> {
    let err = CatastrophicParseError::syntax("bad thing", 99);
    let msg = format!("{}", err);
    assert!(msg.contains("bad thing"));
    assert!(msg.contains("99"));
    Ok(())
}

#[test]
fn lexer_error_display() -> Result<(), Box<dyn std::error::Error>> {
    let err = CatastrophicParseError::LexerError { message: "bad char".to_string() };
    let msg = format!("{}", err);
    assert!(msg.contains("bad char"));
    Ok(())
}

#[test]
fn recursion_limit_display() -> Result<(), Box<dyn std::error::Error>> {
    let err = CatastrophicParseError::RecursionLimit;
    let msg = format!("{}", err);
    assert!(msg.contains("recursion") || msg.contains("Recursion"));
    Ok(())
}

#[test]
fn invalid_number_display() -> Result<(), Box<dyn std::error::Error>> {
    let err = CatastrophicParseError::InvalidNumber { literal: "0xZZ".to_string() };
    let msg = format!("{}", err);
    assert!(msg.contains("0xZZ"));
    Ok(())
}

#[test]
fn invalid_string_display() -> Result<(), Box<dyn std::error::Error>> {
    let err = CatastrophicParseError::InvalidString;
    let msg = format!("{}", err);
    assert!(msg.contains("string") || msg.contains("String"));
    Ok(())
}

#[test]
fn unclosed_delimiter_display() -> Result<(), Box<dyn std::error::Error>> {
    let err = CatastrophicParseError::UnclosedDelimiter { delimiter: '(' };
    let msg = format!("{}", err);
    assert!(msg.contains('('));
    Ok(())
}

#[test]
fn invalid_regex_display() -> Result<(), Box<dyn std::error::Error>> {
    let err = CatastrophicParseError::InvalidRegex { message: "unterminated group".to_string() };
    let msg = format!("{}", err);
    assert!(msg.contains("unterminated group"));
    Ok(())
}

#[test]
fn nesting_too_deep_display() -> Result<(), Box<dyn std::error::Error>> {
    let err = CatastrophicParseError::NestingTooDeep { depth: 300, max_depth: 256 };
    let msg = format!("{}", err);
    assert!(msg.contains("300"));
    assert!(msg.contains("256"));
    Ok(())
}

#[test]
fn location_for_positioned_errors() -> Result<(), Box<dyn std::error::Error>> {
    let err1 = CatastrophicParseError::unexpected("a", "b", 10);
    assert_eq!(err1.location(), Some(10));

    let err2 = CatastrophicParseError::syntax("msg", 20);
    assert_eq!(err2.location(), Some(20));
    Ok(())
}

#[test]
fn location_none_for_unpositioned_errors() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(CatastrophicParseError::UnexpectedEof.location(), None);
    assert_eq!(CatastrophicParseError::RecursionLimit.location(), None);
    assert_eq!(CatastrophicParseError::InvalidString.location(), None);
    assert_eq!(CatastrophicParseError::LexerError { message: "x".to_string() }.location(), None);
    Ok(())
}

#[test]
fn suggestion_for_semicolon() -> Result<(), Box<dyn std::error::Error>> {
    let err = CatastrophicParseError::unexpected("';'", "newline", 5);
    let suggestion = err.suggestion().unwrap_or_default();
    assert!(suggestion.contains("semicolon"));
    Ok(())
}

#[test]
fn suggestion_for_unclosed_delimiter() -> Result<(), Box<dyn std::error::Error>> {
    let err = CatastrophicParseError::UnclosedDelimiter { delimiter: ')' };
    let suggestion = err.suggestion().unwrap_or_default();
    assert!(suggestion.contains(')'));
    Ok(())
}

#[test]
fn suggestion_none_for_generic_errors() -> Result<(), Box<dyn std::error::Error>> {
    assert!(CatastrophicParseError::UnexpectedEof.suggestion().is_none());
    assert!(CatastrophicParseError::RecursionLimit.suggestion().is_none());
    Ok(())
}

#[test]
fn parse_error_equality() -> Result<(), Box<dyn std::error::Error>> {
    let e1 = CatastrophicParseError::UnexpectedEof;
    let e2 = CatastrophicParseError::UnexpectedEof;
    assert_eq!(e1, e2);

    let e3 = CatastrophicParseError::syntax("a", 1);
    let e4 = CatastrophicParseError::syntax("a", 1);
    assert_eq!(e3, e4);

    assert_ne!(e1, e3);
    Ok(())
}

#[test]
fn parse_error_clone() -> Result<(), Box<dyn std::error::Error>> {
    let err = CatastrophicParseError::unexpected("x", "y", 42);
    let cloned = err.clone();
    assert_eq!(err, cloned);
    Ok(())
}
