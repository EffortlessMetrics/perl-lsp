use std::fs;
use std::path::PathBuf;

use color_eyre::eyre::{Context, Result};
use regex::Regex;
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct UxRegressionReceiptConfig {
    pub input: PathBuf,
    pub output: Option<PathBuf>,
    pub sha: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FailureClass {
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
    pub kind: &'static str,
    pub schema_version: u8,
    pub sha: String,
    pub scenario: Option<String>,
    pub test: Option<String>,
    pub result: &'static str,
    pub failure_class: FailureClass,
    pub panic_location: Option<String>,
    pub repro: Option<String>,
    pub first_failing_line: Option<String>,
}

pub fn run(config: UxRegressionReceiptConfig) -> Result<()> {
    let raw = fs::read_to_string(&config.input)
        .with_context(|| format!("reading {}", config.input.display()))?;
    let receipt = classify_log(&raw, config.sha);
    let rendered = format!("{}\n", serde_json::to_string_pretty(&receipt)?);

    if let Some(output) = config.output {
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
        fs::write(&output, rendered).with_context(|| format!("writing {}", output.display()))?;
        println!("Wrote UX regression receipt: {}", output.display());
    } else {
        print!("{rendered}");
    }

    Ok(())
}

fn classify_log(raw: &str, sha: Option<String>) -> UxRegressionReceipt {
    let test = find_capture(raw, r"----\s+([^\s]+)\s+stdout\s+----");
    let panic_location = find_capture(raw, r"panicked at .+?,\s+([^\n]+:\d+:\d+)");
    let first_failing_line = find_capture(raw, r"assertion failed:\s+([^\n]+)");
    let scenario = test
        .as_deref()
        .and_then(|name| name.split("::").next())
        .map(std::string::ToString::to_string);

    let failure_class = classify_failure(raw, &panic_location, test.as_deref());
    let repro = test.as_ref().map(|name| format!("just ux-tests {name}"));

    UxRegressionReceipt {
        kind: "ux_regression_receipt",
        schema_version: 1,
        sha: sha.unwrap_or_else(|| "unknown".to_string()),
        scenario,
        test,
        result: "fail",
        failure_class,
        panic_location,
        repro,
        first_failing_line,
    }
}

fn classify_failure(
    raw: &str,
    panic_location: &Option<String>,
    test_name: Option<&str>,
) -> FailureClass {
    let lower = raw.to_ascii_lowercase();
    if lower.contains("editor_ux_fixture_matrix") || lower.contains("covers_all_scenarios") {
        return FailureClass::MatrixDrift;
    }
    if lower.contains("baseline") && lower.contains("drift") {
        return FailureClass::BaselineDrift;
    }
    if lower.contains("timed out") || lower.contains("timeout") {
        return FailureClass::Timeout;
    }
    if lower.contains("connection reset") || lower.contains("broken pipe") {
        return FailureClass::Infra;
    }
    if lower.contains("panic") && lower.contains("perl-lsp-rs") {
        return FailureClass::ServerCrash;
    }
    if let Some(name) = test_name
        && name.contains("scenario_19_diagnostics_clear_after_fix")
    {
        return FailureClass::TestRace;
    }
    if let Some(location) = panic_location
        && location.contains("crates/perl-lsp-ux-tests/tests/")
    {
        return FailureClass::NewTestBug;
    }
    if panic_location.is_some() {
        return FailureClass::ProviderRegression;
    }
    FailureClass::Unknown
}

fn find_capture(haystack: &str, pattern: &str) -> Option<String> {
    let re = Regex::new(pattern).ok()?;
    re.captures(haystack).and_then(|caps| caps.get(1)).map(|m| m.as_str().trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::{FailureClass, classify_log};

    #[test]
    fn classifies_matrix_drift() {
        let raw = "test editor_ux_fixture_matrix_covers_all_scenarios ... FAILED";
        let receipt = classify_log(raw, Some("abc".to_string()));
        assert_eq!(receipt.failure_class, FailureClass::MatrixDrift);
    }

    #[test]
    fn classifies_scenario_19_race() {
        let raw = "---- scenario_19_diagnostics_clear_after_fix stdout ----\nthread 'x' panicked at 'oops', crates/perl-lsp-ux-tests/tests/ux_scenario_19_diagnostics_lifecycle.rs:102:9\nassertion failed: cleared";
        let receipt = classify_log(raw, Some("abc".to_string()));
        assert_eq!(receipt.failure_class, FailureClass::TestRace);
        assert_eq!(receipt.scenario.as_deref(), Some("scenario_19_diagnostics_clear_after_fix"));
    }
}
