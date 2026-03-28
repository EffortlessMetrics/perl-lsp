//! Generate the parser feature matrix documentation from a corpus audit report.

use crate::utils::project_root;
use chrono::Local;
use color_eyre::eyre::{Context, ContextCompat, Result, bail, eyre};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use toml::Value;

use super::corpus_audit::{AuditReport, FailingFile, ParseOutcomesSummary};

const BASELINE_PATH: &str = "ci/parse_errors_baseline.txt";
const UNKNOWN: &str = "unknown";

const CATEGORY_TAXONOMY: &[(&str, &str, &str)] = &[
    ("ModernFeature", "P1", "class/try/catch/field/method keywords"),
    ("QuoteLike", "P2", "q/qq/qw/qx/qr, heredocs, strings"),
    ("Regex", "P2", "m//, s///, tr///, patterns"),
    ("ControlFlow", "P2", "given/when/default"),
    ("Dereference", "P2", "->, postfix deref"),
    ("Subroutine", "P2", "Signatures, prototypes"),
    ("General", "P3", "Uncategorized"),
];

pub fn run_with_paths(report: PathBuf, output: PathBuf) -> Result<()> {
    let root = project_root()?;
    let report_path = root.join(&report);
    let output_path = root.join(&output);

    let report = load_report(&report_path)?;
    let output = generate_matrix(&root, &report)?;
    fs::create_dir_all(output_path.parent().context("matrix output parent path missing")?)?;
    fs::write(&output_path, output)
        .context(format!("failed to write {}", output_path.display()))?;

    println!("Updated {}", output_path.display());
    println!(
        "  Parse success: {}/{} ({:.0}%)",
        report.parse_outcomes.ok,
        report.parse_outcomes.total,
        success_rate(&report.parse_outcomes)
    );
    println!(
        "  Errors: {} ({} categories)",
        report.parse_outcomes.error,
        report.parse_outcomes.error_by_category.len()
    );

    Ok(())
}

fn load_report(path: &Path) -> Result<AuditReport> {
    if !path.exists() {
        bail!("Error: {} not found. Run 'just parser-audit' first.", path.display());
    }

    let report_text = fs::read_to_string(path)
        .with_context(|| format!("failed to read parser audit report {}", path.display()))?;
    serde_json::from_str(&report_text).context("failed to parse parser audit report JSON")
}

fn get_git_sha(root: &Path) -> String {
    let output =
        Command::new("git").arg("rev-parse").arg("--short").arg("HEAD").current_dir(root).output();
    match output {
        Ok(result) => {
            if result.status.success() {
                let sha = String::from_utf8_lossy(&result.stdout).trim().to_string();
                if sha.is_empty() { UNKNOWN.to_string() } else { sha }
            } else {
                UNKNOWN.to_string()
            }
        }
        Err(_) => UNKNOWN.to_string(),
    }
}

fn get_crate_version(root: &Path, crate_name: &str) -> String {
    let cargo_path = root.join("crates").join(crate_name).join("Cargo.toml");
    let cargo_text = match fs::read_to_string(&cargo_path) {
        Ok(text) => text,
        Err(_) => return UNKNOWN.to_string(),
    };
    let cargo_toml: Value = match toml::from_str(&cargo_text) {
        Ok(value) => value,
        Err(_) => return UNKNOWN.to_string(),
    };
    cargo_toml
        .get("package")
        .and_then(|pkg| pkg.get("version"))
        .and_then(|version| version.as_str())
        .map(ToString::to_string)
        .unwrap_or_else(|| UNKNOWN.to_string())
}

fn success_rate(outcomes: &ParseOutcomesSummary) -> f64 {
    if outcomes.total == 0 { 0.0 } else { outcomes.ok as f64 * 100.0 / outcomes.total as f64 }
}

fn format_location(file: &FailingFile) -> Option<String> {
    file.line_number.map(|line_num| match file.column {
        Some(column) => format!("line {line_num}:{column}"),
        None => format!("line {line_num}"),
    })
}

fn format_token_info(file: &FailingFile) -> Option<String> {
    match (file.expected.as_deref(), file.found_token.as_deref()) {
        (Some(expected), Some(found)) => {
            Some(format!("expected `{}`, found `{}`", expected, found))
        }
        (Some(expected), None) => Some(format!("expected `{}`", expected)),
        (None, Some(found)) => Some(format!("found `{}`", found)),
        (None, None) => None,
    }
}

fn generate_matrix(root: &Path, report: &AuditReport) -> Result<String> {
    let outcomes = &report.parse_outcomes;
    let baseline = baseline_error_count(root)?;
    let success_rate = success_rate(outcomes);
    let ga_coverage = report.ga_coverage.coverage_percentage;
    let mut lines: Vec<String> = Vec::new();

    lines.push("# Parser Feature Matrix".to_string());
    lines.push(String::new());
    lines.push(
        "> **Issue #180**: This document tracks parser coverage and missing features.".to_string(),
    );
    lines.push(String::new());
    lines.push("## Provenance".to_string());
    lines.push(String::new());
    lines.push("| Field | Value |".to_string());
    lines.push("|-------|-------|".to_string());
    lines.push(format!("| Generated | {} |", Local::now().format("%Y-%m-%d %H:%M")));
    lines.push(format!("| Commit | `{}` |", get_git_sha(root)));
    lines.push(format!("| perl-parser | v{} |", get_crate_version(root, "perl-parser")));
    lines.push("| Corpus | `test_corpus/` |".to_string());
    lines.push("| Command | `just parser-audit && just parser-matrix-update` |".to_string());
    lines.push(String::new());

    lines.push("## Summary".to_string());
    lines.push(String::new());
    lines.push("| Metric | Current | Target | Status |".to_string());
    lines.push("|--------|---------|--------|--------|".to_string());
    lines.push(format!(
        "| Parse Success Rate | {:.0}% ({}/{}) | 100% | {} |",
        success_rate,
        outcomes.ok,
        outcomes.total,
        if outcomes.error == 0 { "Passing" } else { "In Progress" }
    ));
    lines.push(format!(
        "| Parse Errors | {} | 0 | {} |",
        outcomes.error,
        if outcomes.error == 0 { "Passing" } else { "Baseline Set" }
    ));
    lines.push(format!(
        "| Timeouts | {} | 0 | {} |",
        outcomes.timeout,
        if outcomes.timeout == 0 { "Passing" } else { "Failed" }
    ));
    lines.push(format!(
        "| Panics | {} | 0 | {} |",
        outcomes.panic,
        if outcomes.panic == 0 { "Passing" } else { "Failed" }
    ));
    lines.push(format!(
        "| Test Corpus Inventory | {:.0}% | 100% | {} |",
        ga_coverage,
        if ga_coverage >= 80.0 { "Passing" } else { "In Progress" }
    ));
    if let Some(baseline) = baseline {
        lines.push(format!("| Baseline | {} | 0 | Ratcheted |", baseline));
    }

    lines.push(String::new());
    lines.push(
        "*Test Corpus Inventory* measures whether the test corpus contains examples of each"
            .to_string(),
    );
    lines.push(
        "GA (generally available) feature defined in `features.toml`. It does NOT measure"
            .to_string(),
    );
    lines.push(
        "whether those features parse successfully—that's what Parse Success Rate tracks."
            .to_string(),
    );
    lines.push(String::new());
    lines.push("## Error Breakdown by Category".to_string());
    lines.push(String::new());
    lines.push("Errors are categorized to help prioritize implementation work:".to_string());
    lines.push(String::new());
    lines.push("| Category | Count | Priority | Description |".to_string());
    lines.push("|----------|-------|----------|-------------|".to_string());

    let mut categories: Vec<(String, usize, String, String)> = CATEGORY_TAXONOMY
        .iter()
        .map(|(name, priority, description)| {
            (
                name.to_string(),
                *outcomes.error_by_category.get(*name).unwrap_or(&0),
                priority.to_string(),
                description.to_string(),
            )
        })
        .collect::<Vec<_>>();
    categories.sort_by(
        |(a_name, a_count, ..): &(String, usize, String, String),
         (b_name, b_count, ..): &(String, usize, String, String)| {
            b_count.cmp(a_count).then_with(|| a_name.cmp(b_name))
        },
    );
    for (name, count, priority, description) in categories {
        lines.push(format!("| {name} | {count} | {priority} | {description} |"));
    }

    lines.push(String::new());
    lines.push("## Failing Files".to_string());
    lines.push(String::new());

    if outcomes.failing_files.is_empty() {
        lines.push("*No failing files* ✅".to_string());
    } else {
        for file in &outcomes.failing_files {
            lines.push(format!("### `{}`", file.path));
            lines.push(String::new());
            lines.push(format!("- **Category**: {}", file.category));
            if let Some(location) = format_location(file) {
                lines.push(format!("- **Location**: {location}"));
            }
            if let Some(token_info) = format_token_info(file) {
                lines.push(format!("- **Error**: {token_info}"));
            }
            if let Some(snippet) = &file.code_snippet {
                lines.push(String::new());
                lines.push("```perl".to_string());
                lines.push(snippet.to_string());
                lines.push("```".to_string());
            }
            lines.push(String::new());
        }
    }

    lines.push(String::new());
    lines.push("## Coverage Roadmap".to_string());
    lines.push(String::new());
    lines.push("### Phase 1: Stabilize Core (Current)".to_string());
    lines.push("- [x] Establish baseline ratchet (Issue #180)".to_string());
    lines.push("- [x] Add error categorization".to_string());
    lines.push(format!(
        "- {} Reduce parse errors to 0",
        if outcomes.error == 0 { "[x]" } else { "[ ]" }
    ));
    lines.push(String::new());

    lines.push("### Phase 2: Modern Perl Features".to_string());
    lines.push("- [ ] `class` keyword (Perl 5.38+, Corinna)".to_string());
    lines.push("- [ ] `try`/`catch`/`finally` blocks".to_string());
    lines.push("- [ ] `field` and `method` declarations".to_string());
    lines.push("- [ ] `builtin::` functions".to_string());
    lines.push(String::new());

    lines.push("### Phase 3: Edge Cases".to_string());
    lines.push("- [ ] Complex heredoc scenarios".to_string());
    lines.push("- [ ] Unicode in quote delimiters".to_string());
    lines.push("- [ ] Recursive regex patterns".to_string());
    lines.push(String::new());

    lines.push("## How to Use".to_string());
    lines.push(String::new());
    lines.push("```bash".to_string());
    lines.push("# View current parse status".to_string());
    lines.push("just parser-audit".to_string());
    lines.push(String::new());
    lines.push("# Check against baseline (CI mode)".to_string());
    lines.push("just ci-parser-features-check".to_string());
    lines.push(String::new());
    lines.push("# Update this document from latest audit".to_string());
    lines.push("just parser-matrix-update".to_string());
    lines.push("```".to_string());
    lines.push(String::new());

    lines.push("## Baseline Ratchet".to_string());
    lines.push(String::new());
    lines.push("The parse error count uses a ratchet mechanism:".to_string());
    lines.push(String::new());
    lines.push("- Baseline stored in `ci/parse_errors_baseline.txt`".to_string());
    lines.push("- CI fails if parse errors **increase**".to_string());
    lines.push("- CI passes if parse errors stay same or decrease".to_string());
    lines.push(
        "- When errors decrease, update baseline: `echo N > ci/parse_errors_baseline.txt`"
            .to_string(),
    );
    lines.push(String::new());

    lines.push(
        "**Philosophy**: Baseline updates are only allowed when the parser actually improves"
            .to_string(),
    );
    lines.push(
        "(error count decreases), never to paper over regressions. The ratchet ensures the"
            .to_string(),
    );
    lines.push("codebase only gets easier to reason about over time.".to_string());
    lines.push(String::new());

    lines.push("## Related Documentation".to_string());
    lines.push(String::new());
    lines.push("- [CLAUDE.md](../CLAUDE.md) - Project overview and commands".to_string());
    lines.push(
        "- [LSP_IMPLEMENTATION_GUIDE.md](LSP_IMPLEMENTATION_GUIDE.md) - LSP server architecture"
            .to_string(),
    );
    lines.push("- [features.toml](../features.toml) - LSP feature catalog".to_string());

    Ok(format!("{}\n", lines.join("\n")))
}

fn baseline_error_count(root: &Path) -> Result<Option<usize>> {
    let baseline_path = root.join(BASELINE_PATH);
    if !baseline_path.exists() {
        return Ok(None);
    }
    let baseline_text = fs::read_to_string(&baseline_path)
        .context("failed to read parser baseline file")?
        .trim()
        .to_string();
    if baseline_text.is_empty() {
        return Ok(None);
    }
    baseline_text
        .parse::<usize>()
        .map(Some)
        .map_err(|_| eyre!("failed to parse baseline value as integer: {baseline_text}"))
}
