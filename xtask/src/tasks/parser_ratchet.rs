use color_eyre::eyre::{Context, Result};
use serde::Serialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ParserRatchetConfig {
    pub profile: String,
    pub receipt: PathBuf,
}

#[derive(Debug, Serialize)]
struct ParserRatchetReceipt {
    check: &'static str,
    schema_version: u32,
    event: String,
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

pub fn run(config: ParserRatchetConfig) -> Result<()> {
    let event = env_or_default("PARSER_RATCHET_EVENT", "unknown");
    let run_id = env_nonempty("PARSER_RATCHET_RUN_ID").or_else(|| env_nonempty("GITHUB_RUN_ID"));
    let base_sha = env_nonempty("PARSER_RATCHET_BASE_SHA");
    let head_sha = env_nonempty("PARSER_RATCHET_HEAD_SHA").or_else(|| env_nonempty("GITHUB_SHA"));
    let candidate_sha = env_nonempty("PARSER_RATCHET_CANDIDATE_SHA");

    let receipt = ParserRatchetReceipt {
        check: "parser-ratchet",
        schema_version: 1,
        event,
        run_id,
        base_sha,
        head_sha,
        candidate_sha,
        selected: false,
        selection_reason: "no-op scaffold: parser scope selection not implemented".to_string(),
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
    };

    if let Some(parent) = config.receipt.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("failed to create receipt directory {}", parent.display())
        })?;
    }

    let raw = serde_json::to_string_pretty(&receipt).context("failed to serialize parser receipt")?;
    fs::write(&config.receipt, raw)
        .with_context(|| format!("failed to write receipt {}", config.receipt.display()))?;

    println!(
        "parser-ratchet: selected=false verdict=pass receipt={} ",
        config.receipt.display()
    );

    Ok(())
}

fn env_nonempty(name: &str) -> Option<String> {
    std::env::var(name).ok().and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
    })
}

fn env_or_default(name: &str, default: &str) -> String {
    env_nonempty(name).unwrap_or_else(|| default.to_string())
}
