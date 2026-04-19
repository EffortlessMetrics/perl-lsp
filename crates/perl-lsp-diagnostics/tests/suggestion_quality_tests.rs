//! Tests for suggestion quality in parser error diagnostics.
//!
//! Verifies that enhanced error messages and suggestions are generated
//! correctly for the most common parse error patterns (#441).

use std::sync::Arc;

use perl_lsp_diagnostics::{Diagnostic, DiagnosticsProvider};
use perl_parser_core::error::ParseError;
use perl_parser_core::{Node, NodeKind, SourceLocation};

fn empty_program() -> Arc<Node> {
    Arc::new(Node::new(
        NodeKind::Program { statements: vec![] },
        SourceLocation { start: 0, end: 0 },
    ))
}

fn diagnostics_for(source: &str, errors: Vec<ParseError>) -> Vec<Diagnostic> {
    let ast = empty_program();
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    provider.get_diagnostics(&ast, &errors, source, None)
}

fn parse_diags(diags: &[Diagnostic]) -> Vec<&Diagnostic> {
    // PL001=ParseError, PL002=SyntaxError, PL003=UnexpectedEof
    diags
        .iter()
        .filter(|d| {
            matches!(
                d.code.as_deref(),
                Some("PL001") | Some("PL002") | Some("PL003")
            )
        })
        .collect()
}

// ---- Enhanced messages ----

#[test]
fn missing_semicolon_enhanced_message() -> Result<(), Box<dyn std::error::Error>> {
    let diags = diagnostics_for(
        "my $x = 1",
        vec![ParseError::UnexpectedToken {
            location: 9,
            expected: ";".to_string(),
            found: "my".to_string(),
        }],
    );
    let pd = parse_diags(&diags);
    assert!(!pd.is_empty(), "should produce a diagnostic");
    assert!(
        pd[0].message.contains("Missing semicolon"),
        "expected semicolon message, got: {}",
        pd[0].message
    );
    Ok(())
}

#[test]
fn expected_variable_enhanced_message() -> Result<(), Box<dyn std::error::Error>> {
    let diags = diagnostics_for(
        "my 1;",
        vec![ParseError::UnexpectedToken {
            location: 3,
            expected: "variable".to_string(),
            found: "1".to_string(),
        }],
    );
    let pd = parse_diags(&diags);
    assert!(!pd.is_empty());
    assert!(
        pd[0].message.contains("$foo")
            && pd[0].message.contains("@bar")
            && pd[0].message.contains("%hash"),
        "expected variable examples, got: {}",
        pd[0].message
    );
    Ok(())
}

#[test]
fn unexpected_closing_brace_enhanced_message() -> Result<(), Box<dyn std::error::Error>> {
    let diags = diagnostics_for(
        "my $x = 1; }",
        vec![ParseError::UnexpectedToken {
            location: 11,
            expected: "expression".to_string(),
            found: "}".to_string(),
        }],
    );
    let pd = parse_diags(&diags);
    assert!(!pd.is_empty());
    assert!(
        pd[0].message.contains("Unexpected `}`") && pd[0].message.contains("{"),
        "expected brace mismatch message, got: {}",
        pd[0].message
    );
    Ok(())
}

#[test]
fn unexpected_closing_paren_enhanced_message() -> Result<(), Box<dyn std::error::Error>> {
    let diags = diagnostics_for(
        "my $x = 1; )",
        vec![ParseError::UnexpectedToken {
            location: 11,
            expected: "expression".to_string(),
            found: ")".to_string(),
        }],
    );
    let pd = parse_diags(&diags);
    assert!(!pd.is_empty());
    assert!(
        pd[0].message.contains("Unexpected")
            && pd[0].message.contains(")")
            && pd[0].message.contains("("),
        "expected paren mismatch message, got: {}",
        pd[0].message
    );
    Ok(())
}

#[test]
fn unexpected_closing_bracket_enhanced_message() -> Result<(), Box<dyn std::error::Error>> {
    let diags = diagnostics_for(
        "my $x = 1; ]",
        vec![ParseError::UnexpectedToken {
            location: 11,
            expected: "expression".to_string(),
            found: "]".to_string(),
        }],
    );
    let pd = parse_diags(&diags);
    assert!(!pd.is_empty());
    assert!(
        pd[0].message.contains("Unexpected `]`") && pd[0].message.contains("["),
        "expected bracket mismatch message, got: {}",
        pd[0].message
    );
    Ok(())
}

#[test]
fn default_message_fallback() -> Result<(), Box<dyn std::error::Error>> {
    let diags = diagnostics_for(
        "bad",
        vec![ParseError::UnexpectedToken {
            location: 0,
            expected: "something_unusual".to_string(),
            found: "other_thing".to_string(),
        }],
    );
    let pd = parse_diags(&diags);
    assert!(!pd.is_empty());
    assert!(
        pd[0].message.contains("Expected something_unusual"),
        "default message should contain expected token: {}",
        pd[0].message
    );
    Ok(())
}

// ---- Suggestions ----

#[test]
fn semicolon_suggestion_text() -> Result<(), Box<dyn std::error::Error>> {
    let diags = diagnostics_for(
        "my $x = 1",
        vec![ParseError::UnexpectedToken {
            location: 9,
            expected: ";".to_string(),
            found: "my".to_string(),
        }],
    );
    let pd = parse_diags(&diags);
    assert!(!pd.is_empty());
    let suggestion = pd[0].suggestion.as_deref().unwrap_or_default();
    assert!(!suggestion.is_empty(), "should have suggestion");
    assert!(
        suggestion.contains("`;`"),
        "semicolon suggestion should mention backtick-quoted semicolon: {suggestion}"
    );
    Ok(())
}

#[test]
fn closing_bracket_suggestion() -> Result<(), Box<dyn std::error::Error>> {
    let diags = diagnostics_for(
        "my @a = (1",
        vec![ParseError::UnexpectedToken {
            location: 10,
            expected: "]".to_string(),
            found: "foo".to_string(),
        }],
    );
    let pd = parse_diags(&diags);
    assert!(!pd.is_empty());
    let suggestion = pd[0].suggestion.as_deref().unwrap_or_default();
    assert!(!suggestion.is_empty(), "should have bracket suggestion");
    assert!(
        suggestion.contains("]") && suggestion.contains("["),
        "should suggest closing bracket: {suggestion}"
    );
    Ok(())
}

#[test]
fn variable_suggestion() -> Result<(), Box<dyn std::error::Error>> {
    let diags = diagnostics_for(
        "my 1;",
        vec![ParseError::UnexpectedToken {
            location: 3,
            expected: "variable".to_string(),
            found: "1".to_string(),
        }],
    );
    let pd = parse_diags(&diags);
    assert!(!pd.is_empty());
    let suggestion = pd[0].suggestion.as_deref().unwrap_or_default();
    assert!(!suggestion.is_empty(), "should have variable suggestion");
    assert!(
        suggestion.contains("$foo") && suggestion.contains("@bar") && suggestion.contains("%hash"),
        "variable suggestion should mention all sigil types: {suggestion}"
    );
    Ok(())
}

// ---- Related information ----

#[test]
fn suggestion_surfaces_as_related_information() -> Result<(), Box<dyn std::error::Error>> {
    let diags = diagnostics_for(
        "my $x = 1",
        vec![ParseError::UnexpectedToken {
            location: 9,
            expected: ";".to_string(),
            found: "my".to_string(),
        }],
    );
    let pd = parse_diags(&diags);
    assert!(!pd.is_empty());
    assert!(pd[0].suggestion.is_some(), "should have a suggestion");
    assert!(
        !pd[0].related_information.is_empty(),
        "diagnostic with suggestion should have related_information"
    );
    assert!(
        pd[0].related_information[0]
            .message
            .starts_with("Suggestion:"),
        "related info should start with 'Suggestion:': {}",
        pd[0].related_information[0].message
    );
    Ok(())
}

#[test]
fn no_related_info_when_no_suggestion() -> Result<(), Box<dyn std::error::Error>> {
    let diags = diagnostics_for("my $x = 1;", vec![]);
    let pd = parse_diags(&diags);
    assert!(
        pd.is_empty(),
        "no parse errors means no parse-error diagnostics"
    );
    Ok(())
}

#[test]
fn all_parse_errors_have_code() -> Result<(), Box<dyn std::error::Error>> {
    // Place errors far apart (> 10 bytes) so cascade suppression does not collapse
    // them into a single cluster.  The goal of this test is to verify that every
    // parse-error diagnostic carries a stable PL-prefixed code.
    //
    // Error placement:
    //   - UnexpectedToken  @ offset 0  (PL001)
    //   - LexerError       @ offset 30 (PL001) — global error, placed at a distinct cluster
    //   - UnexpectedEof    @ source.len() = 60 (PL003)
    let source = "a".repeat(60);
    let errors = vec![
        ParseError::UnexpectedToken {
            location: 0,
            expected: ";".to_string(),
            found: "x".to_string(),
        },
        ParseError::SyntaxError {
            location: 30,
            message: "syntax error mid-file".to_string(),
        },
        ParseError::UnexpectedEof,
    ];
    let diags = diagnostics_for(&source, errors);
    let pd = parse_diags(&diags);
    assert!(pd.len() >= 3, "expected at least 3 parse-error diagnostics");
    for d in &pd {
        // Each parse error should have a stable PL-prefixed code
        assert!(
            matches!(
                d.code.as_deref(),
                Some("PL001") | Some("PL002") | Some("PL003")
            ),
            "Parse error should have stable code PL001/PL002/PL003, got: {:?}",
            d.code
        );
    }
    Ok(())
}

// ---- ParseError::suggestion() method ----

#[test]
fn parse_error_variable_suggestion() -> Result<(), Box<dyn std::error::Error>> {
    let error = ParseError::UnexpectedToken {
        location: 0,
        expected: "variable".to_string(),
        found: "1".to_string(),
    };
    let suggestion = error.suggestion();
    assert!(suggestion.is_some(), "should have variable suggestion");
    let s = suggestion.as_deref().unwrap_or_default();
    assert!(
        s.contains("$foo") && s.contains("@bar") && s.contains("%hash"),
        "variable suggestion should mention all sigil types: {s}"
    );
    Ok(())
}
