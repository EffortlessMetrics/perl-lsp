//! check-version-sync task wrapper.
//!
//! The historical shell script attempted to run a local `perl-ci-hygiene` binary
//! if present and otherwise falls back to `cargo run`.

use crate::utils::project_root;
use color_eyre::eyre::{Context, Result, bail};
use std::process::Command;

pub fn run() -> Result<()> {
    let root = project_root()?;
    let local_binary = root.join(format!(
        "target/debug/perl-ci-hygiene{}",
        std::env::consts::EXE_SUFFIX
    ));
    let mut command = if local_binary.exists() {
        let mut cmd = Command::new(local_binary);
        cmd.arg("check-version-sync");
        cmd
    } else {
        let mut cmd = Command::new("cargo");
        cmd.args([
            "run",
            "--quiet",
            "--manifest-path",
            root.join("Cargo.toml").to_string_lossy().as_ref(),
            "-p",
            "perl-ci-hygiene",
            "--",
            "check-version-sync",
        ]);
        cmd
    };

    let status = command
        .status()
        .context("failed to run check-version-sync")?;
    if !status.success() {
        bail!("check-version-sync command failed");
    }

    Ok(())
}
