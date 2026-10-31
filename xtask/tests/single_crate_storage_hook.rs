//! Real normal-hook controls with fake Cargo and private repository/output paths.
#![cfg(target_os = "linux")]

use anyhow::Result;
use std::process::Command;

#[test]
fn normal_single_crate_hook_preserves_checks_context_and_storage_failures() -> Result<()> {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../scripts/tests/test-single-crate-storage-hook.py");
    let original_tmpdir = std::env::var_os("TMPDIR");
    let output = Command::new("python3")
        .arg(script)
        .args(["--xtask", env!("CARGO_BIN_EXE_xtask")])
        // A real pre-push hook may run without cargo-safe's TMPDIR export.
        .env_remove("TMPDIR")
        .output()?;
    assert_eq!(std::env::var_os("TMPDIR"), original_tmpdir);
    assert!(
        output.status.success(),
        "normal hook controls failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
