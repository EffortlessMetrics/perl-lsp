//! Wrapper task for forbidden-fatal-construct checks.
//!
//! Delegates to `perl-ci-hygiene forbid-fatal-constructs`, preferring the
//! locally built binary when available for speed.

use color_eyre::eyre::{bail, Context, Result};
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
    let status = {
        let local_binary = local_binary_path(&root);
        if local_binary.exists() {
            Command::new(local_binary)
                .arg("forbid-fatal-constructs")
                .args(&args)
                .status()
                .context("Failed to execute local perl-ci-hygiene binary")?
        } else {
            Command::new("cargo")
                .current_dir(root)
                .args(["run", "--quiet", "-p", CI_HYGIENE_PACKAGE, "--", "forbid-fatal-constructs"])
                .args(args)
                .status()
                .context("Failed to run perl-ci-hygiene via cargo")?
        }
    };

    if !status.success() {
        bail!("forbid-fatal-constructs failed (exit code: {status})");
    }

    Ok(())
}
