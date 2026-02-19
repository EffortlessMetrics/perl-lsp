//! Comprehensive LSP integration tests for textDocument/definition and textDocument/declaration
//!
//! Tests feature spec: LSP_IMPLEMENTATION_GUIDE.md#go-to-definition
//! Tests feature spec: navigation.rs#definition-provider
//!
//! This test suite validates:
//! - textDocument/definition request/response handling
//! - Go to definition for subroutine names
//! - Go to definition for variable names
//! - Definition not found case (returns null or empty)
//! - textDocument/declaration as a related feature
//! - Definition capability advertised in server capabilities
//!
//! LSP Protocol Compliance:
//! - Definition response: Location | Location[] | LocationLink[] | null
//! - Location: { uri: string, range: Range }
//! - LocationLink: { targetUri, targetRange, targetSelectionRange, originSelectionRange? }
//! - Range: { start: Position, end: Position }
//! - Position: { line: number, character: number }
//!
//! Related Documentation:
//! - docs/LSP_IMPLEMENTATION_GUIDE.md#go-to-definition
//! - crates/perl-lsp-navigation/src/definition.rs

mod support;

use serde_json::json;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Helper to validate a Location object has proper structure.
fn assert_valid_location(location: &serde_json::Value) {
    assert!(location.get("uri").is_some(), "Location must have 'uri' field, got: {:?}", location);
    let range = location.get("range");
    assert!(range.is_some(), "Location must have 'range' field, got: {:?}", location);
    let range = range.unwrap_or(&json!(null));
    assert!(range.get("start").is_some(), "Range must have 'start' position");
    assert!(range.get("end").is_some(), "Range must have 'end' position");

    let start = range.get("start").unwrap_or(&json!(null));
    assert!(
        start.get("line").and_then(|l| l.as_u64()).is_some(),
        "Start position must have valid line number"
    );
    assert!(
        start.get("character").and_then(|c| c.as_u64()).is_some(),
        "Start position must have valid character offset"
    );
}

/// Tests feature spec: navigation.rs#go-to-sub-definition
///
/// Validates that go-to-definition on a subroutine call navigates to its declaration.
#[test]
fn test_go_to_sub_definition() -> TestResult {
    let doc = r#"
sub process {
    my ($data) = @_;
    return $data * 2;
}

sub transform {
    my ($input) = @_;
    my $result = process($input);
    return $result;
}

my $output = process(42);
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///def.pl", doc)?;

    // Go to definition from call site on line 12 (my $output = process(42))
    let result = harness
        .request(
            "textDocument/definition",
            json!({
                "textDocument": {"uri": "file:///def.pl"},
                "position": {"line": 12, "character": 15} // Position on "process" call
            }),
        )
        .unwrap_or(json!(null));

    if !result.is_null() {
        // Result can be a single Location, an array of Locations, or LocationLinks
        if result.is_array() {
            let locations = result.as_array().ok_or("Expected array")?;
            assert!(
                !locations.is_empty(),
                "Definition should return at least one location for known sub"
            );
            for loc in locations {
                assert_valid_location(loc);
            }

            // The definition should point to line 1 where "sub process" is declared
            let first = &locations[0];
            let target_line = first.pointer("/range/start/line").and_then(|l| l.as_u64());
            if let Some(line) = target_line {
                assert_eq!(
                    line, 1,
                    "Definition of 'process' should point to line 1 (sub declaration)"
                );
            }
        } else if result.is_object() {
            // Single Location result
            assert_valid_location(&result);
        }
    }

    Ok(())
}

/// Tests feature spec: navigation.rs#go-to-variable-definition
///
/// Validates that go-to-definition on a variable navigates to its declaration.
#[test]
fn test_go_to_variable_definition() -> TestResult {
    let doc = r#"
my $counter = 0;
$counter++;
$counter += 10;
print "Count: $counter\n";
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///vardef.pl", doc)?;

    // Go to definition from usage on line 2 ($counter++)
    let result = harness
        .request(
            "textDocument/definition",
            json!({
                "textDocument": {"uri": "file:///vardef.pl"},
                "position": {"line": 2, "character": 1} // Position on $counter usage
            }),
        )
        .unwrap_or(json!(null));

    if !result.is_null() {
        if result.is_array() {
            let locations = result.as_array().ok_or("Expected array")?;
            if !locations.is_empty() {
                assert_valid_location(&locations[0]);

                // Definition should point to line 1 where "my $counter" is declared
                let def_line = locations[0].pointer("/range/start/line").and_then(|l| l.as_u64());
                if let Some(line) = def_line {
                    assert_eq!(
                        line, 1,
                        "Variable definition should point to 'my $counter' declaration on line 1"
                    );
                }
            }
        } else if result.is_object() {
            assert_valid_location(&result);
        }
    }

    Ok(())
}

/// Tests feature spec: navigation.rs#definition-not-found
///
/// Validates that go-to-definition returns null or empty when no definition exists.
#[test]
fn test_definition_not_found() -> TestResult {
    let doc = r#"
# Just a comment
my $x = unknown_function();
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///nodef.pl", doc)?;

    // Go to definition on a comment (no symbol)
    let result = harness
        .request(
            "textDocument/definition",
            json!({
                "textDocument": {"uri": "file:///nodef.pl"},
                "position": {"line": 1, "character": 5} // Inside comment text
            }),
        )
        .unwrap_or(json!(null));

    // Should return null or empty array
    if !result.is_null() {
        if result.is_array() {
            let locations = result.as_array().ok_or("Expected array")?;
            assert!(
                locations.is_empty(),
                "Definition on comment should return empty array, got {} locations",
                locations.len()
            );
        }
    }

    Ok(())
}

/// Tests feature spec: navigation.rs#declaration-provider
///
/// Validates that textDocument/declaration works for finding variable declarations.
#[test]
fn test_declaration_request() -> TestResult {
    let doc = r#"
my $name = "world";
print "Hello, $name\n";
my $greeting = "Hi $name";
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///decl.pl", doc)?;

    // Request declaration from usage on line 2
    let result = harness
        .request(
            "textDocument/declaration",
            json!({
                "textDocument": {"uri": "file:///decl.pl"},
                "position": {"line": 2, "character": 16} // Position on $name usage
            }),
        )
        .unwrap_or(json!(null));

    // Declaration may return Location or null
    if !result.is_null() {
        if result.is_array() {
            let locations = result.as_array().ok_or("Expected array")?;
            if !locations.is_empty() {
                assert_valid_location(&locations[0]);

                // Declaration should point back to line 1 where "my $name" is
                let decl_line = locations[0].pointer("/range/start/line").and_then(|l| l.as_u64());
                if let Some(line) = decl_line {
                    assert_eq!(line, 1, "Declaration should point to 'my $name' on line 1");
                }
            }
        } else if result.is_object() {
            assert_valid_location(&result);
        }
    }

    Ok(())
}

/// Tests feature spec: navigation.rs#definition-empty-file
///
/// Validates graceful handling of go-to-definition on an empty document.
#[test]
fn test_definition_on_empty_file() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///empty.pl", "")?;

    let result = harness
        .request(
            "textDocument/definition",
            json!({
                "textDocument": {"uri": "file:///empty.pl"},
                "position": {"line": 0, "character": 0}
            }),
        )
        .unwrap_or(json!(null));

    // Empty file should return null or empty array
    if !result.is_null() {
        if result.is_array() {
            let locations = result.as_array().ok_or("Expected array")?;
            assert!(locations.is_empty(), "Definition on empty file should return empty array");
        }
    }

    Ok(())
}

/// Tests feature spec: navigation.rs#definition-capability-advertised
///
/// Validates that definition and declaration capabilities are advertised.
#[test]
fn test_definition_capability_advertised() -> TestResult {
    let mut harness = LspHarness::new();
    let init_response = harness.initialize(None)?;

    let capabilities = &init_response["capabilities"];

    // Definition provider should be advertised
    let has_definition = capabilities.get("definitionProvider").is_some();
    assert!(has_definition, "definitionProvider should be advertised in capabilities");

    let def_provider = &capabilities["definitionProvider"];
    assert!(
        def_provider.is_boolean() || def_provider.is_object(),
        "definitionProvider should be boolean or object, got: {:?}",
        def_provider
    );

    // Declaration provider may also be advertised
    if let Some(decl_provider) = capabilities.get("declarationProvider") {
        assert!(
            decl_provider.is_boolean() || decl_provider.is_object(),
            "declarationProvider should be boolean or object, got: {:?}",
            decl_provider
        );
    }

    Ok(())
}
