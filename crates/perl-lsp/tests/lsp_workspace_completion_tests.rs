/// Tests for workspace-aware completion integration
///
/// This module tests that the completion provider properly queries the workspace
/// index to provide cross-file symbol completions.
use serde_json::json;

mod common;
use common::{completion_items, initialize_lsp, send_notification, send_request, start_lsp_server};

/// Test cross-file function completion
///
/// When a user types a function name, the completion provider should suggest
/// functions from other files in the workspace that have been indexed.
#[test]
fn test_completion_cross_file_function() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Index a module file with a function
    let module_uri = "file:///workspace/EmailUtils.pm";
    send_notification(
        &mut server,
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

    // Now open a different file and request completion
    let script_uri = "file:///workspace/script.pl";
    send_notification(
        &mut server,
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

    // Request completion at position after "vali"
    let response = send_request(
        &mut server,
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
#[ignore = "feature: cross-file qualified completion not yet wired to workspace index"]
fn test_completion_cross_file_qualified() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Index a module file
    let module_uri = "file:///workspace/DataProcessor.pm";
    send_notification(
        &mut server,
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

    // Open a file requesting qualified completion
    let script_uri = "file:///workspace/main.pl";
    send_notification(
        &mut server,
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

    // Request completion after "DataProcessor::"
    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": script_uri },
                "position": { "line": 3, "character": 33 }
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

    Ok(())
}

/// Test cross-file variable completion
#[test]
#[ignore = "feature: cross-file variable completion not yet wired to workspace index"]
fn test_completion_cross_file_variable() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Index a module with exported variables
    let module_uri = "file:///workspace/Config.pm";
    send_notification(
        &mut server,
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

    // Open a file requesting variable completion
    let script_uri = "file:///workspace/app.pl";
    send_notification(
        &mut server,
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

    // Request completion after "$Config::CONF"
    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": script_uri },
                "position": { "line": 3, "character": 21 }
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
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Index a module with exported functions
    let module_uri = "file:///workspace/StringUtils.pm";
    send_notification(
        &mut server,
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

    // Open a file that imports the module
    let script_uri = "file:///workspace/text_processor.pl";
    send_notification(
        &mut server,
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

    // Request completion after "tri"
    let response = send_request(
        &mut server,
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
