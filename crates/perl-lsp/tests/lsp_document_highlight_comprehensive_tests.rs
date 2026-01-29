//! Comprehensive integration tests for Document Highlight with modern Perl syntax (Issue #191)

use serde_json::json;

mod support;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn test_highlight_variable_in_try_catch() -> TestResult {
    let code = r#"
use feature 'try';
my $error = "none";
try {
    die "failed";
} catch ($error) {
    warn $error;
}
print $error;
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///test_try.pl", code)?;

    // Highlight $error in catch param (line 5, col 10)
    let result = harness.request(
        "textDocument/documentHighlight",
        json!({
            "textDocument": {"uri": "file:///test_try.pl"},
            "position": {"line": 5, "character": 10}
        }),
    )?;

    let highlights = result.as_array().ok_or("Expected array result")?;
    
    // Should find all 4 occurrences: declaration, catch param, warn, print
    // (Note: Die doesn't use $error)
    assert!(highlights.len() >= 4, "Found {} highlights, expected at least 4", highlights.len());
    
    Ok(())
}

#[test]
fn test_highlight_given_when_variable() -> TestResult {
    let code = r#"
use feature 'switch';
my $value = 42;
given ($value) {
    when (42) { print $value; }
    default { warn $value; }
}
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///test_given.pl", code)?;

    // Highlight $value in given (line 3, col 8)
    let result = harness.request(
        "textDocument/documentHighlight",
        json!({
            "textDocument": {"uri": "file:///test_given.pl"},
            "position": {"line": 3, "character": 8}
        }),
    )?;

    let highlights = result.as_array().ok_or("Expected array result")?;
    
    // Should find all 4 occurrences: declaration, given, when block, default block
    assert!(highlights.len() >= 4, "Found {} highlights, expected at least 4", highlights.len());
    
    Ok(())
}

#[test]
fn test_highlight_method_parameter() -> TestResult {
    let code = r#"
use feature 'class';
class Point {
    field $x :param = 0;
    method move($delta) {
        $x += $delta;
        print "Moved by $delta";
    }
}
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///test_method.pl", code)?;

    // Highlight $delta in method param (line 4, col 17)
    let result = harness.request(
        "textDocument/documentHighlight",
        json!({
            "textDocument": {"uri": "file:///test_method.pl"},
            "position": {"line": 4, "character": 17}
        }),
    )?;

    let highlights = result.as_array().ok_or("Expected array result")?;
    
    // Should find 3 occurrences: parameter, addition, print
    assert!(highlights.len() >= 3, "Found {} highlights, expected at least 3", highlights.len());
    
    Ok(())
}

#[test]
fn test_highlight_signature_parameters() -> TestResult {
    let code = r#"
use feature 'signatures';
sub greet($name, $greeting = "Hello") {
    print "$greeting, $name!";
}
greet("World");
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///test_sig.pl", code)?;

    // Highlight $name in signature (line 2, col 11)
    let result = harness.request(
        "textDocument/documentHighlight",
        json!({
            "textDocument": {"uri": "file:///test_sig.pl"},
            "position": {"line": 2, "character": 11}
        }),
    )?;

    let highlights = result.as_array().ok_or("Expected array result")?;
    
    // Should find 2 occurrences: parameter and string interpolation
    assert!(highlights.len() >= 2, "Found {} highlights, expected at least 2", highlights.len());
    
    Ok(())
}

#[test]
fn test_highlight_statement_modifier_modern() -> TestResult {
    let code = r#"
my $timeout = 30;
do_something() while $timeout-- > 0;
print "Final timeout: $timeout";
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///test_mod.pl", code)?;

    // Highlight $timeout in declaration (line 1, col 4)
    let result = harness.request(
        "textDocument/documentHighlight",
        json!({
            "textDocument": {"uri": "file:///test_mod.pl"},
            "position": {"line": 1, "character": 4}
        }),
    )?;

    let highlights = result.as_array().ok_or("Expected array result")?;
    
    // Should find 3 occurrences: declaration, while modifier, print
    assert!(highlights.len() >= 3, "Found {} highlights, expected at least 3", highlights.len());
    
    Ok(())
}
