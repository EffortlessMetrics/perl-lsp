//! Forensics task wrappers.

use color_eyre::eyre::{Context, Result, bail};
use std::path::Path;
use std::process::Command;

use crate::utils::project_root;

pub fn run_harvest(pr: &str) -> Result<()> {
    let root = project_root()?;
    let script = root.join("scripts").join("forensics").join("pr-harvest.sh");
    run_script(&script, &[pr])
}

pub fn run_temporal(pr: &str) -> Result<()> {
    let root = project_root()?;
    let script = root
        .join("scripts")
        .join("forensics")
        .join("temporal-analysis.sh");
    run_script(&script, &[pr])
}

pub fn run_telemetry_quick(pr: &str) -> Result<()> {
    let root = project_root()?;
    let script = root
        .join("scripts")
        .join("forensics")
        .join("telemetry-runner.sh");
    run_script(&script, &[pr, "--mode", "quick"])
}

pub fn run_telemetry_full(pr: &str) -> Result<()> {
    let root = project_root()?;
    let script = root
        .join("scripts")
        .join("forensics")
        .join("telemetry-runner.sh");
    run_script(&script, &[pr, "--mode", "full"])
}

pub fn run_dossier(pr: &str) -> Result<()> {
    let root = project_root()?;
    let script = root
        .join("scripts")
        .join("forensics")
        .join("dossier-runner.sh");
    run_script(&script, &[pr])
}

pub fn run_render(pr: &str, format: &str) -> Result<()> {
    let root = project_root()?;
    let script = root
        .join("scripts")
        .join("forensics")
        .join("render-dossier.sh");
    run_script(&script, &[pr, "--format", format])
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
        bail!("forensics script failed: {}", script.display());
    }
}
