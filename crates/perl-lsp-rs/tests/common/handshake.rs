//! LSP initialize/shutdown handshake helpers.
//!
//! Provides `initialize_lsp`, `await_index_ready`, and `shutdown_and_exit` which
//! encapsulate the full LSP lifecycle for integration tests.

use perl_tdd_support::must;
use serde_json::{Value, json};
use std::time::Duration;

use super::protocol_io::{
    map_send_error, read_notification_method, read_response_matching_i64, send_message_inner,
};
use super::{
    LspServer, adaptive_timeout, max_concurrent_threads, next_id, send_notification, send_request,
};

fn initialize_lsp_with_params(server: &LspServer, params: Value) -> Value {
    let init = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": params
    });

    // write without reading
    {
        let body = init.to_string();
        if let Err(e) =
            send_message_inner(&mut *server.writer.lock().unwrap_or_else(|e| e.into_inner()), &body)
        {
            // Handle write errors gracefully with proper JSON-RPC envelope (id=1)
            return map_send_error(Some(json!(1)), e, "initialize");
        }
    }

    // wait specifically for id=1 - use extended timeout for initialization
    // Enhanced timeout for LSP cancellation tests with environment-aware scaling
    let base_multiplier = 3; // Increased base multiplier for cancellation infrastructure tests (increased from 2x to 3x)
    let thread_count = max_concurrent_threads();
    let env_multiplier = if thread_count <= 2 { 3 } else { 2 }; // Extra time for constrained environments with cancellation infrastructure (increased from 2x to 3x)

    // Additional CI environment detection for graceful degradation
    let ci_multiplier = if std::env::var("CI").is_ok()
        || std::env::var("GITHUB_ACTIONS").is_ok()
        || std::env::var("CONTINUOUS_INTEGRATION").is_ok()
    {
        2 // Extra time for CI environments with limited resources
    } else {
        1
    };

    let init_timeout = adaptive_timeout() * base_multiplier * env_multiplier * ci_multiplier;

    // Enhanced retry logic for cancellation infrastructure tests
    let mut retry_count = 0;
    let max_retries = 2; // Allow 2 retries for infrastructure tests

    let resp = loop {
        match read_response_matching_i64(server, 1, init_timeout) {
            Some(response) => break response,
            None => {
                retry_count += 1;
                if retry_count > max_retries {
                    eprintln!(
                        "LSP server failed to respond to initialize request within {:?} after {} retries",
                        init_timeout, max_retries
                    );
                    eprintln!(
                        "Check if server started properly and is responding to JSON-RPC requests"
                    );
                    eprintln!("Server process alive: {}", server.is_alive());
                    must(Err::<(), _>("initialize response timeout - server may have crashed or is not responding".to_string()))
                } else {
                    eprintln!(
                        "Initialize timeout attempt {}/{}, retrying with fresh request...",
                        retry_count,
                        max_retries + 1
                    );
                    // Brief delay before retry
                    std::thread::sleep(Duration::from_millis(200));
                    // Send another initialize request with a new ID
                    let retry_id = next_id();
                    send_request(
                        server,
                        json!({"id":retry_id,"method":"initialize","params":{"capabilities":{}}}),
                    );
                    // Try reading the retry response
                    if let Some(retry_resp) =
                        read_response_matching_i64(server, retry_id, init_timeout)
                    {
                        break retry_resp;
                    }
                }
            }
        }
    };

    // Send initialized notification with a brief delay
    std::thread::sleep(Duration::from_millis(50));
    send_notification(server, json!({"jsonrpc":"2.0","method":"initialized"}));

    // Wait for index-ready notification to ensure deterministic completion behavior
    await_index_ready(server);

    resp
}

pub fn initialize_lsp(server: &LspServer) -> Value {
    initialize_lsp_with_params(
        server,
        json!({
            "capabilities": {},
            "clientInfo": {"name":"perl-parser-tests","version":"0"},
            "rootUri": null,
            "workspaceFolders": null
        }),
    )
}

pub fn initialize_lsp_with_capabilities(server: &LspServer, capabilities: Value) -> Value {
    initialize_lsp_with_params(
        server,
        json!({
            "capabilities": capabilities,
            "clientInfo": {"name":"perl-parser-tests","version":"0"},
            "rootUri": null,
            "workspaceFolders": null
        }),
    )
}

/// Wait for the index-ready notification from the server
pub fn await_index_ready(server: &LspServer) {
    // Wait for perl-lsp/index-ready notification with a reasonable timeout
    if let Some(_notification) =
        read_notification_method(server, "perl-lsp/index-ready", Duration::from_millis(500))
    {
        eprintln!("Index ready notification received");
    } else {
        eprintln!("No index-ready notification received within timeout (proceeding anyway)");
    }
}

/// Gracefully shut the server down (best-effort) without hanging tests.
pub fn shutdown_and_exit(server: &LspServer) {
    use std::sync::atomic::Ordering;

    // Mark shutdown as initiated to prevent duplicate shutdown in Drop
    if server.shutdown_initiated.swap(true, Ordering::SeqCst) {
        // Already initiated, just wait for exit
        for _ in 0..20 {
            if server
                .process
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .try_wait()
                .ok()
                .flatten()
                .is_some()
            {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        return;
    }

    // Try a graceful shutdown first; if the server ignores, we'll still exit the test.
    let _ = send_request(
        server,
        json!({"jsonrpc":"2.0","id": 999_001,"method":"shutdown","params":{}}),
    );
    send_notification(server, json!({"jsonrpc":"2.0","method":"exit"}));

    // Give it a moment, then force-kill if needed.
    for _ in 0..20 {
        if server
            .process
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .try_wait()
            .ok()
            .flatten()
            .is_some()
        {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let _ = server.process.lock().unwrap_or_else(|e| e.into_inner()).kill();
}
