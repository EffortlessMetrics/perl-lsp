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
<<<<<<< Updated upstream
=======
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

/// Latency statistics for a benchmark category.
#[derive(Clone, Debug)]
pub(super) struct LatencyStats {
    /// 50th percentile latency in milliseconds.
    pub p50_ms: f64,
    /// 95th percentile latency in milliseconds.
    pub p95_ms: f64,
    /// 99th percentile latency in milliseconds.
    pub p99_ms: f64,
}

/// Read `benchmarks/results/latest.json` (produced by `cargo xtask bench-run --output
/// benchmarks/results/latest.json`) and aggregate latency data by benchmark category.
///
/// The benchmark results file contains timing data categorized as:
/// - parser: parsing benchmarks
/// - lexer: tokenization benchmarks
/// - lsp: LSP operation benchmarks
/// - index: indexing benchmarks
///
/// Returns a map of category → LatencyStats with p50/p95/p99 values.
/// Returns an empty map if the benchmark file doesn't exist.
pub(super) fn collect_latency_by_subsystem(root: &Path) -> BTreeMap<String, LatencyStats> {
    let path = root.join("benchmarks/results/latest.json");
    let Ok(raw) = fs::read_to_string(&path) else {
        return BTreeMap::new();
    };
    let Ok(data) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return BTreeMap::new();
    };

    let results = match data.get("results").and_then(|v| v.as_object()) {
        Some(r) => r,
        None => return BTreeMap::new(),
    };

    let mut latency_by_category: BTreeMap<String, Vec<f64>> = BTreeMap::new();

    for (category, benchmarks) in results {
        let benchmarks = match benchmarks.as_object() {
            Some(b) => b,
            None => continue,
        };

        for (_bench_name, bench_data) in benchmarks {
            // Skip internal keys
            if _bench_name.starts_with('_') {
                continue;
            }

            // Extract mean_ns and convert to ms
            if let Some(mean_ns) = bench_data
                .get("mean_ns")
                .and_then(|v| v.as_u64())
                .or_else(|| bench_data.get("mean_ns").and_then(|v| v.as_f64().map(|f| f as u64)))
            {
                let ms = mean_ns as f64 / 1_000_000.0;
                latency_by_category.entry(category.clone()).or_default().push(ms);
            }
        }
>>>>>>> Stashed changes
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
           the integration-only 10k-line large-file case; planning scaffold at \
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

<<<<<<< Updated upstream
=======
/// Collect subsystem test counts by reading subsystem-mapping.yaml and
/// aggregating per-crate test counts.
pub(super) fn collect_subsystem_test_counts(
    root: &Path,
) -> BTreeMap<crate::tasks::update_status::StatusSubsystem, crate::tasks::update_status::tests::TestCounts> {
    let mapping_path = root.join(".ci/subsystem-mapping.yaml");
    let mapping_raw = match fs::read_to_string(&mapping_path) {
        Ok(r) => r,
        Err(_) => return BTreeMap::new(),
    };
    let mapping_data = match serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&mapping_raw) {
        Ok(d) => d,
        Err(_) => return BTreeMap::new(),
    };

    let crate_to_subsystem =
        match mapping_data.get("crate_to_subsystem").and_then(|v| v.as_mapping()) {
            Some(m) => m,
            None => return BTreeMap::new(),
        };

    // Collect per-crate test counts
    let per_crate_tests = collect_per_crate_test_counts(root);

    // Group by subsystem
    let mut subsystem_counts: BTreeMap<crate::tasks::update_status::StatusSubsystem, (usize, usize)> = BTreeMap::new();

    for (crate_name, test_count) in &per_crate_tests {
        // Find the subsystem for this crate
        let subsystem_str = crate_to_subsystem
            .get(&serde_yaml_ng::Value::String(crate_name.clone()))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let subsystem = match subsystem_str {
            "Parser" => crate::tasks::update_status::StatusSubsystem::Parser,
            "Quality" => crate::tasks::update_status::StatusSubsystem::Quality,
            "Lsp" => crate::tasks::update_status::StatusSubsystem::Lsp,
            "Dap" => crate::tasks::update_status::StatusSubsystem::Dap,
            "Workspace" => crate::tasks::update_status::StatusSubsystem::Workspace,
            "Tests" => crate::tasks::update_status::StatusSubsystem::Tests,
            _ => continue,
        };

        let entry = subsystem_counts.entry(subsystem).or_default();
        entry.0 += test_count;
    }

    // Convert to TestCounts struct
    subsystem_counts
        .into_iter()
        .map(|(subsystem, (test_count, ignore_count))| {
            (
                subsystem,
                crate::tasks::update_status::tests::TestCounts {
                    tier_a_lib_tests: Some(test_count),
                    ignored_total: Some(ignore_count),
                    bug_count: None,
                    manual_count: None,
                },
            )
        })
        .collect()
}

/// Format subsystem test counts as a markdown table.
///
/// Columns: Subsystem | Tests | Ignored
pub(super) fn format_subsystem_test_table(
    counts: &BTreeMap<crate::tasks::update_status::StatusSubsystem, crate::tasks::update_status::tests::TestCounts>,
) -> String {
    if counts.is_empty() {
        return "| Subsystem | Tests | Ignored |\n\
               |-----------|-------|---------|\n\
               | — | no data yet | — |"
            .to_string();
    }

    let mut lines = vec![
        "| Subsystem | Tests | Ignored |".to_string(),
        "|-----------|-------|---------|".to_string(),
    ];

    for (subsystem, counts) in counts {
        let name = match subsystem {
            super::StatusSubsystem::Parser => "Parser",
            super::StatusSubsystem::Quality => "Quality",
            super::StatusSubsystem::Lsp => "LSP",
            super::StatusSubsystem::Dap => "DAP",
            super::StatusSubsystem::Workspace => "Workspace",
            super::StatusSubsystem::Tests => "Tests",
        };
        let tests = counts.tier_a_lib_tests.map_or("-".to_string(), |n| n.to_string());
        let ignored = counts.ignored_total.map_or("-".to_string(), |n| n.to_string());
        lines.push(format!("| {name} | {tests} | {ignored} |"));
    }

    lines.join("\n")
}

/// Collect subsystem test counts and format as a table.
fn collect_subsystem_test_counts_table(root: &Path) -> String {
    let counts = collect_subsystem_test_counts(root);
    format_subsystem_test_table(&counts)
}

>>>>>>> Stashed changes
pub(super) fn generate_editor_ux_receipt(root: &Path) -> Result<String> {
    let scenario_files = collect_ux_scenario_files(root);
    let scenario_count = scenario_files.len();

    let receipt = serde_json::json!({
        "schema_version": 1,
        "receipt_kind": "planning_scaffold",
        "scorecard": "editor_ux",
        "harness": {
            "crate": "crates/perl-lsp-ux-tests",
            "scenario_count": scenario_count,
            "scenario_files": scenario_files,
        },
        "top_line_metrics": [
            {
                "name": "workflow_pass_rate",
                "state": "planned",
                "owner": "perl-lsp-ux-tests",
            },
            {
                "name": "workflow_stability_rate",
                "state": "planned",
                "owner": "perl-lsp-ux-tests",
            },
            {
                "name": "p95_time_to_first_useful_result_ms",
                "state": "planned",
                "owner": "perl-lsp-ux-tests",
            },
        ],
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
<<<<<<< Updated upstream
=======

    #[test]
    fn test_collect_latency_by_subsystem_handles_missing_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let result = collect_latency_by_subsystem(temp_dir.path());
        assert!(
            result.is_empty(),
            "collect_latency_by_subsystem should return empty map when benchmark file missing"
        );
    }

    #[test]
    fn test_format_latency_table_renders_markdown() {
        let mut stats = BTreeMap::new();
        stats.insert("parser".to_string(), LatencyStats { p50_ms: 1.0, p95_ms: 5.0, p99_ms: 10.0 });
        stats.insert("lsp".to_string(), LatencyStats { p50_ms: 2.0, p95_ms: 8.0, p99_ms: 20.0 });

        let table = format_latency_table(&stats);

        assert!(table.contains("Category"), "Table must have Category column");
        assert!(table.contains("p50"), "Table must have p50 column");
        assert!(table.contains("p95"), "Table must have p95 column");
        assert!(table.contains("p99"), "Table must have p99 column");
        assert!(table.contains("parser"), "Table must contain parser category");
        assert!(table.contains("lsp"), "Table must contain lsp category");
        assert!(table.contains("1.00"), "Table should contain p50 value for parser");
    }

    #[test]
    fn test_format_latency_table_handles_empty() {
        let stats = BTreeMap::new();
        let table = format_latency_table(&stats);
        assert!(table.contains("Category"), "Empty table should still have header");
        assert!(table.contains("no benchmark data yet"), "Empty table should show no data message");
    }

    #[test]
    fn test_latency_stats_struct() {
        let stats = LatencyStats { p50_ms: 1.0, p95_ms: 5.0, p99_ms: 10.0 };
        assert_eq!(stats.p50_ms, 1.0);
        assert_eq!(stats.p95_ms, 5.0);
        assert_eq!(stats.p99_ms, 10.0);
    }

    #[test]
    fn test_format_subsystem_test_table_has_header() {
        let mut counts = BTreeMap::new();
        counts.insert(
            crate::tasks::update_status::StatusSubsystem::Parser,
            crate::tasks::update_status::tests::TestCounts {
                tier_a_lib_tests: Some(100),
                ignored_total: Some(5),
                bug_count: None,
                manual_count: None,
            },
        );
        let table = format_subsystem_test_table(&counts);
        assert!(table.contains("Subsystem"), "Missing Subsystem column");
        assert!(table.contains("Tests"), "Missing Tests column");
        assert!(table.contains("Ignored"), "Missing Ignored column");
        assert!(table.contains("Parser"), "Missing Parser row");
        assert!(table.contains("100"), "Missing test count");
    }

    #[test]
    fn test_format_subsystem_test_table_empty() {
        let table = format_subsystem_test_table(&BTreeMap::new());
        assert!(table.contains("Subsystem"), "Empty table should have header");
        assert!(table.contains("no data yet"), "Empty table should show no data message");
    }
>>>>>>> Stashed changes
}
