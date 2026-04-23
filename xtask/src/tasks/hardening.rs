//! Production hardening maintenance tasks.

use color_eyre::eyre::{Context, Result, bail};
use std::path::Path;
use std::process::Command;

use crate::utils::project_root;

pub fn security_hardening() -> Result<()> {
    let root = project_root()?;
    let script = root.join("scripts").join("security-hardening.sh");
    run_script(&script, &[])
}

pub fn performance_hardening() -> Result<()> {
    let root = project_root()?;
    let script = root.join("scripts").join("performance-hardening.sh");
    run_script(&script, &[])
}

pub fn production_gates_validation() -> Result<()> {
    let root = project_root()?;
    let script = root.join("scripts").join("production-gates-validation.sh");
    run_script(&script, &[])
}

fn run_script(script: &Path, args: &[&str]) -> Result<()> {
    let status = Command::new("bash")
        .arg(script)
        .args(args)
        .status()
        .with_context(|| format!("failed to execute {}", script.display()))?;

    if status.success() {
        Ok(())
    } else {
        bail!("production hardening task failed: {}", script.display());
    }
}
