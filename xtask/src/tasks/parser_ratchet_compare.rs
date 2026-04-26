use color_eyre::eyre::{Result, bail};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const DEFAULT_EPSILON: f64 = 0.001;
const DEFAULT_ERROR_NODE_INCREASE: u64 = 10;
const DEFAULT_NODE_KIND_DROP: u64 = 5;

#[derive(Debug, Clone, Deserialize)]
pub struct ParserRatchetProfile {
    pub profile: String,
    #[serde(default)]
    pub selected: Vec<String>,
    #[serde(default = "default_epsilon")]
    pub clean_parse_rate_epsilon: f64,
    #[serde(default = "default_error_node_increase")]
    pub error_node_material_increase: u64,
    #[serde(default = "default_node_kind_drop")]
    pub node_kind_unexpected_drop: u64,
}

fn default_epsilon() -> f64 {
    DEFAULT_EPSILON
}

fn default_error_node_increase() -> u64 {
    DEFAULT_ERROR_NODE_INCREASE
}

fn default_node_kind_drop() -> u64 {
    DEFAULT_NODE_KIND_DROP
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserRatchetMetrics {
    #[serde(default)]
    pub panic_count: u64,
    #[serde(default)]
    pub timeout_count: u64,
    #[serde(default = "default_concept_floors")]
    pub concept_floors_pass: bool,
    #[serde(default)]
    pub clean_parse_rate: f64,
    #[serde(default)]
    pub error_node_count: u64,
    #[serde(default)]
    pub node_kind_seen_count: u64,
    #[serde(default)]
    pub corpus_runtime_ms: u64,
}

fn default_concept_floors() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedMetrics {
    pub selected: String,
    pub metrics: ParserRatchetMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserRatchetViolation {
    pub selected: String,
    pub metric: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserRatchetCompareResult {
    pub violations: Vec<ParserRatchetViolation>,
    pub ratchet_opportunity: bool,
    pub runtime_regression: bool,
    pub verdict: String,
}

pub fn load_profile(path: &Path) -> Result<ParserRatchetProfile> {
    let raw = fs::read_to_string(path)?;
    let profile: ParserRatchetProfile = toml::from_str(&raw)?;
    Ok(profile)
}

pub fn read_metrics(path: &Path, selected: &str) -> Result<ParsedMetrics> {
    let raw = fs::read_to_string(path)?;
    let value: serde_json::Value = serde_json::from_str(&raw)?;

    let metrics = ParserRatchetMetrics {
        panic_count: extract_u64(&value, &["panic_count", "files_with_catastrophic_parse_failure"]),
        timeout_count: extract_u64(&value, &["timeout_count", "timed_out_files"]),
        concept_floors_pass: extract_bool(&value, &["concept_floors_pass"]).unwrap_or(true),
        clean_parse_rate: extract_rate(&value),
        error_node_count: extract_u64(&value, &["error_node_count", "total_error_nodes"]),
        node_kind_seen_count: extract_u64(&value, &["node_kind_seen_count"]),
        corpus_runtime_ms: extract_u64(&value, &["corpus_runtime_ms"]),
    };

    Ok(ParsedMetrics { selected: selected.to_string(), metrics })
}

pub fn compare_metrics(
    profile: &ParserRatchetProfile,
    base: &ParsedMetrics,
    head: &ParsedMetrics,
) -> Result<ParserRatchetCompareResult> {
    if base.selected != head.selected {
        bail!("selected scope mismatch: {} vs {}", base.selected, head.selected);
    }

    let selected = base.selected.as_str();
    let mut violations = Vec::new();
    let mut improved = false;
    let mut runtime_regression = false;

    match selected {
        "perl-corpus" => {
            if head.metrics.panic_count != 0 {
                violations.push(violation(
                    selected,
                    "panic_count",
                    "panic_count must be zero in head",
                ));
            }
            if head.metrics.timeout_count != 0 {
                violations.push(violation(
                    selected,
                    "timeout_count",
                    "timeout_count must be zero in head",
                ));
            }
            if !head.metrics.concept_floors_pass {
                violations.push(violation(
                    selected,
                    "concept_floors_pass",
                    "concept floors must pass",
                ));
            }

            rate_check(
                profile,
                selected,
                &base.metrics,
                &head.metrics,
                &mut violations,
                &mut improved,
            );
            error_node_check(
                profile,
                selected,
                &base.metrics,
                &head.metrics,
                &mut violations,
                &mut improved,
            );
            node_kind_check(
                profile,
                selected,
                &base.metrics,
                &head.metrics,
                &mut violations,
                &mut improved,
            );
        }
        "system-perl" => {
            if head.metrics.panic_count > base.metrics.panic_count {
                violations.push(violation(selected, "panic_count", "new/worsened panic_count"));
            }
            if head.metrics.timeout_count > base.metrics.timeout_count {
                violations.push(violation(selected, "timeout_count", "new/worsened timeout_count"));
            }
            rate_check(
                profile,
                selected,
                &base.metrics,
                &head.metrics,
                &mut violations,
                &mut improved,
            );
        }
        other => {
            bail!("unsupported selected scope: {other}");
        }
    }

    if head.metrics.corpus_runtime_ms > base.metrics.corpus_runtime_ms {
        runtime_regression = true;
    }

    let verdict = if violations.is_empty() { "pass" } else { "fail" }.to_string();

    Ok(ParserRatchetCompareResult {
        violations,
        ratchet_opportunity: improved,
        runtime_regression,
        verdict,
    })
}

fn rate_check(
    profile: &ParserRatchetProfile,
    selected: &str,
    base: &ParserRatchetMetrics,
    head: &ParserRatchetMetrics,
    violations: &mut Vec<ParserRatchetViolation>,
    improved: &mut bool,
) {
    if head.clean_parse_rate + profile.clean_parse_rate_epsilon < base.clean_parse_rate {
        violations.push(violation(
            selected,
            "clean_parse_rate",
            "clean_parse_rate regressed beyond epsilon",
        ));
    } else if head.clean_parse_rate > base.clean_parse_rate {
        *improved = true;
    }
}

fn error_node_check(
    profile: &ParserRatchetProfile,
    selected: &str,
    base: &ParserRatchetMetrics,
    head: &ParserRatchetMetrics,
    violations: &mut Vec<ParserRatchetViolation>,
    improved: &mut bool,
) {
    let allowed = base.error_node_count.saturating_add(profile.error_node_material_increase);
    if head.error_node_count > allowed {
        violations.push(violation(
            selected,
            "error_node_count",
            "error_node_count materially increased",
        ));
    } else if head.error_node_count < base.error_node_count {
        *improved = true;
    }
}

fn node_kind_check(
    profile: &ParserRatchetProfile,
    selected: &str,
    base: &ParserRatchetMetrics,
    head: &ParserRatchetMetrics,
    violations: &mut Vec<ParserRatchetViolation>,
    improved: &mut bool,
) {
    let floor = base.node_kind_seen_count.saturating_sub(profile.node_kind_unexpected_drop);
    if head.node_kind_seen_count < floor {
        violations.push(violation(
            selected,
            "node_kind_seen_count",
            "node_kind_seen_count dropped unexpectedly",
        ));
    } else if head.node_kind_seen_count > base.node_kind_seen_count {
        *improved = true;
    }
}

fn violation(selected: &str, metric: &str, detail: &str) -> ParserRatchetViolation {
    ParserRatchetViolation {
        selected: selected.to_string(),
        metric: metric.to_string(),
        detail: detail.to_string(),
    }
}

fn extract_u64(value: &serde_json::Value, keys: &[&str]) -> u64 {
    keys.iter().find_map(|key| value.get(*key).and_then(serde_json::Value::as_u64)).unwrap_or(0)
}

fn extract_bool(value: &serde_json::Value, keys: &[&str]) -> Option<bool> {
    keys.iter().find_map(|key| value.get(*key).and_then(serde_json::Value::as_bool))
}

fn extract_rate(value: &serde_json::Value) -> f64 {
    if let Some(rate) = value.get("clean_parse_rate").and_then(serde_json::Value::as_f64) {
        return rate;
    }

    let total = value.get("total_files").and_then(serde_json::Value::as_f64);
    let clean = value.get("clean_files").and_then(serde_json::Value::as_f64);
    match (total, clean) {
        (Some(total), Some(clean)) if total > 0.0 => clean / total,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::eyre::Result;
    use std::path::{Path, PathBuf};

    fn fixture(name: &str) -> PathBuf {
        Path::new("tests/fixtures/parser-ratchet").join(name)
    }

    fn profile() -> ParserRatchetProfile {
        ParserRatchetProfile {
            profile: "pr".to_string(),
            selected: vec!["perl-corpus".to_string(), "system-perl".to_string()],
            clean_parse_rate_epsilon: 0.001,
            error_node_material_increase: 10,
            node_kind_unexpected_drop: 5,
        }
    }

    #[test]
    fn equal_metrics_pass() -> Result<()> {
        let p = profile();
        let base = read_metrics(&fixture("equal-base.json"), "perl-corpus")?;
        let head = read_metrics(&fixture("equal-head.json"), "perl-corpus")?;
        let result = compare_metrics(&p, &base, &head)?;
        assert_eq!(result.verdict, "pass");
        assert!(!result.ratchet_opportunity);
        Ok(())
    }

    #[test]
    fn improvement_sets_ratchet_opportunity() -> Result<()> {
        let p = profile();
        let base = read_metrics(&fixture("equal-base.json"), "perl-corpus")?;
        let head = read_metrics(&fixture("improvement-head.json"), "perl-corpus")?;
        let result = compare_metrics(&p, &base, &head)?;
        assert_eq!(result.verdict, "pass");
        assert!(result.ratchet_opportunity);
        Ok(())
    }

    #[test]
    fn perl_corpus_panic_in_head_fails() -> Result<()> {
        let p = profile();
        let base = read_metrics(&fixture("equal-base.json"), "perl-corpus")?;
        let head = read_metrics(&fixture("perl-corpus-panic-head.json"), "perl-corpus")?;
        let result = compare_metrics(&p, &base, &head)?;
        assert_eq!(result.verdict, "fail");
        Ok(())
    }

    #[test]
    fn system_perl_existing_failure_unchanged_passes() -> Result<()> {
        let p = profile();
        let base = read_metrics(&fixture("system-base-failure.json"), "system-perl")?;
        let head = read_metrics(&fixture("system-unchanged-head.json"), "system-perl")?;
        let result = compare_metrics(&p, &base, &head)?;
        assert_eq!(result.verdict, "pass");
        Ok(())
    }

    #[test]
    fn system_perl_worsened_failure_fails() -> Result<()> {
        let p = profile();
        let base = read_metrics(&fixture("system-base-failure.json"), "system-perl")?;
        let head = read_metrics(&fixture("system-worsened-head.json"), "system-perl")?;
        let result = compare_metrics(&p, &base, &head)?;
        assert_eq!(result.verdict, "fail");
        Ok(())
    }

    #[test]
    fn runtime_only_regression_warns_but_passes() -> Result<()> {
        let p = profile();
        let base = read_metrics(&fixture("equal-base.json"), "perl-corpus")?;
        let head = read_metrics(&fixture("runtime-only-head.json"), "perl-corpus")?;
        let result = compare_metrics(&p, &base, &head)?;
        assert_eq!(result.verdict, "pass");
        assert!(result.runtime_regression);
        Ok(())
    }
}
