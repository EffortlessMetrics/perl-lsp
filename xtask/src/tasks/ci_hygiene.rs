//! Pass-through wrapper for `perl-ci-hygiene` subcommands.
//!
//! This task keeps shell wrappers thin and delegates to the crate directly,
//! either via an existing local debug binary or via `cargo run`.

use color_eyre::eyre::{Context, Result, bail};
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::utils::project_root;

pub fn run(command: String, args: Vec<String>) -> Result<()> {
    let root = project_root()?;
    let status = {
        let local_binary = perl_ci_hygiene::binary_path(&root);
        if local_binary_is_fresh(&local_binary, &root) {
            Command::new(local_binary)
                .arg(&command)
                .args(&args)
                .status()
                .context("Failed to execute local perl-ci-hygiene binary")?
        } else {
            let mut cargo_command = Command::new("cargo");
            cargo_command
                .current_dir(&root)
                .args(["run", "--quiet", "-p", perl_ci_hygiene::PACKAGE_NAME, "--", &command])
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

fn local_binary_is_fresh(local_binary: &Path, root: &Path) -> bool {
    let Ok(binary_meta) = fs::metadata(local_binary) else {
        return false;
    };
    let Ok(binary_modified) = binary_meta.modified() else {
        return false;
    };

    for source in perl_ci_hygiene::source_paths(root) {
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
