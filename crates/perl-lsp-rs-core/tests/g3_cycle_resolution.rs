//! Regression test: Verify that protocol absorption dissolved the transport cycle.
//!
//! Prior to Wave G3, perl-lsp-transport → perl-lsp-protocol → perl-lsp-rs-core
//! created a cycle that prevented transport absorption. Protocol absorption dissolves this.
//!
//! This test verifies:
//! 1. The cycle is gone (layer-check equivalent)
//! 2. Transport is now absorbed (directory deleted, module reachable)
//! 3. Protocol types are accessible from rs-core

use std::fs;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("..").join("..")
}

#[test]
fn g3_protocol_absorption_dissolves_transport_cycle() -> Result<(), Box<dyn std::error::Error>> {
    let root = workspace_root();

    // Regression guard: perl-lsp-transport should be DELETED (fully absorbed, not stub)
    let transport_dir = root.join("crates/perl-lsp-transport");
    assert!(
        !transport_dir.exists(),
        "perl-lsp-transport directory should be deleted (cycle dissolved by protocol absorption)"
    );

    // Regression guard: perl-lsp-protocol should be DELETED (fully absorbed)
    let protocol_dir = root.join("crates/perl-lsp-protocol");
    assert!(
        !protocol_dir.exists(),
        "perl-lsp-protocol directory should be deleted"
    );

    Ok(())
}

#[test]
fn g3_transport_reachable_from_rs_core() {
    // Verify that transport module is now accessible from rs-core (cycle is broken)
    // This is a compile-time regression guard - if transport isn't re-exported, this won't compile
    use perl_lsp_rs_core::transport as _transport;
}

#[test]
fn g3_protocol_reachable_from_rs_core() {
    // Verify that protocol module is now accessible from rs-core
    use perl_lsp_rs_core::protocol as _protocol;
}

#[test]
fn g3_no_external_perl_lsp_protocol_references() -> Result<(), Box<dyn std::error::Error>> {
    let root = workspace_root();

    // Check that active crates don't reference the deleted perl-lsp-protocol directly
    // (they should use perl_lsp_rs_core::protocol instead)
    let key_source_files = vec![
        "crates/perl-lsp/src/lib.rs",
        "crates/perl-lsp/src/runtime/diagnostics.rs",
        "crates/perl-lsp/src/runtime/mod.rs",
    ];

    for file in key_source_files {
        let file_path = root.join(file);
        if file_path.exists() {
            let content = fs::read_to_string(&file_path)?;

            // Verify old import is not present (should use rs-core instead)
            // Allow for comments about old paths, but actual imports should be gone
            let has_old_import = content.contains("use perl_lsp_protocol") &&
                                !content.trim_start().starts_with("//");

            if has_old_import {
                // More lenient: check if it's in a comment
                let lines: Vec<&str> = content.lines().collect();
                let mut found_active_import = false;
                for line in lines.iter() {
                    if line.contains("use perl_lsp_protocol") {
                        // Check if this line is commented out
                        let trimmed = line.trim_start();
                        if !trimmed.starts_with("//") && !trimmed.starts_with("/*") {
                            found_active_import = true;
                            break;
                        }
                    }
                }
                assert!(!found_active_import,
                    "File {} should not have active imports of perl_lsp_protocol (use perl_lsp_rs_core::protocol instead)",
                    file);
            }
        }
    }

    Ok(())
}
