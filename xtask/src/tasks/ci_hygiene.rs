//! Pass-through wrapper for `perl-ci-hygiene` subcommands.
//!
//! This task keeps shell wrappers thin and delegates to the crate directly,
//! either via an existing local debug binary or via `cargo run`.

use color_eyre::eyre::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::utils::project_root;

const CI_HYGIENE_PACKAGE: &str = "perl-ci-hygiene";

pub fn run(command: String, args: Vec<String>) -> Result<()> {
    let root = project_root()?;
    let status = {
        let local_binary = local_binary_path(&root);
        if local_binary.exists() {
            Command::new(local_binary)
                .arg(&command)
                .args(&args)
                .status()
                .context("Failed to execute local perl-ci-hygiene binary")?
        } else {
            let mut cargo_command = Command::new("cargo");
            cargo_command
                .current_dir(&root)
                .args(["run", "--quiet", "-p", CI_HYGIENE_PACKAGE, "--", &command])
                .args(args)
                .status()
                .context("Failed to run perl-ci-hygiene via cargo")?
        }
    };

    if !status.success() {
        bail!("perl-ci-hygiene command '{command}' failed (exit code: {status})");
    }

    Ok(())
}

fn local_binary_path(root: &Path) -> PathBuf {
    let mut path = root.join("target").join("debug").join(CI_HYGIENE_PACKAGE);
    if cfg!(windows) {
        path.set_extension(std::env::consts::EXE_EXTENSION);
    }
    path
}
