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
    let lcov = parse_lcov(&args.lcov)?;
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
    let files_below_target = lcov
        .files
        .iter()
        .filter(|file| !file.path.trim().is_empty())
        .filter(|file| file.line_found > 0 && percent(file.line_hit, file.line_found) < 95.0)
        .filter_map(file_gap_json)
        .collect::<Vec<_>>();

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
        "patch_files_below_target": patch_files_below_target,
        "files_below_target": files_below_target,
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
        "line_coverage": percent(file.line_hit, file.line_found),
        "sample_uncovered_lines": samples,
    }))
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
        assert_eq!(gap["sample_uncovered_lines"], json!([3]));
        Ok(())
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
        let repo = temp.path();
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
        assert_eq!(receipt["scope"], json!("workspace-lib-xtask-quality"));
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
}
