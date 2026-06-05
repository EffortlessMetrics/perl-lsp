//! `cargo xtask pr title-check` — local PR title validator.
//!
//! Mirrors the `validate-title` GitHub Actions check so developers can
//! validate before pushing.  Does NOT replace the CI check; it is a
//! local pre-push convenience.
//!
//! # Exit codes
//! - `0` — all checks pass (or only warnings in default mode).
//! - `1` — a required check failed, or `--strict` and at least one warning.
//!
//! # JSON receipt (schema_version 1)
//! ```json
//! {
//!   "schema_version": 1,
//!   "title": "fix(scope): thing (#1234)",
//!   "issue_ref": 1234,
//!   "issue_exists": true,
//!   "issue_open": true,
//!   "type": "fix",
//!   "scope": "scope",
//!   "subject": "thing",
//!   "overall": "ok",
//!   "checks": [
//!     {"name": "issue-ref-present", "status": "ok"},
//!     {"name": "issue-exists",      "status": "ok"},
//!     {"name": "conventional-format","status": "ok"},
//!     {"name": "subject-length",     "status": "ok"},
//!     {"name": "issue-open",         "status": "ok"}
//!   ]
//! }
//! ```

use color_eyre::eyre::{Context, Result, bail};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::process::Command;

// ---------------------------------------------------------------------------
// Public config
// ---------------------------------------------------------------------------

/// Configuration for the `pr title-check` subcommand.
pub struct TitleCheckConfig {
    /// Optional title to check. When `None`, read from `git log -1 --pretty=%s`.
    pub title: Option<String>,
    /// Emit JSON receipt to stdout instead of human-readable output.
    pub json: bool,
    /// Exit 1 on warnings (not just hard failures).
    pub strict: bool,
    /// Skip the GitHub issue-existence check entirely.
    pub no_gh: bool,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// Overall validation status.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OverallStatus {
    /// All checks passed.
    Ok,
    /// At least one warning (no hard failure).
    Warn,
    /// At least one hard failure.
    Fail,
}

impl std::fmt::Display for OverallStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OverallStatus::Ok => write!(f, "ok"),
            OverallStatus::Warn => write!(f, "warn"),
            OverallStatus::Fail => write!(f, "fail"),
        }
    }
}

/// Individual check result status.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    Ok,
    Warn,
    Fail,
    Skipped,
}

impl std::fmt::Display for CheckStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckStatus::Ok => write!(f, "ok"),
            CheckStatus::Warn => write!(f, "warn"),
            CheckStatus::Fail => write!(f, "fail"),
            CheckStatus::Skipped => write!(f, "skipped"),
        }
    }
}

/// A single named check.
#[derive(Debug, Serialize, Deserialize)]
pub struct CheckResult {
    pub name: String,
    pub status: CheckStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Full JSON receipt schema (schema_version 1).
#[derive(Debug, Serialize, Deserialize)]
pub struct TitleCheckReceipt {
    pub schema_version: u32,
    pub title: String,
    pub issue_ref: Option<u64>,
    pub issue_exists: Option<bool>,
    pub issue_open: Option<bool>,
    #[serde(rename = "type")]
    pub commit_type: Option<String>,
    pub scope: Option<String>,
    pub subject: Option<String>,
    pub overall: OverallStatus,
    pub checks: Vec<CheckResult>,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Run the `pr title-check` subcommand.
pub fn run(cfg: TitleCheckConfig) -> Result<()> {
    let title = match cfg.title {
        Some(t) => t,
        None => read_head_commit_subject()?,
    };

    let receipt = validate_title(&title, cfg.no_gh)?;

    if cfg.json {
        let json =
            serde_json::to_string_pretty(&receipt).context("failed to serialize JSON receipt")?;
        println!("{json}");
    } else {
        print_human(&receipt);
    }

    // Decide exit code.
    let fail = match receipt.overall {
        OverallStatus::Fail => true,
        OverallStatus::Warn => cfg.strict,
        OverallStatus::Ok => false,
    };

    if fail {
        bail!("pr title-check failed (overall: {})", receipt.overall);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Validation logic
// ---------------------------------------------------------------------------

/// Perform all checks and return a receipt.
///
/// `no_gh` suppresses the GitHub API call (useful in offline / CI contexts
/// where `GH_TOKEN` is absent or `--no-gh` was passed explicitly).
fn validate_title(title: &str, no_gh: bool) -> Result<TitleCheckReceipt> {
    let mut checks: Vec<CheckResult> = Vec::new();

    // ------------------------------------------------------------------
    // 1. Issue reference present: regex mirrors the GitHub Actions workflow
    //    `.github/workflows/pr-title-check.yml` which uses /\B#(\d+)\b/g
    // ------------------------------------------------------------------
    // Policy (issue #724): a zero-valued reference (`#0` / `#0000`) is the
    // sanctioned placeholder for "issue number not yet known". It is
    // NON-BLOCKING — warn rather than fail — so agents never guess a real
    // issue number. A title with NO reference at all is still a hard failure.
    let issue_re = Regex::new(r"\B#(\d+)\b").ok();
    let refs: Vec<u64> = issue_re
        .as_ref()
        .map(|re| {
            re.captures_iter(title)
                .filter_map(|cap| cap.get(1))
                .filter_map(|m| m.as_str().parse::<u64>().ok())
                .collect()
        })
        .unwrap_or_default();
    let issue_ref: Option<u64> = refs.iter().copied().find(|&n| n != 0);
    let has_placeholder = refs.iter().any(|&n| n == 0);

    if issue_ref.is_some() {
        checks.push(CheckResult {
            name: "issue-ref-present".into(),
            status: CheckStatus::Ok,
            message: None,
        });
    } else if has_placeholder {
        checks.push(CheckResult {
            name: "issue-ref-present".into(),
            status: CheckStatus::Warn,
            message: Some(
                "Placeholder issue reference (#0000) accepted — link a real issue \
                 before merge. Never guess a real issue number."
                    .into(),
            ),
        });
    } else {
        checks.push(CheckResult {
            name: "issue-ref-present".into(),
            status: CheckStatus::Fail,
            message: Some(
                "Title must contain an issue reference like (#1234), or (#0000) \
                 if the issue number is not yet known. Pattern: \\B#\\d+\\b"
                    .into(),
            ),
        });
    }

    // ------------------------------------------------------------------
    // 2. Conventional-commit format: type(scope)?: subject
    //    Types: fix, feat, test, docs, chore, perf, refactor, ci, build,
    //           revert, style, security, tooling, status
    // ------------------------------------------------------------------
    let conv_re = Regex::new(
        r"(?x)
        ^
        (?P<type>feat|fix|docs|style|refactor|perf|test|chore|ci|build|revert|security|tooling|status)
        (?:\((?P<scope>[^)]+)\))?
        !?
        :\s
        (?P<subject>.+)
        $",
    )
    .ok();

    let (commit_type, scope, subject) = match &conv_re {
        Some(re) => match re.captures(title) {
            Some(caps) => {
                let t = caps.name("type").map(|m| m.as_str().to_string());
                let s = caps.name("scope").map(|m| m.as_str().to_string());
                // Subject is everything before the optional trailing issue ref.
                let raw_subject =
                    caps.name("subject").map(|m| m.as_str().to_string()).unwrap_or_default();
                // Strip trailing " (#NNNN)" for subject length check.
                let subject_stripped = strip_issue_ref(&raw_subject);
                checks.push(CheckResult {
                    name: "conventional-format".into(),
                    status: CheckStatus::Ok,
                    message: None,
                });
                (t, s, Some(subject_stripped))
            }
            None => {
                checks.push(CheckResult {
                    name: "conventional-format".into(),
                    status: CheckStatus::Fail,
                    message: Some(
                        "Title must match `type(scope)?: subject`. \
                         Valid types: feat, fix, docs, style, refactor, perf, test, \
                         chore, ci, build, revert, security, tooling, status"
                            .into(),
                    ),
                });
                (None, None, None)
            }
        },
        None => {
            checks.push(CheckResult {
                name: "conventional-format".into(),
                status: CheckStatus::Skipped,
                message: Some("regex compilation failed".into()),
            });
            (None, None, None)
        }
    };

    // ------------------------------------------------------------------
    // 3. Subject length (warn at > 72 chars before the issue ref)
    // ------------------------------------------------------------------
    let subject_for_len = subject.clone().unwrap_or_else(|| strip_issue_ref(title));
    let subject_len = subject_for_len.chars().count();
    if subject_len > 72 {
        checks.push(CheckResult {
            name: "subject-length".into(),
            status: CheckStatus::Warn,
            message: Some(format!("Subject is {subject_len} chars (max 72 before issue ref)")),
        });
    } else {
        checks.push(CheckResult {
            name: "subject-length".into(),
            status: CheckStatus::Ok,
            message: None,
        });
    }

    // ------------------------------------------------------------------
    // 4. Issue exists on GitHub (optional — skipped when no_gh or no token)
    // ------------------------------------------------------------------
    let gh_available = !no_gh && gh_token_present();
    let (issue_exists, issue_open) = if let Some(num) = issue_ref {
        if gh_available {
            match query_issue(num) {
                Ok((exists, open)) => (Some(exists), Some(open)),
                Err(_) => (None, None),
            }
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    // Emit issue-exists check.
    if issue_ref.is_some() {
        match issue_exists {
            Some(true) => {
                checks.push(CheckResult {
                    name: "issue-exists".into(),
                    status: CheckStatus::Ok,
                    message: None,
                });
            }
            Some(false) => {
                checks.push(CheckResult {
                    name: "issue-exists".into(),
                    status: CheckStatus::Warn,
                    message: issue_ref.map(|n| format!("Issue #{n} not found — verify the number")),
                });
            }
            None => {
                checks.push(CheckResult {
                    name: "issue-exists".into(),
                    status: CheckStatus::Skipped,
                    message: Some("GitHub check skipped (--no-gh or GH_TOKEN not set)".into()),
                });
            }
        }
    }

    // ------------------------------------------------------------------
    // 5. Issue open (warn when closed, unless type is docs/status or
    //    subject contains "supersede")
    // ------------------------------------------------------------------
    let closed_exempt =
        commit_type.as_deref().map(|t| matches!(t, "docs" | "status")).unwrap_or(false)
            || subject.as_deref().map(subject_contains_supersede).unwrap_or(false);

    if let (Some(num), Some(true), Some(open)) = (issue_ref, issue_exists, issue_open) {
        if !open && !closed_exempt {
            checks.push(CheckResult {
                name: "issue-open".into(),
                status: CheckStatus::Warn,
                message: Some(format!(
                    "Issue #{num} is closed. Use --strict to fail on this warning, \
                     or use `docs:`/`status:` type to suppress."
                )),
            });
        } else {
            checks.push(CheckResult {
                name: "issue-open".into(),
                status: CheckStatus::Ok,
                message: None,
            });
        }
    }

    // ------------------------------------------------------------------
    // Compute overall status.
    // ------------------------------------------------------------------
    let overall = compute_overall(&checks);

    Ok(TitleCheckReceipt {
        schema_version: 1,
        title: title.to_string(),
        issue_ref,
        issue_exists,
        issue_open,
        commit_type,
        scope,
        subject,
        overall,
        checks,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Strip a trailing ` (#NNNN)` or `(#NNNN)` suffix from a subject string.
fn strip_issue_ref(s: &str) -> String {
    Regex::new(r"\s*\(#\d+\)\s*$")
        .ok()
        .and_then(|re| {
            let s2 = re.replace(s, "");
            if s2 != s { Some(s2.into_owned()) } else { None }
        })
        .unwrap_or_else(|| s.to_string())
}

/// Return true when a subject requests superseding prior work.
fn subject_contains_supersede(subject: &str) -> bool {
    subject.to_ascii_lowercase().contains("supersede")
}

/// Compute overall status from a list of check results.
fn compute_overall(checks: &[CheckResult]) -> OverallStatus {
    let mut overall = OverallStatus::Ok;
    for c in checks {
        match c.status {
            CheckStatus::Fail => return OverallStatus::Fail,
            CheckStatus::Warn => overall = OverallStatus::Warn,
            CheckStatus::Ok | CheckStatus::Skipped => {}
        }
    }
    overall
}

/// Read the subject line of HEAD.
fn read_head_commit_subject() -> Result<String> {
    let out = Command::new("git")
        .args(["log", "-1", "--pretty=%s"])
        .output()
        .context("failed to run `git log -1 --pretty=%s`")?;
    if !out.status.success() {
        bail!(
            "git log exited with status {:?}: {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let subject = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Ok(subject)
}

/// Return `true` when a GH_TOKEN or GITHUB_TOKEN environment variable is set.
fn gh_token_present() -> bool {
    std::env::var("GH_TOKEN").is_ok() || std::env::var("GITHUB_TOKEN").is_ok()
}

/// Query GitHub REST API to check if an issue exists and whether it is open.
/// Returns `(exists, open)`.
fn query_issue(issue_number: u64) -> Result<(bool, bool)> {
    let url = format!(
        "https://api.github.com/repos/EffortlessMetrics/perl-lsp-swarm/issues/{issue_number}"
    );
    let out = Command::new("gh").args(["api", &url]).output().context("failed to run `gh api`")?;

    if !out.status.success() {
        // 404 → issue doesn't exist.
        let stderr = String::from_utf8_lossy(&out.stderr);
        if stderr.contains("404") || out.status.code() == Some(1) {
            return Ok((false, false));
        }
        bail!("gh api exited with status {:?}: {}", out.status.code(), stderr);
    }

    let body: serde_json::Value =
        serde_json::from_slice(&out.stdout).context("failed to parse gh api response")?;
    let state = body["state"].as_str().unwrap_or("unknown");
    Ok((true, state == "open"))
}

/// Print a human-readable summary.
fn print_human(receipt: &TitleCheckReceipt) {
    println!("Title: {}", receipt.title);
    println!();
    for c in &receipt.checks {
        let symbol = match c.status {
            CheckStatus::Ok => "ok  ",
            CheckStatus::Warn => "WARN",
            CheckStatus::Fail => "FAIL",
            CheckStatus::Skipped => "skip",
        };
        if let Some(msg) = &c.message {
            println!("  [{symbol}] {} — {msg}", c.name);
        } else {
            println!("  [{symbol}] {}", c.name);
        }
    }
    println!();
    println!("Overall: {}", receipt.overall);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subject_supersede_detection_is_case_insensitive() {
        assert!(subject_contains_supersede("supersede previous approach"));
        assert!(subject_contains_supersede("Supersede previous approach"));
        assert!(subject_contains_supersede("SUPERSEDED by follow-up"));
        assert!(!subject_contains_supersede("follow-up implementation"));
    }

    #[test]
    fn strips_trailing_issue_reference() {
        assert_eq!(strip_issue_ref("clean up validation flow (#1234)"), "clean up validation flow");
        assert_eq!(strip_issue_ref("clean up validation flow(#1234)"), "clean up validation flow");
    }
}
