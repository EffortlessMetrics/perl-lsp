//! GitHub repository maintenance tasks delegated to the `gh` CLI.

use color_eyre::eyre::{Context, Result, bail};
use std::path::Path;
use std::process::Command;

pub fn run_labels() -> Result<()> {
    let root = crate::utils::project_root()?;
    let script = root.join("scripts").join("gh").join("ensure-labels.sh");
    run_script(&script, &[])
}

pub fn run_issues_needing_triage(limit: usize) -> Result<()> {
    let root = crate::utils::project_root()?;
    let script = root
        .join("scripts")
        .join("gh")
        .join("issues-needing-triage.sh");
    let limit = limit.to_string();
    run_script(&script, &[limit.as_str()])
}

pub fn run_backfill_prefixed_labels(apply: bool) -> Result<()> {
    let root = crate::utils::project_root()?;
    let script = root
        .join("scripts")
        .join("gh")
        .join("backfill-prefixed-labels.sh");
    if apply {
        run_script(&script, &["--apply"])
    } else {
        run_script(&script, &[])
    }
}

fn run_script(script: &Path, args: &[&str]) -> Result<()> {
    let mut command = Command::new("bash");
    command.arg(script);
    for arg in args {
        command.arg(arg);
    }

    let status = command
        .status()
        .with_context(|| format!("failed to execute {}", script.display()))?;

    if status.success() {
        Ok(())
    } else {
        bail!("github maintenance script failed: {}", script.display());
    }
}
