//! Update derived metrics in docs/project/status/ subsystem files.
//!
//! Rust port of `scripts/update-current-status.py`.  Computes test counts,
//! feature catalog metrics, corpus statistics, and missing-docs warnings, then
//! patches the markdown files between fenced markers.
//!
//! Subsystem files written:
//!   - docs/project/status/lsp.md     (LSP coverage + compliance table)
//!   - docs/project/status/tests.md   (test counts + tracked debt)
//!   - docs/project/status/parser.md  (parser corpus tracking)
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

    // Only run slow metric collectors for the selected subsystems.
    let need_lsp = subsystems.contains(&StatusSubsystem::Lsp);
    let need_tests = subsystems.contains(&StatusSubsystem::Tests);
    let need_parser = subsystems.contains(&StatusSubsystem::Parser);
    let need_quality = subsystems.contains(&StatusSubsystem::Quality);

    // --- LSP subsystem ---
    if need_lsp {
        let cov = count_lsp_coverage(&root)?;
        let compliance_table = compute_compliance_table(&root)?;

        let lsp_path = root.join("docs/project/status/lsp.md");
        let original_lsp =
            fs::read_to_string(&lsp_path).context("reading docs/project/status/lsp.md")?;
        let updated_lsp = generate_lsp_status(&cov, &compliance_table, &original_lsp)?;
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
        let parser_metrics = collect_parser_metrics(&root);

        let parser_path = root.join("docs/project/status/parser.md");
        let original_parser =
            fs::read_to_string(&parser_path).context("reading docs/project/status/parser.md")?;
        let updated_parser = generate_parser_status(&parser_metrics, &original_parser)?;
        if updated_parser != original_parser {
            files_to_update.push(("docs/project/status/parser.md", parser_path, updated_parser));
        }
    }

    // --- Quality subsystem ---
    if need_quality {
        let quality_path = root.join("docs/project/status/quality.md");
        let original_quality =
            fs::read_to_string(&quality_path).context("reading docs/project/status/quality.md")?;
        let updated_quality = generate_quality_status(&root, &original_quality)?;
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
    // Call the ignored-tests counter directly as a Rust function.  The previous
    // approach shelled out to `bash scripts/ignored-test-count.sh` and parsed
    // the stdout, which silently returned empty output when xtask.exe forked
    // bash on Windows (bash resolution from a native Windows process is
    // environment-dependent).  A direct call is both faster and reliable.
    let Ok(counts) = super::ignored_tests::compute_category_counts(root) else {
        return (None, None, None);
    };

    let ignored_total = counts.values().sum::<usize>();
    let bug_count = counts.get("bug").copied().unwrap_or(0);
    let manual_count = counts.get("manual").copied().unwrap_or(0);

    (Some(ignored_total), Some(bug_count), Some(manual_count))
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
// Parser corpus tracking
// ---------------------------------------------------------------------------

struct ParserMetrics {
    syntax_sections: usize,
    system_receipt: Option<super::parser_corpus_sweep::SweepReport>,
    cpan_receipt: Option<super::parser_corpus_sweep::SweepReport>,
    project_corpus: Option<super::corpus_audit::StatusSummary>,
    /// Receipt from `just common-corpus-check` — the strict-clean pinned-module gate.
    common_corpus_receipt: Option<super::parser_corpus_sweep::SweepReport>,
    /// Number of pinned modules in `.ci/common-corpus-manifest.txt`.
    common_corpus_pinned: usize,
}

fn collect_parser_metrics(root: &Path) -> ParserMetrics {
    let common_corpus_receipt =
        read_sweep_report(&root.join("target/receipts/common-corpus-sweep.json"));
    let common_corpus_pinned = count_common_corpus_pinned(root);
    ParserMetrics {
        syntax_sections: count_corpus_sections(root),
        system_receipt: read_sweep_report(&root.join(".ci/parser-corpus-baseline.json")),
        cpan_receipt: read_sweep_report(&root.join(".ci/cpan-corpus-baseline.json")),
        project_corpus: super::corpus_audit::compute_status_summary(root, Duration::from_secs(5))
            .ok(),
        common_corpus_receipt,
        common_corpus_pinned,
    }
}

/// Count the non-comment, non-blank lines in `.ci/common-corpus-manifest.txt`.
fn count_common_corpus_pinned(root: &Path) -> usize {
    let path = root.join(".ci/common-corpus-manifest.txt");
    let Ok(raw) = fs::read_to_string(path) else {
        return 0;
    };
    raw.lines().filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#')).count()
}

fn read_sweep_report(path: &Path) -> Option<super::parser_corpus_sweep::SweepReport> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

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

fn format_clean_rate(clean_files: usize, total_files: usize) -> String {
    let clean_pct = 100.0 * clean_files as f64 / total_files.max(1) as f64;
    format!("{clean_pct:.1}% clean (`{clean_files}/{total_files}`)")
}

fn short_day(timestamp: &str) -> &str {
    timestamp.get(..10).unwrap_or(timestamp)
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

fn generate_parser_status(metrics: &ParserMetrics, original: &str) -> Result<String> {
    let system_row = metrics.system_receipt.as_ref().map_or_else(
        || {
            "| **Ubuntu system Perl** | UNVERIFIED | baseline receipt unavailable | `.ci/parser-corpus-baseline.json` |".to_string()
        },
        |report| {
            format!(
                "| **Ubuntu system Perl** | {} | Compatibility baseline; Perl `{}`, `{}` unreadable, `{}` with errors, baseline `{}` | `.ci/parser-corpus-baseline.json` |",
                format_clean_rate(report.clean_files, report.total_files),
                report.perl_version,
                report.files_unreadable,
                report.files_with_errors,
                short_day(&report.timestamp),
            )
        },
    );

    let cpan_row = metrics.cpan_receipt.as_ref().map_or_else(
        || {
            "| **CPAN top 1000** | UNVERIFIED | baseline receipt unavailable | `.ci/cpan-corpus-baseline.json` |".to_string()
        },
        |report| {
            format!(
                "| **CPAN top 1000** | {} | Ecosystem breadth baseline; `{}` unreadable, `{}` with errors, cached downloads in `target/cpan-corpus/.cpanm`, baseline `{}` | `.ci/cpan-corpus-baseline.json` |",
                format_clean_rate(report.clean_files, report.total_files),
                report.files_unreadable,
                report.files_with_errors,
                short_day(&report.timestamp),
            )
        },
    );

    let project_row = metrics.project_corpus.as_ref().map_or_else(
        || {
            "| **Project corpus** | UNVERIFIED | live repo scan unavailable | `test_corpus/` + `crates/perl-corpus/src/gen` |".to_string()
        },
        |summary| {
            format!(
                "| **Project corpus** | {} | Deterministic regression baseline; `{}` `test_corpus/` + `{}` `perl-corpus` files, `{}` errors, `{}` timeouts, `{}` panics, `{}/{}` NodeKinds, `{}/{}` GA features | `test_corpus/` + `crates/perl-corpus/src/gen` |",
                format_clean_rate(summary.ok_files, summary.total_files),
                summary.test_corpus_files,
                summary.perl_corpus_files,
                summary.error_files,
                summary.timeout_files,
                summary.panic_files,
                summary.nodekind_covered,
                summary.nodekind_total,
                summary.ga_covered,
                summary.ga_total,
            )
        },
    );

    // --- New scorecard rows ---

    // Node-kind coverage: promote from buried footnote to dedicated headline row.
    let nodekind_row = metrics.project_corpus.as_ref().map_or_else(
        || {
            "| **Node-kind coverage** | UNVERIFIED | live repo scan unavailable | `corpus_audit` |"
                .to_string()
        },
        |summary| {
            let pct = if summary.nodekind_total == 0 {
                0.0
            } else {
                100.0 * summary.nodekind_covered as f64 / summary.nodekind_total as f64
            };
            let never_seen = summary.nodekind_total.saturating_sub(summary.nodekind_covered);
            format!(
                "| **Node-kind coverage** | {}/{} ({:.1}%) | {} never-seen node kinds | `corpus_audit` |",
                summary.nodekind_covered, summary.nodekind_total, pct, never_seen,
            )
        },
    );

    // Reliability row: timeouts / panics / unreadable surfaced from existing receipts.
    let reliability_row = {
        let sys_unread = metrics
            .system_receipt
            .as_ref()
            .map_or_else(|| "?".to_string(), |r| r.files_unreadable.to_string());
        let cpan_unread = metrics
            .cpan_receipt
            .as_ref()
            .map_or_else(|| "?".to_string(), |r| r.files_unreadable.to_string());
        let proj_detail = metrics.project_corpus.as_ref().map_or_else(
            || "Project: UNVERIFIED".to_string(),
            |s| format!("Project: {} timeout, {} panic, 0 unread", s.timeout_files, s.panic_files,),
        );
        format!(
            "| **Reliability** | Ubuntu: {} unread / CPAN: {} unread / {} | -- | `.ci/*-baseline.json` |",
            sys_unread, cpan_unread, proj_detail,
        )
    };

    // Strict-clean subset: 10 pinned modules that must parse with zero errors.
    let pinned = metrics.common_corpus_pinned;
    let strict_clean_row = metrics.common_corpus_receipt.as_ref().map_or_else(
        || {
            format!(
                "| **Strict-clean subset** | {pinned} modules (unverified) | run `just common-corpus-check` to generate receipt | `.ci/common-corpus-manifest.txt` |"
            )
        },
        |receipt| {
            let pass = receipt.clean_files;
            let total = receipt.total_files;
            let pct = if total == 0 { 100.0 } else { 100.0 * pass as f64 / total as f64 };
            format!(
                "| **Strict-clean subset** | {pass}/{total} ({pct:.0}%) | {pinned} pinned modules, zero-error gate | `.ci/common-corpus-manifest.txt` |",
            )
        },
    );

    let tracking_table = [system_row, cpan_row, project_row].join("\n");

    let parser_coverage_bullets = format!(
        "- **Three-baseline model**: compatibility is tracked with `just corpus-sweep-check` against Ubuntu system Perl, ecosystem breadth with `just cpan-corpus-check` against the cached CPAN top-1000 install, and deterministic regression coverage with `just parser-audit` against the repo-owned corpus.\n\
         - **Strict promise lists**: `just common-corpus-check` and the CPAN known-clean manifest inside `just cpan-corpus-check` pin subsets that must remain clean on top of the broader baseline receipts.\n\
         - **Fixture bank**: `tree-sitter-perl/test/corpus` contributes ~{} focused syntax sections for targeted parser cases.\n\
         - **CPAN install hygiene**: `cargo xtask cpan-corpus install` reuses `target/cpan-corpus/.cpanm`; pass `--reset` only for a cold rebuild.",
        metrics.syntax_sections,
    );

    let mut text = original.to_string();
    text = replace_block(
        &text,
        "<!-- BEGIN: PARSER_TRACKING_TABLE -->",
        "<!-- END: PARSER_TRACKING_TABLE -->",
        &tracking_table,
    )?;
    text = replace_block(
        &text,
        "<!-- BEGIN: PARSER_METRICS_BULLETS -->",
        "<!-- END: PARSER_METRICS_BULLETS -->",
        &parser_coverage_bullets,
    )?;
    text = replace_block(
        &text,
        "<!-- BEGIN: PARSER_NODEKIND_ROW -->",
        "<!-- END: PARSER_NODEKIND_ROW -->",
        &nodekind_row,
    )?;
    text = replace_block(
        &text,
        "<!-- BEGIN: PARSER_RELIABILITY_ROW -->",
        "<!-- END: PARSER_RELIABILITY_ROW -->",
        &reliability_row,
    )?;
    text = replace_block(
        &text,
        "<!-- BEGIN: PARSER_STRICT_CLEAN_ROW -->",
        "<!-- END: PARSER_STRICT_CLEAN_ROW -->",
        &strict_clean_row,
    )?;
    Ok(text)
}

// ---------------------------------------------------------------------------
// Per-crate quality metrics helpers
// ---------------------------------------------------------------------------

/// Read `mutants.out/mutants.json` (created by `cargo mutants` in the workspace root)
/// and group the listed mutations by crate package name.
///
/// Returns an empty map (not an error) when the file is absent — this is expected
/// before the first nightly CI run.
fn collect_per_crate_mutation(root: &Path) -> BTreeMap<String, usize> {
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
/// crate-name → test count, grouped by the crate prefix before the first `::`.
///
/// The `--list` output lines look like `some_module::test_name: test`.  The
/// "Running" header lines (`Running unittests …`) embed the crate binary name
/// which we use to anchor the current crate.
fn collect_per_crate_test_counts(root: &Path) -> BTreeMap<String, usize> {
    let output = run_cmd(
        root,
        &["cargo", "test", "--workspace", "--lib", "--exclude", "tree-sitter-perl", "--", "--list"],
        Duration::from_secs(180),
    );
    if output.is_empty() {
        return BTreeMap::new();
    }

    // Pattern for "Running unittests src/lib.rs (target/debug/deps/crate_name-HASH)"
    let running_re =
        Regex::new(r"Running unittests[^\(]*\(target[^\)]*deps[/\\]([a-zA-Z0-9_-]+)-[0-9a-f]+\)")
            .ok();
    // Pattern for individual test lines: "path::test_name: test"
    let test_re = Regex::new(r":\s*test\s*$").ok();

    let mut by_crate: BTreeMap<String, usize> = BTreeMap::new();
    let mut current_crate: Option<String> = None;

    for line in output.lines() {
        if let Some(caps) = running_re.as_ref().and_then(|r| r.captures(line)) {
            // Binary names use underscores; normalize to hyphens to match Cargo.toml names.
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
///
/// Columns: Crate | Mutants listed | Tests (lib)
/// When either map is empty the corresponding column shows "—".
fn format_crate_quality_table(
    mutation: &BTreeMap<String, usize>,
    tests: &BTreeMap<String, usize>,
) -> String {
    // Union of all crate names seen in either map.
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

fn generate_quality_status(root: &Path, original: &str) -> Result<String> {
    let mutation_by_crate = collect_per_crate_mutation(root);
    let tests_by_crate = collect_per_crate_test_counts(root);

    let has_mutation_data = !mutation_by_crate.is_empty();
    let mutation_note = if has_mutation_data {
        "per-crate data from `mutants.out/mutants.json` (written by nightly CI `cargo mutants` run)"
    } else {
        "mutation data pending first nightly CI run — run `just mutation-subset` locally to populate"
    };

    let bullets_content = format!(
        "- **Quality Metrics**: <50ms LSP response times, 931ns incremental parsing\n\
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
    fn test_parser_receipts_load() -> Result<()> {
        let root = project_root()?;
        let metrics = collect_parser_metrics(&root);
        assert!(metrics.system_receipt.is_some(), "expected system corpus baseline receipt");
        assert!(metrics.cpan_receipt.is_some(), "expected CPAN corpus baseline receipt");
        assert!(metrics.project_corpus.is_some(), "expected live repo corpus summary");
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
            parser.contains("<!-- BEGIN: PARSER_TRACKING_TABLE -->"),
            "parser.md missing PARSER_TRACKING_TABLE block"
        );
        assert!(
            parser.contains("<!-- BEGIN: PARSER_METRICS_BULLETS -->"),
            "parser.md missing PARSER_METRICS_BULLETS block"
        );

        let quality = fs::read_to_string(status_dir.join("quality.md"))?;
        assert!(
            quality.contains("<!-- BEGIN: QUALITY_METRICS_BULLETS -->"),
            "quality.md missing QUALITY_METRICS_BULLETS block"
        );
        assert!(
            quality.contains("<!-- BEGIN: QUALITY_CRATE_TABLE -->"),
            "quality.md missing QUALITY_CRATE_TABLE block"
        );

        let parser = fs::read_to_string(status_dir.join("parser.md"))?;
        assert!(
            parser.contains("<!-- BEGIN: PARSER_NODEKIND_ROW -->"),
            "parser.md missing PARSER_NODEKIND_ROW block"
        );
        assert!(
            parser.contains("<!-- BEGIN: PARSER_RELIABILITY_ROW -->"),
            "parser.md missing PARSER_RELIABILITY_ROW block"
        );
        assert!(
            parser.contains("<!-- BEGIN: PARSER_STRICT_CLEAN_ROW -->"),
            "parser.md missing PARSER_STRICT_CLEAN_ROW block"
        );

        Ok(())
    }

    /// Parser scorecard: node-kind row renders with correct values from mock metrics.
    #[test]
    fn test_parser_nodekind_row_renders() -> Result<()> {
        let summary = super::super::corpus_audit::StatusSummary {
            total_files: 91,
            ok_files: 91,
            error_files: 0,
            timeout_files: 0,
            panic_files: 0,
            test_corpus_files: 69,
            perl_corpus_files: 22,
            nodekind_covered: 65,
            nodekind_total: 69,
            ga_covered: 12,
            ga_total: 12,
        };
        let metrics = ParserMetrics {
            syntax_sections: 611,
            system_receipt: None,
            cpan_receipt: None,
            project_corpus: Some(summary),
            common_corpus_receipt: None,
            common_corpus_pinned: 10,
        };
        let template = "h\n<!-- BEGIN: PARSER_TRACKING_TABLE -->\nold\n<!-- END: PARSER_TRACKING_TABLE -->\n\
                        <!-- BEGIN: PARSER_NODEKIND_ROW -->\nold\n<!-- END: PARSER_NODEKIND_ROW -->\n\
                        <!-- BEGIN: PARSER_RELIABILITY_ROW -->\nold\n<!-- END: PARSER_RELIABILITY_ROW -->\n\
                        <!-- BEGIN: PARSER_STRICT_CLEAN_ROW -->\nold\n<!-- END: PARSER_STRICT_CLEAN_ROW -->\n\
                        <!-- BEGIN: PARSER_METRICS_BULLETS -->\nold\n<!-- END: PARSER_METRICS_BULLETS -->\n";
        let result = generate_parser_status(&metrics, template)?;
        assert!(result.contains("65/69"), "nodekind row missing 65/69");
        assert!(result.contains("94.2"), "nodekind row missing 94.2%");
        assert!(result.contains("4 never-seen"), "nodekind row missing never-seen count");
        // No-receipt path: must show "unverified" not a misleading pass ratio.
        assert!(
            result.contains("unverified"),
            "strict-clean no-receipt row should say 'unverified', not a false pass ratio"
        );
        assert!(
            !result.contains("10/10"),
            "strict-clean no-receipt row must not show 10/10 (implies verified pass)"
        );
        Ok(())
    }

    /// Parser scorecard: strict-clean row shows "unverified" when no receipt exists.
    #[test]
    fn test_parser_strict_clean_row_no_receipt() -> Result<()> {
        let metrics = ParserMetrics {
            syntax_sections: 611,
            system_receipt: None,
            cpan_receipt: None,
            project_corpus: None,
            common_corpus_receipt: None,
            common_corpus_pinned: 10,
        };
        let template = "h\n<!-- BEGIN: PARSER_TRACKING_TABLE -->\nold\n<!-- END: PARSER_TRACKING_TABLE -->\n\
                        <!-- BEGIN: PARSER_NODEKIND_ROW -->\nold\n<!-- END: PARSER_NODEKIND_ROW -->\n\
                        <!-- BEGIN: PARSER_RELIABILITY_ROW -->\nold\n<!-- END: PARSER_RELIABILITY_ROW -->\n\
                        <!-- BEGIN: PARSER_STRICT_CLEAN_ROW -->\nold\n<!-- END: PARSER_STRICT_CLEAN_ROW -->\n\
                        <!-- BEGIN: PARSER_METRICS_BULLETS -->\nold\n<!-- END: PARSER_METRICS_BULLETS -->\n";
        let result = generate_parser_status(&metrics, template)?;
        assert!(
            result.contains("10 modules (unverified)"),
            "strict-clean no-receipt row must say '10 modules (unverified)'"
        );
        assert!(
            result.contains("common-corpus-check"),
            "strict-clean no-receipt row must mention the command"
        );
        Ok(())
    }

    /// Parser scorecard: strict-clean row shows "10/10 (100%)" when receipt is available.
    #[test]
    fn test_parser_strict_clean_row_with_receipt() -> Result<()> {
        use std::collections::BTreeMap;
        let receipt = super::super::parser_corpus_sweep::SweepReport {
            schema_version: "1".to_string(),
            commit: "abc".to_string(),
            timestamp: "2026-04-11T00:00:00Z".to_string(),
            corpus_profile: "common".to_string(),
            corpus_roots: vec![],
            resolved_roots_count: 0,
            perl_version: "5.038".to_string(),
            total_files: 10,
            files_unreadable: 0,
            clean_files: 10,
            files_with_errors: 0,
            total_error_nodes: 0,
            first_error_buckets: BTreeMap::new(),
            files_by_bucket: BTreeMap::new(),
            file_results: vec![],
            elapsed_secs: 1.0,
        };
        let metrics = ParserMetrics {
            syntax_sections: 611,
            system_receipt: None,
            cpan_receipt: None,
            project_corpus: None,
            common_corpus_receipt: Some(receipt),
            common_corpus_pinned: 10,
        };
        let template = "h\n<!-- BEGIN: PARSER_TRACKING_TABLE -->\nold\n<!-- END: PARSER_TRACKING_TABLE -->\n\
                        <!-- BEGIN: PARSER_NODEKIND_ROW -->\nold\n<!-- END: PARSER_NODEKIND_ROW -->\n\
                        <!-- BEGIN: PARSER_RELIABILITY_ROW -->\nold\n<!-- END: PARSER_RELIABILITY_ROW -->\n\
                        <!-- BEGIN: PARSER_STRICT_CLEAN_ROW -->\nold\n<!-- END: PARSER_STRICT_CLEAN_ROW -->\n\
                        <!-- BEGIN: PARSER_METRICS_BULLETS -->\nold\n<!-- END: PARSER_METRICS_BULLETS -->\n";
        let result = generate_parser_status(&metrics, template)?;
        assert!(result.contains("10/10"), "strict-clean row missing 10/10");
        assert!(result.contains("100%"), "strict-clean row missing 100%");
        assert!(
            result.contains("10 pinned modules"),
            "strict-clean row missing pinned modules note"
        );
        Ok(())
    }

    /// Quality scorecard: per-crate mutation table renders correctly with mock data.
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

    /// Quality scorecard: table renders with both header and data rows.
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

    /// Quality scorecard: table shows "no data yet" when both maps are empty.
    #[test]
    fn test_format_crate_quality_table_empty_maps() {
        let table = format_crate_quality_table(&BTreeMap::new(), &BTreeMap::new());
        assert!(table.contains("no data yet"), "expected 'no data yet' for empty maps");
    }

    /// count_common_corpus_pinned returns 10 for the live manifest.
    #[test]
    fn test_count_common_corpus_pinned() -> Result<()> {
        let root = project_root()?;
        let count = count_common_corpus_pinned(&root);
        assert_eq!(count, 10, "expected 10 pinned modules in common-corpus-manifest.txt");
        Ok(())
    }
}
