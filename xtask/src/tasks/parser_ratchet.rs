use crate::tasks::gate_receipts;
use anyhow::{Context, Result, bail};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const CHECK_NAME: &str = "parser-ratchet";
const RECEIPT_SCHEMA_VERSION: &str = "1";
const REGISTRY_PATH: &str = ".ci/receipts/registry.toml";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

#[derive(Debug)]
pub struct ParserRatchetRunConfig {
    pub profile: ParserRatchetProfile,
    pub base: String,
    pub head: String,
    pub receipt: PathBuf,
    pub force_selected: bool,
}

#[derive(Debug, Serialize)]
struct ParserRatchetReceipt {
    schema_version: &'static str,
    check: &'static str,
    event: &'static str,
    profile: String,
    base_sha: String,
    head_sha: String,
    selected: bool,
    selection_reason: String,
    verdict: &'static str,
    repro: Repro,
}

#[derive(Debug, Serialize)]
struct Repro {
    command: String,
}

pub fn run(config: ParserRatchetRunConfig) -> Result<()> {
    let workspace_root = std::env::current_dir().context("failed to get current directory")?;

    let base_sha = resolve_git_sha(&workspace_root, &config.base)
        .with_context(|| format!("failed to resolve --base ref '{}'", config.base))?;
    let head_sha = resolve_git_sha(&workspace_root, &config.head)
        .with_context(|| format!("failed to resolve --head ref '{}'", config.head))?;

    let (selected, selection_reason) = if config.force_selected {
        (true, "force-selected (scaffold only; measurement not yet implemented)")
    } else {
        (false, "not selected by ci-scope")
    };

    let receipt = ParserRatchetReceipt {
        schema_version: RECEIPT_SCHEMA_VERSION,
        check: CHECK_NAME,
        event: "local",
        profile: config.profile.as_str().to_string(),
        base_sha,
        head_sha,
        selected,
        selection_reason: selection_reason.to_string(),
        verdict: "pass",
        repro: Repro {
            command: format!(
                "cargo xtask parser-ratchet run --profile {} --base {} --head {} --receipt {}{}",
                config.profile.as_str(),
                config.base,
                config.head,
                config.receipt.display(),
                if config.force_selected { " --force-selected" } else { "" }
            ),
        },
    };

    if let Some(parent) = config.receipt.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create receipt directory {}", parent.display()))?;
    }

    let rendered = serde_json::to_string_pretty(&receipt).context("failed to serialize receipt")?;
    fs::write(&config.receipt, rendered)
        .with_context(|| format!("failed to write receipt to {}", config.receipt.display()))?;

    // Validate against the shared receipt registry/schema when available.
    if workspace_root.join(REGISTRY_PATH).exists() {
        gate_receipts::validate(&config.receipt, gate_receipts::OutputFormat::Human)
            .context("parser-ratchet receipt failed schema/registry validation")?;
    }

    println!("wrote parser-ratchet scaffold receipt to {}", config.receipt.display());
    Ok(())
}

fn resolve_git_sha(workspace_root: &Path, reference: &str) -> Result<String> {
    let output = Command::new("git")
        .current_dir(workspace_root)
        .args(["rev-parse", "--verify", &format!("{reference}^{{commit}}")])
        .output()
        .context("failed to execute git rev-parse")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git rev-parse failed for '{reference}': {}", stderr.trim());
    }

    let sha = String::from_utf8(output.stdout).context("git rev-parse stdout was not UTF-8")?;
    Ok(sha.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use tempfile::TempDir;

    #[test]
    fn profile_names_are_stable() {
        assert_eq!(ParserRatchetProfile::Pr.as_str(), "pr");
        assert_eq!(ParserRatchetProfile::Nightly.as_str(), "nightly");
        assert_eq!(ParserRatchetProfile::Release.as_str(), "release");
    }

    #[test]
    fn resolve_git_sha_fails_for_missing_ref() -> Result<()> {
        let dir = TempDir::new()?;
        let result = resolve_git_sha(dir.path(), "missing-ref");
        assert!(result.is_err(), "missing ref should fail");
        Ok(())
    }
}
