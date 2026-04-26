use crate::tasks::parser_ratchet_compare::{
    ParsedMetrics, ParserRatchetCompareResult, compare_metrics, load_profile, read_metrics,
};
use color_eyre::eyre::{Result, bail};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ParserRatchetRunConfig {
    pub profile: String,
    pub base_sha: String,
    pub head_sha: String,
    pub manifest: PathBuf,
    pub receipt: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ParserRatchetCompareConfig {
    pub profile: String,
    pub selected: String,
    pub base_metrics: PathBuf,
    pub head_metrics: PathBuf,
    pub receipt: PathBuf,
    pub base_sha: Option<String>,
    pub head_sha: Option<String>,
    pub manifest: Option<PathBuf>,
    pub selection_reason: Option<String>,
}

#[derive(Debug, Serialize)]
struct ParserRatchetReceipt {
    check: String,
    profile: String,
    selected: String,
    selection_reason: String,
    manifest_fingerprint: String,
    base_sha: String,
    candidate_sha: String,
    metrics: ReceiptMetrics,
    violations: Vec<crate::tasks::parser_ratchet_compare::ParserRatchetViolation>,
    ratchet_opportunity: bool,
    verdict: String,
    repro: Repro,
}

#[derive(Debug, Serialize)]
struct ReceiptMetrics {
    base: crate::tasks::parser_ratchet_compare::ParserRatchetMetrics,
    head: crate::tasks::parser_ratchet_compare::ParserRatchetMetrics,
}

#[derive(Debug, Serialize)]
struct Repro {
    command: String,
}

pub fn run(config: ParserRatchetRunConfig) -> Result<()> {
    let profile_path = profile_path(&config.profile);
    let profile = load_profile(&profile_path)?;
    if profile.profile != config.profile {
        bail!("profile file mismatch: expected {}, found {}", config.profile, profile.profile);
    }
    let selected = profile.selected.first().cloned().unwrap_or_else(|| "perl-corpus".to_string());

    let base_metrics = PathBuf::from("target/parser-ratchet/base-metrics.json");
    let head_metrics = PathBuf::from("target/parser-ratchet/head-metrics.json");

    if !(base_metrics.exists() && head_metrics.exists()) {
        bail!(
            "live metric acquisition hook not wired yet. expected {} and {}. \
             Generate metric JSON for base/head with the same manifest, then run `cargo xtask parser-ratchet compare ...`",
            base_metrics.display(),
            head_metrics.display()
        );
    }

    run_compare(ParserRatchetCompareConfig {
        profile: config.profile,
        selected,
        base_metrics,
        head_metrics,
        receipt: config.receipt,
        base_sha: Some(config.base_sha),
        head_sha: Some(config.head_sha),
        manifest: Some(config.manifest),
        selection_reason: Some("PR mode selected live base-vs-candidate comparison".to_string()),
    })
}

pub fn run_compare(config: ParserRatchetCompareConfig) -> Result<()> {
    let profile_path = profile_path(&config.profile);
    let profile = load_profile(&profile_path)?;
    if profile.profile != config.profile {
        bail!("profile file mismatch: expected {}, found {}", config.profile, profile.profile);
    }
    let base = read_metrics(&config.base_metrics, &config.selected)?;
    let head = read_metrics(&config.head_metrics, &config.selected)?;
    let comparison = compare_metrics(&profile, &base, &head)?;

    write_receipt(config, base, head, comparison)
}

fn write_receipt(
    config: ParserRatchetCompareConfig,
    base: ParsedMetrics,
    head: ParsedMetrics,
    comparison: ParserRatchetCompareResult,
) -> Result<()> {
    let manifest_fingerprint = match &config.manifest {
        Some(path) => fingerprint(path)?,
        None => "unknown".to_string(),
    };

    let base_sha = config.base_sha.unwrap_or_else(|| "base-unknown".to_string());
    let candidate_sha = config.head_sha.unwrap_or_else(|| "candidate-unknown".to_string());

    let repro = format!(
        "cargo xtask parser-ratchet compare --profile {} --selected {} --base-metrics {} --head-metrics {} --receipt {}",
        config.profile,
        config.selected,
        config.base_metrics.display(),
        config.head_metrics.display(),
        config.receipt.display()
    );

    let selection_reason =
        config.selection_reason.unwrap_or_else(|| "profile default selection".to_string());

    let receipt = ParserRatchetReceipt {
        check: "Parser Ratchet".to_string(),
        profile: config.profile,
        selected: config.selected,
        selection_reason,
        manifest_fingerprint,
        base_sha,
        candidate_sha,
        metrics: ReceiptMetrics { base: base.metrics, head: head.metrics },
        violations: comparison.violations,
        ratchet_opportunity: comparison.ratchet_opportunity,
        verdict: comparison.verdict,
        repro: Repro { command: repro },
    };

    if let Some(parent) = config.receipt.parent() {
        fs::create_dir_all(parent)?;
    }

    let pretty = serde_json::to_string_pretty(&receipt)?;
    fs::write(&config.receipt, format!("{pretty}\n"))?;

    if receipt.verdict == "fail" {
        bail!("Parser Ratchet failed; see {}", config.receipt.display());
    }

    Ok(())
}

fn profile_path(profile: &str) -> PathBuf {
    PathBuf::from(format!(".ci/parser-ratchet/profiles/{profile}.toml"))
}

fn fingerprint(path: &Path) -> Result<String> {
    let bytes = fs::read(path)?;
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    Ok(format!("fnv64:{hash:016x}"))
}
