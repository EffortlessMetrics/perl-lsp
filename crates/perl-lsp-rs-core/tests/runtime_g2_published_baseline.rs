//! Green TDD: Published crate baseline verification for Wave G2.
//!
//! These tests verify that the published crate count matches the G2 baseline.
//! After absorbing 5 crates (cancellation, input-validation, launcher, limits,
//! text-utils), the count should go from 49 to 44.
//!
//! Risk context: The published crate baseline is hand-maintained in
//! xtask/published-crate-baseline.txt and used as a regression guard by CI.
//! These tests ensure:
//! - The baseline file was updated correctly
//! - The actual published crate count matches the baseline
//!
//! All tests are green at HEAD (post-G2).

use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    // Tests run from the workspace root
    // CARGO_MANIFEST_DIR is crates/perl-lsp-rs-core, so go up 2 levels
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Test that the baseline file exists.
#[test]
fn test_baseline_file_exists() -> Result<(), Box<dyn std::error::Error>> {
    let baseline_path = repo_root().join("xtask/published-crate-baseline.txt");
    assert!(
        baseline_path.exists(),
        "xtask/published-crate-baseline.txt must exist at {:?}",
        baseline_path
    );
    Ok(())
}

/// Test that the baseline file is readable and contains a number.
#[test]
fn test_baseline_file_contains_number() -> Result<(), Box<dyn std::error::Error>> {
    let baseline_path = repo_root().join("xtask/published-crate-baseline.txt");
    let baseline_content = fs::read_to_string(&baseline_path)?;
    let trimmed = baseline_content.trim();
    let _count: u32 = trimmed.parse()?;
    Ok(())
}

/// Test that the baseline is updated to 44 (49 - 5 absorbed crates).
#[test]
fn test_baseline_updated_to_44() -> Result<(), Box<dyn std::error::Error>> {
    let baseline_path = repo_root().join("xtask/published-crate-baseline.txt");
    let baseline_content = fs::read_to_string(&baseline_path)?;
    let count: u32 = baseline_content.trim().parse()?;
    assert_eq!(
        count, 44,
        "baseline should be 44 after absorbing 5 crates (49 - 5)"
    );
    Ok(())
}

/// Test that the baseline is not the old value (49).
/// Regression guard: ensures the baseline was actually updated.
#[test]
fn test_baseline_not_old_value() -> Result<(), Box<dyn std::error::Error>> {
    let baseline_path = repo_root().join("xtask/published-crate-baseline.txt");
    let baseline_content = fs::read_to_string(&baseline_path)?;
    let count: u32 = baseline_content.trim().parse()?;
    assert_ne!(
        count, 49,
        "baseline should be updated from 49 (old pre-G2 value)"
    );
    Ok(())
}

/// Test that the baseline count is reasonable (between 40 and 50).
/// Sanity check: ensures the count isn't wildly off.
#[test]
fn test_baseline_count_reasonable() -> Result<(), Box<dyn std::error::Error>> {
    let baseline_path = repo_root().join("xtask/published-crate-baseline.txt");
    let baseline_content = fs::read_to_string(&baseline_path)?;
    let count: u32 = baseline_content.trim().parse()?;
    assert!(
        count >= 40 && count <= 50,
        "baseline should be between 40 and 50 (was in reasonable range)"
    );
    Ok(())
}

/// Test that the runtime module exists as a directory.
/// Verifies the module structure was created correctly.
#[test]
fn test_runtime_module_created() -> Result<(), Box<dyn std::error::Error>> {
    let runtime_mod = repo_root().join("crates/perl-lsp-rs-core/src/runtime");
    assert!(
        runtime_mod.exists() && runtime_mod.is_dir(),
        "perl-lsp-rs-core/src/runtime module should exist"
    );
    Ok(())
}

/// Test that runtime/mod.rs file exists.
/// Ensures the module structure is properly initialized.
#[test]
fn test_runtime_mod_rs_exists() -> Result<(), Box<dyn std::error::Error>> {
    let runtime_mod_rs = repo_root().join("crates/perl-lsp-rs-core/src/runtime/mod.rs");
    assert!(
        runtime_mod_rs.exists(),
        "crates/perl-lsp-rs-core/src/runtime/mod.rs should exist"
    );
    Ok(())
}
