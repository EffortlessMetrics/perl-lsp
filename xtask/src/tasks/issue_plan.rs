//! Issue Research / Plan Review Desk — report-only issue-plan audit.
//!
//! Scans GitHub issues (from a JSON fixture or live `gh issue list`) for the
//! issue-plan quality problems the desk cares about:
//!
//! - `builder-ready` on a closed issue (label drift)
//! - `builder-ready` whose body is missing a required work-order section
//! - a stale routing-label contradiction (`needs-plan-review` co-present with a
//!   later sign-off such as `builder-ready` or `plan-reviewed`)
//! - a `#0000` placeholder issue reference
//!
//! Report-only by design (instrument before enforcement): the command always
//! exits 0 and emits an advisory receipt. See
//! `docs/reference/ISSUE_PLAN_DOCTRINE.md`.

use color_eyre::eyre::{Context, Result, bail};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum IssuePlanOutputFormat {
    Human,
    Json,
}

#[derive(Debug, Clone)]
pub struct AuditConfig {
    pub fixture: Option<PathBuf>,
    pub repo: Option<String>,
    pub labels: Vec<String>,
    pub receipt: PathBuf,
    pub dry_run: bool,
    pub format: IssuePlanOutputFormat,
}

/// An issue as produced by `gh issue list --json ...` or a test fixture.
#[derive(Debug, Clone, Deserialize)]
struct Issue {
    number: u64,
    #[serde(default)]
    title: String,
    #[serde(default)]
    body: Option<String>,
    /// `OPEN`/`CLOSED` (gh) or `open`/`closed`. Treated as open when absent.
    #[serde(default = "default_state")]
    state: String,
    #[serde(default)]
    labels: Vec<LabelObject>,
}

fn default_state() -> String {
    "OPEN".to_string()
}

#[derive(Debug, Clone, Deserialize)]
struct LabelObject {
    name: String,
}

impl Issue {
    fn has_label(&self, name: &str) -> bool {
        self.labels.iter().any(|label| label.name == name)
    }

    fn is_closed(&self) -> bool {
        self.state.eq_ignore_ascii_case("closed")
    }

    fn body_text(&self) -> &str {
        self.body.as_deref().unwrap_or("")
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct Finding {
    issue: u64,
    check: String,
    severity: String,
    message: String,
}

#[derive(Debug, Serialize)]
struct AuditReceipt {
    schema_version: String,
    mode: String,
    summary: String,
    issues_scanned: usize,
    findings_count: usize,
    by_check: BTreeMap<String, usize>,
    findings: Vec<Finding>,
}

pub fn audit(config: AuditConfig) -> Result<()> {
    let issues = load_issues(&config)?;
    let findings = run_checks(&issues);

    let mut by_check: BTreeMap<String, usize> = BTreeMap::new();
    for finding in &findings {
        *by_check.entry(finding.check.clone()).or_default() += 1;
    }

    let summary = if findings.is_empty() {
        format!("no issue-plan problems across {} issue(s)", issues.len())
    } else {
        format!("{} finding(s) across {} issue(s)", findings.len(), issues.len())
    };

    let receipt = AuditReceipt {
        schema_version: "1".to_string(),
        mode: "advisory".to_string(),
        summary,
        issues_scanned: issues.len(),
        findings_count: findings.len(),
        by_check,
        findings,
    };

    write_receipt(&config, &receipt)?;
    print_receipt(&config, &receipt)?;

    // Report-only: surfacing findings never fails the command.
    Ok(())
}

// --- checks -----------------------------------------------------------------

static RE_ACCEPTANCE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?i)acceptance").ok());
static RE_REPRO: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?i)reproduc|\brepro\b|\bexample\b").ok());
static RE_ROOT: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?i)root\s+(cause|area)|suspected\s+root").ok());
static RE_NON_GOAL: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?i)non[-\s]?goal").ok());
static RE_DEPENDENCIES: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?i)dependenc|blocked\s+by|sequenc").ok());
static RE_RISK: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?i)\brisk\b|rollback|rollout").ok());
static RE_VERIFICATION: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(r"(?i)verif").ok());
static RE_PLACEHOLDER_REF: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(r"#0000\b").ok());

fn re_matches(re: &LazyLock<Option<Regex>>, text: &str) -> bool {
    re.as_ref().is_some_and(|regex| regex.is_match(text))
}

fn run_checks(issues: &[Issue]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for issue in issues {
        check_builder_ready(issue, &mut findings);
        check_routing_contradiction(issue, &mut findings);
        check_placeholder_ref(issue, &mut findings);
    }
    findings.sort_by(|a, b| a.issue.cmp(&b.issue).then_with(|| a.check.cmp(&b.check)));
    findings
}

fn check_builder_ready(issue: &Issue, out: &mut Vec<Finding>) {
    if !issue.has_label("builder-ready") {
        return;
    }

    if issue.is_closed() {
        out.push(Finding {
            issue: issue.number,
            check: "builder-ready-on-closed".to_string(),
            severity: "high".to_string(),
            message: "closed issue still carries the `builder-ready` label".to_string(),
        });
        // A closed builder-ready issue is drift; section completeness is moot.
        return;
    }

    let body = issue.body_text();
    let sections = [
        ("acceptance tests", &RE_ACCEPTANCE),
        ("reproduction / example", &RE_REPRO),
        ("suspected root area", &RE_ROOT),
        ("non-goals", &RE_NON_GOAL),
        ("dependencies / sequencing", &RE_DEPENDENCIES),
        ("risk / rollback", &RE_RISK),
        ("verification notes", &RE_VERIFICATION),
    ];
    for (label, re) in sections {
        if !re_matches(re, body) {
            out.push(Finding {
                issue: issue.number,
                check: "builder-ready-missing-section".to_string(),
                severity: "medium".to_string(),
                message: format!("`builder-ready` but body is missing a \"{label}\" section"),
            });
        }
    }
}

fn check_routing_contradiction(issue: &Issue, out: &mut Vec<Finding>) {
    if !issue.has_label("needs-plan-review") {
        return;
    }
    for later_signoff in ["builder-ready", "plan-reviewed"] {
        if issue.has_label(later_signoff) {
            out.push(Finding {
                issue: issue.number,
                check: "routing-label-contradiction".to_string(),
                severity: "high".to_string(),
                message: format!(
                    "carries `needs-plan-review` alongside `{later_signoff}` — stale routing label"
                ),
            });
        }
    }
}

fn check_placeholder_ref(issue: &Issue, out: &mut Vec<Finding>) {
    let in_title = re_matches(&RE_PLACEHOLDER_REF, &issue.title);
    let in_body = re_matches(&RE_PLACEHOLDER_REF, issue.body_text());
    if in_title || in_body {
        let location = if in_title { "title" } else { "body" };
        out.push(Finding {
            issue: issue.number,
            check: "placeholder-issue-ref".to_string(),
            severity: "low".to_string(),
            message: format!("contains a `#0000` placeholder reference in the {location}"),
        });
    }
}

// --- io ---------------------------------------------------------------------

fn load_issues(config: &AuditConfig) -> Result<Vec<Issue>> {
    if let Some(path) = &config.fixture {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("failed to read fixture {}", path.display()))?;
        let issues: Vec<Issue> = serde_json::from_str(&contents)
            .with_context(|| format!("failed to parse fixture {}", path.display()))?;
        return Ok(issues);
    }
    load_issues_from_gh(config)
}

/// Build the `gh issue list` argv for the live loader.
///
/// `--state all` is required, not `open`: the `builder-ready-on-closed` drift
/// check (see `check_builder_ready`) exists specifically to catch closed
/// issues that still carry the `builder-ready` label. A `--state open` filter
/// would never retrieve a closed issue, so that check would be dead code in
/// live mode — it would only ever fire against fixture-injected test data.
/// The `--label` filter (when configured) already scopes the query to a
/// small, builder-ready-labeled set, so widening `--state` here stays bounded.
fn build_gh_issue_list_args(config: &AuditConfig) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "issue".to_string(),
        "list".to_string(),
        "--state".to_string(),
        "all".to_string(),
        "--limit".to_string(),
        "500".to_string(),
        "--json".to_string(),
        "number,title,body,state,labels".to_string(),
    ];
    if let Some(repo) = &config.repo {
        args.push("--repo".to_string());
        args.push(repo.clone());
    }
    for label in &config.labels {
        args.push("--label".to_string());
        args.push(label.clone());
    }
    args
}

fn load_issues_from_gh(config: &AuditConfig) -> Result<Vec<Issue>> {
    let args = build_gh_issue_list_args(config);

    let output = std::process::Command::new("gh")
        .args(&args)
        .output()
        .context("failed to execute gh for issue list (install gh, or pass --fixture)")?;

    if !output.status.success() {
        bail!(
            "gh issue list failed with status {}",
            output.status.code().map_or_else(|| "signal".to_string(), |code| code.to_string())
        );
    }

    let issues: Vec<Issue> =
        serde_json::from_slice(&output.stdout).context("failed to decode gh issue list JSON")?;
    Ok(issues)
}

fn print_receipt(config: &AuditConfig, receipt: &AuditReceipt) -> Result<()> {
    match config.format {
        IssuePlanOutputFormat::Human => {
            println!("Issue-Plan Audit [{}]: {}", receipt.mode, receipt.summary);
            for (check, count) in &receipt.by_check {
                println!("  {check}: {count}");
            }
            for finding in &receipt.findings {
                println!(
                    "  #{} [{}] {}: {}",
                    finding.issue, finding.severity, finding.check, finding.message
                );
            }
            Ok(())
        }
        IssuePlanOutputFormat::Json => {
            let rendered = serde_json::to_string_pretty(receipt)?;
            println!("{rendered}");
            Ok(())
        }
    }
}

fn write_receipt(config: &AuditConfig, receipt: &AuditReceipt) -> Result<()> {
    if config.dry_run {
        return Ok(());
    }
    if let Some(parent) = config.receipt.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let rendered = serde_json::to_string_pretty(receipt)?;
    fs::write(&config.receipt, rendered)
        .with_context(|| format!("failed to write receipt {}", config.receipt.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn issue(number: u64, labels: &[&str], state: &str, body: &str) -> Issue {
        Issue {
            number,
            title: format!("issue {number}"),
            body: Some(body.to_string()),
            state: state.to_string(),
            labels: labels.iter().map(|name| LabelObject { name: (*name).to_string() }).collect(),
        }
    }

    const FULL_BODY: &str = "## Problem\nx\n## Reproduction / example\nx\n## Suspected root area\nx\n## Acceptance tests\nx\n## Non-goals\nx\n## Dependencies / sequencing\nx\n## Risk / rollback\nx\n## Verification notes\nx";

    #[test]
    fn closed_builder_ready_is_flagged() {
        let issues = vec![issue(860, &["builder-ready", "plan-reviewed"], "CLOSED", FULL_BODY)];
        let findings = run_checks(&issues);
        assert!(findings.iter().any(|f| f.check == "builder-ready-on-closed"));
    }

    #[test]
    fn routing_contradiction_is_flagged_for_both_signoffs() {
        let issues = vec![issue(
            911,
            &["builder-ready", "needs-plan-review", "plan-reviewed"],
            "OPEN",
            FULL_BODY,
        )];
        let findings = run_checks(&issues);
        let contradictions =
            findings.iter().filter(|f| f.check == "routing-label-contradiction").count();
        assert_eq!(contradictions, 2, "expected builder-ready + plan-reviewed contradictions");
    }

    #[test]
    fn placeholder_ref_in_title_is_flagged() {
        let mut subject = issue(1, &[], "OPEN", "no placeholder in the body");
        subject.title = "fix things (#0000)".to_string();
        let findings = run_checks(&[subject]);
        assert!(findings.iter().any(|f| f.check == "placeholder-issue-ref"));
    }

    #[test]
    fn complete_builder_ready_has_no_section_findings() {
        let issues = vec![issue(924, &["builder-ready"], "OPEN", FULL_BODY)];
        let findings = run_checks(&issues);
        assert!(
            !findings.iter().any(|f| f.check == "builder-ready-missing-section"),
            "unexpected section findings: {findings:?}"
        );
    }

    #[test]
    fn builder_ready_missing_acceptance_is_flagged() {
        let issues = vec![issue(2, &["builder-ready"], "OPEN", "## Problem\nonly a problem")];
        let findings = run_checks(&issues);
        assert!(
            findings.iter().any(|f| {
                f.check == "builder-ready-missing-section" && f.message.contains("acceptance")
            }),
            "expected a missing-acceptance finding: {findings:?}"
        );
    }

    #[test]
    fn live_loader_args_request_all_states_not_just_open() {
        // Fix-proving test (mutation-check discipline): the fixture-based
        // integration tests bypass `load_issues_from_gh` entirely, so they
        // never catch a `--state open` regression here. This test pins the
        // actual argv the live loader sends to `gh issue list`.
        //
        // Without the fix (state hardcoded to "open"), this assertion fails:
        // args contains "open", not "all" — the `builder-ready-on-closed`
        // check (see `closed_builder_ready_is_flagged` above) would never
        // see a closed issue in production, because `gh issue list --state
        // open` never returns one.
        let config = AuditConfig {
            fixture: None,
            repo: Some("EffortlessMetrics/perl-lsp-swarm".to_string()),
            labels: vec!["builder-ready".to_string()],
            receipt: PathBuf::from("/tmp/unused-receipt.json"),
            dry_run: true,
            format: IssuePlanOutputFormat::Json,
        };
        let args = build_gh_issue_list_args(&config);

        let state_idx = args.iter().position(|arg| arg == "--state").expect(
            "gh issue list args must include --state \
             (test assertion, not a banned prod pattern)",
        );
        let state_value = args.get(state_idx + 1).map(String::as_str);
        assert_eq!(
            state_value,
            Some("all"),
            "live loader must request --state all so closed issues (and the \
             builder-ready-on-closed drift check) are reachable in \
             production, not just via --fixture; got args: {args:?}"
        );

        // The --label filter still scopes the (now-broader) query.
        assert!(args.windows(2).any(|w| w[0] == "--label" && w[1] == "builder-ready"));
    }

    #[test]
    fn non_builder_ready_issue_is_not_section_checked() {
        let issues =
            vec![issue(3, &["needs-plan-review", "swarm-discovered"], "OPEN", "bare body")];
        let findings = run_checks(&issues);
        assert!(!findings.iter().any(|f| f.check == "builder-ready-missing-section"));
        // needs-plan-review alone (no later sign-off) is not a contradiction.
        assert!(!findings.iter().any(|f| f.check == "routing-label-contradiction"));
    }
}
