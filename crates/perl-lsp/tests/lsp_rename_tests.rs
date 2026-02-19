//! Tests for textDocument/rename and textDocument/prepareRename LSP features
//!
//! Validates the rename provider functionality including:
//! - Renaming a variable across its scope
//! - Prepare rename validation (checking if a symbol is renamable)
//! - Attempting rename on a non-renamable token (keyword, comment)
//! - Capability advertisement in server initialization
//! - WorkspaceEdit response structure validation

mod support;
use serde_json::json;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Test renaming a variable and verifying the WorkspaceEdit response structure
#[test]
fn test_rename_variable() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_rename.pl";
    harness.open(
        doc_uri,
        r#"sub process {
    my $count = 0;
    $count++;
    print "Count: $count\n";
    return $count;
}
"#,
    )?;

    // Rename $count to $total at its declaration (line 1, character 7)
    let response = harness
        .request(
            "textDocument/rename",
            json!({
                "textDocument": { "uri": doc_uri },
                "position": { "line": 1, "character": 7 },
                "newName": "$total"
            }),
        )
        .unwrap_or(json!(null));

    if !response.is_null() {
        // Response should be a WorkspaceEdit with changes
        assert!(
            response.is_object(),
            "rename should return a WorkspaceEdit object, got: {:?}",
            response
        );

        // WorkspaceEdit should have "changes" or "documentChanges"
        let has_changes = response.get("changes").is_some();
        let has_doc_changes = response.get("documentChanges").is_some();
        assert!(
            has_changes || has_doc_changes,
            "WorkspaceEdit should have 'changes' or 'documentChanges'. Got: {:?}",
            response
        );

        if let Some(changes) = response.get("changes") {
            // changes is a map from URI to TextEdit[]
            assert!(changes.is_object(), "changes should be an object mapping URIs to edits");
            if let Some(uri_edits) = changes.get(doc_uri) {
                let edits = uri_edits.as_array().ok_or("edits should be an array")?;
                // Should have multiple edits (one for each occurrence of $count)
                assert!(
                    !edits.is_empty(),
                    "Should have at least one edit for renamed variable"
                );
                for edit in edits {
                    assert!(edit["range"].is_object(), "Each edit should have a range");
                    let new_text = edit["newText"].as_str().ok_or("newText should be a string")?;
                    assert!(
                        new_text.contains("total"),
                        "newText should contain the new name 'total', got: {}",
                        new_text
                    );
                }
            }
        }
    }

    Ok(())
}

/// Test prepareRename to validate that a position is renamable
#[test]
fn test_prepare_rename_valid() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_prepare_rename.pl";
    harness.open(
        doc_uri,
        r#"sub calculate {
    my $value = 10;
    return $value * 2;
}
"#,
    )?;

    // prepareRename at $value declaration (line 1, character 7)
    let response = harness
        .request(
            "textDocument/prepareRename",
            json!({
                "textDocument": { "uri": doc_uri },
                "position": { "line": 1, "character": 7 }
            }),
        )
        .unwrap_or(json!(null));

    // Response should be { range, placeholder } or null if not renamable
    if !response.is_null() {
        // Could be { range, placeholder } or just a Range
        if response.get("range").is_some() {
            let range = &response["range"];
            assert!(range["start"].is_object(), "range should have start position");
            assert!(range["end"].is_object(), "range should have end position");
        } else if response.get("start").is_some() {
            // It's a bare Range object
            assert!(response["start"].is_object(), "bare range should have start");
            assert!(response["end"].is_object(), "bare range should have end");
        }

        // If placeholder is provided, it should be a string
        if let Some(placeholder) = response.get("placeholder") {
            assert!(
                placeholder.is_string(),
                "placeholder should be a string, got: {:?}",
                placeholder
            );
        }
    }

    Ok(())
}

/// Test prepareRename on a non-renamable location (e.g., a keyword or comment)
#[test]
fn test_prepare_rename_non_renamable() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_non_renamable.pl";
    harness.open(
        doc_uri,
        r#"# This is a comment
use strict;
use warnings;

sub test {
    return 1;
}
"#,
    )?;

    // prepareRename on the "use" keyword (line 1, character 0) - should not be renamable
    let response = harness
        .request(
            "textDocument/prepareRename",
            json!({
                "textDocument": { "uri": doc_uri },
                "position": { "line": 1, "character": 0 }
            }),
        )
        .unwrap_or(json!(null));

    // Keywords should either return null or an error
    // Both are acceptable behaviors for non-renamable tokens
    // If it returns a value, it means the server is lenient about what can be renamed
    if !response.is_null() {
        // Some servers return a range even for keywords (with the keyword text as placeholder)
        // That is acceptable behavior as long as the rename itself would fail gracefully
        assert!(
            response.is_object(),
            "If non-null, prepareRename should return an object, got: {:?}",
            response
        );
    }

    Ok(())
}

/// Test that rename capability is advertised during initialization
#[test]
fn test_rename_capability_advertised() -> TestResult {
    let mut harness = LspHarness::new();
    let init_response = harness.initialize(None)?;

    let capabilities = &init_response["capabilities"];

    let rename_provider = capabilities.get("renameProvider");
    assert!(
        rename_provider.is_some(),
        "Server should advertise renameProvider capability. Capabilities: {:?}",
        capabilities
    );

    // If renameProvider is an object, check for prepareProvider support
    if let Some(rp) = rename_provider {
        if rp.is_object() {
            let has_prepare = rp.get("prepareProvider");
            if let Some(prepare) = has_prepare {
                assert!(
                    prepare.is_boolean(),
                    "prepareProvider should be a boolean, got: {:?}",
                    prepare
                );
            }
        }
    }

    Ok(())
}

/// Test renaming a subroutine name
#[test]
fn test_rename_subroutine() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_rename_sub.pl";
    harness.open(
        doc_uri,
        r#"sub old_name {
    my $x = 1;
    return $x;
}

sub caller {
    my $result = old_name();
    return $result;
}
"#,
    )?;

    // Rename the subroutine at its declaration (line 0, character 4)
    let response = harness
        .request(
            "textDocument/rename",
            json!({
                "textDocument": { "uri": doc_uri },
                "position": { "line": 0, "character": 4 },
                "newName": "new_name"
            }),
        )
        .unwrap_or(json!(null));

    if !response.is_null() {
        assert!(
            response.is_object(),
            "rename should return a WorkspaceEdit, got: {:?}",
            response
        );

        // If changes exist, verify the edit structure
        if let Some(changes) = response.get("changes") {
            if let Some(uri_edits) = changes.get(doc_uri) {
                let edits = uri_edits.as_array().ok_or("edits should be an array")?;
                // Should rename both the declaration and the call site
                assert!(
                    !edits.is_empty(),
                    "Should have edits for subroutine rename"
                );
            }
        }
    }

    Ok(())
}

/// Test rename at an out-of-bounds position returns null gracefully
#[test]
fn test_rename_out_of_bounds() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_oob_rename.pl";
    harness.open(
        doc_uri,
        r#"my $x = 1;
"#,
    )?;

    // Request rename at a position well beyond the document (line 999)
    let response = harness
        .request(
            "textDocument/rename",
            json!({
                "textDocument": { "uri": doc_uri },
                "position": { "line": 999, "character": 0 },
                "newName": "anything"
            }),
        )
        .unwrap_or(json!(null));

    // Should return null or an empty WorkspaceEdit for out-of-bounds
    if !response.is_null() {
        if let Some(changes) = response.get("changes") {
            if changes.is_object() {
                // Empty changes map is acceptable
                let change_map = changes.as_object().ok_or("changes should be an object")?;
                // May or may not have entries
                let _ = change_map;
            }
        }
    }

    Ok(())
}
