mod support;

use perl_parser::lsp_server::LspServer;
use serde_json::json;
use std::io::Cursor;
use std::sync::{Arc, Mutex};
use support::assert_call_hierarchy_items;

#[test]
fn test_call_hierarchy_prepare() {
    let mut server = LspServer::with_output(Arc::new(Mutex::new(
        Box::new(Cursor::new(Vec::new())) as Box<dyn std::io::Write + Send>,
    )));

    // Initialize
    let _ = server.handle_request(
        serde_json::from_value(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": null,
                "rootUri": null,
                "capabilities": {}
            }
        }))
        .unwrap(),
    );
    let _ = server.handle_request(
        serde_json::from_value(json!({"jsonrpc":"2.0","method":"initialized","params":{}}))
            .unwrap(),
    );

    // Open document
    let _ = server.handle_request(
        serde_json::from_value(json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///test_hierarchy.pl",
                    "languageId": "perl",
                    "version": 1,
                    "text": r#"
sub main {
    helper();
    process_data();
}

sub helper {
    print "Helper\n";
}

sub process_data {
    helper();
    $obj->helper();
}
1;
"#
                }
            }
        }))
        .unwrap(),
    );

    // Prepare call hierarchy at main
    let result = server
        .handle_request(
            serde_json::from_value(json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "textDocument/prepareCallHierarchy",
                "params": {
                    "textDocument": {"uri": "file:///test_hierarchy.pl"},
                    "position": {"line": 1, "character": 4}
                }
            }))
            .unwrap(),
        )
        .and_then(|r| r.result);

    assert_call_hierarchy_items(&result, Some("main"));
}
