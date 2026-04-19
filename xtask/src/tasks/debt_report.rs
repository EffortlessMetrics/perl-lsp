//! Technical debt report generator (native Rust port of `scripts/debt-report.py`).
//!
//! Reads `.ci/debt-ledger.yaml` and generates reports on current technical debt,
//! flaky tests, and known issues. Supports:
//!   - Console output (default)
//!   - JSON output (`--json`)
//!   - CI gate mode (`--check`)
//!   - Expired-only view (`--expired`)

use chrono::{NaiveDate, Utc};
use color_eyre::eyre::{Context, Result, eyre};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use crate::utils::project_root;

// ---------------------------------------------------------------------------
// Deserialization types for debt-ledger.yaml
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct Ledger {
    #[serde(default = "default_schema_version")]
    schema_version: u32,
    #[serde(default)]
    budgets: Budgets,
    #[serde(default)]
    flaky_tests: Option<Vec<FlakyTest>>,
    #[serde(default)]
    known_issues: Option<Vec<KnownIssue>>,
    #[serde(default)]
    technical_debt: Option<Vec<TechDebt>>,
}

fn default_schema_version() -> u32 {
    1
}

#[derive(Debug, Deserialize)]
struct Budgets {
    #[serde(default = "default_max_quarantined")]
    max_quarantined_tests: u32,
    #[serde(default = "default_max_known_issues")]
    max_known_issues: u32,
    #[serde(default = "default_max_technical_debt")]
    max_technical_debt: u32,
    #[serde(default = "default_warning_threshold")]
    warning_threshold_percent: u32,
    #[serde(default = "default_critical_threshold")]
    critical_threshold_percent: u32,
}

fn default_max_quarantined() -> u32 {
    10
}
fn default_max_known_issues() -> u32 {
    20
}
fn default_max_technical_debt() -> u32 {
    30
}
fn default_warning_threshold() -> u32 {
    80
}
fn default_critical_threshold() -> u32 {
    95
}

impl Default for Budgets {
    fn default() -> Self {
        Self {
            max_quarantined_tests: default_max_quarantined(),
            max_known_issues: default_max_known_issues(),
            max_technical_debt: default_max_technical_debt(),
            warning_threshold_percent: default_warning_threshold(),
            critical_threshold_percent: default_critical_threshold(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct FlakyTest {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    added: Option<String>,
    #[serde(default)]
    issue: Option<String>,
    #[serde(default)]
    quarantine_days: Option<i64>,
    #[serde(default)]
    expires: Option<String>,
}

#[derive(Debug, Deserialize)]
struct KnownIssue {
    #[serde(default)]
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TechDebt {
    #[serde(default)]
    area: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    priority: Option<String>,
    #[serde(default)]
    issue: Option<String>,
}

// ---------------------------------------------------------------------------
// Report output types (JSON-serialisable)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct Report {
    timestamp: String,
    schema_version: u32,
    summary: Summary,
    alerts: Vec<String>,
    details: Details,
}

#[derive(Debug, Serialize)]
struct Summary {
    overall_status: String,
    quarantined_tests: BudgetStatus,
    known_issues: KnownIssueStatus,
    technical_debt: TechDebtStatus,
}

#[derive(Debug, Serialize)]
struct BudgetStatus {
    count: u32,
    budget: u32,
    percent: f64,
    status: String,
    expired: u32,
    expiring_soon: u32,
}

#[derive(Debug, Serialize)]
struct KnownIssueStatus {
    count: u32,
    budget: u32,
    percent: f64,
    status: String,
    by_status: BTreeMap<String, u32>,
}

#[derive(Debug, Serialize)]
struct TechDebtStatus {
    count: u32,
    budget: u32,
    percent: f64,
    status: String,
    by_priority: BTreeMap<String, u32>,
}

#[derive(Debug, Serialize)]
struct Details {
    expired_quarantines: Vec<ExpiredQuarantine>,
    expiring_soon: Vec<ExpiringSoon>,
    critical_debt: Vec<CriticalDebt>,
}

#[derive(Debug, Serialize)]
struct ExpiredQuarantine {
    name: Option<String>,
    issue: Option<String>,
    expired: Option<String>,
    days_overdue: i64,
}

#[derive(Debug, Serialize)]
struct ExpiringSoon {
    name: Option<String>,
    issue: Option<String>,
    expires: Option<String>,
    days_remaining: i64,
}

#[derive(Debug, Serialize)]
struct CriticalDebt {
    area: Option<String>,
    description: Option<String>,
    issue: Option<String>,
}

// ---------------------------------------------------------------------------
// Configuration for the debt-report subcommand
// ---------------------------------------------------------------------------

pub struct DebtReportConfig {
    pub check: bool,
    pub json: bool,
    pub summary: bool,
    pub expired: bool,
    pub ledger: Option<PathBuf>,
}

// ---------------------------------------------------------------------------
// Core logic
// ---------------------------------------------------------------------------

fn load_ledger(ledger_path: &PathBuf) -> Result<Ledger> {
    if !ledger_path.exists() {
        return Ok(Ledger {
            schema_version: 1,
            budgets: Budgets::default(),
            flaky_tests: Some(Vec::new()),
            known_issues: Some(Vec::new()),
            technical_debt: Some(Vec::new()),
        });
    }

    let content = fs::read_to_string(ledger_path)
        .with_context(|| format!("reading {}", ledger_path.display()))?;
    let ledger: Ledger = serde_yaml_ng::from_str(&content)
        .with_context(|| format!("parsing YAML from {}", ledger_path.display()))?;
    Ok(ledger)
}

fn parse_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

fn calculate_expiry(item: &FlakyTest) -> Option<NaiveDate> {
    if let Some(ref exp) = item.expires
        && let Some(d) = parse_date(exp)
    {
        return Some(d);
    }

    if let (Some(added_str), Some(days)) = (&item.added, item.quarantine_days)
        && let Some(added) = parse_date(added_str)
    {
        return added.checked_add_signed(chrono::Duration::days(days));
    }

    None
}

fn days_until_expiry(item: &FlakyTest, today: NaiveDate) -> Option<i64> {
    calculate_expiry(item).map(|exp| (exp - today).num_days())
}

fn is_expired(item: &FlakyTest, today: NaiveDate) -> bool {
    match calculate_expiry(item) {
        Some(exp) => exp < today,
        None => false,
    }
}

fn count_by_field<'a>(items: impl Iterator<Item = &'a str>, target: &str) -> u32 {
    items.filter(|v| *v == target).count() as u32
}

fn budget_level(pct: f64, warning: u32, critical: u32) -> String {
    if pct >= f64::from(critical) {
        "critical".to_string()
    } else if pct >= f64::from(warning) {
        "warning".to_string()
    } else {
        "ok".to_string()
    }
}

fn generate_report(ledger: &Ledger, today: NaiveDate) -> Report {
    let budgets = &ledger.budgets;
    let flaky_tests = ledger.flaky_tests.as_deref().unwrap_or(&[]);
    let known_issues = ledger.known_issues.as_deref().unwrap_or(&[]);
    let technical_debt = ledger.technical_debt.as_deref().unwrap_or(&[]);

    let quarantined_count = flaky_tests.len() as u32;
    let known_issues_count = known_issues.len() as u32;
    let tech_debt_count = technical_debt.len() as u32;

    let max_q = budgets.max_quarantined_tests;
    let max_i = budgets.max_known_issues;
    let max_d = budgets.max_technical_debt;
    let warn = budgets.warning_threshold_percent;
    let crit = budgets.critical_threshold_percent;

    let q_pct = if max_q > 0 {
        f64::from(quarantined_count) / f64::from(max_q) * 100.0
    } else {
        0.0
    };
    let i_pct = if max_i > 0 {
        f64::from(known_issues_count) / f64::from(max_i) * 100.0
    } else {
        0.0
    };
    let d_pct = if max_d > 0 {
        f64::from(tech_debt_count) / f64::from(max_d) * 100.0
    } else {
        0.0
    };

    let expired_quarantines: Vec<&FlakyTest> = flaky_tests
        .iter()
        .filter(|t| is_expired(t, today))
        .collect();
    let expiring_soon: Vec<&FlakyTest> = flaky_tests
        .iter()
        .filter(|t| {
            if is_expired(t, today) {
                return false;
            }
            matches!(days_until_expiry(t, today), Some(d) if (0..=7).contains(&d))
        })
        .collect();

    let q_status = budget_level(q_pct, warn, crit);
    let i_status = budget_level(i_pct, warn, crit);
    let d_status = budget_level(d_pct, warn, crit);

    let mut overall = "ok".to_string();
    if q_status == "critical" || i_status == "critical" || d_status == "critical" {
        overall = "critical".to_string();
    } else if q_status == "warning" || i_status == "warning" || d_status == "warning" {
        overall = "warning".to_string();
    }
    if !expired_quarantines.is_empty() {
        overall = "critical".to_string();
    }

    // known issues by status
    let issue_statuses = ["accepted", "deferred", "monitoring", "wontfix"];
    let mut by_status = BTreeMap::new();
    for s in &issue_statuses {
        let c = count_by_field(known_issues.iter().filter_map(|ki| ki.status.as_deref()), s);
        by_status.insert((*s).to_string(), c);
    }

    // technical debt by priority
    let priorities = ["critical", "high", "medium", "low"];
    let mut by_priority = BTreeMap::new();
    for p in &priorities {
        let c = count_by_field(
            technical_debt
                .iter()
                .filter_map(|td| td.priority.as_deref()),
            p,
        );
        by_priority.insert((*p).to_string(), c);
    }

    let details = Details {
        expired_quarantines: expired_quarantines
            .iter()
            .map(|t| ExpiredQuarantine {
                name: t.name.clone(),
                issue: t.issue.clone(),
                expired: calculate_expiry(t).map(|d| d.format("%Y-%m-%d").to_string()),
                days_overdue: -(days_until_expiry(t, today).unwrap_or(0)),
            })
            .collect(),
        expiring_soon: expiring_soon
            .iter()
            .map(|t| ExpiringSoon {
                name: t.name.clone(),
                issue: t.issue.clone(),
                expires: calculate_expiry(t).map(|d| d.format("%Y-%m-%d").to_string()),
                days_remaining: days_until_expiry(t, today).unwrap_or(0),
            })
            .collect(),
        critical_debt: technical_debt
            .iter()
            .filter(|td| td.priority.as_deref() == Some("critical"))
            .map(|td| CriticalDebt {
                area: td.area.clone(),
                description: td.description.clone(),
                issue: td.issue.clone(),
            })
            .collect(),
    };

    let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    Report {
        timestamp,
        schema_version: ledger.schema_version,
        summary: Summary {
            overall_status: overall,
            quarantined_tests: BudgetStatus {
                count: quarantined_count,
                budget: max_q,
                percent: round1(q_pct),
                status: q_status,
                expired: expired_quarantines.len() as u32,
                expiring_soon: expiring_soon.len() as u32,
            },
            known_issues: KnownIssueStatus {
                count: known_issues_count,
                budget: max_i,
                percent: round1(i_pct),
                status: i_status,
                by_status,
            },
            technical_debt: TechDebtStatus {
                count: tech_debt_count,
                budget: max_d,
                percent: round1(d_pct),
                status: d_status,
                by_priority,
            },
        },
        alerts: Vec::new(),
        details,
    }
}

/// Round to one decimal place.
fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

// ---------------------------------------------------------------------------
// Console formatting
// ---------------------------------------------------------------------------

fn status_color(status: &str) -> String {
    let (code, label) = match status {
        "ok" => ("\x1b[32m", "OK"),
        "warning" => ("\x1b[33m", "WARNING"),
        "critical" => ("\x1b[31m", "CRITICAL"),
        _ => ("", status),
    };
    let upper = if label == status {
        status.to_uppercase()
    } else {
        label.to_string()
    };
    format!("{code}{upper}\x1b[0m")
}

fn format_console_report(report: &Report) -> String {
    let mut lines: Vec<String> = Vec::new();
    let summary = &report.summary;

    let sep = "=".repeat(60);
    lines.push(sep.clone());
    lines.push("           Technical Debt Report".to_string());
    lines.push(sep.clone());
    lines.push(format!("Generated: {}", report.timestamp));
    lines.push(format!(
        "Overall Status: {}",
        status_color(&summary.overall_status)
    ));
    lines.push(String::new());

    // Quarantined tests
    let q = &summary.quarantined_tests;
    lines.push(format!(
        "Quarantined Tests: {}/{} ({}%) [{}]",
        q.count,
        q.budget,
        q.percent,
        status_color(&q.status)
    ));
    if q.expired > 0 {
        lines.push(format!(
            "  \x1b[31mEXPIRED: {} quarantine(s) need resolution!\x1b[0m",
            q.expired
        ));
    }
    if q.expiring_soon > 0 {
        lines.push(format!(
            "  \x1b[33mExpiring soon: {} within 7 days\x1b[0m",
            q.expiring_soon
        ));
    }

    // Known issues
    let k = &summary.known_issues;
    lines.push(format!(
        "Known Issues: {}/{} ({}%) [{}]",
        k.count,
        k.budget,
        k.percent,
        status_color(&k.status)
    ));
    let status_parts: Vec<String> = k
        .by_status
        .iter()
        .filter(|(_, c)| **c > 0)
        .map(|(s, c)| format!("{s}: {c}"))
        .collect();
    if !status_parts.is_empty() {
        lines.push(format!("  {}", status_parts.join(", ")));
    }

    // Technical debt
    let t = &summary.technical_debt;
    lines.push(format!(
        "Technical Debt: {}/{} ({}%) [{}]",
        t.count,
        t.budget,
        t.percent,
        status_color(&t.status)
    ));
    let priority_parts: Vec<String> = t
        .by_priority
        .iter()
        .filter(|(_, c)| **c > 0)
        .map(|(p, c)| format!("{p}: {c}"))
        .collect();
    if !priority_parts.is_empty() {
        lines.push(format!("  {}", priority_parts.join(", ")));
    }

    // Details
    let details = &report.details;

    if !details.expired_quarantines.is_empty() {
        lines.push(String::new());
        lines.push("\x1b[31mExpired Quarantines (action required):\x1b[0m".to_string());
        for item in &details.expired_quarantines {
            let name = item.name.as_deref().unwrap_or("<unnamed>");
            let issue = item
                .issue
                .as_ref()
                .map(|i| format!(" ({i})"))
                .unwrap_or_default();
            lines.push(format!(
                "  - {name}{issue}: {} days overdue",
                item.days_overdue
            ));
        }
    }

    if !details.expiring_soon.is_empty() {
        lines.push(String::new());
        lines.push("\x1b[33mExpiring Soon:\x1b[0m".to_string());
        for item in &details.expiring_soon {
            let name = item.name.as_deref().unwrap_or("<unnamed>");
            let issue = item
                .issue
                .as_ref()
                .map(|i| format!(" ({i})"))
                .unwrap_or_default();
            lines.push(format!(
                "  - {name}{issue}: {} days remaining",
                item.days_remaining
            ));
        }
    }

    if !details.critical_debt.is_empty() {
        lines.push(String::new());
        lines.push("\x1b[31mCritical Technical Debt:\x1b[0m".to_string());
        for item in &details.critical_debt {
            let area = item.area.as_deref().unwrap_or("unknown");
            let desc = item.description.as_deref().unwrap_or("");
            let issue = item
                .issue
                .as_ref()
                .map(|i| format!(" ({i})"))
                .unwrap_or_default();
            lines.push(format!("  - [{area}] {desc}{issue}"));
        }
    }

    lines.push(String::new());
    lines.push(sep.clone());
    lines
        .push("Run `cargo xtask debt-report --check` to verify debt budget compliance".to_string());
    lines.push("Edit `.ci/debt-ledger.yaml` to add/remove tracked items".to_string());
    lines.push(sep);

    lines.join("\n")
}

fn format_summary_markdown(report: &Report) -> String {
    let mut lines: Vec<String> = Vec::new();
    let q = &report.summary.quarantined_tests;
    let k = &report.summary.known_issues;
    let t = &report.summary.technical_debt;

    lines.push("| Category | Count | Budget | Status |".to_string());
    lines.push("|----------|-------|--------|--------|".to_string());
    lines.push(format!(
        "| Quarantined Tests | {} | {} | {} |",
        q.count, q.budget, q.status
    ));
    lines.push(format!(
        "| Known Issues | {} | {} | {} |",
        k.count, k.budget, k.status
    ));
    lines.push(format!(
        "| Technical Debt | {} | {} | {} |",
        t.count, t.budget, t.status
    ));

    if q.expired > 0 {
        lines.push(String::new());
        lines.push(format!(
            "**Warning:** {0} expired quarantine(s) need attention!",
            q.expired
        ));
    }

    lines.join("\n")
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub fn run(config: DebtReportConfig) -> Result<()> {
    let root = project_root()?;
    let ledger_path = config
        .ledger
        .unwrap_or_else(|| root.join(".ci/debt-ledger.yaml"));

    let ledger = load_ledger(&ledger_path)?;
    let today = Utc::now().date_naive();
    let report = generate_report(&ledger, today);

    // --expired mode
    if config.expired {
        let expired = &report.details.expired_quarantines;
        if config.json {
            let json =
                serde_json::to_string_pretty(expired).context("serialising expired quarantines")?;
            println!("{json}");
        } else if expired.is_empty() {
            println!("No expired quarantines");
        } else {
            println!("Expired Quarantines:");
            for item in expired {
                let name = item.name.as_deref().unwrap_or("<unnamed>");
                println!("  - {name}: {} days overdue", item.days_overdue);
            }
            return Err(eyre!("Expired quarantines found"));
        }
        return Ok(());
    }

    if config.summary {
        println!("{}", format_summary_markdown(&report));
        if !config.check {
            return Ok(());
        }
    }

    // Produce output
    if config.json {
        let json = serde_json::to_string_pretty(&report).context("serialising report")?;
        println!("{json}");
    } else {
        println!("{}", format_console_report(&report));
    }

    // --check gate
    if config.check {
        let summary = &report.summary;
        let mut failures: Vec<String> = Vec::new();

        if summary.quarantined_tests.expired > 0 {
            failures.push(format!(
                "{} expired quarantine(s)",
                summary.quarantined_tests.expired
            ));
        }
        if summary.quarantined_tests.status == "critical" {
            failures.push("quarantined tests at critical level".to_string());
        }
        if summary.known_issues.status == "critical" {
            failures.push("known issues at critical level".to_string());
        }
        if summary.technical_debt.status == "critical" {
            failures.push("technical debt at critical level".to_string());
        }

        if failures.is_empty() {
            println!("\nDebt check PASSED");
            Ok(())
        } else {
            Err(eyre!("Debt check FAILED: {}", failures.join(", ")))
        }
    } else {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_ledger() -> Ledger {
        Ledger {
            schema_version: 1,
            budgets: Budgets {
                max_quarantined_tests: 10,
                max_known_issues: 20,
                max_technical_debt: 30,
                warning_threshold_percent: 80,
                critical_threshold_percent: 95,
            },
            flaky_tests: Some(vec![
                FlakyTest {
                    name: Some("test_a".to_string()),
                    added: Some("2026-01-01".to_string()),
                    issue: Some("#100".to_string()),
                    quarantine_days: Some(14),
                    expires: None,
                },
                FlakyTest {
                    name: Some("test_b".to_string()),
                    added: None,
                    issue: None,
                    quarantine_days: None,
                    expires: Some("2026-06-01".to_string()),
                },
            ]),
            known_issues: Some(vec![
                KnownIssue {
                    status: Some("accepted".to_string()),
                },
                KnownIssue {
                    status: Some("deferred".to_string()),
                },
            ]),
            technical_debt: Some(vec![
                TechDebt {
                    area: Some("error_handling".to_string()),
                    description: Some("unwrap calls".to_string()),
                    priority: Some("low".to_string()),
                    issue: None,
                },
                TechDebt {
                    area: Some("architecture".to_string()),
                    description: Some("monolith".to_string()),
                    priority: Some("critical".to_string()),
                    issue: Some("#42".to_string()),
                },
            ]),
        }
    }

    #[test]
    fn test_parse_date_valid() {
        assert!(parse_date("2026-01-15").is_some());
    }

    #[test]
    fn test_parse_date_invalid() {
        assert!(parse_date("not-a-date").is_none());
    }

    #[test]
    fn test_calculate_expiry_from_expires_field() {
        let item = FlakyTest {
            name: None,
            added: None,
            issue: None,
            quarantine_days: None,
            expires: Some("2026-03-01".to_string()),
        };
        let exp = calculate_expiry(&item);
        assert_eq!(exp, parse_date("2026-03-01"));
    }

    #[test]
    fn test_calculate_expiry_from_added_plus_days() {
        let item = FlakyTest {
            name: None,
            added: Some("2026-01-01".to_string()),
            issue: None,
            quarantine_days: Some(14),
            expires: None,
        };
        let exp = calculate_expiry(&item);
        assert_eq!(exp, parse_date("2026-01-15"));
    }

    #[test]
    fn test_is_expired_true() {
        let item = FlakyTest {
            name: None,
            added: None,
            issue: None,
            quarantine_days: None,
            expires: Some("2026-01-01".to_string()),
        };
        let today = parse_date("2026-02-01");
        assert!(today.is_some());
        assert!(is_expired(&item, today.unwrap_or(NaiveDate::MIN)));
    }

    #[test]
    fn test_is_expired_false() {
        let item = FlakyTest {
            name: None,
            added: None,
            issue: None,
            quarantine_days: None,
            expires: Some("2026-06-01".to_string()),
        };
        let today = parse_date("2026-02-01");
        assert!(today.is_some());
        assert!(!is_expired(&item, today.unwrap_or(NaiveDate::MIN)));
    }

    #[test]
    fn test_budget_level_ok() {
        assert_eq!(budget_level(50.0, 80, 95), "ok");
    }

    #[test]
    fn test_budget_level_warning() {
        assert_eq!(budget_level(85.0, 80, 95), "warning");
    }

    #[test]
    fn test_budget_level_critical() {
        assert_eq!(budget_level(96.0, 80, 95), "critical");
    }

    #[test]
    fn test_generate_report_overall_ok() {
        let ledger = Ledger {
            schema_version: 1,
            budgets: Budgets::default(),
            flaky_tests: Some(Vec::new()),
            known_issues: Some(Vec::new()),
            technical_debt: Some(Vec::new()),
        };
        let today = parse_date("2026-03-15").unwrap_or(NaiveDate::MIN);
        let report = generate_report(&ledger, today);
        assert_eq!(report.summary.overall_status, "ok");
    }

    #[test]
    fn test_generate_report_with_expired_quarantine() {
        let ledger = Ledger {
            schema_version: 1,
            budgets: Budgets::default(),
            flaky_tests: Some(vec![FlakyTest {
                name: Some("test_expired".to_string()),
                added: None,
                issue: Some("#1".to_string()),
                quarantine_days: None,
                expires: Some("2026-01-01".to_string()),
            }]),
            known_issues: Some(Vec::new()),
            technical_debt: Some(Vec::new()),
        };
        let today = parse_date("2026-03-15").unwrap_or(NaiveDate::MIN);
        let report = generate_report(&ledger, today);
        assert_eq!(report.summary.overall_status, "critical");
        assert_eq!(report.summary.quarantined_tests.expired, 1);
        assert_eq!(report.details.expired_quarantines.len(), 1);
    }

    #[test]
    fn test_generate_report_counts_by_priority() {
        let ledger = sample_ledger();
        let today = parse_date("2026-06-01").unwrap_or(NaiveDate::MIN);
        let report = generate_report(&ledger, today);
        assert_eq!(
            report.summary.technical_debt.by_priority.get("critical"),
            Some(&1)
        );
        assert_eq!(
            report.summary.technical_debt.by_priority.get("low"),
            Some(&1)
        );
        assert_eq!(
            report.summary.technical_debt.by_priority.get("medium"),
            Some(&0)
        );
    }

    #[test]
    fn test_generate_report_counts_by_status() {
        let ledger = sample_ledger();
        let today = parse_date("2026-06-01").unwrap_or(NaiveDate::MIN);
        let report = generate_report(&ledger, today);
        assert_eq!(
            report.summary.known_issues.by_status.get("accepted"),
            Some(&1)
        );
        assert_eq!(
            report.summary.known_issues.by_status.get("deferred"),
            Some(&1)
        );
        assert_eq!(
            report.summary.known_issues.by_status.get("monitoring"),
            Some(&0)
        );
    }

    #[test]
    fn test_console_report_contains_header() {
        let ledger = Ledger {
            schema_version: 1,
            budgets: Budgets::default(),
            flaky_tests: Some(Vec::new()),
            known_issues: Some(Vec::new()),
            technical_debt: Some(Vec::new()),
        };
        let today = parse_date("2026-03-15").unwrap_or(NaiveDate::MIN);
        let report = generate_report(&ledger, today);
        let output = format_console_report(&report);
        assert!(output.contains("Technical Debt Report"));
        assert!(output.contains("Quarantined Tests:"));
        assert!(output.contains("Known Issues:"));
        assert!(output.contains("Technical Debt:"));
    }

    #[test]
    fn test_round1() {
        assert!((round1(13.333_333) - 13.3).abs() < f64::EPSILON);
        assert!((round1(0.0) - 0.0).abs() < f64::EPSILON);
        assert!((round1(100.0) - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_load_ledger_real_file() -> Result<()> {
        let root = project_root()?;
        let path = root.join(".ci/debt-ledger.yaml");
        if path.exists() {
            let ledger = load_ledger(&path)?;
            assert_eq!(ledger.schema_version, 1);
        }
        Ok(())
    }

    #[test]
    fn test_load_ledger_missing_file() -> Result<()> {
        let path = PathBuf::from("/tmp/nonexistent-debt-ledger-test.yaml");
        let ledger = load_ledger(&path)?;
        assert_eq!(ledger.schema_version, 1);
        Ok(())
    }

    #[test]
    fn test_json_round_trip() -> Result<()> {
        let ledger = sample_ledger();
        let today = parse_date("2026-06-01").unwrap_or(NaiveDate::MIN);
        let report = generate_report(&ledger, today);
        let json = serde_json::to_string_pretty(&report).context("serialising")?;
        // Verify it's valid JSON by deserialising to generic Value
        let _: serde_json::Value = serde_json::from_str(&json).context("deserialising")?;
        Ok(())
    }
}
