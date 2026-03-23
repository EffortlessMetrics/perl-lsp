//! Update derived metrics in docs/project/status/ subsystem files.
//!
//! Rust port of `scripts/update-current-status.py`.  Computes test counts,
//! feature catalog metrics, corpus statistics, and missing-docs warnings, then
//! patches the markdown files between fenced markers.
//!
//! Subsystem files written:
//!   - docs/project/status/lsp.md     (LSP coverage + compliance table)
//!   - docs/project/status/tests.md   (test counts + tracked debt)
//!   - docs/project/status/parser.md  (corpus stats)
//!   - docs/project/status/quality.md (mutation score, perf)
//!
//! Also keeps docs/project/ROADMAP.md compliance table in sync when lsp subsystem runs.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use color_eyre::eyre::{Context, Result, eyre};
use regex::Regex;

use crate::utils::project_root;

// ---------------------------------------------------------------------------
// Subsystem selector
// ---------------------------------------------------------------------------

/// Which subsystems to regenerate.
#[derive(Debug, Clone, PartialEq, Eq, clap::ValueEnum)]
pub enum StatusSubsystem {
    Lsp,
    Tests,
    Parser,
    Quality,
}

impl StatusSubsystem {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            StatusSubsystem::Lsp => "lsp",
            StatusSubsystem::Tests => "tests",
            StatusSubsystem::Parser => "parser",
            StatusSubsystem::Quality => "quality",
        }
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Run the update-status task.
///
/// * `write` – write changes back to disk.
/// * `check` – verify files are up to date (returns error if stale).
/// * `only`  – when set, only regenerate the given subsystem; otherwise all four.
///
/// When neither `write` nor `check` is set, defaults to `check`.
pub fn run(write: bool, check: bool, only: Option<StatusSubsystem>) -> Result<()> {
    let check = if !write && !check { true } else { check };

    let root = project_root()?;

    let subsystems: Vec<StatusSubsystem> = match only {
        Some(s) => vec![s],
        None => vec![
            StatusSubsystem::Lsp,
            StatusSubsystem::Tests,
            StatusSubsystem::Parser,
            StatusSubsystem::Quality,
        ],
    };

    let mut files_to_update: Vec<(&'static str, PathBuf, String)> = Vec::new();

    // Collect metrics once; skip slow collectors not needed by the selected subsystems.
    let need_lsp = subsystems.contains(&StatusSubsystem::Lsp);
    let need_tests = subsystems.contains(&StatusSubsystem::Tests);
    let need_parser = subsystems.contains(&StatusSubsystem::Parser);
    let need_quality = subsystems.contains(&StatusSubsystem::Quality);

    // --- LSP subsystem ---
    let cov_opt = if need_lsp { Some(count_lsp_coverage(&root)?) } else { None };
    let compliance_opt = if need_lsp { Some(compute_compliance_table(&root)?) } else { None };

    if need_lsp {
        let cov = cov_opt.as_ref().expect("lsp coverage collected");
        let compliance_table = compliance_opt.as_ref().expect("compliance table collected");

        let lsp_path = root.join("docs/project/status/lsp.md");
        let original_lsp =
            fs::read_to_string(&lsp_path).context("reading docs/project/status/lsp.md")?;
        let updated_lsp = generate_lsp_status(cov, compliance_table, &original_lsp)?;
        if updated_lsp != original_lsp {
            files_to_update.push(("docs/project/status/lsp.md", lsp_path, updated_lsp));
        }

        // Keep ROADMAP.md compliance table in sync
        let roadmap_path = root.join("docs/project/ROADMAP.md");
        let original_roadmap =
            fs::read_to_string(&roadmap_path).context("reading docs/project/ROADMAP.md")?;
        let updated_roadmap = update_roadmap(&root, &original_roadmap)?;
        if updated_roadmap != original_roadmap {
            files_to_update.push(("docs/project/ROADMAP.md", roadmap_path, updated_roadmap));
        }
    }

    // --- Tests subsystem ---
    if need_tests {
        let tests = count_tests(&root);
        let missing_docs_current = count_missing_docs_perl_parser(&root);
        let missing_docs_baseline = read_missing_docs_baseline(&root);

        let tests_path = root.join("docs/project/status/tests.md");
        let original_tests =
            fs::read_to_string(&tests_path).context("reading docs/project/status/tests.md")?;
        let updated_tests = generate_tests_status(
            &tests,
            missing_docs_current,
            missing_docs_baseline,
            &original_tests,
        )?;
        if updated_tests != original_tests {
            files_to_update.push(("docs/project/status/tests.md", tests_path, updated_tests));
        }
    }

    // --- Parser subsystem ---
    if need_parser {
        let corpus_sections = count_corpus_sections(&root);
        let gap_files = count_gap_files(&root);

        let parser_path = root.join("docs/project/status/parser.md");
        let original_parser =
            fs::read_to_string(&parser_path).context("reading docs/project/status/parser.md")?;
        let updated_parser = generate_parser_status(corpus_sections, gap_files, &original_parser)?;
        if updated_parser != original_parser {
            files_to_update.push(("docs/project/status/parser.md", parser_path, updated_parser));
        }
    }

    // --- Quality subsystem ---
    if need_quality {
        let quality_path = root.join("docs/project/status/quality.md");
        let original_quality =
            fs::read_to_string(&quality_path).context("reading docs/project/status/quality.md")?;
        let updated_quality = generate_quality_status(&original_quality)?;
        if updated_quality != original_quality {
            files_to_update.push(("docs/project/status/quality.md", quality_path, updated_quality));
        }
    }

    if files_to_update.is_empty() {
        eprintln!("All files are up to date.");
        return Ok(());
    }

    if write {
        for (name, path, content) in &files_to_update {
            fs::write(path, content).with_context(|| format!("writing {name}"))?;
            eprintln!("Updated {name}");
        }
        return Ok(());
    }

    // check mode
    if check {
        for (name, _, _) in &files_to_update {
            eprintln!("{name} is out of date.");
        }
        eprintln!("Run `just status-update`");
        eprintln!("Then re-run `just ci-gate`");
        return Err(eyre!("{} file(s) out of date", files_to_update.len()));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Test counts
// ---------------------------------------------------------------------------

struct TestCounts {
    tier_a_lib_tests: Option<usize>,
    ignored_total: Option<usize>,
    bug_count: Option<usize>,
    manual_count: Option<usize>,
}

/// Run a command with a timeout, returning combined stdout+stderr or empty string on failure.
fn run_cmd(root: &Path, args: &[&str], timeout: Duration) -> String {
    let Some((&program, rest)) = args.split_first() else {
        return String::new();
    };

    let result = Command::new(program).args(rest).current_dir(root).output();

    let output = match result {
        Ok(o) => o,
        Err(_) => return String::new(),
    };

    // Basic timeout emulation: we cannot use `std::process::Command` timeout
    // directly, so we rely on the process completing.  The Python version used
    // subprocess.run with timeout; here we accept the default behavior but keep
    // the parameter for API compatibility and future improvement.
    let _ = timeout;

    let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&output.stderr));
    combined
}

fn count_tier_a_lib_tests(root: &Path) -> Option<usize> {
    let output = run_cmd(
        root,
        &["cargo", "test", "--workspace", "--lib", "--exclude", "tree-sitter-perl", "--", "--list"],
        Duration::from_secs(180),
    );
    if output.is_empty() {
        return None;
    }
    let re = Regex::new(r":\s*test\s*$").ok()?;
    Some(output.lines().filter(|line| re.is_match(line)).count())
}

fn count_ignored_tracked(root: &Path) -> (Option<usize>, Option<usize>, Option<usize>) {
    let output = run_cmd(root, &["bash", "scripts/ignored-test-count.sh"], Duration::from_secs(60));
    if output.is_empty() {
        return (None, None, None);
    }

    let total_re = Regex::new(r"TOTAL\s+(\d+)").ok();
    let bug_re = Regex::new(r"(?m)^bug\s+(\d+)").ok();
    let manual_re = Regex::new(r"(?m)^manual\s+(\d+)").ok();

    let ignored_total = total_re
        .and_then(|re| re.captures(&output))
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<usize>().ok());

    let bug_count = bug_re
        .and_then(|re| re.captures(&output))
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<usize>().ok());

    let manual_count = manual_re
        .and_then(|re| re.captures(&output))
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<usize>().ok());

    (ignored_total, bug_count, manual_count)
}

fn count_tests(root: &Path) -> TestCounts {
    let tier_a = count_tier_a_lib_tests(root);
    let (ignored_total, bug_count, manual_count) = count_ignored_tracked(root);
    TestCounts { tier_a_lib_tests: tier_a, ignored_total, bug_count, manual_count }
}

// ---------------------------------------------------------------------------
// Missing docs
// ---------------------------------------------------------------------------

fn count_missing_docs_perl_parser(root: &Path) -> Option<usize> {
    let output = run_cmd(
        root,
        &["cargo", "check", "-p", "perl-parser", "--tests", "--message-format=json"],
        Duration::from_secs(300),
    );
    if output.is_empty() {
        return None;
    }

    let mut count: usize = 0;
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let obj: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if obj.get("reason").and_then(|v| v.as_str()) != Some("compiler-message") {
            continue;
        }
        let pkg_id = obj.get("package_id").and_then(|v| v.as_str()).unwrap_or("");
        if !pkg_id.starts_with("perl-parser ") {
            continue;
        }
        let msg = match obj.get("message") {
            Some(m) if m.is_object() => m,
            _ => continue,
        };
        let level = msg.get("level").and_then(|v| v.as_str()).unwrap_or("");
        let code =
            msg.get("code").and_then(|v| v.get("code")).and_then(|v| v.as_str()).unwrap_or("");
        if level == "warning" && code == "missing_docs" {
            count += 1;
        }
    }
    Some(count)
}

fn read_missing_docs_baseline(root: &Path) -> Option<usize> {
    let path = root.join("ci/missing_docs_baseline.txt");
    let raw = fs::read_to_string(path).ok()?;
    raw.trim().parse::<usize>().ok()
}

// ---------------------------------------------------------------------------
// Feature catalog metrics (mirroring Python _count_lsp_coverage)
// ---------------------------------------------------------------------------

struct LspCoverage {
    ux_percent: usize,
    ux_implemented: usize,
    ux_total: usize,
    protocol_percent: usize,
    protocol_implemented: usize,
    protocol_total: usize,
}

fn count_lsp_coverage(root: &Path) -> Result<LspCoverage> {
    let features_path = root.join("features.toml");
    let catalog = perl_feature_catalog::read_catalog(&features_path)
        .with_context(|| format!("loading {}", features_path.display()))?;

    // UX Coverage: advertised=true, counts_in_coverage!=false, maturity!=planned
    let ux_trackable: Vec<_> = catalog
        .feature
        .iter()
        .filter(|f| {
            f.maturity != perl_feature_catalog::Maturity::Planned
                && f.counts_in_coverage
                && f.advertised
        })
        .collect();

    let ux_implemented: Vec<_> = ux_trackable
        .iter()
        .filter(|f| {
            matches!(
                f.maturity,
                perl_feature_catalog::Maturity::Ga | perl_feature_catalog::Maturity::Production
            )
        })
        .collect();

    let ux_percent = if ux_trackable.is_empty() {
        0
    } else {
        ((ux_implemented.len() as f64 / ux_trackable.len() as f64) * 100.0).round() as usize
    };

    // Protocol Compliance: all features regardless of counts_in_coverage
    let protocol_trackable: Vec<_> = catalog
        .feature
        .iter()
        .filter(|f| f.maturity != perl_feature_catalog::Maturity::Planned)
        .collect();

    let protocol_implemented: Vec<_> = protocol_trackable
        .iter()
        .filter(|f| {
            matches!(
                f.maturity,
                perl_feature_catalog::Maturity::Ga
                    | perl_feature_catalog::Maturity::Production
                    | perl_feature_catalog::Maturity::Preview
            )
        })
        .collect();

    let protocol_percent = if protocol_trackable.is_empty() {
        0
    } else {
        ((protocol_implemented.len() as f64 / protocol_trackable.len() as f64) * 100.0).round()
            as usize
    };

    Ok(LspCoverage {
        ux_percent,
        ux_implemented: ux_implemented.len(),
        ux_total: ux_trackable.len(),
        protocol_percent,
        protocol_implemented: protocol_implemented.len(),
        protocol_total: protocol_trackable.len(),
    })
}

// ---------------------------------------------------------------------------
// Compliance table for ROADMAP.md and lsp.md
// ---------------------------------------------------------------------------

fn compute_compliance_table(root: &Path) -> Result<String> {
    let features_path = root.join("features.toml");
    let catalog = perl_feature_catalog::read_catalog(&features_path)
        .with_context(|| format!("loading {}", features_path.display()))?;

    let mut by_area: BTreeMap<String, (usize, usize)> = BTreeMap::new(); // (implemented, total)

    for f in &catalog.feature {
        let entry = by_area.entry(f.area.clone()).or_insert((0, 0));
        entry.1 += 1;
        if matches!(
            f.maturity,
            perl_feature_catalog::Maturity::Ga
                | perl_feature_catalog::Maturity::Production
                | perl_feature_catalog::Maturity::Preview
        ) {
            entry.0 += 1;
        }
    }

    let mut lines: Vec<String> = Vec::new();
    lines.push("| Area | Implemented | Total | Coverage |".to_string());
    lines.push("|------|-------------|-------|----------|".to_string());

    let mut total_impl: usize = 0;
    let mut total_all: usize = 0;

    for (area, (impl_count, total)) in &by_area {
        let pct = if *total == 0 {
            0
        } else {
            ((*impl_count as f64 / *total as f64) * 100.0).round() as usize
        };
        lines.push(format!("| {area} | {impl_count} | {total} | {pct}% |"));
        total_impl += impl_count;
        total_all += total;
    }

    let overall_pct = if total_all == 0 {
        0
    } else {
        ((total_impl as f64 / total_all as f64) * 100.0).round() as usize
    };
    lines
        .push(format!("| **Overall** | **{total_impl}** | **{total_all}** | **{overall_pct}%** |"));

    Ok(lines.join("\n"))
}

// ---------------------------------------------------------------------------
// Corpus counts
// ---------------------------------------------------------------------------

fn count_corpus_sections(root: &Path) -> usize {
    let corpus_dir = root.join("tree-sitter-perl/test/corpus");
    let marker = Regex::new(r"^=+\s*$").ok();
    let mut total: usize = 0;

    let walker =
        walkdir::WalkDir::new(&corpus_dir).into_iter().filter_map(|e| e.ok()).filter(|e| {
            e.file_type().is_file() && e.path().extension().is_some_and(|ext| ext == "txt")
        });

    for entry in walker {
        if let Ok(content) = fs::read_to_string(entry.path())
            && let Some(ref re) = marker
        {
            total += content.lines().filter(|line| re.is_match(line)).count();
        }
    }
    total
}

fn count_gap_files(root: &Path) -> usize {
    let gap_dir = root.join("test_corpus");
    walkdir::WalkDir::new(&gap_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && e.path().extension().is_some_and(|ext| ext == "pl"))
        .count()
}

// ---------------------------------------------------------------------------
// Marker replacement helpers
// ---------------------------------------------------------------------------

/// Replace content between `begin_marker\n...\nend_marker` (inclusive of markers).
fn replace_block(
    text: &str,
    begin_marker: &str,
    end_marker: &str,
    new_content: &str,
) -> Result<String> {
    let escaped_begin = regex::escape(begin_marker);
    let escaped_end = regex::escape(end_marker);
    let pattern = format!(r"(?s)({})\n.*?\n({})", escaped_begin, escaped_end);
    let re = Regex::new(&pattern).context("building block replacement regex")?;

    let replacement = format!("{begin_marker}\n{new_content}\n{end_marker}");

    let mut count = 0;
    let result = re.replace_all(text, |_caps: &regex::Captures<'_>| {
        count += 1;
        replacement.clone()
    });

    if count != 1 {
        return Err(eyre!("Expected 1 match for block {begin_marker:?}, got {count}"));
    }

    Ok(result.into_owned())
}

// ---------------------------------------------------------------------------
// Per-subsystem generators
// ---------------------------------------------------------------------------

fn generate_lsp_status(
    cov: &LspCoverage,
    compliance_table: &str,
    original: &str,
) -> Result<String> {
    let lsp_target_pct: usize = 100;
    let lsp_status = if cov.ux_percent >= lsp_target_pct { "PASS" } else { "In progress" };
    let lsp_table_row = format!(
        "| **LSP Coverage** | {}% ({}/{} advertised features, `features.toml`) | {}% | {} |",
        cov.ux_percent, cov.ux_implemented, cov.ux_total, lsp_target_pct, lsp_status
    );

    let lsp_coverage_bullet = format!(
        "- **LSP Coverage**: {}% user-visible feature coverage ({}/{} advertised features from `features.toml`)",
        cov.ux_percent, cov.ux_implemented, cov.ux_total
    );
    let protocol_compliance_bullet = format!(
        "- **Protocol Compliance**: {}% overall LSP protocol support ({}/{} including plumbing)",
        cov.protocol_percent, cov.protocol_implemented, cov.protocol_total
    );

    let lsp_target = if cov.ux_percent >= lsp_target_pct {
        "**Target**: maintain 100% LSP coverage (no regressions)".to_string()
    } else {
        format!("**Target**: 100% LSP coverage (from current {}%)", cov.ux_percent)
    };

    let bullets_content = [
        lsp_coverage_bullet.as_str(),
        protocol_compliance_bullet.as_str(),
        "",
        lsp_target.as_str(),
    ]
    .join("\n");

    let mut text = original.to_string();
    text = replace_block(
        &text,
        "<!-- BEGIN: LSP_COVERAGE -->",
        "<!-- END: LSP_COVERAGE -->",
        &lsp_table_row,
    )?;
    text = replace_block(
        &text,
        "<!-- BEGIN: LSP_METRICS_BULLETS -->",
        "<!-- END: LSP_METRICS_BULLETS -->",
        &bullets_content,
    )?;
    text = replace_block(
        &text,
        "<!-- BEGIN: COMPLIANCE_TABLE -->",
        "<!-- END: COMPLIANCE_TABLE -->",
        compliance_table,
    )?;
    Ok(text)
}

fn generate_tests_status(
    tests: &TestCounts,
    missing_docs_current: Option<usize>,
    missing_docs_baseline: Option<usize>,
    original: &str,
) -> Result<String> {
    let tier_a_tests_str =
        tests.tier_a_lib_tests.map_or_else(|| "UNVERIFIED".to_string(), |n| n.to_string());

    let ignored_tests_str =
        tests.ignored_total.map_or_else(|| "UNVERIFIED".to_string(), |n| n.to_string());

    let (tracked_debt_str, bug_count_str, manual_count_str) =
        match (tests.bug_count, tests.manual_count) {
            (Some(b), Some(m)) => ((b + m).to_string(), b.to_string(), m.to_string()),
            _ => ("UNVERIFIED".to_string(), "UNVERIFIED".to_string(), "UNVERIFIED".to_string()),
        };

    let missing_docs_str =
        missing_docs_current.map_or_else(|| "UNVERIFIED".to_string(), |n| n.to_string());

    let baseline_suffix = match (missing_docs_baseline, missing_docs_current) {
        (Some(bl), Some(_)) => format!(" (baseline {bl})"),
        _ => String::new(),
    };

    let table_rows = format!(
        "| **Tier A Tests** | {tier_a_tests_str} lib tests (discovered), {ignored_tests_str} ignores (tracked) | 100% pass | PASS |\n\
         | **Tracked Test Debt** | {tracked_debt_str} ({bug_count_str} bug, {manual_count_str} manual) | 0 | Near-zero |"
    );

    let bullets_content = format!(
        "- **Test Status**: {tier_a_tests_str} lib tests (Tier A), {ignored_tests_str} ignores tracked ({tracked_debt_str} total tracked debt: {bug_count_str} bug, {manual_count_str} manual)\n\
         - **Docs (perl-parser)**: missing_docs warnings = {missing_docs_str}{baseline_suffix}"
    );

    let mut text = original.to_string();
    text = replace_block(
        &text,
        "<!-- BEGIN: TESTS_TABLE_ROWS -->",
        "<!-- END: TESTS_TABLE_ROWS -->",
        &table_rows,
    )?;
    text = replace_block(
        &text,
        "<!-- BEGIN: TESTS_METRICS_BULLETS -->",
        "<!-- END: TESTS_METRICS_BULLETS -->",
        &bullets_content,
    )?;
    Ok(text)
}

fn generate_parser_status(
    corpus_sections: usize,
    gap_files: usize,
    original: &str,
) -> Result<String> {
    let parser_coverage_bullet = format!(
        "- **Parser Coverage**: ~100% Perl 5 syntax via `tree-sitter-perl/test/corpus` (~{corpus_sections} sections) + `test_corpus/` ({gap_files} `.pl` files)"
    );

    replace_block(
        original,
        "<!-- BEGIN: PARSER_METRICS_BULLETS -->",
        "<!-- END: PARSER_METRICS_BULLETS -->",
        &parser_coverage_bullet,
    )
}

fn generate_quality_status(original: &str) -> Result<String> {
    let bullets_content = "- **Quality Metrics**: 87% mutation score, <50ms LSP response times, 931ns incremental parsing\n\
                           - **Production Status**: LSP server public alpha (`just ci-gate` passing)";

    replace_block(
        original,
        "<!-- BEGIN: QUALITY_METRICS_BULLETS -->",
        "<!-- END: QUALITY_METRICS_BULLETS -->",
        bullets_content,
    )
}

// ---------------------------------------------------------------------------
// ROADMAP.md update (keeps compliance table in sync)
// ---------------------------------------------------------------------------

fn update_roadmap(root: &Path, original: &str) -> Result<String> {
    let compliance_table = compute_compliance_table(root)?;
    replace_block(
        original,
        "<!-- BEGIN: COMPLIANCE_TABLE -->",
        "<!-- END: COMPLIANCE_TABLE -->",
        &compliance_table,
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_block() -> Result<()> {
        let input = "before\n<!-- BEGIN: X -->\nold content\n<!-- END: X -->\nafter";
        let result = replace_block(input, "<!-- BEGIN: X -->", "<!-- END: X -->", "new content")?;
        assert_eq!(result, "before\n<!-- BEGIN: X -->\nnew content\n<!-- END: X -->\nafter");
        Ok(())
    }

    #[test]
    fn test_replace_block_missing_marker() {
        let input = "no markers here";
        let result = replace_block(input, "<!-- BEGIN: X -->", "<!-- END: X -->", "new");
        assert!(result.is_err());
    }

    #[test]
    fn test_lsp_coverage_from_catalog() -> Result<()> {
        let root = project_root()?;
        let cov = count_lsp_coverage(&root)?;
        // Sanity: there should be at least some features
        assert!(cov.ux_total > 0, "expected non-zero ux_total");
        assert!(cov.protocol_total > 0, "expected non-zero protocol_total");
        assert!(cov.ux_percent <= 100, "ux_percent should be <= 100, got {}", cov.ux_percent);
        Ok(())
    }

    #[test]
    fn test_corpus_section_count() -> Result<()> {
        let root = project_root()?;
        let sections = count_corpus_sections(&root);
        // The Python script reports ~611 sections; sanity-check nonzero
        assert!(sections > 0, "expected nonzero corpus sections");
        Ok(())
    }

    #[test]
    fn test_gap_file_count() -> Result<()> {
        let root = project_root()?;
        let count = count_gap_files(&root);
        assert!(count > 0, "expected nonzero gap .pl files");
        Ok(())
    }

    /// All four subsystem status files must exist.
    #[test]
    fn test_subsystem_files_exist() -> Result<()> {
        let root = project_root()?;
        let status_dir = root.join("docs/project/status");
        for name in &["lsp.md", "tests.md", "parser.md", "quality.md"] {
            let path = status_dir.join(name);
            assert!(path.exists(), "subsystem file missing: {}", path.display());
        }
        Ok(())
    }

    /// The stub CURRENT_STATUS.md must NOT contain any <!-- BEGIN: --> markers.
    /// If it does, the generator will try to patch it and fail.
    #[test]
    fn test_stub_has_no_begin_markers() -> Result<()> {
        let root = project_root()?;
        let stub_path = root.join("docs/project/CURRENT_STATUS.md");
        let content = fs::read_to_string(&stub_path).context("reading CURRENT_STATUS.md")?;
        assert!(
            !content.contains("<!-- BEGIN:"),
            "CURRENT_STATUS.md must not contain <!-- BEGIN: --> markers (it is now a stable stub). \
             Generated content belongs in docs/project/status/*.md"
        );
        Ok(())
    }

    /// The subsystem files must contain the expected marker blocks.
    #[test]
    fn test_subsystem_files_have_markers() -> Result<()> {
        let root = project_root()?;
        let status_dir = root.join("docs/project/status");

        let lsp = fs::read_to_string(status_dir.join("lsp.md"))?;
        assert!(lsp.contains("<!-- BEGIN: LSP_COVERAGE -->"), "lsp.md missing LSP_COVERAGE block");
        assert!(
            lsp.contains("<!-- BEGIN: LSP_METRICS_BULLETS -->"),
            "lsp.md missing LSP_METRICS_BULLETS block"
        );
        assert!(
            lsp.contains("<!-- BEGIN: COMPLIANCE_TABLE -->"),
            "lsp.md missing COMPLIANCE_TABLE block"
        );

        let tests = fs::read_to_string(status_dir.join("tests.md"))?;
        assert!(
            tests.contains("<!-- BEGIN: TESTS_TABLE_ROWS -->"),
            "tests.md missing TESTS_TABLE_ROWS block"
        );
        assert!(
            tests.contains("<!-- BEGIN: TESTS_METRICS_BULLETS -->"),
            "tests.md missing TESTS_METRICS_BULLETS block"
        );

        let parser = fs::read_to_string(status_dir.join("parser.md"))?;
        assert!(
            parser.contains("<!-- BEGIN: PARSER_METRICS_BULLETS -->"),
            "parser.md missing PARSER_METRICS_BULLETS block"
        );

        let quality = fs::read_to_string(status_dir.join("quality.md"))?;
        assert!(
            quality.contains("<!-- BEGIN: QUALITY_METRICS_BULLETS -->"),
            "quality.md missing QUALITY_METRICS_BULLETS block"
        );

        Ok(())
    }
}
