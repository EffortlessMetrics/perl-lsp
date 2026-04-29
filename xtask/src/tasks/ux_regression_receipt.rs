use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;

use chrono::Utc;
use color_eyre::eyre::{Context, Result};
use regex::Regex;
use serde::Serialize;

static FAILED_TEST_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"test\s+([^\s]+)\s+\.\.\.\s+FAILED").expect("failed test regex must compile")
});
// Matches both pre-1.73 format ("panicked at 'msg', path:row:col") and
// post-1.73 format ("panicked at path:row:col:") where the location appears
// directly after "panicked at " without a quoted message.
static PANIC_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"panicked at (?:'[^']*',\s*)?([a-zA-Z][^:\s][^:]*:\d+:\d+)")
        .expect("panic regex must compile")
});

#[derive(Debug, Clone)]
pub struct UxRegressionReceiptConfig {
    pub input: PathBuf,
    pub receipt: Option<PathBuf>,
    pub sha: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
enum FailureClass {
    MatrixDrift,
    BaselineDrift,
    TestRace,
    NewTestBug,
    ProviderRegression,
    ServerCrash,
    Timeout,
    Infra,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct UxRegressionReceipt {
    kind: &'static str,
    schema_version: u32,
    measured_at: String,
    sha: String,
    workflow: Option<String>,
    scenario_file: Option<String>,
    scenario: Option<String>,
    test: Option<String>,
    result: String,
    failure_class: FailureClass,
    panic_location: Option<String>,
    repro: Option<String>,
    first_failing_line: Option<String>,
    route: String,
}

pub fn run(config: UxRegressionReceiptConfig) -> Result<()> {
    let raw = fs::read_to_string(&config.input)
        .with_context(|| format!("reading {}", config.input.display()))?;
    let receipt = classify(&raw, config.sha);
    let payload = serde_json::to_string_pretty(&receipt)?;

    if let Some(path) = config.receipt {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
        fs::write(&path, format!("{payload}\n"))
            .with_context(|| format!("writing {}", path.display()))?;
        println!("Wrote UX regression receipt: {}", path.display());
    } else {
        println!("{payload}");
    }

    Ok(())
}

fn classify(raw: &str, sha: Option<String>) -> UxRegressionReceipt {
    let lines: Vec<&str> = raw.lines().collect();
    let first_fail_line =
        lines.iter().find(|line| line.contains("FAILED")).map(|line| (*line).trim().to_string());
    let test =
        lines.iter().find_map(|line| FAILED_TEST_RE.captures(line).map(|cap| cap[1].to_string()));
    let panic_location =
        lines.iter().find_map(|line| PANIC_RE.captures(line).map(|cap| cap[1].to_string()));
    let scenario = test.as_ref().and_then(|name| scenario_from_test_name(name));
    let workflow = test.as_ref().and_then(|name| workflow_from_test_name(name));

    let failure_class = infer_failure_class(raw);

    let repro = test.as_ref().map(|name| {
        format!("cargo test -p perl-lsp-ux-tests {name} -- --test-threads=1 --nocapture")
    });

    let route = route_for_class(&failure_class).to_string();

    UxRegressionReceipt {
        kind: "ux_regression_receipt",
        schema_version: 1,
        measured_at: Utc::now().to_rfc3339(),
        sha: sha.unwrap_or_else(|| "unknown".to_string()),
        workflow,
        scenario_file: scenario.clone(),
        scenario,
        test,
        result: if raw.contains("test result: ok") { "pass" } else { "fail" }.to_string(),
        failure_class,
        panic_location,
        repro,
        first_failing_line: first_fail_line,
        route,
    }
}

fn scenario_from_test_name(test: &str) -> Option<String> {
    let scenario = test.split("::").next()?;
    if scenario.starts_with("ux_scenario_") { Some(format!("{scenario}.rs")) } else { None }
}

fn infer_failure_class(raw: &str) -> FailureClass {
    let lower = raw.to_ascii_lowercase();
    if lower.contains("fixture matrix") || lower.contains("matrix drift") {
        FailureClass::MatrixDrift
    } else if lower.contains("baseline") || lower.contains("snapshot") {
        FailureClass::BaselineDrift
    } else if lower.contains("timed out") || lower.contains("timeout") {
        FailureClass::Timeout
    } else if lower.contains("race") || lower.contains("flaky") {
        FailureClass::TestRace
    } else if lower.contains("panicked") && lower.contains("tests/ux_scenario_") {
        FailureClass::NewTestBug
    } else if lower.contains("no such file") || lower.contains("permission denied") {
        FailureClass::Infra
    } else if lower.contains("assertion failed") {
        // Check ProviderRegression before the generic panicked/ServerCrash catch-all:
        // a typical assertion failure log contains both "panicked" and "assertion failed",
        // so this branch must precede the ServerCrash arm to remain reachable.
        // Note: we do NOT match on bare "expected" because it appears as a substring of
        // unrelated words like "unexpectedly", causing false positives on ServerCrash logs.
        FailureClass::ProviderRegression
    } else if lower.contains("panicked") || lower.contains("server exited") {
        FailureClass::ServerCrash
    } else {
        FailureClass::Unknown
    }
}

fn workflow_from_test_name(test: &str) -> Option<String> {
    let workflow = test.split("::").nth(1)?;
    if workflow.is_empty() { None } else { Some(workflow.to_string()) }
}

fn route_for_class(class: &FailureClass) -> &'static str {
    match class {
        FailureClass::MatrixDrift | FailureClass::BaselineDrift => "needs-fixture-fix",
        FailureClass::TestRace | FailureClass::NewTestBug => "needs-test-fix",
        FailureClass::ProviderRegression | FailureClass::ServerCrash => "needs-provider-fix",
        FailureClass::Timeout | FailureClass::Infra => "needs-ci-fix",
        FailureClass::Unknown => "needs-triage",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_extracts_structured_fields() {
        // Uses the Rust 1.73+ panic format: "panicked at path:row:col:" (no quoted message).
        // The project toolchain is 1.92, so this is the format actual test output uses.
        let log = "running 1 test\ntest ux_scenario_19_diagnostics_lifecycle::scenario_19_diagnostics_clear_after_fix ... FAILED\nthread 'x' panicked at crates/perl-lsp-ux-tests/tests/ux_scenario_19_diagnostics_lifecycle.rs:102:5:\nboom\ntest result: FAILED. 0 passed; 1 failed";
        let receipt = classify(log, Some("abc123".to_string()));
        assert_eq!(receipt.sha, "abc123", "sha should match input");
        assert_eq!(
            receipt.scenario.as_deref(),
            Some("ux_scenario_19_diagnostics_lifecycle.rs"),
            "scenario should be extracted from test name"
        );
        assert_eq!(
            receipt.workflow.as_deref(),
            Some("scenario_19_diagnostics_clear_after_fix"),
            "workflow should map to test function name"
        );
        assert_eq!(
            receipt.test.as_deref(),
            Some("ux_scenario_19_diagnostics_lifecycle::scenario_19_diagnostics_clear_after_fix"),
            "test name should be extracted from log"
        );
        assert!(
            matches!(receipt.failure_class, FailureClass::NewTestBug),
            "failure_class should be NewTestBug for panicked test in ux_scenario"
        );
        assert_eq!(
            receipt.panic_location.as_deref(),
            Some("crates/perl-lsp-ux-tests/tests/ux_scenario_19_diagnostics_lifecycle.rs:102:5"),
            "panic_location should be extracted from panic line (Rust 1.73+ format)"
        );
        assert_eq!(receipt.route, "needs-test-fix", "race/new test bug routes to test fix");
    }

    #[test]
    fn classify_timeout_routes_to_ci_fix() {
        let log = "running 1 test\ntest ux_scenario_01_startup::start_server ... FAILED\ntest timed out after 30s\ntest result: FAILED. 0 passed; 1 failed";
        let receipt = classify(log, Some("sha1".to_string()));
        assert!(
            matches!(receipt.failure_class, FailureClass::Timeout),
            "timed out log should classify as Timeout"
        );
        assert_eq!(receipt.route, "needs-ci-fix", "Timeout routes to needs-ci-fix");
        assert_eq!(receipt.result, "fail");
    }

    #[test]
    fn classify_server_crash_routes_to_provider_fix() {
        // ServerCrash: panicked in non-ux_scenario path (e.g., the LSP server process itself).
        // Must not contain "tests/ux_scenario_" (NewTestBug) or "assertion failed" (ProviderRegression).
        let log = "running 1 test\ntest ux_scenario_02_open::open_file ... FAILED\nthread 'server' panicked at crates/perl-lsp-rs/src/provider.rs:55:9:\nserver crashed with SIGABRT\ntest result: FAILED. 0 passed; 1 failed";
        let receipt = classify(log, Some("sha2".to_string()));
        assert!(
            matches!(receipt.failure_class, FailureClass::ServerCrash),
            "non-ux_scenario panic should classify as ServerCrash, got {:?}",
            receipt.failure_class
        );
        assert_eq!(receipt.route, "needs-provider-fix", "ServerCrash routes to needs-provider-fix");
    }

    #[test]
    fn classify_server_exited_unexpectedly_is_server_crash_not_provider() {
        // "unexpectedly" contains the substring "expected", but ProviderRegression only
        // triggers on "assertion failed" — not bare "expected" — so this must classify
        // as ServerCrash, not ProviderRegression.
        let log = "running 1 test\ntest ux_scenario_03_diag::diag_test ... FAILED\nthread 'main' panicked at crates/perl-lsp-rs/src/server.rs:10:1:\nserver exited unexpectedly\ntest result: FAILED. 0 passed; 1 failed";
        let receipt = classify(log, Some("sha8".to_string()));
        assert!(
            matches!(receipt.failure_class, FailureClass::ServerCrash),
            "log with 'unexpectedly' (substring of 'expected') should be ServerCrash, got {:?}",
            receipt.failure_class
        );
        assert_eq!(receipt.route, "needs-provider-fix");
    }

    #[test]
    fn classify_matrix_drift_routes_to_fixture_fix() {
        let log = "running 2 tests\ntest ux_scenario_05_matrix::check_matrix ... FAILED\nfixture matrix mismatch: expected 3 items, got 4\ntest result: FAILED. 1 passed; 1 failed";
        let receipt = classify(log, Some("sha3".to_string()));
        assert!(
            matches!(receipt.failure_class, FailureClass::MatrixDrift),
            "fixture matrix log should classify as MatrixDrift"
        );
        assert_eq!(receipt.route, "needs-fixture-fix", "MatrixDrift routes to needs-fixture-fix");
    }

    #[test]
    fn classify_baseline_drift_routes_to_fixture_fix() {
        let log = "running 1 test\ntest ux_scenario_10_hover::hover_type ... FAILED\nbaseline snapshot mismatch\ntest result: FAILED. 0 passed; 1 failed";
        let receipt = classify(log, Some("sha4".to_string()));
        assert!(
            matches!(receipt.failure_class, FailureClass::BaselineDrift),
            "baseline snapshot log should classify as BaselineDrift"
        );
        assert_eq!(receipt.route, "needs-fixture-fix", "BaselineDrift routes to needs-fixture-fix");
    }

    #[test]
    fn classify_provider_regression_routes_to_provider_fix() {
        // ProviderRegression: assertion failure without a panic in a ux_scenario_ path.
        // Must reach the ProviderRegression branch (not be swallowed by ServerCrash).
        let log = "running 1 test\ntest ux_scenario_07_completion::completions ... FAILED\nassertion failed: left == right\n  left: 3\n right: 5\ntest result: FAILED. 0 passed; 1 failed";
        let receipt = classify(log, Some("sha5".to_string()));
        assert!(
            matches!(receipt.failure_class, FailureClass::ProviderRegression),
            "assertion-failed log without panic-in-ux_scenario_ should classify as ProviderRegression, got {:?}",
            receipt.failure_class
        );
        assert_eq!(
            receipt.route, "needs-provider-fix",
            "ProviderRegression routes to needs-provider-fix"
        );
    }

    #[test]
    fn classify_unknown_routes_to_triage() {
        let log = "running 1 test\ntest ux_scenario_99_misc::misc_test ... FAILED\nsome completely unrecognized error message\ntest result: FAILED. 0 passed; 1 failed";
        let receipt = classify(log, Some("sha6".to_string()));
        assert!(
            matches!(receipt.failure_class, FailureClass::Unknown),
            "unrecognized log should classify as Unknown"
        );
        assert_eq!(receipt.route, "needs-triage", "Unknown routes to needs-triage");
    }

    #[test]
    fn classify_sha_unknown_when_none() {
        let log = "test result: FAILED. 0 passed; 1 failed";
        let receipt = classify(log, None);
        assert_eq!(receipt.sha, "unknown", "None sha should produce 'unknown' in receipt");
    }

    #[test]
    fn classify_result_pass_on_ok_output() {
        let log = "running 5 tests\ntest result: ok. 5 passed; 0 failed";
        let receipt = classify(log, Some("sha7".to_string()));
        assert_eq!(receipt.result, "pass", "log with 'test result: ok' should produce result=pass");
    }

    #[test]
    fn workflow_from_test_name_returns_none_for_bare_name() {
        // A test name with no "::" has no workflow segment.
        assert_eq!(workflow_from_test_name("bare_test"), None);
    }

    #[test]
    fn scenario_from_test_name_returns_none_for_non_ux_prefix() {
        // Module not prefixed with "ux_scenario_" should not produce a scenario.
        assert_eq!(scenario_from_test_name("other_module::some_test"), None);
    }

    #[test]
    fn panic_re_matches_modern_rust_format() {
        // Rust 1.73+ format: "panicked at path:row:col:" with no quoted message.
        let line = "thread 'test' panicked at crates/perl-lsp-rs/src/lib.rs:42:8:";
        let cap = PANIC_RE.captures(line).expect("should match modern panic format");
        assert_eq!(&cap[1], "crates/perl-lsp-rs/src/lib.rs:42:8");
    }
}
