//! Ignored test counting and categorization.
//!
//! Walks the `crates/` directory tree, finds `#[ignore]` attributes in Rust
//! source files, categorises each by its reason string, and prints a summary
//! table compared against a persisted baseline.  This is the Rust-native
//! replacement for `scripts/ignored-test-count.sh`.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;

use color_eyre::eyre::{Context, Result, eyre};
use regex::Regex;
use walkdir::WalkDir;

use crate::utils::project_root;

// ── ANSI colours ────────────────────────────────────────────────────────────

const RED: &str = "\x1b[0;31m";
const GREEN: &str = "\x1b[0;32m";
const YELLOW: &str = "\x1b[0;33m";
const NC: &str = "\x1b[0m";

// ── Categories ──────────────────────────────────────────────────────────────

const CATEGORIES: [&str; 9] =
    ["brokenpipe", "feature", "infra", "protocol", "manual", "stress", "bug", "bare", "other"];

// ── Public entry points ─────────────────────────────────────────────────────

/// Compute per-category ignored test counts from the repo at `root`, without
/// printing anything.  Used by `update_status` to avoid a fragile
/// `bash scripts/ignored-test-count.sh` round-trip that was silently producing
/// empty output when invoked by `xtask.exe` on Windows.
pub fn compute_category_counts(root: &Path) -> Result<HashMap<String, usize>> {
    let crates_root = root.join("crates");
    let detail_matches = collect_ignored_matches(&crates_root, root)?;

    let mut counts: HashMap<String, usize> =
        CATEGORIES.iter().map(|c| ((*c).to_string(), 0)).collect();

    for detail in detail_matches {
        let category = categorize_ignore(&detail.reason, &detail.context);
        *counts.entry(category).or_default() += 1;
    }

    Ok(counts)
}

pub fn run(update: bool, check: bool, verbose: bool) -> Result<()> {
    if update && check {
        return Err(eyre!("choose exactly one of --update or --check for ignored-tests"));
    }

    let root = project_root()?;
    let crates_root = root.join("crates");
    let baseline_path = root.join("scripts").join(".ignored-baseline");

    // Collect all #[ignore] occurrences.
    let detail_matches = collect_ignored_matches(&crates_root, &root)?;

    let mut counts: HashMap<String, usize> =
        CATEGORIES.iter().map(|c| ((*c).to_string(), 0)).collect();
    let mut records: Vec<IgnoredDetail> = Vec::new();

    for detail in detail_matches {
        let category = categorize_ignore(&detail.reason, &detail.context);
        *counts.entry(category.clone()).or_default() += 1;
        records.push(IgnoredDetail {
            category,
            location: detail.location,
            test_name: detail.test_name,
            reason: detail.reason,
        });
    }

    let total: usize = CATEGORIES.iter().map(|c| counts.get(*c).copied().unwrap_or(0)).sum();

    let baseline = load_ignored_baseline(&baseline_path).unwrap_or_else(|_| {
        let mut empty = HashMap::new();
        for c in &CATEGORIES {
            empty.insert((*c).to_string(), 0);
        }
        empty.insert("total".to_string(), 0);
        empty
    });

    let baseline_total = baseline.get("total").copied().unwrap_or(0);

    // ── Pretty-print summary table ──────────────────────────────────────

    println!("===============================================");
    println!("        Ignored Tests Summary");
    println!("===============================================");
    println!("{:<12} {:>8} {:>8} {:>8}", "Category", "Count", "Baseline", "Delta");
    println!("-----------------------------------------------");
    for category in CATEGORIES {
        let current = counts.get(category).copied().unwrap_or(0);
        let previous = baseline.get(category).copied().unwrap_or(0);
        println!(
            "{:<12} {:>8} {:>8} {:>8}",
            category,
            current,
            previous,
            format_delta(current, previous),
        );
    }
    println!("-----------------------------------------------");
    println!(
        "{:<12} {:>8} {:>8} {:>8}",
        "TOTAL",
        total,
        baseline_total,
        format_delta(total, baseline_total),
    );
    println!("===============================================");

    let ci_debt = counts.get("brokenpipe").copied().unwrap_or(0)
        + counts.get("bug").copied().unwrap_or(0)
        + counts.get("bare").copied().unwrap_or(0)
        + counts.get("other").copied().unwrap_or(0);
    let backlog =
        counts.get("feature").copied().unwrap_or(0) + counts.get("infra").copied().unwrap_or(0);
    let permanent =
        counts.get("manual").copied().unwrap_or(0) + counts.get("stress").copied().unwrap_or(0);
    println!();
    println!("CI_DEBT    = {ci_debt:>3}  (brokenpipe + bug + bare + other; must be 0)");
    println!("BACKLOG    = {backlog:>3}  (feature + infra; planned work)");
    println!("PERMANENT  = {permanent:>3}  (manual + stress; bench/helpers)");
    println!();

    // ── Verbose per-category detail ─────────────────────────────────────

    if verbose {
        println!("Detailed breakdown by category:");
        println!();
        for category in CATEGORIES {
            let cat_count = counts.get(category).copied().unwrap_or(0);
            if cat_count == 0 {
                continue;
            }
            println!("{YELLOW}=== {category} ({cat_count}) ==={NC}");
            for record in &records {
                if record.category != category {
                    continue;
                }
                println!("  {}", record.location);
                if !record.test_name.is_empty() {
                    println!("    fn: {}", record.test_name);
                }
                if !record.reason.is_empty() {
                    println!("    reason: {}", record.reason);
                }
            }
            println!();
        }
    }

    // ── Mode dispatch ───────────────────────────────────────────────────

    if update {
        write_ignored_baseline(&baseline_path, &counts, total)?;
        println!("{GREEN}Baseline updated successfully.{NC}");
    } else if check {
        if total > baseline_total {
            println!(
                "{RED}ERROR: Ignored test count increased from {baseline_total} to {total}{NC}"
            );
            println!();
            println!("New ignores must be justified. If intentional, run:");
            println!("  cargo run -p xtask -- ignored-tests --update");
            println!();
            return Err(eyre!("ignored test count increased from {} to {}", baseline_total, total));
        }
        println!(
            "{GREEN}OK: Ignored test count ({total}) is not higher than baseline ({baseline_total}){NC}"
        );
    } else {
        // "show" mode
        if total > 0 {
            println!("Run with --verbose for detailed breakdown:");
            println!("  cargo run -p xtask -- ignored-tests --verbose");
            println!();
            println!("To update baseline:");
            println!("  cargo run -p xtask -- ignored-tests --update");
        }
    }

    Ok(())
}

// ── Internal helpers ────────────────────────────────────────────────────────

fn format_delta(current: usize, baseline: usize) -> String {
    let delta = current.abs_diff(baseline);
    if current > baseline {
        format!("{RED}+{delta}{NC}")
    } else if current < baseline {
        format!("{GREEN}-{delta}{NC}")
    } else {
        "0".to_string()
    }
}

// ── Baseline persistence ────────────────────────────────────────────────────

fn load_ignored_baseline(path: &Path) -> Result<HashMap<String, usize>> {
    let raw = fs::read_to_string(path).with_context(|| format!("reading {path:?}"))?;
    let mut values = HashMap::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let Ok(parsed) = value.trim().parse::<usize>() else {
            continue;
        };
        values.insert(key.trim().to_string(), parsed);
    }
    Ok(values)
}

fn write_ignored_baseline(
    path: &Path,
    counts: &HashMap<String, usize>,
    total: usize,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut lines = Vec::new();
    lines.push(
        "# Ignored test baseline - updated by: cargo xtask ignored-tests --update".to_string(),
    );
    let mut ordered = BTreeMap::new();
    for key in CATEGORIES {
        ordered.insert(key, counts.get(key).copied().unwrap_or(0));
    }
    for (key, value) in &ordered {
        lines.push(format!("{key}={value}"));
    }
    lines.push(format!("total={total}"));
    fs::write(path, format!("{}\n", lines.join("\n")))?;
    Ok(())
}

// ── Source scanning ─────────────────────────────────────────────────────────

struct IgnoreMatch {
    location: String,
    context: String,
    reason: String,
    test_name: String,
}

#[derive(Clone)]
struct IgnoredDetail {
    category: String,
    location: String,
    reason: String,
    test_name: String,
}

fn display_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map_or_else(|_| path.display().to_string(), |relative| relative.display().to_string())
}

fn read_lines(path: &Path) -> Result<Vec<String>> {
    fs::read_to_string(path)
        .with_context(|| format!("reading {path:?}"))
        .map(|contents| contents.lines().map(str::to_string).collect())
}

fn collect_ignored_matches(crates_root: &Path, repo_root: &Path) -> Result<Vec<IgnoreMatch>> {
    let mut results = Vec::new();
    let ignore_attr_re =
        Regex::new(r#"^\s*#\[ignore\b(?:(?:\s*=\s*)?\"(?P<d>[^\"]+)\"|\s*=\s*'(?P<s>[^']+)')?"#)
            .with_context(|| "compiling ignore attribute regex")?;
    let fn_re =
        Regex::new(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)").with_context(|| "compiling fn regex")?;
    let comment_re = Regex::new(r"//\s*(.+)$").with_context(|| "compiling comment regex")?;

    for entry in WalkDir::new(crates_root).follow_links(false).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if !entry.file_type().is_file() || path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        let rel = display_path(repo_root, path);
        let lines = read_lines(path)?;
        for i in 0..lines.len() {
            let line = &lines[i];
            if !line.trim_start().starts_with("#[ignore") {
                continue;
            }

            let mut reason = String::new();
            if let Some(caps) = ignore_attr_re.captures(line) {
                if let Some(matched) = caps.name("d") {
                    reason = matched.as_str().to_string();
                } else if let Some(matched) = caps.name("s") {
                    reason = matched.as_str().to_string();
                }
            }

            let context_lines = {
                let end = std::cmp::min(lines.len(), i + 4);
                lines[i..end].join("\n")
            };

            // Try to extract reason from inline comment.
            if reason.is_empty()
                && let Some(comment) = comment_re.captures(line).and_then(|m| m.get(1))
            {
                reason = comment.as_str().to_string();
            }
            // Try next line.
            if reason.is_empty()
                && i + 1 < lines.len()
                && let Some(comment) = comment_re.captures(&lines[i + 1]).and_then(|m| m.get(1))
            {
                reason = comment.as_str().to_string();
            }
            // Try line after that.
            if reason.is_empty()
                && i + 2 < lines.len()
                && let Some(comment) = comment_re.captures(&lines[i + 2]).and_then(|m| m.get(1))
            {
                reason = comment.as_str().to_string();
            }

            let mut test_name = String::new();
            if let Some(found) = fn_re.captures(&context_lines).and_then(|m| m.get(1)) {
                test_name = found.as_str().to_string();
            }

            results.push(IgnoreMatch {
                location: format!("{rel}:{}", i + 1),
                context: context_lines,
                reason,
                test_name,
            });
        }
    }
    Ok(results)
}

// ── Categorisation ──────────────────────────────────────────────────────────

fn categorize_ignore(reason: &str, context: &str) -> String {
    let reason = reason.trim().to_lowercase();
    let context = context.to_lowercase();

    if reason.starts_with("manual:")
        || reason.contains("manual ")
        || reason.contains("regenerate")
        || reason.contains("helper")
    {
        return "manual".to_string();
    }
    if reason.starts_with("stress:")
        || reason.contains("stress test")
        || reason.contains("memory.stress")
        || reason.contains("performance.stress")
        || reason.contains("load.test")
        || reason.contains("stack.overflow")
        || reason.contains("designed.to.fail")
    {
        return "stress".to_string();
    }
    if reason.starts_with("bug:")
        || reason.contains("bug:")
        || reason.contains("known.bug")
        || reason.contains("regression")
        || reason.contains("incorrect.behavior")
        || reason.contains("parser.bug")
        || reason.contains("missing.notification")
        || reason.contains("missing.initialize")
        || reason.contains("server.returns.instead")
        || reason.contains("will.kill")
        || reason.contains("known.inconsistencies")
        || reason.contains("mut_")
        || reason.contains("matching.issue")
        || reason.contains("investigate")
        || reason.contains("instead.of.expected")
        || reason.contains("different.error.format")
        || reason.contains("expects")
    {
        return "bug".to_string();
    }
    if reason.starts_with("todo:")
        || reason.starts_with("infra:")
        || reason.contains("infra ")
        || reason.contains("fixme")
        || reason.contains("needs")
        || reason.contains("requires")
        || reason.contains("setup")
        || reason.contains("config")
        || reason.contains("environment")
        || reason.contains("run.with")
        || reason.contains("only.run.after")
        || reason.contains("only.run.when")
    {
        return "infra".to_string();
    }
    if reason.starts_with("feature:")
        || reason.contains("feature ")
        || reason.contains("not.implemented")
        || reason.contains("unimplemented")
        || reason.contains("wip")
        || reason.contains("work.in.progress")
        || reason.contains("pending")
        || reason.contains("when.implemented")
        || reason.contains("remove.when")
        || reason.contains("ac")
        || reason.contains("not.yet")
        || reason.contains("tdd.scaffold")
        || reason.contains("scaffold")
        || reason.contains("doesn.t.support")
        || reason.contains("doesn't.support")
        || reason.contains("parser.limitation")
        || reason.contains("expected.to.fail")
        || reason.contains("not.fully.supported")
        || reason.contains("enable.after")
        || reason.contains("after.phase")
        || reason.contains("parser.doesn")
        || reason.contains("tracked in #")
    {
        return "feature".to_string();
    }
    if reason.starts_with("brokenpipe:")
        || reason.contains("brokenpipe ")
        || reason.contains("broken.pipe")
        || reason.contains("transport.error")
        || reason.contains("transport.flake")
        || reason.contains("flaky")
    {
        return "brokenpipe".to_string();
    }
    if reason.contains("protocol")
        || reason.contains("lsp")
        || reason.contains("dap")
        || reason.contains("compliance")
        || reason.contains("specification")
    {
        return "protocol".to_string();
    }
    if reason.contains("tracked in #") {
        return "feature".to_string();
    }
    if reason.contains("doesn.t.have.field")
        || reason.contains("may.not.produce")
        || reason.contains("doesn.t.yet")
        || reason.contains("fewer.than.expected")
    {
        return "feature".to_string();
    }
    if reason.contains("recursion.limit.behavior") || reason.contains("behavior.changed") {
        return "feature".to_string();
    }
    if reason.contains("integration.test.that.spawns")
        || reason.contains("spawns.external")
        || reason.contains("burn.down")
        || reason.contains("mutation.hardening")
    {
        return "infra".to_string();
    }
    if reason.contains("clippy.warnings") || reason.contains("warnings.burn") {
        return "infra".to_string();
    }
    if reason.starts_with("ac:") {
        return "feature".to_string();
    }
    if reason.is_empty() || reason == "ignore" {
        return "bare".to_string();
    }
    if context.contains("ac:") {
        return "feature".to_string();
    }
    "other".to_string()
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categorize_manual() {
        assert_eq!(categorize_ignore("manual: run locally", ""), "manual");
    }

    #[test]
    fn categorize_infra() {
        assert_eq!(categorize_ignore("TODO: requires CI setup", ""), "infra");
    }

    #[test]
    fn categorize_feature() {
        assert_eq!(categorize_ignore("feature: not implemented", ""), "feature");
    }

    #[test]
    fn categorize_bug() {
        assert_eq!(categorize_ignore("bug: known regression", ""), "bug");
    }

    #[test]
    fn categorize_stress() {
        assert_eq!(categorize_ignore("stress: memory stress", ""), "stress");
    }

    #[test]
    fn categorize_brokenpipe() {
        assert_eq!(categorize_ignore("brokenpipe: transport error", ""), "brokenpipe");
    }

    #[test]
    fn categorize_protocol() {
        assert_eq!(categorize_ignore("lsp compliance check", ""), "protocol");
    }

    #[test]
    fn categorize_bare() {
        assert_eq!(categorize_ignore("", ""), "bare");
        assert_eq!(categorize_ignore("ignore", ""), "bare");
    }

    #[test]
    fn categorize_other() {
        assert_eq!(categorize_ignore("some unique unmatched reason", ""), "other");
    }

    #[test]
    fn categorize_context_ac() {
        assert_eq!(categorize_ignore("xyz", "ac: something"), "feature");
    }

    #[test]
    fn format_delta_increase() {
        let s = format_delta(5, 3);
        assert!(s.contains("+2"));
    }

    #[test]
    fn format_delta_decrease() {
        let s = format_delta(3, 5);
        assert!(s.contains("-2"));
    }

    #[test]
    fn format_delta_same() {
        assert_eq!(format_delta(3, 3), "0");
    }

    #[test]
    fn load_baseline_round_trip() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join(".ignored-baseline");

        let mut counts = HashMap::new();
        counts.insert("feature".to_string(), 5);
        counts.insert("bug".to_string(), 2);
        write_ignored_baseline(&path, &counts, 7)?;

        let loaded = load_ignored_baseline(&path)?;
        assert_eq!(loaded.get("feature").copied(), Some(5));
        assert_eq!(loaded.get("bug").copied(), Some(2));
        assert_eq!(loaded.get("total").copied(), Some(7));
        Ok(())
    }
}
