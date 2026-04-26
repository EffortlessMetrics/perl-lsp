use color_eyre::eyre::{Result, eyre};
use serde::Serialize;
use std::fs;
use std::path::PathBuf;

const DEFAULT_RECEIPT_PATH: &str = "target/receipts/parser-ratchet.json";
const CHECK_NAME: &str = "parser-ratchet";
const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone)]
pub struct ParserRatchetConfig {
    pub profile: String,
    pub receipt: PathBuf,
}

#[derive(Debug, Serialize)]
struct ParserRatchetReceipt {
    check: &'static str,
    schema_version: u32,
    event: Option<String>,
    run_id: Option<String>,
    base_sha: Option<String>,
    head_sha: Option<String>,
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

pub fn run(profile: String, receipt: Option<PathBuf>) -> Result<()> {
    let config = ParserRatchetConfig {
        profile,
        receipt: receipt.unwrap_or_else(|| PathBuf::from(DEFAULT_RECEIPT_PATH)),
    };
    run_with_config(config)
}

fn run_with_config(config: ParserRatchetConfig) -> Result<()> {
    let receipt = build_receipt(&config);
    write_receipt(&config.receipt, &receipt)?;

    println!(
        "{CHECK_NAME}: profile={} selected=false verdict=pass classification=skipped receipt={}",
        config.profile,
        config.receipt.display()
    );

    Ok(())
}

fn build_receipt(config: &ParserRatchetConfig) -> ParserRatchetReceipt {
    let event = env_var("GITHUB_EVENT_NAME");
    let run_id = env_var("GITHUB_RUN_ID");
    let base_sha = env_var("GITHUB_BASE_SHA");
    let head_sha = env_var("GITHUB_HEAD_SHA");
    let candidate_sha = env_var("GITHUB_CANDIDATE_SHA");

    let selection_reason =
        "Parser Ratchet scaffold: selection scope/comparison not implemented yet".to_string();

    ParserRatchetReceipt {
        check: CHECK_NAME,
        schema_version: SCHEMA_VERSION,
        event,
        run_id,
        base_sha,
        head_sha,
        candidate_sha,
        selected: false,
        selection_reason,
        profile: config.profile.clone(),
        verdict: "pass",
        classification: "skipped",
        repro: Repro {
            command: format!(
                "cargo xtask parser-ratchet --profile {} --receipt {}",
                config.profile,
                config.receipt.display()
            ),
        },
    }
}

fn env_var(name: &str) -> Option<String> {
    std::env::var(name).ok().and_then(|value| (!value.trim().is_empty()).then_some(value))
}

fn write_receipt(path: &PathBuf, receipt: &ParserRatchetReceipt) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| eyre!("failed to create {}: {error}", parent.display()))?;
    }

    let payload = serde_json::to_string_pretty(receipt)?;
    fs::write(path, format!("{payload}\n"))
        .map_err(|error| eyre!("failed to write {}: {error}", path.display()))
}
