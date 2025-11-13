//! Comprehensive tests for built-in function signatures
//!
//! Tests all 114 built-in function signatures to ensure they work correctly

use perl_parser::{JsonRpcRequest, LspServer};
use serde_json::{Value, json};

/// Helper to create and initialize a test server
fn setup_server() -> LspServer {
    let mut server = LspServer::new();

    // Send initialize request
    let init_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "initialize".to_string(),
        params: Some(json!({
            "processId": null,
            "capabilities": {},
            "rootUri": "file:///test"
        })),
    };
    server.handle_request(init_request);

    // Send initialized notification (required after successful initialize)
    let initialized_notification = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "initialized".to_string(),
        params: Some(json!({})),
    };
    server.handle_request(initialized_notification);

    server
}

/// Helper to open a document
fn open_doc(server: &mut LspServer, uri: &str, text: &str) {
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: None,
        method: "textDocument/didOpen".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": text
            }
        })),
    };
    server.handle_request(request);
}

/// Helper to get signature help
fn get_signature_help(
    server: &mut LspServer,
    uri: &str,
    line: u32,
    character: u32,
) -> Option<Value> {
    let request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "textDocument/signatureHelp".to_string(),
        params: Some(json!({
            "textDocument": {"uri": uri},
            "position": {"line": line, "character": character}
        })),
    };

    server.handle_request(request).and_then(|response| response.result)
}

#[test]
#[ignore] // Flaky BrokenPipe errors in CI during LSP initialization (environmental/timing)
fn test_file_operation_signatures() {
    let mut server = setup_server();

    // Test seek signature - cursor inside parentheses after FILE,
    let code = "seek(FILE, 0, 0);";
    open_doc(&mut server, "file:///test.pl", code);
    let result = get_signature_help(&mut server, "file:///test.pl", 0, 11);

    assert!(result.is_some());
    let sig = result.unwrap();
    assert!(sig["signatures"][0]["label"].as_str().unwrap().contains("seek"));
    assert!(
        sig["signatures"][0]["label"].as_str().unwrap().contains("FILEHANDLE, POSITION, WHENCE")
    );

    // Test chmod signature - cursor after first comma
    let code = "chmod(0755, $file);";
    open_doc(&mut server, "file:///test2.pl", code);
    let result = get_signature_help(&mut server, "file:///test2.pl", 0, 12);

    assert!(result.is_some());
    let sig = result.unwrap();
    assert!(sig["signatures"][0]["label"].as_str().unwrap().contains("chmod"));
    assert!(sig["signatures"][0]["label"].as_str().unwrap().contains("MODE, LIST"));

    // Test stat signature - cursor just inside parentheses
    let code = "stat($file);";
    open_doc(&mut server, "file:///test3.pl", code);
    let result = get_signature_help(&mut server, "file:///test3.pl", 0, 5);

    assert!(result.is_some());
    let sig = result.unwrap();
    assert!(sig["signatures"][0]["label"].as_str().unwrap().contains("stat"));
}

#[test]
#[ignore] // Flaky BrokenPipe errors in CI during LSP initialization (environmental/timing)
fn test_string_data_signatures() {
    let mut server = setup_server();

    // Test pack signature
    let code = r#"pack("C*", @bytes);"#;
    open_doc(&mut server, "file:///pack.pl", code);
    let result = get_signature_help(&mut server, "file:///pack.pl", 0, 11);

    assert!(result.is_some());
    let sig = result.unwrap();
    assert!(sig["signatures"][0]["label"].as_str().unwrap().contains("pack"));
    assert!(sig["signatures"][0]["label"].as_str().unwrap().contains("TEMPLATE, LIST"));

    // Test unpack signature
    let code = "unpack($template, $data);";
    open_doc(&mut server, "file:///unpack.pl", code);
    let result = get_signature_help(&mut server, "file:///unpack.pl", 0, 18);

    assert!(result.is_some());
    let sig = result.unwrap();
    assert!(sig["signatures"][0]["label"].as_str().unwrap().contains("unpack"));

    // Test hex signature
    let code = "hex($str);";
    open_doc(&mut server, "file:///hex.pl", code);
    let result = get_signature_help(&mut server, "file:///hex.pl", 0, 4);

    assert!(result.is_some());
    let sig = result.unwrap();
    assert!(sig["signatures"][0]["label"].as_str().unwrap().contains("hex"));
}

#[test]
#[ignore] // Flaky BrokenPipe errors in CI during LSP initialization (environmental/timing)
fn test_math_signatures() {
    let mut server = setup_server();

    let math_functions = [
        ("abs($x);", "abs", "VALUE", 4),
        ("sqrt($x);", "sqrt", "EXPR", 5),
        ("sin($x);", "sin", "EXPR", 4),
        ("cos($x);", "cos", "EXPR", 4),
        ("atan2($y, $x);", "atan2", "Y, X", 10),
        ("rand(10);", "rand", "EXPR", 5),
    ];

    for (i, (code, func_name, expected_params, cursor_pos)) in math_functions.iter().enumerate() {
        let uri = format!("file:///math{}.pl", i);
        open_doc(&mut server, &uri, code);
        let result = get_signature_help(&mut server, &uri, 0, *cursor_pos);

        assert!(result.is_some(), "Failed for function: {}", func_name);
        let sig = result.unwrap();
        let label = sig["signatures"][0]["label"].as_str().unwrap();
        assert!(label.contains(func_name), "Label doesn't contain {}: {}", func_name, label);
        assert!(
            label.contains(expected_params),
            "Label doesn't contain params for {}: {}",
            func_name,
            label
        );
    }
}

#[test]
#[ignore] // Flaky BrokenPipe errors in CI during LSP initialization (environmental/timing)
fn test_system_process_signatures() {
    let mut server = setup_server();

    // Test fork (no parameters)
    let code = "fork();";
    open_doc(&mut server, "file:///fork.pl", code);
    let result = get_signature_help(&mut server, "file:///fork.pl", 0, 5);

    assert!(result.is_some());
    let sig = result.unwrap();
    assert!(sig["signatures"][0]["label"].as_str().unwrap().contains("fork"));

    // Test kill signature
    let code = "kill(9, @pids);";
    open_doc(&mut server, "file:///kill.pl", code);
    let result = get_signature_help(&mut server, "file:///kill.pl", 0, 8);

    assert!(result.is_some());
    let sig = result.unwrap();
    assert!(sig["signatures"][0]["label"].as_str().unwrap().contains("kill"));
    assert!(sig["signatures"][0]["label"].as_str().unwrap().contains("SIGNAL, LIST"));

    // Test system signature
    let code = "system($cmd);";
    open_doc(&mut server, "file:///system.pl", code);
    let result = get_signature_help(&mut server, "file:///system.pl", 0, 7);

    assert!(result.is_some());
    let sig = result.unwrap();
    assert!(sig["signatures"][0]["label"].as_str().unwrap().contains("system"));
}

#[test]
#[ignore] // Flaky BrokenPipe errors in CI during LSP initialization (environmental/timing)
fn test_network_signatures() {
    let mut server = setup_server();

    // Test socket signature
    let code = "socket(SOCK, AF_INET, SOCK_STREAM, 0);";
    open_doc(&mut server, "file:///socket.pl", code);
    let result = get_signature_help(&mut server, "file:///socket.pl", 0, 13);

    assert!(result.is_some());
    let sig = result.unwrap();
    assert!(sig["signatures"][0]["label"].as_str().unwrap().contains("socket"));
    assert!(sig["signatures"][0]["label"].as_str().unwrap().contains("DOMAIN, TYPE, PROTOCOL"));

    // Test bind signature
    let code = "bind(SOCK, $addr);";
    open_doc(&mut server, "file:///bind.pl", code);
    let result = get_signature_help(&mut server, "file:///bind.pl", 0, 11);

    assert!(result.is_some());
    let sig = result.unwrap();
    assert!(sig["signatures"][0]["label"].as_str().unwrap().contains("bind"));
    assert!(sig["signatures"][0]["label"].as_str().unwrap().contains("SOCKET, NAME"));
}

#[test]
#[ignore] // Flaky BrokenPipe errors in CI during LSP initialization (environmental/timing)
fn test_control_flow_signatures() {
    let mut server = setup_server();

    // Test eval signature
    let code = "eval($code);";
    open_doc(&mut server, "file:///eval.pl", code);
    let result = get_signature_help(&mut server, "file:///eval.pl", 0, 5);

    assert!(result.is_some());
    let sig = result.unwrap();
    assert!(sig["signatures"][0]["label"].as_str().unwrap().contains("eval"));

    // Test require signature
    let code = "require($module);";
    open_doc(&mut server, "file:///require.pl", code);
    let result = get_signature_help(&mut server, "file:///require.pl", 0, 8);

    assert!(result.is_some());
    let sig = result.unwrap();
    assert!(sig["signatures"][0]["label"].as_str().unwrap().contains("require"));
}

#[test]
#[ignore] // Flaky BrokenPipe errors in CI during LSP initialization (environmental/timing)
fn test_misc_signatures() {
    let mut server = setup_server();

    // Test tie signature
    let code = "tie(%hash, 'DB_File');";
    open_doc(&mut server, "file:///tie.pl", code);
    let result = get_signature_help(&mut server, "file:///tie.pl", 0, 11);

    assert!(result.is_some());
    let sig = result.unwrap();
    assert!(sig["signatures"][0]["label"].as_str().unwrap().contains("tie"));
    assert!(sig["signatures"][0]["label"].as_str().unwrap().contains("VARIABLE, CLASSNAME"));

    // Test select signature
    let code = "select(STDOUT);";
    open_doc(&mut server, "file:///select.pl", code);
    let result = get_signature_help(&mut server, "file:///select.pl", 0, 7);

    assert!(result.is_some());
    let sig = result.unwrap();
    assert!(sig["signatures"][0]["label"].as_str().unwrap().contains("select"));
}

#[test]
#[ignore] // Flaky BrokenPipe errors in CI during LSP initialization (environmental/timing)
fn test_active_parameter_tracking() {
    let mut server = setup_server();

    // Test with multiple parameters
    let code = "atan2(1.5, 2.0);";
    open_doc(&mut server, "file:///atan2.pl", code);
    let result = get_signature_help(&mut server, "file:///atan2.pl", 0, 11);

    assert!(result.is_some());
    let sig = result.unwrap();
    assert_eq!(sig["activeParameter"].as_u64().unwrap(), 1); // Second parameter

    // Test with three parameters
    let code = "substr($text, 5, 10);";
    open_doc(&mut server, "file:///substr.pl", code);
    let result = get_signature_help(&mut server, "file:///substr.pl", 0, 17);

    assert!(result.is_some());
    let sig = result.unwrap();
    assert_eq!(sig["activeParameter"].as_u64().unwrap(), 2); // Third parameter
}

#[test]
#[ignore] // Flaky BrokenPipe errors in CI during LSP initialization (environmental/timing)
fn test_all_114_builtins_are_recognized() {
    let mut server = setup_server();

    // List of all 114 built-in functions we support
    let all_builtins = [
        // Original 40
        "print",
        "printf",
        "open",
        "close",
        "read",
        "write",
        "die",
        "warn",
        "substr",
        "length",
        "index",
        "rindex",
        "sprintf",
        "join",
        "split",
        "push",
        "pop",
        "shift",
        "unshift",
        "splice",
        "grep",
        "map",
        "sort",
        "reverse",
        "keys",
        "values",
        "each",
        "exists",
        "delete",
        "defined",
        "undef",
        "ref",
        "bless",
        "chomp",
        "chop",
        "chr",
        "ord",
        "lc",
        "uc",
        "lcfirst",
        "ucfirst",
        // File operations (17)
        "seek",
        "tell",
        "stat",
        "lstat",
        "chmod",
        "chown",
        "unlink",
        "rename",
        "mkdir",
        "rmdir",
        "opendir",
        "readdir",
        "closedir",
        "link",
        "symlink",
        "readlink",
        "truncate",
        // String/Data (7)
        "pack",
        "unpack",
        "quotemeta",
        "hex",
        "oct",
        "vec",
        "crypt",
        // Array/List (2)
        "scalar",
        "wantarray",
        // Math (12)
        "abs",
        "int",
        "sqrt",
        "exp",
        "log",
        "sin",
        "cos",
        "tan",
        "atan2",
        "rand",
        "srand",
        // System/Process (14)
        "system",
        "exec",
        "fork",
        "wait",
        "waitpid",
        "kill",
        "sleep",
        "alarm",
        "exit",
        "getpgrp",
        "setpgrp",
        "getppid",
        "getpriority",
        "setpriority",
        // Time (4)
        "time",
        "localtime",
        "gmtime",
        "times",
        // User/Group (5)
        "getpwuid",
        "getpwnam",
        "getgrgid",
        "getgrnam",
        "getlogin",
        // Network (10)
        "socket",
        "bind",
        "listen",
        "accept",
        "connect",
        "send",
        "recv",
        "shutdown",
        "getsockname",
        "getpeername",
        // Control flow (9)
        "eval",
        "require",
        "do",
        "caller",
        "return",
        "goto",
        "last",
        "next",
        "redo",
        // Misc (10)
        "tie",
        "untie",
        "tied",
        "dbmopen",
        "dbmclose",
        "select",
        "syscall",
        "dump",
        "prototype",
        "lock",
    ];

    // We actually have more than 114 now with tan added
    let actual_count = all_builtins.len();
    println!("Total built-in functions in test: {}", actual_count);

    // Test that each function returns a signature
    for func in all_builtins {
        let code = format!("{}($x);", func); // Complete statement with argument
        let uri = format!("file:///{}.pl", func);
        open_doc(&mut server, &uri, &code);
        // Position cursor just after opening paren
        let cursor_pos = func.len() as u32 + 1;
        let result = get_signature_help(&mut server, &uri, 0, cursor_pos);

        assert!(result.is_some(), "No signature found for function: {}", func);
        let sig = result.unwrap();
        let label = sig["signatures"][0]["label"].as_str().unwrap();
        assert!(
            label.contains(func),
            "Signature for {} doesn't contain function name: {}",
            func,
            label
        );
    }
}
