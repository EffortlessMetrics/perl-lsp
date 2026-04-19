use perl_parser_core::error_classifier::{ErrorClassifier, ParseErrorKind};
use perl_parser_core::{Node as V1Node, NodeKind as V1NodeKind, SourceLocation};

#[test]
fn classifier_default_and_new() -> Result<(), Box<dyn std::error::Error>> {
    let _c1 = ErrorClassifier::new();
    let _c2 = ErrorClassifier;
    Ok(())
}

#[test]
fn classify_unclosed_double_quote() -> Result<(), Box<dyn std::error::Error>> {
    let classifier = ErrorClassifier::new();
    let source = r#"my $x = "hello"#;
    let node = V1Node::new(
        V1NodeKind::Error {
            message: "err".to_string(),
            expected: vec![],
            found: None,
            partial: None,
        },
        SourceLocation::new(9, 15),
    );
    let kind = classifier.classify(&node, source);
    assert_eq!(kind, ParseErrorKind::UnclosedString);
    Ok(())
}

#[test]
fn classify_missing_semicolon() -> Result<(), Box<dyn std::error::Error>> {
    let classifier = ErrorClassifier::new();
    let source = "my $x = 42\nmy $y = 10;";
    let node = V1Node::new(
        V1NodeKind::Error {
            message: "err".to_string(),
            expected: vec![],
            found: None,
            partial: None,
        },
        SourceLocation::new(10, 11),
    );
    let kind = classifier.classify(&node, source);
    assert_eq!(kind, ParseErrorKind::MissingSemicolon);
    Ok(())
}

#[test]
fn classify_unclosed_paren() -> Result<(), Box<dyn std::error::Error>> {
    let classifier = ErrorClassifier::new();
    let source = "my $x = (1 + 2;";
    let node = V1Node::new(
        V1NodeKind::Error {
            message: "err".to_string(),
            expected: vec![],
            found: None,
            partial: None,
        },
        SourceLocation::new(8, 9),
    );
    let kind = classifier.classify(&node, source);
    assert_eq!(kind, ParseErrorKind::UnclosedParenthesis);
    Ok(())
}

#[test]
fn diagnostic_message_all_kinds() -> Result<(), Box<dyn std::error::Error>> {
    let c = ErrorClassifier::new();
    // Just verify messages are non-empty for each kind
    let kinds = vec![
        ParseErrorKind::UnclosedString,
        ParseErrorKind::UnclosedRegex,
        ParseErrorKind::UnclosedBlock,
        ParseErrorKind::MissingSemicolon,
        ParseErrorKind::InvalidSyntax,
        ParseErrorKind::UnclosedParenthesis,
        ParseErrorKind::UnclosedBracket,
        ParseErrorKind::UnclosedBrace,
        ParseErrorKind::UnterminatedHeredoc,
        ParseErrorKind::InvalidVariableName,
        ParseErrorKind::InvalidSubroutineName,
        ParseErrorKind::MissingOperator,
        ParseErrorKind::MissingOperand,
        ParseErrorKind::UnexpectedEof,
        ParseErrorKind::UnexpectedToken {
            expected: "ident".to_string(),
            found: "number".to_string(),
        },
    ];
    for kind in &kinds {
        let msg = c.get_diagnostic_message(kind);
        assert!(!msg.is_empty(), "empty message for {:?}", kind);
    }
    Ok(())
}

#[test]
fn suggestion_some_for_most_kinds() -> Result<(), Box<dyn std::error::Error>> {
    let c = ErrorClassifier::new();
    // These should all have suggestions
    let kinds_with_suggestions = vec![
        ParseErrorKind::MissingSemicolon,
        ParseErrorKind::UnclosedString,
        ParseErrorKind::UnclosedParenthesis,
        ParseErrorKind::UnclosedBracket,
        ParseErrorKind::UnclosedBrace,
        ParseErrorKind::UnclosedBlock,
        ParseErrorKind::UnclosedRegex,
        ParseErrorKind::UnterminatedHeredoc,
        ParseErrorKind::UnexpectedEof,
    ];
    for kind in &kinds_with_suggestions {
        assert!(
            c.get_suggestion(kind).is_some(),
            "no suggestion for {:?}",
            kind
        );
    }
    // InvalidSyntax should have no suggestion
    assert!(c.get_suggestion(&ParseErrorKind::InvalidSyntax).is_none());
    Ok(())
}

#[test]
fn explanation_some_for_common_kinds() -> Result<(), Box<dyn std::error::Error>> {
    let c = ErrorClassifier::new();
    assert!(
        c.get_explanation(&ParseErrorKind::MissingSemicolon)
            .is_some()
    );
    assert!(c.get_explanation(&ParseErrorKind::UnclosedString).is_some());
    assert!(c.get_explanation(&ParseErrorKind::UnclosedRegex).is_some());
    assert!(
        c.get_explanation(&ParseErrorKind::UnterminatedHeredoc)
            .is_some()
    );
    assert!(c.get_explanation(&ParseErrorKind::UnclosedBlock).is_some());
    // InvalidSyntax has no explanation
    assert!(c.get_explanation(&ParseErrorKind::InvalidSyntax).is_none());
    Ok(())
}
