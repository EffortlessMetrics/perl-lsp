//! Integration test: `perl-lsp-protocol` public API reachable via `perl_lsp_rs_core::protocol`.

use perl_lsp_rs_core::protocol::*;

#[test]
fn protocol_module_exposes_jsonrpc_request() {
    // Verify that JsonRpcRequest is accessible post-absorption
    let _: Option<JsonRpcRequest> = None;
}

#[test]
fn protocol_module_exposes_jsonrpc_response() {
    // Verify that JsonRpcResponse is accessible post-absorption
    let _: Option<JsonRpcResponse> = None;
}

#[test]
fn protocol_module_exposes_jsonrpc_error() {
    // Verify that JsonRpcError is accessible post-absorption
    let _: Option<JsonRpcError> = None;
}

#[test]
fn protocol_module_exposes_error_code() {
    // Verify that ErrorCode is accessible post-absorption
    let _: Option<ErrorCode> = None;
}

#[test]
fn protocol_module_exposes_capabilities_module() {
    // Verify that capabilities submodule is accessible post-absorption
    let _: Option<capabilities::ServerCapabilities> = None;
}

#[test]
fn protocol_module_exposes_methods_module() {
    // Verify that methods submodule is accessible post-absorption
    let _: Option<&str> = Some(methods::INITIALIZE);
}

#[test]
fn protocol_module_exposes_lsp_error_helpers() {
    // Verify that LSP error building functions are accessible post-absorption
    let error = lsp_error("test error");
    assert!(error.code == ErrorCode::ServerErrorStart as i32, "lsp_error should set code");
}
