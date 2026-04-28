use color_eyre::eyre::{Result, WrapErr, bail, eyre};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ParserRatchetProfile {
    Pr,
    Nightly,
    Release,
}

impl ParserRatchetProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pr => "pr",
            Self::Nightly => "nightly",
            Self::Release => "release",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParserRatchetRunArgs {
    pub profile: ParserRatchetProfile,
    pub base: String,
    pub head: String,
    pub receipt: PathBuf,
    pub force_selected: bool,
}

#[derive(Debug, Serialize)]
struct ParserRatchetReceipt {
    schema_version: u32,
    check: &'static str,
    event: &'static str,
    profile: String,
    base_sha: String,
    head_sha: String,
    selected: bool,
    selection_reason: Vec<String>,
    verdict: &'static str,
    repro: Repro,
}

#[derive(Debug, Serialize)]
struct Repro {
    command: String,
}

pub fn run(args: ParserRatchetRunArgs) -> Result<()> {
    let base_sha = resolve_git_revision(&args.base)?;
    let head_sha = resolve_git_revision(&args.head)?;

    let selected = args.force_selected;
    let selection_reason = if selected {
        vec!["forced by --force-selected (scaffold-only run)".to_string()]
    } else {
        vec!["not selected by ci-scope".to_string()]
    };

    let receipt = ParserRatchetReceipt {
        schema_version: 1,
        check: "parser-ratchet",
        event: "local",
        profile: args.profile.as_str().to_string(),
        base_sha,
        head_sha,
        selected,
        selection_reason,
        verdict: "pass",
        repro: Repro {
            command: format!(
                "cargo xtask parser-ratchet run --profile {} --base {} --head {} --receipt {}{}",
                args.profile.as_str(),
                args.base,
                args.head,
                args.receipt.display(),
                if args.force_selected { " --force-selected" } else { "" }
            ),
        },
    };

    write_receipt(&args.receipt, &receipt)?;
    validate_receipt_if_registry_available(&args.receipt)?;
    println!("parser-ratchet scaffold receipt written: {}", args.receipt.display());
    Ok(())
}

fn resolve_git_revision(rev: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", rev])
        .output()
        .wrap_err_with(|| format!("failed to invoke git rev-parse for '{rev}'"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        bail!("invalid git revision '{rev}': {stderr}");
    }

    let sha = String::from_utf8(output.stdout)
        .wrap_err("git rev-parse output was not valid UTF-8")?
        .trim()
        .to_string();

    if sha.is_empty() {
        bail!("resolved git revision '{rev}' to an empty SHA");
    }

    Ok(sha)
}

fn write_receipt(path: &Path, receipt: &ParserRatchetReceipt) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .wrap_err_with(|| format!("failed to create receipt directory {}", parent.display()))?;
    }

    let rendered = serde_json::to_string_pretty(receipt).wrap_err("failed to serialize receipt")?;
    fs::write(path, format!("{rendered}\n"))
        .wrap_err_with(|| format!("failed to write receipt {}", path.display()))
}

fn validate_receipt_if_registry_available(receipt_path: &Path) -> Result<()> {
    let registry_path = Path::new(".ci/receipts/registry.toml");
    if registry_path.exists() {
        crate::tasks::gate_receipts::validate(
            receipt_path,
            crate::tasks::gate_receipts::OutputFormat::Human,
        )
        .map_err(|error| eyre!("parser-ratchet receipt failed gate schema validation: {error}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_string_mapping_is_stable() {
        assert_eq!(ParserRatchetProfile::Pr.as_str(), "pr");
        assert_eq!(ParserRatchetProfile::Nightly.as_str(), "nightly");
        assert_eq!(ParserRatchetProfile::Release.as_str(), "release");
    }
}
