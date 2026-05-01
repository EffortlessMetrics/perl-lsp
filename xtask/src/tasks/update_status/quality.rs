//! Quality subsystem status generator.
//!
//! Owns per-crate mutation and lib-test counts; delegates UX receipt generation
//! to `editor_ux` and flaky-test tracking to `flaky`.

// LazyLock<Regex> initializers use .expect() for known-good patterns — permitted by coding standards.
#![allow(clippy::expect_used)]

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;
use std::time::Duration;

use color_eyre::eyre::Result;
use regex::Regex;

use super::editor_ux::count_ux_scenarios;
use super::flaky::{collect_flaky_test_summary, format_flaky_tests_section};
use super::{replace_block, run_cmd_merged};

static RUNNING_TEST_BINARY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"Running unittests[^\(]*\(target[^\)]*deps[/\\]([a-zA-Z0-9_-]+)-[0-9a-f]+(?:\.exe)?\)",
    )
    .expect("running-test regex is valid")
});

static TEST_LIST_LINE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r":\s*test\s*$").expect("test-list-line regex is valid"));

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
        if let Some(pkg) = entry.get("package").and_then(|v| v.as_str())
            && !pkg.trim().is_empty()
        {
            *by_crate.entry(pkg.to_string()).or_default() += 1;
        }
    }
    by_crate
}

/// Run `cargo test --workspace --lib -- --list` and return a map of crate-name → test count.
///
/// `cargo test -- --list` writes crate headers ("Running unittests …") to stderr and test
/// names to stdout.  `run_cmd_merged` (shell `2>&1`) ensures headers appear immediately
/// before the test names they introduce, so the parser correctly associates each name with
/// its crate.
pub(super) fn collect_per_crate_test_counts(root: &Path) -> BTreeMap<String, usize> {
    let output = run_cmd_merged(
        root,
        &["cargo", "test", "--workspace", "--lib", "--exclude", "tree-sitter-perl", "--", "--list"],
        Duration::from_secs(180),
    );
    if output.is_empty() {
        return BTreeMap::new();
    }
    parse_per_crate_test_counts(&output)
}

fn parse_per_crate_test_counts(output: &str) -> BTreeMap<String, usize> {
    let mut by_crate: BTreeMap<String, usize> = BTreeMap::new();
    let mut current_crate: Option<String> = None;
    for line in output.lines() {
        if let Some(caps) = RUNNING_TEST_BINARY_RE.captures(line) {
            current_crate = Some(caps[1].replace('_', "-"));
            continue;
        }
        if TEST_LIST_LINE_RE.is_match(line)
            && let Some(ref krate) = current_crate
        {
            *by_crate.entry(krate.clone()).or_default() += 1;
        }
    }
    by_crate
}

/// Format a per-crate markdown table showing mutation count and test count.
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
    for c in crates {
        let m = mutation.get(c).map_or_else(|| "—".to_string(), |n| n.to_string());
        let t = tests.get(c).map_or_else(|| "—".to_string(), |n| n.to_string());
        lines.push(format!("| {c} | {m} | {t} |"));
    }
    lines.join("\n")
}

// ---------------------------------------------------------------------------
// Generators
// ---------------------------------------------------------------------------

pub(super) fn generate_quality_status(root: &Path, original: &str) -> Result<String> {
    let mutation_by_crate = collect_per_crate_mutation(root);
    let tests_by_crate = collect_per_crate_test_counts(root);
    let ux_scenarios = count_ux_scenarios(root);
    let flaky = collect_flaky_test_summary(root);

    let mutation_note = if mutation_by_crate.is_empty() {
        "mutation data pending first nightly CI run — run `just mutation-subset` locally to populate"
    } else {
        "per-crate data from `mutants.out/mutants.json` (written by nightly CI `cargo mutants` run)"
    };

    let bullets = format!(
        "- **Quality Metrics**: <50ms LSP response times, 931ns incremental parsing\n\
         - **UX workflow harness**: {ux_scenarios} scenario files in `perl-lsp-ux-tests`; \
           `just ux-tests` runs the default release-confidence lane and `just ux-tests-full` adds \
           the integration-only 10k-line large-file case; confidence signals (manual smoke, \
           first-5-minutes coverage, issue-burndown regression guards) are tracked in \
           `docs/project/status/editor_ux.json`\n\
         - **Mutation testing**: {mutation_note}\n\
         - **Lexer performance scorecard**: `cargo bench -p perl-lexer --bench lexer_benchmarks` writes `benchmarks/results/lexer_scorecard.json` for trend comparisons
\
         - **Production Status**: LSP server public alpha (`just ci-gate` passing)"
    );

    let crate_table = format_crate_quality_table(&mutation_by_crate, &tests_by_crate);
    let flaky_section = format_flaky_tests_section(&flaky);

    let mut text = original.to_string();
    text = replace_block(
        &text,
        "<!-- BEGIN: QUALITY_METRICS_BULLETS -->",
        "<!-- END: QUALITY_METRICS_BULLETS -->",
        &bullets,
    )?;
    text = replace_block(
        &text,
        "<!-- BEGIN: QUALITY_CRATE_TABLE -->",
        "<!-- END: QUALITY_CRATE_TABLE -->",
        &crate_table,
    )?;
    text = replace_block(
        &text,
        "<!-- BEGIN: FLAKY_TESTS_SUMMARY -->",
        "<!-- END: FLAKY_TESTS_SUMMARY -->",
        &flaky_section,
    )?;
    Ok(text)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(result.get("perl-quote"), Some(&2));
        assert_eq!(result.get("perl-parser"), Some(&1));
        Ok(())
    }

    #[test]
    fn test_collect_per_crate_mutation_ignores_entries_without_package() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let out_dir = dir.path().join("mutants.out");
        fs::create_dir_all(&out_dir)?;
        let json = r#"[
            {"package":"perl-quote","file":"crates/perl-quote/src/lib.rs"},
            {"file":"crates/perl-parser/src/lib.rs","genre":"FnValue"},
            {"package":null,"file":"crates/perl-parser/src/lib.rs"},
            {"package":"perl-quote","file":"crates/perl-quote/src/lib.rs"}
        ]"#;
        fs::write(out_dir.join("mutants.json"), json)?;
        let result = collect_per_crate_mutation(dir.path());
        assert_eq!(result.len(), 1);
        assert_eq!(result.get("perl-quote"), Some(&2));
        Ok(())
    }

    #[test]
    fn test_collect_per_crate_mutation_ignores_blank_package_names() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let out_dir = dir.path().join("mutants.out");
        fs::create_dir_all(&out_dir)?;
        let json = r#"[
            {"package":"perl-quote","file":"crates/perl-quote/src/lib.rs"},
            {"package":"","file":"crates/perl-parser/src/lib.rs"},
            {"package":"   ","file":"crates/perl-parser/src/lib.rs"}
        ]"#;
        fs::write(out_dir.join("mutants.json"), json)?;
        let result = collect_per_crate_mutation(dir.path());
        assert_eq!(result.len(), 1);
        assert_eq!(result.get("perl-quote"), Some(&1));
        Ok(())
    }

    #[test]
    fn test_collect_per_crate_mutation_invalid_json_returns_empty_map() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let out_dir = dir.path().join("mutants.out");
        fs::create_dir_all(&out_dir)?;
        fs::write(out_dir.join("mutants.json"), "{not-json")?;
        let result = collect_per_crate_mutation(dir.path());
        assert!(result.is_empty());
        Ok(())
    }

    #[test]
    fn test_format_crate_quality_table_has_header_and_data() {
        let mut mutation = BTreeMap::new();
        mutation.insert("perl-quote".to_string(), 249);
        let mut tests = BTreeMap::new();
        tests.insert("perl-quote".to_string(), 42);
        let table = format_crate_quality_table(&mutation, &tests);
        assert!(
            table.contains("Crate")
                && table.contains("perl-quote")
                && table.contains("249")
                && table.contains("42")
        );
    }

    #[test]
    fn test_format_crate_quality_table_empty_maps() {
        let table = format_crate_quality_table(&BTreeMap::new(), &BTreeMap::new());
        assert!(table.contains("no data yet"));
    }

    #[test]
    fn test_parse_per_crate_test_counts_parses_unix_and_windows_paths() {
        let output = "Running unittests src/lib.rs \
            (target/debug/deps/perl_parser_core-abc123)\n\
            lexer_edge_case: test\nparser_smoke: test\n\
            Running unittests src/lib.rs \
            (target\\debug\\deps\\perl_workspace_index-123def.exe)\n\
            index_builds: test\n";
        let counts = parse_per_crate_test_counts(output);
        assert_eq!(counts.get("perl-parser-core"), Some(&2));
        assert_eq!(counts.get("perl-workspace-index"), Some(&1));
    }

    #[test]
    fn test_parse_per_crate_test_counts_ignores_tests_without_active_crate() {
        let output = "orphan_test: test\n\
            Running unittests src/lib.rs (target/debug/deps/perl_parser_core-abc123)\n\
            parser_smoke: test\n\
            note: test\n\
            Running unittests src/lib.rs (target/debug/deps/perl_lexer-987def)\n\
            lexer_smoke: test\n";
        let counts = parse_per_crate_test_counts(output);
        assert_eq!(counts.get("perl-parser-core"), Some(&2));
        assert_eq!(counts.get("perl-lexer"), Some(&1));
        assert_eq!(counts.len(), 2);
    }
}
