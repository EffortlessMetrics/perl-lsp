//! Linux wrapper admission checks use real private locks and stub disk/Cargo.
#![cfg(target_os = "linux")]

use color_eyre::eyre::{Result, ensure};
use std::path::Path;
use std::process::Command;

#[test]
fn cargo_safe_admission_survives_lock_contention() -> Result<()> {
    let script =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../scripts/tests/test-cargo-safe-admission.py");
    let output = Command::new("python3").arg(script).output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    ensure!(
        output.status.success(),
        "cargo-safe fixture suite failed:\n{}\n{stderr}",
        String::from_utf8_lossy(&output.stdout)
    );
    for case in [
        "test_rechecks_free_space_after_waiting_for_lock",
        "test_rechecks_usage_after_waiting_for_lock",
        "test_post_lock_success_preserves_arguments_environment_status_and_lock",
    ] {
        ensure!(stderr.contains(case), "required contention control did not execute: {case}");
    }
    ensure!(stderr.contains("\nOK\n"), "fixture suite did not report successful completion");
    Ok(())
}
