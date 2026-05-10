#![allow(clippy::collapsible_if)]

use serde_json::json;

mod support;
use support::lsp_client::LspClient;

#[test]
fn native_default_document_formatting() -> Result<(), Box<dyn std::error::Error>> {
    let bin = env!("CARGO_BIN_EXE_perl-lsp");
    let mut client = LspClient::spawn(bin)?;
    let uri = "file:///fmt.pl";

    let source = "sub test{my$x=1;return$x;}\nsub another{return 2;}\n";

    client.did_open(uri, "perl", source)?;

    let response = client.request(
        "textDocument/formatting",
        json!({
            "textDocument": {"uri": uri},
            "options": {"tabSize": 4, "insertSpaces": true}
        }),
    )?;

    let edits =
        response["result"].as_array().ok_or("formatting should return an array of edits")?;

    assert!(!edits.is_empty(), "Should return formatting edits");

    let edit_text = edits.first().ok_or("edits array should have at least one element")?["newText"]
        .as_str()
        .ok_or("Edit should have newText")?;

    assert!(
        edit_text.contains("sub test") && edit_text.contains("{"),
        "Should format subroutine declaration, got: {}",
        edit_text
    );
    assert!(edit_text.contains("my $x = 1"), "Should add spaces around operators");
    assert!(edit_text.contains("return $x"), "Should format return statement");
    assert!(
        edit_text.contains("sub another") && edit_text.contains("{"),
        "Should format second subroutine"
    );

    client.shutdown()?;
    Ok(())
}

#[test]
fn native_default_range_formatting() -> Result<(), Box<dyn std::error::Error>> {
    let bin = env!("CARGO_BIN_EXE_perl-lsp");
    let mut client = LspClient::spawn(bin)?;
    let uri = "file:///range.pl";

    let source = "# First subroutine - leave this comment untouched\nsub first{my$a=1;return$a;}\n\n# Second subroutine - don't format this\nsub second{my$b=2;return$b;}\n";

    client.did_open(uri, "perl", source)?;

    let response = client.request(
        "textDocument/rangeFormatting",
        json!({
            "textDocument": {"uri": uri},
            "range": {
                "start": {"line": 1, "character": 0},
                "end": {"line": 1, "character": 27}
            },
            "options": {"tabSize": 4, "insertSpaces": true}
        }),
    )?;

    let edits =
        response["result"].as_array().ok_or("rangeFormatting should return an edit array")?;
    assert!(!edits.is_empty(), "native default range formatting should return edits");

    let edit = edits.first().ok_or("edits array should have at least one element")?;
    assert_eq!(edit["range"]["start"]["line"], 1);
    assert_eq!(edit["range"]["end"]["line"], 1);
    let edit_text = edit["newText"].as_str().ok_or("Edit should have newText")?;

    assert!(edit_text.contains("sub first") && edit_text.contains("{"));
    assert!(edit_text.contains("my $a = 1"), "Should format selected subroutine content");
    assert!(!edit_text.contains("Second subroutine"), "Should not include unselected text");

    client.shutdown()?;
    Ok(())
}

#[test]
fn native_default_formatting_preserves_comments() -> Result<(), Box<dyn std::error::Error>> {
    let bin = env!("CARGO_BIN_EXE_perl-lsp");
    let mut client = LspClient::spawn(bin)?;
    let uri = "file:///comments.pl";

    let source = r#"#!/usr/bin/perl
# Main script comment
use strict;use warnings;
# Function comment
sub test{
# Inner comment
my$x=1;# Inline comment
return$x;
}
"#;

    client.did_open(uri, "perl", source)?;

    let response = client.request(
        "textDocument/formatting",
        json!({
            "textDocument": {"uri": uri},
            "options": {"tabSize": 4, "insertSpaces": true}
        }),
    )?;

    let edits =
        response["result"].as_array().ok_or("formatting should return an array of edits")?;

    assert!(!edits.is_empty(), "native default formatting should return comment-safe edits");
    let edit_text = edits.first().ok_or("edits array should have at least one element")?["newText"]
        .as_str()
        .ok_or("Edit should have newText")?;

    assert!(edit_text.contains("# Main script comment"), "Should preserve main comment");
    assert!(edit_text.contains("# Function comment"), "Should preserve function comment");
    assert!(edit_text.contains("# Inner comment"), "Should preserve inner comment");
    assert!(edit_text.contains("# Inline comment"), "Should preserve inline comment");

    assert!(edit_text.contains("use strict"), "Should format use statements");
    assert!(edit_text.contains("use warnings"), "Should separate use statements");
    assert!(edit_text.contains("sub test") && edit_text.contains("{"), "Should format subroutine");

    client.shutdown()?;
    Ok(())
}

#[test]
fn native_default_formatting_honors_lsp_tab_size() -> Result<(), Box<dyn std::error::Error>> {
    let bin = env!("CARGO_BIN_EXE_perl-lsp");
    let mut client = LspClient::spawn(bin)?;
    let uri = "file:///tab-size.pl";

    let source = "sub test{my$x=1;return$x;}\n";

    client.did_open(uri, "perl", source)?;

    let response = client.request(
        "textDocument/formatting",
        json!({
            "textDocument": {"uri": uri},
            "options": {"tabSize": 2, "insertSpaces": true}
        }),
    )?;

    let edits =
        response["result"].as_array().ok_or("formatting should return an array of edits")?;

    assert!(!edits.is_empty(), "native default formatting should return edits");
    let edit_text = edits.first().ok_or("edits array should have at least one element")?["newText"]
        .as_str()
        .ok_or("Edit should have newText")?;

    assert!(edit_text.contains("sub test {\n  my $x = 1;\n  return $x;\n}"));

    client.shutdown()?;
    Ok(())
}

#[test]

fn ranges_formatting() -> Result<(), Box<dyn std::error::Error>> {
    // Skip test if perltidy is not available
    if std::process::Command::new("perltidy").arg("--version").output().is_err() {
        eprintln!("Skipping test: perltidy not installed");
        return Ok(());
    }

    let bin = env!("CARGO_BIN_EXE_perl-lsp");
    let mut client = LspClient::spawn(bin)?;
    let uri = "file:///ranges.pl";

    let source = r#"
# First subroutine - format this
sub first{my$a=1;return$a;}

# Second subroutine - don't format this
sub second{my$b=2;return$b;}

# Third subroutine - format this too
sub third{my$c=3;return$c;}
"#;

    client.did_open(uri, "perl", source)?;

    // Request formatting for multiple ranges (first and third subroutines)
    let response = client.request(
        "textDocument/rangesFormatting",
        json!({
            "textDocument": {"uri": uri},
            "ranges": [
                {
                    "start": {"line": 1, "character": 0},
                    "end": {"line": 2, "character": 28}
                },
                {
                    "start": {"line": 7, "character": 0},
                    "end": {"line": 8, "character": 28}
                }
            ],
            "options": {"tabSize": 4, "insertSpaces": true}
        }),
    )?;

    if let Some(result) = response.get("result") {
        if let Some(edits) = result.as_array() {
            assert!(!edits.is_empty(), "Should return formatting edits for multiple ranges");

            // Verify that we got edits (exact number depends on perltidy behavior)
            let edit_count = edits.len();
            eprintln!("Received {} edits for ranges formatting", edit_count);
            assert!(edit_count > 0, "Should have at least one edit");

            // Check that at least one edit contains formatted code
            let has_formatted = edits.iter().any(|edit| {
                if let Some(new_text) = edit["newText"].as_str() {
                    new_text.contains("sub first") || new_text.contains("sub third")
                } else {
                    false
                }
            });
            assert!(has_formatted, "Should contain formatted code for first or third subroutine");
        }
    }

    client.shutdown()?;
    Ok(())
}
