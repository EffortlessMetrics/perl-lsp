use color_eyre::eyre::{Result, bail};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const DEFAULT_EPSILON: f64 = 0.000_5;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParserRatchetMetrics {
    pub selected: String,
    pub clean_parse_rate: f64,
    pub panic_count: u64,
    pub timeout_count: u64,
    pub error_node_count: u64,
    #[serde(default)]
    pub node_kind_seen_count: Option<u64>,
    #[serde(default)]
    pub concept_floors_pass: Option<bool>,
    #[serde(default)]
    pub corpus_runtime_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ViolationSeverity {
    Error,
    Warn,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RatchetViolation {
    pub metric: String,
    pub message: String,
    pub severity: ViolationSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompareConfig {
    #[serde(default = "default_epsilon")]
    pub clean_parse_rate_epsilon: f64,
    #[serde(default)]
    pub error_node_material_increase: u64,
    #[serde(default)]
    pub node_kind_unexpected_drop: u64,
    #[serde(default)]
    pub runtime_regression_warn_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ParserRatchetVerdict {
    Pass,
    Fail,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompareOutcome {
    pub violations: Vec<RatchetViolation>,
    pub ratchet_opportunity: bool,
    pub verdict: ParserRatchetVerdict,
}

impl Default for CompareConfig {
    fn default() -> Self {
        Self {
            clean_parse_rate_epsilon: default_epsilon(),
            error_node_material_increase: 0,
            node_kind_unexpected_drop: 0,
            runtime_regression_warn_ms: 10_000,
        }
    }
}

fn default_epsilon() -> f64 {
    DEFAULT_EPSILON
}

fn is_regression(base: f64, head: f64, epsilon: f64) -> bool {
    (base - head) > epsilon
}

pub fn compare_metrics(
    selected: &str,
    base: &ParserRatchetMetrics,
    head: &ParserRatchetMetrics,
    config: &CompareConfig,
) -> CompareOutcome {
    let mut violations = Vec::new();

    match selected {
        "perl-corpus" => {
            if head.panic_count > 0 {
                violations.push(RatchetViolation {
                    metric: "panic_count".to_string(),
                    message: format!(
                        "perl-corpus requires panic_count=0, found {}",
                        head.panic_count
                    ),
                    severity: ViolationSeverity::Error,
                });
            }
            if head.timeout_count > 0 {
                violations.push(RatchetViolation {
                    metric: "timeout_count".to_string(),
                    message: format!(
                        "perl-corpus requires timeout_count=0, found {}",
                        head.timeout_count
                    ),
                    severity: ViolationSeverity::Error,
                });
            }
            if matches!(head.concept_floors_pass, Some(false)) {
                violations.push(RatchetViolation {
                    metric: "concept_floors_pass".to_string(),
                    message: "concept floors must pass for perl-corpus".to_string(),
                    severity: ViolationSeverity::Error,
                });
            }
            if is_regression(
                base.clean_parse_rate,
                head.clean_parse_rate,
                config.clean_parse_rate_epsilon,
            ) {
                violations.push(RatchetViolation {
                    metric: "clean_parse_rate".to_string(),
                    message: format!(
                        "clean_parse_rate regressed from {:.6} to {:.6} beyond epsilon {}",
                        base.clean_parse_rate,
                        head.clean_parse_rate,
                        config.clean_parse_rate_epsilon
                    ),
                    severity: ViolationSeverity::Error,
                });
            }
            if head.error_node_count
                > base.error_node_count.saturating_add(config.error_node_material_increase)
            {
                violations.push(RatchetViolation {
                    metric: "error_node_count".to_string(),
                    message: format!(
                        "error_node_count materially increased from {} to {}",
                        base.error_node_count, head.error_node_count
                    ),
                    severity: ViolationSeverity::Error,
                });
            }

            if let (Some(base_count), Some(head_count)) =
                (base.node_kind_seen_count, head.node_kind_seen_count)
                && base_count > head_count
                && base_count - head_count > config.node_kind_unexpected_drop
            {
                violations.push(RatchetViolation {
                    metric: "node_kind_seen_count".to_string(),
                    message: format!(
                        "node_kind_seen_count dropped unexpectedly from {} to {}",
                        base_count, head_count
                    ),
                    severity: ViolationSeverity::Error,
                });
            }
        }
        "system-perl" => {
            if head.panic_count > base.panic_count {
                violations.push(RatchetViolation {
                    metric: "panic_count".to_string(),
                    message: format!(
                        "system-perl panic_count worsened from {} to {}",
                        base.panic_count, head.panic_count
                    ),
                    severity: ViolationSeverity::Error,
                });
            }
            if head.timeout_count > base.timeout_count {
                violations.push(RatchetViolation {
                    metric: "timeout_count".to_string(),
                    message: format!(
                        "system-perl timeout_count worsened from {} to {}",
                        base.timeout_count, head.timeout_count
                    ),
                    severity: ViolationSeverity::Error,
                });
            }
            if is_regression(
                base.clean_parse_rate,
                head.clean_parse_rate,
                config.clean_parse_rate_epsilon,
            ) {
                violations.push(RatchetViolation {
                    metric: "clean_parse_rate".to_string(),
                    message: format!(
                        "system-perl clean_parse_rate regressed from {:.6} to {:.6} beyond epsilon {}",
                        base.clean_parse_rate, head.clean_parse_rate, config.clean_parse_rate_epsilon
                    ),
                    severity: ViolationSeverity::Error,
                });
            }
        }
        _ => {}
    }

    if let (Some(base_runtime), Some(head_runtime)) =
        (base.corpus_runtime_ms, head.corpus_runtime_ms)
        && head_runtime > base_runtime.saturating_add(config.runtime_regression_warn_ms)
    {
        violations.push(RatchetViolation {
            metric: "corpus_runtime_ms".to_string(),
            message: format!(
                "runtime regression observed ({} -> {} ms); advisory only",
                base_runtime, head_runtime
            ),
            severity: ViolationSeverity::Warn,
        });
    }

    let ratchet_opportunity = head.clean_parse_rate > base.clean_parse_rate
        || head.error_node_count < base.error_node_count
        || head.panic_count < base.panic_count
        || head.timeout_count < base.timeout_count;

    let verdict = if violations.iter().any(|v| v.severity == ViolationSeverity::Error) {
        ParserRatchetVerdict::Fail
    } else {
        ParserRatchetVerdict::Pass
    };

    CompareOutcome { violations, ratchet_opportunity, verdict }
}

pub fn run_compare(base_metrics: &Path, head_metrics: &Path, receipt: &Path) -> Result<()> {
    let base: ParserRatchetMetrics = serde_json::from_str(&fs::read_to_string(base_metrics)?)?;
    let head: ParserRatchetMetrics = serde_json::from_str(&fs::read_to_string(head_metrics)?)?;
    if base.selected != head.selected {
        bail!("selected profile mismatch: base={} head={}", base.selected, head.selected);
    }

    let outcome = compare_metrics(&head.selected, &base, &head, &CompareConfig::default());
    if let Some(parent) = receipt.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(receipt, serde_json::to_string_pretty(&outcome)?)?;

    if matches!(outcome.verdict, ParserRatchetVerdict::Fail) {
        bail!("parser-ratchet compare failed");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metrics(selected: &str) -> ParserRatchetMetrics {
        ParserRatchetMetrics {
            selected: selected.to_string(),
            clean_parse_rate: 0.9,
            panic_count: 0,
            timeout_count: 0,
            error_node_count: 100,
            node_kind_seen_count: Some(400),
            concept_floors_pass: Some(true),
            corpus_runtime_ms: Some(1000),
        }
    }

    #[test]
    fn equal_metrics_pass() {
        let base = metrics("perl-corpus");
        let head = metrics("perl-corpus");
        let outcome = compare_metrics("perl-corpus", &base, &head, &CompareConfig::default());
        assert!(matches!(outcome.verdict, ParserRatchetVerdict::Pass));
    }

    #[test]
    fn improvement_sets_opportunity() {
        let base = metrics("perl-corpus");
        let mut head = metrics("perl-corpus");
        head.clean_parse_rate = 0.95;
        head.error_node_count = 90;
        let outcome = compare_metrics("perl-corpus", &base, &head, &CompareConfig::default());
        assert!(outcome.ratchet_opportunity);
        assert!(matches!(outcome.verdict, ParserRatchetVerdict::Pass));
    }

    #[test]
    fn perl_corpus_panic_fails() {
        let base = metrics("perl-corpus");
        let mut head = metrics("perl-corpus");
        head.panic_count = 1;
        let outcome = compare_metrics("perl-corpus", &base, &head, &CompareConfig::default());
        assert!(matches!(outcome.verdict, ParserRatchetVerdict::Fail));
    }

    #[test]
    fn system_perl_unchanged_existing_failures_pass() {
        let mut base = metrics("system-perl");
        base.timeout_count = 4;
        let mut head = base.clone();
        head.selected = "system-perl".to_string();
        let outcome = compare_metrics("system-perl", &base, &head, &CompareConfig::default());
        assert!(matches!(outcome.verdict, ParserRatchetVerdict::Pass));
    }

    #[test]
    fn system_perl_worsened_failure_fails() {
        let mut base = metrics("system-perl");
        base.panic_count = 1;
        let mut head = base.clone();
        head.selected = "system-perl".to_string();
        head.panic_count = 2;
        let outcome = compare_metrics("system-perl", &base, &head, &CompareConfig::default());
        assert!(matches!(outcome.verdict, ParserRatchetVerdict::Fail));
    }

    #[test]
    fn runtime_only_regression_warns_and_passes() {
        let base = metrics("perl-corpus");
        let mut head = metrics("perl-corpus");
        head.corpus_runtime_ms = Some(25_000);
        let outcome = compare_metrics("perl-corpus", &base, &head, &CompareConfig::default());
        assert!(matches!(outcome.verdict, ParserRatchetVerdict::Pass));
        assert!(
            outcome
                .violations
                .iter()
                .any(|v| v.metric == "corpus_runtime_ms" && v.severity == ViolationSeverity::Warn)
        );
    }
}
