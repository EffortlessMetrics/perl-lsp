//! Integration test: Verify that ADR amendments 7 and 8 are present (per D6).
//!
//! Decision D6 requires writing Amendment 7 (G2 retrospective) and Amendment 8
//! (G3 decision: Option A confirmed, zero external consumers, config deferred)
//! to docs/adr/0041-microcrate-collapse.md BEFORE any absorption commits.
//!
//! This test verifies both amendments are documented.

use std::fs;
use std::path::PathBuf;

fn adr_path() -> PathBuf {
    // Tests run from the crate directory; navigate up to workspace root.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("../../docs/adr/0041-microcrate-collapse.md")
}

#[test]
fn g3_adr_amendment_7_present() -> Result<(), Box<dyn std::error::Error>> {
    let adr_path = adr_path();
    assert!(adr_path.exists(), "ADR 0041 file should exist");

    let content = fs::read_to_string(&adr_path)?;

    // Look for Amendment 7 header
    assert!(
        content.contains("### Amendment 7") || content.contains("Amendment 7"),
        "ADR should contain Amendment 7 documenting G2 retrospective (transport+performance deferred)"
    );

    // Amendment 7 should explain the protocol cycle reason for deferral
    assert!(
        content.contains("protocol")
            && (content.contains("cycle")
                || content.contains("deferred")
                || content.contains("transport")
                || content.contains("performance")),
        "Amendment 7 should explain why transport and performance were deferred due to protocol cycle"
    );

    Ok(())
}

#[test]
fn g3_adr_amendment_8_present() -> Result<(), Box<dyn std::error::Error>> {
    let adr_path = adr_path();
    assert!(adr_path.exists(), "ADR 0041 file should exist");

    let content = fs::read_to_string(&adr_path)?;

    // Look for Amendment 8 header
    assert!(
        content.contains("### Amendment 8") || content.contains("Amendment 8"),
        "ADR should contain Amendment 8 documenting G3 decision"
    );

    // Amendment 8 should document Option A confirmation and zero external consumers
    assert!(
        content.contains("Option A") && content.contains("external"),
        "Amendment 8 should confirm Option A and verify zero external consumers"
    );

    // Amendment 8 should mention config deferral and count correction
    assert!(
        (content.contains("perl-lsp-config") || content.contains("config"))
            && (content.contains("37") || content.contains("count")),
        "Amendment 8 should document config deferral and count correction (44 → 37)"
    );

    Ok(())
}

#[test]
fn g3_adr_amendments_in_correct_order() -> Result<(), Box<dyn std::error::Error>> {
    let adr_path = adr_path();
    let content = fs::read_to_string(&adr_path)?;

    // Find positions of both amendments
    let amend_7_pos = content.find("Amendment 7");
    let amend_8_pos = content.find("Amendment 8");

    assert!(amend_7_pos.is_some(), "Amendment 7 should be found in ADR");
    assert!(amend_8_pos.is_some(), "Amendment 8 should be found in ADR");

    if let (Some(pos7), Some(pos8)) = (amend_7_pos, amend_8_pos) {
        assert!(pos7 < pos8, "Amendment 7 should appear before Amendment 8 in the document");
    }

    Ok(())
}
