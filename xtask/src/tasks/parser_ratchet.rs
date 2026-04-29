use clap::ValueEnum;
use color_eyre::eyre::{Context, Result, eyre};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum RatchetProfile {
    Pr,
    Nightly,
    Release,
}

#[derive(Debug, Clone)]
pub struct ParserRatchetRunConfig {
    pub profile: RatchetProfile,
    pub base: String,
    pub head: String,
    pub receipt: PathBuf,
    pub force_selected: bool,
}

#[derive(Debug, Serialize)]
struct ParserRatchetReceipt {
    schema_version: String,
    check: String,
    event: String,
    profile: String,
    base_sha: String,
    head_sha: String,
    selected: bool,
    selection_reason: Vec<String>,
    verdict: String,
    repro: Repro,
}

#[derive(Debug, Serialize)]
struct Repro {
    command: String,
}

pub fn run(config: ParserRatchetRunConfig) -> Result<()> {
    let profile_name = profile_name(config.profile);
    let base_sha = resolve_revision(&config.base)?;
    let head_sha = resolve_revision(&config.head)?;

    let selected = config.force_selected;
    let selection_reason = if selected {
        vec!["force-selected (scaffold only; measurements disabled)".to_string()]
    } else {
        vec!["not selected by ci-scope".to_string()]
    };

    let receipt = ParserRatchetReceipt {
        schema_version: "1".to_string(),
        check: "parser-ratchet".to_string(),
        event: "local".to_string(),
        profile: profile_name.to_string(),
        base_sha,
        head_sha,
        selected,
        selection_reason,
        verdict: "pass".to_string(),
        repro: Repro {
            command: format!(
                "cargo xtask parser-ratchet run --profile {} --base {} --head {} --receipt {}{}",
                profile_name,
                config.base,
                config.head,
                config.receipt.display(),
                if config.force_selected { " --force-selected" } else { "" }
            ),
        },
    };

    write_receipt(&config.receipt, &receipt)?;

    if registry_exists() {
        super::gate_receipts::validate(&config.receipt, super::gate_receipts::OutputFormat::Human)
            .map_err(|error| eyre!("parser-ratchet receipt failed registry validation: {error}"))?;
    }

    Ok(())
}

fn write_receipt(path: &Path, receipt: &ParserRatchetReceipt) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create receipt directory {}", parent.display()))?;
    }
    let payload = serde_json::to_string_pretty(receipt)?;
    fs::write(path, payload)
        .with_context(|| format!("failed to write receipt {}", path.display()))?;
    Ok(())
}

fn resolve_revision(reference: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", reference])
        .output()
        .with_context(|| format!("failed to run git rev-parse for '{reference}'"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(eyre!("failed to resolve git revision '{reference}': {stderr}"));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn registry_exists() -> bool {
    Path::new(".ci/receipts/registry.toml").exists()
}

fn profile_name(profile: RatchetProfile) -> &'static str {
    match profile {
        RatchetProfile::Pr => "pr",
        RatchetProfile::Nightly => "nightly",
        RatchetProfile::Release => "release",
    }
}
