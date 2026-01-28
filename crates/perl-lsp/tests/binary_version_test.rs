//! Binary version regression test
//!
//! This test ensures that the LSP server binary being tested matches the
//! expected crate version. This catches the "stale binary" problem where
//! tests accidentally run against an old installed binary instead of the
//! freshly-built one.
//!
//! If this test fails, you're running against the wrong binary!
//! Common causes:
//! - Stale release binary in target/release/perl-lsp
//! - Old perl-lsp installed in PATH
//! - PERL_LSP_BIN pointing to wrong binary
//!
//! Fix: Run `cargo build -p perl-lsp` or just `cargo test -p perl-lsp`

mod common;

use serde_json::json;

/// The expected version from the crate being tested
const EXPECTED_VERSION: &str = env!("CARGO_PKG_VERSION");

#[test]
fn lsp_server_version_matches_crate_version() {
    // Start the server using the same resolution logic as other tests
    let mut server = common::start_lsp_server();

    // Send initialize request
    let response = common::send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "capabilities": {},
                "clientInfo": {"name": "version-test", "version": "0"},
                "rootUri": null,
                "workspaceFolders": null
            }
        }),
    );

    // Extract serverInfo.version from the response
    let server_version = response
        .get("result")
        .and_then(|r| r.get("serverInfo"))
        .and_then(|s| s.get("version"))
        .and_then(|v| v.as_str())
        .unwrap_or("<missing>");

    // Assert version matches
    assert_eq!(
        server_version,
        EXPECTED_VERSION,
        "\n\
        ╔════════════════════════════════════════════════════════════════════╗\n\
        ║ WRONG BINARY VERSION DETECTED!                                     ║\n\
        ╠════════════════════════════════════════════════════════════════════╣\n\
        ║ Expected: {expected:50} ║\n\
        ║ Got:      {got:50} ║\n\
        ╠════════════════════════════════════════════════════════════════════╣\n\
        ║ You are running tests against a stale or incorrect perl-lsp binary ║\n\
        ║                                                                    ║\n\
        ║ FIX: Run one of these commands:                                    ║\n\
        ║   cargo build -p perl-lsp       # Rebuild the binary               ║\n\
        ║   cargo test -p perl-lsp        # Rebuild and test                 ║\n\
        ║                                                                    ║\n\
        ║ If using PERL_LSP_BIN, verify it points to the correct binary.     ║\n\
        ╚════════════════════════════════════════════════════════════════════╝\n",
        expected = EXPECTED_VERSION,
        got = server_version,
    );

    // Clean shutdown
    common::shutdown_and_exit(&mut server);

    eprintln!("✓ Server version {} matches expected {}", server_version, EXPECTED_VERSION);
}

#[test]
fn lsp_server_identifier_is_perl_lsp() {
    // Start the server
    let mut server = common::start_lsp_server();

    // Send initialize request
    let response = common::send_request(
        &mut server,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "capabilities": {},
                "rootUri": null
            }
        }),
    );

    // Extract serverInfo.name from the response
    let server_name = response
        .get("result")
        .and_then(|r| r.get("serverInfo"))
        .and_then(|s| s.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("<missing>");

    assert_eq!(
        server_name, "perl-lsp",
        "Server identifier should be 'perl-lsp', got '{}'",
        server_name
    );

    // Clean shutdown
    common::shutdown_and_exit(&mut server);
}
