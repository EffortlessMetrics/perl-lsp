//! Provenance detection tests for Perl module resolution.
//!
//! These tests verify that CPAN distribution markers (META.json/yml, SIGNATURE,
//! CHECKSUMS) are correctly detected adjacent to resolved modules.
//!
//! IMPORTANT: These tests verify EXPECTED behavior that does not yet exist.
//! They will FAIL until the implementation is complete.

use perl_module::resolution::uri::{IncRoot, IncRootKind, Provenance, detect_provenance};
use std::path::PathBuf;

/// Test type aliases to make the test expectations clear.
/// These types should be importable from perl_module::resolution::uri

// ============================================================================
// Provenance struct tests
// ============================================================================

/// Provenance should be constructable with default values (all false)
#[test]
fn provenance_default_constructs_all_false() {
    let prov = Provenance::default();
    assert!(!prov.has_meta, "has_meta should default to false");
    assert!(!prov.has_signature, "has_signature should default to false");
    assert!(!prov.has_checksums, "has_checksums should default to false");
}

/// Provenance should be constructable with all true values
#[test]
fn provenance_constructs_with_all_true() {
    let prov = Provenance { has_meta: true, has_signature: true, has_checksums: true };
    assert!(prov.has_meta, "has_meta should be true");
    assert!(prov.has_signature, "has_signature should be true");
    assert!(prov.has_checksums, "has_checksums should be true");
}

/// Provenance should be cloneable and equal after clone
#[test]
fn provenance_clone_is_equal() {
    let prov = Provenance { has_meta: true, has_signature: false, has_checksums: true };
    let cloned = prov.clone();
    assert_eq!(prov, cloned, "cloned provenance should equal original");
}

// ============================================================================
// detect_provenance function tests
// ============================================================================

/// detect_provenance should return all-false for directory with no markers
#[test]
fn detect_provenance_returns_all_false_for_empty_dir() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let module_dir = temp.path().join("My-Module");
    std::fs::create_dir_all(&module_dir)?;

    let prov = detect_provenance(&module_dir);

    assert!(!prov.has_meta, "has_meta should be false with no META file");
    assert!(!prov.has_signature, "has_signature should be false with no SIGNATURE file");
    assert!(!prov.has_checksums, "has_checksums should be false with no CHECKSUMS file");
    Ok(())
}

/// detect_provenance should detect META.json
#[test]
fn detect_provenance_detects_meta_json() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let module_dir = temp.path().join("My-Module");
    std::fs::create_dir_all(&module_dir)?;
    std::fs::write(module_dir.join("META.json"), "{}")?;

    let prov = detect_provenance(&module_dir);

    assert!(prov.has_meta, "has_meta should be true when META.json exists");
    assert!(!prov.has_signature, "has_signature should be false");
    assert!(!prov.has_checksums, "has_checksums should be false");
    Ok(())
}

/// detect_provenance should detect META.yml
#[test]
fn detect_provenance_detects_meta_yml() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let module_dir = temp.path().join("My-Module");
    std::fs::create_dir_all(&module_dir)?;
    std::fs::write(module_dir.join("META.yml"), "--- {}")?;

    let prov = detect_provenance(&module_dir);

    assert!(prov.has_meta, "has_meta should be true when META.yml exists");
    assert!(!prov.has_signature, "has_signature should be false");
    assert!(!prov.has_checksums, "has_checksums should be false");
    Ok(())
}

/// detect_provenance should detect SIGNATURE file
#[test]
fn detect_provenance_detects_signature() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let module_dir = temp.path().join("My-Module");
    std::fs::create_dir_all(&module_dir)?;
    std::fs::write(module_dir.join("SIGNATURE"), "hash-value")?;

    let prov = detect_provenance(&module_dir);

    assert!(!prov.has_meta, "has_meta should be false");
    assert!(prov.has_signature, "has_signature should be true when SIGNATURE file exists");
    assert!(!prov.has_checksums, "has_checksums should be false");
    Ok(())
}

/// detect_provenance should detect CHECKSUMS file
#[test]
fn detect_provenance_detects_checksums() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let module_dir = temp.path().join("My-Module");
    std::fs::create_dir_all(&module_dir)?;
    std::fs::write(module_dir.join("CHECKSUMS"), "hash-values")?;

    let prov = detect_provenance(&module_dir);

    assert!(!prov.has_meta, "has_meta should be false");
    assert!(!prov.has_signature, "has_signature should be false");
    assert!(prov.has_checksums, "has_checksums should be true when CHECKSUMS file exists");
    Ok(())
}

/// detect_provenance should detect all three markers simultaneously
#[test]
fn detect_provenance_detects_all_three_markers() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let module_dir = temp.path().join("My-Module");
    std::fs::create_dir_all(&module_dir)?;
    std::fs::write(module_dir.join("META.json"), "{}")?;
    std::fs::write(module_dir.join("SIGNATURE"), "hash")?;
    std::fs::write(module_dir.join("CHECKSUMS"), "sha256: abc")?;

    let prov = detect_provenance(&module_dir);

    assert!(prov.has_meta, "has_meta should be true");
    assert!(prov.has_signature, "has_signature should be true");
    assert!(prov.has_checksums, "has_checksums should be true");
    Ok(())
}

// ============================================================================
// IncRoot::detect_provenance method tests
// ============================================================================

/// IncRoot should have a provenance field that is Option<Provenance>
/// After construction, provenance should be None (lazy evaluation)
#[test]
fn inc_root_provenance_field_is_none_by_default() {
    let root = IncRoot {
        kind: IncRootKind::WorkspaceRelative,
        path: PathBuf::from("/some/path"),
        precedence: 0,
        source: "test".to_string(),
    };

    // The provenance field should be None initially (not yet scanned)
    // This tests that IncRoot has a provenance: Option<Provenance> field
    assert!(
        root.provenance.is_none(),
        "IncRoot.provenance should be None by default (lazy evaluation)"
    );
}

/// IncRoot should have a detect_provenance method that populates the provenance field
#[test]
fn inc_root_detect_provenance_populates_field() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let module_dir = temp.path().join("My-Module");
    std::fs::create_dir_all(&module_dir)?;
    std::fs::write(module_dir.join("META.json"), "{}")?;

    let mut root = IncRoot {
        kind: IncRootKind::WorkspaceRelative,
        path: module_dir.clone(),
        precedence: 0,
        source: "test".to_string(),
    };

    // Initially None
    assert!(root.provenance.is_none(), "provenance should be None before detection");

    // After detection, should be populated
    root.detect_provenance();

    assert!(
        root.provenance.is_some(),
        "provenance should be Some after detect_provenance() is called"
    );

    let prov = root.provenance.unwrap();
    assert!(prov.has_meta, "has_meta should be true after detection");
    Ok(())
}

/// IncRoot::detect_provenance should correctly detect all marker types
#[test]
fn inc_root_detect_provenance_all_markers() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let module_dir = temp.path().join("My-Module");
    std::fs::create_dir_all(&module_dir)?;
    std::fs::write(module_dir.join("META.json"), "{}")?;
    std::fs::write(module_dir.join("SIGNATURE"), "hash")?;
    std::fs::write(module_dir.join("CHECKSUMS"), "sha256: abc")?;

    let mut root = IncRoot {
        kind: IncRootKind::WorkspaceRelative,
        path: module_dir,
        precedence: 0,
        source: "test".to_string(),
    };

    root.detect_provenance();

    let prov = root.provenance.unwrap();
    assert!(prov.has_meta, "has_meta should be true");
    assert!(prov.has_signature, "has_signature should be true");
    assert!(prov.has_checksums, "has_checksums should be true");
    Ok(())
}

// ============================================================================
// Trust classification tests (informational only)
// ============================================================================

/// Modules with has_signature=true should be classified as "Signed"
/// This is informational only - we just verify the provenance fields
#[test]
fn provenance_signed_classification() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let module_dir = temp.path().join("My-Module");
    std::fs::create_dir_all(&module_dir)?;
    std::fs::write(module_dir.join("SIGNATURE"), "hash")?;

    let prov = detect_provenance(&module_dir);

    // A module with SIGNATURE is considered "Signed" (informational)
    assert!(prov.has_signature, "Signed classification requires has_signature=true");
    assert!(!prov.has_meta, "Signed does not require has_meta");
    Ok(())
}

/// Modules with has_meta=true but no has_signature should be "KnownDistributor"
#[test]
fn provenance_known_distributor_classification() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let module_dir = temp.path().join("My-Module");
    std::fs::create_dir_all(&module_dir)?;
    std::fs::write(module_dir.join("META.json"), "{}")?;

    let prov = detect_provenance(&module_dir);

    // A module with META but no SIGNATURE is "KnownDistributor"
    assert!(prov.has_meta, "KnownDistributor requires has_meta=true");
    assert!(!prov.has_signature, "KnownDistributor requires has_signature=false");
    Ok(())
}

/// Modules with no CPAN markers should be "Unknown"
#[test]
fn provenance_unknown_classification() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let module_dir = temp.path().join("My-Module");
    std::fs::create_dir_all(&module_dir)?;
    // No META.json/yml, no SIGNATURE, no CHECKSUMS

    let prov = detect_provenance(&module_dir);

    // A module with no CPAN markers is "Unknown"
    assert!(!prov.has_meta, "Unknown classification requires has_meta=false");
    assert!(!prov.has_signature, "Unknown classification requires has_signature=false");
    assert!(!prov.has_checksums, "Unknown classification requires has_checksums=false");
    Ok(())
}
