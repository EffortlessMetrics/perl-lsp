//! Coverage baseline receipt generation for the proof lane.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use color_eyre::eyre::{Context, Result, bail};
use serde_json::{Value as JsonValue, json};
use serde_yaml_ng::Value as YamlValue;

const COVERAGE_TARGET: f64 = 95.0;

// ---------------------------------------------------------------------------
// cfg(test) line detection
// ---------------------------------------------------------------------------
//
// Inline `#[cfg(test)]` module blocks live inside production `src/*.rs` files,
// so `cargo llvm-cov --ignore-filename-regex` cannot exclude them — it is
// path-only.  Lines inside those blocks may legitimately be uncovered even
// after running the full test suite (dead branches, never-triggered arms,
// etc.), but they are pure test infrastructure, not production code.
//
// This module detects which source-file lines fall inside any
// `#[cfg(test)]`-gated scope and strips them from the parsed `LcovSummary`
// **before** patch-coverage arithmetic runs.  Production lines that happen to
// be *exercised by* the test suite still appear in the LCOV (they are
// measured when the test binary runs them) and are unaffected by the filter.
//
// Algorithm:
//   1. Scan lines looking for `#[cfg(test)]`.
//   2. When found, mark the attribute line and all subsequent lines as
//      belonging to the test span until the matching closing brace of the
//      item that follows the attribute is seen (brace-depth tracking).
//   3. Brace depth is counted using `structural_brace_delta`, which skips
//      braces inside string literals (including raw strings), char literals,
//      byte literals, and `//` line comments.  This prevents false depth
//      changes from patterns like `assert!(s.starts_with('{'))` or
//      `// }` comment braces, which would otherwise either prematurely end
//      the test span (conservative direction) or extend it into production
//      code (dangerous direction, masks real coverage gaps).
//
// The returned set contains 1-based line numbers (matching LCOV `DA:` records).

/// Compute the net structural brace delta for a single line of Rust source,
/// ignoring braces that appear inside:
/// - `//` line comments
/// - double-quoted string literals `"..."` (including escaped quotes `\"`)
/// - single-char literals `'.'` (including `'\''` and `'\\'`)
/// - byte literals `b"..."` and `b'.'`
/// - raw string literals `r#"..."#` and `br#"..."#` (arbitrary hash count)
///
/// Block comments `/* ... */` are not handled (they can span lines); they are
/// rare enough in Rust test modules that omitting them is acceptable.
///
/// Returns the signed sum of structural `{` (+1) and `}` (-1) characters.
fn structural_brace_delta(line: &str) -> i32 {
    let mut delta: i32 = 0;
    let chars: Vec<char> = line.chars().collect();
    let n = chars.len();
    let mut i = 0;

    while i < n {
        let ch = chars[i];

        // `//` line comment — everything from here is a comment.
        if ch == '/' && i + 1 < n && chars[i + 1] == '/' {
            break;
        }

        // Raw byte string `br#"..."#` — must be checked before `b"..."`.
        if ch == 'b'
            && i + 1 < n
            && chars[i + 1] == 'r'
            && {
                let mut k = i + 2;
                while k < n && chars[k] == '#' {
                    k += 1;
                }
                k < n && chars[k] == '"'
            }
        {
            let mut hash_count = 0usize;
            let mut k = i + 2;
            while k < n && chars[k] == '#' {
                hash_count += 1;
                k += 1;
            }
            // skip opening `"`
            k += 1;
            loop {
                if k >= n {
                    i = n;
                    break;
                }
                if chars[k] == '"' {
                    let mut h = 0usize;
                    while k + 1 + h < n && chars[k + 1 + h] == '#' {
                        h += 1;
                    }
                    if h >= hash_count {
                        i = k + 1 + hash_count;
                        break;
                    }
                }
                k += 1;
            }
            continue;
        }

        // Raw string literal `r#"..."#`.
        if ch == 'r'
            && {
                let mut k = i + 1;
                while k < n && chars[k] == '#' {
                    k += 1;
                }
                k < n && chars[k] == '"'
            }
        {
            let mut hash_count = 0usize;
            let mut k = i + 1;
            while k < n && chars[k] == '#' {
                hash_count += 1;
                k += 1;
            }
            // skip opening `"`
            k += 1;
            loop {
                if k >= n {
                    i = n;
                    break;
                }
                if chars[k] == '"' {
                    let mut h = 0usize;
                    while k + 1 + h < n && chars[k + 1 + h] == '#' {
                        h += 1;
                    }
                    if h >= hash_count {
                        i = k + 1 + hash_count;
                        break;
                    }
                }
                k += 1;
            }
            continue;
        }

        // Byte string literal `b"..."`.
        if ch == 'b' && i + 1 < n && chars[i + 1] == '"' {
            i += 2;
            while i < n {
                if chars[i] == '\\' {
                    i += 2;
                } else if chars[i] == '"' {
                    i += 1;
                    break;
                } else {
                    i += 1;
                }
            }
            continue;
        }

        // Byte char literal `b'.'`.
        if ch == 'b' && i + 1 < n && chars[i + 1] == '\'' {
            i += 2;
            while i < n {
                if chars[i] == '\\' {
                    i += 2;
                } else if chars[i] == '\'' {
                    i += 1;
                    break;
                } else {
                    i += 1;
                }
            }
            continue;
        }

        // Double-quoted string literal `"..."`.
        if ch == '"' {
            i += 1;
            while i < n {
                if chars[i] == '\\' {
                    i += 2;
                } else if chars[i] == '"' {
                    i += 1;
                    break;
                } else {
                    i += 1;
                }
            }
            continue;
        }

        // Single-char literal `'.'`.  Distinguish from lifetime annotations
        // (`'a`, `'static`, `'_`) by checking whether a closing `'` follows
        // within the expected positions for a char literal.
        if ch == '\'' {
            // Lifetime or label heuristic: `'` followed by an identifier char
            // with no closing `'` after the identifier means it is a lifetime.
            let is_lifetime = i + 1 < n
                && (chars[i + 1].is_ascii_alphabetic() || chars[i + 1] == '_')
                && {
                    let mut k = i + 1;
                    while k < n && (chars[k].is_ascii_alphanumeric() || chars[k] == '_') {
                        k += 1;
                    }
                    k >= n || chars[k] != '\''
                };
            if is_lifetime {
                i += 1;
                continue;
            }
            // Char literal: consume until closing `'` (handling `\\` escapes).
            i += 1;
            while i < n {
                if chars[i] == '\\' {
                    i += 2;
                } else if chars[i] == '\'' {
                    i += 1;
                    break;
                } else {
                    i += 1;
                }
            }
            continue;
        }

        // Structural brace — counts toward depth.
        if ch == '{' {
            delta += 1;
        } else if ch == '}' {
            delta -= 1;
        }

        i += 1;
    }

    delta
}

/// Return the set of 1-based line numbers that fall inside any
/// `#[cfg(test)]`-gated item in `source_text`.
///
/// The attribute line itself and the closing brace line of the block are
/// included.  Lines outside any `#[cfg(test)]` block are not included.
fn cfg_test_line_numbers(source_text: &str) -> BTreeSet<u64> {
    // We advance the iterator via `.next()` so that the inner scan can consume
    // additional lines from the same iterator.  A plain `for (x, y) in iter`
    // holds an exclusive borrow for the entire loop body, preventing the inner
    // while-let from re-borrowing the same iterator; `.next()` sidesteps that.
    let mut test_lines = BTreeSet::new();
    let mut lines_iter = source_text.lines().enumerate();

    while let Some((idx, line)) = lines_iter.next() {
        let lineno = idx as u64 + 1; // 1-based

        // Detect `#[cfg(test)]` anywhere on the line (handles leading
        // whitespace and multi-attribute stacking).
        if !is_cfg_test_attr(line) {
            continue;
        }

        // Mark the attribute line itself.
        test_lines.insert(lineno);

        // Consume lines until we've opened and closed the item's brace block.
        // depth > 0 once we've seen the first `{`; back to 0 means block done.
        let mut depth: i32 = 0;
        let mut entered = false;

        // `while let` with a labeled break is needed here: a `for` loop would
        // require `iter.by_ref()` which conflicts with the outer while-let
        // borrow on the same iterator.
        #[allow(clippy::while_let_on_iterator)]
        'block: while let Some((idx2, content)) = lines_iter.next() {
            let ln2 = idx2 as u64 + 1;
            test_lines.insert(ln2);

            // structural_brace_delta skips braces in string/char literals and
            // // comments, so patterns like `assert!(s.starts_with('{'))` or
            // `// }` do not corrupt the depth counter.
            let delta = structural_brace_delta(content);
            let prev_depth = depth;
            depth += delta;
            if !entered && depth > 0 {
                entered = true;
            }
            // The block closes when depth returns to 0 (or below, guarding
            // against negative deltas from unrecognized patterns).
            if entered && prev_depth > 0 && depth <= 0 {
                break 'block;
            }
        }
    }

    test_lines
}

/// Return `true` if `line` contains a `#[cfg(test)]` attribute.
fn is_cfg_test_attr(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.contains("#[cfg(test)]")
        || trimmed.contains("#[cfg(test,")
        || trimmed.contains("#[cfg(all(test")
}

/// Strip lines that fall inside `#[cfg(test)]` blocks from a parsed
/// `LcovSummary`.  Source files are resolved relative to `source_root`.
///
/// Files that cannot be read (e.g. absolute paths in a different tree,
/// generated files) are silently skipped — their lines remain measured,
/// which is the conservative direction.
fn strip_cfg_test_lines(summary: &mut LcovSummary, source_root: &Path) {
    for file in &mut summary.files {
        let source_path = resolve_source_path(&file.path, source_root);
        let source_text = source_path.and_then(|p| fs::read_to_string(p).ok());
        let Some(text) = source_text else {
            continue; // cannot resolve — leave file untouched
        };
        let test_line_set = cfg_test_line_numbers(&text);
        if test_line_set.is_empty() {
            continue;
        }

        // Rebuild the line list, dropping test-block lines from hit/found.
        let mut new_lines = Vec::with_capacity(file.lines.len());
        let mut new_line_hit: u64 = 0;
        let mut new_line_found: u64 = 0;
        let mut new_uncovered: Vec<u64> = Vec::new();

        for lcov_line in &file.lines {
            if test_line_set.contains(&lcov_line.number) {
                continue; // drop test-infra line from accounting
            }
            new_line_found += 1;
            if lcov_line.hit_count > 0 {
                new_line_hit += 1;
            } else {
                new_uncovered.push(lcov_line.number);
            }
            new_lines.push(LcovLine { number: lcov_line.number, hit_count: lcov_line.hit_count });
        }

        // Adjust summary totals by the delta we removed.
        summary.line_hit -= file.line_hit.saturating_sub(new_line_hit);
        summary.line_found -= file.line_found.saturating_sub(new_line_found);

        file.lines = new_lines;
        file.line_hit = new_line_hit;
        file.line_found = new_line_found;
        file.uncovered_lines = new_uncovered;
    }
}

/// Resolve an LCOV `SF:` path to an existing filesystem path.
///
/// llvm-cov may emit absolute paths or workspace-relative paths.  We try
/// the path as-is first, then join it to `source_root`.
fn resolve_source_path(lcov_path: &str, source_root: &Path) -> Option<PathBuf> {
    let p = PathBuf::from(lcov_path);
    if p.is_absolute() && p.exists() {
        return Some(p);
    }
    let joined = source_root.join(lcov_path);
    if joined.exists() {
        return Some(joined);
    }
    None
}

#[derive(Debug)]
pub struct CoverageBaselineArgs {
    pub lcov: PathBuf,
    pub receipt: PathBuf,
    pub codecov: PathBuf,
    pub patch_coverage: Option<f64>,
    pub patch_base: Option<String>,
    pub scope: Option<String>,
    pub check: bool,
}

#[derive(Debug, Default)]
struct LcovSummary {
    line_hit: u64,
    line_found: u64,
    files: Vec<FileCoverage>,
}

#[derive(Debug, Default)]
struct FileCoverage {
    path: String,
    line_hit: u64,
    line_found: u64,
    uncovered_lines: Vec<u64>,
    lines: Vec<LcovLine>,
}

#[derive(Debug, Default)]
struct LcovLine {
    number: u64,
    hit_count: u64,
}

pub fn run(args: CoverageBaselineArgs) -> Result<()> {
    let root = std::env::current_dir().context("resolving current directory")?;
    let receipt = build_receipt(&root, &args)?;
    let rendered = render_json(&receipt)?;

    if args.check {
        let existing = fs::read_to_string(&args.receipt)
            .with_context(|| format!("reading coverage receipt {}", args.receipt.display()))?;
        if normalize(&existing) != normalize(&rendered) {
            bail!(
                "coverage baseline receipt is stale: regenerate with `{}`",
                coverage_baseline_command(&args, false)
            );
        }
        println!("coverage baseline receipt is current: {}", args.receipt.display());
        return Ok(());
    }

    write_text(&args.receipt, &rendered)?;
    println!("wrote coverage baseline receipt: {}", args.receipt.display());
    Ok(())
}

fn build_receipt(root: &Path, args: &CoverageBaselineArgs) -> Result<JsonValue> {
    let mut lcov = parse_lcov(&args.lcov)?;
    // Strip lines inside `#[cfg(test)]` blocks before coverage arithmetic.
    // These are test-infra lines, not production code, and may be legitimately
    // uncovered even after running the full test suite.  Production lines
    // exercised by the test binary are unaffected (they appear in the LCOV
    // regardless of which `cfg` block they live in).
    strip_cfg_test_lines(&mut lcov, root);
    let codecov = read_codecov_status(&args.codecov)?;
    let line_coverage = percent(lcov.line_hit, lcov.line_found);
    let changed_lines =
        args.patch_base.as_deref().map(|base| changed_lines_since(root, base)).transpose()?;
    let patch_coverage = match (args.patch_coverage, changed_lines.as_ref()) {
        (Some(patch), _) => Some(round2(patch)),
        (None, Some(changed)) => {
            Some(patch_coverage_from_changed_lines_for_root(Some(root), &lcov, changed))
        }
        (None, None) => None,
    };
    let patch_files_below_target = changed_lines
        .as_ref()
        .map(|changed| patch_file_gaps_for_root(Some(root), &lcov, changed))
        .unwrap_or_default();
    let project_file_rows = project_file_gaps(Some(root), &lcov);
    let project_files_below_target =
        project_file_rows.iter().map(|file| file.gap.clone()).collect::<Vec<_>>();
    let top_project_files = top_project_file_gaps_from_rows(project_file_rows, 10);
    let recommended_project_clusters =
        recommended_project_clusters(&project_files_below_target, 10);

    let mut coverage = serde_json::Map::new();
    coverage.insert("project".to_string(), json!(line_coverage));
    if let Some(patch) = patch_coverage {
        coverage.insert("patch".to_string(), json!(patch));
    }

    Ok(json!({
        "schema_version": 1,
        "kind": "coverage_baseline",
        "head": current_head(root)?,
        "lcov": display_path(&args.lcov),
        "scope": args.scope.as_deref().unwrap_or("unspecified"),
        "coverage": coverage,
        "codecov_status": codecov,
        "coverage_scope": {
            "kind": "lcov",
            "source_files": lcov.files.len(),
        },
        "measured": {
            "line_hit": lcov.line_hit,
            "line_found": lcov.line_found,
            "line_coverage": line_coverage,
        },
        "project_burndown": {
            "target": COVERAGE_TARGET,
            "current": line_coverage,
            "remaining_percentage_points": round2((COVERAGE_TARGET - line_coverage).max(0.0)),
            "status": if line_coverage >= COVERAGE_TARGET { "at_target" } else { "burn_down_required" },
        },
        "patch_files_below_target": patch_files_below_target,
        "files_below_target": project_files_below_target.clone(),
        "project_files_below_target": project_files_below_target,
        "top_project_files": top_project_files,
        "recommended_project_clusters": recommended_project_clusters,
    }))
}

fn parse_lcov(path: &Path) -> Result<LcovSummary> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("reading LCOV {}", path.display()))?;
    let mut summary = LcovSummary::default();
    let mut current = FileCoverage::default();

    for line in raw.lines() {
        if let Some(path) = line.strip_prefix("SF:") {
            finish_file(&mut summary, &mut current);
            current.path = path.trim().replace('\\', "/");
            continue;
        }

        if let Some(entry) = line.strip_prefix("DA:") {
            let Some((line_number, hit_count)) = entry.split_once(',') else {
                continue;
            };
            let Ok(line_number) = line_number.trim().parse::<u64>() else {
                continue;
            };
            if line_number == 0 {
                continue;
            }
            let Ok(hit_count) = hit_count.trim().parse::<u64>() else {
                continue;
            };
            current.line_found += 1;
            current.lines.push(LcovLine { number: line_number, hit_count });
            if hit_count > 0 {
                current.line_hit += 1;
            } else {
                current.uncovered_lines.push(line_number);
            }
            continue;
        }

        if line.trim() == "end_of_record" {
            finish_file(&mut summary, &mut current);
        }
    }
    finish_file(&mut summary, &mut current);

    Ok(summary)
}

fn finish_file(summary: &mut LcovSummary, current: &mut FileCoverage) {
    if current.path.trim().is_empty() && current.line_found == 0 {
        return;
    }
    summary.line_hit += current.line_hit;
    summary.line_found += current.line_found;
    summary.files.push(std::mem::take(current));
}

fn changed_lines_since(root: &Path, base: &str) -> Result<BTreeMap<String, BTreeSet<u64>>> {
    let diff_range = format!("{base}...HEAD");
    let output = Command::new("git")
        .args(["diff", "--unified=0", "--no-ext-diff", &diff_range, "--", ":(glob)**/*.rs"])
        .current_dir(root)
        .output()
        .with_context(|| format!("running git diff for patch coverage against {base}"))?;
    if !output.status.success() {
        bail!("git diff for patch coverage failed with status {}", output.status);
    }
    let diff = String::from_utf8(output.stdout).context("git diff returned non-UTF8 output")?;
    Ok(parse_changed_lines(&diff))
}

fn parse_changed_lines(diff: &str) -> BTreeMap<String, BTreeSet<u64>> {
    let mut changed = BTreeMap::<String, BTreeSet<u64>>::new();
    let mut current_file: Option<String> = None;
    let mut new_line: Option<u64> = None;

    for line in diff.lines() {
        if let Some(path) = line.strip_prefix("+++ b/") {
            current_file = Some(path.trim().replace('\\', "/"));
            new_line = None;
            continue;
        }
        if line.starts_with("+++ /dev/null") {
            current_file = None;
            new_line = None;
            continue;
        }
        if let Some(hunk) = line.strip_prefix("@@ ") {
            new_line = parse_hunk_new_start(hunk);
            continue;
        }

        let Some(path) = current_file.as_ref() else {
            continue;
        };
        let Some(line_number) = new_line else {
            continue;
        };

        if line.starts_with('+') && !line.starts_with("+++") {
            changed.entry(path.clone()).or_default().insert(line_number);
            new_line = Some(line_number + 1);
        } else if line.starts_with('-') && !line.starts_with("---") {
            continue;
        } else if !line.starts_with('\\') {
            new_line = Some(line_number + 1);
        }
    }

    changed
}

fn parse_hunk_new_start(hunk: &str) -> Option<u64> {
    hunk.split_whitespace()
        .find(|part| part.starts_with('+'))
        .and_then(|part| part.trim_start_matches('+').split(',').next())
        .and_then(|start| start.parse::<u64>().ok())
}

fn patch_coverage_from_changed_lines_for_root(
    root: Option<&Path>,
    lcov: &LcovSummary,
    changed_lines: &BTreeMap<String, BTreeSet<u64>>,
) -> f64 {
    let mut executable_found = 0;
    let mut executable_hit = 0;

    for file in &lcov.files {
        let Some(lines) = changed_lines_for_lcov_file(root, &file.path, changed_lines) else {
            continue;
        };
        for line in &file.lines {
            if lines.contains(&line.number) {
                executable_found += 1;
                if line.hit_count > 0 {
                    executable_hit += 1;
                }
            }
        }
    }

    if executable_found == 0 { 100.0 } else { percent(executable_hit, executable_found) }
}

fn patch_file_gaps_for_root(
    root: Option<&Path>,
    lcov: &LcovSummary,
    changed_lines: &BTreeMap<String, BTreeSet<u64>>,
) -> Vec<JsonValue> {
    lcov.files.iter().filter_map(|file| patch_file_gap_json(root, file, changed_lines)).collect()
}

fn patch_file_gap_json(
    root: Option<&Path>,
    file: &FileCoverage,
    changed_lines: &BTreeMap<String, BTreeSet<u64>>,
) -> Option<JsonValue> {
    let changed = changed_lines_for_lcov_file(root, &file.path, changed_lines)?;
    let mut line_found = 0;
    let mut line_hit = 0;
    let mut sample_uncovered_lines = Vec::new();

    for line in &file.lines {
        if !changed.contains(&line.number) {
            continue;
        }
        line_found += 1;
        if line.hit_count > 0 {
            line_hit += 1;
        } else if sample_uncovered_lines.len() < 10 {
            sample_uncovered_lines.push(line.number);
        }
    }

    if line_found == 0 || percent(line_hit, line_found) >= 95.0 || sample_uncovered_lines.is_empty()
    {
        return None;
    }

    let path = root
        .and_then(|root| relative_lcov_path(root, &file.path))
        .unwrap_or_else(|| file.path.clone());
    Some(json!({
        "path": path,
        "line_hit": line_hit,
        "line_found": line_found,
        "line_coverage": percent(line_hit, line_found),
        "sample_uncovered_lines": sample_uncovered_lines,
    }))
}

fn changed_lines_for_lcov_file<'a>(
    root: Option<&Path>,
    path: &str,
    changed_lines: &'a BTreeMap<String, BTreeSet<u64>>,
) -> Option<&'a BTreeSet<u64>> {
    changed_lines.get(path).or_else(|| {
        root.and_then(|root| relative_lcov_path(root, path))
            .and_then(|relative| changed_lines.get(&relative))
    })
}

fn relative_lcov_path(root: &Path, path: &str) -> Option<String> {
    let root_text = root
        .canonicalize()
        .unwrap_or_else(|_| root.to_path_buf())
        .to_string_lossy()
        .replace('\\', "/");
    let path_text = Path::new(path)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(path))
        .to_string_lossy()
        .replace('\\', "/");
    let prefix = format!("{}/", root_text.trim_end_matches('/'));
    path_text.strip_prefix(&prefix).map(str::to_string)
}

fn read_codecov_status(path: &Path) -> Result<JsonValue> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("reading Codecov config {}", path.display()))?;
    let parsed: YamlValue = serde_yaml_ng::from_str(&raw)
        .with_context(|| format!("parsing Codecov config {}", path.display()))?;
    let status = yaml_path(&parsed, &["coverage", "status"]).unwrap_or(&YamlValue::Null);
    yaml_to_json(status)
}

fn yaml_path<'a>(value: &'a YamlValue, path: &[&str]) -> Option<&'a YamlValue> {
    let mut current = value;
    for key in path {
        current = match current {
            YamlValue::Mapping(mapping) => mapping.get(YamlValue::String((*key).to_string()))?,
            _ => return None,
        };
    }
    Some(current)
}

fn yaml_to_json(value: &YamlValue) -> Result<JsonValue> {
    serde_json::to_value(value).context("converting YAML value to JSON")
}

fn file_gap_json(file: &FileCoverage) -> Option<JsonValue> {
    let samples =
        file.uncovered_lines.iter().copied().filter(|line| *line > 0).take(10).collect::<Vec<_>>();
    if samples.is_empty() {
        return None;
    }

    Some(json!({
        "path": file.path,
        "line_hit": file.line_hit,
        "line_found": file.line_found,
        "uncovered_line_count": file.line_found.saturating_sub(file.line_hit),
        "line_coverage": percent(file.line_hit, file.line_found),
        "sample_uncovered_lines": samples,
    }))
}

#[cfg(test)]
fn top_project_file_gaps(root: Option<&Path>, lcov: &LcovSummary, limit: usize) -> Vec<JsonValue> {
    top_project_file_gaps_from_rows(project_file_gaps(root, lcov), limit)
}

fn project_file_gaps(root: Option<&Path>, lcov: &LcovSummary) -> Vec<ProjectFileGap> {
    lcov.files
        .iter()
        .filter(|file| !file.path.trim().is_empty())
        .filter(|file| project_file_below_target(file))
        .filter_map(|file| project_file_gap(root, file))
        .collect()
}

fn project_file_gap(root: Option<&Path>, file: &FileCoverage) -> Option<ProjectFileGap> {
    let mut gap = file_gap_json(file)?;
    let path = root
        .and_then(|root| relative_lcov_path(root, &file.path))
        .unwrap_or_else(|| file.path.clone());
    if let Some(object) = gap.as_object_mut() {
        object.insert("path".to_string(), json!(path.clone()));
    }
    Some(ProjectFileGap {
        path,
        line_coverage: percent(file.line_hit, file.line_found),
        uncovered_line_count: file.line_found.saturating_sub(file.line_hit),
        gap,
    })
}

fn top_project_file_gaps_from_rows(mut rows: Vec<ProjectFileGap>, limit: usize) -> Vec<JsonValue> {
    rows.sort_by(|left, right| {
        right
            .uncovered_line_count
            .cmp(&left.uncovered_line_count)
            .then_with(|| {
                left.line_coverage
                    .partial_cmp(&right.line_coverage)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| left.path.cmp(&right.path))
    });
    rows.truncate(limit);
    rows.into_iter().map(|row| row.gap).collect()
}

fn project_file_below_target(file: &FileCoverage) -> bool {
    file.line_found > 0 && percent(file.line_hit, file.line_found) < COVERAGE_TARGET
}

#[derive(Debug)]
struct ProjectFileGap {
    path: String,
    line_coverage: f64,
    uncovered_line_count: u64,
    gap: JsonValue,
}

fn recommended_project_clusters(top_project_files: &[JsonValue], limit: usize) -> Vec<JsonValue> {
    let mut clusters = BTreeMap::<String, ProjectClusterRecommendation>::new();
    for file in top_project_files {
        let Some(path) = file.get("path").and_then(JsonValue::as_str) else {
            continue;
        };
        let uncovered =
            file.get("uncovered_line_count").and_then(JsonValue::as_u64).unwrap_or_default();
        let (name, reason) = project_cluster_for_path(path);
        clusters
            .entry(name.to_string())
            .or_insert_with(|| ProjectClusterRecommendation::new(name, reason))
            .push_file(path, uncovered);
    }

    let mut rows = clusters.into_values().collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .uncovered_line_count
            .cmp(&left.uncovered_line_count)
            .then_with(|| left.name.cmp(&right.name))
    });
    rows.truncate(limit);
    rows.into_iter().map(ProjectClusterRecommendation::into_json).collect()
}

fn project_cluster_for_path(path: &str) -> (&'static str, &'static str) {
    let normalized = path.replace('\\', "/").to_ascii_lowercase();
    if normalized.starts_with("xtask/")
        || normalized.starts_with(".github/")
        || normalized.starts_with(".ci/")
        || normalized.starts_with("scripts/")
        || normalized.starts_with("policy/")
    {
        (
            "proof-infrastructure",
            "Coverage proof, quality-gate, workflow, and policy surfaces are owned by this lane.",
        )
    } else if normalized.contains("quality")
        || normalized.contains("coverage")
        || normalized.contains("ripr")
        || normalized.contains("receipt")
        || normalized.contains("report")
        || normalized.contains("summary")
    {
        (
            "cli-report-generation",
            "Receipt and report generators should be covered with output-contract tests.",
        )
    } else if normalized.contains("config") || normalized.contains("toml") {
        (
            "config-parsing",
            "Configuration surfaces should be covered with parse and failure-path tests.",
        )
    } else if normalized.contains("serde")
        || normalized.contains("json")
        || normalized.contains("serialize")
        || normalized.contains("deserialize")
        || normalized.contains("schema")
    {
        (
            "serialization-deserialization",
            "Structured data surfaces should be covered with schema and round-trip tests.",
        )
    } else if normalized.contains("cancel")
        || normalized.contains("scheduler")
        || normalized.contains("lifecycle")
        || normalized.contains("runtime")
    {
        (
            "scheduler-cancellation",
            "Scheduler, lifecycle, and cancellation paths should be covered with stale-state tests.",
        )
    } else if normalized.contains("error")
        || normalized.contains("diagnostic")
        || normalized.contains("failure")
    {
        ("error-handling", "Error paths should be covered with behavior assertions.")
    } else if normalized.contains("provider")
        || normalized.contains("completion")
        || normalized.contains("hover")
        || normalized.contains("definition")
        || normalized.contains("lsp")
    {
        (
            "provider-decision-logic",
            "Provider decisions should be covered with table-driven behavior tests.",
        )
    } else {
        (
            "project-coverage-inventory",
            "Use the top project files to split a focused coverage burn-down PR.",
        )
    }
}

#[derive(Debug)]
struct ProjectClusterRecommendation {
    name: String,
    reason: String,
    file_count: u64,
    uncovered_line_count: u64,
    example_files: BTreeSet<String>,
}

impl ProjectClusterRecommendation {
    fn new(name: &str, reason: &str) -> Self {
        Self {
            name: name.to_string(),
            reason: reason.to_string(),
            file_count: 0,
            uncovered_line_count: 0,
            example_files: BTreeSet::new(),
        }
    }

    fn push_file(&mut self, path: &str, uncovered_line_count: u64) {
        self.file_count += 1;
        self.uncovered_line_count += uncovered_line_count;
        if self.example_files.len() < 3 {
            self.example_files.insert(path.to_string());
        }
    }

    fn into_json(self) -> JsonValue {
        json!({
            "name": self.name,
            "file_count": self.file_count,
            "uncovered_line_count": self.uncovered_line_count,
            "reason": self.reason,
            "example_files": self.example_files.into_iter().collect::<Vec<_>>(),
        })
    }
}

fn current_head(root: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()
        .context("running git rev-parse HEAD")?;
    if !output.status.success() {
        bail!("git rev-parse HEAD failed with status {}", output.status);
    }
    Ok(String::from_utf8(output.stdout)
        .context("git rev-parse HEAD returned non-UTF8 output")?
        .trim()
        .to_string())
}

fn percent(hit: u64, found: u64) -> f64 {
    if found == 0 { 0.0 } else { round2(hit as f64 * 100.0 / found as f64) }
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn render_json(value: &JsonValue) -> Result<String> {
    Ok(format!("{}\n", serde_json::to_string_pretty(value)?))
}

fn normalize(value: &str) -> String {
    value.trim().replace("\r\n", "\n")
}

fn write_text(path: &Path, text: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    fs::write(path, text).with_context(|| format!("writing {}", path.display()))
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn coverage_baseline_command(args: &CoverageBaselineArgs, check: bool) -> String {
    let mut command = format!(
        "rtk cargo xtask coverage-baseline --lcov {} --receipt {} --codecov {}",
        args.lcov.display(),
        args.receipt.display(),
        args.codecov.display()
    );
    if let Some(patch) = args.patch_coverage {
        command.push_str(&format!(" --patch-coverage {patch:.2}"));
    }
    if let Some(base) = &args.patch_base {
        command.push_str(&format!(" --patch-base {base}"));
    }
    if let Some(scope) = &args.scope {
        command.push_str(&format!(" --scope {scope}"));
    }
    if check {
        command.push_str(" --check");
    }
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

    #[test]
    fn parse_changed_lines_tracks_added_new_file_lines() -> TestResult {
        let diff = "\
diff --git a/crates/example/src/lib.rs b/crates/example/src/lib.rs
index 1111111..2222222 100644
--- a/crates/example/src/lib.rs
+++ b/crates/example/src/lib.rs
@@ -10,0 +11,2 @@
+let covered = true;
+let uncovered = false;
@@ -20 +23,0 @@
-let removed = true;
@@ -30 +32,2 @@
-let old = true;
+let replacement = true;
+let second = true;
\\ No newline at end of file
diff --git a/crates/deleted/src/lib.rs b/crates/deleted/src/lib.rs
deleted file mode 100644
--- a/crates/deleted/src/lib.rs
+++ /dev/null
@@ -1 +0,0 @@
-let removed_file = true;
diff --git a/crates/nohunk/src/lib.rs b/crates/nohunk/src/lib.rs
index 3333333..4444444 100644
--- a/crates/nohunk/src/lib.rs
+++ b/crates/nohunk/src/lib.rs
+let ignored_without_hunk = true;
";

        let changed = parse_changed_lines(diff);
        let lines = changed.get("crates/example/src/lib.rs").ok_or("missing changed file entry")?;

        assert_eq!(lines.iter().copied().collect::<Vec<_>>(), vec![11, 12, 32, 33]);
        assert!(!changed.contains_key("crates/nohunk/src/lib.rs"));
        Ok(())
    }

    #[test]
    fn patch_coverage_counts_only_changed_executable_lcov_lines() -> TestResult {
        let lcov = LcovSummary {
            line_hit: 3,
            line_found: 4,
            files: vec![
                FileCoverage {
                    path: "crates/example/src/lib.rs".to_string(),
                    line_hit: 1,
                    line_found: 3,
                    uncovered_lines: vec![12, 40],
                    lines: vec![
                        LcovLine { number: 11, hit_count: 1 },
                        LcovLine { number: 12, hit_count: 0 },
                        LcovLine { number: 40, hit_count: 0 },
                    ],
                },
                FileCoverage {
                    path: "crates/other/src/lib.rs".to_string(),
                    line_hit: 0,
                    line_found: 1,
                    uncovered_lines: vec![5],
                    lines: vec![LcovLine { number: 5, hit_count: 0 }],
                },
            ],
        };
        let changed = BTreeMap::from([(
            "crates/example/src/lib.rs".to_string(),
            BTreeSet::from([11, 12, 99]),
        )]);

        assert_eq!(patch_coverage_from_changed_lines_for_root(None, &lcov, &changed), 50.0);
        let gaps = patch_file_gaps_for_root(None, &lcov, &changed);
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0]["path"], json!("crates/example/src/lib.rs"));
        assert_eq!(gaps[0]["line_coverage"], json!(50.0));
        assert_eq!(gaps[0]["sample_uncovered_lines"], json!([12]));
        Ok(())
    }

    #[test]
    fn changed_lines_since_reports_bad_base() -> TestResult {
        let temp = tempfile::tempdir()?;
        let repo = temp.path();
        fs::write(repo.join("tracked.rs"), "fn tracked() {}\n")?;
        run_git(repo, &["init"])?;
        run_git(repo, &["add", "tracked.rs"])?;
        run_git(
            repo,
            &["-c", "user.name=test", "-c", "user.email=test@example.com", "commit", "-m", "base"],
        )?;

        assert!(changed_lines_since(repo, "refs/heads/does-not-exist").is_err());
        Ok(())
    }

    #[test]
    fn patch_coverage_is_full_when_diff_has_no_executable_lcov_lines() -> TestResult {
        let lcov = LcovSummary::default();
        let changed = BTreeMap::from([("docs/ci/ripr.md".to_string(), BTreeSet::from([10, 11]))]);

        assert_eq!(patch_coverage_from_changed_lines_for_root(None, &lcov, &changed), 100.0);
        Ok(())
    }

    #[test]
    fn parse_lcov_ignores_malformed_rows_and_tracks_uncovered_samples() -> TestResult {
        let temp = tempfile::tempdir()?;
        let lcov = temp.path().join("lcov.info");
        fs::write(
            &lcov,
            "\
SF:crates/example/src/lib.rs
DA:not-a-line,1
DA:0,1
DA:2,not-a-hit-count
DA:3,0
DA:4,7
end_of_record
",
        )?;

        let summary = parse_lcov(&lcov)?;
        let file = summary.files.first().ok_or("missing LCOV file summary")?;
        let gap = file_gap_json(file).ok_or("missing uncovered file gap")?;

        assert_eq!(summary.line_found, 2);
        assert_eq!(summary.line_hit, 1);
        assert_eq!(file.uncovered_lines, vec![3]);
        assert_eq!(gap["uncovered_line_count"], json!(1));
        assert_eq!(gap["sample_uncovered_lines"], json!([3]));
        Ok(())
    }

    #[test]
    fn top_project_files_rank_uncovered_project_surfaces() -> TestResult {
        let temp = tempfile::tempdir()?;
        let lcov = temp.path().join("lcov.info");
        fs::write(
            &lcov,
            "\
SF:crates/perl-parser/src/lib.rs
DA:1,0
DA:2,0
DA:3,1
end_of_record
SF:xtask/src/tasks/quality_baseline.rs
DA:1,0
DA:2,1
end_of_record
SF:crates/perl-config/src/lib.rs
DA:1,0
DA:2,0
DA:3,0
DA:4,1
end_of_record
SF:crates/perl-covered/src/lib.rs
DA:1,1
DA:2,1
end_of_record
SF:crates/perl-empty/src/lib.rs
end_of_record
",
        )?;
        let summary = parse_lcov(&lcov)?;

        let rows = top_project_file_gaps(None, &summary, 2);

        assert_eq!(rows[0]["path"], json!("crates/perl-config/src/lib.rs"));
        assert_eq!(rows[0]["uncovered_line_count"], json!(3));
        assert_eq!(rows[1]["path"], json!("crates/perl-parser/src/lib.rs"));
        assert_eq!(rows[1]["uncovered_line_count"], json!(2));
        assert!(!rows.iter().any(|row| row["path"] == json!("crates/perl-empty/src/lib.rs")));
        Ok(())
    }

    #[test]
    fn top_project_files_skip_zero_line_coverage_files() -> TestResult {
        let temp = tempfile::tempdir()?;
        let lcov = temp.path().join("lcov.info");
        fs::write(
            &lcov,
            "\
SF:crates/perl-empty/src/lib.rs
end_of_record
",
        )?;
        let summary = parse_lcov(&lcov)?;

        assert_eq!(summary.files[0].line_found, 0);
        assert_eq!(percent(summary.files[0].line_hit, summary.files[0].line_found), 0.0);
        assert!(!project_file_below_target(&summary.files[0]));
        assert_eq!(top_project_file_gaps(None, &summary, 10), Vec::<JsonValue>::new());
        Ok(())
    }

    #[test]
    fn project_file_below_target_requires_executable_lines() {
        let zero_line_file = FileCoverage {
            path: "crates/perl-empty/src/lib.rs".to_string(),
            line_hit: 0,
            line_found: 0,
            lines: Vec::new(),
            uncovered_lines: Vec::new(),
        };
        let low_file = FileCoverage {
            path: "crates/perl-low/src/lib.rs".to_string(),
            line_hit: 1,
            line_found: 2,
            lines: Vec::new(),
            uncovered_lines: vec![2],
        };
        let covered_file = FileCoverage {
            path: "crates/perl-covered/src/lib.rs".to_string(),
            line_hit: 2,
            line_found: 2,
            lines: Vec::new(),
            uncovered_lines: Vec::new(),
        };

        assert!(!project_file_below_target(&zero_line_file));
        assert!(project_file_below_target(&low_file));
        assert!(!project_file_below_target(&covered_file));
    }

    #[test]
    fn recommended_project_clusters_group_current_burn_down_buckets() {
        let top_files = vec![
            json!({"path": "xtask/src/tasks/quality_baseline.rs", "uncovered_line_count": 20}),
            json!({"path": "crates/perl-lsp-provider/src/hover.rs", "uncovered_line_count": 18}),
            json!({"path": "crates/perl-runtime/src/cancellation.rs", "uncovered_line_count": 16}),
            json!({"path": "crates/perl-config/src/lib.rs", "uncovered_line_count": 14}),
            json!({"path": "crates/perl-json/src/schema.rs", "uncovered_line_count": 12}),
            json!({"path": "crates/perl-errors/src/lib.rs", "uncovered_line_count": 10}),
            json!({"path": "crates/perl-parser/src/lib.rs", "uncovered_line_count": 8}),
            json!({"uncovered_line_count": 999}),
        ];

        let rows = recommended_project_clusters(&top_files, 10);

        assert_eq!(rows[0].pointer("/name"), Some(&json!("proof-infrastructure")));
        assert_eq!(rows[0].pointer("/uncovered_line_count"), Some(&json!(20)));
        assert!(
            rows.iter().any(|row| row.pointer("/name") == Some(&json!("provider-decision-logic")))
        );
        assert!(
            rows.iter().any(|row| row.pointer("/name") == Some(&json!("scheduler-cancellation")))
        );
        assert!(rows.iter().any(|row| row.pointer("/name") == Some(&json!("config-parsing"))));
        assert!(
            rows.iter()
                .any(|row| row.pointer("/name") == Some(&json!("serialization-deserialization")))
        );
        assert!(rows.iter().any(|row| row.pointer("/name") == Some(&json!("error-handling"))));
        assert!(
            rows.iter()
                .any(|row| row.pointer("/name") == Some(&json!("project-coverage-inventory")))
        );
    }

    #[test]
    fn project_cluster_mapping_names_burn_down_surfaces() {
        assert_eq!(
            project_cluster_for_path("xtask/src/tasks/quality_baseline.rs").0,
            "proof-infrastructure"
        );
        assert_eq!(
            project_cluster_for_path("crates/perl-receipt/src/report.rs").0,
            "cli-report-generation"
        );
        assert_eq!(project_cluster_for_path("crates/perl-config/src/lib.rs").0, "config-parsing");
        assert_eq!(
            project_cluster_for_path("crates/perl-json/src/schema.rs").0,
            "serialization-deserialization"
        );
        assert_eq!(
            project_cluster_for_path("crates/perl-runtime/src/cancellation.rs").0,
            "scheduler-cancellation"
        );
        assert_eq!(project_cluster_for_path("crates/perl-errors/src/lib.rs").0, "error-handling");
        assert_eq!(
            project_cluster_for_path("crates/perl-lsp-provider/src/hover.rs").0,
            "provider-decision-logic"
        );
        assert_eq!(
            project_cluster_for_path("crates/perl-parser/src/lib.rs").0,
            "project-coverage-inventory"
        );
    }

    #[test]
    fn read_codecov_status_returns_status_tree() -> TestResult {
        let temp = tempfile::tempdir()?;
        let codecov = temp.path().join("codecov.yml");
        fs::write(
            &codecov,
            "\
coverage:
  status:
    patch:
      default:
        target: 95%
        threshold: 0%
    project:
      default:
        target: 95%
        informational: true
",
        )?;

        let status = read_codecov_status(&codecov)?;

        assert_eq!(status["patch"]["default"]["target"], json!("95%"));
        assert_eq!(status["project"]["default"]["informational"], json!(true));
        Ok(())
    }

    #[test]
    fn absolute_lcov_paths_match_repo_relative_changed_lines() -> TestResult {
        let temp = tempfile::tempdir()?;
        let repo = temp.path();
        fs::create_dir_all(repo.join("crates/example/src"))?;
        let source = repo.join("crates/example/src/lib.rs");
        fs::write(&source, "pub fn covered() {}\n")?;
        let changed =
            BTreeMap::from([("crates/example/src/lib.rs".to_string(), BTreeSet::from([1]))]);

        let lines =
            changed_lines_for_lcov_file(Some(repo), &source.display().to_string(), &changed)
                .ok_or("absolute LCOV path did not match changed lines")?;

        assert!(lines.contains(&1));
        assert_eq!(
            relative_lcov_path(repo, &source.display().to_string()).as_deref(),
            Some("crates/example/src/lib.rs")
        );
        Ok(())
    }

    #[test]
    fn build_receipt_derives_patch_coverage_from_git_diff() -> TestResult {
        let temp = tempfile::tempdir()?;
        let repo_root = temp.path().join("perl-lsp-swarm-coverage-target");
        let repo = repo_root.as_path();
        fs::create_dir_all(repo.join("src"))?;
        fs::write(repo.join("src/lib.rs"), "pub fn value() -> bool {\n    true\n}\n")?;
        run_git(repo, &["init"])?;
        run_git(repo, &["add", "src/lib.rs"])?;
        run_git(
            repo,
            &["-c", "user.name=test", "-c", "user.email=test@example.com", "commit", "-m", "base"],
        )?;
        let base = run_git(repo, &["rev-parse", "HEAD"])?.trim().to_string();

        fs::write(repo.join("src/lib.rs"), "pub fn value() -> bool {\n    false\n}\n")?;
        run_git(repo, &["add", "src/lib.rs"])?;
        run_git(
            repo,
            &["-c", "user.name=test", "-c", "user.email=test@example.com", "commit", "-m", "head"],
        )?;
        let lcov = repo.join("lcov.info");
        fs::write(
            &lcov,
            format!(
                "SF:{}\nDA:1,1\nDA:2,0\nDA:3,1\nend_of_record\n",
                repo.join("src/lib.rs").display()
            ),
        )?;
        let codecov = repo.join("codecov.yml");
        fs::write(
            &codecov,
            "coverage:\n  status:\n    patch:\n      default:\n        target: 95%\n",
        )?;
        let args = CoverageBaselineArgs {
            lcov,
            receipt: repo.join("target/coverage-baseline.json"),
            codecov,
            patch_coverage: None,
            patch_base: Some(base),
            scope: Some("workspace-lib-xtask-quality".to_string()),
            check: false,
        };

        let receipt = build_receipt(repo, &args)?;
        assert_eq!(receipt["coverage"]["patch"], json!(0.0));
        assert_eq!(receipt["patch_files_below_target"][0]["path"], json!("src/lib.rs"));
        assert_eq!(receipt["patch_files_below_target"][0]["sample_uncovered_lines"], json!([2]));
        assert_eq!(receipt["project_burndown"]["target"], json!(95.0));
        assert_eq!(receipt["project_burndown"]["status"], json!("burn_down_required"));
        assert_eq!(receipt["project_files_below_target"][0]["path"], json!("src/lib.rs"));
        assert_eq!(receipt["top_project_files"][0]["uncovered_line_count"], json!(1));
        assert_eq!(
            receipt["recommended_project_clusters"][0]["name"],
            json!("project-coverage-inventory")
        );
        assert_eq!(receipt["scope"], json!("workspace-lib-xtask-quality"));
        Ok(())
    }

    #[test]
    fn build_receipt_clusters_all_project_files_below_target() -> TestResult {
        let temp = tempfile::tempdir()?;
        let repo = temp.path();
        run_git(repo, &["init"])?;
        run_git(
            repo,
            &[
                "-c",
                "user.name=test",
                "-c",
                "user.email=test@example.com",
                "commit",
                "--allow-empty",
                "-m",
                "head",
            ],
        )?;
        let lcov = repo.join("lcov.info");
        let mut lcov_body = String::new();
        for file_index in 0..12 {
            lcov_body.push_str(&format!(
                "SF:crates/product-{file_index}/src/lib.rs\nDA:1,0\nDA:2,0\nDA:3,1\nend_of_record\n"
            ));
        }
        lcov_body.push_str("SF:xtask/src/tasks/quality_gate.rs\nDA:1,0\nDA:2,1\nend_of_record\n");
        fs::write(&lcov, lcov_body)?;
        let codecov = repo.join("codecov.yml");
        fs::write(
            &codecov,
            "coverage:\n  status:\n    patch:\n      default:\n        target: 95%\n",
        )?;
        let args = CoverageBaselineArgs {
            lcov,
            receipt: repo.join("target/coverage-baseline.json"),
            codecov,
            patch_coverage: None,
            patch_base: None,
            scope: Some("routed-coverage-packs".to_string()),
            check: false,
        };

        let receipt = build_receipt(repo, &args)?;
        let top_files = receipt
            .get("top_project_files")
            .and_then(JsonValue::as_array)
            .ok_or("top_project_files must be an array")?;
        assert!(
            !top_files.iter().any(|row| {
                row.get("path").and_then(JsonValue::as_str)
                    == Some("xtask/src/tasks/quality_gate.rs")
            }),
            "xtask proof-infra file should sit below the truncated top_project_files list"
        );
        let clusters = receipt
            .get("recommended_project_clusters")
            .and_then(JsonValue::as_array)
            .ok_or("recommended_project_clusters must be an array")?;
        assert!(
            clusters.iter().any(|row| {
                row.get("name").and_then(JsonValue::as_str) == Some("proof-infrastructure")
                    && row.get("example_files").and_then(JsonValue::as_array).is_some_and(|files| {
                        files
                            .iter()
                            .any(|file| file.as_str() == Some("xtask/src/tasks/quality_gate.rs"))
                    })
            }),
            "cluster recommendations must preserve proof-infra work below the display top list: {clusters:?}"
        );
        Ok(())
    }

    #[test]
    fn build_receipt_allows_missing_patch_source() -> TestResult {
        let temp = tempfile::tempdir()?;
        let repo = temp.path();
        run_git(repo, &["init"])?;
        fs::write(repo.join("tracked.rs"), "fn tracked() {}\n")?;
        run_git(repo, &["add", "tracked.rs"])?;
        run_git(
            repo,
            &["-c", "user.name=test", "-c", "user.email=test@example.com", "commit", "-m", "base"],
        )?;
        let lcov = repo.join("lcov.info");
        fs::write(
            &lcov,
            format!("SF:{}\nDA:1,1\nend_of_record\n", repo.join("tracked.rs").display()),
        )?;
        let codecov = repo.join("codecov.yml");
        fs::write(&codecov, "coverage:\n  status: {}\n")?;
        let args = CoverageBaselineArgs {
            lcov,
            receipt: repo.join("target/coverage-baseline.json"),
            codecov,
            patch_coverage: None,
            patch_base: None,
            scope: None,
            check: false,
        };

        let receipt = build_receipt(repo, &args)?;

        assert_eq!(receipt.pointer("/coverage/project").and_then(JsonValue::as_f64), Some(100.0));
        assert!(receipt.pointer("/coverage/patch").is_none());
        assert_eq!(receipt["scope"], json!("unspecified"));
        Ok(())
    }

    #[test]
    fn coverage_baseline_command_preserves_regeneration_inputs() -> TestResult {
        let args = CoverageBaselineArgs {
            lcov: PathBuf::from("target/lcov.info"),
            receipt: PathBuf::from("target/receipts/quality/coverage-baseline.json"),
            codecov: PathBuf::from("codecov.yml"),
            patch_coverage: Some(96.123),
            patch_base: Some("origin/main".to_string()),
            scope: Some("workspace-lib-xtask-quality".to_string()),
            check: false,
        };

        let command = coverage_baseline_command(&args, true);

        assert!(command.contains("--patch-coverage 96.12"));
        assert!(command.contains("--patch-base origin/main"));
        assert!(command.contains("--scope workspace-lib-xtask-quality"));
        assert!(command.ends_with(" --check"));
        Ok(())
    }

    #[test]
    fn check_mode_reports_full_regeneration_command_for_stale_receipt() -> TestResult {
        let temp = tempfile::tempdir()?;
        let repo = temp.path();
        let lcov = repo.join("lcov.info");
        let receipt = repo.join("target/receipts/quality/coverage-baseline.json");
        let codecov = repo.join("codecov.yml");
        fs::create_dir_all(receipt.parent().ok_or("receipt missing parent")?)?;
        fs::write(&lcov, "SF:xtask/src/tasks/quality_baseline.rs\nDA:1,1\nend_of_record\n")?;
        fs::write(
            &codecov,
            "coverage:\n  status:\n    patch:\n      default:\n        target: 95%\n",
        )?;
        fs::write(&receipt, "{}\n")?;
        let args = CoverageBaselineArgs {
            lcov,
            receipt,
            codecov,
            patch_coverage: Some(94.0),
            patch_base: None,
            scope: Some("workspace-lib-xtask-quality".to_string()),
            check: true,
        };

        let err = run(args).expect_err("stale receipt should fail check mode");
        let message = err.to_string();

        assert!(message.contains("coverage baseline receipt is stale"));
        assert!(message.contains("rtk cargo xtask coverage-baseline"));
        assert!(message.contains("--patch-coverage 94.00"));
        assert!(message.contains("--scope workspace-lib-xtask-quality"));
        Ok(())
    }

    fn run_git(repo: &Path, args: &[&str]) -> TestResult<String> {
        let output = Command::new("git").args(args).current_dir(repo).output()?;
        if !output.status.success() {
            return Err(format!("git {:?} failed with status {}", args, output.status).into());
        }
        Ok(String::from_utf8(output.stdout)?)
    }

    #[test]
    fn run_git_reports_failure_status() -> TestResult {
        let temp = tempfile::tempdir()?;

        assert!(run_git(temp.path(), &["definitely-not-a-git-command"]).is_err());
        Ok(())
    }

    // ------------------------------------------------------------------
    // structural_brace_delta — unit tests
    // ------------------------------------------------------------------

    #[test]
    fn structural_brace_delta_counts_plain_braces() {
        assert_eq!(structural_brace_delta("mod tests {"), 1);
        assert_eq!(structural_brace_delta("}"), -1);
        assert_eq!(structural_brace_delta("fn f() {}"), 0);
        assert_eq!(structural_brace_delta(""), 0);
    }

    #[test]
    fn structural_brace_delta_skips_char_literal_open_brace() {
        // The classic dangerous pattern: assert!(s.starts_with('{'));
        // The '{' is a char literal; net structural delta must be 0, not +1.
        assert_eq!(
            structural_brace_delta("        assert!(s.starts_with('{'));"),
            0
        );
    }

    #[test]
    fn structural_brace_delta_skips_char_literal_close_brace() {
        // A '}' char literal must not decrement depth.
        assert_eq!(
            structural_brace_delta("        assert!(s.ends_with('}'));"),
            0
        );
    }

    #[test]
    fn structural_brace_delta_skips_string_literal_braces() {
        // Braces inside double-quoted strings must be ignored.
        assert_eq!(structural_brace_delta(r#"        let s = "{";"#), 0);
        assert_eq!(structural_brace_delta(r#"        let s = "}";"#), 0);
        assert_eq!(structural_brace_delta(r#"        let s = "{ foo }";"#), 0);
    }

    #[test]
    fn structural_brace_delta_skips_comment_braces() {
        // Braces after `//` must not count.
        assert_eq!(structural_brace_delta("        // closing: }"), 0);
        assert_eq!(structural_brace_delta("        // opening: {"), 0);
        // A brace before `//` counts; braces after do not.
        assert_eq!(structural_brace_delta("    fn f() { // }"), 1);
    }

    #[test]
    fn structural_brace_delta_skips_raw_string_braces() {
        // r#"{"# — the { is inside the raw string, net delta 0.
        assert_eq!(structural_brace_delta(r##"        let s = r#"{"#;"##), 0);
        assert_eq!(structural_brace_delta(r###"        let s = r##"{"##;"###), 0);
    }

    #[test]
    fn structural_brace_delta_skips_byte_literal_braces() {
        // b'{' and b'}' must be treated as byte literals, not structural braces.
        assert_eq!(structural_brace_delta("        let c = b'{';"), 0);
        assert_eq!(structural_brace_delta("        let c = b'}';"), 0);
    }

    #[test]
    fn structural_brace_delta_lifetime_not_mistaken_for_char() {
        // Lifetime annotations ('a, 'static) must not suppress the next
        // structural brace.
        assert_eq!(structural_brace_delta("    fn t<'a>() {"), 1);
        assert_eq!(
            structural_brace_delta("        let _: &'static str = \"hi\";"),
            0
        );
    }

    // ------------------------------------------------------------------
    // cfg_test_line_numbers — literal/comment brace regression tests
    // ------------------------------------------------------------------

    #[test]
    fn cfg_test_char_literal_brace_does_not_include_production() {
        // Regression: a naive char-by-char brace counter treats the `{` in
        // `s.starts_with('{')` as a depth increment, causing the module close
        // to be missed and production code after the test module to be
        // incorrectly sucked into the test set (masking real coverage gaps).
        //
        // With `structural_brace_delta` the char literal is skipped, so the
        // module correctly closes at the real `}` on line 10.
        let source = [
            "pub fn prod() -> bool { true }\n", // 1
            "\n",                               // 2
            "#[cfg(test)]\n",                   // 3
            "mod tests {\n",                    // 4
            "    #[test]\n",                    // 5
            "    fn t() {\n",                   // 6
            "        let s = \"{\";\n",       // 7
            "        assert!(s.starts_with('{'));\n", // 8 char literal {
            "    }\n",                          // 9
            "}\n",                              // 10 real module close
            "\n",                               // 11
            "pub fn prod_after() -> bool {\n",  // 12 must NOT be in set
            "    false\n",                      // 13
            "}\n",                              // 14
        ]
        .concat();

        let test_lines = cfg_test_line_numbers(&source);

        assert!(
            !test_lines.contains(&1),
            "prod fn before must not be in test set"
        );
        assert!(
            !test_lines.contains(&12),
            "prod fn after must not be in test set (naive scanner bug)"
        );
        assert!(
            !test_lines.contains(&13),
            "prod fn body must not be in test set"
        );
        assert!(
            !test_lines.contains(&14),
            "prod fn close must not be in test set"
        );
        assert!(test_lines.contains(&3), "cfg(test) attr must be in test set");
        assert!(test_lines.contains(&4), "mod tests open must be in test set");
        assert!(
            test_lines.contains(&8),
            "assert! line with char literal must be in test set"
        );
        assert!(test_lines.contains(&10), "mod tests close must be in test set");
    }

    #[test]
    fn cfg_test_comment_close_brace_does_not_exit_early() {
        // Regression: a `}` in a `//` comment must not decrement depth
        // prematurely.  If it did, the scanner exits before the real module
        // close and production code after would remain unguarded.
        let source = [
            "#[cfg(test)]\n",                        // 1
            "mod tests {\n",                         // 2
            "    fn t() {\n",                        // 3
            "        // this comment ends with }\n", // 4  comment brace
            "    }\n",                               // 5
            "}\n",                                   // 6  real module close
            "pub fn prod_after() {}\n",              // 7  must NOT be in set
        ]
        .concat();

        let test_lines = cfg_test_line_numbers(&source);

        assert!(
            !test_lines.contains(&7),
            "prod after must not be in test set (comment brace regression)"
        );
        assert!(
            test_lines.contains(&6),
            "real module close must be in test set"
        );
    }

    // ------------------------------------------------------------------
    // cfg_test_line_numbers — unit tests
    // ------------------------------------------------------------------

    #[test]
    fn cfg_test_line_numbers_detects_inline_test_module() {
        // Lines 5–11 are inside the #[cfg(test)] block.
        let source = "\
pub fn prod_fn() -> bool {\n\
    true\n\
}\n\
\n\
#[cfg(test)]\n\
mod tests {\n\
    use super::*;\n\
\n\
    #[test]\n\
    fn it_works() { assert!(prod_fn()); }\n\
}\n";
        //  line 1: pub fn prod_fn
        //  line 2:     true
        //  line 3: }
        //  line 4: (empty)
        //  line 5: #[cfg(test)]        <- test span starts
        //  line 6: mod tests {
        //  line 7:     use super::*;
        //  line 8: (empty)
        //  line 9:     #[test]
        //  line 10:    fn it_works()
        //  line 11: }                  <- test span ends

        let test_lines = cfg_test_line_numbers(source);

        // Production lines must NOT be in the set.
        assert!(!test_lines.contains(&1), "prod fn opening should not be test");
        assert!(!test_lines.contains(&2), "prod fn body should not be test");
        assert!(!test_lines.contains(&3), "prod fn closing should not be test");
        assert!(!test_lines.contains(&4), "blank line between prod and test should not be test");

        // Test module lines MUST be in the set.
        assert!(test_lines.contains(&5), "#[cfg(test)] attr line must be in set");
        assert!(test_lines.contains(&6), "mod tests opening must be in set");
        assert!(test_lines.contains(&7), "use statement inside test mod must be in set");
        assert!(test_lines.contains(&9), "#[test] attr inside test mod must be in set");
        assert!(test_lines.contains(&10), "test fn inside test mod must be in set");
        assert!(test_lines.contains(&11), "closing brace of test mod must be in set");
    }

    #[test]
    fn cfg_test_line_numbers_empty_file_returns_empty_set() {
        assert!(cfg_test_line_numbers("").is_empty());
        assert!(cfg_test_line_numbers("pub fn f() {}\n").is_empty());
    }

    #[test]
    fn cfg_test_line_numbers_handles_multiple_test_blocks() {
        // Two separate #[cfg(test)] blocks in the same file.
        let source = "\
fn a() {}\n\
#[cfg(test)]\n\
mod tests_a {\n\
    fn ta() {}\n\
}\n\
fn b() {}\n\
#[cfg(test)]\n\
mod tests_b {\n\
    fn tb() {}\n\
}\n";
        //  1: fn a() {}
        //  2: #[cfg(test)]     <- block A attr
        //  3: mod tests_a {    <- block A open
        //  4:     fn ta() {}
        //  5: }                <- block A close
        //  6: fn b() {}
        //  7: #[cfg(test)]     <- block B attr
        //  8: mod tests_b {    <- block B open
        //  9:     fn tb() {}
        // 10: }                <- block B close

        let test_lines = cfg_test_line_numbers(source);

        assert!(!test_lines.contains(&1), "fn a is production");
        assert!(!test_lines.contains(&6), "fn b is production");
        for line in [2u64, 3, 4, 5] {
            assert!(test_lines.contains(&line), "block A line {line} should be in set");
        }
        for line in [7u64, 8, 9, 10] {
            assert!(test_lines.contains(&line), "block B line {line} should be in set");
        }
    }

    // ------------------------------------------------------------------
    // strip_cfg_test_lines — integration with LcovSummary
    // ------------------------------------------------------------------

    #[test]
    fn strip_cfg_test_lines_removes_test_block_lines_from_lcov() -> TestResult {
        // Source file: 3 production lines + 4 test lines.
        // The test line at line 6 (inside the test module) is "never hit" —
        // simulating the dead branch that triggered issue #1326.
        let source = "\
pub fn prod() -> bool { true }\n\
pub fn prod2() -> i32 { 1 }\n\
pub fn prod3() -> i32 { 2 }\n\
#[cfg(test)]\n\
mod tests {\n\
    fn dead_branch() { if false { return; } }\n\
}\n";
        //  line 1: prod() — production, hit
        //  line 2: prod2() — production, hit
        //  line 3: prod3() — production, NOT hit
        //  line 4: #[cfg(test)]
        //  line 5: mod tests {
        //  line 6:     fn dead_branch — NEVER HIT in test suite
        //  line 7: }

        let temp = tempfile::tempdir()?;
        let source_file = temp.path().join("lib.rs");
        fs::write(&source_file, source)?;

        let source_path_str = source_file.to_string_lossy().replace('\\', "/");

        // LCOV has coverage data: lines 1,2 hit; line 3 not hit; line 6 not hit.
        let lcov_content = format!(
            "SF:{source_path_str}\n\
             DA:1,5\n\
             DA:2,3\n\
             DA:3,0\n\
             DA:6,0\n\
             end_of_record\n"
        );
        let lcov_path = temp.path().join("lcov.info");
        fs::write(&lcov_path, &lcov_content)?;

        let mut summary = parse_lcov(&lcov_path)?;

        // Before stripping: 4 lines found (1 hit=5, 2 hit=3, 3 hit=0, 6 hit=0).
        assert_eq!(summary.line_found, 4, "pre-strip: 4 executable lines");
        assert_eq!(summary.line_hit, 2, "pre-strip: 2 hit lines");

        strip_cfg_test_lines(&mut summary, temp.path());

        // After stripping: only production lines 1,2,3 remain; test line 6 is gone.
        assert_eq!(summary.line_found, 3, "post-strip: only 3 production lines");
        assert_eq!(summary.line_hit, 2, "post-strip: still 2 production hits");

        let file = summary.files.first().ok_or("expected file entry")?;
        assert_eq!(file.line_found, 3);
        assert_eq!(file.line_hit, 2);
        // Line 6 (test code) must be absent.
        assert!(!file.lines.iter().any(|l| l.number == 6), "test line 6 must be stripped");
        // Production uncovered line 3 must still be reported.
        assert!(file.uncovered_lines.contains(&3), "production uncovered line 3 must remain");
        Ok(())
    }

    #[test]
    fn strip_cfg_test_lines_does_not_affect_production_hit_count() -> TestResult {
        // Ensure that stripping test lines does not accidentally lower
        // production hit counts (regression guard for the summary delta update).
        let source = "\
pub fn prod() -> bool { true }\n\
#[cfg(test)]\n\
mod tests {\n\
    fn unused() {}\n\
}\n";
        let temp = tempfile::tempdir()?;
        let source_file = temp.path().join("lib.rs");
        fs::write(&source_file, source)?;

        let source_path_str = source_file.to_string_lossy().replace('\\', "/");
        let lcov_content = format!(
            "SF:{source_path_str}\n\
             DA:1,10\n\
             DA:4,0\n\
             end_of_record\n"
        );
        let lcov_path = temp.path().join("lcov.info");
        fs::write(&lcov_path, &lcov_content)?;

        let mut summary = parse_lcov(&lcov_path)?;
        strip_cfg_test_lines(&mut summary, temp.path());

        // Only line 1 (production, hit) should remain.
        assert_eq!(summary.line_found, 1);
        assert_eq!(summary.line_hit, 1);
        Ok(())
    }

    #[test]
    fn strip_cfg_test_lines_patch_coverage_full_for_test_only_pr() -> TestResult {
        // Simulate a test-only PR: the diff adds lines 4-7 (all inside the
        // test module).  After stripping, none of those lines appear in the
        // LCOV, so patch_coverage_from_changed_lines returns 100.0.
        let source = "\
pub fn prod() -> bool { true }\n\
\n\
#[cfg(test)]\n\
mod tests {\n\
    fn new_test() { assert!(prod()); }\n\
    fn dead() { if false { return; } }\n\
}\n";
        let temp = tempfile::tempdir()?;
        let source_file = temp.path().join("crates/mylib/src/lib.rs");
        fs::create_dir_all(source_file.parent().ok_or("no parent")?)?;
        fs::write(&source_file, source)?;

        let source_path_str = source_file.to_string_lossy().replace('\\', "/");
        // LCOV: line 1 hit; lines 5 and 6 never hit (test-only dead branches).
        let lcov_content = format!(
            "SF:{source_path_str}\n\
             DA:1,1\n\
             DA:5,0\n\
             DA:6,0\n\
             end_of_record\n"
        );
        let lcov_path = temp.path().join("lcov.info");
        fs::write(&lcov_path, &lcov_content)?;

        let mut summary = parse_lcov(&lcov_path)?;
        strip_cfg_test_lines(&mut summary, temp.path());

        // Changed lines are only within the test block (lines 3-7).
        let changed: BTreeMap<String, BTreeSet<u64>> = BTreeMap::from([(
            "crates/mylib/src/lib.rs".to_string(),
            BTreeSet::from([3u64, 4, 5, 6, 7]),
        )]);

        // After stripping, those lines no longer appear in LCOV, so
        // executable_found == 0 → patch coverage returns 100.0.
        let patch =
            patch_coverage_from_changed_lines_for_root(Some(temp.path()), &summary, &changed);
        assert_eq!(patch, 100.0, "test-only PR patch coverage must be 100.0 after stripping");
        Ok(())
    }
}
