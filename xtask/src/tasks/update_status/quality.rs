//! Quality subsystem status generator.
//!
//! Owns per-crate mutation and test counts, UX scenario receipt, and quality.md generation.
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::sync::LazyLock;
use std::time::Duration;

use color_eyre::eyre::{Context, Result};
use serde_json::Value;

use super::{replace_block, run_cmd};
static RUNNING_TEST_CRATE_RE: LazyLock<Option<regex::Regex>> = LazyLock::new(|| {
    regex::Regex::new(r"Running unittests[^\(]*\(target[^\)]*deps[/\\]([a-zA-Z0-9_-]+)-[0-9a-f]+\)")
        .ok()
});
static LISTED_TEST_RE: LazyLock<Option<regex::Regex>> =
    LazyLock::new(|| regex::Regex::new(r":\s*test\s*$").ok());
static ISSUE_REF_RE: LazyLock<Option<regex::Regex>> =
    LazyLock::new(|| regex::Regex::new(r"\[#(\d+)\]\(").ok());

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

    let mut by_crate: BTreeMap<String, usize> = BTreeMap::new();
    let mut current_crate: Option<String> = None;

    for line in output.lines() {
        if let Some(caps) = RUNNING_TEST_CRATE_RE.as_ref().and_then(|r| r.captures(line)) {
            let name = caps[1].replace('_', "-");
            current_crate = Some(name);
            continue;
        }
        if let Some(re) = LISTED_TEST_RE.as_ref()
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
fn fixture_matrix_workflow_count(root: &Path) -> usize {
    let matrix_path = root.join("crates/perl-lsp-ux-tests/fixtures/editor_ux_fixture_matrix.json");
    let Ok(raw) = fs::read_to_string(matrix_path) else {
        return 0;
    };
    let Ok(json) = serde_json::from_str::<Value>(&raw) else {
        return 0;
    };
    json.get("workflows").and_then(Value::as_array).map_or(0, std::vec::Vec::len)
}

fn fixture_matrix_tier_counts(root: &Path) -> BTreeMap<String, usize> {
    let matrix_path = root.join("crates/perl-lsp-ux-tests/fixtures/editor_ux_fixture_matrix.json");
    let Ok(raw) = fs::read_to_string(matrix_path) else {
        return BTreeMap::new();
    };
    let Ok(json) = serde_json::from_str::<Value>(&raw) else {
        return BTreeMap::new();
    };
    let Some(workflows) = json.get("workflows").and_then(Value::as_array) else {
        return BTreeMap::new();
    };
    let mut counts = BTreeMap::new();
    for workflow in workflows {
        if let Some(tier) = workflow.get("ci_tier").and_then(Value::as_str) {
            *counts.entry(tier.to_string()).or_insert(0) += 1;
        }
    }
    counts
}

fn manual_smoke_action_count(root: &Path) -> usize {
    let path = root.join("docs/project/protocols/verification.md");
    let Ok(raw) = fs::read_to_string(path) else {
        return 0;
    };
    let Some(manual_line) = raw.lines().find(|line| line.starts_with("Manual editor smoke test: "))
    else {
        return 0;
    };
    let Some((_, actions)) = manual_line.split_once(':') else {
        return 0;
    };
    actions.split(',').map(str::trim).filter(|entry| !entry.is_empty()).count()
}

fn known_gap_issue_ref_count(root: &Path) -> usize {
    let path = root.join("README.md");
    let Ok(raw) = fs::read_to_string(path) else {
        return 0;
    };
    let start = raw.find("### Known gaps toward solid UX").unwrap_or(0);
    let end = raw.find("## Security").unwrap_or(raw.len());
    let known_gaps_block = &raw[start..end];

    let mut issues = BTreeSet::new();
    if let Some(re) = ISSUE_REF_RE.as_ref() {
        for captures in re.captures_iter(known_gaps_block) {
            issues.insert(captures[1].to_string());
        }
    }
    issues.len()
}

fn latest_top_line_metric_values(root: &Path) -> BTreeMap<String, f64> {
    let metrics_path = root.join(".ci/metrics/editor_ux.json");
    let Ok(raw) = fs::read_to_string(metrics_path) else {
        return BTreeMap::new();
    };
    let Ok(json) = serde_json::from_str::<Value>(&raw) else {
        return BTreeMap::new();
    };
    let Some(metrics) = json.get("metrics").and_then(Value::as_object) else {
        return BTreeMap::new();
    };
    let mut values = BTreeMap::new();
    for key in
        ["workflow_pass_rate", "workflow_stability_rate", "p95_time_to_first_useful_result_ms"]
    {
        if let Some(value) = metrics.get(key).and_then(Value::as_f64) {
            values.insert(key.to_string(), value);
        }
    }
    values
}

pub(super) fn generate_quality_status(root: &Path, original: &str) -> Result<String> {
    let mutation_by_crate = collect_per_crate_mutation(root);
    let tests_by_crate = collect_per_crate_test_counts(root);
    let ux_scenarios = count_ux_scenarios(root);
    let known_gap_issue_refs = known_gap_issue_ref_count(root);
    let manual_smoke_actions = manual_smoke_action_count(root);

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
           the integration-only 10k-line large-file case; tracked UX signal snapshot at \
           `docs/project/status/editor_ux.json`\n\
         - **UX confidence signals tracked**: {ux_scenarios} automated workflows + \
           {manual_smoke_actions} manual smoke actions + {known_gap_issue_refs} linked open-gap references\n\
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
    let workflow_count = fixture_matrix_workflow_count(root);
    let tier_counts = fixture_matrix_tier_counts(root);
    let metric_values = latest_top_line_metric_values(root);
    let top_line_metrics: Vec<Value> =
        ["workflow_pass_rate", "workflow_stability_rate", "p95_time_to_first_useful_result_ms"]
            .into_iter()
            .map(|name| {
                if let Some(value) = metric_values.get(name) {
                    serde_json::json!({
                        "name": name,
                        "state": "measured",
                        "owner": "perl-lsp-ux-tests",
                        "value": value,
                        "source": ".ci/metrics/editor_ux.json",
                    })
                } else {
                    serde_json::json!({
                        "name": name,
                        "state": "tracked",
                        "owner": "perl-lsp-ux-tests",
                        "source": "crates/perl-lsp-ux-tests/fixtures/editor_ux_fixture_matrix.json",
                    })
                }
            })
            .collect();

    let receipt = serde_json::json!({
        "schema_version": 1,
        "receipt_kind": "tracking_snapshot",
        "scorecard": "editor_ux",
        "harness": {
            "crate": "crates/perl-lsp-ux-tests",
            "scenario_count": scenario_count,
            "scenario_files": scenario_files,
            "workflow_count": workflow_count,
        },
        "top_line_metrics": top_line_metrics,
        "signals": {
            "automated_workflows": workflow_count,
            "manual_smoke_actions": manual_smoke_action_count(root),
            "known_gap_issue_refs": known_gap_issue_ref_count(root),
            "ci_tier_workflow_counts": tier_counts,
        },
        "integration_points": {
            "ci_lane": "just ux-tests",
            "release_lane": "just ux-tests-full",
            "status_update": "cargo xtask update-status --only quality",
            "quality_surface": "docs/project/status/quality.md",
            "measured_metrics_receipt": ".ci/metrics/editor_ux.json",
        },
    });

    serde_json::to_string_pretty(&receipt).context("serializing editor UX receipt")
}

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
        assert_eq!(receipt["receipt_kind"], "tracking_snapshot");
        assert_eq!(receipt["scorecard"], "editor_ux");
        assert_eq!(receipt["harness"]["crate"], "crates/perl-lsp-ux-tests");
        assert_eq!(
            receipt["harness"]["scenario_count"].as_u64(),
            Some(count_ux_scenarios(&root) as u64)
        );
        assert!(receipt["harness"]["workflow_count"].as_u64().unwrap_or_default() > 0);
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
        assert!(
            receipt["signals"]["manual_smoke_actions"].as_u64().unwrap_or_default() >= 1,
            "manual smoke actions should be tracked from verification protocol"
        );
        assert!(
            receipt["signals"]["known_gap_issue_refs"].as_u64().unwrap_or_default() >= 1,
            "known-gap issue links should be tracked from README"
        );
        assert_eq!(receipt["integration_points"]["ci_lane"], "just ux-tests");
        Ok(())
    }
}
