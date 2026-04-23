//! Quality subsystem status generator.
//!
//! Owns per-crate mutation and test counts, UX scenario receipt, and quality.md generation.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;
use std::time::Duration;

use color_eyre::eyre::{Context, Result};
use regex::Regex;

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

#[derive(Debug, Clone, PartialEq, Eq)]
struct UxP0Burndown {
    total: usize,
    merged: usize,
    open: usize,
}

fn manual_smoke_workflow_count(root: &Path) -> usize {
    let verification = root.join("docs/project/protocols/verification.md");
    let Ok(raw) = fs::read_to_string(verification) else {
        return 0;
    };

    static MANUAL_SMOKE_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"Manual editor smoke test:\s*(?P<steps>[^\n]+)")
            .expect("manual smoke regex must compile")
    });

    let Some(caps) = MANUAL_SMOKE_RE.captures(&raw) else {
        return 0;
    };

    let Some(steps) = caps.name("steps").map(|m| m.as_str()) else {
        return 0;
    };

    steps.split(',').map(str::trim).filter(|step| !step.is_empty()).count()
}

fn ux_p0_burndown(root: &Path) -> Option<UxP0Burndown> {
    let checklist = root.join("docs/project/PRE_ANNOUNCEMENT_CHECKLIST.md");
    let raw = fs::read_to_string(checklist).ok()?;

    static UX_P0_TOTAL_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"Current status:\s*⚠\s*—\s*(\d+)\s+UX P0 items tracked")
            .expect("ux p0 total regex must compile")
    });
    static UX_P0_MERGED_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\s*-\s*✓\s*#\d+").expect("ux p0 merged regex must compile"));
    static UX_P0_OPEN_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\s*-\s*⚠\s*#\d+").expect("ux p0 open regex must compile"));

    let total = UX_P0_TOTAL_RE.captures(&raw)?.get(1)?.as_str().parse::<usize>().ok()?;

    let mut merged = 0;
    let mut open = 0;
    for line in raw.lines() {
        if UX_P0_MERGED_RE.is_match(line) {
            merged += 1;
        }
        if UX_P0_OPEN_RE.is_match(line) {
            open += 1;
        }
    }

    Some(UxP0Burndown { total, merged, open })
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

pub(super) fn generate_editor_ux_receipt(root: &Path) -> Result<String> {
    let scenario_files = collect_ux_scenario_files(root);
    let scenario_count = scenario_files.len();
    let manual_smoke_steps = manual_smoke_workflow_count(root);
    let ux_p0 = ux_p0_burndown(root);

    let receipt = serde_json::json!({
        "schema_version": 1,
        "receipt_kind": "tracked_signals",
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
        "confidence_signals": {
            "manual_editor_smoke": {
                "workflow_count": manual_smoke_steps,
                "source": "docs/project/protocols/verification.md",
                "tracking": "counted from Tier C workflow list",
            },
            "first_5_minutes_harness": {
                "scenario_count": scenario_count,
                "source": "crates/perl-lsp-ux-tests/tests",
                "tracking": "counted from ux_scenario_*.rs files",
            },
            "ux_p0_issue_burndown": {
                "tracked_total": ux_p0.as_ref().map(|v| v.total),
                "merged": ux_p0.as_ref().map(|v| v.merged),
                "open": ux_p0.as_ref().map(|v| v.open),
                "source": "docs/project/PRE_ANNOUNCEMENT_CHECKLIST.md",
                "tracking": "counted from UX P0 status bullets",
            },
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
        assert_eq!(receipt["receipt_kind"], "tracked_signals");
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
        assert_eq!(
            receipt["confidence_signals"]["manual_editor_smoke"]["workflow_count"].as_u64(),
            Some(manual_smoke_workflow_count(&root) as u64)
        );
        Ok(())
    }

    #[test]
    fn test_manual_smoke_workflow_count_parses_tier_c_list() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let protocol_path = dir.path().join("docs/project/protocols");
        fs::create_dir_all(&protocol_path)?;
        fs::write(
            protocol_path.join("verification.md"),
            "## Tier C\nManual editor smoke test: diagnostics, completion, hover, go-to-definition, rename\n",
        )?;
        assert_eq!(manual_smoke_workflow_count(dir.path()), 5);
        Ok(())
    }

    #[test]
    fn test_ux_p0_burndown_extracts_counts() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let docs_path = dir.path().join("docs/project");
        fs::create_dir_all(&docs_path)?;
        fs::write(
            docs_path.join("PRE_ANNOUNCEMENT_CHECKLIST.md"),
            "Current status: ⚠ — 7 UX P0 items tracked; status as of 2026-04-08:\n- ✓ #1 merged\n- ⚠ #2 open\n- ⚠ #3 open\n",
        )?;
        let parsed = ux_p0_burndown(dir.path()).ok_or_else(|| eyre!("expected ux p0 parse"))?;
        assert_eq!(parsed.total, 7);
        assert_eq!(parsed.merged, 1);
        assert_eq!(parsed.open, 2);
        Ok(())
    }
}
