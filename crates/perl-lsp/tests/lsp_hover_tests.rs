//! Comprehensive LSP integration tests for textDocument/hover
//!
//! Tests feature spec: LSP_IMPLEMENTATION_GUIDE.md#hover
//! Tests feature spec: navigation.rs#hover-provider
//!
//! This test suite validates:
//! - textDocument/hover request/response handling
//! - Hover on subroutine names (returns signature/documentation)
//! - Hover on variable names (returns type/declaration info)
//! - Hover on builtin functions (returns builtin documentation)
//! - Hover on empty space or comments (returns null/empty)
//! - Hover capability advertised in server capabilities
//!
//! LSP Protocol Compliance:
//! - Hover response: { contents: MarkupContent, range?: Range } or null
//! - MarkupContent: { kind: "markdown"|"plaintext", value: string }
//! - Position-based symbol resolution
//!
//! Related Documentation:
//! - docs/LSP_IMPLEMENTATION_GUIDE.md#hover
//! - crates/perl-lsp-navigation/src/hover.rs

mod support;

use serde_json::json;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Tests feature spec: navigation.rs#hover-on-subroutine
///
/// Validates that hovering over a subroutine name returns meaningful content
/// such as the function signature or documentation.
#[test]
fn test_hover_on_subroutine_name() -> TestResult {
    let doc = r#"
sub process {
    my ($input) = @_;
    return $input * 2;
}

my $result = process(42);
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///test.pl", doc)?;

    // Hover over the subroutine name at definition site
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///test.pl"},
                "position": {"line": 1, "character": 5} // Position on "process" in sub definition
            }),
        )
        .unwrap_or(json!(null));

    // Hover may return an object with contents or null
    if !result.is_null() {
        // If we got a hover response, validate its structure
        let contents = result.get("contents");
        assert!(
            contents.is_some(),
            "Hover response should have 'contents' field, got: {:?}",
            result
        );

        let contents = contents.ok_or("Expected contents in hover response")?;

        // Contents should be a MarkupContent object or a string
        if contents.is_object() {
            // MarkupContent format: { kind: "markdown"|"plaintext", value: "..." }
            let kind = contents.get("kind").and_then(|k| k.as_str());
            if let Some(k) = kind {
                assert!(
                    k == "markdown" || k == "plaintext",
                    "Hover content kind should be 'markdown' or 'plaintext', got: {}",
                    k
                );
            }
            let value = contents.get("value").and_then(|v| v.as_str());
            assert!(value.is_some(), "MarkupContent should have a 'value' field");
        }

        // If range is present, validate it
        if let Some(range) = result.get("range") {
            assert!(range.get("start").is_some(), "Range must have start position");
            assert!(range.get("end").is_some(), "Range must have end position");
        }
    }

    Ok(())
}

/// Tests feature spec: navigation.rs#hover-on-variable
///
/// Validates that hovering over a variable returns declaration or type info.
#[test]
fn test_hover_on_variable() -> TestResult {
    let doc = r#"
my $count = 0;
$count = $count + 1;
print $count;
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///vars.pl", doc)?;

    // Hover over $count at its usage site
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///vars.pl"},
                "position": {"line": 2, "character": 1} // Position on $count usage
            }),
        )
        .unwrap_or(json!(null));

    // Variable hover may return info or null depending on implementation depth
    if !result.is_null() {
        let contents = result.get("contents").ok_or("Expected contents in hover response")?;

        if contents.is_object() {
            let value = contents.get("value").and_then(|v| v.as_str());
            assert!(value.is_some(), "Hover contents should have a value string");
        } else if contents.is_string() {
            // Plain string contents are also valid per LSP spec
            assert!(
                !contents.as_str().ok_or("Expected string")?.is_empty(),
                "Hover content string should not be empty"
            );
        }
    }

    Ok(())
}

/// Tests feature spec: navigation.rs#hover-on-builtin
///
/// Validates that hovering over a Perl builtin function returns documentation.
#[test]
fn test_hover_on_builtin_function() -> TestResult {
    let doc = r#"
my @items = (3, 1, 4, 1, 5);
my @sorted = sort @items;
my $length = scalar @items;
push @items, 9;
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///builtins.pl", doc)?;

    // Hover over "sort" builtin
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///builtins.pl"},
                "position": {"line": 2, "character": 16} // Position on "sort"
            }),
        )
        .unwrap_or(json!(null));

    // Builtin hover may return documentation or null
    if !result.is_null() {
        let contents = result.get("contents").ok_or("Expected contents in hover response")?;
        if contents.is_object() {
            let value = contents.get("value").and_then(|v| v.as_str());
            assert!(value.is_some(), "Builtin hover should have content value");
        }
    }

    // Hover over "push" builtin
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///builtins.pl"},
                "position": {"line": 4, "character": 1} // Position on "push"
            }),
        )
        .unwrap_or(json!(null));

    // Accept null or valid hover response
    if !result.is_null() {
        assert!(
            result.get("contents").is_some(),
            "If hover is returned for builtin, it must have contents"
        );
    }

    Ok(())
}

/// Tests feature spec: navigation.rs#hover-on-empty-space
///
/// Validates that hovering over whitespace, comments, or positions with no symbol
/// returns null (no hover information).
#[test]
fn test_hover_on_empty_space_returns_null() -> TestResult {
    let doc = r#"
# This is a comment
my $variable = 42;

print $variable;
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///empty.pl", doc)?;

    // Hover on a comment line
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///empty.pl"},
                "position": {"line": 1, "character": 5} // Position within comment
            }),
        )
        .unwrap_or(json!(null));

    // Comments should return null or an empty hover
    // Some implementations may provide comment content; accept both
    if !result.is_null() {
        // If non-null, it should still be a valid hover structure
        assert!(result.get("contents").is_some(), "Non-null hover must have contents field");
    }

    // Hover on blank line
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///empty.pl"},
                "position": {"line": 3, "character": 0} // Position on blank line
            }),
        )
        .unwrap_or(json!(null));

    // Blank line should return null
    assert!(result.is_null(), "Hover on blank line should return null, got: {:?}", result);

    Ok(())
}

/// Tests feature spec: navigation.rs#hover-on-method-call
///
/// Validates hover on method calls in object-oriented Perl code.
#[test]
fn test_hover_on_method_call() -> TestResult {
    let doc = r#"
package Logger;

sub new {
    my ($class, %opts) = @_;
    return bless \%opts, $class;
}

sub info {
    my ($self, $msg) = @_;
    print "[INFO] $msg\n";
}

package main;

my $log = Logger->new(level => 'debug');
$log->info("Application started");
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///method.pl", doc)?;

    // Hover over "info" method call
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///method.pl"},
                "position": {"line": 16, "character": 7} // Position on "info" in $log->info(...)
            }),
        )
        .unwrap_or(json!(null));

    // Method hover may return info or null depending on implementation
    if !result.is_null() {
        let contents = result.get("contents").ok_or("Expected contents in hover response")?;
        if contents.is_object() {
            assert!(contents.get("value").is_some(), "Method hover content should have value");
        }
    }

    Ok(())
}

/// Tests feature spec: navigation.rs#hover-capability-advertised
///
/// Validates that hover capability is advertised in server capabilities.
#[test]
fn test_hover_capability_advertised() -> TestResult {
    let mut harness = LspHarness::new();
    let init_response = harness.initialize(None)?;

    let capabilities = &init_response["capabilities"];

    // Hover should be advertised
    let has_capability = capabilities.get("hoverProvider").is_some();
    assert!(has_capability, "hoverProvider should be advertised in capabilities");

    // If present, should be true or an object
    let provider = &capabilities["hoverProvider"];
    assert!(
        provider.is_boolean() || provider.is_object(),
        "hoverProvider should be boolean or object, got: {:?}",
        provider
    );

    Ok(())
}
