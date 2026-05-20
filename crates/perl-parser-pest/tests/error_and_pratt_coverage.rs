use perl_parser_pest::error::{ParseErrorKind, ScannerError, UnicodeError};
use perl_parser_pest::pratt_parser::Associativity;
use perl_parser_pest::{ParseError, PrattParser};
use perl_tdd_support::must_err;

fn invalid_token_message(error: ParseError) -> Result<String, Box<dyn std::error::Error>> {
    match error {
        ParseError::InvalidToken(message) => Ok(message),
        other => Err(format!("expected InvalidToken, got {other:?}").into()),
    }
}

fn scanner_error_message(error: ParseError) -> Result<String, Box<dyn std::error::Error>> {
    match error {
        ParseError::ScannerError(message) => Ok(message),
        other => Err(format!("expected ScannerError, got {other:?}").into()),
    }
}

#[test]
fn parse_error_new_formats_each_error_kind() -> Result<(), Box<dyn std::error::Error>> {
    let cases = [
        (ParseErrorKind::UnexpectedToken, "Unexpected token at position 7: detail"),
        (ParseErrorKind::UnexpectedEndOfInput, "Unexpected end of input at position 7: detail"),
        (ParseErrorKind::InvalidSyntax, "Invalid syntax at position 7: detail"),
        (ParseErrorKind::InvalidNumber, "Invalid number at position 7: detail"),
        (ParseErrorKind::InvalidString, "Invalid string at position 7: detail"),
        (ParseErrorKind::InvalidRegex, "Invalid regex at position 7: detail"),
        (ParseErrorKind::InvalidVariable, "Invalid variable at position 7: detail"),
        (
            ParseErrorKind::MissingToken("semicolon".to_string()),
            "Missing semicolon at position 7: detail",
        ),
        (ParseErrorKind::InvalidOperator, "Invalid operator at position 7: detail"),
        (ParseErrorKind::InvalidIdentifier, "Invalid identifier at position 7: detail"),
    ];

    for (kind, expected) in cases {
        let message = invalid_token_message(ParseError::new(kind, 7, "detail".to_string()))?;
        assert_eq!(message, expected);
    }

    Ok(())
}

#[test]
fn parse_error_constructors_preserve_position_and_token_text()
-> Result<(), Box<dyn std::error::Error>> {
    let unterminated = scanner_error_message(ParseError::unterminated_string((3, 14)))?;
    assert_eq!(unterminated, "Unterminated string literal at line 3, column 14");

    let invalid = invalid_token_message(ParseError::invalid_token("???".to_string(), (8, 2)))?;
    assert_eq!(invalid, "Invalid token '???' at line 8, column 2");

    assert_eq!(ParseError::unicode_error("bad codepoint"), ParseError::InvalidUnicode);

    let scanner = scanner_error_message(ParseError::scanner_error_simple("state mismatch"))?;
    assert_eq!(scanner, "state mismatch");

    Ok(())
}

#[test]
fn parse_error_from_conversions_keep_source_context() -> Result<(), Box<dyn std::error::Error>> {
    let scanner =
        scanner_error_message(ParseError::from(ScannerError::InvalidEscape("\\q".to_string())))?;
    assert_eq!(scanner, "Invalid escape sequence: \\q");

    let unicode =
        scanner_error_message(ParseError::from(UnicodeError::InvalidCodePoint(0x11_0000)))?;
    assert_eq!(unicode, "Invalid Unicode code point: 1114112");

    let invalid_utf8_bytes = vec![u8::MAX];
    let utf8_error = must_err(std::str::from_utf8(&invalid_utf8_bytes));
    let ParseError::InvalidUtf8(message) = ParseError::from(utf8_error) else {
        return Err("expected InvalidUtf8 from Utf8Error".into());
    };
    assert!(message.contains("invalid utf-8 sequence"));

    let from_utf8_error = must_err(String::from_utf8(invalid_utf8_bytes));
    let ParseError::InvalidUtf8(message) = ParseError::from(from_utf8_error) else {
        return Err("expected InvalidUtf8 from FromUtf8Error".into());
    };
    assert!(message.contains("invalid utf-8 sequence"));

    let io = scanner_error_message(ParseError::from(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "fixture denied",
    )))?;
    assert_eq!(io, "I/O error: fixture denied");

    Ok(())
}

#[test]
fn pratt_operator_table_exposes_precedence_and_associativity()
-> Result<(), Box<dyn std::error::Error>> {
    let parser = PrattParser::default();

    let assignment = parser.get_operator_info("=").ok_or("expected assignment operator info")?;
    assert_eq!(assignment.precedence.0, 3);
    assert_eq!(assignment.associativity, Associativity::Right);

    let range = parser.get_operator_info("...").ok_or("expected range operator info")?;
    assert_eq!(range.precedence.0, 5);
    assert_eq!(range.associativity, Associativity::None);

    let match_operator = parser.get_operator_info("=~").ok_or("expected match operator info")?;
    assert_eq!(match_operator.precedence.0, 29);
    assert_eq!(match_operator.associativity, Associativity::Left);

    assert!(parser.get_operator_info("~~not-an-operator~~").is_none());

    Ok(())
}

#[test]
fn pratt_prefix_and_postfix_operator_classifiers_cover_perl_specific_forms()
-> Result<(), Box<dyn std::error::Error>> {
    for op in ["!", "not", "~.", "\\", "defined", "state"] {
        assert!(PrattParser::is_prefix_operator(op), "expected {op} to be prefix");
    }

    for op in ["++", "--"] {
        assert!(PrattParser::is_prefix_operator(op), "expected {op} to be prefix");
        assert!(PrattParser::is_postfix_operator(op), "expected {op} to be postfix");
    }

    for op in ["=", "=>", "print", "~~not-an-operator~~"] {
        assert!(!PrattParser::is_prefix_operator(op), "expected {op} not to be prefix");
        assert!(!PrattParser::is_postfix_operator(op), "expected {op} not to be postfix");
    }

    Ok(())
}
