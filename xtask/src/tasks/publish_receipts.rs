//! Publish a review receipt bundle under review/receipts/YYYY-MM-DD/.
//!
//! Combines `cargo xtask gates` and `cargo xtask receipts` into a single
//! command, archives outputs, and writes a short README with provenance.

use chrono::Utc;
use color_eyre::eyre::{Context, Result, bail};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;

use crate::utils::project_root;

const RECEIPT_FILES: [&str; 5] =
    ["test-output.txt", "test-summary.json", "rustdoc.log", "doc-summary.json", "state.json"];

pub fn run(date: Option<String>) -> Result<()> {
    let date = date.unwrap_or_else(|| Utc::now().format("%Y-%m-%d").to_string());
    if date.trim().is_empty() {
        bail!("receipt date cannot be empty");
    }

    let root = project_root()?;
    let destination = root.join("review").join("receipts").join(&date);
    let artifacts_source = root.join("artifacts");
    let artifacts_destination = destination.join("artifacts");

    fs::create_dir_all(&artifacts_destination)
        .with_context(|| format!("Failed to create {}", artifacts_destination.display()))?;

    println!("Publishing receipts to: {}", destination.display());

    let ci_output = run_and_log(
        "ci gate",
        &root,
        &mut cargo_xtask_gates("merge-gate"),
        &destination.join("ci-gate.log"),
    )?;
    println!("{}", ci_output);

    let receipts_output = run_and_log(
        "receipt generation",
        &root,
        &mut command_receipts(),
        &destination.join("generate-receipts.log"),
    )?;
    println!("{}", receipts_output);

    let mut copied = 0usize;
    for file in RECEIPT_FILES {
        let source = artifacts_source.join(file);
        let target = artifacts_destination.join(file);

        match fs::copy(&source, &target) {
            Ok(_) => copied += 1,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                println!("⚠️  Missing artifact file: {}", source.display());
            }
            Err(err) => {
                bail!("failed to copy {} to {}: {}", source.display(), target.display(), err);
            }
        }
    }

    let readme = format!(
        r#"# Receipt Bundle: {date}

## Provenance
- Commit: `{sha}`
- rustc: `{rustc}`
- cargo: `{cargo}`
- Host: `{host}`

## What ran
- `cargo xtask gates --tier merge-gate --receipt` (see `ci-gate.log`)
- `cargo xtask receipts` (see `generate-receipts.log`)
- {copied} artifact file(s) copied from `artifacts/`
"#,
        date = date,
        sha = command_output_or_unknown(&root, &["git", "rev-parse", "HEAD"], "UNVERIFIED"),
        rustc = command_output_or_unknown(&root, &["rustc", "--version"], "UNVERIFIED"),
        cargo = command_output_or_unknown(&root, &["cargo", "--version"], "UNVERIFIED"),
        host = command_output_or_unknown(&root, &["uname", "-a"], "UNVERIFIED"),
        copied = copied,
    );

    fs::write(destination.join("README.md"), readme)
        .with_context(|| format!("Failed to write {}", destination.join("README.md").display()))?;

    println!("Receipt bundle ready: {}", destination.display());
    Ok(())
}

fn cargo_xtask_gates(tier: &str) -> Command {
    let mut command = Command::new("cargo");
    command.args(["xtask", "gates", "--tier", tier, "--receipt"]);
    command
}

fn command_receipts() -> Command {
    let mut command = Command::new("cargo");
    command.args(["xtask", "receipts"]);
    command
}

fn run_and_log(stage: &str, root: &Path, command: &mut Command, log_path: &Path) -> Result<String> {
    let output = command.current_dir(root).output().with_context(|| {
        format!("Failed to execute {stage} command in {}", root.join(".").display())
    })?;

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let mut log = fs::File::create(log_path)
        .with_context(|| format!("Failed to create log at {}", log_path.display()))?;
    log.write_all(combined.as_bytes())
        .with_context(|| format!("Failed to write {}", log_path.display()))?;

    if !output.status.success() {
        bail!("{stage} failed (see {})", log_path.display());
    }

    Ok(combined)
}

fn command_output_or_unknown(root: &Path, args: &[&str], fallback: &str) -> String {
    if args.is_empty() {
        return fallback.to_string();
    }

    let mut command = Command::new(args[0]);
    command.current_dir(root).args(&args[1..]);

    match command.output() {
        Ok(output) => {
            let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if value.is_empty() { fallback.to_string() } else { value }
        }
        Err(_) => fallback.to_string(),
    }
}
