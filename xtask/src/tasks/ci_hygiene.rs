//! Pass-through wrapper for `perl-ci-hygiene` subcommands.
//!
//! This task keeps shell wrappers thin and delegates to the crate directly,
//! either via an existing local debug binary or via `cargo run`.

use color_eyre::eyre::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::utils::project_root;

const CI_HYGIENE_PACKAGE: &str = "perl-ci-hygiene";

pub fn run(command: String, args: Vec<String>) -> Result<()> {
    let root = project_root()?;
    let status = {
        let local_binary = local_binary_path(&root);
        if local_binary_is_fresh(&root, &local_binary) {
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

fn local_binary_is_fresh(root: &Path, local_binary: &Path) -> bool {
    let Ok(binary_meta) = fs::metadata(local_binary) else {
        return false;
    };
    let Ok(binary_modified) = binary_meta.modified() else {
        return false;
    };

    for source in ci_hygiene_sources(root) {
        let Ok(source_meta) = fs::metadata(source) else {
            return false;
        };
        let Ok(source_modified) = source_meta.modified() else {
            return false;
        };
        if source_modified > binary_modified {
            return false;
        }
    }

    true
}

fn ci_hygiene_sources(root: &Path) -> [PathBuf; 2] {
    [
        root.join("crates")
            .join(CI_HYGIENE_PACKAGE)
            .join("Cargo.toml"),
        root.join("crates")
            .join(CI_HYGIENE_PACKAGE)
            .join("src")
            .join("main.rs"),
    ]
}
