//! Real normal-hook controls with fake Cargo and private repository/output paths.
#![cfg(target_os = "linux")]

use anyhow::Result;
use std::process::Command;

#[test]
fn normal_single_crate_hook_preserves_checks_context_and_storage_failures() -> Result<()> {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../scripts/tests/test-single-crate-storage-hook.py");
    let output = Command::new("python3")
        .arg(script)
        .args(["--xtask", env!("CARGO_BIN_EXE_xtask")])
        .output()?;
    assert!(
        output.status.success(),
        "normal hook controls failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
