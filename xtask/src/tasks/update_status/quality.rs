//! Quality subsystem status generator.
//!
//! Owns per-crate mutation and test counts, UX scenario receipt, and quality.md generation.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::Duration;

use color_eyre::eyre::{Context, Result};
use serde::Deserialize;

use super::{replace_block, run_cmd};

// ---------------------------------------------------------------------------
// Metric collectors
// ---------------------------------------------------------------------------

/// Read `mutants.out/mutants.json` and group mutations by crate package name.
pub(super) fn collect_per_crate_mutation(root: &Path) -> BTreeMap<String, usize> {
    let path = root.join("mutants.out").join("mutants.json");
    let Ok(raw) = fs::read_to_string(&path) else {
        return BTreeMap::new();
    };
    let Ok(entries) = serde_json::from_str::<Vec<serde_json::Value>>(&raw) else {
        return BTreeMap::new();
    };
    let mut by_crate: BTreeMap<String, usize> = BTreeMap::new();
    for entry in entries {
        if let Some(pkg) = entry.get("package").and_then(|v| v.as_str()) {
            *by_crate.entry(pkg.to_string()).or_default() += 1;
        }
    }
    by_crate
}

/// Parse `cargo test --workspace --lib -- --list` output and return a map of
/// crate-name → test count.
pub(super) fn collect_per_crate_test_counts(root: &Path) -> BTreeMap<String, usize> {
    let output = run_cmd(
        root,
        &["cargo", "test", "--workspace", "--lib", "--exclude", "tree-sitter-perl", "--", "--list"],
        Duration::from_secs(180),
    );
    if output.is_empty() {
        return BTreeMap::new();
    }

    let running_re = regex::Regex::new(
        r"Running unittests[^\(]*\(target[^\)]*deps[/\\]([a-zA-Z0-9_-]+)-[0-9a-f]+\)",
    )
    .ok();
    let test_re = regex::Regex::new(r":\s*test\s*$").ok();

    let mut by_crate: BTreeMap<String, usize> = BTreeMap::new();
    let mut current_crate: Option<String> = None;

    for line in output.lines() {
        if let Some(caps) = running_re.as_ref().and_then(|r| r.captures(line)) {
            let name = caps[1].replace('_', "-");
            current_crate = Some(name);
            continue;
        }
        if let Some(re) = test_re.as_ref()
            && re.is_match(line)
            && let Some(ref crate_name) = current_crate
        {
            *by_crate.entry(crate_name.clone()).or_default() += 1;
        }
    }
    by_crate
}

/// Format a combined per-crate markdown table showing mutation count and test count.
pub(super) fn format_crate_quality_table(
    mutation: &BTreeMap<String, usize>,
    tests: &BTreeMap<String, usize>,
) -> String {
    let mut crates: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for k in mutation.keys() {
        crates.insert(k.as_str());
    }
    for k in tests.keys() {
        crates.insert(k.as_str());
    }

    if crates.is_empty() {
        return "| Crate | Mutants listed | Tests (lib) |\n\
                |-------|---------------|-------------|\n\
                | — | no data yet | no data yet |"
            .to_string();
    }

    let mut lines = vec![
        "| Crate | Mutants listed | Tests (lib) |".to_string(),
        "|-------|---------------|-------------|".to_string(),
    ];
    for crate_name in crates {
        let mutants = mutation.get(crate_name).map_or_else(|| "—".to_string(), |n| n.to_string());
        let test_count = tests.get(crate_name).map_or_else(|| "—".to_string(), |n| n.to_string());
        lines.push(format!("| {crate_name} | {mutants} | {test_count} |"));
    }
    lines.join("\n")
}

pub(super) fn collect_ux_scenario_files(root: &Path) -> Vec<String> {
    let tests_dir = root.join("crates/perl-lsp-ux-tests/tests");
    let Ok(entries) = fs::read_dir(tests_dir) else {
        return Vec::new();
    };

    let mut files: Vec<String> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.starts_with("ux_scenario_") && name.ends_with(".rs"))
        .map(|name| format!("crates/perl-lsp-ux-tests/tests/{name}"))
        .collect();
    files.sort();
    files
}

pub(super) fn count_ux_scenarios(root: &Path) -> usize {
    collect_ux_scenario_files(root).len()
}

#[derive(Debug, Deserialize)]
struct EditorUxMetricsEnvelope {
    measured_at: Option<String>,
    metrics: Option<EditorUxTopLineMetrics>,
}

#[derive(Debug, Deserialize)]
struct EditorUxTopLineMetrics {
    workflow_pass_rate: Option<f64>,
    workflow_stability_rate: Option<f64>,
    p95_time_to_first_useful_result_ms: Option<u64>,
}

fn load_editor_ux_top_line_metrics(root: &Path) -> Option<EditorUxMetricsEnvelope> {
    let metrics_path = root.join(".ci/metrics/editor_ux.json");
    let raw = fs::read_to_string(metrics_path).ok()?;
    serde_json::from_str::<EditorUxMetricsEnvelope>(&raw).ok()
}

// ---------------------------------------------------------------------------
// Generators
// ---------------------------------------------------------------------------

pub(super) fn generate_quality_status(root: &Path, original: &str) -> Result<String> {
    let mutation_by_crate = collect_per_crate_mutation(root);
    let tests_by_crate = collect_per_crate_test_counts(root);
    let ux_scenarios = count_ux_scenarios(root);

    let has_mutation_data = !mutation_by_crate.is_empty();
    let mutation_note = if has_mutation_data {
        "per-crate data from `mutants.out/mutants.json` (written by nightly CI `cargo mutants` run)"
    } else {
        "mutation data pending first nightly CI run — run `just mutation-subset` locally to populate"
    };

    let bullets_content = format!(
        "- **Quality Metrics**: <50ms LSP response times, 931ns incremental parsing\n\
         - **UX workflow harness**: {ux_scenarios} scenario files in `perl-lsp-ux-tests`; \
           `just ux-tests` runs the default release-confidence lane and `just ux-tests-full` adds \
           the integration-only 10k-line large-file case; scorecard receipt at \
           `docs/project/status/editor_ux.json`\n\
         - **Mutation testing**: {mutation_note}\n\
         - **Production Status**: LSP server public alpha (`just ci-gate` passing)"
    );

    let crate_table = format_crate_quality_table(&mutation_by_crate, &tests_by_crate);

    let mut text = original.to_string();
    text = replace_block(
        &text,
        "<!-- BEGIN: QUALITY_METRICS_BULLETS -->",
        "<!-- END: QUALITY_METRICS_BULLETS -->",
        &bullets_content,
    )?;
    text = replace_block(
        &text,
        "<!-- BEGIN: QUALITY_CRATE_TABLE -->",
        "<!-- END: QUALITY_CRATE_TABLE -->",
        &crate_table,
    )?;
    Ok(text)
}

pub(super) fn generate_editor_ux_receipt(root: &Path) -> Result<String> {
    let scenario_files = collect_ux_scenario_files(root);
    let scenario_count = scenario_files.len();
    let measured = load_editor_ux_top_line_metrics(root);
    let measured_at = measured.as_ref().and_then(|m| m.measured_at.as_ref()).cloned();
    let top_line = measured.as_ref().and_then(|m| m.metrics.as_ref());

    let workflow_pass_rate = top_line.and_then(|m| m.workflow_pass_rate);
    let workflow_stability_rate = top_line.and_then(|m| m.workflow_stability_rate);
    let p95_time_to_first_useful_result_ms =
        top_line.and_then(|m| m.p95_time_to_first_useful_result_ms);

    let receipt_kind = if workflow_pass_rate.is_some()
        || workflow_stability_rate.is_some()
        || p95_time_to_first_useful_result_ms.is_some()
    {
        "tracked_metrics"
    } else {
        "planning_scaffold"
    };

    let receipt = serde_json::json!({
        "schema_version": 1,
        "receipt_kind": receipt_kind,
        "scorecard": "editor_ux",
        "harness": {
            "crate": "crates/perl-lsp-ux-tests",
            "scenario_count": scenario_count,
            "scenario_files": scenario_files,
        },
        "top_line_metrics": [
            {
                "name": "workflow_pass_rate",
                "state": if workflow_pass_rate.is_some() { "measured" } else { "planned" },
                "owner": "perl-lsp-ux-tests",
                "observed_value": workflow_pass_rate,
            },
            {
                "name": "workflow_stability_rate",
                "state": if workflow_stability_rate.is_some() { "measured" } else { "planned" },
                "owner": "perl-lsp-ux-tests",
                "observed_value": workflow_stability_rate,
            },
            {
                "name": "p95_time_to_first_useful_result_ms",
                "state": if p95_time_to_first_useful_result_ms.is_some() {
                    "measured"
                } else {
                    "planned"
                },
                "owner": "perl-lsp-ux-tests",
                "observed_value": p95_time_to_first_useful_result_ms,
            },
        ],
        "measured_at": measured_at,
        "integration_points": {
            "ci_lane": "just ux-tests",
            "release_lane": "just ux-tests-full",
            "status_update": "cargo xtask update-status --only quality",
            "quality_surface": "docs/project/status/quality.md",
        },
    });

    serde_json::to_string_pretty(&receipt).context("serializing editor UX receipt")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::eyre::{Result, eyre};

    #[test]
    fn test_collect_per_crate_mutation_from_mock_file() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let out_dir = dir.path().join("mutants.out");
        fs::create_dir_all(&out_dir)?;
        let json = r#"[
            {"package":"perl-quote","file":"crates/perl-quote/src/lib.rs","genre":"FnValue"},
            {"package":"perl-quote","file":"crates/perl-quote/src/lib.rs","genre":"BinaryOperator"},
            {"package":"perl-parser","file":"crates/perl-parser/src/lib.rs","genre":"FnValue"}
        ]"#;
        fs::write(out_dir.join("mutants.json"), json)?;
        let result = collect_per_crate_mutation(dir.path());
        assert_eq!(result.get("perl-quote"), Some(&2), "expected 2 mutants for perl-quote");
        assert_eq!(result.get("perl-parser"), Some(&1), "expected 1 mutant for perl-parser");
        Ok(())
    }

    #[test]
    fn test_format_crate_quality_table_has_header_and_data() {
        let mut mutation = BTreeMap::new();
        mutation.insert("perl-quote".to_string(), 249);
        let mut tests = BTreeMap::new();
        tests.insert("perl-quote".to_string(), 42);
        let table = format_crate_quality_table(&mutation, &tests);
        assert!(table.contains("Crate"), "missing header");
        assert!(table.contains("perl-quote"), "missing crate name");
        assert!(table.contains("249"), "missing mutant count");
        assert!(table.contains("42"), "missing test count");
    }

    #[test]
    fn test_format_crate_quality_table_empty_maps() {
        let table = format_crate_quality_table(&BTreeMap::new(), &BTreeMap::new());
        assert!(table.contains("no data yet"), "expected 'no data yet' for empty maps");
    }

    #[test]
    fn test_editor_ux_receipt_shape() -> Result<()> {
        let root = crate::utils::project_root()?;
        let receipt_raw = generate_editor_ux_receipt(&root)?;
        let receipt: serde_json::Value = serde_json::from_str(&receipt_raw)?;
        assert_eq!(receipt["schema_version"], 1);
        assert_eq!(receipt["receipt_kind"], "planning_scaffold");
        assert_eq!(receipt["scorecard"], "editor_ux");
        assert_eq!(receipt["harness"]["crate"], "crates/perl-lsp-ux-tests");
        assert_eq!(
            receipt["harness"]["scenario_count"].as_u64(),
            Some(count_ux_scenarios(&root) as u64)
        );
        let top_line_names = receipt["top_line_metrics"]
            .as_array()
            .ok_or_else(|| eyre!("top_line_metrics must be an array"))?
            .iter()
            .map(|row| row["name"].as_str().ok_or_else(|| eyre!("top_line metric name missing")))
            .collect::<Result<std::collections::BTreeSet<_>>>()?;
        assert_eq!(
            top_line_names,
            std::collections::BTreeSet::from([
                "workflow_pass_rate",
                "workflow_stability_rate",
                "p95_time_to_first_useful_result_ms",
            ])
        );
        assert_eq!(receipt["integration_points"]["ci_lane"], "just ux-tests");
        Ok(())
    }

    #[test]
    fn test_editor_ux_receipt_uses_measured_values_when_available() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = dir.path();
        fs::create_dir_all(root.join("crates/perl-lsp-ux-tests/tests"))?;
        fs::write(
            root.join("crates/perl-lsp-ux-tests/tests/ux_scenario_01_simple_file.rs"),
            "// fixture",
        )?;
        fs::create_dir_all(root.join(".ci/metrics"))?;
        fs::write(
            root.join(".ci/metrics/editor_ux.json"),
            r#"{
                "schema_version": 1,
                "measured_at": "2026-04-23T00:00:00Z",
                "subsystem": "editor_ux",
                "metrics": {
                    "workflow_pass_rate": 0.87,
                    "workflow_stability_rate": null,
                    "p95_time_to_first_useful_result_ms": 82
                }
            }"#,
        )?;

        let receipt_raw = generate_editor_ux_receipt(root)?;
        let receipt: serde_json::Value = serde_json::from_str(&receipt_raw)?;
        assert_eq!(receipt["receipt_kind"], "tracked_metrics");
        assert_eq!(receipt["measured_at"], "2026-04-23T00:00:00Z");

        let metrics = receipt["top_line_metrics"]
            .as_array()
            .ok_or_else(|| eyre!("top_line_metrics must be an array"))?;
        let pass_row = metrics
            .iter()
            .find(|row| row["name"] == "workflow_pass_rate")
            .ok_or_else(|| eyre!("missing workflow_pass_rate row"))?;
        assert_eq!(pass_row["state"], "measured");
        assert_eq!(pass_row["observed_value"], 0.87);

        let stability_row = metrics
            .iter()
            .find(|row| row["name"] == "workflow_stability_rate")
            .ok_or_else(|| eyre!("missing workflow_stability_rate row"))?;
        assert_eq!(stability_row["state"], "planned");
        assert!(stability_row["observed_value"].is_null());

        let latency_row = metrics
            .iter()
            .find(|row| row["name"] == "p95_time_to_first_useful_result_ms")
            .ok_or_else(|| eyre!("missing p95_time_to_first_useful_result_ms row"))?;
        assert_eq!(latency_row["state"], "measured");
        assert_eq!(latency_row["observed_value"], 82);

        Ok(())
    }
}
