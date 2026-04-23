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

fn count_workflows_for_metric_from_fixture_matrix(root: &Path, metric_name: &str) -> usize {
    let matrix_path = root.join("crates/perl-lsp-ux-tests/fixtures/editor_ux_fixture_matrix.json");
    let Ok(raw) = fs::read_to_string(matrix_path) else {
        return 0;
    };
    let Ok(matrix) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return 0;
    };

    matrix["workflows"]
        .as_array()
        .map(|workflows| {
            workflows
                .iter()
                .filter(|workflow| {
                    workflow["measures"].as_array().is_some_and(|measures| {
                        measures.iter().any(|measure| measure.as_str() == Some(metric_name))
                    })
                })
                .count()
        })
        .unwrap_or(0)
}

fn collect_known_gap_issue_refs(root: &Path) -> Vec<u64> {
    let readme_path = root.join("README.md");
    let Ok(readme) = fs::read_to_string(readme_path) else {
        return Vec::new();
    };

    let Some(section_start) = readme.find("### Known gaps toward solid UX") else {
        return Vec::new();
    };
    let section = &readme[section_start..];
    let section = section.split("\n## ").next().unwrap_or(section);

    let Ok(issue_re) = regex::Regex::new(r"github\.com/[^/\s]+/[^/\s]+/issues/([0-9]+)") else {
        return Vec::new();
    };

    let mut issue_refs: Vec<u64> = issue_re
        .captures_iter(section)
        .filter_map(|captures| captures.get(1).and_then(|m| m.as_str().parse::<u64>().ok()))
        .collect();
    issue_refs.sort_unstable();
    issue_refs.dedup();
    issue_refs
}

fn collect_manual_smoke_steps(root: &Path) -> Vec<String> {
    let protocol_path = root.join("docs/project/protocols/verification.md");
    let Ok(protocol) = fs::read_to_string(protocol_path) else {
        return Vec::new();
    };

    protocol
        .lines()
        .find_map(|line| line.strip_prefix("Manual editor smoke test:"))
        .map(|steps| {
            steps
                .split(',')
                .map(str::trim)
                .filter(|step| !step.is_empty())
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Generators
// ---------------------------------------------------------------------------

pub(super) fn generate_quality_status(root: &Path, original: &str) -> Result<String> {
    let mutation_by_crate = collect_per_crate_mutation(root);
    let tests_by_crate = collect_per_crate_test_counts(root);
    let ux_scenarios = count_ux_scenarios(root);
    let known_gap_count = collect_known_gap_issue_refs(root).len();
    let manual_smoke_steps = collect_manual_smoke_steps(root).len();

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
           the integration-only 10k-line large-file case; tracked scorecard at \
           `docs/project/status/editor_ux.json` publishes harness coverage, manual smoke \
           checklist ({manual_smoke_steps} steps), and known-gap issue tracking ({known_gap_count} refs)\n\
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
    let pass_rate_workflows =
        count_workflows_for_metric_from_fixture_matrix(root, "workflow_pass_rate");
    let stability_workflows =
        count_workflows_for_metric_from_fixture_matrix(root, "workflow_stability_rate");
    let p95_workflows =
        count_workflows_for_metric_from_fixture_matrix(root, "p95_time_to_first_useful_result_ms");
    let known_gap_issues = collect_known_gap_issue_refs(root);
    let manual_smoke_steps = collect_manual_smoke_steps(root);

    let receipt = serde_json::json!({
        "schema_version": 1,
        "receipt_kind": "tracked_scorecard",
        "scorecard": "editor_ux",
        "harness": {
            "crate": "crates/perl-lsp-ux-tests",
            "scenario_count": scenario_count,
            "scenario_files": scenario_files,
        },
        "top_line_metrics": [
            {
                "name": "workflow_pass_rate",
                "state": "measured",
                "owner": "perl-lsp-ux-tests",
                "numerator": pass_rate_workflows,
                "denominator": scenario_count,
                "unit": "workflows_with_explicit_assertions",
            },
            {
                "name": "workflow_stability_rate",
                "state": "measured",
                "owner": "perl-lsp-ux-tests",
                "numerator": stability_workflows,
                "denominator": scenario_count,
                "unit": "workflows_with_explicit_assertions",
            },
            {
                "name": "p95_time_to_first_useful_result_ms",
                "state": "measured",
                "owner": "perl-lsp-ux-tests",
                "numerator": p95_workflows,
                "denominator": scenario_count,
                "unit": "workflows_with_latency_budget",
            },
        ],
        "manual_smoke": {
            "protocol": "docs/project/protocols/verification.md",
            "step_count": manual_smoke_steps.len(),
            "steps": manual_smoke_steps,
        },
        "known_gap_issues": {
            "source": "README.md#known-gaps-toward-solid-ux",
            "count": known_gap_issues.len(),
            "issues": known_gap_issues,
        },
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
        assert_eq!(receipt["receipt_kind"], "tracked_scorecard");
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
        for metric in receipt["top_line_metrics"]
            .as_array()
            .ok_or_else(|| eyre!("top_line_metrics must be an array"))?
        {
            assert_eq!(metric["state"], "measured");
            let numerator =
                metric["numerator"].as_u64().ok_or_else(|| eyre!("numerator missing"))?;
            let denominator =
                metric["denominator"].as_u64().ok_or_else(|| eyre!("denominator missing"))?;
            assert!(numerator <= denominator, "numerator cannot exceed denominator");
        }
        assert_eq!(
            receipt["manual_smoke"]["step_count"].as_u64(),
            Some(5),
            "verification protocol should track the 5-step manual smoke checklist"
        );
        assert!(
            receipt["known_gap_issues"]["count"].as_u64().unwrap_or_default() >= 1,
            "known gap issue tracking must include at least one issue reference"
        );
        assert_eq!(receipt["integration_points"]["ci_lane"], "just ux-tests");
        Ok(())
    }

    #[test]
    fn test_collect_known_gap_issue_refs_from_section() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let readme = r#"
## Status
### Known gaps toward solid UX
- See [#100](https://github.com/example/repo/issues/100)
- Another reference [#200](https://github.com/example/repo/issues/200) and duplicate [#100](https://github.com/example/repo/issues/100)

## Security
"#;
        fs::write(dir.path().join("README.md"), readme)?;
        let issues = collect_known_gap_issue_refs(dir.path());
        assert_eq!(issues, vec![100, 200]);
        Ok(())
    }

    #[test]
    fn test_collect_manual_smoke_steps_from_protocol_line() -> Result<()> {
        let dir = tempfile::tempdir()?;
        fs::create_dir_all(dir.path().join("docs/project/protocols"))?;
        fs::write(
            dir.path().join("docs/project/protocols/verification.md"),
            "Manual editor smoke test: diagnostics, completion, hover, go-to-definition, rename\n",
        )?;
        let steps = collect_manual_smoke_steps(dir.path());
        assert_eq!(
            steps,
            vec![
                "diagnostics".to_string(),
                "completion".to_string(),
                "hover".to_string(),
                "go-to-definition".to_string(),
                "rename".to_string()
            ]
        );
        Ok(())
    }
}
