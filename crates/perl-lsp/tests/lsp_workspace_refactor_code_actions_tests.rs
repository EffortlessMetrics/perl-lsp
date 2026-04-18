//! Tests for workspace-wide refactoring code actions
//!
//! These tests verify that workspace-wide refactoring operations are exposed
//! through the LSP code action system. The key capability being tested is that
//! code actions can produce WorkspaceEdit responses with changes spanning
//! multiple files (multiple URIs).
//!
//! Issue #3522: "[workspace] Workspace-wide refactoring operations not supported"

use serde_json::json;

mod common;
use common::{
    initialize_lsp, send_notification, send_request, shutdown_and_exit, start_lsp_server,
};

/// Test that "Extract to module" refactoring produces multi-file WorkspaceEdit.
///
/// When selecting a subroutine that could be extracted to its own module,
/// the code action should:
/// 1. Be available (not empty response)
/// 2. Produce a WorkspaceEdit with documentChanges or changes spanning multiple URIs
///
/// Currently this test FAILS because extract-to-module is not exposed via LSP.
#[test]
fn test_extract_subroutine_to_module_produces_multi_file_edit()
-> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    // Open a file with a subroutine that could be extracted
    let uri = "file:///test_extract.pl";
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": r#"
package MyApp;

sub helper_function {
    my ($x) = @_;
    return $x * 2;
}

sub main {
    my $result = helper_function(42);
    print "Result: $result\n";
}

1;
"#
                }
            }
        }),
    );

    // Select the helper_function subroutine (lines 2-5)
    let response = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/codeAction",
            "params": {
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": 2, "character": 0 },
                    "end": { "line": 5, "character": 1 }
                },
                "context": {
                    "diagnostics": [],
                    "only": ["refactor.extract"]
                }
            }
        }),
    );

    let actions = response["result"].as_array().ok_or("Expected result to be an array")?;

    // There should be at least one extract-to-module action
    let extract_action = actions.iter().find(|a| {
        let title = a["title"].as_str().unwrap_or("");
        title.contains("Extract") && (title.contains("module") || title.contains("file"))
    });

    assert!(
        extract_action.is_some(),
        "Expected 'Extract to module' code action but got: {:?}",
        actions.iter().map(|a| a["title"].as_str().unwrap_or("")).collect::<Vec<_>>()
    );

    // The action should produce edits in multiple files (the new module + the original)
    // This requires documentChanges (LSP 3.16+) or changes map with multiple URIs
    let action = extract_action.unwrap();
    let edit = action.get("edit").ok_or("Action must have edit field")?;

    // Check for multi-file edit capability
    let has_document_changes = edit.get("documentChanges").is_some();
    let has_changes = edit.get("changes").and_then(|c| c.as_object()).map(|m| m.len()).unwrap_or(0);

    assert!(
        has_document_changes || has_changes > 1,
        "Extract-to-module must produce edits across multiple files. \
         Got documentChanges: {:?}, changes count: {}",
        has_document_changes,
        has_changes
    );

    shutdown_and_exit(&server);
    Ok(())
}

/// Test that "Move module" refactoring produces multi-file WorkspaceEdit.
///
/// When selecting a package declaration, the code action should:
/// 1. Be available (not empty response)
/// 2. Produce a WorkspaceEdit that updates use statements across multiple files
///
/// Currently this test FAILS because move-module is not exposed via LSP.
#[test]
fn test_move_module_updates_imports_in_other_files() -> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    // Open main file that uses a module
    let main_uri = "file:///lib/MyApp.pm";
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": main_uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": r#"
package MyApp;

use Internal::Utils;

sub process {
    return Internal::Utils::format_data("test");
}

1;
"#
                }
            }
        }),
    );

    // Open the Utils module that could be moved
    let utils_uri = "file:///lib/Internal/Utils.pm";
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": utils_uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": r#"
package Internal::Utils;

sub format_data {
    my ($str) = @_;
    return uc($str);
}

1;
"#
                }
            }
        }),
    );

    // Select the package declaration in Utils.pm
    let response = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/codeAction",
            "params": {
                "textDocument": { "uri": utils_uri },
                "range": {
                    "start": { "line": 0, "character": 0 },
                    "end": { "line": 0, "character": 25 }
                },
                "context": {
                    "diagnostics": [],
                    "only": ["refactor.move"]
                }
            }
        }),
    );

    let actions = response["result"].as_array().ok_or("Expected result to be an array")?;

    // There should be at least one move-module action
    let move_action = actions.iter().find(|a| {
        let title = a["title"].as_str().unwrap_or("");
        title.contains("Move") && (title.contains("module") || title.contains("package"))
    });

    assert!(
        move_action.is_some(),
        "Expected 'Move module' code action but got: {:?}",
        actions.iter().map(|a| a["title"].as_str().unwrap_or("")).collect::<Vec<_>>()
    );

    // The action should produce edits in both the moved file AND files that import it
    let action = move_action.unwrap();
    let edit = action.get("edit").ok_or("Action must have edit field")?;

    // Check for multi-file edit capability
    let has_document_changes = edit.get("documentChanges").is_some();
    let changes_map = edit.get("changes").and_then(|c| c.as_object());
    let changes_count = changes_map.map(|m| m.len()).unwrap_or(0);

    assert!(
        has_document_changes || changes_count > 1,
        "Move-module must produce edits across multiple files (moved module + importing files). \
         Got documentChanges: {:?}, changes count: {}",
        has_document_changes,
        changes_count
    );

    shutdown_and_exit(&server);
    Ok(())
}

/// Test that "source.organizeImports" can work across multiple files.
///
/// The organize imports action should:
/// 1. Be available when requested with source.organizeImports kind
/// 2. Produce a WorkspaceEdit with documentChanges or changes spanning multiple URIs
///
/// Currently this test FAILS because multi-file organize imports is not implemented.
#[test]
fn test_organize_imports_produces_multi_file_edit() -> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    // Open first file with messy imports
    let file1_uri = "file:///lib/File1.pm";
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": file1_uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": r#"
use strict;
use warnings;
use Data::Dumper;
use Carp qw(carp);
use Data::Dumper;
use File::Spec;

1;
"#
                }
            }
        }),
    );

    // Open second file with messy imports
    let file2_uri = "file:///lib/File2.pm";
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": file2_uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": r#"
use File::Spec;
use strict;
use Carp qw(croak);
use warnings;

1;
"#
                }
            }
        }),
    );

    // Request source.organizeImports for file1
    let response = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "textDocument/codeAction",
            "params": {
                "textDocument": { "uri": file1_uri },
                "range": {
                    "start": { "line": 0, "character": 0 },
                    "end": { "line": 0, "character": 0 }
                },
                "context": {
                    "diagnostics": [],
                    "only": ["source.organizeImports"]
                }
            }
        }),
    );

    let actions = response["result"].as_array().ok_or("Expected result to be an array")?;

    // There should be at least one organize imports action
    let organize_action = actions.iter().find(|a| {
        let kind = a["kind"].as_str().unwrap_or("");
        kind == "source.organizeImports"
    });

    assert!(
        organize_action.is_some(),
        "Expected 'source.organizeImports' code action but got: {:?}",
        actions
            .iter()
            .map(|a| (a["title"].as_str().unwrap_or(""), a["kind"].as_str().unwrap_or("")))
            .collect::<Vec<_>>()
    );

    // The action should produce edits in multiple files (both File1 and File2 have duplicate/dirty imports)
    let action = organize_action.unwrap();
    let edit = action.get("edit").ok_or("Action must have edit field")?;

    // Check for multi-file edit capability
    let has_document_changes = edit.get("documentChanges").is_some();
    let changes_map = edit.get("changes").and_then(|c| c.as_object());
    let changes_count = changes_map.map(|m| m.len()).unwrap_or(0);

    assert!(
        has_document_changes || changes_count > 1,
        "Organize imports should produce edits across multiple files when workspace-wide. \
         Got documentChanges: {:?}, changes count: {}",
        has_document_changes,
        changes_count
    );

    shutdown_and_exit(&server);
    Ok(())
}

/// Test that "refactor.safeDelete" checks for references before deletion.
///
/// When attempting to delete a symbol that is referenced elsewhere,
/// the safe delete code action should either:
/// 1. Not be offered (if there are references)
/// 2. Be offered with a warning in the description
///
/// Currently this test FAILS because safe delete is not implemented.
#[test]
fn test_safe_delete_checks_references() -> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    // Open main file that defines and uses a subroutine
    let main_uri = "file:///lib/Main.pm";
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": main_uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": r#"
package Main;

sub used_helper {
    return "help";
}

sub main {
    print used_helper();
}

1;
"#
                }
            }
        }),
    );

    // Open another file that also uses the helper
    let other_uri = "file:///lib/Other.pm";
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": other_uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": r#"
package Other;

use Main qw(used_helper);

sub process {
    return used_helper();
}

1;
"#
                }
            }
        }),
    );

    // Select the used_helper subroutine in Main.pm
    let response = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "textDocument/codeAction",
            "params": {
                "textDocument": { "uri": main_uri },
                "range": {
                    "start": { "line": 1, "character": 0 },
                    "end": { "line": 3, "character": 1 }
                },
                "context": {
                    "diagnostics": [],
                    "only": ["refactor.safeDelete"]
                }
            }
        }),
    );

    let actions = response["result"].as_array().ok_or("Expected result to be an array")?;

    // Find safe delete action (may not exist if there are references)
    let safe_delete_action = actions.iter().find(|a| {
        let kind = a["kind"].as_str().unwrap_or("");
        kind == "refactor.safeDelete"
    });

    // If safe delete is offered, it should indicate there are references
    // (either through a warning in the title or through diagnostics)
    if let Some(action) = safe_delete_action {
        let title = action["title"].as_str().unwrap_or("");
        let has_warning = title.to_lowercase().contains("reference")
            || title.to_lowercase().contains("used")
            || title.to_lowercase().contains("called");

        // If no warning in title, check diagnostics
        let has_diagnostics = action.get("diagnostics").is_some();

        assert!(
            has_warning || has_diagnostics,
            "safeDelete for referenced symbol should warn about references. \
             Title: {}, Has diagnostics: {}",
            title,
            has_diagnostics
        );
    }
    // If safe delete is NOT offered, that's also acceptable behavior when references exist

    shutdown_and_exit(&server);
    Ok(())
}

/// Test that cross-file rename via code action produces multi-file WorkspaceEdit.
///
/// When renaming a subroutine that is referenced in other files,
/// the code action should produce edits across all affected files.
///
/// This is different from textDocument/rename which is already implemented.
/// This tests the refactor.codeAction avenue for rename.
#[test]
fn test_workspace_rename_code_action_produces_multi_file_edit()
-> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    // Open file that defines a subroutine
    let def_uri = "file:///lib/Def.pm";
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": def_uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": r#"
package Def;

sub target_sub {
    return 42;
}

1;
"#
                }
            }
        }),
    );

    // Open file that uses the subroutine
    let use_uri = "file:///lib/Use.pm";
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": use_uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": r#"
package Use;

use Def qw(target_sub);

sub run {
    return target_sub();
}

1;
"#
                }
            }
        }),
    );

    // Select the subroutine definition in Def.pm and request refactor.rename
    let response = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "textDocument/codeAction",
            "params": {
                "textDocument": { "uri": def_uri },
                "range": {
                    "start": { "line": 1, "character": 0 },
                    "end": { "line": 3, "character": 1 }
                },
                "context": {
                    "diagnostics": [],
                    "only": ["refactor"]
                }
            }
        }),
    );

    let actions = response["result"].as_array().ok_or("Expected result to be an array")?;

    // Find rename action
    let rename_action = actions.iter().find(|a| {
        let title = a["title"].as_str().unwrap_or("");
        title.contains("Rename")
    });

    if let Some(action) = rename_action {
        let edit = action.get("edit").ok_or("Action must have edit field")?;

        // Check for multi-file edit capability
        let has_document_changes = edit.get("documentChanges").is_some();
        let changes_map = edit.get("changes").and_then(|c| c.as_object());
        let changes_count = changes_map.map(|m| m.len()).unwrap_or(0);

        assert!(
            has_document_changes || changes_count > 1,
            "Workspace rename must produce edits across multiple files. \
             Got documentChanges: {:?}, changes count: {}",
            has_document_changes,
            changes_count
        );
    }

    shutdown_and_exit(&server);
    Ok(())
}
