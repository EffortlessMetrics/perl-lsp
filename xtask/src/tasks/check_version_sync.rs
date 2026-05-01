//! check-version-sync task wrapper.
//!
//! Always invokes `perl-ci-hygiene` through Cargo so release verification cannot
//! accidentally reuse a stale repo-local helper binary.

use crate::utils::project_root;
use color_eyre::eyre::{Context, Result, bail};
use std::process::Command;

pub fn run() -> Result<()> {
    let root = project_root()?;
    let mut command = Command::new("cargo");
    command.args([
        "run",
        "--quiet",
        "--manifest-path",
        root.join("Cargo.toml").to_string_lossy().as_ref(),
        "-p",
        "perl-ci-hygiene",
        "--",
        "check-version-sync",
    ]);

    let status = command.status().context("failed to run check-version-sync")?;
    if !status.success() {
        bail!("check-version-sync command failed");
    }

    Ok(())
}
