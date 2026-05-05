use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;

use chrono::Utc;
use color_eyre::eyre::{Context, Result};
use perl_lsp_ux_tests::taxonomy::{UxComponent, UxFailureClass, UxRoute, route_for_failure_class};
use regex::Regex;
use serde::Serialize;

#[allow(clippy::expect_used, reason = "static LazyLock regex with known-good pattern")]
static FAILED_TEST_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"test\s+([^\s]+)\s+\.\.\.\s+FAILED").expect("failed test regex must compile")
});
// Matches both pre-1.73 format ("panicked at 'msg', path:row:col") and
// post-1.73 format ("panicked at path:row:col:") where the location appears
// directly after "panicked at " without a quoted message.
#[allow(clippy::expect_used, reason = "static LazyLock regex with known-good pattern")]
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
pub struct UxRegressionReceipt {
    kind: &'static str,
    schema_version: u32,
    measured_at: String,
    sha: String,
    workflow: Option<String>,
    scenario_file: Option<String>,
    scenario: Option<String>,
    first_failing_test: Option<String>,
    result: String,
    failure_class: UxFailureClass,
    panic_location: Option<String>,
    canonical_repro: Option<String>,
    friendly_repro: Option<String>,
    first_failing_line: Option<String>,
    route: UxRoute,
    blocking: bool,
    merge_action: String,
    human_summary: String,
    component: Option<UxComponent>,
    run_id: Option<String>,
    attempt: Option<u32>,
    platform: Option<String>,
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
    let first_failing_test =
        lines.iter().find_map(|line| FAILED_TEST_RE.captures(line).map(|cap| cap[1].to_string()));
    let panic_location =
        lines.iter().find_map(|line| PANIC_RE.captures(line).map(|cap| cap[1].to_string()));
    let scenario = first_failing_test.as_ref().and_then(|name| scenario_from_test_name(name));
    let workflow = first_failing_test.as_ref().and_then(|name| workflow_from_test_name(name));

    let failure_class = infer_failure_class(raw);

    let canonical_repro = first_failing_test.as_ref().map(|name| {
        format!("cargo test -p perl-lsp-ux-tests {name} -- --test-threads=1 --nocapture")
    });

    let friendly_repro = first_failing_test.as_ref().map(|name| {
        // Extract just the test function name (after ::) for the shorthand command.
        let short = name.split("::").last().unwrap_or(name);
        format!("just ux-tests {short}")
    });

    let route = route_for_failure_class(failure_class);
    let result = if raw.contains("test result: ok") { "pass" } else { "fail" }.to_string();
    let blocking = result != "pass";
    let merge_action = if !blocking {
        "merge_allowed"
    } else {
        match failure_class {
            UxFailureClass::TestRace => "quarantine_or_fix_test",
            UxFailureClass::ProviderRegression => "fix_provider",
            UxFailureClass::MatrixDrift => "update_fixture_matrix",
            UxFailureClass::BaselineDrift => "update_baseline",
            UxFailureClass::Timeout => "triage_timeout",
            UxFailureClass::Infra => "fix_ci_infra",
            UxFailureClass::ServerCrash => "fix_crash",
            UxFailureClass::NewTestBug => "fix_test",
            UxFailureClass::Unknown => "triage",
            _ => "triage",
        }
    }
    .to_string();
    let human_summary = if result == "pass" {
        "UX regression passed; merge_allowed.".to_string()
    } else {
        let test = first_failing_test.as_deref().unwrap_or("unknown_test");
        let repro = canonical_repro.as_deref().unwrap_or("see ux-regression.log");
        format!("UX regression failed in {test}; classified as {failure_class:?}; repro: {repro}")
    };

    UxRegressionReceipt {
        kind: "ux_regression_receipt",
        schema_version: 1,
        measured_at: Utc::now().to_rfc3339(),
        sha: sha.unwrap_or_else(|| "unknown".to_string()),
        workflow,
        scenario_file: scenario.clone(),
        scenario,
        first_failing_test,
        result,
        failure_class,
        panic_location,
        canonical_repro,
        friendly_repro,
        first_failing_line: first_fail_line,
        route,
        blocking,
        merge_action,
        human_summary,
        component: None,
        run_id: None,
        attempt: None,
        platform: None,
    }
}

fn scenario_from_test_name(test: &str) -> Option<String> {
    let scenario = test.split("::").next()?;
    if scenario.starts_with("ux_scenario_") { Some(format!("{scenario}.rs")) } else { None }
}

fn infer_failure_class(raw: &str) -> UxFailureClass {
    let lower = raw.to_ascii_lowercase();
    if looks_like_scenario_19_race(&lower) {
        UxFailureClass::TestRace
    } else if looks_like_scenario_14_provider_regression(&lower) {
        UxFailureClass::ProviderRegression
    } else if lower.contains("fixture matrix") || lower.contains("matrix drift") {
        UxFailureClass::MatrixDrift
    } else if lower.contains("baseline") || lower.contains("snapshot") {
        UxFailureClass::BaselineDrift
    } else if lower.contains("timed out") || lower.contains("timeout") {
        UxFailureClass::Timeout
    } else if lower.contains("race") || lower.contains("flaky") {
        UxFailureClass::TestRace
    } else if lower.contains("panicked") && lower.contains("tests/ux_scenario_") {
        UxFailureClass::NewTestBug
    } else if lower.contains("no such file") || lower.contains("permission denied") {
        UxFailureClass::Infra
    } else if lower.contains("assertion failed") {
        // Check ProviderRegression before the generic panicked/ServerCrash catch-all:
        // a typical assertion failure log contains both "panicked" and "assertion failed",
        // so this branch must precede the ServerCrash arm to remain reachable.
        // Note: we do NOT match on bare "expected" because it appears as a substring of
        // unrelated words like "unexpectedly", causing false positives on ServerCrash logs.
        UxFailureClass::ProviderRegression
    } else if lower.contains("panicked") || lower.contains("server exited") {
        UxFailureClass::ServerCrash
    } else {
        UxFailureClass::Unknown
    }
}

fn looks_like_scenario_19_race(lower: &str) -> bool {
    lower.contains("scenario_19_diagnostics_clear_after_fix")
        && (lower.contains("pre-fix")
            || lower.contains("post-fix")
            || lower.contains("diagnostics race")
            || lower.contains("events:")
            || lower.contains("expected diagnostics to clear"))
}

fn looks_like_scenario_14_provider_regression(lower: &str) -> bool {
    lower.contains("ux_scenario_14_inc_conformance")
        && (lower.contains("goto-definition")
            || lower.contains("include path")
            || lower.contains("include_paths")
            || lower.contains("includepaths")
            || lower.contains("perl5lib")
            || lower.contains("usesysteminc")
            || lower.contains("@inc"))
}

fn workflow_from_test_name(test: &str) -> Option<String> {
    let workflow = test.split("::").nth(1)?;
    if workflow.is_empty() { None } else { Some(workflow.to_string()) }
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
            receipt.first_failing_test.as_deref(),
            Some("ux_scenario_19_diagnostics_lifecycle::scenario_19_diagnostics_clear_after_fix"),
            "test name should be extracted from log"
        );
        assert!(
            matches!(receipt.failure_class, UxFailureClass::NewTestBug),
            "failure_class should be NewTestBug for panicked test in ux_scenario"
        );
        assert_eq!(
            receipt.panic_location.as_deref(),
            Some("crates/perl-lsp-ux-tests/tests/ux_scenario_19_diagnostics_lifecycle.rs:102:5"),
            "panic_location should be extracted from panic line (Rust 1.73+ format)"
        );
        assert_eq!(receipt.route, UxRoute::TestFix, "race/new test bug routes to test fix");
    }

    #[test]
    fn classify_timeout_routes_to_timeout_triage() {
        let log = "running 1 test\ntest ux_scenario_01_startup::start_server ... FAILED\ntest timed out after 30s\ntest result: FAILED. 0 passed; 1 failed";
        let receipt = classify(log, Some("sha1".to_string()));
        assert!(
            matches!(receipt.failure_class, UxFailureClass::Timeout),
            "timed out log should classify as Timeout"
        );
        assert_eq!(receipt.route, UxRoute::TimeoutTriage, "Timeout routes to TimeoutTriage");
        assert_eq!(receipt.result, "fail");
        assert!(receipt.blocking);
        assert_eq!(receipt.merge_action, "triage_timeout");
    }

    #[test]
    fn classify_server_crash_routes_to_crash_fix() {
        // ServerCrash: panicked in non-ux_scenario path (e.g., the LSP server process itself).
        // Must not contain "tests/ux_scenario_" (NewTestBug) or "assertion failed" (ProviderRegression).
        let log = "running 1 test\ntest ux_scenario_02_open::open_file ... FAILED\nthread 'server' panicked at crates/perl-lsp-rs/src/provider.rs:55:9:\nserver crashed with SIGABRT\ntest result: FAILED. 0 passed; 1 failed";
        let receipt = classify(log, Some("sha2".to_string()));
        assert!(
            matches!(receipt.failure_class, UxFailureClass::ServerCrash),
            "non-ux_scenario panic should classify as ServerCrash, got {:?}",
            receipt.failure_class
        );
        assert_eq!(receipt.route, UxRoute::CrashFix, "ServerCrash routes to CrashFix");
    }

    #[test]
    fn classify_unexpected_exit_routes_to_crash_fix() {
        // "unexpectedly" contains the substring "expected", but ProviderRegression only
        // triggers on "assertion failed" — not bare "expected" — so this must classify
        // as ServerCrash, not ProviderRegression.
        let log = "running 1 test\ntest ux_scenario_03_diag::diag_test ... FAILED\nthread 'main' panicked at crates/perl-lsp-rs/src/server.rs:10:1:\nserver exited unexpectedly\ntest result: FAILED. 0 passed; 1 failed";
        let receipt = classify(log, Some("sha8".to_string()));
        assert!(
            matches!(receipt.failure_class, UxFailureClass::ServerCrash),
            "log with 'unexpectedly' (substring of 'expected') should be ServerCrash, got {:?}",
            receipt.failure_class
        );
        assert_eq!(receipt.route, UxRoute::CrashFix);
    }

    #[test]
    fn classify_matrix_drift_routes_to_fixture_update() {
        let log = "running 2 tests\ntest ux_scenario_05_matrix::check_matrix ... FAILED\nfixture matrix mismatch: expected 3 items, got 4\ntest result: FAILED. 1 passed; 1 failed";
        let receipt = classify(log, Some("sha3".to_string()));
        assert!(
            matches!(receipt.failure_class, UxFailureClass::MatrixDrift),
            "fixture matrix log should classify as MatrixDrift"
        );
        assert_eq!(receipt.route, UxRoute::FixtureUpdate, "MatrixDrift routes to FixtureUpdate");
        assert_eq!(receipt.merge_action, "update_fixture_matrix");
    }

    #[test]
    fn classify_baseline_drift_routes_to_baseline_update() {
        let log = "running 1 test\ntest ux_scenario_10_hover::hover_type ... FAILED\nbaseline snapshot mismatch\ntest result: FAILED. 0 passed; 1 failed";
        let receipt = classify(log, Some("sha4".to_string()));
        assert!(
            matches!(receipt.failure_class, UxFailureClass::BaselineDrift),
            "baseline snapshot log should classify as BaselineDrift"
        );
        assert_eq!(
            receipt.route,
            UxRoute::BaselineUpdate,
            "BaselineDrift routes to BaselineUpdate"
        );
        assert_eq!(receipt.merge_action, "update_baseline");
    }

    #[test]
    fn classify_provider_regression_routes_to_provider_fix() {
        // ProviderRegression: assertion failure without a panic in a ux_scenario_ path.
        // Must reach the ProviderRegression branch (not be swallowed by ServerCrash).
        let log = "running 1 test\ntest ux_scenario_07_completion::completions ... FAILED\nassertion failed: left == right\n  left: 3\n right: 5\ntest result: FAILED. 0 passed; 1 failed";
        let receipt = classify(log, Some("sha5".to_string()));
        assert!(
            matches!(receipt.failure_class, UxFailureClass::ProviderRegression),
            "assertion-failed log without panic-in-ux_scenario_ should classify as ProviderRegression, got {:?}",
            receipt.failure_class
        );
        assert_eq!(receipt.route, UxRoute::ProviderFix, "ProviderRegression routes to ProviderFix");
    }

    #[test]
    fn classify_unknown_routes_to_triage() {
        let log = "running 1 test\ntest ux_scenario_99_misc::misc_test ... FAILED\nsome completely unrecognized error message\ntest result: FAILED. 0 passed; 1 failed";
        let receipt = classify(log, Some("sha6".to_string()));
        assert!(
            matches!(receipt.failure_class, UxFailureClass::Unknown),
            "unrecognized log should classify as Unknown"
        );
        assert_eq!(receipt.route, UxRoute::Triage, "Unknown routes to Triage");
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
        assert!(!receipt.blocking);
        assert_eq!(receipt.merge_action, "merge_allowed");
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

    /// Dual repro completeness: for representative test names, the receipt
    /// contains both a non-empty `canonical_repro` (cargo test command) and
    /// a non-empty `friendly_repro` (just ux-tests shorthand).
    ///
    /// Feature: ux-readiness-system, Property 2: Dual repro completeness
    /// Validates: Requirements 0.4, 1.9
    #[test]
    fn dual_repro_completeness() -> Result<()> {
        let representative_tests = [
            ("ux_scenario_01_startup::start_server", "start_server"),
            ("ux_scenario_07_completion::completions", "completions"),
            (
                "ux_scenario_14_inc_conformance::scenario_14_include_path_completion_external_module",
                "scenario_14_include_path_completion_external_module",
            ),
            (
                "ux_scenario_19_diagnostics_lifecycle::scenario_19_diagnostics_clear_after_fix",
                "scenario_19_diagnostics_clear_after_fix",
            ),
        ];

        for (full_test_name, expected_short) in representative_tests {
            let log = format!(
                "running 1 test\n\
                 test {full_test_name} ... FAILED\n\
                 assertion failed: left == right\n\
                 test result: FAILED. 0 passed; 1 failed"
            );

            let receipt = classify(&log, Some("deadbeef".to_string()));

            // canonical_repro must be Some and non-empty
            let canonical = receipt.canonical_repro.as_deref().unwrap_or("");
            assert!(
                !canonical.is_empty(),
                "canonical_repro should be non-empty for test {full_test_name}"
            );
            assert!(
                canonical.contains("cargo test -p perl-lsp-ux-tests"),
                "canonical_repro should contain 'cargo test -p perl-lsp-ux-tests', got: {canonical}"
            );
            assert!(
                canonical.contains(full_test_name),
                "canonical_repro should contain the full test name, got: {canonical}"
            );

            // friendly_repro must be Some and non-empty
            let friendly = receipt.friendly_repro.as_deref().unwrap_or("");
            assert!(
                !friendly.is_empty(),
                "friendly_repro should be non-empty for test {full_test_name}"
            );
            assert!(
                friendly.contains("just ux-tests"),
                "friendly_repro should contain 'just ux-tests', got: {friendly}"
            );
            assert!(
                friendly.contains(expected_short),
                "friendly_repro should contain the short test name '{expected_short}', got: {friendly}"
            );
        }

        Ok(())
    }

    #[test]
    fn panic_re_matches_modern_rust_format() {
        // Rust 1.73+ format: "panicked at path:row:col:" with no quoted message.
        let line = "thread 'test' panicked at crates/perl-lsp-rs/src/lib.rs:42:8:";
        let cap = PANIC_RE.captures(line).expect("should match modern panic format");
        assert_eq!(&cap[1], "crates/perl-lsp-rs/src/lib.rs:42:8");
    }

    // =========================================================================
    // Scenario 14 / 19 classifier fixture tests (Task 0.4)
    // =========================================================================

    /// Scenario 14 — external module resolution via `includePaths` fails.
    ///
    /// When goto-definition returns empty for a module that should resolve via
    /// `includePaths`, the test assertion produces an "assertion failed" log.
    /// The classifier should identify this as `ProviderRegression` because the
    /// LSP provider failed to resolve a module that the configuration says
    /// should be resolvable.
    #[test]
    fn classifier_extracts_scenario_14_external_module_failure() -> Result<()> {
        // Representative log: an assertion failure from scenario_14 when an
        // external module configured via includePaths fails to resolve.
        // The log contains "assertion failed" which the classifier maps to
        // ProviderRegression (module resolution provider did not honour config).
        let log = "\
running 1 test\n\
test ux_scenario_14_inc_conformance::scenario_14_include_path_completion_external_module ... FAILED\n\
\n\
failures:\n\
\n\
---- ux_scenario_14_inc_conformance::scenario_14_include_path_completion_external_module stdout ----\n\
[conformance] mode=completion_external_module | PL701=PASS | goto-def=FAIL | hover=PASS\n\
assertion failed: left == right\n\
  left: false\n\
 right: true\n\
Expected goto-definition to resolve GreetModule from completion scenario; defs=[]\n\
\n\
failures:\n\
    ux_scenario_14_inc_conformance::scenario_14_include_path_completion_external_module\n\
\n\
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out";

        let receipt = classify(log, Some("fix14a".to_string()));

        assert!(
            matches!(receipt.failure_class, UxFailureClass::ProviderRegression),
            "scenario_14 external module failure should classify as ProviderRegression, got {:?}",
            receipt.failure_class
        );
        assert_eq!(receipt.route, UxRoute::ProviderFix, "ProviderRegression routes to ProviderFix");
        assert_eq!(
            receipt.scenario.as_deref(),
            Some("ux_scenario_14_inc_conformance.rs"),
            "scenario file should be extracted"
        );
        assert_eq!(receipt.result, "fail");
        assert!(receipt.blocking);
        assert_eq!(receipt.merge_action, "fix_provider");

        Ok(())
    }

    /// Scenario 14 — system `@INC` opt-in via `PERL5LIB` / `useSystemInc` fails.
    ///
    /// When the server cannot resolve a module via system `@INC` despite
    /// `useSystemInc: true`, the test assertion produces an "assertion failed"
    /// log. The classifier should identify this as `ProviderRegression` because
    /// the module resolution provider failed to honour the system `@INC`
    /// configuration.
    #[test]
    fn classifier_extracts_scenario_14_system_inc_opt_in_failure() -> Result<()> {
        // Representative log: an assertion failure from scenario_14 when the
        // server cannot resolve a module via system @INC despite useSystemInc
        // being enabled. The "assertion failed" content triggers ProviderRegression.
        let log = "\
running 1 test\n\
test ux_scenario_14_inc_conformance::scenario_14_system_inc ... FAILED\n\
\n\
failures:\n\
\n\
---- ux_scenario_14_inc_conformance::scenario_14_system_inc stdout ----\n\
[conformance] mode=system_inc | PL701=FAIL | goto-def=FAIL | hover=PASS\n\
assertion failed: definition should resolve SystemModule.pm via system @INC (PERL5LIB); defs=[]\n\
\n\
failures:\n\
    ux_scenario_14_inc_conformance::scenario_14_system_inc\n\
\n\
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out";

        let receipt = classify(log, Some("fix14b".to_string()));

        assert!(
            matches!(receipt.failure_class, UxFailureClass::ProviderRegression),
            "scenario_14 system @INC failure should classify as ProviderRegression, got {:?}",
            receipt.failure_class
        );
        assert_eq!(receipt.route, UxRoute::ProviderFix, "ProviderRegression routes to ProviderFix");
        assert_eq!(
            receipt.scenario.as_deref(),
            Some("ux_scenario_14_inc_conformance.rs"),
            "scenario file should be extracted"
        );
        assert_eq!(
            receipt.first_failing_test.as_deref(),
            Some("ux_scenario_14_inc_conformance::scenario_14_system_inc"),
            "test name should be extracted"
        );
        assert_eq!(receipt.result, "fail");

        Ok(())
    }

    /// Scenario 19 — diagnostics race condition during edit lifecycle.
    ///
    /// When the diagnostics clear-after-fix check fails due to a race between
    /// pre-fix and post-fix diagnostic events, the log contains "race" in the
    /// diagnostic context. The classifier should identify this as `TestRace`
    /// because the failure is a non-deterministic timing issue in the test
    /// harness, not a provider regression.
    #[test]
    fn classifier_extracts_scenario_19_diagnostics_race_failure() -> Result<()> {
        let log = "\
running 1 test\n\
test ux_scenario_19_diagnostics_lifecycle::scenario_19_diagnostics_clear_after_fix ... FAILED\n\
\n\
failures:\n\
\n\
---- ux_scenario_19_diagnostics_lifecycle::scenario_19_diagnostics_clear_after_fix stdout ----\n\
diagnostics race: pre-fix events leaked into post-fix window\n\
Expected diagnostics to clear (or no new errors) after fixing the file; \
events: [Diagnostics { uri: \"file:///tmp/ws/live.pl\", diagnostics: [{\"code\":\"PL001\"}] }]\n\
note: this is a known flaky race condition in the diagnostics drain pipeline\n\
\n\
failures:\n\
    ux_scenario_19_diagnostics_lifecycle::scenario_19_diagnostics_clear_after_fix\n\
\n\
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out";

        let receipt = classify(log, Some("fix19".to_string()));

        assert!(
            matches!(receipt.failure_class, UxFailureClass::TestRace),
            "scenario_19 diagnostics race should classify as TestRace, got {:?}",
            receipt.failure_class
        );
        assert_eq!(receipt.route, UxRoute::TestFix, "TestRace routes to TestFix");
        assert_eq!(
            receipt.scenario.as_deref(),
            Some("ux_scenario_19_diagnostics_lifecycle.rs"),
            "scenario file should be extracted"
        );
        assert_eq!(
            receipt.first_failing_test.as_deref(),
            Some("ux_scenario_19_diagnostics_lifecycle::scenario_19_diagnostics_clear_after_fix"),
            "test name should be extracted"
        );
        assert_eq!(receipt.result, "fail");
        assert!(receipt.blocking);
        assert_eq!(receipt.merge_action, "quarantine_or_fix_test");

        Ok(())
    }
}
