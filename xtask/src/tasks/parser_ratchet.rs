use crate::tasks::parser_corpus_sweep::{self, SweepReport};
use crate::tasks::parser_ratchet_compare::{
    CompareConfig, ParserRatchetMetrics, ParserRatchetVerdict, compare_metrics,
};
use color_eyre::eyre::{Context, Result, bail};
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

#[derive(Debug, Clone, Serialize)]
struct ParserRatchetReceipt {
    check: String,
    profile: String,
    selected: String,
    selection_reason: String,
    manifest_fingerprint: String,
    base_sha: String,
    candidate_sha: String,
    head_sha: String,
    metrics: ReceiptMetrics,
    violations: Vec<crate::tasks::parser_ratchet_compare::RatchetViolation>,
    ratchet_opportunity: bool,
    verdict: ParserRatchetVerdict,
    repro: Repro,
}

#[derive(Debug, Clone, Serialize)]
struct ReceiptMetrics {
    base: ParserRatchetMetrics,
    head: ParserRatchetMetrics,
}

#[derive(Debug, Clone, Serialize)]
struct Repro {
    command: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct ProfileConfig {
    selected: Option<String>,
    selection_reason: Option<String>,
    clean_parse_rate_epsilon: Option<f64>,
    error_node_material_increase: Option<u64>,
    node_kind_unexpected_drop: Option<u64>,
    runtime_regression_warn_ms: Option<u64>,
}

pub fn run(config: ParserRatchetRunConfig) -> Result<()> {
    let profile = load_profile(&config.profile)?;
    let selected = profile.selected.unwrap_or_else(|| "perl-corpus".to_string());
    let selection_reason = profile.selection_reason.unwrap_or_else(|| {
        "PR mode default: compare base vs candidate on one discovered manifest".to_string()
    });
    let compare_config = CompareConfig {
        clean_parse_rate_epsilon: profile.clean_parse_rate_epsilon.unwrap_or(0.0005),
        error_node_material_increase: profile.error_node_material_increase.unwrap_or(0),
        node_kind_unexpected_drop: profile.node_kind_unexpected_drop.unwrap_or(0),
        runtime_regression_warn_ms: profile.runtime_regression_warn_ms.unwrap_or(10_000),
    };

    let manifest_fingerprint = fingerprint_manifest(&config.manifest)?;

    let base_report_path = PathBuf::from("target/parser-ratchet/base-metrics.json");
    let head_report_path = PathBuf::from("target/parser-ratchet/head-metrics.json");
    if let Some(parent) = base_report_path.parent() {
        fs::create_dir_all(parent)?;
    }

    run_sweep_for_sha(&config.base_sha, &config.manifest, &base_report_path)?;
    run_sweep_for_current(&config.manifest, &head_report_path)?;

    let base = normalize_report(&selected, &base_report_path)?;
    let head = normalize_report(&selected, &head_report_path)?;

    let outcome = compare_metrics(&selected, &base, &head, &compare_config);

    if let Some(parent) = config.receipt.parent() {
        fs::create_dir_all(parent)?;
    }
    let receipt = ParserRatchetReceipt {
        check: "Parser Ratchet".to_string(),
        profile: config.profile,
        selected,
        selection_reason,
        manifest_fingerprint,
        base_sha: config.base_sha,
        candidate_sha: config.head_sha.clone(),
        head_sha: config.head_sha,
        metrics: ReceiptMetrics { base, head },
        violations: outcome.violations,
        ratchet_opportunity: outcome.ratchet_opportunity,
        verdict: outcome.verdict.clone(),
        repro: Repro {
            command: format!(
                "cargo xtask parser-ratchet --profile pr --base {} --head {} --manifest {} --receipt {}",
                receipt_safe("$BASE_SHA"),
                receipt_safe("$HEAD_SHA"),
                receipt_safe("target/parser-ratchet/corpus-manifest.json"),
                receipt_safe("target/receipts/parser-ratchet.json"),
            ),
        },
    };
    fs::write(&config.receipt, serde_json::to_string_pretty(&receipt)?)?;

    if matches!(receipt.verdict, ParserRatchetVerdict::Fail) {
        bail!("parser-ratchet failed");
    }
    Ok(())
}

fn receipt_safe(value: &str) -> String {
    value.to_string()
}

fn run_sweep_for_current(manifest: &Path, output: &Path) -> Result<()> {
    parser_corpus_sweep::run(parser_corpus_sweep::SweepConfig {
        corpus_profile: Some("parser-ratchet".to_string()),
        base_roots: parser_corpus_sweep::default_base_roots(),
        corpus_roots: parser_corpus_sweep::resolve_corpus_roots(
            &parser_corpus_sweep::default_base_roots(),
        ),
        manifest_path: Some(manifest.to_path_buf()),
        manifest_perl5lib: Vec::new(),
        output_path: Some(output.to_path_buf()),
        baseline_path: None,
        enforce: false,
        verbose: false,
        receipt: false,
    })
}

fn run_sweep_for_sha(base_sha: &str, manifest: &Path, output: &Path) -> Result<()> {
    let root = crate::utils::project_root()?;
    let worktree = root.join("target/parser-ratchet/base-worktree");
    if worktree.exists() {
        let _ = std::process::Command::new("git")
            .arg("worktree")
            .arg("remove")
            .arg("--force")
            .arg(&worktree)
            .current_dir(&root)
            .status();
    }

    let add_status = std::process::Command::new("git")
        .arg("worktree")
        .arg("add")
        .arg("--detach")
        .arg(&worktree)
        .arg(base_sha)
        .current_dir(&root)
        .status()
        .context("failed to create base worktree")?;
    if !add_status.success() {
        bail!("unable to create worktree for base sha {base_sha}");
    }

    let status = std::process::Command::new("cargo")
        .arg("run")
        .arg("-p")
        .arg("xtask")
        .arg("--")
        .arg("parser-corpus-sweep")
        .arg("--manifest")
        .arg(manifest)
        .arg("--output")
        .arg(output)
        .current_dir(&worktree)
        .status()
        .context("failed to run base parser sweep")?;

    let _ = std::process::Command::new("git")
        .arg("worktree")
        .arg("remove")
        .arg("--force")
        .arg(&worktree)
        .current_dir(&root)
        .status();

    if !status.success() {
        bail!("base parser sweep failed for sha {base_sha}");
    }
    Ok(())
}

fn load_profile(profile: &str) -> Result<ProfileConfig> {
    let path = PathBuf::from(format!(".ci/parser-ratchet/profiles/{profile}.toml"));
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read parser-ratchet profile {}", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("invalid profile {}", path.display()))
}

fn fingerprint_manifest(path: &Path) -> Result<String> {
    let data = fs::read(path)?;
    let mut hash: u64 = 1469598103934665603;
    for byte in data {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(1099511628211);
    }
    Ok(format!("fnv1a64:{hash:016x}"))
}

fn normalize_report(selected: &str, path: &Path) -> Result<ParserRatchetMetrics> {
    let report: SweepReport = serde_json::from_str(&fs::read_to_string(path)?)?;
    let clean_parse_rate = if report.total_files == 0 {
        1.0
    } else {
        report.clean_files as f64 / report.total_files as f64
    };
    Ok(ParserRatchetMetrics {
        selected: selected.to_string(),
        clean_parse_rate,
        panic_count: report.files_with_catastrophic_parse_failure as u64,
        timeout_count: 0,
        error_node_count: report.total_error_nodes as u64,
        node_kind_seen_count: None,
        concept_floors_pass: None,
        corpus_runtime_ms: report.phase_timings.map(|p| p.total_ms),
    })
}
