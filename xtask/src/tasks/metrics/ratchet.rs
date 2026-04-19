//! Scorecard floor-metric ratchet checker.
//!
//! Loads a committed baseline JSON (`.ci/metrics/baselines/<subsystem>.json`)
//! and checks a set of current metric values against it.  Violations are
//! returned as a `Vec<RatchetViolation>`; an empty vec means all checks
//! passed.
//!
//! # Naming convention for direction
//!
//! Metrics whose names end with `_count`, `_nodes`, or `_unreadable` are
//! treated as **lower-is-better** (a higher current value is a regression).
//! All other metrics are **higher-is-better** (a lower current value is a
//! regression).
//!
//! If you add a lower-is-better metric that does not fit these suffixes (e.g.
//! `latency_p95_ms`), add its name to the `lower_is_better` field on
//! `SubsystemBaseline` rather than extending the suffix list.

use chrono::Utc;
use color_eyre::eyre::{Context, Result, eyre};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Schema version this module writes / expects.
pub const SCHEMA_VERSION: u32 = 1;

/// Committed floor-metric baseline for a single subsystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsystemBaseline {
    pub schema_version: u32,
    /// ISO-8601 timestamp of the measurement.
    pub measured_at: String,
    /// Subsystem identifier (e.g. `"parser"`).
    pub subsystem: String,
    /// Git SHA of the commit this baseline was measured on.
    pub commit: String,
    /// Hard-floor metrics.  `None` means not yet instrumented — skipped.
    pub floor_metrics: BTreeMap<String, Option<f64>>,
    /// Aspirational improvement metrics.  `None` means not yet instrumented.
    /// These are informational only; violations here never block.
    pub improvement_metrics: BTreeMap<String, Option<f64>>,
    /// Fractional noise band.  A regression must exceed this fraction of the
    /// baseline value to count as a violation.  Default 0.005 (0.5 %).
    #[serde(default = "default_tolerance")]
    pub tolerance_pct: f64,
    /// Extra metric names that are lower-is-better regardless of their suffix.
    #[serde(default)]
    pub lower_is_better: Vec<String>,
}

fn default_tolerance() -> f64 {
    0.005
}

/// A single floor-metric regression.
#[derive(Debug, Clone)]
pub struct RatchetViolation {
    pub metric: String,
    pub baseline_value: f64,
    pub current_value: f64,
    /// Fractional regression magnitude (0.0 … 1.0+).
    pub regression_pct: f64,
}

/// Load a `SubsystemBaseline` from the committed baselines directory.
///
/// `repo_root` is the workspace root; the file is expected at
/// `.ci/metrics/baselines/<subsystem>.json`.
pub fn load_baseline(repo_root: &Path, subsystem: &str) -> Result<SubsystemBaseline> {
    let path = repo_root
        .join(".ci/metrics/baselines")
        .join(format!("{subsystem}.json"));
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read baseline: {}", path.display()))?;
    let baseline: SubsystemBaseline = serde_json::from_str(&raw)
        .with_context(|| format!("Failed to parse baseline: {}", path.display()))?;
    if baseline.schema_version != SCHEMA_VERSION {
        return Err(eyre!(
            "Baseline schema version mismatch: expected {SCHEMA_VERSION}, got {}",
            baseline.schema_version
        ));
    }
    Ok(baseline)
}

/// Check `current` floor-metric values against `baseline`.
///
/// Returns the list of violations (empty ⇒ all checks passed).
/// `null` (absent) values in either map are skipped silently.
pub fn check_floor_metrics(
    baseline: &SubsystemBaseline,
    current: &BTreeMap<String, Option<f64>>,
) -> Vec<RatchetViolation> {
    let mut violations = Vec::new();

    for (metric, baseline_val) in &baseline.floor_metrics {
        // Skip unset baseline entries.
        let Some(bv) = baseline_val else {
            continue;
        };

        // Skip if current measurement is absent.
        let Some(cv) = current.get(metric).and_then(|v| *v) else {
            continue;
        };

        let is_lower_better = is_lower_better_metric(metric, &baseline.lower_is_better);

        let regression_pct = if is_lower_better {
            // Higher current value = regression.
            // Use bv.max(f64::EPSILON) so that sub-1.0 baselines (e.g. an
            // error_rate of 0.05) produce a correct fractional regression rather
            // than being attenuated by a hard floor of 1.0.
            if cv > *bv {
                (cv - bv) / bv.max(f64::EPSILON)
            } else {
                0.0
            }
        } else {
            // Lower current value = regression.
            if cv < *bv {
                (bv - cv) / bv.max(f64::EPSILON)
            } else {
                0.0
            }
        };

        if regression_pct > baseline.tolerance_pct {
            violations.push(RatchetViolation {
                metric: metric.clone(),
                baseline_value: *bv,
                current_value: cv,
                regression_pct,
            });
        }
    }

    violations
}

/// Determine whether a metric is lower-is-better.
///
/// Returns `true` if the metric name ends with `_count`, `_nodes`, or
/// `_unreadable`, or if it is present in the explicit `lower_is_better` list.
fn is_lower_better_metric(metric: &str, explicit: &[String]) -> bool {
    if explicit.iter().any(|s| s == metric) {
        return true;
    }
    metric.ends_with("_count") || metric.ends_with("_nodes") || metric.ends_with("_unreadable")
}

// =============================================================================
// Receipt format (runtime, not committed)
// =============================================================================

/// Runtime metric receipt produced by a subsystem sweep.
///
/// Written to `target/receipts/metrics/<subsystem>.json`.
/// The #4063 builder emits this format from `parser-stats --json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricReceipt {
    pub subsystem: String,
    pub generated_at: String,
    pub commit: String,
    pub floor_metrics: BTreeMap<String, Option<f64>>,
    pub improvement_metrics: BTreeMap<String, Option<f64>>,
}

// =============================================================================
// CLI handlers
// =============================================================================

/// Run `cargo xtask metrics ratchet-check`.
///
/// - Loads `.ci/metrics/baselines/<subsystem>.json`
/// - Loads current metrics from `--current` path or `target/receipts/metrics/<subsystem>.json`
/// - Reports violations; exits nonzero if any floor metric is breached
/// - Optionally records this run in the stable-wins state file
pub fn run_ratchet_check(
    repo_root: &Path,
    subsystem: &str,
    current_path: Option<PathBuf>,
    record: bool,
) -> Result<()> {
    let baseline = load_baseline(repo_root, subsystem)?;

    // Resolve the current-metrics file.
    let receipt_path = current_path.unwrap_or_else(|| {
        repo_root
            .join("target/receipts/metrics")
            .join(format!("{subsystem}.json"))
    });

    // Load current metrics from receipt file, falling back to the baseline
    // values themselves when no receipt exists yet (bootstrapping).
    let current_metrics: BTreeMap<String, Option<f64>> = if receipt_path.exists() {
        let raw = std::fs::read_to_string(&receipt_path)
            .with_context(|| format!("Failed to read receipt: {}", receipt_path.display()))?;
        let receipt: MetricReceipt = serde_json::from_str(&raw)
            .with_context(|| format!("Failed to parse receipt: {}", receipt_path.display()))?;
        receipt.floor_metrics
    } else {
        // No receipt yet — fall back to baseline values (idempotent, always
        // passes, confirms infrastructure is wired).
        eprintln!(
            "note: no receipt at {} — using baseline values as current (bootstrap mode)",
            receipt_path.display()
        );
        baseline.floor_metrics.clone()
    };

    let violations = check_floor_metrics(&baseline, &current_metrics);

    if violations.is_empty() {
        println!("ratchet-check [{subsystem}]: all floor metrics passed");
    } else {
        for v in &violations {
            eprintln!(
                "VIOLATION [{subsystem}] {}: baseline={:.4} current={:.4} regression={:.2}%",
                v.metric,
                v.baseline_value,
                v.current_value,
                v.regression_pct * 100.0
            );
        }
        return Err(eyre!(
            "{} floor metric violation(s) for subsystem '{subsystem}'",
            violations.len()
        ));
    }

    // Informational: summarize improvement metrics.
    let instrumented: Vec<_> = baseline
        .improvement_metrics
        .iter()
        .filter_map(|(k, v)| v.map(|val| (k, val)))
        .collect();
    if !instrumented.is_empty() {
        println!("  improvement metrics (informational):");
        for (k, v) in &instrumented {
            println!("    {k}: {v:.4}");
        }
    }

    // Record this run in stable-wins state if requested.
    if record {
        use crate::tasks::metrics::stable_wins::{StableWinsState, record_run};

        let state_dir = repo_root.join("target/metrics/stable_wins");
        std::fs::create_dir_all(&state_dir)
            .with_context(|| format!("Failed to create {}", state_dir.display()))?;
        let state_path = state_dir.join(format!("{subsystem}.json"));

        let mut state: StableWinsState = if state_path.exists() {
            let raw = std::fs::read_to_string(&state_path).with_context(|| {
                format!("Failed to read stable-wins state: {}", state_path.display())
            })?;
            serde_json::from_str(&raw).with_context(|| {
                format!(
                    "Failed to parse stable-wins state: {}",
                    state_path.display()
                )
            })?
        } else {
            StableWinsState {
                subsystem: subsystem.to_string(),
                recent_runs: BTreeMap::new(),
            }
        };

        let commit = std::env::var("GITHUB_SHA").unwrap_or_else(|_| "unknown".to_string());
        let timestamp = Utc::now().to_rfc3339();
        record_run(&mut state, &commit, &timestamp, &current_metrics);

        let json = serde_json::to_string_pretty(&state)
            .context("Failed to serialize stable-wins state")?;
        std::fs::write(&state_path, json).with_context(|| {
            format!(
                "Failed to write stable-wins state: {}",
                state_path.display()
            )
        })?;

        println!("  stable-wins state updated: {}", state_path.display());
    }

    Ok(())
}

/// Run `cargo xtask metrics promote-baseline`.
///
/// Reads the stable-wins state and reports which improvement metrics have
/// been consistently above the floor for at least `STABLE_WIN_THRESHOLD`
/// consecutive runs by at least `delta_pct`.
pub fn run_promote_baseline(repo_root: &Path, subsystem: &str, delta_pct: f64) -> Result<()> {
    use crate::tasks::metrics::stable_wins::{
        STABLE_WIN_THRESHOLD, StableWinsState, stable_improvements,
    };

    let baseline = load_baseline(repo_root, subsystem)?;

    let state_path = repo_root
        .join("target/metrics/stable_wins")
        .join(format!("{subsystem}.json"));

    if !state_path.exists() {
        println!("No stable-wins state found for '{subsystem}'. Run ratchet-check --record first.");
        return Ok(());
    }

    let raw = std::fs::read_to_string(&state_path)
        .with_context(|| format!("Failed to read stable-wins state: {}", state_path.display()))?;
    let state: StableWinsState = serde_json::from_str(&raw).with_context(|| {
        format!(
            "Failed to parse stable-wins state: {}",
            state_path.display()
        )
    })?;

    let eligible = stable_improvements(&state, &baseline.improvement_metrics, delta_pct);

    if eligible.is_empty() {
        println!(
            "No improvement metrics for '{subsystem}' are stable across {STABLE_WIN_THRESHOLD} runs at +{:.1}%.",
            delta_pct * 100.0
        );
    } else {
        println!(
            "Eligible for baseline promotion (stable {STABLE_WIN_THRESHOLD} runs, +{:.1}%):",
            delta_pct * 100.0
        );
        for name in &eligible {
            println!("  {name}");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_baseline(floor: BTreeMap<String, Option<f64>>) -> SubsystemBaseline {
        SubsystemBaseline {
            schema_version: SCHEMA_VERSION,
            measured_at: "2026-01-01T00:00:00Z".to_string(),
            subsystem: "test".to_string(),
            commit: "deadbeef".to_string(),
            floor_metrics: floor,
            improvement_metrics: BTreeMap::new(),
            tolerance_pct: 0.005,
            lower_is_better: Vec::new(),
        }
    }

    // -------------------------------------------------------------------------
    // No violation when current equals baseline
    // -------------------------------------------------------------------------
    #[test]
    fn test_ratchet_no_violation_on_exact_match() {
        let baseline = make_baseline(BTreeMap::from([
            ("system_clean_rate".to_string(), Some(0.971)),
            ("system_crash_count".to_string(), Some(0.0)),
        ]));

        let current = BTreeMap::from([
            ("system_clean_rate".to_string(), Some(0.971)),
            ("system_crash_count".to_string(), Some(0.0)),
        ]);

        let violations = check_floor_metrics(&baseline, &current);
        assert!(
            violations.is_empty(),
            "expected no violations, got: {violations:?}"
        );
    }

    // -------------------------------------------------------------------------
    // Violation fires when higher-is-better rate drops below floor
    // -------------------------------------------------------------------------
    #[test]
    fn test_ratchet_violation_on_rate_regression() {
        let baseline = make_baseline(BTreeMap::from([(
            "system_clean_rate".to_string(),
            Some(0.971),
        )]));

        // 0.90 is well below 0.971 — must trigger a violation.
        let current = BTreeMap::from([("system_clean_rate".to_string(), Some(0.90))]);

        let violations = check_floor_metrics(&baseline, &current);
        assert_eq!(violations.len(), 1, "expected exactly one violation");
        assert_eq!(violations[0].metric, "system_clean_rate");
        assert!(
            violations[0].regression_pct > 0.005,
            "regression_pct should exceed tolerance"
        );
    }

    // -------------------------------------------------------------------------
    // No violation within tolerance band
    // -------------------------------------------------------------------------
    #[test]
    fn test_ratchet_no_violation_within_tolerance() {
        let mut baseline = make_baseline(BTreeMap::from([(
            "system_clean_rate".to_string(),
            Some(0.971),
        )]));
        baseline.tolerance_pct = 0.005;

        // 0.3 % drop — within the 0.5 % tolerance band.
        // 0.971 * (1 - 0.003) ≈ 0.96808
        let current = BTreeMap::from([("system_clean_rate".to_string(), Some(0.96808))]);

        let violations = check_floor_metrics(&baseline, &current);
        assert!(
            violations.is_empty(),
            "0.3% drop within 0.5% band should not be a violation; got {violations:?}"
        );
    }

    // -------------------------------------------------------------------------
    // Violation fires when lower-is-better count increases (_count suffix)
    // -------------------------------------------------------------------------
    #[test]
    fn test_ratchet_violation_on_count_increase() {
        let baseline = make_baseline(BTreeMap::from([(
            "system_crash_count".to_string(),
            Some(0.0),
        )]));

        let current = BTreeMap::from([("system_crash_count".to_string(), Some(1.0))]);

        let violations = check_floor_metrics(&baseline, &current);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].metric, "system_crash_count");
    }

    // -------------------------------------------------------------------------
    // Violation fires when lower-is-better _nodes metric increases
    // -------------------------------------------------------------------------
    #[test]
    fn test_ratchet_violation_on_nodes_increase() {
        let baseline = make_baseline(BTreeMap::from([(
            "system_total_error_nodes".to_string(),
            Some(604.0),
        )]));

        let current = BTreeMap::from([("system_total_error_nodes".to_string(), Some(700.0))]);

        let violations = check_floor_metrics(&baseline, &current);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].metric, "system_total_error_nodes");
    }

    // -------------------------------------------------------------------------
    // Null baseline metrics are silently skipped
    // -------------------------------------------------------------------------
    #[test]
    fn test_ratchet_null_baseline_skipped() {
        let baseline = make_baseline(BTreeMap::from([
            ("system_clean_rate".to_string(), Some(0.971)),
            ("node_kind_coverage".to_string(), None),
        ]));

        // Provide a terrible value for the null metric — must be ignored.
        let current = BTreeMap::from([
            ("system_clean_rate".to_string(), Some(0.972)),
            ("node_kind_coverage".to_string(), Some(0.0)),
        ]);

        let violations = check_floor_metrics(&baseline, &current);
        assert!(
            violations.is_empty(),
            "null baseline metric must be skipped; got {violations:?}"
        );
    }

    // -------------------------------------------------------------------------
    // Missing current measurement is silently skipped
    // -------------------------------------------------------------------------
    #[test]
    fn test_ratchet_missing_current_skipped() {
        let baseline = make_baseline(BTreeMap::from([(
            "system_clean_rate".to_string(),
            Some(0.971),
        )]));

        // Current map is empty — metric is absent.
        let current: BTreeMap<String, Option<f64>> = BTreeMap::new();

        let violations = check_floor_metrics(&baseline, &current);
        assert!(
            violations.is_empty(),
            "missing current metric must be skipped; got {violations:?}"
        );
    }

    // -------------------------------------------------------------------------
    // Explicit lower_is_better list overrides suffix convention
    // -------------------------------------------------------------------------
    #[test]
    fn test_ratchet_explicit_lower_is_better() {
        let mut baseline = make_baseline(BTreeMap::from([(
            "latency_p95_ms".to_string(),
            Some(100.0),
        )]));
        baseline.lower_is_better = vec!["latency_p95_ms".to_string()];

        // Increase in latency is a regression.
        let current = BTreeMap::from([("latency_p95_ms".to_string(), Some(120.0))]);

        let violations = check_floor_metrics(&baseline, &current);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].metric, "latency_p95_ms");
    }

    // -------------------------------------------------------------------------
    // No violation when lower-is-better metric actually decreases (improves)
    // -------------------------------------------------------------------------
    #[test]
    fn test_ratchet_no_violation_on_count_decrease() {
        let baseline = make_baseline(BTreeMap::from([(
            "system_crash_count".to_string(),
            Some(5.0),
        )]));

        let current = BTreeMap::from([("system_crash_count".to_string(), Some(3.0))]);

        let violations = check_floor_metrics(&baseline, &current);
        assert!(
            violations.is_empty(),
            "count decrease is an improvement, not a violation"
        );
    }

    // -------------------------------------------------------------------------
    // Lower-is-better with sub-1.0 baseline: doubling must be a violation.
    // Previously bv.max(1.0) attenuated this to ~0.1 % — below any tolerance.
    // After fix to bv.max(f64::EPSILON), it registers as 100 % regression.
    // -------------------------------------------------------------------------
    #[test]
    fn test_ratchet_lower_is_better_sub_one_baseline_regression() {
        let mut baseline = make_baseline(BTreeMap::from([(
            "error_rate_count".to_string(),
            Some(0.05),
        )]));
        baseline.tolerance_pct = 0.005; // 0.5 % band

        // Doubling 0.05 -> 0.10 is a 100 % regression; must exceed tolerance.
        let current = BTreeMap::from([("error_rate_count".to_string(), Some(0.10))]);

        let violations = check_floor_metrics(&baseline, &current);
        assert_eq!(
            violations.len(),
            1,
            "doubling a sub-1.0 lower-is-better metric must violate"
        );
        assert_eq!(violations[0].metric, "error_rate_count");
        // regression_pct = (0.10 - 0.05) / 0.05 = 1.0 (100 %)
        assert!(
            violations[0].regression_pct > 0.5,
            "regression_pct should be ~1.0 (100%), got {}",
            violations[0].regression_pct
        );
    }

    // -------------------------------------------------------------------------
    // Schema version mismatch must return an error.
    // load_baseline is tested here via a temp file since it reads from disk.
    // -------------------------------------------------------------------------
    #[test]
    fn test_load_baseline_schema_version_mismatch() {
        use std::io::Write;
        let dir = tempfile::tempdir().expect("failed to create tempdir");
        let baselines_dir = dir.path().join(".ci/metrics/baselines");
        std::fs::create_dir_all(&baselines_dir).unwrap();
        let path = baselines_dir.join("test.json");
        let mut f = std::fs::File::create(&path).unwrap();
        write!(
            f,
            r#"{{"schema_version":99,"measured_at":"2026-01-01T00:00:00Z","subsystem":"test","commit":"abc","floor_metrics":{{}},"improvement_metrics":{{}}}}"#
        )
        .unwrap();

        let result = load_baseline(dir.path(), "test");
        assert!(result.is_err(), "schema version mismatch must return Err");
        let msg = format!("{}", result.unwrap_err());
        assert!(
            msg.contains("schema version mismatch"),
            "error message must mention schema version mismatch; got: {msg}"
        );
    }
}
