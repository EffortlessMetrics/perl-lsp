use perl_parser::error_classifier::{ErrorClassifier, ParseErrorKind};
use perl_parser::{Node, NodeKind, SourceLocation};

#[test]
fn test_classify_unclosed_string_double_quote() {
    let classifier = ErrorClassifier::new();
    let error_node = Node {
        kind: NodeKind::Error { message: String::new() },
        location: SourceLocation { start: 0, end: 10 },
    };

    let source = r#""hello world"#;
    let result = classifier.classify(&error_node, source);
    assert_eq!(result, ParseErrorKind::UnclosedString);
}

#[test]
fn test_classify_unclosed_string_single_quote() {
    let classifier = ErrorClassifier::new();
    let error_node = Node {
        kind: NodeKind::Error { message: String::new() },
        location: SourceLocation { start: 0, end: 10 },
    };

    let source = "'hello world";
    let result = classifier.classify(&error_node, source);
    assert_eq!(result, ParseErrorKind::UnclosedString);
}

#[test]
fn test_classify_unclosed_regex() {
    let classifier = ErrorClassifier::new();
    let error_node = Node {
        kind: NodeKind::Error { message: String::new() },
        location: SourceLocation { start: 0, end: 10 },
    };

    let source = "/pattern";
    let result = classifier.classify(&error_node, source);
    assert_eq!(result, ParseErrorKind::UnclosedRegex);
}

#[test]
fn test_classify_missing_semicolon() {
    let classifier = ErrorClassifier::new();
    let error_node = Node {
        kind: NodeKind::Error { message: String::new() },
        location: SourceLocation { start: 13, end: 14 },
    };

    let source = "my $x = 42\nprint $x";
    let result = classifier.classify(&error_node, source);
    assert_eq!(result, ParseErrorKind::MissingSemicolon);
}

#[test]
fn test_classify_unclosed_parenthesis() {
    let classifier = ErrorClassifier::new();
    let error_node = Node {
        kind: NodeKind::Error { message: String::new() },
        location: SourceLocation { start: 8, end: 9 },
    };

    let source = "print (42";
    let result = classifier.classify(&error_node, source);
    assert_eq!(result, ParseErrorKind::UnclosedParenthesis);
}

#[test]
fn test_classify_unclosed_bracket() {
    let classifier = ErrorClassifier::new();
    let error_node = Node {
        kind: NodeKind::Error { message: String::new() },
        location: SourceLocation { start: 10, end: 11 },
    };

    let source = "my @arr = [1, 2, 3";
    let result = classifier.classify(&error_node, source);
    assert_eq!(result, ParseErrorKind::UnclosedBracket);
}

#[test]
fn test_classify_unclosed_brace() {
    let classifier = ErrorClassifier::new();
    let error_node = Node {
        kind: NodeKind::Error { message: String::new() },
        location: SourceLocation { start: 11, end: 12 },
    };

    let source = "my %hash = {a => 1";
    let result = classifier.classify(&error_node, source);
    assert_eq!(result, ParseErrorKind::UnclosedBrace);
}

#[test]
fn test_classify_unexpected_eof() {
    let classifier = ErrorClassifier::new();
    let error_node = Node {
        kind: NodeKind::Error { message: String::new() },
        location: SourceLocation { start: 5, end: 5 }, // At EOF
    };

    let source = "print";
    let result = classifier.classify(&error_node, source);
    assert_eq!(result, ParseErrorKind::UnexpectedEof);
}

#[test]
fn test_classify_invalid_syntax() {
    let classifier = ErrorClassifier::new();
    let error_node = Node {
        kind: NodeKind::Error { message: String::new() },
        location: SourceLocation { start: 0, end: 1 },
    };

    let source = "@@ invalid";
    let result = classifier.classify(&error_node, source);
    assert_eq!(result, ParseErrorKind::InvalidSyntax);
}

#[test]
fn test_get_suggestion_unclosed_string() {
    let classifier = ErrorClassifier::new();
    let suggestion = classifier.get_suggestion(&ParseErrorKind::UnclosedString);
    assert_eq!(suggestion, Some("Add a closing quote to terminate the string".to_string()));
}

#[test]
fn test_get_suggestion_missing_semicolon() {
    let classifier = ErrorClassifier::new();
    let suggestion = classifier.get_suggestion(&ParseErrorKind::MissingSemicolon);
    assert_eq!(suggestion, Some("Add a semicolon ';' at the end of the statement".to_string()));
}

#[test]
fn test_get_suggestion_unclosed_parenthesis() {
    let classifier = ErrorClassifier::new();
    let suggestion = classifier.get_suggestion(&ParseErrorKind::UnclosedParenthesis);
    assert_eq!(suggestion, Some("Add a closing parenthesis ')'".to_string()));
}

#[test]
fn test_boundary_conditions() {
    let classifier = ErrorClassifier::new();

    // Test with empty source
    let error_node = Node {
        kind: NodeKind::Error { message: String::new() },
        location: SourceLocation { start: 0, end: 0 },
    };
    let result = classifier.classify(&error_node, "");
    assert_eq!(result, ParseErrorKind::UnexpectedEof);

    // Test with out-of-bounds location
    let error_node = Node {
        kind: NodeKind::Error { message: String::new() },
        location: SourceLocation { start: 100, end: 200 },
    };
    let result = classifier.classify(&error_node, "short");
    assert_eq!(result, ParseErrorKind::InvalidSyntax);
}

#[test]
fn test_heredoc_classification() {
    let classifier = ErrorClassifier::new();
    let error_node = Node {
        kind: NodeKind::Error { message: String::new() },
        location: SourceLocation { start: 7, end: 8 },
    };

    let source = "<<'EOF'\nsome text";
    let result = classifier.classify(&error_node, source);
    assert_eq!(result, ParseErrorKind::UnterminatedHeredoc);
}

#[test]
fn test_classify_with_context() {
    let classifier = ErrorClassifier::new();

    // Test missing operator between operands
    let error_node = Node {
        kind: NodeKind::Error { message: String::new() },
        location: SourceLocation { start: 7, end: 8 },
    };
    let source = "my $x = 5 $y";
    let result = classifier.classify(&error_node, source);
    // This should detect missing operator
    assert!(matches!(result, ParseErrorKind::MissingOperator | ParseErrorKind::InvalidSyntax));
}

#[test]
fn test_error_classifier_default() {
    // Test that Default trait is implemented correctly
    let classifier1 = ErrorClassifier;
    let classifier2 = ErrorClassifier::new();

    // Both should behave the same
    let error_node = Node {
        kind: NodeKind::Error { message: String::new() },
        location: SourceLocation { start: 0, end: 5 },
    };

    let source = "'test";
    assert_eq!(
        classifier1.classify(&error_node, source),
        classifier2.classify(&error_node, source)
    );
}

#[test]
fn test_get_diagnostic_message() {
    let classifier = ErrorClassifier::new();

    // Test various error kinds
    assert_eq!(
        classifier.get_diagnostic_message(&ParseErrorKind::UnclosedString),
        "Unclosed string literal"
    );

    assert_eq!(
        classifier.get_diagnostic_message(&ParseErrorKind::MissingSemicolon),
        "Missing semicolon at end of statement"
    );

    assert_eq!(
        classifier.get_diagnostic_message(&ParseErrorKind::UnexpectedToken {
            expected: "identifier".to_string(),
            found: "keyword".to_string()
        }),
        "Expected identifier but found keyword"
    );

    assert_eq!(
        classifier.get_diagnostic_message(&ParseErrorKind::UnterminatedHeredoc),
        "Unterminated heredoc"
    );

    assert_eq!(
        classifier.get_diagnostic_message(&ParseErrorKind::InvalidVariableName),
        "Invalid variable name"
    );
}
