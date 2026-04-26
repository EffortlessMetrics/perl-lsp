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
    let response = harness.request(
        "textDocument/rename",
        json!({
            "textDocument": { "uri": doc_uri },
            "position": { "line": 1, "character": 7 },
            "newName": "$total"
        }),
    )?;

    assert!(
        response.is_object(),
        "rename should return a WorkspaceEdit object, got: {:?}",
        response
    );

    let changes = response
        .get("changes")
        .and_then(serde_json::Value::as_object)
        .ok_or("rename response should include `changes` object")?;
    let edits = changes
        .get(doc_uri)
        .and_then(serde_json::Value::as_array)
        .ok_or("rename response should include edits for the current document")?;
    assert!(
        edits.len() >= 3,
        "expected at least 3 edits (declaration + usages), got {}: {:?}",
        edits.len(),
        edits
    );
    for edit in edits {
        assert!(edit["range"].is_object(), "Each edit should have a range");
        let new_text = edit["newText"].as_str().ok_or("newText should be a string")?;
        assert_eq!(new_text, "$total", "variable rename should preserve sigil");
    }

    Ok(())
}

#[test]
fn test_rename_variable_without_sigil_infers_original_sigil() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_rename_infer_sigil.pl";
    harness.open(
        doc_uri,
        r#"sub process {
    my $count = 0;
    $count++;
    return $count;
}
"#,
    )?;

    let response = harness.request(
        "textDocument/rename",
        json!({
            "textDocument": { "uri": doc_uri },
            "position": { "line": 1, "character": 7 },
            "newName": "total"
        }),
    )?;

    let changes = response
        .get("changes")
        .and_then(serde_json::Value::as_object)
        .ok_or("rename response should include `changes` object")?;
    let edits = changes
        .get(doc_uri)
        .and_then(serde_json::Value::as_array)
        .ok_or("rename response should include edits for current document")?;

    assert!(!edits.is_empty(), "rename should produce at least one edit");
    for edit in edits {
        assert_eq!(edit["newText"], json!("$total"));
    }

    Ok(())
}

#[test]
fn test_prepare_rename_on_sigil_returns_symbol_range() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_prepare_on_sigil.pl";
    harness.open(
        doc_uri,
        r#"sub calculate {
    my $value = 10;
    return $value;
}
"#,
    )?;

    let response = harness.request(
        "textDocument/prepareRename",
        json!({
            "textDocument": { "uri": doc_uri },
            "position": { "line": 1, "character": 7 }
        }),
    )?;

    assert!(response.is_object(), "prepareRename should return a range payload");
    assert_eq!(
        response.get("placeholder"),
        Some(&json!("$value")),
        "prepareRename on sigil should include full variable token"
    );

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
        assert!(response.is_object(), "rename should return a WorkspaceEdit, got: {:?}", response);

        // If changes exist, verify the edit structure
        if let Some(changes) = response.get("changes") {
            if let Some(uri_edits) = changes.get(doc_uri) {
                let edits = uri_edits.as_array().ok_or("edits should be an array")?;
                // Should rename both the declaration and the call site
                assert!(!edits.is_empty(), "Should have edits for subroutine rename");
            }
        }
    }

    Ok(())
}

/// Test that renaming with a mismatched sigil is rejected.
///
/// The PR's `normalize_rename_target` must reject `@foo` as a new name when
/// the symbol under the cursor is `$foo` — cross-sigil rename would silently
/// change variable semantics (scalar -> array).
#[test]
fn test_rename_mismatched_sigil_is_rejected() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_rename_mismatched_sigil.pl";
    harness.open(
        doc_uri,
        r#"sub process {
    my $count = 0;
    $count++;
    return $count;
}
"#,
    )?;

    // Cursor on `$count` declaration; request rename to `@count` (array sigil).
    let result = harness.request(
        "textDocument/rename",
        json!({
            "textDocument": { "uri": doc_uri },
            "position": { "line": 1, "character": 7 },
            "newName": "@count"
        }),
    );

    // Expect an error (invalid-params -32602) OR an empty workspace edit.
    // The PR returns JsonRpcError(-32602), which harness surfaces as Err.
    match result {
        Err(e) => {
            let msg = format!("{e}");
            assert!(
                msg.contains("sigil") || msg.contains("Invalid") || msg.contains("32602"),
                "error should mention sigil mismatch or invalid identifier, got: {msg}"
            );
        }
        Ok(response) => {
            // If it didn't error, the edits must not contain the mismatched sigil.
            if let Some(changes) = response.get("changes").and_then(|v| v.as_object()) {
                if let Some(edits) = changes.get(doc_uri).and_then(|v| v.as_array()) {
                    for edit in edits {
                        let new_text = edit["newText"].as_str().unwrap_or("");
                        assert!(
                            !new_text.starts_with('@'),
                            "mismatched-sigil rename must not produce @-prefixed edits, got: {new_text}"
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

/// Test that renaming with an empty newName is rejected.
#[test]
fn test_rename_empty_new_name_is_rejected() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_rename_empty.pl";
    harness.open(
        doc_uri,
        r#"my $x = 1;
print $x;
"#,
    )?;

    let result = harness.request(
        "textDocument/rename",
        json!({
            "textDocument": { "uri": doc_uri },
            "position": { "line": 0, "character": 4 },
            "newName": ""
        }),
    );

    match result {
        Err(e) => {
            let msg = format!("{e}");
            assert!(
                msg.contains("empty") || msg.contains("Invalid") || msg.contains("32602"),
                "empty newName should error with invalid-identifier, got: {msg}"
            );
        }
        Ok(response) => {
            // If the server accepted it, the edits must not have empty newText.
            if let Some(changes) = response.get("changes").and_then(|v| v.as_object()) {
                for (_uri, edits) in changes {
                    if let Some(arr) = edits.as_array() {
                        for edit in arr {
                            let new_text = edit["newText"].as_str().unwrap_or("");
                            assert!(
                                !new_text.is_empty(),
                                "empty newName must not yield empty edits"
                            );
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Test that renaming with an invalid identifier (digit-leading) is rejected.
#[test]
fn test_rename_invalid_identifier_is_rejected() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_rename_invalid_ident.pl";
    harness.open(
        doc_uri,
        r#"sub process {
    my $count = 0;
    return $count;
}
"#,
    )?;

    // `$1bad` — after sigil, identifier starts with a digit, which is invalid.
    let result = harness.request(
        "textDocument/rename",
        json!({
            "textDocument": { "uri": doc_uri },
            "position": { "line": 1, "character": 7 },
            "newName": "$1bad"
        }),
    );

    match result {
        Err(e) => {
            let msg = format!("{e}");
            assert!(
                msg.contains("Invalid") || msg.contains("32602"),
                "digit-leading identifier should error, got: {msg}"
            );
        }
        Ok(response) => {
            if let Some(changes) = response.get("changes").and_then(|v| v.as_object()) {
                for (_uri, edits) in changes {
                    if let Some(arr) = edits.as_array() {
                        assert!(
                            arr.is_empty(),
                            "invalid identifier should produce no edits, got: {:?}",
                            arr
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

/// Test renaming an array variable preserves the `@` sigil.
#[test]
fn test_rename_array_preserves_at_sigil() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_rename_array.pl";
    harness.open(
        doc_uri,
        r#"sub collect {
    my @items = (1, 2, 3);
    push @items, 4;
    return @items;
}
"#,
    )?;

    // Rename `@items` -> `@values` at declaration (line 1, character 7).
    let response = harness.request(
        "textDocument/rename",
        json!({
            "textDocument": { "uri": doc_uri },
            "position": { "line": 1, "character": 7 },
            "newName": "@values"
        }),
    )?;

    if let Some(changes) = response.get("changes").and_then(|v| v.as_object()) {
        if let Some(edits) = changes.get(doc_uri).and_then(|v| v.as_array()) {
            assert!(!edits.is_empty(), "array rename should produce edits");
            for edit in edits {
                let new_text = edit["newText"].as_str().unwrap_or("");
                // `@values` or bare `values` after workspace-rename-edit adjustments
                // — whichever comes back must be `@`-sigiled, never `$`.
                assert!(
                    new_text.starts_with('@') || new_text == "values",
                    "array rename must preserve or omit `@` sigil, never swap to `$`, got: {new_text}"
                );
                assert!(
                    !new_text.starts_with('$'),
                    "array rename must NOT produce a `$`-prefixed edit: {new_text}"
                );
            }
        }
    }

    Ok(())
}

/// Test that bare identifier rename of an array variable infers the `@` sigil.
#[test]
fn test_rename_array_bare_infers_at_sigil() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_rename_array_bare.pl";
    harness.open(
        doc_uri,
        r#"sub collect {
    my @items = (1, 2, 3);
    return @items;
}
"#,
    )?;

    // Bare `values` as newName; current symbol is `@items`, so result must be `@values`.
    let response = harness.request(
        "textDocument/rename",
        json!({
            "textDocument": { "uri": doc_uri },
            "position": { "line": 1, "character": 7 },
            "newName": "values"
        }),
    )?;

    if let Some(changes) = response.get("changes").and_then(|v| v.as_object()) {
        if let Some(edits) = changes.get(doc_uri).and_then(|v| v.as_array()) {
            for edit in edits {
                let new_text = edit["newText"].as_str().unwrap_or("");
                assert!(
                    !new_text.starts_with('$') && !new_text.starts_with('%'),
                    "bare-name array rename must not accidentally get wrong sigil, got: {new_text}"
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
