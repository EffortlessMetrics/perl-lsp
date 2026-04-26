use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct RatchetProfile {
    #[serde(default)]
    pub clean_parse_rate_epsilon: f64,
    #[serde(default)]
    pub error_node_material_increase: usize,
    #[serde(default)]
    pub node_kind_drop_tolerance: usize,
}

impl Default for RatchetProfile {
    fn default() -> Self {
        Self {
            clean_parse_rate_epsilon: 0.0005,
            error_node_material_increase: 0,
            node_kind_drop_tolerance: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserRatchetMetrics {
    pub selected: String,
    #[serde(default)]
    pub concept_floors_pass: Option<bool>,
    pub panic_count: usize,
    pub timeout_count: usize,
    pub clean_parse_rate: f64,
    pub error_node_count: usize,
    pub node_kind_seen_count: usize,
    #[serde(default)]
    pub corpus_runtime_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ViolationLevel {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize)]
pub struct RatchetViolation {
    pub code: String,
    pub level: ViolationLevel,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReproCommand {
    pub command: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParserRatchetReceipt {
    pub check: String,
    pub profile: String,
    pub selected: String,
    pub selection_reason: String,
    pub manifest_fingerprint: String,
    pub base_sha: String,
    pub head_sha: String,
    pub metrics: ReceiptMetrics,
    pub violations: Vec<RatchetViolation>,
    pub ratchet_opportunity: bool,
    pub verdict: String,
    pub repro: ReproCommand,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReceiptMetrics {
    pub base: ParserRatchetMetrics,
    pub head: ParserRatchetMetrics,
}

pub fn load_profile(path: &Path) -> Result<RatchetProfile> {
    if !path.exists() {
        return Ok(RatchetProfile::default());
    }
    let text = fs::read_to_string(path)
        .with_context(|| format!("failed to read profile {}", path.display()))?;
    let profile: RatchetProfile = toml::from_str(&text)
        .with_context(|| format!("failed to parse profile {}", path.display()))?;
    Ok(profile)
}

pub fn compare_metrics(
    profile_name: &str,
    profile: &RatchetProfile,
    base_sha: &str,
    head_sha: &str,
    manifest_fingerprint: String,
    base: ParserRatchetMetrics,
    head: ParserRatchetMetrics,
    command: String,
) -> ParserRatchetReceipt {
    let selected = head.selected.clone();
    let mut violations = Vec::new();

    if base.selected != head.selected {
        violations.push(RatchetViolation {
            code: "selection.changed".to_string(),
            level: ViolationLevel::Error,
            message: format!(
                "selected corpus changed between base ({}) and head ({})",
                base.selected, head.selected
            ),
        });
    }

    let scope = selected.as_str();
    match scope {
        "perl-corpus" => compare_perl_corpus(profile, &base, &head, &mut violations),
        "system-perl" => compare_system_perl(profile, &base, &head, &mut violations),
        _ => {
            violations.push(RatchetViolation {
                code: "selection.unknown".to_string(),
                level: ViolationLevel::Error,
                message: format!("unsupported selected corpus scope '{scope}'"),
            });
        }
    }

    if let Some(base_runtime) = base.corpus_runtime_ms
        && let Some(head_runtime) = head.corpus_runtime_ms
        && head_runtime > base_runtime
    {
        violations.push(RatchetViolation {
            code: "runtime.regression".to_string(),
            level: ViolationLevel::Warning,
            message: format!(
                "runtime increased from {base_runtime}ms to {head_runtime}ms (advisory)"
            ),
        });
    }

    let has_error = violations.iter().any(|v| v.level == ViolationLevel::Error);
    let ratchet_opportunity = !has_error && is_improvement(&base, &head);

    ParserRatchetReceipt {
        check: "Parser Ratchet".to_string(),
        profile: profile_name.to_string(),
        selected,
        selection_reason: "profile-driven PR parser ratchet selection".to_string(),
        manifest_fingerprint,
        base_sha: base_sha.to_string(),
        head_sha: head_sha.to_string(),
        metrics: ReceiptMetrics { base, head },
        violations,
        ratchet_opportunity,
        verdict: if has_error { "fail" } else { "pass" }.to_string(),
        repro: ReproCommand { command },
    }
}

fn compare_perl_corpus(
    profile: &RatchetProfile,
    base: &ParserRatchetMetrics,
    head: &ParserRatchetMetrics,
    violations: &mut Vec<RatchetViolation>,
) {
    if head.panic_count > 0 {
        violations.push(error(
            "panic_count.nonzero",
            format!("panic_count={} (must be 0)", head.panic_count),
        ));
    }
    if head.timeout_count > 0 {
        violations.push(error(
            "timeout_count.nonzero",
            format!("timeout_count={} (must be 0)", head.timeout_count),
        ));
    }
    if matches!(head.concept_floors_pass, Some(false)) {
        violations
            .push(error("concept_floors.failed", "concept floors failed in head".to_string()));
    }
    if base.clean_parse_rate - head.clean_parse_rate > profile.clean_parse_rate_epsilon {
        violations.push(error(
            "clean_parse_rate.regression",
            format!(
                "clean_parse_rate regressed from {:.6} to {:.6}",
                base.clean_parse_rate, head.clean_parse_rate
            ),
        ));
    }
    if head.error_node_count.saturating_sub(base.error_node_count)
        > profile.error_node_material_increase
    {
        violations.push(error(
            "error_node_count.increase",
            format!(
                "error_node_count increased from {} to {}",
                base.error_node_count, head.error_node_count
            ),
        ));
    }
    if base.node_kind_seen_count.saturating_sub(head.node_kind_seen_count)
        > profile.node_kind_drop_tolerance
    {
        violations.push(error(
            "node_kind_seen_count.drop",
            format!(
                "node_kind_seen_count dropped from {} to {}",
                base.node_kind_seen_count, head.node_kind_seen_count
            ),
        ));
    }
}

fn compare_system_perl(
    profile: &RatchetProfile,
    base: &ParserRatchetMetrics,
    head: &ParserRatchetMetrics,
    violations: &mut Vec<RatchetViolation>,
) {
    if head.panic_count > base.panic_count {
        violations.push(error(
            "panic_count.worsened",
            format!("panic_count worsened from {} to {}", base.panic_count, head.panic_count),
        ));
    }
    if head.timeout_count > base.timeout_count {
        violations.push(error(
            "timeout_count.worsened",
            format!("timeout_count worsened from {} to {}", base.timeout_count, head.timeout_count),
        ));
    }
    if base.clean_parse_rate - head.clean_parse_rate > profile.clean_parse_rate_epsilon {
        violations.push(error(
            "clean_parse_rate.regression",
            format!(
                "clean_parse_rate regressed from {:.6} to {:.6}",
                base.clean_parse_rate, head.clean_parse_rate
            ),
        ));
    }
}

fn is_improvement(base: &ParserRatchetMetrics, head: &ParserRatchetMetrics) -> bool {
    head.panic_count < base.panic_count
        || head.timeout_count < base.timeout_count
        || head.clean_parse_rate > base.clean_parse_rate
        || head.error_node_count < base.error_node_count
        || head.node_kind_seen_count > base.node_kind_seen_count
}

fn error(code: &str, message: String) -> RatchetViolation {
    RatchetViolation { code: code.to_string(), level: ViolationLevel::Error, message }
}

pub fn load_metrics(path: &Path) -> Result<ParserRatchetMetrics> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read metrics file {}", path.display()))?;
    let parsed: ParserRatchetMetrics = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse metrics file {}", path.display()))?;
    Ok(parsed)
}

pub fn write_receipt(path: &Path, receipt: &ParserRatchetReceipt) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create receipt parent {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(receipt).context("failed to serialize receipt")?;
    fs::write(path, format!("{json}\n"))
        .with_context(|| format!("failed to write receipt {}", path.display()))?;
    Ok(())
}

pub struct CompareCommandConfig {
    pub profile_name: String,
    pub profile_path: PathBuf,
    pub base_sha: String,
    pub head_sha: String,
    pub manifest_fingerprint: String,
    pub base_metrics: PathBuf,
    pub head_metrics: PathBuf,
    pub receipt_path: PathBuf,
    pub repro_command: String,
}

pub fn compare_command(config: CompareCommandConfig) -> Result<()> {
    let profile = load_profile(&config.profile_path)?;
    let base = load_metrics(&config.base_metrics)?;
    let head = load_metrics(&config.head_metrics)?;
    let receipt = compare_metrics(
        &config.profile_name,
        &profile,
        &config.base_sha,
        &config.head_sha,
        config.manifest_fingerprint,
        base,
        head,
        config.repro_command,
    );
    write_receipt(&config.receipt_path, &receipt)?;
    if receipt.verdict == "fail" {
        bail!("parser ratchet comparison failed")
    }
    Ok(())
}

pub fn default_profile_path(profile_name: &str) -> PathBuf {
    PathBuf::from(format!(".ci/parser-ratchet/profiles/{profile_name}.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::Result;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(format!("tests/fixtures/parser-ratchet/{name}.json"))
    }

    #[test]
    fn equal_metrics_pass() -> Result<()> {
        let base = load_metrics(&fixture("equal-base"))?;
        let head = load_metrics(&fixture("equal-head"))?;
        let receipt = compare_metrics(
            "pr",
            &RatchetProfile::default(),
            "base",
            "head",
            "manifest123".to_string(),
            base,
            head,
            "cargo xtask parser-ratchet".to_string(),
        );
        assert_eq!(receipt.verdict, "pass");
        assert!(!receipt.ratchet_opportunity);
        Ok(())
    }

    #[test]
    fn improvement_sets_ratchet_opportunity() -> Result<()> {
        let base = load_metrics(&fixture("equal-base"))?;
        let head = load_metrics(&fixture("improvement-head"))?;
        let receipt = compare_metrics(
            "pr",
            &RatchetProfile::default(),
            "base",
            "head",
            "manifest123".to_string(),
            base,
            head,
            "cargo xtask parser-ratchet".to_string(),
        );
        assert_eq!(receipt.verdict, "pass");
        assert!(receipt.ratchet_opportunity);
        Ok(())
    }

    #[test]
    fn perl_corpus_panic_in_head_fails() -> Result<()> {
        let base = load_metrics(&fixture("equal-base"))?;
        let head = load_metrics(&fixture("perl-corpus-head-panic"))?;
        let receipt = compare_metrics(
            "pr",
            &RatchetProfile::default(),
            "base",
            "head",
            "manifest123".to_string(),
            base,
            head,
            "cargo xtask parser-ratchet".to_string(),
        );
        assert_eq!(receipt.verdict, "fail");
        assert!(receipt.violations.iter().any(|v| v.code == "panic_count.nonzero"));
        Ok(())
    }

    #[test]
    fn system_perl_existing_base_failure_unchanged_passes() -> Result<()> {
        let base = load_metrics(&fixture("system-base-existing-failure"))?;
        let head = load_metrics(&fixture("system-head-unchanged"))?;
        let receipt = compare_metrics(
            "pr",
            &RatchetProfile::default(),
            "base",
            "head",
            "manifest123".to_string(),
            base,
            head,
            "cargo xtask parser-ratchet".to_string(),
        );
        assert_eq!(receipt.verdict, "pass");
        Ok(())
    }

    #[test]
    fn system_perl_worsened_failure_fails() -> Result<()> {
        let base = load_metrics(&fixture("system-base-existing-failure"))?;
        let head = load_metrics(&fixture("system-head-worsened"))?;
        let receipt = compare_metrics(
            "pr",
            &RatchetProfile::default(),
            "base",
            "head",
            "manifest123".to_string(),
            base,
            head,
            "cargo xtask parser-ratchet".to_string(),
        );
        assert_eq!(receipt.verdict, "fail");
        Ok(())
    }

    #[test]
    fn runtime_only_regression_warns_and_passes() -> Result<()> {
        let base = load_metrics(&fixture("equal-base"))?;
        let head = load_metrics(&fixture("runtime-only-regression-head"))?;
        let receipt = compare_metrics(
            "pr",
            &RatchetProfile::default(),
            "base",
            "head",
            "manifest123".to_string(),
            base,
            head,
            "cargo xtask parser-ratchet".to_string(),
        );
        assert_eq!(receipt.verdict, "pass");
        assert!(
            receipt
                .violations
                .iter()
                .any(|v| v.code == "runtime.regression" && v.level == ViolationLevel::Warning)
        );
        Ok(())
    }
}
