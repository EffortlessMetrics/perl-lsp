//! Multi-run stability tracker for scorecard improvement metrics.
//!
//! Before raising the committed floor baseline, we want N consecutive CI runs
//! to all show improvement.  This module persists that per-subsystem history
//! in `target/metrics/stable_wins/<subsystem>.json` (gitignored ephemeral
//! CI state).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Minimum consecutive runs required before an improvement is considered stable.
pub const STABLE_WIN_THRESHOLD: usize = 3;

/// Persisted state for the stable-wins tracker.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StableWinsState {
    pub subsystem: String,
    /// Recent runs keyed by metric name.
    pub recent_runs: BTreeMap<String, Vec<MetricRun>>,
}

/// A single data point for one metric on one CI run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricRun {
    pub commit: String,
    pub value: f64,
    pub timestamp: String,
}

/// Append the current run's metric values to `state`.
///
/// `null` values are skipped.  The window is capped at
/// `STABLE_WIN_THRESHOLD + 1` entries per metric.
pub fn record_run(
    state: &mut StableWinsState,
    commit: &str,
    timestamp: &str,
    metrics: &BTreeMap<String, Option<f64>>,
) {
    for (name, val) in metrics {
        let Some(v) = val else {
            continue;
        };
        let runs = state.recent_runs.entry(name.clone()).or_default();
        runs.push(MetricRun {
            commit: commit.to_string(),
            value: *v,
            timestamp: timestamp.to_string(),
        });
        // Keep one extra entry beyond threshold for context.
        let cap = STABLE_WIN_THRESHOLD + 1;
        if runs.len() > cap {
            runs.drain(0..runs.len() - cap);
        }
    }
}

/// Return the names of improvement metrics that have been stable across at
/// least `STABLE_WIN_THRESHOLD` consecutive runs AND whose recent values all
/// exceed `baseline_value * (1 + material_delta_pct)`.
///
/// Only higher-is-better metrics are considered here; pass a pre-filtered
/// `baseline` map if you need lower-is-better support.
pub fn stable_improvements(
    state: &StableWinsState,
    baseline: &BTreeMap<String, Option<f64>>,
    material_delta_pct: f64,
) -> Vec<String> {
    let mut result = Vec::new();

    for (name, runs) in &state.recent_runs {
        if runs.len() < STABLE_WIN_THRESHOLD {
            continue;
        }
        let Some(Some(bv)) = baseline.get(name) else {
            continue;
        };
        let threshold = bv * (1.0 + material_delta_pct);
        let last_n = &runs[runs.len() - STABLE_WIN_THRESHOLD..];
        if last_n.iter().all(|r| r.value >= threshold) {
            result.push(name.clone());
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_run_appends_values() {
        let mut state =
            StableWinsState { subsystem: "parser".to_string(), recent_runs: BTreeMap::new() };
        let metrics = BTreeMap::from([("system_clean_rate".to_string(), Some(0.972_f64))]);
        record_run(&mut state, "abc123", "2026-01-01T00:00:00Z", &metrics);

        assert_eq!(state.recent_runs["system_clean_rate"].len(), 1);
        assert!((state.recent_runs["system_clean_rate"][0].value - 0.972).abs() < f64::EPSILON);
    }

    #[test]
    fn test_record_run_caps_window() {
        let mut state =
            StableWinsState { subsystem: "parser".to_string(), recent_runs: BTreeMap::new() };
        let metrics = BTreeMap::from([("system_clean_rate".to_string(), Some(0.972_f64))]);

        // Insert more entries than the cap.
        for i in 0..(STABLE_WIN_THRESHOLD + 5) {
            record_run(&mut state, &format!("commit{i}"), "2026-01-01T00:00:00Z", &metrics);
        }

        let runs = &state.recent_runs["system_clean_rate"];
        assert!(
            runs.len() <= STABLE_WIN_THRESHOLD + 1,
            "window should be capped at THRESHOLD+1, got {}",
            runs.len()
        );
    }

    #[test]
    fn test_stable_improvements_requires_threshold_runs() {
        let mut state =
            StableWinsState { subsystem: "parser".to_string(), recent_runs: BTreeMap::new() };
        let baseline = BTreeMap::from([("system_clean_rate".to_string(), Some(0.971_f64))]);

        // Only 2 runs — not enough.
        let metrics = BTreeMap::from([("system_clean_rate".to_string(), Some(0.985_f64))]);
        record_run(&mut state, "c1", "2026-01-01T00:00:00Z", &metrics);
        record_run(&mut state, "c2", "2026-01-01T00:00:00Z", &metrics);

        let stable = stable_improvements(&state, &baseline, 0.01);
        assert!(stable.is_empty(), "need {STABLE_WIN_THRESHOLD} runs, only have 2");
    }

    #[test]
    fn test_stable_improvements_detected_after_threshold() {
        let mut state =
            StableWinsState { subsystem: "parser".to_string(), recent_runs: BTreeMap::new() };
        let baseline = BTreeMap::from([("system_clean_rate".to_string(), Some(0.971_f64))]);

        // 3 runs all above baseline + 1% delta.
        let metrics = BTreeMap::from([("system_clean_rate".to_string(), Some(0.985_f64))]);
        for i in 0..STABLE_WIN_THRESHOLD {
            record_run(&mut state, &format!("c{i}"), "2026-01-01T00:00:00Z", &metrics);
        }

        let stable = stable_improvements(&state, &baseline, 0.01);
        assert_eq!(stable, vec!["system_clean_rate".to_string()]);
    }

    #[test]
    fn test_stable_improvements_not_triggered_below_delta() {
        let mut state =
            StableWinsState { subsystem: "parser".to_string(), recent_runs: BTreeMap::new() };
        let baseline = BTreeMap::from([("system_clean_rate".to_string(), Some(0.971_f64))]);

        // 3 runs only 0.5 % above baseline — below the 1 % material_delta_pct.
        let marginal = 0.971 * 1.005; // just 0.5% above
        let metrics = BTreeMap::from([("system_clean_rate".to_string(), Some(marginal))]);
        for i in 0..STABLE_WIN_THRESHOLD {
            record_run(&mut state, &format!("c{i}"), "2026-01-01T00:00:00Z", &metrics);
        }

        let stable = stable_improvements(&state, &baseline, 0.01);
        assert!(stable.is_empty(), "0.5% improvement below 1% delta should not trigger");
    }
}
