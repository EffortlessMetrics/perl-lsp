//! Wrapper task for forbidden-fatal-construct checks.
//!
//! Delegates to `perl-ci-hygiene forbid-fatal-constructs`, preferring the
//! locally built binary when available for speed.

use color_eyre::eyre::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::utils::project_root;

const CI_HYGIENE_PACKAGE: &str = "perl-ci-hygiene";

fn local_binary_path(root: &Path) -> PathBuf {
    let mut path = root.join("target").join("debug").join(CI_HYGIENE_PACKAGE);
    if cfg!(windows) {
        path.set_extension(std::env::consts::EXE_EXTENSION);
    }
    path
}

pub fn run(args: Vec<String>) -> Result<()> {
    let root = project_root()?;
    let local_binary = local_binary_path(&root);

    let build_status = Command::new("cargo")
        .current_dir(&root)
        .args(["build", "--quiet", "-p", CI_HYGIENE_PACKAGE])
        .status()
        .context("Failed to build perl-ci-hygiene before running forbid-fatal-constructs")?;

    if !build_status.success() {
        bail!("Failed to build perl-ci-hygiene before forbid-fatal-constructs");
    }

    let status = Command::new(local_binary)
        .arg("forbid-fatal-constructs")
        .args(&args)
        .status()
        .context("Failed to execute local perl-ci-hygiene binary")?;

    if !status.success() {
        bail!("forbid-fatal-constructs failed (exit code: {status})");
    }

    Ok(())
}
