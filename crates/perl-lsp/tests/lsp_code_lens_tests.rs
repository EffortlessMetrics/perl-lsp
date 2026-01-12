//! Comprehensive LSP integration tests for code_lens feature
//!
//! Tests feature spec: code_lens_provider.rs - Code Lens provider implementation
//!
//! This test suite validates:
//! - Basic codeLens request for subroutine definitions
//! - codeLens for package declarations
//! - codeLensResolve handler
//! - Edge cases: Unicode identifiers, CRLF line endings, large files
//! - Empty files and files with no lenses
//! - Test subroutine detection and "Run Test" lens
//! - Reference counting and lens resolution
//!
//! LSP Protocol Compliance:
//! - textDocument/codeLens request/response handling
//! - codeLens/resolve request/response handling
//! - CodeLens data structure validation
//! - Command structure validation
//!
//! Related Documentation:
//! - docs/LSP_IMPLEMENTATION_GUIDE.md#code-lens
//! - crates/perl-parser/src/code_lens_provider.rs

mod support;

use serde_json::json;
use support::lsp_harness::LspHarness;

/// Tests feature spec: code_lens_provider.rs#basic-code-lens-extraction
///
/// Validates that basic codeLens requests return appropriate lenses for
/// subroutine definitions with reference counting data.
#[test]
fn test_basic_code_lens_for_subroutines() {
    let doc = r#"
package MyModule;

sub add {
    my ($x, $y) = @_;
    return $x + $y;
}

sub subtract {
    my ($x, $y) = @_;
    return $x - $y;
}

my $sum = add(5, 3);
my $diff = subtract(10, 4);
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None).unwrap();
    harness.open_document("file:///test.pl", doc).unwrap();

    let result = harness
        .request(
            "textDocument/codeLens",
            json!({
                "textDocument": {"uri": "file:///test.pl"}
            }),
        )
        .unwrap_or(json!(null));

    assert!(result.is_array(), "codeLens should return an array of lenses");

    let lenses = result.as_array().unwrap();
    assert!(!lenses.is_empty(), "Should have code lenses for subroutines and package");

    // Verify structure of returned lenses
    for lens in lenses {
        assert!(lens.get("range").is_some(), "Each lens must have a range");
        let range = lens.get("range").unwrap();
        assert!(
            range.get("start").is_some() && range.get("end").is_some(),
            "Range must have start and end positions"
        );

        // Either command or data should be present
        let has_command = lens.get("command").is_some();
        let has_data = lens.get("data").is_some();
        assert!(has_command || has_data, "Lens should have either command or data for resolution");
    }
}

/// Tests feature spec: code_lens_provider.rs#package-references
///
/// Validates that package declarations receive reference counting lenses.
#[test]
fn test_code_lens_for_package_declarations() {
    let doc = r#"
package MyPackage;

use strict;
use warnings;

sub new {
    my $class = shift;
    return bless {}, $class;
}

sub method {
    my ($self, $value) = @_;
    return $value * 2;
}

1;
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None).unwrap();
    harness.open_document("file:///MyPackage.pm", doc).unwrap();

    let result = harness
        .request(
            "textDocument/codeLens",
            json!({
                "textDocument": {"uri": "file:///MyPackage.pm"}
            }),
        )
        .unwrap_or(json!(null));

    let lenses = result.as_array().unwrap();

    // Should have lenses for package, new, and method
    assert!(lenses.len() >= 3, "Should have lenses for package and subroutines");

    // Find the package lens
    let package_lens = lenses.iter().find(|lens| {
        lens.get("data")
            .and_then(|d| d.get("kind"))
            .and_then(|k| k.as_str())
            .map(|k| k == "package")
            .unwrap_or(false)
    });

    assert!(package_lens.is_some(), "Should have a lens for the package declaration");
}

/// Tests feature spec: code_lens_provider.rs#test-subroutine-detection
///
/// Validates that test subroutines receive "Run Test" code lenses.
#[test]
fn test_run_test_lens_for_test_subroutines() {
    let doc = r#"
use Test::More;

sub test_addition {
    my $result = 2 + 2;
    is($result, 4, "2 + 2 equals 4");
}

sub test_subtraction {
    my $result = 5 - 3;
    is($result, 2, "5 - 3 equals 2");
}

sub helper_function {
    return 42;
}

sub t_multiplication {
    ok(2 * 3 == 6, "multiplication works");
}

done_testing();
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None).unwrap();
    harness.open_document("file:///test.t", doc).unwrap();

    let result = harness
        .request(
            "textDocument/codeLens",
            json!({
                "textDocument": {"uri": "file:///test.t"}
            }),
        )
        .unwrap_or(json!(null));

    let lenses = result.as_array().unwrap();

    // Find Run Test lenses
    let run_test_lenses: Vec<_> = lenses
        .iter()
        .filter(|lens| {
            lens.get("command")
                .and_then(|c| c.get("title"))
                .and_then(|t| t.as_str())
                .map(|t| t.contains("Run Test"))
                .unwrap_or(false)
        })
        .collect();

    assert!(
        run_test_lenses.len() >= 3,
        "Should have Run Test lenses for test_addition, test_subtraction, and t_multiplication"
    );

    // Verify command structure
    for lens in &run_test_lenses {
        let command = lens.get("command").unwrap();
        assert_eq!(
            command.get("command").and_then(|c| c.as_str()),
            Some("perl.runTest"),
            "Run Test lens should have perl.runTest command"
        );
        assert!(command.get("arguments").is_some(), "Run Test command should have arguments");
    }
}

/// Tests feature spec: code_lens_provider.rs#lens-resolution
///
/// Validates codeLens/resolve handler correctly resolves reference counting.
#[test]
fn test_code_lens_resolve_with_reference_count() {
    let doc = r#"
sub calculate {
    my ($x, $y) = @_;
    return $x * $y;
}

my $a = calculate(2, 3);
my $b = calculate(4, 5);
my $c = calculate(6, 7);
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None).unwrap();
    harness.open_document("file:///math.pl", doc).unwrap();

    // Get code lenses
    let lenses_result = harness
        .request(
            "textDocument/codeLens",
            json!({
                "textDocument": {"uri": "file:///math.pl"}
            }),
        )
        .unwrap_or(json!(null));

    let lenses = lenses_result.as_array().unwrap();

    // Find unresolved reference lens (has data, no command)
    let unresolved_lens =
        lenses.iter().find(|lens| lens.get("data").is_some() && lens.get("command").is_none());

    assert!(unresolved_lens.is_some(), "Should have at least one unresolved reference lens");

    // Resolve the lens
    let resolved = harness
        .request("codeLens/resolve", unresolved_lens.unwrap().clone())
        .unwrap_or(json!(null));

    // After resolution, should have a command
    assert!(resolved.get("command").is_some(), "Resolved lens should have a command");

    let command = resolved.get("command").unwrap();
    let title = command.get("title").and_then(|t| t.as_str()).unwrap_or("");

    // Should contain reference count
    assert!(
        title.contains("reference"),
        "Resolved lens title should mention references, got: {}",
        title
    );
    assert_eq!(
        command.get("command").and_then(|c| c.as_str()),
        Some("editor.action.findReferences"),
        "Reference lens should use findReferences command"
    );
}

/// Tests feature spec: code_lens_provider.rs#zero-references
///
/// Validates that unused functions show "0 references" lens.
#[test]
fn test_code_lens_resolve_zero_references() {
    let doc = r#"
sub used_function {
    return 1;
}

sub unused_function {
    return 2;
}

my $value = used_function();
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None).unwrap();
    harness.open_document("file:///unused.pl", doc).unwrap();

    let lenses_result = harness
        .request(
            "textDocument/codeLens",
            json!({
                "textDocument": {"uri": "file:///unused.pl"}
            }),
        )
        .unwrap_or(json!(null));

    let lenses = lenses_result.as_array().unwrap();

    // Find lens for unused_function
    let unused_lens = lenses.iter().find(|lens| {
        lens.get("data")
            .and_then(|d| d.get("name"))
            .and_then(|n| n.as_str())
            .map(|n| n == "unused_function")
            .unwrap_or(false)
    });

    assert!(unused_lens.is_some(), "Should have lens for unused_function");

    // Resolve it
    let resolved =
        harness.request("codeLens/resolve", unused_lens.unwrap().clone()).unwrap_or(json!(null));

    let title =
        resolved.get("command").and_then(|c| c.get("title")).and_then(|t| t.as_str()).unwrap_or("");

    assert!(
        title.contains("0 reference"),
        "Unused function should show 0 references, got: {}",
        title
    );
}

/// Tests feature spec: code_lens_provider.rs#unicode-identifiers
///
/// Validates code lens handling with Unicode function and package names.
#[test]
fn test_code_lens_unicode_identifiers() {
    let doc = r#"
package Café;

sub 你好 {
    my $π = 3.14159;
    return $π;
}

sub Σ {
    my @numbers = @_;
    my $sum = 0;
    $sum += $_ for @numbers;
    return $sum;
}

my $value = 你好();
my $total = Σ(1, 2, 3);
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None).unwrap();
    harness.open_document("file:///unicode.pl", doc).unwrap();

    let result = harness
        .request(
            "textDocument/codeLens",
            json!({
                "textDocument": {"uri": "file:///unicode.pl"}
            }),
        )
        .unwrap_or(json!(null));

    assert!(result.is_array(), "Should successfully return lenses for Unicode identifiers");

    let lenses = result.as_array().unwrap();
    assert!(!lenses.is_empty(), "Should have code lenses for Unicode identifiers");

    // Verify all lenses have valid ranges
    for lens in lenses {
        let range = lens.get("range").unwrap();
        let start = range.get("start").unwrap();
        let end = range.get("end").unwrap();

        assert!(
            start.get("line").and_then(|l| l.as_u64()).is_some(),
            "Start position should have valid line number"
        );
        assert!(
            start.get("character").and_then(|c| c.as_u64()).is_some(),
            "Start position should have valid character offset"
        );
        assert!(
            end.get("line").and_then(|l| l.as_u64()).is_some(),
            "End position should have valid line number"
        );
        assert!(
            end.get("character").and_then(|c| c.as_u64()).is_some(),
            "End position should have valid character offset"
        );
    }
}

/// Tests feature spec: code_lens_provider.rs#crlf-handling
///
/// Validates code lens position calculation with CRLF line endings.
#[test]
fn test_code_lens_crlf_line_endings() {
    let doc = "package TestPackage;\r\n\r\nsub function_one {\r\n    return 1;\r\n}\r\n\r\nsub function_two {\r\n    return 2;\r\n}\r\n\r\nmy $a = function_one();\r\nmy $b = function_two();\r\n";

    let mut harness = LspHarness::new();
    harness.initialize(None).unwrap();
    harness.open_document("file:///crlf.pl", doc).unwrap();

    let result = harness
        .request(
            "textDocument/codeLens",
            json!({
                "textDocument": {"uri": "file:///crlf.pl"}
            }),
        )
        .unwrap_or(json!(null));

    assert!(result.is_array(), "Should handle CRLF line endings correctly");

    let lenses = result.as_array().unwrap();
    assert!(!lenses.is_empty(), "Should have code lenses with CRLF endings");

    // Verify positions are reasonable (no negative or extreme values)
    for lens in lenses {
        let range = lens.get("range").unwrap();
        let start_line = range
            .get("start")
            .and_then(|s| s.get("line"))
            .and_then(|l| l.as_u64())
            .unwrap_or(u64::MAX);
        let start_char = range
            .get("start")
            .and_then(|s| s.get("character"))
            .and_then(|c| c.as_u64())
            .unwrap_or(u64::MAX);

        assert!(start_line < 100, "Line number should be reasonable: {}", start_line);
        assert!(start_char < 1000, "Character offset should be reasonable: {}", start_char);
    }
}

/// Tests feature spec: code_lens_provider.rs#empty-file
///
/// Validates graceful handling of empty files.
#[test]
fn test_code_lens_empty_file() {
    let mut harness = LspHarness::new();
    harness.initialize(None).unwrap();
    harness.open_document("file:///empty.pl", "").unwrap();

    let result = harness
        .request(
            "textDocument/codeLens",
            json!({
                "textDocument": {"uri": "file:///empty.pl"}
            }),
        )
        .unwrap_or(json!(null));

    // Should return empty array for empty file
    assert!(result.is_array(), "Empty file should return an array");
    assert_eq!(result.as_array().unwrap().len(), 0, "Empty file should have no code lenses");
}

/// Tests feature spec: code_lens_provider.rs#no-lensable-items
///
/// Validates files with no subroutines or packages return empty lens array.
#[test]
fn test_code_lens_file_with_no_lenses() {
    let doc = r#"
# Just comments and simple statements
use strict;
use warnings;

my $var = 42;
my @array = (1, 2, 3);
my %hash = (a => 1, b => 2);

print "Hello, World\n";
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None).unwrap();
    harness.open_document("file:///simple.pl", doc).unwrap();

    let result = harness
        .request(
            "textDocument/codeLens",
            json!({
                "textDocument": {"uri": "file:///simple.pl"}
            }),
        )
        .unwrap_or(json!(null));

    assert!(result.is_array(), "Should return array even with no lenses");

    // May have 0 lenses or minimal lenses depending on implementation
    let lenses = result.as_array().unwrap();
    // This is acceptable - no subroutines means no lenses
    assert!(lenses.len() < 2, "File with no subroutines should have minimal lenses");
}

/// Tests feature spec: code_lens_provider.rs#large-file-handling
///
/// Validates performance with large files containing many subroutines.
#[test]
fn test_code_lens_large_file() {
    // Generate a large file with many subroutines
    let mut doc = String::from("package LargeModule;\n\n");
    for i in 0..100 {
        doc.push_str(&format!("sub function_{} {{\n    return {};\n}}\n\n", i, i));
    }
    doc.push_str("1;\n");

    let mut harness = LspHarness::new();
    harness.initialize(None).unwrap();
    harness.open_document("file:///large.pm", &doc).unwrap();

    let start = std::time::Instant::now();
    let result = harness
        .request(
            "textDocument/codeLens",
            json!({
                "textDocument": {"uri": "file:///large.pm"}
            }),
        )
        .unwrap_or(json!(null));
    let elapsed = start.elapsed();

    assert!(result.is_array(), "Should handle large files");

    let lenses = result.as_array().unwrap();
    assert!(lenses.len() >= 100, "Should have lenses for all 100+ functions");

    // Performance check - should complete in reasonable time
    assert!(
        elapsed.as_secs() < 5,
        "Large file code lens should complete within 5 seconds, took {:?}",
        elapsed
    );
}

/// Tests feature spec: code_lens_provider.rs#shebang-lens
///
/// Validates "Run Script" lens for files with shebang.
#[test]
fn test_code_lens_shebang_run_script() {
    let doc = r#"#!/usr/bin/perl

use strict;
use warnings;

print "Hello from script\n";

sub helper {
    return 42;
}
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None).unwrap();
    harness.open_document("file:///script.pl", doc).unwrap();

    let result = harness
        .request(
            "textDocument/codeLens",
            json!({
                "textDocument": {"uri": "file:///script.pl"}
            }),
        )
        .unwrap_or(json!(null));

    let lenses = result.as_array().unwrap();

    // Look for Run Script lens
    let run_script_lens = lenses.iter().find(|lens| {
        lens.get("command")
            .and_then(|c| c.get("title"))
            .and_then(|t| t.as_str())
            .map(|t| t.contains("Run Script"))
            .unwrap_or(false)
    });

    assert!(run_script_lens.is_some(), "Script with shebang should have 'Run Script' lens");

    if let Some(lens) = run_script_lens {
        let command = lens.get("command").unwrap();
        assert_eq!(
            command.get("command").and_then(|c| c.as_str()),
            Some("perl.runScript"),
            "Run Script lens should use perl.runScript command"
        );
    }
}

/// Tests feature spec: code_lens_provider.rs#multiple-packages
///
/// Validates code lens handling for files with multiple package declarations.
#[test]
fn test_code_lens_multiple_packages() {
    let doc = r#"
package Package::One;

sub method_one {
    return 1;
}

package Package::Two;

sub method_two {
    return 2;
}

package Package::Three;

sub method_three {
    return 3;
}

1;
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None).unwrap();
    harness.open_document("file:///multi.pm", doc).unwrap();

    let result = harness
        .request(
            "textDocument/codeLens",
            json!({
                "textDocument": {"uri": "file:///multi.pm"}
            }),
        )
        .unwrap_or(json!(null));

    let lenses = result.as_array().unwrap();

    // Should have lenses for 3 packages and 3 methods (at least 6 lenses)
    assert!(lenses.len() >= 6, "Should have lenses for multiple packages and their methods");

    // Count package lenses
    let package_lenses: Vec<_> = lenses
        .iter()
        .filter(|lens| {
            lens.get("data")
                .and_then(|d| d.get("kind"))
                .and_then(|k| k.as_str())
                .map(|k| k == "package")
                .unwrap_or(false)
        })
        .collect();

    assert_eq!(package_lenses.len(), 3, "Should have exactly 3 package lenses");
}

/// Tests feature spec: code_lens_provider.rs#nested-subroutines
///
/// Validates code lens for nested subroutine declarations.
#[test]
fn test_code_lens_nested_subroutines() {
    let doc = r#"
sub outer {
    my $x = 10;

    my $inner = sub {
        return $x * 2;
    };

    return $inner->();
}

my $result = outer();
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None).unwrap();
    harness.open_document("file:///nested.pl", doc).unwrap();

    let result = harness
        .request(
            "textDocument/codeLens",
            json!({
                "textDocument": {"uri": "file:///nested.pl"}
            }),
        )
        .unwrap_or(json!(null));

    let lenses = result.as_array().unwrap();

    // Should at least have lens for outer subroutine
    assert!(!lenses.is_empty(), "Should have lenses for nested subroutines");
}

/// Tests feature spec: code_lens_provider.rs#malformed-code
///
/// Validates graceful degradation with syntax errors.
#[test]
fn test_code_lens_malformed_code() {
    let doc = r#"
sub broken {
    my ($x = @_;  # Missing closing paren
    return $x;

sub also_broken  # Missing opening brace
    return 42;
}

sub valid_function {
    return 1;
}
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None).unwrap();
    harness.open_document("file:///broken.pl", doc).unwrap();

    let result = harness
        .request(
            "textDocument/codeLens",
            json!({
                "textDocument": {"uri": "file:///broken.pl"}
            }),
        )
        .unwrap_or(json!(null));

    // Should not crash, return array even if partially parsed
    assert!(result.is_array(), "Should handle malformed code gracefully");
}

/// Tests feature spec: code_lens_provider.rs#position-accuracy
///
/// Validates that lens positions accurately correspond to declarations.
#[test]
fn test_code_lens_position_accuracy() {
    let doc = r#"package TestPackage;

sub first_function {
    return 1;
}

sub second_function {
    return 2;
}
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None).unwrap();
    harness.open_document("file:///positions.pl", doc).unwrap();

    let result = harness
        .request(
            "textDocument/codeLens",
            json!({
                "textDocument": {"uri": "file:///positions.pl"}
            }),
        )
        .unwrap_or(json!(null));

    let lenses = result.as_array().unwrap();

    // Find lens for first_function
    let first_lens = lenses.iter().find(|lens| {
        lens.get("data")
            .and_then(|d| d.get("name"))
            .and_then(|n| n.as_str())
            .map(|n| n == "first_function")
            .unwrap_or(false)
    });

    assert!(first_lens.is_some(), "Should find lens for first_function");

    if let Some(lens) = first_lens {
        let start_line = lens
            .get("range")
            .and_then(|r| r.get("start"))
            .and_then(|s| s.get("line"))
            .and_then(|l| l.as_u64())
            .unwrap_or(u64::MAX);

        // first_function is on line 2 (0-indexed)
        assert!(start_line == 2, "first_function lens should be on line 2, got {}", start_line);
    }
}

/// Tests feature spec: code_lens_provider.rs#test-naming-patterns
///
/// Validates detection of various test naming patterns.
#[test]
fn test_code_lens_test_naming_patterns() {
    let doc = r#"
sub test_basic { return 1; }
sub function_test { return 2; }
sub t_another { return 3; }
sub test { return 4; }
sub ok_validation { return 5; }
sub is_equal { return 6; }
sub like_pattern { return 7; }
sub can_execute { return 8; }
sub regular_function { return 9; }
"#;

    let mut harness = LspHarness::new();
    harness.initialize(None).unwrap();
    harness.open_document("file:///patterns.t", doc).unwrap();

    let result = harness
        .request(
            "textDocument/codeLens",
            json!({
                "textDocument": {"uri": "file:///patterns.t"}
            }),
        )
        .unwrap_or(json!(null));

    let lenses = result.as_array().unwrap();

    // Count Run Test lenses
    let run_test_lenses: Vec<_> = lenses
        .iter()
        .filter(|lens| {
            lens.get("command")
                .and_then(|c| c.get("title"))
                .and_then(|t| t.as_str())
                .map(|t| t.contains("Run Test"))
                .unwrap_or(false)
        })
        .collect();

    // Should detect: test_basic, function_test, t_another, test, ok_validation,
    // is_equal, like_pattern, can_execute (8 test functions)
    assert!(
        run_test_lenses.len() >= 8,
        "Should detect all test naming patterns, found {}",
        run_test_lenses.len()
    );
}
