//! Publication facts verification task.

use color_eyre::eyre::{Context, Result, bail};
use std::process::Command;

use crate::utils::project_root;

pub fn run(args: Vec<String>) -> Result<()> {
    let root = project_root()?;
    let script = root.join("scripts").join("verify-publication-facts.sh");

    let status = Command::new("bash")
        .current_dir(&root)
        .arg(script)
        .args(args)
        .status()
        .context("failed to execute scripts/verify-publication-facts.sh")?;

    if status.success() {
        Ok(())
    } else {
        bail!("verify-publication-facts failed");
    }
}
