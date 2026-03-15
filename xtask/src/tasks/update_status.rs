//! Update derived metrics in docs/project/CURRENT_STATUS.md and docs/project/ROADMAP.md.
//!
//! Rust port of `scripts/update-current-status.py`.  Computes test counts,
//! feature catalog metrics, corpus statistics, and missing-docs warnings, then
//! patches the markdown files between fenced markers.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use color_eyre::eyre::{Context, Result, eyre};
use regex::Regex;

use crate::utils::project_root;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Run the update-status task.
///
/// * `write` – write changes back to disk.
/// * `check` – verify files are up to date (returns error if stale).
///
/// When neither flag is set, defaults to `check`.
pub fn run(write: bool, check: bool) -> Result<()> {
    let check = if !write && !check { true } else { check };

    let root = project_root()?;

    let mut files_to_update: Vec<(&str, PathBuf, String)> = Vec::new();

    // --- CURRENT_STATUS.md ---
    let status_path = root.join("docs/project/CURRENT_STATUS.md");
    let original_status =
        fs::read_to_string(&status_path).context("reading docs/project/CURRENT_STATUS.md")?;
    let updated_status = update_current_status(&root, &original_status)?;
    if updated_status != original_status {
        files_to_update.push(("docs/project/CURRENT_STATUS.md", status_path, updated_status));
    }

    // --- ROADMAP.md ---
    let roadmap_path = root.join("docs/project/ROADMAP.md");
    let original_roadmap =
        fs::read_to_string(&roadmap_path).context("reading docs/project/ROADMAP.md")?;
    let updated_roadmap = update_roadmap(&root, &original_roadmap)?;
    if updated_roadmap != original_roadmap {
        files_to_update.push(("docs/project/ROADMAP.md", roadmap_path, updated_roadmap));
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
// Compliance table for ROADMAP.md
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

/// Replace a single row matching `pattern` with `replacement`.
fn replace_row(text: &str, pattern: &str, replacement: &str) -> Result<String> {
    let re = Regex::new(pattern).context("building row replacement regex")?;

    let mut count = 0;
    let result = re.replace_all(text, |_caps: &regex::Captures<'_>| {
        count += 1;
        replacement.to_string()
    });

    if count != 1 {
        return Err(eyre!("Expected 1 match for row pattern {pattern:?}, got {count}"));
    }

    Ok(result.into_owned())
}

// ---------------------------------------------------------------------------
// CURRENT_STATUS.md update
// ---------------------------------------------------------------------------

fn update_current_status(root: &Path, original: &str) -> Result<String> {
    let cov = count_lsp_coverage(root)?;
    let corpus_sections = count_corpus_sections(root);
    let gap_files = count_gap_files(root);
    let tests = count_tests(root);
    let missing_docs_current = count_missing_docs_perl_parser(root);
    let missing_docs_baseline = read_missing_docs_baseline(root);

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

    // Build table row content — uses UX coverage (headline metric)
    let lsp_target_pct: usize = 100;
    let lsp_status = if cov.ux_percent >= lsp_target_pct { "PASS" } else { "In progress" };
    let lsp_table_row = format!(
        "| **LSP Coverage** | {}% ({}/{} advertised features, `features.toml`) | {}% | {} |",
        cov.ux_percent, cov.ux_implemented, cov.ux_total, lsp_target_pct, lsp_status
    );

    // Build bullets section content
    let lsp_coverage = format!(
        "- **LSP Coverage**: {}% user-visible feature coverage ({}/{} advertised features from `features.toml`)",
        cov.ux_percent, cov.ux_implemented, cov.ux_total
    );
    let protocol_compliance = format!(
        "- **Protocol Compliance**: {}% overall LSP protocol support ({}/{} including plumbing)",
        cov.protocol_percent, cov.protocol_implemented, cov.protocol_total
    );
    let parser_coverage = format!(
        "- **Parser Coverage**: ~100% Perl 5 syntax via `tree-sitter-perl/test/corpus` (~{corpus_sections} sections) + `test_corpus/` ({gap_files} `.pl` files)"
    );
    let test_status = format!(
        "- **Test Status**: {tier_a_tests_str} lib tests (Tier A), {ignored_tests_str} ignores tracked ({tracked_debt_str} total tracked debt: {bug_count_str} bug, {manual_count_str} manual)"
    );
    let docs_status = format!(
        "- **Docs (perl-parser)**: missing_docs warnings = {missing_docs_str}{baseline_suffix}"
    );
    let quality_metrics = "- **Quality Metrics**: 87% mutation score, <50ms LSP response times, 931ns incremental parsing";
    let production_status =
        "- **Production Status**: LSP server public alpha (`just ci-gate` passing)";

    let lsp_target = if cov.ux_percent >= lsp_target_pct {
        "**Target**: maintain 100% LSP coverage (no regressions)".to_string()
    } else {
        format!("**Target**: 100% LSP coverage (from current {}%)", cov.ux_percent)
    };

    let bullets_content = [
        lsp_coverage.as_str(),
        protocol_compliance.as_str(),
        parser_coverage.as_str(),
        test_status.as_str(),
        docs_status.as_str(),
        quality_metrics,
        production_status,
        "",
        lsp_target.as_str(),
    ]
    .join("\n");

    let mut text = original.to_string();

    // Replace Tier A Tests row
    text = replace_row(
        &text,
        r"(?m)^\| \*\*Tier A Tests\*\* \| .* \| 100% pass \| .* \|$",
        &format!(
            "| **Tier A Tests** | {tier_a_tests_str} lib tests (discovered), {ignored_tests_str} ignores (tracked) | 100% pass | PASS |"
        ),
    )?;

    // Replace Tracked Test Debt row
    text = replace_row(
        &text,
        r"(?m)^\| \*\*Tracked Test Debt\*\* \| .* \| 0 \| .* \|$",
        &format!(
            "| **Tracked Test Debt** | {tracked_debt_str} ({bug_count_str} bug, {manual_count_str} manual) | 0 | Near-zero |"
        ),
    )?;

    // Replace Documentation row
    text = replace_row(
        &text,
        r"(?m)^\| \*\*Documentation\*\* \| .* \| 0 \| .* \|$",
        &format!(
            "| **Documentation** | perl-parser missing_docs = {missing_docs_str}{baseline_suffix} | 0 | Ratchet |"
        ),
    )?;

    // Replace STATUS_METRICS_TABLE block
    text = replace_block(
        &text,
        "<!-- BEGIN: STATUS_METRICS_TABLE -->",
        "<!-- END: STATUS_METRICS_TABLE -->",
        &lsp_table_row,
    )?;

    // Replace STATUS_METRICS_BULLETS block
    text = replace_block(
        &text,
        "<!-- BEGIN: STATUS_METRICS_BULLETS -->",
        "<!-- END: STATUS_METRICS_BULLETS -->",
        &bullets_content,
    )?;

    // Replace doc violations line (if present)
    if let Ok(re) = Regex::new(r"(?m)^\-\s+\*\*484 doc violations\*\*:.*$") {
        let replacement = format!(
            "- **missing_docs (perl-parser)**: {missing_docs_str}{baseline_suffix} (ratcheted by `ci/check_missing_docs.sh`)"
        );
        text = re.replace(&text, replacement.as_str()).into_owned();
    }

    Ok(text)
}

// ---------------------------------------------------------------------------
// ROADMAP.md update
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
    fn test_replace_row() -> Result<()> {
        let input = "line1\n| **Foo** | old | bar | baz |\nline3";
        let result = replace_row(
            input,
            r"(?m)^\| \*\*Foo\*\* \| .* \| bar \| .* \|$",
            "| **Foo** | new | bar | qux |",
        )?;
        assert_eq!(result, "line1\n| **Foo** | new | bar | qux |\nline3");
        Ok(())
    }

    #[test]
    fn test_replace_row_no_match() {
        let input = "no matching row";
        let result = replace_row(input, r"(?m)^\| \*\*Missing\*\* \|.*$", "replacement");
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
}
