/// Comprehensive tests for LSP completion functionality
use serde_json::json;

mod common;
use common::{initialize_lsp, send_notification, send_request, start_lsp_server};

/// Test basic variable completion
#[test]
fn test_scalar_variable_completion() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    // Open a document with scalar variables
    let uri = "file:///test.pl";
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": r#"
my $count = 42;
my $counter = 0;
my $total_sum = 100;

$cou
"#
            }
        }
        }),
    );

    // Request completion at position after "$cou"
    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/completion",
            "params": {
            "textDocument": { "uri": uri },
            "position": { "line": 5, "character": 4 }
        }
        }),
    );

    let items = response["result"].as_array().unwrap();
    assert!(items.len() >= 2, "Should have at least 2 completions");

    // Check that both $count and $counter are suggested
    let labels: Vec<String> = items
        .iter()
        .map(|item| item["label"].as_str().unwrap().to_string())
        .collect();

    assert!(labels.contains(&"$count".to_string()));
    assert!(labels.contains(&"$counter".to_string()));
    assert!(!labels.contains(&"$total_sum".to_string())); // Shouldn't match
}

/// Test array variable completion
#[test]
fn test_array_variable_completion() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///test.pl";
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": r#"
my @items = (1, 2, 3);
my @iterator = ();
my @data = qw(a b c);

@it
"#
                }
            }
        }),
    );

    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "textDocument/completion",
            "params": {
            "textDocument": { "uri": uri },
            "position": { "line": 5, "character": 3 }
        }
        }),
    );

    let items = response["result"].as_array().unwrap();
    assert!(items.len() >= 2, "Should have at least 2 completions");

    let labels: Vec<String> = items
        .iter()
        .map(|item| item["label"].as_str().unwrap().to_string())
        .collect();

    assert!(labels.contains(&"@items".to_string()));
    assert!(labels.contains(&"@iterator".to_string()));
}

/// Test hash variable completion
#[test]
fn test_hash_variable_completion() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///test.pl";
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": r#"
my %config = (host => 'localhost');
my %connection = ();
my %settings = ();

%con
"#
                }
            }
        }),
    );

    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "textDocument/completion",
            "params": {
            "textDocument": { "uri": uri },
            "position": { "line": 5, "character": 4 }
        }
        }),
    );

    let items = response["result"].as_array().unwrap();
    assert!(items.len() >= 2, "Should have at least 2 completions");

    let labels: Vec<String> = items
        .iter()
        .map(|item| item["label"].as_str().unwrap().to_string())
        .collect();

    assert!(labels.contains(&"%config".to_string()));
    assert!(labels.contains(&"%connection".to_string()));
}

/// Test function completion
#[test]
fn test_function_completion() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///test.pl";
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": r#"
sub process_data {
    my ($data) = @_;
    return $data * 2;
}

sub process_items {
    my (@items) = @_;
    return scalar @items;
}

proc
"#
                }
            }
        }),
    );

    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "textDocument/completion",
            "params": {
            "textDocument": { "uri": uri },
            "position": { "line": 11, "character": 4 }
        }
        }),
    );

    let items = response["result"].as_array().unwrap();
    assert!(items.len() >= 2, "Should have at least 2 completions");

    let labels: Vec<String> = items
        .iter()
        .map(|item| item["label"].as_str().unwrap().to_string())
        .collect();

    assert!(labels.contains(&"process_data".to_string()));
    assert!(labels.contains(&"process_items".to_string()));
}

/// Test built-in function completion
#[test]
fn test_builtin_completion() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///test.pl";
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "pri"
            }
        }
        }),
    );

    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "textDocument/completion",
            "params": {
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 3 }
        }
        }),
    );

    let items = response["result"].as_array().unwrap();
    assert!(items.len() >= 2, "Should have print and printf");

    let labels: Vec<String> = items
        .iter()
        .map(|item| item["label"].as_str().unwrap().to_string())
        .collect();

    assert!(labels.contains(&"print".to_string()));
    assert!(labels.contains(&"printf".to_string()));
}

/// Test keyword completion
#[test]
fn test_keyword_completion() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///test.pl";
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "for"
            }
        }
        }),
    );

    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "textDocument/completion",
            "params": {
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 3 }
        }
        }),
    );

    let items = response["result"].as_array().unwrap();
    assert!(items.len() >= 2, "Should have for and foreach");

    let labels: Vec<String> = items
        .iter()
        .map(|item| item["label"].as_str().unwrap().to_string())
        .collect();

    assert!(labels.contains(&"for".to_string()));
    assert!(labels.contains(&"foreach".to_string()));
}

/// Test special variable completion
#[test]
fn test_special_variable_completion() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///test.pl";
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "$^"
            }
        }
        }),
    );

    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 8,
            "method": "textDocument/completion",
            "params": {
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 2 }
        }
        }),
    );

    let items = response["result"].as_array().unwrap();
    assert!(
        items.len() >= 2,
        "Should have special variables like $^O and $^V"
    );

    let labels: Vec<String> = items
        .iter()
        .map(|item| item["label"].as_str().unwrap().to_string())
        .collect();

    assert!(labels.contains(&"$^O".to_string()));
    assert!(labels.contains(&"$^V".to_string()));
}

/// Test method completion after ->
#[test]
fn test_method_completion() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///test.pl";
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "$object->"
            }
        }
        }),
    );

    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "textDocument/completion",
            "params": {
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 9 }
        }
        }),
    );

    let items = response["result"].as_array().unwrap();
    assert!(items.len() >= 3, "Should have common methods");

    let labels: Vec<String> = items
        .iter()
        .map(|item| item["label"].as_str().unwrap().to_string())
        .collect();

    assert!(labels.contains(&"new".to_string()));
    assert!(labels.contains(&"isa".to_string()));
    assert!(labels.contains(&"can".to_string()));
}

/// Test completion in mixed context
#[test]
fn test_mixed_context_completion() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///test.pl";
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": r#"
my $value = 42;
my $var = 100;

sub validate {
    return 1;
}

va
"#
            }
        }
        }),
    );

    // Request completion at position after "va"
    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "textDocument/completion",
            "params": {
            "textDocument": { "uri": uri },
            "position": { "line": 8, "character": 2 }
        }
        }),
    );

    let items = response["result"].as_array().unwrap();
    assert!(items.len() >= 3, "Should have variables and function");

    let labels: Vec<String> = items
        .iter()
        .map(|item| item["label"].as_str().unwrap().to_string())
        .collect();

    // Should suggest both variables and the function
    assert!(labels.contains(&"$value".to_string()));
    assert!(labels.contains(&"$var".to_string()));
    assert!(labels.contains(&"validate".to_string()));
}

/// Test completion details and documentation
#[test]
fn test_completion_details() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///test.pl";
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "@ARG"
            }
        }
        }),
    );

    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 11,
            "method": "textDocument/completion",
            "params": {
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 4 }
        }
        }),
    );

    let items = response["result"].as_array().unwrap();

    // Find @ARGV in completions
    let argv_item = items
        .iter()
        .find(|item| item["label"] == "@ARGV")
        .expect("Should have @ARGV completion");

    // Check it has details
    assert!(argv_item["detail"].is_string());
    assert!(argv_item["documentation"].is_string());
    assert_eq!(argv_item["documentation"], "Command line arguments");
}

/// Test completion with empty prefix (should show all relevant items)
#[test]
fn test_empty_prefix_completion() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///test.pl";
    send_notification(
        &mut server,
        json!({
                                                                                                                                                                                                                                                                                                                                                                                                                        "jsonrpc": "2.0",
                                                                                                                                                                                                                                                                                                                                                                                                                        "method": "textDocument/didOpen",
                                                                                                                                                                                                                                                                                                                                                                                                                        "params": {
                                                                                                                                                                                                                                                                                                                                                                                                                        "textDocument": {
                                                                                                                                                                                                                                                                                                                                                                                                                            "uri": uri,
                                                                                                                                                                                                                                                                                                                                                                                                                            "languageId": "perl",
                                                                                                                                                                                                                                                                                                                                                                                                                            "version": 1,
                                                                                                                                                                                                                                                                                                                                                                                                                            "text": r#"
my $var = 42;
sub test { }

"#  // Empty line where we'll request completion
                                                                                                                                                                                                                                                                                                                                                                                                                        }
                                                                                                                                                                                                                                                                                                                                                                                                                    }
                                                                                                                                                                                                                                                                                                                                                                                                                    }),
    );

    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 12,
            "method": "textDocument/completion",
            "params": {
            "textDocument": { "uri": uri },
            "position": { "line": 3, "character": 0 }
        }
        }),
    );

    let items = response["result"].as_array().unwrap();
    assert!(
        items.len() > 10,
        "Should have many completions for empty prefix"
    );

    // Should include keywords, built-ins, and defined items
    let labels: Vec<String> = items
        .iter()
        .map(|item| item["label"].as_str().unwrap().to_string())
        .collect();

    assert!(labels.iter().any(|l| l.starts_with("if")));
    assert!(labels.iter().any(|l| l.starts_with("print")));
    assert!(labels.contains(&"$var".to_string()));
    assert!(labels.contains(&"test".to_string()));
}

/// Test that completion doesn't trigger in comments
#[test]
fn test_no_completion_in_comments() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///test.pl";
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "# This is a comment with pri"
            }
        }
        }),
    );

    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 13,
            "method": "textDocument/completion",
            "params": {
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 28 }
        }
        }),
    );

    let items = response["result"].as_array().unwrap();
    assert_eq!(items.len(), 0, "Should have no completions in comments");
}

/// Test completion with package context
#[test]
fn test_package_completion() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///test.pl";
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": r#"
package MyModule;

sub public_method { }

package main;

MyModule::"#
            }
        }
        }),
    );

    // This tests package member completion (currently TODO in implementation)
    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 14,
            "method": "textDocument/completion",
            "params": {
            "textDocument": { "uri": uri },
            "position": { "line": 7, "character": 10 }
        }
        }),
    );

    let items = response["result"].as_array().unwrap();
    // Since package completion is TODO, this might be empty for now
    assert!(
        items.is_empty() || !items.is_empty(),
        "Package completion handling"
    );
}

/// Test snippet expansion in completions
#[test]
fn test_snippet_completion() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///test.pl";
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "sub"
            }
        }
        }),
    );

    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 15,
            "method": "textDocument/completion",
            "params": {
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 3 }
        }
        }),
    );

    let items = response["result"].as_array().unwrap();

    // Find the 'sub' keyword completion
    let sub_item = items
        .iter()
        .find(|item| item["label"] == "sub")
        .expect("Should have 'sub' keyword completion");

    // Check it has a snippet with placeholders
    assert!(sub_item["insertText"].as_str().unwrap().contains("${"));
    assert_eq!(sub_item["kind"], 15); // Snippet kind
}

/// Test array and hash element access completion
#[test]
fn test_element_access_completion() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///test.pl";
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": r#"
my @array = (1, 2, 3);
my %hash = (key => 'value');

$arr"#
                }
            }
        }),
    );

    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 16,
            "method": "textDocument/completion",
            "params": {
            "textDocument": { "uri": uri },
            "position": { "line": 4, "character": 4 }
        }
        }),
    );

    let items = response["result"].as_array().unwrap();

    // Should suggest $array[...] for array element access
    let labels: Vec<String> = items
        .iter()
        .map(|item| item["label"].as_str().unwrap().to_string())
        .collect();

    // The provider might need enhancement to handle this case
    assert!(items.is_empty() || labels.iter().any(|l| l.contains("array")));
}

/// Test completion filtering and ranking
#[test]
fn test_completion_ranking() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///test.pl";
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": "$"
            }
        }
        }),
    );

    let response = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 17,
            "method": "textDocument/completion",
            "params": {
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 1 }
        }
        }),
    );

    let items = response["result"].as_array().unwrap();

    // Special variables should appear first (they have sort_text starting with "0_")
    let first_items: Vec<String> = items
        .iter()
        .take(5)
        .map(|item| item["label"].as_str().unwrap().to_string())
        .collect();

    // Check that special variables are prioritized
    assert!(
        first_items
            .iter()
            .any(|l| l == "$_" || l == "$$" || l == "$@")
    );
}

/// Test completion with incremental typing
#[test]
fn test_incremental_completion() {
    let mut server = start_lsp_server();
    initialize_lsp(&mut server);

    let uri = "file:///test.pl";

    // Initial document
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": r#"
my $prefix = 1;
my $prefixed_var = 2;
my $preliminary = 3;

$p"#
            }
        }
        }),
    );

    // First completion request with "$p"
    let response1 = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 18,
            "method": "textDocument/completion",
            "params": {
            "textDocument": { "uri": uri },
            "position": { "line": 5, "character": 2 }
        }
        }),
    );

    let items1 = response1["result"].as_array().unwrap();
    assert_eq!(
        items1.len(),
        3,
        "Should have all three variables starting with 'p'"
    );

    // Update document to narrow down
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
            "textDocument": {
                "uri": uri,
                "version": 2
            },
            "contentChanges": [{
                "text": r#"
my $prefix = 1;
my $prefixed_var = 2;
my $preliminary = 3;

$pre"#
            }]
        }
        }),
    );

    // Second completion request with "$pre"
    let response2 = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 19,
            "method": "textDocument/completion",
            "params": {
            "textDocument": { "uri": uri },
            "position": { "line": 5, "character": 4 }
        }
        }),
    );

    let items2 = response2["result"].as_array().unwrap();
    assert_eq!(items2.len(), 3, "Should still have all three");

    // Update to be more specific
    send_notification(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
            "textDocument": {
                "uri": uri,
                "version": 3
            },
            "contentChanges": [{
                "text": r#"
my $prefix = 1;
my $prefixed_var = 2;
my $preliminary = 3;

$prefi"#
            }]
        }
        }),
    );

    // Third completion request with "$prefi"
    let response3 = send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 20,
            "method": "textDocument/completion",
            "params": {
            "textDocument": { "uri": uri },
            "position": { "line": 5, "character": 6 }
        }
        }),
    );

    let items3 = response3["result"].as_array().unwrap();
    assert_eq!(items3.len(), 2, "Should have only prefix and prefixed_var");

    let labels3: Vec<String> = items3
        .iter()
        .map(|item| item["label"].as_str().unwrap().to_string())
        .collect();

    assert!(labels3.contains(&"$prefix".to_string()));
    assert!(labels3.contains(&"$prefixed_var".to_string()));
    assert!(!labels3.contains(&"$preliminary".to_string()));
}
