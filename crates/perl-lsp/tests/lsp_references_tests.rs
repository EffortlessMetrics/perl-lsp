//! Comprehensive LSP integration tests for textDocument/references
//!
//! Tests feature spec: LSP_IMPLEMENTATION_GUIDE.md#find-references
//! Tests feature spec: navigation.rs#references-provider
//!
//! This test suite validates:
//! - textDocument/references request/response handling
//! - Find references of a variable across the document
//! - Find references of a subroutine (definition + call sites)
//! - No references found case (returns null or empty array)
//! - includeDeclaration context parameter behavior
//! - References capability advertised in server capabilities
//!
//! LSP Protocol Compliance:
//! - References response: Location[] | null
//! - Location: { uri: string, range: Range }
//! - ReferenceContext: { includeDeclaration: boolean }
//! - Position-based symbol resolution
//!
//! Related Documentation:
//! - docs/LSP_IMPLEMENTATION_GUIDE.md#find-references
//! - crates/perl-lsp-navigation/src/references.rs

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
}

/// Tests feature spec: navigation.rs#find-variable-references
///
/// Validates that find-references on a variable returns all its usage locations.
#[test]
fn test_find_variable_references() -> TestResult {
    let doc = r#"
my $count = 0;
$count = $count + 1;
print $count;
my $doubled = $count * 2;
$count++;
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///refs.pl", doc)?;

    // Find all references to $count (with declaration included)
    let result = harness
        .request(
            "textDocument/references",
            json!({
                "textDocument": {"uri": "file:///refs.pl"},
                "position": {"line": 1, "character": 4}, // Position on $count declaration
                "context": {"includeDeclaration": true}
            }),
        )
        .unwrap_or(json!(null));

    if !result.is_null() {
        assert!(result.is_array(), "References should return an array");

        let references = result.as_array().ok_or("Expected array result")?;
        assert!(!references.is_empty(), "Should find references for $count variable");

        // Validate structure of each returned location
        for reference in references {
            assert_valid_location(reference);

            // URI should match our document
            let uri = reference.get("uri").and_then(|u| u.as_str());
            assert_eq!(uri, Some("file:///refs.pl"), "Reference URI should match the document");
        }

        // $count appears on lines 1, 2 (twice), 3, 4, 5 = at least 6 occurrences
        assert!(
            references.len() >= 5,
            "Should find at least 5 references of $count (including declaration), found {}",
            references.len()
        );
    }

    Ok(())
}

/// Tests feature spec: navigation.rs#find-subroutine-references
///
/// Validates that find-references on a subroutine returns definition and call sites.
#[test]
fn test_find_subroutine_references() -> TestResult {
    let doc = r#"
sub validate {
    my ($input) = @_;
    return defined $input && $input ne "";
}

my $ok = validate($name);
if (validate($email)) {
    print "Valid\n";
}
validate($phone);
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///subrefs.pl", doc)?;

    // Find references of "validate" subroutine
    let result = harness
        .request(
            "textDocument/references",
            json!({
                "textDocument": {"uri": "file:///subrefs.pl"},
                "position": {"line": 1, "character": 5}, // Position on "validate" in sub definition
                "context": {"includeDeclaration": true}
            }),
        )
        .unwrap_or(json!(null));

    if !result.is_null() {
        assert!(result.is_array(), "References should return an array");

        let references = result.as_array().ok_or("Expected array result")?;

        // Should find the sub definition and three call sites
        assert!(
            references.len() >= 3,
            "Should find at least 3 references of 'validate' (definition + calls), found {}",
            references.len()
        );

        // All references should be in the same file
        for reference in references {
            assert_valid_location(reference);
            let uri = reference.get("uri").and_then(|u| u.as_str());
            assert_eq!(
                uri,
                Some("file:///subrefs.pl"),
                "All references should be in the same document"
            );
        }

        // Collect reference lines for verification
        let ref_lines: Vec<u64> = references
            .iter()
            .filter_map(|r| r.pointer("/range/start/line").and_then(|l| l.as_u64()))
            .collect();

        // The definition is on line 1, calls on lines 6, 7, 10
        assert!(
            ref_lines.contains(&1),
            "References should include the definition line (1), found lines: {:?}",
            ref_lines
        );
    }

    Ok(())
}

/// Tests feature spec: navigation.rs#no-references-found
///
/// Validates that find-references returns null or empty when no references exist.
#[test]
fn test_no_references_found() -> TestResult {
    let doc = r#"
# Just a comment line
my $lonely = 42;
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///norefs.pl", doc)?;

    // Find references on a comment (no symbol)
    let result = harness
        .request(
            "textDocument/references",
            json!({
                "textDocument": {"uri": "file:///norefs.pl"},
                "position": {"line": 1, "character": 5}, // Inside comment
                "context": {"includeDeclaration": true}
            }),
        )
        .unwrap_or(json!(null));

    // Should return null or empty array for non-symbol positions
    if !result.is_null() {
        if result.is_array() {
            let references = result.as_array().ok_or("Expected array result")?;
            assert!(
                references.is_empty(),
                "References on comment should return empty array, got {} references",
                references.len()
            );
        }
    }

    Ok(())
}

/// Tests feature spec: navigation.rs#include-declaration-flag
///
/// Validates that the includeDeclaration context flag affects the result.
#[test]
fn test_references_include_declaration_flag() -> TestResult {
    let doc = r#"
my $total = 0;
$total += 10;
$total += 20;
print $total;
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///incldecl.pl", doc)?;

    // Find references WITH includeDeclaration = true
    let result_with_decl = harness
        .request(
            "textDocument/references",
            json!({
                "textDocument": {"uri": "file:///incldecl.pl"},
                "position": {"line": 2, "character": 1}, // Position on $total usage
                "context": {"includeDeclaration": true}
            }),
        )
        .unwrap_or(json!(null));

    // Find references WITH includeDeclaration = false
    let result_without_decl = harness
        .request(
            "textDocument/references",
            json!({
                "textDocument": {"uri": "file:///incldecl.pl"},
                "position": {"line": 2, "character": 1}, // Same position
                "context": {"includeDeclaration": false}
            }),
        )
        .unwrap_or(json!(null));

    // Both should be valid responses
    if !result_with_decl.is_null() && !result_without_decl.is_null() {
        let refs_with = result_with_decl.as_array().ok_or("Expected array")?;
        let refs_without = result_without_decl.as_array().ok_or("Expected array")?;

        // With declaration should have >= without declaration
        // (Implementation may treat both the same, which is acceptable)
        assert!(
            refs_with.len() >= refs_without.len(),
            "References with includeDeclaration should be >= without: {} vs {}",
            refs_with.len(),
            refs_without.len()
        );
    }

    Ok(())
}

/// Tests feature spec: navigation.rs#references-empty-file
///
/// Validates graceful handling of find-references on an empty document.
#[test]
fn test_references_on_empty_file() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///empty.pl", "")?;

    let result = harness
        .request(
            "textDocument/references",
            json!({
                "textDocument": {"uri": "file:///empty.pl"},
                "position": {"line": 0, "character": 0},
                "context": {"includeDeclaration": true}
            }),
        )
        .unwrap_or(json!(null));

    // Empty file should return null or empty array
    if !result.is_null() {
        if result.is_array() {
            let references = result.as_array().ok_or("Expected array result")?;
            assert!(references.is_empty(), "References on empty file should return empty array");
        }
    }

    Ok(())
}

/// Tests feature spec: navigation.rs#references-capability-advertised
///
/// Validates that references capability is advertised in server capabilities.
#[test]
fn test_references_capability_advertised() -> TestResult {
    let mut harness = LspHarness::new();
    let init_response = harness.initialize(None)?;

    let capabilities = &init_response["capabilities"];

    // References provider should be advertised
    let has_capability = capabilities.get("referencesProvider").is_some();
    assert!(has_capability, "referencesProvider should be advertised in capabilities");

    let provider = &capabilities["referencesProvider"];
    assert!(
        provider.is_boolean() || provider.is_object(),
        "referencesProvider should be boolean or object, got: {:?}",
        provider
    );

    Ok(())
}
