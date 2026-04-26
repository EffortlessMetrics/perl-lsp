use crate::tasks::parser_ratchet_compare;
use crate::utils::project_root;
use color_eyre::eyre::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

pub struct ParserRatchetRunConfig {
    pub profile: String,
    pub base_sha: String,
    pub head_sha: String,
    pub manifest: PathBuf,
    pub receipt: PathBuf,
}

pub struct ParserRatchetCompareConfig {
    pub profile: String,
    pub base_sha: Option<String>,
    pub head_sha: Option<String>,
    pub manifest: Option<PathBuf>,
    pub base_metrics: PathBuf,
    pub head_metrics: PathBuf,
    pub receipt: PathBuf,
}

pub fn run(config: ParserRatchetRunConfig) -> Result<()> {
    let root = project_root()?;
    let manifest_path = resolve_path(&root, &config.manifest);
    let profile_path =
        resolve_path(&root, &parser_ratchet_compare::default_profile_path(&config.profile));
    let receipt_path = resolve_path(&root, &config.receipt);

    // Integration hook: live base/head metric collection is intentionally
    // decoupled from comparison logic so PR-mode can compare two explicit
    // metric files generated from the same manifest.
    let base_metrics = root.join(format!("target/parser-ratchet/metrics-{}.json", config.base_sha));
    let head_metrics = root.join(format!("target/parser-ratchet/metrics-{}.json", config.head_sha));

    if !base_metrics.exists() || !head_metrics.exists() {
        bail!(
            "missing metrics files for live compare: {} and {}. Generate both from the same manifest ({}) and re-run.",
            base_metrics.display(),
            head_metrics.display(),
            manifest_path.display()
        );
    }

    let manifest_fingerprint = fingerprint(&manifest_path)?;
    let command = format!(
        "cargo xtask parser-ratchet --profile {} --base {} --head {} --manifest {} --receipt {}",
        config.profile,
        config.base_sha,
        config.head_sha,
        manifest_path.display(),
        receipt_path.display()
    );

    parser_ratchet_compare::compare_command(parser_ratchet_compare::CompareCommandConfig {
        profile_name: config.profile,
        profile_path,
        base_sha: config.base_sha,
        head_sha: config.head_sha,
        manifest_fingerprint,
        base_metrics,
        head_metrics,
        receipt_path,
        repro_command: command,
    })
}

pub fn compare(config: ParserRatchetCompareConfig) -> Result<()> {
    let root = project_root()?;
    let profile_path =
        resolve_path(&root, &parser_ratchet_compare::default_profile_path(&config.profile));
    let base_metrics = resolve_path(&root, &config.base_metrics);
    let head_metrics = resolve_path(&root, &config.head_metrics);
    let receipt_path = resolve_path(&root, &config.receipt);

    let base_sha = config.base_sha.unwrap_or_else(|| "base".to_string());
    let head_sha = config.head_sha.unwrap_or_else(|| "head".to_string());
    let manifest_fingerprint = if let Some(manifest) = config.manifest {
        fingerprint(&resolve_path(&root, &manifest))?
    } else {
        "unknown".to_string()
    };

    let command = format!(
        "cargo xtask parser-ratchet compare --base-metrics {} --head-metrics {} --receipt {}",
        base_metrics.display(),
        head_metrics.display(),
        receipt_path.display()
    );

    parser_ratchet_compare::compare_command(parser_ratchet_compare::CompareCommandConfig {
        profile_name: config.profile,
        profile_path,
        base_sha,
        head_sha,
        manifest_fingerprint,
        base_metrics,
        head_metrics,
        receipt_path,
        repro_command: command,
    })
}

fn resolve_path(root: &Path, p: &Path) -> PathBuf {
    if p.is_absolute() { p.to_path_buf() } else { root.join(p) }
}

fn fingerprint(manifest_path: &Path) -> Result<String> {
    let bytes = fs::read(manifest_path)
        .with_context(|| format!("failed to read manifest {}", manifest_path.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(format!("sha256:{:x}", hasher.finalize()))
}
