//! Populate mdBook source tree from project documentation files.

use color_eyre::eyre::{Context, Result, bail};
use std::process::Command;

use crate::utils::project_root;

pub fn run() -> Result<()> {
    let root = project_root()?;
    let script = root.join("scripts").join("populate-book.sh");

    let status = Command::new("bash")
        .current_dir(&root)
        .arg(script.clone())
        .status()
        .context("failed to execute scripts/populate-book.sh")?;

    if status.success() {
        Ok(())
    } else {
        bail!("populate-book failed");
    }
}
