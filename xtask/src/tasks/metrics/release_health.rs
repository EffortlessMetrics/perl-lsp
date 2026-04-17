//! Release-health dashboard subcommand.
//!
//! Surfaces a single-pane release-confidence scorecard built from data the
//! repository already produces:
//!
//! * `.ci/debt-ledger.yaml` — flaky tests, known issues, technical-debt items,
//!   and the budgets that scope each.
//! * `target/metrics/ci_baseline.json` — written by `cargo xtask ci-baseline`,
//!   giving merge-gate pass-rate and billable-minutes for a recent window.
//! * `Cargo.toml` workspace version — the alpha release we are tracking
//!   against.
//!
//! ## Usage
//!
//! ```bash
//! # Print the scorecard
//! cargo xtask metrics release-health
//!
//! # Also write `.ci/metrics/release-health.json`
//! cargo xtask metrics release-health --json
//!
//! # Override the history window reported in the receipt (default: 30 days).
//! cargo xtask metrics release-health --days 7
//! ```
//!
//! ## Schema (`.ci/metrics/release-health.json`)
//!
//! ```json
//! {
//!   "schema_version": 1,
//!   "measured_at": "2026-04-15T00:00:00Z",
//!   "subsystem": "release_health",
//!   "metrics": {
//!     "current_release_version": "0.12.4",
//!     "history_window_days": 30,
//!     "flaky_test_count": 0,
//!     "quarantined_test_count": 0,
//!     "known_issues_count": 0,
//!     "technical_debt_count": 4,
//!     "debt_budget_utilization_pct": {
//!       "flaky_tests": 0.0,
//!       "known_issues": 0.0,
//!       "technical_debt": 13.3
//!     },
//!     "merge_gate_pass_rate": null,
//!     "merge_gate_runs_analyzed": null,
//!     "merge_gate_billable_minutes": null
//!   }
//! }
//! ```
//!
//! Any metric whose source data is missing is emitted as JSON `null` rather
//! than fabricated.  This keeps the scorecard honest when running locally
//! without the optional CI-baseline artifact.

use crate::utils::project_root;
use chrono::Utc;
use color_eyre::eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// Output schema (`.ci/metrics/release-health.json`)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct ReleaseHealthOutput {
    schema_version: u32,
    measured_at: String,
    subsystem: &'static str,
    metrics: ReleaseHealthMetrics,
}

#[derive(Debug, Clone, Serialize)]
struct ReleaseHealthMetrics {
    current_release_version: Option<String>,
    history_window_days: u64,

    // Debt ledger counts
    flaky_test_count: usize,
    quarantined_test_count: usize,
    known_issues_count: usize,
    technical_debt_count: usize,

    // Per-bucket budget utilization (percentage of configured cap, 0–100+).
    debt_budget_utilization_pct: BTreeMap<String, Option<f64>>,

    // CI baseline (optional — null when target/metrics/ci_baseline.json absent).
    merge_gate_pass_rate: Option<f64>,
    merge_gate_runs_analyzed: Option<u64>,
    merge_gate_billable_minutes: Option<u64>,
}

// ---------------------------------------------------------------------------
// Debt-ledger schema (subset we need)
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
struct DebtLedger {
    #[serde(default)]
    budgets: DebtBudgets,
    #[serde(default)]
    flaky_tests: Vec<DebtFlakyTest>,
    #[serde(default)]
    known_issues: Vec<serde_yaml_ng::Value>,
    #[serde(default)]
    technical_debt: Vec<serde_yaml_ng::Value>,
}

#[derive(Debug, Default, Deserialize)]
struct DebtBudgets {
    #[serde(default)]
    max_quarantined_tests: Option<u64>,
    #[serde(default)]
    max_known_issues: Option<u64>,
    #[serde(default)]
    max_technical_debt: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct DebtFlakyTest {
    #[serde(default)]
    tier: Option<String>,
}

// ---------------------------------------------------------------------------
// CI baseline schema (subset we need)
// ---------------------------------------------------------------------------
//
// Mirrors the public `BaselineReport` written by
// `xtask::tasks::ci_metrics::run_ci_baseline`.  We deliberately re-declare the
// minimum fields rather than depend on that struct so this module stays a
// pure consumer of an on-disk artifact.

#[derive(Debug, Deserialize)]
struct CiBaselineFile {
    #[serde(default)]
    summary: Option<CiBaselineSummary>,
}

#[derive(Debug, Deserialize)]
struct CiBaselineSummary {
    total_runs: u64,
    total_billable_minutes: u64,
    overall_success_rate_percent: f64,
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Run `cargo xtask metrics release-health`.
pub fn run(days: u64, json: bool) -> Result<()> {
    let root = project_root()?;
    let metrics = collect_release_health(&root, days)?;

    print_table(&metrics);

    if json {
        write_json_output(&root, &metrics)?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Collection
// ---------------------------------------------------------------------------

fn collect_release_health(root: &Path, days: u64) -> Result<ReleaseHealthMetrics> {
    let ledger = read_debt_ledger(root)?;
    let baseline = read_ci_baseline(root);
    let version = read_workspace_version(root);

    let quarantined =
        ledger.flaky_tests.iter().filter(|t| t.tier.as_deref() == Some("quarantine")).count();

    let mut utilization: BTreeMap<String, Option<f64>> = BTreeMap::new();
    utilization.insert(
        "flaky_tests".to_string(),
        budget_utilization(quarantined as u64, ledger.budgets.max_quarantined_tests),
    );
    utilization.insert(
        "known_issues".to_string(),
        budget_utilization(ledger.known_issues.len() as u64, ledger.budgets.max_known_issues),
    );
    utilization.insert(
        "technical_debt".to_string(),
        budget_utilization(ledger.technical_debt.len() as u64, ledger.budgets.max_technical_debt),
    );

    let (pass_rate, runs, minutes) = match baseline.and_then(|b| b.summary) {
        Some(s) => (
            Some(s.overall_success_rate_percent),
            Some(s.total_runs),
            Some(s.total_billable_minutes),
        ),
        None => (None, None, None),
    };

    Ok(ReleaseHealthMetrics {
        current_release_version: version,
        history_window_days: days,
        flaky_test_count: ledger.flaky_tests.len(),
        quarantined_test_count: quarantined,
        known_issues_count: ledger.known_issues.len(),
        technical_debt_count: ledger.technical_debt.len(),
        debt_budget_utilization_pct: utilization,
        merge_gate_pass_rate: pass_rate,
        merge_gate_runs_analyzed: runs,
        merge_gate_billable_minutes: minutes,
    })
}

/// Read the YAML debt ledger.  An absent file is treated as an empty ledger so
/// the scorecard still renders cleanly on a fresh clone.
fn read_debt_ledger(root: &Path) -> Result<DebtLedger> {
    let path = root.join(".ci").join("debt-ledger.yaml");
    let Ok(raw) = fs::read_to_string(&path) else {
        return Ok(DebtLedger::default());
    };
    serde_yaml_ng::from_str::<DebtLedger>(&raw)
        .with_context(|| format!("parsing {}", path.display()))
}

/// Read the optional CI baseline JSON written by `cargo xtask ci-baseline`.
/// Returns `None` if the file is absent or fails to parse — the scorecard
/// degrades gracefully and reports `null` for the merge-gate metrics.
fn read_ci_baseline(root: &Path) -> Option<CiBaselineFile> {
    let path = root.join("target").join("metrics").join("ci_baseline.json");
    let raw = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Extract `[workspace.package].version` from the root `Cargo.toml`.
fn read_workspace_version(root: &Path) -> Option<String> {
    let raw = fs::read_to_string(root.join("Cargo.toml")).ok()?;
    let parsed: toml::Value = toml::from_str(&raw).ok()?;
    parsed.get("workspace")?.get("package")?.get("version")?.as_str().map(str::to_string)
}

/// Return `Some(percent)` of `cap` consumed by `count`, or `None` when the
/// cap is unknown.  `cap == 0` is treated as `None` to avoid divide-by-zero.
fn budget_utilization(count: u64, cap: Option<u64>) -> Option<f64> {
    let cap = cap?;
    if cap == 0 {
        return None;
    }
    Some(round_one_decimal((count as f64) * 100.0 / (cap as f64)))
}

fn round_one_decimal(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

// ---------------------------------------------------------------------------
// Presentation
// ---------------------------------------------------------------------------

fn print_table(m: &ReleaseHealthMetrics) {
    println!("Release Health Scorecard");
    println!("{}", "=".repeat(56));
    println!(
        "  Current release version : {}",
        m.current_release_version.as_deref().unwrap_or("(unknown)")
    );
    println!("  History window (days)   : {}", m.history_window_days);
    println!();

    println!("Debt ledger");
    println!("{}", "-".repeat(56));
    println!("  {:<32} {:>5}   utilization", "Bucket", "Count",);
    print_debt_row(
        "Flaky tests (quarantined)",
        m.quarantined_test_count,
        m.debt_budget_utilization_pct.get("flaky_tests").copied().flatten(),
    );
    print_debt_row(
        "Known issues",
        m.known_issues_count,
        m.debt_budget_utilization_pct.get("known_issues").copied().flatten(),
    );
    print_debt_row(
        "Technical debt items",
        m.technical_debt_count,
        m.debt_budget_utilization_pct.get("technical_debt").copied().flatten(),
    );
    if m.flaky_test_count != m.quarantined_test_count {
        println!(
            "  (note: ledger lists {} flaky entries total; {} are tier=quarantine)",
            m.flaky_test_count, m.quarantined_test_count
        );
    }
    println!();

    println!("Merge-gate signal (last {} days)", m.history_window_days);
    println!("{}", "-".repeat(56));
    match m.merge_gate_pass_rate {
        Some(rate) => {
            let runs = m.merge_gate_runs_analyzed.unwrap_or(0);
            let mins = m.merge_gate_billable_minutes.unwrap_or(0);
            println!("  Pass rate          : {rate:>6.1}%");
            println!("  Runs analyzed      : {runs}");
            println!("  Billable minutes   : {mins}");
        }
        None => {
            println!("  No CI baseline available.");
            println!(
                "  Run `cargo xtask ci-baseline --branch master --days {}` to populate.",
                m.history_window_days
            );
        }
    }
}

fn print_debt_row(label: &str, count: usize, util: Option<f64>) {
    let util_cell = util.map(|p| format!("{p:>5.1}%")).unwrap_or_else(|| "  n/a".to_string());
    println!("  {label:<32} {count:>5}   {util_cell}");
}

fn write_json_output(root: &Path, metrics: &ReleaseHealthMetrics) -> Result<()> {
    let metrics_dir = root.join(".ci").join("metrics");
    fs::create_dir_all(&metrics_dir)
        .with_context(|| format!("creating {}", metrics_dir.display()))?;

    let output = ReleaseHealthOutput {
        schema_version: 1,
        measured_at: Utc::now().to_rfc3339(),
        subsystem: "release_health",
        metrics: metrics.clone(),
    };

    let path = metrics_dir.join("release-health.json");
    let json = serde_json::to_string_pretty(&output).context("serializing release-health JSON")?;
    fs::write(&path, &json).with_context(|| format!("writing {}", path.display()))?;
    println!();
    println!("Wrote {}", path.display());
    Ok(())
}


// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_ledger(root: &Path, contents: &str) -> Result<()> {
        let dir = root.join(".ci");
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("debt-ledger.yaml"), contents)?;
        Ok(())
    }

    fn write_cargo_toml(root: &Path, version: &str) -> Result<()> {
        fs::write(
            root.join("Cargo.toml"),
            format!("[workspace.package]\nversion = \"{version}\"\n"),
        )?;
        Ok(())
    }

    fn write_ci_baseline(root: &Path, summary_json: &str) -> Result<()> {
        let dir = root.join("target").join("metrics");
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("ci_baseline.json"), format!("{{\"summary\": {summary_json}}}"))?;
        Ok(())
    }

    #[test]
    fn budget_utilization_handles_zero_and_missing_caps() {
        assert_eq!(budget_utilization(5, None), None);
        assert_eq!(budget_utilization(5, Some(0)), None);
        assert_eq!(budget_utilization(0, Some(10)), Some(0.0));
        assert_eq!(budget_utilization(5, Some(10)), Some(50.0));
        assert_eq!(budget_utilization(20, Some(10)), Some(200.0));
    }

    #[test]
    fn collect_handles_empty_repo() -> Result<()> {
        let tmp = TempDir::new()?;
        let m = collect_release_health(tmp.path(), 30)?;
        assert_eq!(m.flaky_test_count, 0);
        assert_eq!(m.quarantined_test_count, 0);
        assert_eq!(m.known_issues_count, 0);
        assert_eq!(m.technical_debt_count, 0);
        assert_eq!(m.merge_gate_pass_rate, None);
        assert_eq!(m.merge_gate_runs_analyzed, None);
        assert_eq!(m.merge_gate_billable_minutes, None);
        assert_eq!(m.current_release_version, None);
        assert_eq!(m.history_window_days, 30);
        // Caps unknown → utilization is None for every bucket.
        for k in ["flaky_tests", "known_issues", "technical_debt"] {
            assert_eq!(
                m.debt_budget_utilization_pct.get(k).copied().flatten(),
                None,
                "expected None utilization for {k} when no ledger present"
            );
        }
        Ok(())
    }

    #[test]
    fn collect_counts_debt_ledger_contents() -> Result<()> {
        let tmp = TempDir::new()?;
        write_cargo_toml(tmp.path(), "0.12.4")?;
        write_ledger(
            tmp.path(),
            r#"
schema_version: 1
budgets:
  max_quarantined_tests: 10
  max_known_issues: 20
  max_technical_debt: 30
flaky_tests:
  - name: "lsp::a"
    tier: "quarantine"
  - name: "lsp::b"
    tier: "quarantine"
  - name: "lsp::c"
    tier: "disabled"
known_issues:
  - id: 1
  - id: 2
technical_debt:
  - area: "x"
  - area: "y"
  - area: "z"
"#,
        )?;
        let m = collect_release_health(tmp.path(), 14)?;
        assert_eq!(m.current_release_version.as_deref(), Some("0.12.4"));
        assert_eq!(m.flaky_test_count, 3, "all flaky entries are counted");
        assert_eq!(m.quarantined_test_count, 2, "only tier=quarantine count for budget");
        assert_eq!(m.known_issues_count, 2);
        assert_eq!(m.technical_debt_count, 3);
        // 2 / 10 = 20%, 2 / 20 = 10%, 3 / 30 = 10%
        assert_eq!(m.debt_budget_utilization_pct["flaky_tests"], Some(20.0));
        assert_eq!(m.debt_budget_utilization_pct["known_issues"], Some(10.0));
        assert_eq!(m.debt_budget_utilization_pct["technical_debt"], Some(10.0));
        assert_eq!(m.history_window_days, 14);
        Ok(())
    }

    #[test]
    fn collect_picks_up_ci_baseline_when_present() -> Result<()> {
        let tmp = TempDir::new()?;
        write_ci_baseline(
            tmp.path(),
            r#"{"total_runs": 42, "total_billable_minutes": 137, "overall_success_rate_percent": 95.5}"#,
        )?;
        let m = collect_release_health(tmp.path(), 30)?;
        assert_eq!(m.merge_gate_pass_rate, Some(95.5));
        assert_eq!(m.merge_gate_runs_analyzed, Some(42));
        assert_eq!(m.merge_gate_billable_minutes, Some(137));
        Ok(())
    }

    #[test]
    fn collect_tolerates_malformed_ci_baseline() -> Result<()> {
        let tmp = TempDir::new()?;
        let dir = tmp.path().join("target").join("metrics");
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("ci_baseline.json"), "{ this is not json")?;
        let m = collect_release_health(tmp.path(), 30)?;
        assert_eq!(m.merge_gate_pass_rate, None);
        assert_eq!(m.merge_gate_runs_analyzed, None);
        assert_eq!(m.merge_gate_billable_minutes, None);
        Ok(())
    }

    #[test]
    fn collect_propagates_yaml_parse_failure() -> Result<()> {
        let tmp = TempDir::new()?;
        write_ledger(tmp.path(), "this: is: not: valid: yaml: [")?;
        let err = collect_release_health(tmp.path(), 30).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("debt-ledger.yaml"),
            "error must reference the failing file, got: {msg}"
        );
        Ok(())
    }

    #[test]
    fn write_json_output_emits_required_schema() -> Result<()> {
        let tmp = TempDir::new()?;
        write_cargo_toml(tmp.path(), "0.13.0")?;
        write_ledger(
            tmp.path(),
            r#"
budgets:
  max_quarantined_tests: 10
  max_known_issues: 20
  max_technical_debt: 30
flaky_tests: []
known_issues: []
technical_debt:
  - area: "documentation"
  - area: "dependencies"
"#,
        )?;
        let m = collect_release_health(tmp.path(), 7)?;
        write_json_output(tmp.path(), &m)?;

        let path = tmp.path().join(".ci").join("metrics").join("release-health.json");
        let raw = fs::read_to_string(&path)?;
        let v: serde_json::Value = serde_json::from_str(&raw)?;

        assert_eq!(v["schema_version"], 1);
        assert_eq!(v["subsystem"], "release_health");
        assert!(v["measured_at"].is_string());
        let metrics = &v["metrics"];
        assert_eq!(metrics["current_release_version"], "0.13.0");
        assert_eq!(metrics["history_window_days"], 7);
        assert_eq!(metrics["flaky_test_count"], 0);
        assert_eq!(metrics["quarantined_test_count"], 0);
        assert_eq!(metrics["known_issues_count"], 0);
        assert_eq!(metrics["technical_debt_count"], 2);
        // No CI baseline written → all merge-gate numbers must serialise as null,
        // not be omitted.
        assert!(metrics["merge_gate_pass_rate"].is_null());
        assert!(metrics["merge_gate_runs_analyzed"].is_null());
        assert!(metrics["merge_gate_billable_minutes"].is_null());
        // 0 / 10 = 0%, 0 / 20 = 0%, 2 / 30 ≈ 6.7%.
        assert_eq!(metrics["debt_budget_utilization_pct"]["flaky_tests"], 0.0);
        assert_eq!(metrics["debt_budget_utilization_pct"]["known_issues"], 0.0);
        assert!(
            (metrics["debt_budget_utilization_pct"]["technical_debt"].as_f64().unwrap_or(0.0)
                - 6.7)
                .abs()
                < 0.05
        );
        Ok(())
    }

    #[test]
    fn print_table_does_not_panic_with_or_without_ci_baseline() -> Result<()> {
        let tmp = TempDir::new()?;
        write_ledger(tmp.path(), "flaky_tests: []\nknown_issues: []\ntechnical_debt: []\n")?;
        let m = collect_release_health(tmp.path(), 30)?;
        print_table(&m);
        write_ci_baseline(
            tmp.path(),
            r#"{"total_runs": 1, "total_billable_minutes": 2, "overall_success_rate_percent": 100.0}"#,
        )?;
        let m2 = collect_release_health(tmp.path(), 30)?;
        print_table(&m2);
        Ok(())
    }
}
