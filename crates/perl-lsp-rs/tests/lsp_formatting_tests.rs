//! Tests for textDocument/formatting and textDocument/rangeFormatting LSP features
//!
//! Validates the document formatting provider functionality including:
//! - Full document formatting
//! - Range formatting for a specific region
//! - Formatting options (tabSize, insertSpaces)
//! - Capability advertisement in server initialization
//! - Graceful handling when formatter produces no changes

mod support;
use serde_json::json;
use support::lsp_harness::LspHarness;
use support::test_helpers::apply_text_edits;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Test whole-document formatting request structure and response
#[test]
fn test_formatting_whole_document() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_format.pl";
    harness.open(
        doc_uri,
        r#"sub hello{my$name=shift;return 1;}
sub world{return "world";}
"#,
    )?;

    let response = harness.request(
        "textDocument/formatting",
        json!({
            "textDocument": { "uri": doc_uri },
            "options": {
                "tabSize": 4,
                "insertSpaces": true
            }
        }),
    )?;

    let edits = response.as_array().ok_or("formatting response must be an array")?;
    assert_eq!(edits.len(), 1, "native formatting should produce one document edit");
    let formatted = apply_text_edits(
        r#"sub hello{my$name=shift;return 1;}
sub world{return "world";}
"#,
        edits,
    );
    assert_eq!(
        formatted,
        concat!(
            "sub hello {\n",
            "    my $name = shift;\n",
            "    return 1;\n",
            "}\n",
            "sub world {\n",
            "    return \"world\";\n",
            "}\n",
            "\n",
        )
    );

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

    let response = harness.request(
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
    )?;

    let edits = response.as_array().ok_or("rangeFormatting response must be an array")?;
    assert_eq!(edits.len(), 1, "native range formatting should edit the selected line");
    let formatted = apply_text_edits(
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
        edits,
    );
    assert_eq!(
        formatted,
        r#"
# Well-formatted section
sub clean_func {
    my $x = 1;
    return $x;
}

# Poorly formatted section to be range-formatted
sub messy {
    my $y = 2;
    return $y;
}

# Another well-formatted section
sub another_clean {
    return 42;
}
"#
    );

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
        r#"sub test{my$x=1;return$x;}
"#,
    )?;

    let source = r#"sub test{my$x=1;return$x;}
"#;

    let response_2 = harness.request(
        "textDocument/formatting",
        json!({
            "textDocument": { "uri": doc_uri },
            "options": {
                "tabSize": 2,
                "insertSpaces": true
            }
        }),
    )?;

    harness.open("file:///test_tab_size_8.pl", source)?;

    let response_8 = harness.request(
        "textDocument/formatting",
        json!({
            "textDocument": { "uri": "file:///test_tab_size_8.pl" },
            "options": {
                "tabSize": 8,
                "insertSpaces": true
            }
        }),
    )?;

    let edits_2 = response_2.as_array().ok_or("tabSize 2 response must be an array")?;
    let edits_8 = response_8.as_array().ok_or("tabSize 8 response must be an array")?;
    let formatted_2 = apply_text_edits(source, edits_2);
    let formatted_8 = apply_text_edits(source, edits_8);
    assert!(
        formatted_2.contains("\n  my $x = 1;"),
        "tabSize 2 should use two-space indentation, got:\n{formatted_2}"
    );
    assert!(
        formatted_8.contains("\n        my $x = 1;"),
        "tabSize 8 should use eight-space indentation, got:\n{formatted_8}"
    );

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

/// Test formatting on already well-formatted code returns empty edits.
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

    let response = harness.request(
        "textDocument/formatting",
        json!({
            "textDocument": { "uri": doc_uri },
            "options": {
                "tabSize": 4,
                "insertSpaces": true
            }
        }),
    )?;

    let edits = response.as_array().ok_or("formatting response must be an array")?;
    assert!(edits.is_empty(), "well-formatted native code should not be edited");

    Ok(())
}

/// Test that formatting requests return no edits when formatting is disabled at runtime.
#[test]
fn test_formatting_disabled_via_configuration_returns_no_edits() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    harness.notify(
        "workspace/didChangeConfiguration",
        json!({
            "settings": {
                "perl": {
                    "formatting": {
                        "enabled": false
                    }
                }
            }
        }),
    );

    let doc_uri = "file:///test_formatting_disabled.pl";
    harness.open(doc_uri, "sub messy{my$x=1;return$x;}\n")?;

    let response = harness.request(
        "textDocument/formatting",
        json!({
            "textDocument": { "uri": doc_uri },
            "options": {
                "tabSize": 4,
                "insertSpaces": true
            }
        }),
    )?;

    let edits = response.as_array().ok_or("formatting response must be an array")?;
    assert!(edits.is_empty(), "expected no formatting edits when formatting is disabled");

    let range_response = harness.request(
        "textDocument/rangeFormatting",
        json!({
            "textDocument": { "uri": doc_uri },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 0, "character": 26 }
            },
            "options": {
                "tabSize": 4,
                "insertSpaces": true
            }
        }),
    )?;
    let range_edits =
        range_response.as_array().ok_or("rangeFormatting response must be an array")?;
    assert!(
        range_edits.is_empty(),
        "expected no range formatting edits when formatting is disabled"
    );

    Ok(())
}

/// Test that configured native formatter policy fields route through LSP document formatting.
#[test]
fn test_native_formatting_policies_route_through_document_formatting() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    harness.notify(
        "workspace/didChangeConfiguration",
        json!({
            "settings": {
                "perl": {
                    "formatting": {
                        "engine": "native",
                        "maximumLineLength": 20,
                        "openingBraceOnNewLine": true,
                        "cuddledElse": false,
                        "spaceAfterKeyword": false,
                        "addTrailingCommas": true
                    }
                }
            }
        }),
    );

    let doc_uri = "file:///test_native_formatting_policies.pl";
    let source = "if($ok){return foo($alpha,$beta,$gamma);}else{return bar();}\n";
    harness.open(doc_uri, source)?;

    let response = harness.request(
        "textDocument/formatting",
        json!({
            "textDocument": { "uri": doc_uri },
            "options": {
                "tabSize": 4,
                "insertSpaces": true
            }
        }),
    )?;
    let edits = response.as_array().ok_or("formatting response must be an array")?;
    let formatted = edits
        .first()
        .and_then(|edit| edit.get("newText"))
        .and_then(|text| text.as_str())
        .ok_or("formatting edit must include newText")?;

    assert_eq!(
        formatted,
        concat!(
            "if($ok)\n",
            "{\n",
            "    return foo(\n",
            "    $alpha,\n",
            "    $beta,\n",
            "    $gamma,\n",
            ");\n",
            "}\n",
            "else\n",
            "{\n",
            "    return bar();\n",
            "}\n",
        )
    );

    Ok(())
}

/// Test that configured native formatter policy fields route through LSP range formatting.
#[test]
fn test_native_formatting_policies_route_through_range_formatting() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    harness.notify(
        "workspace/didChangeConfiguration",
        json!({
            "settings": {
                "perl": {
                    "formatting": {
                        "engine": "native",
                        "openingBraceOnNewLine": true,
                        "cuddledElse": false,
                        "spaceAfterKeyword": false
                    }
                }
            }
        }),
    );

    let doc_uri = "file:///test_native_range_formatting_policies.pl";
    let source = "my $prefix = 1;\nif($ok){return 1;}else{return 0;}\nmy $suffix = 1;\n";
    harness.open(doc_uri, source)?;

    let response = harness.request(
        "textDocument/rangeFormatting",
        json!({
            "textDocument": { "uri": doc_uri },
            "range": {
                "start": { "line": 1, "character": 0 },
                "end": { "line": 1, "character": 34 }
            },
            "options": {
                "tabSize": 4,
                "insertSpaces": true
            }
        }),
    )?;
    let edits = response.as_array().ok_or("rangeFormatting response must be an array")?;
    let formatted = apply_text_edits(source, edits);

    assert_eq!(
        formatted,
        concat!(
            "my $prefix = 1;\n",
            "if($ok)\n",
            "{\n",
            "    return 1;\n",
            "}\n",
            "else\n",
            "{\n",
            "    return 0;\n",
            "}\n",
            "my $suffix = 1;\n",
        )
    );

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

    let response = harness.request(
        "textDocument/formatting",
        json!({
            "textDocument": { "uri": doc_uri },
            "options": {
                "tabSize": 4,
                "insertSpaces": true
            }
        }),
    )?;

    let edits = response.as_array().ok_or("formatting response must be an array")?;
    assert!(edits.is_empty(), "comment-only native formatting should return no edits");

    Ok(())
}
