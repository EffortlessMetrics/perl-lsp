//! DAP subsystem status generator.
//!
//! Owns DAP test count discovery and dap.md generation.

use std::fs;
use std::path::Path;

use color_eyre::eyre::Result;
use serde::Deserialize;

use super::replace_block;

// ---------------------------------------------------------------------------
// DAP test counts struct
// ---------------------------------------------------------------------------

/// Counts of DAP tests discovered from source files.
pub(super) struct DapTestCounts {
    /// Number of `[[test]]` integration test targets in `crates/perl-dap/Cargo.toml`.
    pub integration_test_targets: usize,
    /// Number of `#[test]` functions found across all `perl-dap-*` test files.
    pub scorecard_fixtures: usize,
}

#[derive(Debug, Deserialize)]
struct DapScorecardReceipt {
    perl_available: bool,
    launch_rows: Vec<DapMetricRow>,
    session_rows: Vec<DapMetricRow>,
}

#[derive(Debug, Deserialize)]
struct DapMetricRow {
    metric: String,
    value: String,
    target: String,
    status: String,
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

fn read_dap_scorecard_receipt(root: &Path) -> Option<DapScorecardReceipt> {
    let receipt_path = root.join("target/receipts/dap-scorecard.json");
    let raw = fs::read_to_string(receipt_path).ok()?;
    serde_json::from_str::<DapScorecardReceipt>(&raw).ok()
}

fn render_metric_rows(rows: &[DapMetricRow]) -> String {
    let mut output = String::from("| Metric | Value | Target | Status |\n|---|---|---|---|");
    for row in rows {
        output.push_str(&format!(
            "\n| {} | {} | {} | {} |",
            row.metric, row.value, row.target, row.status
        ));
    }
    output
}

// ---------------------------------------------------------------------------
// Generator
// ---------------------------------------------------------------------------

/// Regenerate the marker blocks in `docs/project/status/dap.md`.
pub(super) fn generate_dap_status(counts: &DapTestCounts, original: &str) -> Result<String> {
    let scorecard = read_dap_scorecard_receipt(&crate::utils::project_root()?);
    let launch_table = scorecard.as_ref().map_or_else(
        || {
            render_metric_rows(&[DapMetricRow {
                metric: "Launch success rate".to_string(),
                value: "UNVERIFIED".to_string(),
                target: "Run dap_scorecard_harness".to_string(),
                status: "SKIP".to_string(),
            }])
        },
        |receipt| render_metric_rows(&receipt.launch_rows),
    );
    let session_table = scorecard.as_ref().map_or_else(
        || {
            render_metric_rows(&[DapMetricRow {
                metric: "Session quality metrics".to_string(),
                value: "UNVERIFIED".to_string(),
                target: "Run dap_scorecard_harness".to_string(),
                status: "SKIP".to_string(),
            }])
        },
        |receipt| {
            if receipt.perl_available {
                render_metric_rows(&receipt.session_rows)
            } else {
                render_metric_rows(&[DapMetricRow {
                    metric: "Session quality metrics".to_string(),
                    value: "SKIP".to_string(),
                    target: "perl on PATH".to_string(),
                    status: "SKIP".to_string(),
                }])
            }
        },
    );

    let test_counts_table = format!(
        "| Suite | Count |\n\
         |---|---|\n\
         | Integration tests (`perl-dap`) | {} test targets |\n\
         | Scorecard fixtures | {} |",
        counts.integration_test_targets, counts.scorecard_fixtures,
    );

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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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
                        old content\n\
                        <!-- END: DAP_TEST_COUNTS -->\n\
                        tail\n";
        let result = generate_dap_status(&counts, template)?;
        assert!(result.contains("20 test targets"), "expected '20 test targets' in output");
        assert!(result.contains("| Scorecard fixtures | 5 |"), "expected scorecard fixture count");
        assert!(result.contains("tail"), "suffix text should be preserved");
        Ok(())
    }
}
