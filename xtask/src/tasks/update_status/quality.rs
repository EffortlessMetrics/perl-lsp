//! Quality subsystem status generator.
//!
//! Owns per-crate mutation and test counts, UX scenario receipt, and quality.md generation.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::Duration;

use color_eyre::eyre::{Context, Result};

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

#[derive(Debug, Clone, Default)]
struct UxWorkflowInventory {
    total: usize,
    pr_tier: usize,
    nightly_tier: usize,
}

fn collect_ux_workflow_inventory(root: &Path) -> UxWorkflowInventory {
    let matrix_path = root.join("crates/perl-lsp-ux-tests/fixtures/editor_ux_fixture_matrix.json");
    let Ok(raw) = fs::read_to_string(&matrix_path) else {
        let total = count_ux_scenarios(root);
        return UxWorkflowInventory { total, ..UxWorkflowInventory::default() };
    };
    let Ok(doc) = serde_json::from_str::<serde_json::Value>(&raw) else {
        let total = count_ux_scenarios(root);
        return UxWorkflowInventory { total, ..UxWorkflowInventory::default() };
    };

    let Some(workflows) = doc.get("workflows").and_then(serde_json::Value::as_array) else {
        let total = count_ux_scenarios(root);
        return UxWorkflowInventory { total, ..UxWorkflowInventory::default() };
    };

    let mut inventory =
        UxWorkflowInventory { total: workflows.len(), ..UxWorkflowInventory::default() };
    for workflow in workflows {
        match workflow.get("ci_tier").and_then(serde_json::Value::as_str) {
            Some("pr") => inventory.pr_tier += 1,
            Some("nightly") => inventory.nightly_tier += 1,
            _ => {}
        }
    }
    inventory
}

fn load_editor_ux_metrics(root: &Path) -> BTreeMap<String, serde_json::Value> {
    let metrics_path = root.join(".ci/metrics/editor_ux.json");
    let Ok(raw) = fs::read_to_string(&metrics_path) else {
        return BTreeMap::new();
    };
    let Ok(doc) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return BTreeMap::new();
    };
    let Some(metrics_obj) = doc.get("metrics").and_then(serde_json::Value::as_object) else {
        return BTreeMap::new();
    };
    metrics_obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
}

// ---------------------------------------------------------------------------
// Generators
// ---------------------------------------------------------------------------

pub(super) fn generate_quality_status(root: &Path, original: &str) -> Result<String> {
    let mutation_by_crate = collect_per_crate_mutation(root);
    let tests_by_crate = collect_per_crate_test_counts(root);
    let ux_inventory = collect_ux_workflow_inventory(root);
    let ux_scenarios =
        if ux_inventory.total > 0 { ux_inventory.total } else { count_ux_scenarios(root) };

    let has_mutation_data = !mutation_by_crate.is_empty();
    let mutation_note = if has_mutation_data {
        "per-crate data from `mutants.out/mutants.json` (written by nightly CI `cargo mutants` run)"
    } else {
        "mutation data pending first nightly CI run — run `just mutation-subset` locally to populate"
    };

    let bullets_content = format!(
        "- **Quality Metrics**: <50ms LSP response times, 931ns incremental parsing\n\
         - **UX workflow harness**: {ux_scenarios} canonical workflows in `perl-lsp-ux-tests` \
           ({pr_workflows} PR-tier, {nightly_workflows} nightly-tier); \
           `just ux-tests` runs the default release-confidence lane and `just ux-tests-full` adds \
           the integration-only 10k-line large-file case; planning scaffold at \
           `docs/project/status/editor_ux.json`\n\
         - **Mutation testing**: {mutation_note}\n\
         - **Production Status**: LSP server public alpha (`just ci-gate` passing)",
        pr_workflows = ux_inventory.pr_tier,
        nightly_workflows = ux_inventory.nightly_tier
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
    let measured_metrics = load_editor_ux_metrics(root);
    let metric_names =
        ["workflow_pass_rate", "workflow_stability_rate", "p95_time_to_first_useful_result_ms"];
    let has_measured_top_line = metric_names
        .iter()
        .any(|name| measured_metrics.get(*name).is_some_and(|value| !value.is_null()));
    let receipt_kind =
        if has_measured_top_line { "tracked_scorecard" } else { "planning_scaffold" };

    let top_line_metrics = metric_names
        .iter()
        .map(|name| {
            let value = measured_metrics.get(*name).cloned().unwrap_or(serde_json::Value::Null);
            let state = if value.is_null() { "planned" } else { "measured" };
            serde_json::json!({
                "name": name,
                "state": state,
                "owner": "perl-lsp-ux-tests",
                "value": value,
            })
        })
        .collect::<Vec<_>>();

    let receipt = serde_json::json!({
        "schema_version": 1,
        "receipt_kind": receipt_kind,
        "scorecard": "editor_ux",
        "harness": {
            "crate": "crates/perl-lsp-ux-tests",
            "scenario_count": scenario_count,
            "scenario_files": scenario_files,
        },
        "top_line_metrics": top_line_metrics,
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
        for row in receipt["top_line_metrics"]
            .as_array()
            .ok_or_else(|| eyre!("top_line_metrics must be an array"))?
        {
            assert!(
                row.get("value").is_some(),
                "top_line metric rows should include a nullable `value` field"
            );
        }
        assert_eq!(receipt["integration_points"]["ci_lane"], "just ux-tests");
        Ok(())
    }

    #[test]
    fn test_collect_ux_workflow_inventory_from_fixture_matrix() -> Result<()> {
        let root = crate::utils::project_root()?;
        let inventory = collect_ux_workflow_inventory(&root);
        assert!(inventory.total >= 1, "fixture matrix should define at least one workflow");
        assert_eq!(
            inventory.total,
            inventory.pr_tier + inventory.nightly_tier,
            "workflow inventory should be partitioned across expected ci tiers"
        );
        Ok(())
    }

    #[test]
    fn test_generate_editor_ux_receipt_marks_measured_metrics() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = dir.path();
        fs::create_dir_all(root.join(".ci/metrics"))?;
        fs::create_dir_all(root.join("crates/perl-lsp-ux-tests/tests"))?;
        fs::write(
            root.join("crates/perl-lsp-ux-tests/tests/ux_scenario_01_simple_file.rs"),
            "// scenario",
        )?;
        fs::write(
            root.join(".ci/metrics/editor_ux.json"),
            r#"{
                "schema_version": 1,
                "metrics": {
                    "workflow_pass_rate": 0.92,
                    "workflow_stability_rate": null,
                    "p95_time_to_first_useful_result_ms": 48
                }
            }"#,
        )?;

        let receipt_raw = generate_editor_ux_receipt(root)?;
        let receipt: serde_json::Value = serde_json::from_str(&receipt_raw)?;
        assert_eq!(receipt["receipt_kind"], "tracked_scorecard");

        let top_line = receipt["top_line_metrics"]
            .as_array()
            .ok_or_else(|| eyre!("top_line_metrics should be an array"))?;
        let measured_count = top_line.iter().filter(|row| row["state"] == "measured").count();
        assert_eq!(measured_count, 2, "two non-null metric values should be measured");
        Ok(())
    }
}
