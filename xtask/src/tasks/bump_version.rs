//! bump-version task wrapper.
//!
//! Delegates to the `perl-ci-hygiene bump-version` subcommand, which owns
//! the canonical list of version sites. This keeps the bump command and
//! the `check-version-sync` CI gate walking the same list — they cannot
//! drift because they share a module.
//!
//! Mirrors the pattern used by `check_version_sync.rs`: always run the helper
//! through Cargo so release verification cannot reuse stale local binaries.

use crate::utils::project_root;
use color_eyre::eyre::{Context, Result, bail};
use std::process::Command;

pub fn run(version: String) -> Result<()> {
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
        "bump-version",
        &version,
    ]);

    let status = command.status().context("failed to run bump-version")?;
    if !status.success() {
        bail!("bump-version command failed");
    }

    Ok(())
}
