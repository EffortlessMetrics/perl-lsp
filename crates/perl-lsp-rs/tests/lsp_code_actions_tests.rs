/// Comprehensive tests for LSP code actions and refactorings
use perl_diagnostics::codes::DiagnosticCode;
use serde_json::json;

mod common;
use common::{
    initialize_lsp, send_notification, send_request, shutdown_and_exit, start_lsp_server,
};

/// Test extract variable refactoring
#[test]
fn test_extract_variable() -> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    let uri = "file:///test.pl";
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
my $str = "hello";
my $result = length($str) + 10;
print $result;
"#
                }
            }
        }),
    );

    // Request code actions for the expression "length($str)"
    let response = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/codeAction",
            "params": {
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": 2, "character": 13 },
                    "end": { "line": 2, "character": 25 }
                },
                "context": {
                    "diagnostics": []
                }
            }
        }),
    );

    let actions = response["result"].as_array().ok_or("Expected result to be an array")?;
    assert!(actions.iter().any(|a| {
        let title = a["title"].as_str().unwrap_or("");
        title.contains("Extract") && title.contains("variable")
    }));
    shutdown_and_exit(&server);
    Ok(())
}

/// Test adding error checking to file operations
#[test]
fn test_add_error_checking() -> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    let uri = "file:///test.pl";
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
open($fh, '<', 'data.txt');
print "Hello\n";
close($fh);
"#
                }
            }
        }),
    );

    // Request code actions for the open statement
    let response = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/codeAction",
            "params": {
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": 1, "character": 0 },
                    "end": { "line": 1, "character": 30 }
                },
                "context": {
                    "diagnostics": []
                }
            }
        }),
    );

    let actions = response["result"].as_array().ok_or("Expected result to be an array")?;
    assert!(actions.iter().any(|a| a["title"].as_str().unwrap_or("").contains("error checking")));
    shutdown_and_exit(&server);
    Ok(())
}

/// Test converting old-style for loops to foreach
#[test]
fn test_convert_loop_style() -> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    let uri = "file:///test.pl";
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
for (my $i = 0; $i < @array; $i++) {
    print $array[$i];
}
"#
                }
            }
        }),
    );

    // Request code actions for the for loop
    let response = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "textDocument/codeAction",
            "params": {
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": 1, "character": 0 },
                    "end": { "line": 3, "character": 1 }
                },
                "context": {
                    "diagnostics": []
                }
            }
        }),
    );

    let actions = response["result"].as_array().ok_or("Expected result to be an array")?;
    assert!(
        actions.iter().any(|a| a["title"].as_str().unwrap_or("").contains("foreach loop")),
        "Expected 'foreach loop' conversion action but got: {:?}",
        actions.iter().map(|a| a["title"].as_str()).collect::<Vec<_>>()
    );
    shutdown_and_exit(&server);
    Ok(())
}

/// Test converting to postfix form
#[test]
fn test_convert_to_postfix() -> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    let uri = "file:///test.pl";
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
if ($debug) {
    print "Debug mode\n";
}
"#
                }
            }
        }),
    );

    // Request code actions for the if statement
    let response = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "textDocument/codeAction",
            "params": {
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": 1, "character": 0 },
                    "end": { "line": 3, "character": 1 }
                },
                "context": {
                    "diagnostics": []
                }
            }
        }),
    );

    let actions = response["result"].as_array().ok_or("Expected result to be an array")?;
    assert!(actions.iter().any(|a| a["title"].as_str().unwrap_or("").contains("postfix")));
    shutdown_and_exit(&server);
    Ok(())
}

/// Test adding missing pragmas
#[test]
fn test_add_missing_pragmas() -> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    let uri = "file:///test.pl";
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
#!/usr/bin/perl

my $x = 42;
print $x;
"#
                }
            }
        }),
    );

    // Request code actions for the entire document
    let response = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "textDocument/codeAction",
            "params": {
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": 0, "character": 0 },
                    "end": { "line": 4, "character": 0 }
                },
                "context": {
                    "diagnostics": []
                }
            }
        }),
    );

    let actions = response["result"].as_array().ok_or("Expected result to be an array")?;
    assert!(actions.iter().any(|a| a["title"].as_str().unwrap_or("").contains("pragma")));
    shutdown_and_exit(&server);
    Ok(())
}

/// Test quick fix for undefined variable
#[test]
fn test_fix_undefined_variable() -> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    let uri = "file:///test.pl";
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
use strict;
use warnings;

print $undefined_var;
"#
                }
            }
        }),
    );

    // First get diagnostics
    let diag_response = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "textDocument/diagnostic",
            "params": {
                "textDocument": { "uri": uri }
            }
        }),
    );

    // Request code actions with diagnostics
    let response = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "textDocument/codeAction",
            "params": {
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": 4, "character": 6 },
                    "end": { "line": 4, "character": 20 }
                },
                "context": {
                    "diagnostics": diag_response["result"]["items"].clone()
                }
            }
        }),
    );

    let actions = response["result"].as_array().ok_or("Expected result to be an array")?;
    assert!(actions.iter().any(|a| {
        let title = a["title"].as_str().unwrap_or("");
        title.contains("Declare") && title.contains("my")
    }));
    shutdown_and_exit(&server);
    Ok(())
}

/// Test quick fixes preserve associated diagnostics in the LSP response
#[test]
fn test_quickfix_actions_include_associated_diagnostics() -> Result<(), Box<dyn std::error::Error>>
{
    let server = start_lsp_server();
    initialize_lsp(&server);

    let uri = "file:///test.pl";
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
use strict;
use warnings;

print $undefined_var;
"#
                }
            }
        }),
    );

    let diag_response = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 69,
            "method": "textDocument/diagnostic",
            "params": {
                "textDocument": { "uri": uri }
            }
        }),
    );
    let reported_diagnostics =
        diag_response["result"]["items"].as_array().ok_or("Expected diagnostics result items")?;
    let reported_diagnostic = reported_diagnostics
        .iter()
        .find(|diagnostic| {
            matches!(
                diagnostic["code"].as_str(),
                Some(code)
                    if code == DiagnosticCode::UndefinedVariable.as_str()
                        || matches!(code, "undeclared-variable" | "undefined-variable")
            )
        })
        .ok_or("Expected undefined-variable style diagnostic in pull diagnostics")?;

    let response = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 70,
            "method": "textDocument/codeAction",
            "params": {
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": 4, "character": 6 },
                    "end": { "line": 4, "character": 20 }
                },
                "context": {
                    "diagnostics": diag_response["result"]["items"].clone()
                }
            }
        }),
    );

    let actions = response["result"].as_array().ok_or("Expected result to be an array")?;
    let declare_action = actions
        .iter()
        .find(|action| action["title"].as_str() == Some("Declare '$undefined_var' with 'my'"))
        .ok_or("Expected quick fix for undefined variable")?;

    let diagnostics = declare_action["diagnostics"]
        .as_array()
        .ok_or("Expected quick fix to include associated diagnostics")?;
    assert_eq!(diagnostics.len(), 1);

    let diagnostic = &diagnostics[0];
    assert_eq!(diagnostic["range"], reported_diagnostic["range"]);
    assert_eq!(diagnostic["severity"], reported_diagnostic["severity"]);
    assert_eq!(diagnostic["code"], reported_diagnostic["code"]);
    assert_eq!(diagnostic["source"], reported_diagnostic["source"]);
    assert_eq!(diagnostic["message"], reported_diagnostic["message"]);

    shutdown_and_exit(&server);
    Ok(())
}

/// Test extract subroutine refactoring
#[test]
fn test_extract_subroutine() -> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    let uri = "file:///test.pl";
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
my $x = 10;
my $y = 20;
{
    my $sum = $x + $y;
    print "Sum: $sum\n";
    my $product = $x * $y;
    print "Product: $product\n";
}
"#
                }
            }
        }),
    );

    // Request code actions for the block
    let response = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 8,
            "method": "textDocument/codeAction",
            "params": {
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": 3, "character": 0 },
                    "end": { "line": 8, "character": 1 }
                },
                "context": {
                    "diagnostics": []
                }
            }
        }),
    );

    let actions = response["result"].as_array().ok_or("Expected result to be an array")?;
    assert!(actions.iter().any(|a| a["title"].as_str().unwrap_or("").contains("subroutine")));
    shutdown_and_exit(&server);
    Ok(())
}

/// Test organize imports refactoring
#[test]
fn test_organize_imports() -> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    let uri = "file:///test.pl";
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
#!/usr/bin/perl
use JSON;
use Data::Dumper;
use warnings;
use File::Path;
use strict;
use lib './lib';

print "test\n";
"#
                }
            }
        }),
    );

    // Request code actions for the import section
    let response = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "textDocument/codeAction",
            "params": {
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": 1, "character": 0 },
                    "end": { "line": 7, "character": 0 }
                },
                "context": {
                    "diagnostics": []
                }
            }
        }),
    );

    let actions = response["result"].as_array().ok_or("Expected result to be an array")?;
    assert!(actions.iter().any(|a| a["title"].as_str().unwrap_or("").contains("Organize imports")));
    shutdown_and_exit(&server);
    Ok(())
}

/// Test multiple code action kinds available for the same selection
#[test]
fn test_multiple_code_action_kinds() -> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    let uri = "file:///test.pl";
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
use strict;
use warnings;

open(my $fh, '<', 'data.txt');
"#
                }
            }
        }),
    );

    // Request code actions for the file operation. Current behavior offers a
    // refactor plus other applicable action kinds for the same selection.
    let response = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "textDocument/codeAction",
            "params": {
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": 4, "character": 0 },
                    "end": { "line": 4, "character": 30 }
                },
                "context": {
                    "diagnostics": []
                }
            }
        }),
    );

    let actions = response["result"].as_array().ok_or("Expected result to be an array")?;

    // Should have multiple action kinds available for the same selection.
    assert!(!actions.is_empty(), "Expected code actions but got none");
    assert!(actions.iter().any(|a| a["kind"].as_str() == Some("refactor.rewrite")));
    assert!(actions.iter().any(|a| a["kind"].as_str() == Some("quickfix")));
    shutdown_and_exit(&server);
    Ok(())
}

/// Test that context.only filters code actions to the requested kind family
#[test]
fn test_context_only_filters_to_requested_code_action_kinds()
-> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    let uri = "file:///test.pl";
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
use strict;
use warnings;

open(my $fh, '<', 'data.txt');
"#
                }
            }
        }),
    );

    let response = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "id": 11,
            "method": "textDocument/codeAction",
            "params": {
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": 4, "character": 0 },
                    "end": { "line": 4, "character": 30 }
                },
                "context": {
                    "diagnostics": [],
                    "only": ["refactor"]
                }
            }
        }),
    );

    let actions = response["result"].as_array().ok_or("Expected result to be an array")?;

    assert!(!actions.is_empty(), "Expected refactor code actions but got none");
    assert!(actions.iter().all(|action| {
        action["kind"]
            .as_str()
            .is_some_and(|kind| kind == "refactor" || kind.starts_with("refactor."))
    }));
    assert!(actions.iter().any(|action| action["kind"].as_str() == Some("refactor.rewrite")));
    assert!(!actions.iter().any(|action| action["kind"].as_str() == Some("quickfix")));

    shutdown_and_exit(&server);
    Ok(())
}
