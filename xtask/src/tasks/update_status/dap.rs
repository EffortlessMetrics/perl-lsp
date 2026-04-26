//! DAP subsystem status generator.
//!
//! Owns DAP test count discovery and dap.md generation.

use std::fs;
use std::path::Path;

use color_eyre::eyre::Result;
use serde::Deserialize;

use super::replace_block;

const RECEIPT_PATH: &str = "target/dap_scorecard_receipt.json";

/// Counts of DAP tests discovered from source files.
pub(super) struct DapTestCounts {
    /// Number of `[[test]]` integration test targets in `crates/perl-dap/Cargo.toml`.
    pub integration_test_targets: usize,
    /// Number of `#[test]` functions found across all `perl-dap-*` test files.
    pub scorecard_fixtures: usize,
}

#[derive(Debug, Deserialize)]
struct ScorecardReceipt {
    perl_available: bool,
    launch: RateMetric,
    attach: RateMetric,
    variables: BinaryMetric,
    evaluate: BinaryMetric,
    deep_pagination: BinaryMetric,
    memory: BinaryMetric,
}

#[derive(Debug, Deserialize)]
struct RateMetric {
    passed: usize,
    total: usize,
    threshold_pct: u8,
    p50_ms: Option<u128>,
    p95_ms: Option<u128>,
}

#[derive(Debug, Deserialize)]
struct BinaryMetric {
    status: String,
    detail: String,
}

/// Count DAP test targets and scorecard fixtures without running cargo.
pub(super) fn count_dap_tests(root: &Path) -> DapTestCounts {
    let cargo_toml_path = root.join("crates/perl-dap/Cargo.toml");
    let integration_test_targets = fs::read_to_string(&cargo_toml_path)
        .map(|content| content.matches("[[test]]").count())
        .unwrap_or(0);

    let fixture_dir = root.join("crates/perl-dap/tests/fixtures");
    let scorecard_fixtures = fs::read_dir(&fixture_dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path().extension().and_then(|s| s.to_str()) == Some("pl")
                        && !e
                            .file_name()
                            .to_string_lossy()
                            .starts_with("breakpoints_file_boundaries")
                        && !e.file_name().to_string_lossy().starts_with("breakpoints_comments")
                        && !e.file_name().to_string_lossy().starts_with("breakpoints_heredocs")
                        && !e.file_name().to_string_lossy().starts_with("breakpoints_multiline")
                        && !e.file_name().to_string_lossy().starts_with("breakpoints_pod")
                })
                .count()
        })
        .unwrap_or(0);

    DapTestCounts { integration_test_targets, scorecard_fixtures }
}

fn read_receipt(root: &Path) -> Option<ScorecardReceipt> {
    let path = root.join(RECEIPT_PATH);
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn format_rate(criterion: &RateMetric) -> String {
    if criterion.total == 0 {
        return "SKIP".to_string();
    }
    let pct = (criterion.passed * 100) / criterion.total;
    format!("{}/{} ({} %)", criterion.passed, criterion.total, pct)
}

fn status_for_rate(criterion: &RateMetric) -> &'static str {
    if criterion.total == 0 {
        return "SKIP";
    }
    let pct = (criterion.passed * 100) / criterion.total;
    if pct >= usize::from(criterion.threshold_pct) { "PASS" } else { "FAIL" }
}

fn launch_table_from_receipt(receipt: Option<&ScorecardReceipt>) -> String {
    let Some(receipt) = receipt else {
        return "| Metric | Value | Target | Status |\n\
                |---|---|---|---|\n\
                | Launch success rate | receipt missing (`cargo test -p perl-dap --test dap_scorecard_harness -- --nocapture`) | ≥ 80 % | SKIP |\n\
                | Fixtures tested | hello, loops, eval, args, begin_end | 5 | — |\n\
                | cold_launch_p50 | — | ≤ 2 000 ms | SKIP |\n\
                | cold_launch_p95 | — | ≤ 5 000 ms | SKIP |"
            .to_string();
    };

    let p50 = receipt.launch.p50_ms.map(|ms| format!("{ms} ms")).unwrap_or_else(|| "—".to_string());
    let p95 = receipt.launch.p95_ms.map(|ms| format!("{ms} ms")).unwrap_or_else(|| "—".to_string());

    let p50_status =
        receipt.launch.p50_ms.map_or("SKIP", |ms| if ms <= 2_000 { "PASS" } else { "FAIL" });
    let p95_status =
        receipt.launch.p95_ms.map_or("SKIP", |ms| if ms <= 5_000 { "PASS" } else { "FAIL" });

    format!(
        "| Metric | Value | Target | Status |\n\
         |---|---|---|---|\n\
         | Launch success rate | {} | ≥ {} % | {} |\n\
         | Fixtures tested | hello, loops, eval, args, begin_end | 5 | — |\n\
         | cold_launch_p50 | {} | ≤ 2 000 ms | {} |\n\
         | cold_launch_p95 | {} | ≤ 5 000 ms | {} |",
        format_rate(&receipt.launch),
        receipt.launch.threshold_pct,
        status_for_rate(&receipt.launch),
        p50,
        p50_status,
        p95,
        p95_status,
    )
}

fn session_table_from_receipt(receipt: Option<&ScorecardReceipt>) -> String {
    let Some(receipt) = receipt else {
        return "| Metric | Value | Target | Status |\n\
                |---|---|---|---|\n\
                | Attach success rate (TCP loopback) | receipt missing (`cargo test -p perl-dap --test dap_scorecard_harness -- --nocapture`) | ≥ 80 % | SKIP |\n\
                | Variables pane correctness (real session) | receipt missing | expected named variables in scope | SKIP |\n\
                | Evaluate correctness (real session) | receipt missing | evaluate($x + 1) => 42 | SKIP |\n\
                | Deep truncation/pagination correctness | receipt missing | page [250..274] over @big | SKIP |\n\
                | Memory footprint baseline (portable proxy) | receipt missing | best-effort baseline | SKIP |"
            .to_string();
    };

    let availability_note = if receipt.perl_available { "" } else { " (perl unavailable)" };

    format!(
        "| Metric | Value | Target | Status |\n\
         |---|---|---|---|\n\
         | Attach success rate (TCP loopback) | {}{} | ≥ {} % | {} |\n\
         | Variables pane correctness (real session) | {} | expected named variables in scope | {} |\n\
         | Evaluate correctness (real session) | {} | evaluate($x + 1) => 42 | {} |\n\
         | Deep truncation/pagination correctness | {} | page [250..274] over @big | {} |\n\
         | Memory footprint baseline (portable proxy) | {} | best-effort baseline | {} |",
        format_rate(&receipt.attach),
        availability_note,
        receipt.attach.threshold_pct,
        status_for_rate(&receipt.attach),
        receipt.variables.detail,
        receipt.variables.status,
        receipt.evaluate.detail,
        receipt.evaluate.status,
        receipt.deep_pagination.detail,
        receipt.deep_pagination.status,
        receipt.memory.detail,
        receipt.memory.status,
    )
}

/// Regenerate the marker blocks in `docs/project/status/dap.md`.
pub(super) fn generate_dap_status(
    root: &Path,
    counts: &DapTestCounts,
    original: &str,
) -> Result<String> {
    let test_counts_table = format!(
        "| Suite | Count |\n\
         |---|---|\n\
         | Integration tests (`perl-dap`) | {} test targets |\n\
         | Scorecard fixtures | {} |",
        counts.integration_test_targets, counts.scorecard_fixtures,
    );

    let receipt = read_receipt(root);
    let launch_table = launch_table_from_receipt(receipt.as_ref());
    let session_table = session_table_from_receipt(receipt.as_ref());

    let mut text = original.to_string();
    text = replace_block(
        &text,
        "<!-- BEGIN: DAP_LAUNCH_SCORECARD -->",
        "<!-- END: DAP_LAUNCH_SCORECARD -->",
        &launch_table,
    )?;
    text = replace_block(
        &text,
        "<!-- BEGIN: DAP_SESSION_SCORECARD -->",
        "<!-- END: DAP_SESSION_SCORECARD -->",
        &session_table,
    )?;
    text = replace_block(
        &text,
        "<!-- BEGIN: DAP_TEST_COUNTS -->",
        "<!-- END: DAP_TEST_COUNTS -->",
        &test_counts_table,
    )?;
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::eyre::Result;

    #[test]
    fn test_count_dap_tests() -> Result<()> {
        let root = crate::utils::project_root()?;
        let counts = count_dap_tests(&root);
        assert!(
            counts.integration_test_targets >= 1,
            "expected at least 1 [[test]] target in perl-dap/Cargo.toml, got {}",
            counts.integration_test_targets
        );
        assert_eq!(
            counts.scorecard_fixtures, 5,
            "expected 5 scorecard fixtures (hello, loops, eval, args, breakpoints_begin_end), got {}",
            counts.scorecard_fixtures
        );
        Ok(())
    }

    #[test]
    fn test_generate_dap_status_roundtrip() -> Result<()> {
        let counts = DapTestCounts { integration_test_targets: 20, scorecard_fixtures: 5 };
        let template = "# DAP\n\
                        <!-- BEGIN: DAP_LAUNCH_SCORECARD -->\n\
                        old launch\n\
                        <!-- END: DAP_LAUNCH_SCORECARD -->\n\
                        <!-- BEGIN: DAP_SESSION_SCORECARD -->\n\
                        old session\n\
                        <!-- END: DAP_SESSION_SCORECARD -->\n\
                        <!-- BEGIN: DAP_TEST_COUNTS -->\n\
                        old counts\n\
                        <!-- END: DAP_TEST_COUNTS -->\n\
                        tail\n";
        let root = crate::utils::project_root()?;
        let result = generate_dap_status(&root, &counts, template)?;
        assert!(result.contains("20 test targets"), "expected '20 test targets' in output");
        assert!(result.contains("| Scorecard fixtures | 5 |"), "expected scorecard fixture count");
        assert!(
            result.contains("Attach success rate (TCP loopback)"),
            "expected session scorecard block"
        );
        assert!(result.contains("tail"), "suffix text should be preserved");
        Ok(())
    }
}
