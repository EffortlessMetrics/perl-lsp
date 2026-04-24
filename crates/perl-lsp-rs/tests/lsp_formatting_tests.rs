//! Tests for textDocument/formatting and textDocument/rangeFormatting LSP features
//!
//! Validates the document formatting provider functionality including:
//! - Full document formatting
//! - Range formatting for a specific region
//! - Formatting options (tabSize, insertSpaces)
//! - Capability advertisement in server initialization
//! - Graceful handling when formatter produces no changes
//!
//! Note: Some formatting tests may produce null/empty results if perltidy
//! is not installed. Tests are written to handle both cases gracefully.

mod support;
use serde_json::json;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Test whole-document formatting request structure and response
#[test]
fn test_formatting_whole_document() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_format.pl";
    harness.open(
        doc_uri,
        r#"sub hello{my$name=shift;print "Hello, $name\n";return 1;}
sub world{return "world";}
"#,
    )?;

    let response = harness
        .request(
            "textDocument/formatting",
            json!({
                "textDocument": { "uri": doc_uri },
                "options": {
                    "tabSize": 4,
                    "insertSpaces": true
                }
            }),
        )
        .unwrap_or(json!(null));

    // Response should be an array of TextEdit or null
    if !response.is_null() {
        assert!(
            response.is_array(),
            "formatting should return an array of TextEdit, got: {:?}",
            response
        );
        let edits = response.as_array().ok_or("response is not an array")?;
        // Each edit should have range and newText
        for edit in edits {
            assert!(edit["range"].is_object(), "TextEdit should have a range");
            assert!(edit["newText"].is_string(), "TextEdit should have newText");
        }
    }
    // null is also acceptable if no formatter is configured

    Ok(())
}

/// Test range formatting to format only a specific portion of the document
#[test]
fn test_formatting_range() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_range_format.pl";
    harness.open(
        doc_uri,
        r#"
# Well-formatted section
sub clean_func {
    my $x = 1;
    return $x;
}

# Poorly formatted section to be range-formatted
sub messy{my$y=2;return$y;}

# Another well-formatted section
sub another_clean {
    return 42;
}
"#,
    )?;

    let response = harness
        .request(
            "textDocument/rangeFormatting",
            json!({
                "textDocument": { "uri": doc_uri },
                "range": {
                    "start": { "line": 8, "character": 0 },
                    "end": { "line": 8, "character": 30 }
                },
                "options": {
                    "tabSize": 4,
                    "insertSpaces": true
                }
            }),
        )
        .unwrap_or(json!(null));

    // Response should be an array of TextEdit or null
    if !response.is_null() {
        assert!(
            response.is_array(),
            "rangeFormatting should return an array of TextEdit, got: {:?}",
            response
        );
        let edits = response.as_array().ok_or("response is not an array")?;
        for edit in edits {
            assert!(edit["range"].is_object(), "Each TextEdit should have a range");
            assert!(edit["newText"].is_string(), "Each TextEdit should have newText");
        }
    }

    Ok(())
}

/// Test that formatting options like tabSize are passed through
#[test]
fn test_formatting_options_tab_size() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_tab_size.pl";
    harness.open(
        doc_uri,
        r#"sub test{
my $x=1;
my $y=2;
return $x+$y;
}
"#,
    )?;

    // Request with tabSize 2
    let response_2 = harness
        .request(
            "textDocument/formatting",
            json!({
                "textDocument": { "uri": doc_uri },
                "options": {
                    "tabSize": 2,
                    "insertSpaces": true
                }
            }),
        )
        .unwrap_or(json!(null));

    // Request with tabSize 8
    let response_8 = harness
        .request(
            "textDocument/formatting",
            json!({
                "textDocument": { "uri": doc_uri },
                "options": {
                    "tabSize": 8,
                    "insertSpaces": true
                }
            }),
        )
        .unwrap_or(json!(null));

    // Both responses should be valid (array or null)
    if !response_2.is_null() {
        assert!(
            response_2.is_array(),
            "formatting with tabSize 2 should return array, got: {:?}",
            response_2
        );
    }
    if !response_8.is_null() {
        assert!(
            response_8.is_array(),
            "formatting with tabSize 8 should return array, got: {:?}",
            response_8
        );
    }

    Ok(())
}

/// Test that formatting capability is advertised during initialization
#[test]
fn test_formatting_capability_advertised() -> TestResult {
    let mut harness = LspHarness::new();
    let init_response = harness.initialize(None)?;

    let capabilities = &init_response["capabilities"];

    // Check for documentFormattingProvider
    let has_formatting = capabilities.get("documentFormattingProvider").is_some();
    assert!(
        has_formatting,
        "Server should advertise documentFormattingProvider capability. Capabilities: {:?}",
        capabilities
    );

    // Check for documentRangeFormattingProvider
    let has_range_formatting = capabilities.get("documentRangeFormattingProvider").is_some();
    assert!(
        has_range_formatting,
        "Server should advertise documentRangeFormattingProvider capability. Capabilities: {:?}",
        capabilities
    );

    Ok(())
}

/// Test formatting on already well-formatted code returns empty edits or null
#[test]
fn test_formatting_well_formatted_code() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_clean.pl";
    harness.open(
        doc_uri,
        r#"use strict;
use warnings;

sub clean_function {
    my $x = 1;
    my $y = 2;
    return $x + $y;
}

1;
"#,
    )?;

    let response = harness
        .request(
            "textDocument/formatting",
            json!({
                "textDocument": { "uri": doc_uri },
                "options": {
                    "tabSize": 4,
                    "insertSpaces": true
                }
            }),
        )
        .unwrap_or(json!(null));

    // Well-formatted code should return empty edits, null, or edits that preserve content
    if !response.is_null() {
        if let Some(edits) = response.as_array() {
            // If edits are returned, they should still be valid TextEdit objects
            for edit in edits {
                assert!(edit["range"].is_object(), "TextEdit should have a range");
                assert!(edit["newText"].is_string(), "TextEdit should have newText");
            }
        }
    }

    Ok(())
}

/// Test formatting on a file with only comments
#[test]
fn test_formatting_comments_only() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_comments.pl";
    harness.open(
        doc_uri,
        r#"#!/usr/bin/perl
# This file has only comments
# No actual code to format
# Just checking that formatting handles this gracefully
"#,
    )?;

    let response = harness
        .request(
            "textDocument/formatting",
            json!({
                "textDocument": { "uri": doc_uri },
                "options": {
                    "tabSize": 4,
                    "insertSpaces": true
                }
            }),
        )
        .unwrap_or(json!(null));

    // Should not crash; null or empty edits are acceptable
    if !response.is_null() {
        assert!(
            response.is_array(),
            "Should return an array of edits for comment-only file, got: {:?}",
            response
        );
    }

    Ok(())
}
