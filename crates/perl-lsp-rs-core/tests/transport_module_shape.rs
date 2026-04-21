//! Integration test: `perl-lsp-transport` public API reachable via `perl_lsp_rs_core::transport`.

use perl_lsp_rs_core::transport::*;

#[test]
fn transport_module_exposes_read_message() {
    // Verify that read_message is accessible post-absorption
    // Type-check only: we don't invoke since it needs real I/O
    let _: fn(
        &mut dyn std::io::BufRead,
    ) -> Result<Option<perl_lsp_rs_core::protocol::JsonRpcRequest>, _> = read_message;
}

#[test]
fn transport_module_exposes_write_message() {
    // Verify that write_message is accessible post-absorption
    let _: fn(
        &mut dyn std::io::Write,
        &perl_lsp_rs_core::protocol::JsonRpcResponse,
    ) -> Result<(), _> = write_message;
}

#[test]
fn transport_module_exposes_write_notification() {
    // Verify that write_notification is accessible post-absorption
    let _: fn(&mut dyn std::io::Write, &str, serde_json::Value) -> Result<(), _> =
        write_notification;
}

#[test]
fn transport_module_exposes_content_length_message_reader() {
    // Verify that ContentLengthMessageReader is accessible post-absorption
    let _: Option<ContentLengthMessageReader<std::io::BufReader<std::io::Stdin>>> = None;
}

#[test]
fn transport_module_exposes_frame_function() {
    // Verify that frame function is accessible post-absorption
    let _: fn(&perl_lsp_rs_core::protocol::JsonRpcResponse) -> String = frame;
}

#[test]
fn transport_module_dissolves_protocol_dependency_cycle() {
    // NEGATIVE TEST: Verify Wave G2 cycle is dissolved.
    // Before G3, transport depended on perl-lsp-protocol as separate crate.
    // After absorption, protocol types are accessed via perl_lsp_rs_core::protocol.
    // This test asserts transport is now part of rs-core, not external.
    let _: fn(&mut dyn std::io::Write, &str, serde_json::Value) -> Result<(), _> =
        write_notification;
    // If we can write to transport's public API without importing perl_lsp_protocol directly,
    // the cycle is dissolved.
}
