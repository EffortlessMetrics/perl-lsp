use crate::tasks::parser_corpus_sweep::SweepReport;
use crate::tasks::parser_ratchet_compare::{
    ParserRatchetMetrics, ParserRatchetViolation, compare_metrics,
};
use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

#[derive(Debug, Clone)]
pub struct ParserRatchetConfig {
    pub profile: String,
    pub base_sha: String,
    pub head_sha: String,
    pub manifest: PathBuf,
    pub receipt: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserRatchetReceipt {
    pub check: String,
    pub profile: String,
    pub selected: String,
    pub selection_reason: String,
    pub manifest_fingerprint: String,
    pub base_sha: String,
    pub candidate_sha: String,
    pub metrics: ParserRatchetReceiptMetrics,
    pub violations: Vec<ParserRatchetViolation>,
    pub ratchet_opportunity: bool,
    pub verdict: String,
    pub repro: ParserRatchetRepro,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserRatchetReceiptMetrics {
    pub base: ParserRatchetMetrics,
    pub head: ParserRatchetMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserRatchetRepro {
    pub command: String,
}

pub fn run(config: ParserRatchetConfig) -> Result<()> {
    let manifest_path = resolve_path(&config.manifest)?;
    let manifest_text = fs::read_to_string(&manifest_path)
        .with_context(|| format!("failed to read manifest {}", manifest_path.display()))?;
    let mut hasher = DefaultHasher::new();
    manifest_text.hash(&mut hasher);
    let manifest_fingerprint = format!("stdhash:{:x}", hasher.finish());

    let profile_path =
        PathBuf::from(format!(".ci/parser-ratchet/profiles/{}.toml", config.profile));
    let profile_text = fs::read_to_string(&profile_path)
        .with_context(|| format!("failed to read {}", profile_path.display()))?;
    let profile: toml::Table = toml::from_str(&profile_text)
        .with_context(|| format!("failed to parse {}", profile_path.display()))?;
    let selected =
        profile.get("selected").and_then(toml::Value::as_str).unwrap_or("perl-corpus").to_string();
    let selection_reason = profile
        .get("selection_reason")
        .and_then(toml::Value::as_str)
        .unwrap_or("profile default")
        .to_string();

    let temp_dir = TempDir::new().context("failed to create temp dir for parser ratchet")?;
    let stable_manifest = temp_dir.path().join("corpus-manifest.txt");
    fs::write(&stable_manifest, manifest_text).context("failed to write normalized manifest")?;

    let base_report = run_sweep_for_sha(&config.base_sha, &stable_manifest, &selected)?;
    let head_report = run_sweep_for_sha(&config.head_sha, &stable_manifest, &selected)?;

    let base_metrics = metrics_from_sweep(&base_report, &selected);
    let head_metrics = metrics_from_sweep(&head_report, &selected);
    let comparison = compare_metrics(&base_metrics, &head_metrics);

    let receipt = ParserRatchetReceipt {
        check: "Parser Ratchet".to_string(),
        profile: config.profile,
        selected,
        selection_reason,
        manifest_fingerprint,
        base_sha: config.base_sha.clone(),
        candidate_sha: config.head_sha.clone(),
        metrics: ParserRatchetReceiptMetrics { base: base_metrics, head: head_metrics },
        violations: comparison.violations,
        ratchet_opportunity: comparison.ratchet_opportunity,
        verdict: comparison.verdict,
        repro: ParserRatchetRepro {
            command: format!(
                "cargo xtask parser-ratchet --profile pr --base {} --head {} --manifest {} --receipt {}",
                receipt_escape(&config.base_sha),
                receipt_escape(&config.head_sha),
                manifest_path.display(),
                config.receipt.display()
            ),
        },
    };

    if let Some(parent) = config.receipt.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(&config.receipt, format!("{}\n", serde_json::to_string_pretty(&receipt)?))
        .with_context(|| format!("failed to write {}", config.receipt.display()))?;

    if receipt.verdict == "fail" {
        bail!("parser ratchet failed")
    }
    Ok(())
}

fn run_sweep_for_sha(sha: &str, manifest: &Path, selected: &str) -> Result<SweepReport> {
    let repo_root =
        PathBuf::from(".").canonicalize().context("failed to canonicalize repo root")?;
    let worktree_root = tempfile::tempdir().context("failed to create temp worktree dir")?;
    let worktree_path = worktree_root.path().join(format!("ratchet-{}", &sha[..sha.len().min(12)]));

    let add_status = Command::new("git")
        .arg("worktree")
        .arg("add")
        .arg("--detach")
        .arg(&worktree_path)
        .arg(sha)
        .status()
        .context("failed to execute git worktree add")?;
    if !add_status.success() {
        bail!("git worktree add failed for {sha}");
    }

    let output_path = worktree_path.join("target/parser-ratchet-metrics.json");
    let mut cmd = Command::new("cargo");
    cmd.current_dir(&worktree_path)
        .arg("run")
        .arg("-p")
        .arg("xtask")
        .arg("--")
        .arg("parser-corpus-sweep")
        .arg("--manifest")
        .arg(manifest)
        .arg("--output")
        .arg(&output_path);

    let status = cmd.status().context("failed to run parser-corpus-sweep")?;

    let remove_status = Command::new("git")
        .current_dir(&repo_root)
        .arg("worktree")
        .arg("remove")
        .arg("--force")
        .arg(&worktree_path)
        .status()
        .context("failed to remove temporary worktree")?;
    if !remove_status.success() {
        bail!("failed to remove temporary worktree {}", worktree_path.display());
    }

    if !status.success() {
        bail!("parser-corpus-sweep failed at commit {sha}");
    }

    let output_text = fs::read_to_string(output_path)
        .with_context(|| format!("failed to read sweep output for {sha}"))?;
    let mut report: SweepReport = serde_json::from_str(&output_text)
        .with_context(|| format!("failed to parse sweep output for {sha}"))?;
    report.corpus_profile = selected.to_string();
    Ok(report)
}

fn metrics_from_sweep(report: &SweepReport, scope: &str) -> ParserRatchetMetrics {
    let total = report.total_files.max(1) as f64;
    let mut concept_floors = BTreeMap::new();
    concept_floors.insert(
        "clean_parse_nonzero".to_string(),
        report.clean_files > 0 || report.total_files == 0,
    );
    ParserRatchetMetrics {
        panic_count: report.files_with_catastrophic_parse_failure as u64,
        timeout_count: 0,
        clean_parse_rate: report.clean_files as f64 / total,
        error_node_count: report.total_error_nodes as u64,
        node_kind_seen_count: None,
        concept_floors,
        corpus_runtime_ms: (report.elapsed_secs * 1000.0).round() as u64,
        scope: scope.to_string(),
    }
}

fn resolve_path(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() { Ok(path.to_path_buf()) } else { Ok(PathBuf::from(".").join(path)) }
}

fn receipt_escape(value: &str) -> String {
    value.replace('"', "\\\"")
}
