/// Tests for workspace-aware completion integration
///
/// This module tests that the completion provider properly queries the workspace
/// index to provide cross-file symbol completions.
use insta::assert_yaml_snapshot;
use serde_json::json;
use std::time::Duration;

mod common;
use common::{
    completion_items, drain_until_quiet, initialize_lsp, send_notification, send_request,
    start_lsp_server,
};

fn completion_snapshot(items: &[serde_json::Value], prefix: &str) -> Vec<serde_json::Value> {
    let mut snapshot_items: Vec<serde_json::Value> = items
        .iter()
        .filter_map(|item| {
            let label = item.get("label")?.as_str()?;
            if !label.contains(prefix) {
                return None;
            }

            Some(json!({
                "label": label,
                "kind": item.get("kind"),
                "detail": item.get("detail"),
                "documentation": item
                    .get("documentation")
                    .and_then(|doc| doc.get("value").and_then(serde_json::Value::as_str))
                    .or_else(|| item.get("documentation").and_then(serde_json::Value::as_str)),
                "insertText": item.get("insertText"),
            }))
        })
        .collect();

    snapshot_items.sort_by(|a, b| {
        let a_label = a.get("label").and_then(serde_json::Value::as_str).unwrap_or("");
        let b_label = b.get("label").and_then(serde_json::Value::as_str).unwrap_or("");
        a_label.cmp(b_label)
    });

    snapshot_items
}

fn completion_snapshot_for_labels(
    items: &[serde_json::Value],
    expected_labels: &[&str],
) -> Vec<serde_json::Value> {
    let mut snapshot_items: Vec<serde_json::Value> = expected_labels
        .iter()
        .filter_map(|expected_label| {
            let item = items.iter().find(|item| {
                item.get("label").and_then(serde_json::Value::as_str) == Some(*expected_label)
            })?;

            Some(json!({
                "label": expected_label,
                "kind": item.get("kind"),
                "detail": item.get("detail"),
                "documentation": item
                    .get("documentation")
                    .and_then(|doc| doc.get("value").and_then(serde_json::Value::as_str))
                    .or_else(|| item.get("documentation").and_then(serde_json::Value::as_str)),
                "insertText": item.get("insertText"),
            }))
        })
        .collect();

    snapshot_items.sort_by(|a, b| {
        let a_label = a.get("label").and_then(serde_json::Value::as_str).unwrap_or("");
        let b_label = b.get("label").and_then(serde_json::Value::as_str).unwrap_or("");
        a_label.cmp(b_label)
    });

    snapshot_items
}

fn await_open_processing(server: &common::LspServer) {
    // didOpen triggers parse + indexing work asynchronously in the spawned server.
    // Drain until quiet before asserting on workspace-aware completions.
    drain_until_quiet(server, Duration::from_millis(50), Duration::from_millis(500));
}

/// Wait for the workspace index to incorporate a freshly opened module.
///
/// After `textDocument/didOpen` for a module file, the server dispatches a
/// background task (tokio blocking pool) to extract and insert symbols into
/// the workspace index.  No LSP notification is emitted when a per-file
/// background indexing task completes (the `perl-lsp/index-ready` notification
/// is sent once during `initialized` and is consumed by `initialize_lsp`).
///
/// This helper drains pending LSP traffic first, then waits a fixed interval
/// to give the blocking-pool task time to commit its symbol insertions.  The
/// 500ms wall-clock budget is generous enough for debug builds on slow CI
/// machines while remaining acceptable in total test time.
fn await_module_indexed(server: &common::LspServer) {
    // Drain any pending diagnostics / notifications from the didOpen.
    drain_until_quiet(server, Duration::from_millis(50), Duration::from_millis(500));
    // Fixed sleep: the background indexing task emits no notification when it
    // completes, so we must wait a wall-clock budget for it to finish.
    // 500ms is sufficient for debug-build Perl symbol extraction on slow machines.
    std::thread::sleep(Duration::from_millis(500));
}

/// Test cross-file function completion
///
/// When a user types a function name, the completion provider should suggest
/// functions from other files in the workspace that have been indexed.
#[test]
fn test_completion_cross_file_function() -> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    // Index a module file with a function
    let module_uri = "file:///workspace/EmailUtils.pm";
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": module_uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": r#"
package EmailUtils;

sub validate_email {
    my ($email) = @_;
    return $email =~ /@/;
}

sub parse_email_header {
    my ($header) = @_;
    return split /: /, $header;
}

1;
"#
                }
            }
        }),
    );
    await_open_processing(&server);

    // Now open a different file and request completion
    let script_uri = "file:///workspace/script.pl";
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": script_uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": r#"
use EmailUtils;

my $email = 'test@example.com';
vali
"#
                }
            }
        }),
    );
    await_open_processing(&server);

    // Request completion at position after "vali"
    let response = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": script_uri },
                "position": { "line": 4, "character": 4 }
            }
        }),
    );

    let items = completion_items(&response);
    let labels: Vec<String> =
        items.iter().filter_map(|item| item["label"].as_str().map(String::from)).collect();

    // Should suggest validate_email from the workspace index
    assert!(
        labels.iter().any(|l| l.contains("validate_email")),
        "Should suggest validate_email from workspace index. Got: {:?}",
        labels
    );

    Ok(())
}

/// Test cross-file package member completion with qualified names
#[test]
fn test_completion_cross_file_qualified() -> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    // Index a module file
    let module_uri = "file:///workspace/DataProcessor.pm";
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": module_uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": r#"
package DataProcessor;

sub process_data {
    my ($data) = @_;
    return uc $data;
}

sub transform_data {
    my ($data) = @_;
    return lc $data;
}

1;
"#
                }
            }
        }),
    );
    await_open_processing(&server);

    // Open a file requesting qualified completion
    let script_uri = "file:///workspace/main.pl";
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": script_uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": r#"
use DataProcessor;

my $result = DataProcessor::
"#
                }
            }
        }),
    );
    await_open_processing(&server);

    // Request completion after "DataProcessor::"
    let response = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": script_uri },
                // Cursor just after `DataProcessor::`
                "position": { "line": 3, "character": 28 }
            }
        }),
    );

    let items = completion_items(&response);
    let labels: Vec<String> =
        items.iter().filter_map(|item| item["label"].as_str().map(String::from)).collect();

    // Should suggest both functions from the module
    assert!(
        labels.contains(&"process_data".to_string()),
        "Should suggest process_data. Got: {:?}",
        labels
    );
    assert!(
        labels.contains(&"transform_data".to_string()),
        "Should suggest transform_data. Got: {:?}",
        labels
    );
    let process_data = items
        .iter()
        .find(|item| item["label"].as_str() == Some("process_data"))
        .ok_or("process_data completion should be present to verify documentation")?;
    let documentation = process_data["documentation"]["value"]
        .as_str()
        .ok_or("process_data should include markdown documentation")?;
    assert!(
        documentation.contains("DataProcessor::process_data"),
        "cross-file package completion should expose a qualified documentation snippet, got: {documentation:?}"
    );

    let qualified_snapshot = completion_snapshot(items, "data");
    assert_yaml_snapshot!("workspace_completion_qualified_data_processor", qualified_snapshot);

    Ok(())
}

/// Test cross-file variable completion
#[test]
fn test_completion_cross_file_variable() -> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    // Index a module with exported variables
    let module_uri = "file:///workspace/Config.pm";
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": module_uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": r#"
package Config;

our $CONFIG_PATH = '/etc/app.conf';
our $DEBUG_MODE = 1;

1;
"#
                }
            }
        }),
    );
    await_open_processing(&server);

    // Open a file requesting variable completion
    let script_uri = "file:///workspace/app.pl";
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": script_uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": r#"
use Config;

print $Config::CONF
"#
                }
            }
        }),
    );
    await_open_processing(&server);

    // Request completion after "$Config::CONF"
    let response = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": script_uri },
                "position": { "line": 3, "character": 19 }
            }
        }),
    );

    let items = completion_items(&response);
    let labels: Vec<String> =
        items.iter().filter_map(|item| item["label"].as_str().map(String::from)).collect();

    // Should suggest CONFIG_PATH from the workspace index
    assert!(
        labels.iter().any(|l| l.contains("CONFIG_PATH")),
        "Should suggest CONFIG_PATH from workspace. Got: {:?}",
        labels
    );

    Ok(())
}

/// Test that workspace completions are provided even for unqualified calls
#[test]
fn test_completion_bare_function_from_workspace() -> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    // Index a module with exported functions
    let module_uri = "file:///workspace/StringUtils.pm";
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": module_uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": r#"
package StringUtils;

use Exporter 'import';
our @EXPORT = qw(trim uppercase);

sub trim {
    my ($str) = @_;
    $str =~ s/^\s+|\s+$//g;
    return $str;
}

sub uppercase {
    my ($str) = @_;
    return uc $str;
}

1;
"#
                }
            }
        }),
    );
    await_open_processing(&server);

    // Open a file that imports the module
    let script_uri = "file:///workspace/text_processor.pl";
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": script_uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": r#"
use StringUtils;

my $text = "  hello  ";
tri
"#
                }
            }
        }),
    );
    await_open_processing(&server);

    // Request completion after "tri"
    let response = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": script_uri },
                "position": { "line": 4, "character": 3 }
            }
        }),
    );

    let items = completion_items(&response);
    let labels: Vec<String> =
        items.iter().filter_map(|item| item["label"].as_str().map(String::from)).collect();

    // Should suggest trim from the workspace index (bare name completion)
    assert!(
        labels.iter().any(|l| l == "trim" || l.contains("trim")),
        "Should suggest trim from workspace index. Got: {:?}",
        labels
    );

    Ok(())
}

/// Test that completing an unimported workspace subroutine attaches an
/// `additionalTextEdits` entry inserting the required `use Module;` statement
/// (issue #1694 — next-edit auto-import closure).
#[test]
fn test_completion_bare_function_auto_imports_module() -> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    let module_uri = "file:///workspace/StringUtils.pm";
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": module_uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": "package StringUtils;\nsub trimmer { }\n1;\n"
                }
            }
        }),
    );
    await_open_processing(&server);

    // Script does NOT import StringUtils — accepting the completion should add it.
    let script_uri = "file:///workspace/needs_import.pl";
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": script_uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": "use strict;\ntrimm\n"
                }
            }
        }),
    );
    await_open_processing(&server);

    let response = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": script_uri },
                "position": { "line": 1, "character": 5 }
            }
        }),
    );

    let items = completion_items(&response);
    let trimmer = items
        .iter()
        .find(|item| item["label"].as_str().is_some_and(|l| l.contains("trimmer")))
        .ok_or_else(|| {
            format!(
                "expected a `trimmer` workspace completion; got: {:?}",
                items.iter().filter_map(|i| i["label"].as_str()).collect::<Vec<_>>()
            )
        })?;

    let edits = trimmer["additionalTextEdits"]
        .as_array()
        .ok_or("trimmer completion should carry a serialized additionalTextEdits array")?;
    assert!(
        edits.iter().any(|e| e["newText"].as_str() == Some("use StringUtils;\n")),
        "completion should auto-insert `use StringUtils;`; got additionalTextEdits: {edits:?}"
    );

    Ok(())
}

/// Test that `->` completion suggests inherited methods from parent class.
///
/// Validates Gap 3 of issue #3482: add_workspace_method_completions must traverse
/// the inheritance chain (via collect_all_package_members BFS) so that methods
/// defined in parent packages appear when completing on a child class receiver.
#[test]
fn test_completion_inherited_method_from_parent() -> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    // Index the base class with a method
    let base_uri = "file:///workspace/CompletionBase.pm";
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": base_uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": "package CompletionBase;\n\nsub new {\n    my ($class) = @_;\n    return bless {}, $class;\n}\n\nsub inherited_greet {\n    my ($self) = @_;\n    return \"hello\";\n}\n\n1;\n"
                }
            }
        }),
    );
    await_open_processing(&server);

    // Index the child class that inherits from base
    let child_uri = "file:///workspace/CompletionChild.pm";
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": child_uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": "package CompletionChild;\nuse parent 'CompletionBase';\n\nsub child_only_method {\n    my ($self) = @_;\n    return \"child\";\n}\n\n1;\n"
                }
            }
        }),
    );
    await_open_processing(&server);

    // Open a script that uses the child class with '->'-prefixed cursor
    // We place the cursor right after 'CompletionChild->' on line 3 (0-indexed: line 2)
    let script_uri = "file:///workspace/main_completion.pl";
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": script_uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": "use CompletionChild;\nmy $c = CompletionChild->new();\n$c->"
                }
            }
        }),
    );
    await_open_processing(&server);

    // Request completion at position right after '$c->' (line 2, char 4)
    let response = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": script_uri },
                "position": { "line": 2, "character": 4 }
            }
        }),
    );

    let items = completion_items(&response);
    let labels: Vec<String> =
        items.iter().filter_map(|item| item["label"].as_str().map(String::from)).collect();

    // child_only_method should appear (direct member)
    assert!(
        labels.iter().any(|l| l.contains("child_only_method")),
        "Should suggest child_only_method. Got: {:?}",
        labels
    );

    // inherited_greet should appear from parent via collect_all_package_members BFS
    assert!(
        labels.iter().any(|l| l.contains("inherited_greet")),
        "Should suggest inherited_greet from parent class. Got: {:?}",
        labels
    );

    let inherited_snapshot =
        completion_snapshot_for_labels(items, &["child_only_method", "inherited_greet"]);
    assert_yaml_snapshot!("workspace_completion_inherited_methods", inherited_snapshot);

    Ok(())
}

/// Test that method completion detail includes medium-confidence receiver labels.
///
/// Integration counterpart for the receiver-evidence detail format covered in
/// provider unit tests: when receiver inference comes from literal `bless`,
/// completion detail should advertise that medium-confidence provenance.
#[test]
fn test_completion_detail_includes_literal_bless_confidence_label()
-> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    let module_uri = "file:///workspace/BlessedGreeter.pm";
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": module_uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": "package BlessedGreeter;\n\nsub greet {\n    my ($self) = @_;\n    return 'hi';\n}\n\n1;\n"
                }
            }
        }),
    );
    await_open_processing(&server);

    let script_uri = "file:///workspace/bless_usage.pl";
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": script_uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": "my $obj = bless {}, 'BlessedGreeter';\n$obj->\n"
                }
            }
        }),
    );
    await_open_processing(&server);

    let response = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": script_uri },
                "position": { "line": 1, "character": 6 }
            }
        }),
    );

    let items = completion_items(&response);
    let greet = items
        .iter()
        .find(|item| item["label"].as_str() == Some("greet"))
        .ok_or_else(|| format!("greet completion should be present. Got: {items:#?}"))?;

    let detail = greet["detail"].as_str().ok_or("greet should include detail")?;
    assert!(
        detail.contains("receiver: literal bless, medium confidence"),
        "literal bless completion detail should expose medium-confidence receiver evidence. Got: {detail:?}"
    );

    Ok(())
}

/// Verify that `textEdit` is emitted for qualified variable completions.
///
/// LSP 3.17 §3.16.1: when a `textEdit` is present it takes precedence over
/// `insertText`.  Without `textEdit`, clients insert the full resolved name
/// *after* the typed prefix rather than replacing it, producing "$v$variable"
/// instead of "$variable".
///
/// Setup: index `Cfg.pm` (contains `our $CFG_VALUE`), then request completion
/// at the cursor position after `$Cfg::CF` in a usage file.  The returned item
/// should carry a `textEdit` whose range covers exactly the typed prefix and
/// whose `newText` equals the full qualified label.
#[test]
fn test_completion_textedit_replaces_qualified_prefix() -> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    // Index a module that exposes a package variable.
    let module_uri = "file:///workspace/Cfg.pm";
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": module_uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": "package Cfg;\n\nour $CFG_VALUE = 1;\n\n1;\n"
                }
            }
        }),
    );
    // Wait for the workspace index to incorporate $CFG_VALUE before requesting completion.
    await_module_indexed(&server);

    // Open a usage file with a partially-typed qualified variable.
    //
    // Text (0-indexed columns):
    //   "my $x = $Cfg::CF"
    //    0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5
    //                        1 1 1 1 1 1
    //
    // "my $x = " = 8 chars (cols 0-7).  "$Cfg::CF" = 8 chars (cols 8-15).
    // Cursor is at column 16 (one past the last char).  The typed prefix is
    // "$Cfg::CF" (8 bytes), so the replace range is columns [8, 16) on line 0.
    let script_uri = "file:///workspace/cfg_usage.pl";
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": script_uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": "my $x = $Cfg::CF"
                }
            }
        }),
    );
    await_open_processing(&server);

    // Request completion at end-of-prefix (line 0, char 16).
    let response = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": script_uri },
                "position": { "line": 0, "character": 16 }
            }
        }),
    );

    let items = completion_items(&response);

    // Find the CFG_VALUE completion item.
    let cfg_item = items
        .iter()
        .find(|item| item["label"].as_str().map_or(false, |l| l.contains("CFG_VALUE")))
        .ok_or_else(|| format!("Expected a CFG_VALUE completion item. Got items: {items:?}"))?;

    // The item must carry a `textEdit` field.
    let text_edit =
        cfg_item.get("textEdit").ok_or("Expected textEdit field on CFG_VALUE completion item")?;

    // Range must cover exactly the typed prefix: start col 8, end col 16 on line 0.
    let start = &text_edit["range"]["start"];
    let end = &text_edit["range"]["end"];
    assert_eq!(start["line"], 0, "textEdit start line");
    assert_eq!(start["character"], 8, "textEdit start character (after 'my $x = ')");
    assert_eq!(end["line"], 0, "textEdit end line");
    assert_eq!(end["character"], 16, "textEdit end character (end of '$Cfg::CF')");

    // newText must be the fully-qualified variable name, not just the bare label.
    // An exact-equality check catches the regression this PR fixes: without the
    // textEdit field, clients append the resolved name *after* the prefix; with it,
    // newText must be the complete replacement string "$Cfg::CFG_VALUE".
    let new_text = text_edit["newText"].as_str().ok_or("textEdit.newText must be a string")?;
    assert_eq!(new_text, "$Cfg::CFG_VALUE", "textEdit.newText must be the fully-qualified name");

    Ok(())
}

/// Like `test_completion_textedit_replaces_qualified_prefix` but with a multibyte character
/// before the cursor to verify UTF-16 position encoding.
///
/// The pound sign `£` (U+00A3) is a 2-byte UTF-8 sequence but a single UTF-16 code unit,
/// so UTF-16 column indices equal UTF-8 column indices here (both advance by 1 code unit).
/// This test confirms the offset→position conversion path produces correct UTF-16 columns
/// even when non-ASCII bytes precede the typed prefix.
#[test]
fn test_completion_textedit_utf16_position() -> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    // Index a module with a qualified variable.
    let module_uri = "file:///workspace/Enc.pm";
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": module_uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": "package Enc;\n\nour $ENC_KEY = 'utf8';\n\n1;\n"
                }
            }
        }),
    );
    // Wait for the workspace index to incorporate $ENC_KEY before requesting completion.
    await_module_indexed(&server);

    // Open a usage file.  The `£` sign is U+00A3: 2 UTF-8 bytes, 1 UTF-16 code unit.
    //
    // Text: "my $cost = £$Enc::EN"  (21 bytes total)
    //
    // UTF-8 byte layout:
    //   m  y     $  c  o  s  t     =     £(hi) £(lo) $  E  n  c  :  :  E  N
    //   0  1  2  3  4  5  6  7  8  9 10    11    12  13 14 15 16 17 18 19 20
    //
    // UTF-16 code-unit layout (£ is U+00A3, in BMP → 1 UTF-16 unit):
    //   m  y     $  c  o  s  t     =     £  $  E  n  c  :  :  E  N
    //   0  1  2  3  4  5  6  7  8  9 10 11 12 13 14 15 16 17 18 19
    //
    // "$Enc::EN" starts at byte 13 (UTF-16 col 12) and ends at byte 21
    // (UTF-16 col 20).  The cursor is placed at UTF-16 col 20 (end of prefix).
    let script_uri = "file:///workspace/enc_usage.pl";
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": script_uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": "my $cost = \u{00A3}$Enc::EN"
                }
            }
        }),
    );
    await_open_processing(&server);

    // Cursor at end of `$Enc::EN`:
    //   UTF-8 offset 21 → UTF-16 col 20 (£ costs 2 UTF-8 bytes, 1 UTF-16 unit)
    let response = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": script_uri },
                "position": { "line": 0, "character": 20 }
            }
        }),
    );

    let items = completion_items(&response);

    // Find the ENC_KEY completion item.
    let enc_item = items
        .iter()
        .find(|item| item["label"].as_str().map_or(false, |l| l.contains("ENC_KEY")))
        .ok_or_else(|| format!("Expected an ENC_KEY completion item. Got items: {items:?}"))?;

    let text_edit =
        enc_item.get("textEdit").ok_or("Expected textEdit field on ENC_KEY completion item")?;

    // UTF-16 positions: prefix starts at col 12, ends at col 20.
    let start = &text_edit["range"]["start"];
    let end = &text_edit["range"]["end"];
    assert_eq!(start["line"], 0, "textEdit start line");
    assert_eq!(
        start["character"], 12,
        "textEdit start character (UTF-16 col after 'my $cost = £')"
    );
    assert_eq!(end["line"], 0, "textEdit end line");
    assert_eq!(end["character"], 20, "textEdit end character (UTF-16 col at end of '$Enc::EN')");

    let new_text = text_edit["newText"].as_str().ok_or("textEdit.newText must be a string")?;
    assert_eq!(new_text, "$Enc::ENC_KEY", "textEdit.newText must be the fully-qualified name");

    Ok(())
}
