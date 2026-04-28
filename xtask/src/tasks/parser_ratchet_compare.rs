use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ParserRatchetMetrics {
    #[serde(default)]
    pub panic_count: u64,
    #[serde(default)]
    pub timeout_count: u64,
    #[serde(default)]
    pub clean_parse_rate: f64,
    #[serde(default)]
    pub error_node_count: u64,
    pub node_kind_seen_count: Option<u64>,
    #[serde(default)]
    pub concept_floors: BTreeMap<String, bool>,
    #[serde(default)]
    pub corpus_runtime_ms: u64,
    #[serde(default)]
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserRatchetViolation {
    pub scope: String,
    pub metric: String,
    pub severity: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserRatchetComparison {
    pub violations: Vec<ParserRatchetViolation>,
    pub ratchet_opportunity: bool,
    pub verdict: String,
}

#[derive(Debug, Clone)]
pub struct CompareConfig {
    pub base_metrics: PathBuf,
    pub head_metrics: PathBuf,
    pub receipt: PathBuf,
}

const EPSILON: f64 = 0.0005;
const ERROR_NODE_MATERIAL_DELTA: u64 = 5;

pub fn run_compare(config: CompareConfig) -> Result<()> {
    let base = load_metrics(&config.base_metrics)?;
    let head = load_metrics(&config.head_metrics)?;
    let comparison = compare_metrics(&base, &head);

    if let Some(parent) = config.receipt.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let payload = serde_json::json!({
        "check": "Parser Ratchet",
        "metrics": {
            "base": base,
            "head": head,
        },
        "violations": comparison.violations,
        "ratchet_opportunity": comparison.ratchet_opportunity,
        "verdict": comparison.verdict,
    });
    fs::write(&config.receipt, format!("{}\n", serde_json::to_string_pretty(&payload)?))
        .with_context(|| format!("failed to write {}", config.receipt.display()))?;

    if payload["verdict"] == "fail" {
        bail!("parser ratchet comparison failed")
    }
    Ok(())
}

pub fn compare_metrics(
    base: &ParserRatchetMetrics,
    head: &ParserRatchetMetrics,
) -> ParserRatchetComparison {
    let scope = if head.scope.is_empty() {
        if base.scope.is_empty() { "perl-corpus".to_string() } else { base.scope.clone() }
    } else {
        head.scope.clone()
    };

    let mut violations = Vec::new();
    let mut improved = false;

    let clean_rate_drop = base.clean_parse_rate - head.clean_parse_rate;
    if clean_rate_drop > EPSILON {
        violations.push(ParserRatchetViolation {
            scope: scope.clone(),
            metric: "clean_parse_rate".to_string(),
            severity: "error".to_string(),
            message: format!(
                "clean_parse_rate regressed from {:.5} to {:.5}",
                base.clean_parse_rate, head.clean_parse_rate
            ),
        });
    } else if head.clean_parse_rate > base.clean_parse_rate + EPSILON {
        improved = true;
    }

    if scope == "perl-corpus" {
        if head.panic_count > 0 {
            violations.push(ParserRatchetViolation {
                scope: scope.clone(),
                metric: "panic_count".to_string(),
                severity: "error".to_string(),
                message: format!("panic_count must be zero in head, got {}", head.panic_count),
            });
        }
        if head.timeout_count > 0 {
            violations.push(ParserRatchetViolation {
                scope: scope.clone(),
                metric: "timeout_count".to_string(),
                severity: "error".to_string(),
                message: format!("timeout_count must be zero in head, got {}", head.timeout_count),
            });
        }
        for (name, passed) in &head.concept_floors {
            if !*passed {
                violations.push(ParserRatchetViolation {
                    scope: scope.clone(),
                    metric: format!("concept_floor:{name}"),
                    severity: "error".to_string(),
                    message: format!("concept floor '{name}' failed"),
                });
            }
        }
        if head.error_node_count > base.error_node_count.saturating_add(ERROR_NODE_MATERIAL_DELTA) {
            violations.push(ParserRatchetViolation {
                scope: scope.clone(),
                metric: "error_node_count".to_string(),
                severity: "error".to_string(),
                message: format!(
                    "error_node_count materially increased from {} to {}",
                    base.error_node_count, head.error_node_count
                ),
            });
        } else if head.error_node_count + ERROR_NODE_MATERIAL_DELTA < base.error_node_count {
            improved = true;
        }
        if let (Some(b), Some(h)) = (base.node_kind_seen_count, head.node_kind_seen_count) {
            if h < b {
                violations.push(ParserRatchetViolation {
                    scope: scope.clone(),
                    metric: "node_kind_seen_count".to_string(),
                    severity: "error".to_string(),
                    message: format!("node_kind_seen_count dropped from {b} to {h}"),
                });
            } else if h > b {
                improved = true;
            }
        }
    } else {
        if head.panic_count > base.panic_count {
            violations.push(ParserRatchetViolation {
                scope: scope.clone(),
                metric: "panic_count".to_string(),
                severity: "error".to_string(),
                message: format!(
                    "panic_count worsened from {} to {}",
                    base.panic_count, head.panic_count
                ),
            });
        }
        if head.timeout_count > base.timeout_count {
            violations.push(ParserRatchetViolation {
                scope: scope.clone(),
                metric: "timeout_count".to_string(),
                severity: "error".to_string(),
                message: format!(
                    "timeout_count worsened from {} to {}",
                    base.timeout_count, head.timeout_count
                ),
            });
        }
        if head.panic_count < base.panic_count || head.timeout_count < base.timeout_count {
            improved = true;
        }
    }

    if head.corpus_runtime_ms > base.corpus_runtime_ms {
        violations.push(ParserRatchetViolation {
            scope: scope.clone(),
            metric: "corpus_runtime_ms".to_string(),
            severity: "warn".to_string(),
            message: format!(
                "runtime regressed from {}ms to {}ms",
                base.corpus_runtime_ms, head.corpus_runtime_ms
            ),
        });
    }

    let fail_count = violations.iter().filter(|v| v.severity == "error").count();
    ParserRatchetComparison {
        violations,
        ratchet_opportunity: fail_count == 0 && improved,
        verdict: if fail_count == 0 { "pass".to_string() } else { "fail".to_string() },
    }
}

pub fn load_metrics(path: &Path) -> Result<ParserRatchetMetrics> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("failed to read metrics {}", path.display()))?;
    serde_json::from_str(&text)
        .with_context(|| format!("failed to parse metrics {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::Result;

    fn fixture(path: &str) -> PathBuf {
        PathBuf::from("tests/fixtures/parser-ratchet").join(path)
    }

    #[test]
    fn equal_metrics_pass() -> Result<()> {
        let base = load_metrics(&fixture("equal.base.json"))?;
        let head = load_metrics(&fixture("equal.head.json"))?;
        let comparison = compare_metrics(&base, &head);
        assert_eq!(comparison.verdict, "pass");
        assert!(!comparison.ratchet_opportunity);
        Ok(())
    }

    #[test]
    fn improvement_sets_ratchet_opportunity() -> Result<()> {
        let base = load_metrics(&fixture("improvement.base.json"))?;
        let head = load_metrics(&fixture("improvement.head.json"))?;
        let comparison = compare_metrics(&base, &head);
        assert_eq!(comparison.verdict, "pass");
        assert!(comparison.ratchet_opportunity);
        Ok(())
    }

    #[test]
    fn perl_corpus_panic_fails() -> Result<()> {
        let base = load_metrics(&fixture("panic.base.json"))?;
        let head = load_metrics(&fixture("panic.head.json"))?;
        let comparison = compare_metrics(&base, &head);
        assert_eq!(comparison.verdict, "fail");
        Ok(())
    }

    #[test]
    fn system_perl_unchanged_existing_failure_passes() -> Result<()> {
        let base = load_metrics(&fixture("system-unchanged.base.json"))?;
        let head = load_metrics(&fixture("system-unchanged.head.json"))?;
        let comparison = compare_metrics(&base, &head);
        assert_eq!(comparison.verdict, "pass");
        Ok(())
    }

    #[test]
    fn system_perl_worsened_failure_fails() -> Result<()> {
        let base = load_metrics(&fixture("system-worse.base.json"))?;
        let head = load_metrics(&fixture("system-worse.head.json"))?;
        let comparison = compare_metrics(&base, &head);
        assert_eq!(comparison.verdict, "fail");
        Ok(())
    }

    #[test]
    fn runtime_only_regression_warns_and_passes() -> Result<()> {
        let base = load_metrics(&fixture("runtime.base.json"))?;
        let head = load_metrics(&fixture("runtime.head.json"))?;
        let comparison = compare_metrics(&base, &head);
        assert_eq!(comparison.verdict, "pass");
        assert_eq!(comparison.violations.iter().filter(|v| v.severity == "warn").count(), 1);
        Ok(())
    }
}
