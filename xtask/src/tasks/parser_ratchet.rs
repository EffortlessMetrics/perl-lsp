use crate::utils::project_root;
use color_eyre::eyre::{Context, Result};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_RECEIPT_PATH: &str = "target/receipts/parser-ratchet.json";
const CHECK_ID: &str = "parser-ratchet";
const SCHEMA_VERSION: &str = "1";
const DEFAULT_CLASSIFICATION: &str = "skipped";

#[derive(Debug, Serialize)]
struct ParserRatchetReceipt {
    check: &'static str,
    schema_version: &'static str,
    event: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    base_sha: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    head_sha: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    candidate_sha: Option<String>,
    selected: bool,
    selection_reason: String,
    profile: String,
    verdict: &'static str,
    classification: &'static str,
    repro: Repro,
}

#[derive(Debug, Serialize)]
struct Repro {
    command: String,
}

pub fn run(profile: String, receipt_path: Option<PathBuf>) -> Result<()> {
    let root = project_root()?;
    let receipt_path = resolve_receipt_path(&root, receipt_path);

    if let Some(parent) = receipt_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create receipt directory {}", parent.display()))?;
    }

    let repro_command = format!(
        "cargo xtask parser-ratchet --profile {} --receipt {}",
        profile,
        receipt_path.display()
    );

    let receipt = ParserRatchetReceipt {
        check: CHECK_ID,
        schema_version: SCHEMA_VERSION,
        event: env_or_default("GITHUB_EVENT_NAME", "manual"),
        run_id: env_optional("GITHUB_RUN_ID"),
        base_sha: env_optional("GITHUB_BASE_SHA"),
        head_sha: env_optional("GITHUB_HEAD_SHA"),
        candidate_sha: env_optional("GITHUB_SHA"),
        selected: false,
        selection_reason: "Parser ratchet scaffold is no-op until scope/comparison logic lands"
            .to_string(),
        profile,
        verdict: "pass",
        classification: DEFAULT_CLASSIFICATION,
        repro: Repro { command: repro_command },
    };

    let payload = serde_json::to_string_pretty(&receipt)
        .context("Failed to serialize parser-ratchet receipt")?;
    fs::write(&receipt_path, format!("{payload}\n"))
        .with_context(|| format!("Failed to write receipt to {}", receipt_path.display()))?;

    println!("parser-ratchet scaffold receipt written to {}", receipt_path.display());
    println!("{payload}");

    Ok(())
}

fn resolve_receipt_path(root: &Path, receipt_path: Option<PathBuf>) -> PathBuf {
    match receipt_path {
        Some(path) if path.is_absolute() => path,
        Some(path) => root.join(path),
        None => root.join(DEFAULT_RECEIPT_PATH),
    }
}

fn env_optional(key: &str) -> Option<String> {
    std::env::var(key).ok().and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
    })
}

fn env_or_default(key: &str, default: &str) -> String {
    env_optional(key).unwrap_or_else(|| default.to_string())
}
