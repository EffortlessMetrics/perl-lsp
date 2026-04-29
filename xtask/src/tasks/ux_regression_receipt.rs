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
static PANIC_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"panicked at .*?,\s+([^:]+:\d+:\d+)").expect("panic regex must compile")
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
    let workflow = test
        .as_ref()
        .and_then(|name| name.split("::").nth(1))
        .map(std::string::ToString::to_string);
    let scenario_file = test.as_ref().and_then(|name| scenario_from_test_name(name));

    let failure_class = infer_failure_class(raw);
    let route = route_for_failure_class(&failure_class);

    let repro = test.as_ref().map(|name| {
        format!("just ux-tests {name}")
    });

    UxRegressionReceipt {
        kind: "ux_regression_receipt",
        schema_version: 1,
        measured_at: Utc::now().to_rfc3339(),
        sha: sha.unwrap_or_else(|| "unknown".to_string()),
        workflow,
        scenario_file,
        test,
        result: if raw.contains("test result: ok") { "pass" } else { "fail" }.to_string(),
        failure_class,
        panic_location,
        repro,
        first_failing_line: first_fail_line,
        route: route.to_string(),
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
    } else if lower.contains("panicked") || lower.contains("server exited") {
        FailureClass::ServerCrash
    } else if lower.contains("no such file") || lower.contains("permission denied") {
        FailureClass::Infra
    } else if lower.contains("assertion failed") || lower.contains("expected") {
        FailureClass::ProviderRegression
    } else {
        FailureClass::Unknown
    }
}

fn route_for_failure_class(class: &FailureClass) -> &'static str {
    match class {
        FailureClass::MatrixDrift | FailureClass::BaselineDrift => "needs-fixture-update",
        FailureClass::TestRace | FailureClass::NewTestBug => "needs-test-fix",
        FailureClass::ProviderRegression | FailureClass::ServerCrash => "needs-provider-fix",
        FailureClass::Timeout | FailureClass::Infra => "needs-ci-investigation",
        FailureClass::Unknown => "needs-triage",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_extracts_structured_fields() {
        let log = "running 1 test\ntest ux_scenario_19_diagnostics_lifecycle::scenario_19_diagnostics_clear_after_fix ... FAILED\nthread 'x' panicked at 'boom', crates/perl-lsp-ux-tests/tests/ux_scenario_19_diagnostics_lifecycle.rs:102:5\ntest result: FAILED. 0 passed; 1 failed";
        let receipt = classify(log, Some("abc123".to_string()));
        assert_eq!(receipt.sha, "abc123", "sha should match input");
        assert_eq!(
            receipt.scenario_file.as_deref(),
            Some("ux_scenario_19_diagnostics_lifecycle.rs"),
            "scenario should be extracted from test name"
        );
        assert_eq!(
            receipt.workflow.as_deref(),
            Some("scenario_19_diagnostics_clear_after_fix"),
            "workflow should be the test fn name"
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
            "panic_location should be extracted from panic line"
        );
        assert_eq!(receipt.route, "needs-test-fix");
    }
}
