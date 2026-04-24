//! DAP subsystem status generator.
//!
//! Owns DAP test count discovery and dap.md generation.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::Duration;

use color_eyre::eyre::Result;

use super::{replace_block, run_cmd};

/// Counts of DAP tests discovered from source files.
pub(super) struct DapTestCounts {
    /// Number of `[[test]]` integration test targets in `crates/perl-dap/Cargo.toml`.
    pub integration_test_targets: usize,
    /// Number of scorecard fixtures (`*.pl`) in `crates/perl-dap/tests/fixtures`.
    pub scorecard_fixtures: usize,
}

pub(super) struct DapScorecardMetric {
    pub metric: String,
    pub value: String,
    pub target: String,
    pub status: String,
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

pub(super) fn collect_scorecard_metrics(root: &Path) -> Vec<DapScorecardMetric> {
    let output = run_cmd(
        root,
        &[
            "cargo",
            "test",
            "-p",
            "perl-dap",
            "--test",
            "dap_scorecard_harness",
            "--",
            "--nocapture",
        ],
        Duration::from_secs(180),
    );

    let mut by_metric: BTreeMap<String, DapScorecardMetric> = BTreeMap::new();

    for line in output.lines() {
        if !line.starts_with("DAP_SCORECARD_ROW|") {
            continue;
        }
        let parts: Vec<&str> = line.splitn(5, '|').collect();
        if parts.len() != 5 {
            continue;
        }
        by_metric.insert(
            parts[1].to_string(),
            DapScorecardMetric {
                metric: parts[1].to_string(),
                value: parts[2].to_string(),
                target: parts[3].to_string(),
                status: parts[4].to_string(),
            },
        );
    }

    by_metric.into_values().collect()
}

fn metric_label(metric: &str) -> &str {
    match metric {
        "launch_success_rate" => "Launch success rate",
        "cold_launch_p50" => "cold_launch_p50",
        "cold_launch_p95" => "cold_launch_p95",
        "attach_success_rate" => "Attach success rate",
        "variables_session_correctness" => "Variables pane correctness (session)",
        "evaluate_session_correctness" => "Evaluate correctness (session)",
        "deep_pagination_correctness" => "Deep truncation/pagination correctness",
        "memory_baseline_proxy" => "Memory footprint baseline proxy",
        _ => metric,
    }
}

fn display_value(metric: &DapScorecardMetric) -> String {
    match metric.metric.as_str() {
        "cold_launch_p50" | "cold_launch_p95" => "measured (harness output)".to_string(),
        "memory_baseline_proxy" if metric.status == "INFO" => {
            "best-effort sample captured".to_string()
        }
        _ => metric.value.clone(),
    }
}

fn render_metric_table(metrics: &[DapScorecardMetric]) -> String {
    let ordered = [
        "launch_success_rate",
        "cold_launch_p50",
        "cold_launch_p95",
        "attach_success_rate",
        "variables_session_correctness",
        "evaluate_session_correctness",
        "deep_pagination_correctness",
        "memory_baseline_proxy",
    ];

    let lookup: BTreeMap<&str, &DapScorecardMetric> =
        metrics.iter().map(|m| (m.metric.as_str(), m)).collect();

    let mut lines =
        vec!["| Metric | Value | Target | Status |".to_string(), "|---|---|---|---|".to_string()];

    for key in ordered {
        if let Some(metric) = lookup.get(key) {
            lines.push(format!(
                "| {} | {} | {} | {} |",
                metric_label(key),
                display_value(metric),
                metric.target,
                metric.status
            ));
        }
    }

    if lines.len() == 2 {
        lines.push(
            "| Scorecard harness | SKIP (no metric rows parsed) | best effort | SKIP |".to_string(),
        );
    }

    lines.join("\n")
}

/// Regenerate the marker blocks in `docs/project/status/dap.md`.
pub(super) fn generate_dap_status(
    counts: &DapTestCounts,
    metrics: &[DapScorecardMetric],
    original: &str,
) -> Result<String> {
    let test_counts_table = format!(
        "| Suite | Count |\n\
         |---|---|\n\
         | Integration tests (`perl-dap`) | {} test targets |\n\
         | Scorecard fixtures | {} |",
        counts.integration_test_targets, counts.scorecard_fixtures,
    );

    let scorecard_table = render_metric_table(metrics);

    let mut text = original.to_string();
    text = replace_block(
        &text,
        "<!-- BEGIN: DAP_LAUNCH_SCORECARD -->",
        "<!-- END: DAP_LAUNCH_SCORECARD -->",
        &scorecard_table,
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
        let metrics = vec![DapScorecardMetric {
            metric: "launch_success_rate".to_string(),
            value: "5/5 (100 %)".to_string(),
            target: "≥ 80 %".to_string(),
            status: "PASS".to_string(),
        }];

        let template = "# DAP\n\
                        <!-- BEGIN: DAP_LAUNCH_SCORECARD -->\n\
                        old launch\n\
                        <!-- END: DAP_LAUNCH_SCORECARD -->\n\
                        <!-- BEGIN: DAP_TEST_COUNTS -->\n\
                        old content\n\
                        <!-- END: DAP_TEST_COUNTS -->\n\
                        tail\n";
        let result = generate_dap_status(&counts, &metrics, template)?;
        assert!(result.contains("20 test targets"), "expected '20 test targets' in output");
        assert!(result.contains("Launch success rate"), "expected launch scorecard row");
        assert!(result.contains("tail"), "suffix text should be preserved");
        Ok(())
    }
}
