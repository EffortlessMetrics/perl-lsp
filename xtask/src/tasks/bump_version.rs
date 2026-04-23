//! bump-version task wrapper.
//!
//! Delegates to the `perl-ci-hygiene bump-version` subcommand, which owns
//! the canonical list of version sites. This keeps the bump command and
//! the `check-version-sync` CI gate walking the same list — they cannot
//! drift because they share a module.
//!
//! Mirrors the pattern used by `check_version_sync.rs`: prefer a pre-built
//! binary if one is available (fast path for CI), otherwise fall back to
//! `cargo run`.

use crate::utils::project_root;
use color_eyre::eyre::{Context, Result, bail};
use std::process::Command;

pub fn run(version: String) -> Result<()> {
    let root = project_root()?;
    let local_binary =
        root.join(format!("target/debug/perl-ci-hygiene{}", std::env::consts::EXE_SUFFIX));
    let mut command = if local_binary.exists() {
        let mut cmd = Command::new(local_binary);
        cmd.arg("bump-version").arg(&version);
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
            "bump-version",
            &version,
        ]);
        cmd
    };

    let status = command.status().context("failed to run bump-version")?;
    if !status.success() {
        bail!("bump-version command failed");
    }

    Ok(())
}
