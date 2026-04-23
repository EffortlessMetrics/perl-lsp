//! TextDocumentSyncKind accuracy test — issue #2349
//!
//! The server does full reparsing on every keystroke. The advertised sync
//! kind must match actual behaviour: Full (1), not Incremental (2).

mod support;
use support::lsp_harness::LspHarness;

/// TextDocumentSyncKind::Full = 1, Incremental = 2.
/// The server always reparses the full document text after each
/// didChange, so it must advertise Full (1).
#[test]
fn test_text_document_sync_kind_is_full() -> Result<(), Box<dyn std::error::Error>> {
    let mut harness = LspHarness::new();
    let response = harness.initialize(None)?;
    let caps = response.get("capabilities").ok_or("missing capabilities")?;

    let sync_change = caps
        .pointer("/textDocumentSync/change")
        .and_then(|v| v.as_u64())
        .ok_or("textDocumentSync.change must be a number")?;

    assert_eq!(
        sync_change, 1,
        "textDocumentSync.change must be 1 (Full) because the server does full \
         reparse on every edit, not incremental AST updates. Got: {}",
        sync_change
    );

    Ok(())
}
