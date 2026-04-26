use color_eyre::eyre::{Context, Result};
use serde::Serialize;
use std::{env, fs, path::PathBuf};

const CHECK_NAME: &str = "parser-ratchet";
const SCHEMA_VERSION: &str = "1.0.0";

#[derive(Debug, Clone)]
pub struct ParserRatchetConfig {
    pub profile: String,
    pub receipt_path: PathBuf,
}

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

pub fn run(config: ParserRatchetConfig) -> Result<()> {
    let receipt = ParserRatchetReceipt {
        check: CHECK_NAME,
        schema_version: SCHEMA_VERSION,
        event: read_ci_var("GITHUB_EVENT_NAME").unwrap_or_else(|| "local".to_string()),
        run_id: read_ci_var("GITHUB_RUN_ID"),
        base_sha: read_ci_var("GITHUB_BASE_SHA").or_else(|| read_ci_var("BASE_SHA")),
        head_sha: read_ci_var("GITHUB_HEAD_SHA").or_else(|| read_ci_var("HEAD_SHA")),
        candidate_sha: read_ci_var("GITHUB_SHA"),
        selected: false,
        selection_reason: "parser scope/comparison scaffold only; always no-op".to_string(),
        profile: config.profile,
        verdict: "pass",
        classification: "skipped",
        repro: Repro {
            command: "cargo xtask parser-ratchet --profile pr --receipt target/receipts/parser-ratchet.json"
                .to_string(),
        },
    };

    if let Some(parent) = config.receipt_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating receipt directory {}", parent.display()))?;
    }

    let payload = serde_json::to_string_pretty(&receipt).context("serializing parser-ratchet receipt")?;
    fs::write(&config.receipt_path, payload)
        .with_context(|| format!("writing receipt {}", config.receipt_path.display()))?;

    println!("parser-ratchet scaffold: selected=false verdict=pass");
    println!("receipt: {}", config.receipt_path.display());

    Ok(())
}

fn read_ci_var(key: &str) -> Option<String> {
    env::var(key).ok().filter(|value| !value.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_no_op_receipt() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let receipt_path = tmp.path().join("parser-ratchet.json");

        run(ParserRatchetConfig { profile: "pr".to_string(), receipt_path: receipt_path.clone() })?;

        let raw = fs::read_to_string(&receipt_path)?;
        let receipt: serde_json::Value = serde_json::from_str(&raw)?;

        assert_eq!(receipt["check"], CHECK_NAME);
        assert_eq!(receipt["selected"], false);
        assert_eq!(receipt["profile"], "pr");
        assert_eq!(receipt["verdict"], "pass");
        assert_eq!(receipt["classification"], "skipped");

        Ok(())
    }
}
