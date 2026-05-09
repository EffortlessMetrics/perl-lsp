//! Native critic check receipts.

use chrono::{DateTime, Utc};
use color_eyre::eyre::{Context, Result, eyre};
use perl_lsp_rs_core::tooling::perl_critic::{
    CriticConfig, CriticContext, CriticFinding, NativeCriticRegistry,
};
use perl_parser::Parser;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

const SCHEMA_VERSION: u32 = 1;

/// Options for `cargo xtask native-critic check`.
pub struct NativeCriticCheckConfig {
    /// Files or directories containing Perl sources.
    pub roots: Vec<PathBuf>,
    /// Minimum native critic severity to report.
    pub severity: u8,
    /// Native rule IDs to include. Empty means all recommended rules.
    pub include: Vec<String>,
    /// Native rule IDs to exclude.
    pub exclude: Vec<String>,
    /// Output JSON receipt path.
    pub receipt: PathBuf,
    /// Output markdown summary path.
    pub summary: PathBuf,
}

#[derive(Debug, Serialize)]
struct NativeCriticCheckReceipt {
    kind: &'static str,
    schema_version: u32,
    generated_at: DateTime<Utc>,
    commit: String,
    roots: Vec<String>,
    profile: &'static str,
    engine: &'static str,
    severity: u8,
    include: Vec<String>,
    exclude: Vec<String>,
    rules_run: usize,
    rule_ids: Vec<String>,
    files_checked: usize,
    files_with_parse_errors: usize,
    findings_count: usize,
    suppressed_findings_count: usize,
    fixable_findings_count: usize,
    findings_by_rule: BTreeMap<String, usize>,
    suppressed_findings_by_rule: BTreeMap<String, usize>,
    files: Vec<NativeCriticFileResult>,
}

#[derive(Debug, Serialize)]
struct NativeCriticFileResult {
    path: String,
    parse_ok: bool,
    parse_error: Option<String>,
    findings_count: usize,
    suppressed_findings_count: usize,
    fixable_findings_count: usize,
    findings: Vec<CriticFinding>,
}

/// Run native critic rules over Perl source files and write JSON/markdown receipts.
pub fn check(config: NativeCriticCheckConfig) -> Result<()> {
    let roots = if config.roots.is_empty() { default_roots() } else { config.roots };
    let files = collect_paths(&roots)?;
    if files.is_empty() {
        return Err(eyre!(
            "no native critic source files found under {}",
            roots.iter().map(|root| root.display().to_string()).collect::<Vec<_>>().join(", ")
        ));
    }

    let critic_config = CriticConfig {
        severity: config.severity,
        include: config.include.clone(),
        exclude: config.exclude.clone(),
        ..CriticConfig::default()
    };
    let registry = NativeCriticRegistry::recommended();
    let rule_ids = registry.rule_ids().into_iter().map(ToOwned::to_owned).collect::<Vec<_>>();
    let mut results = Vec::new();

    for file in files {
        results.push(check_file(&registry, &critic_config, &file)?);
    }

    let mut findings_by_rule = BTreeMap::new();
    let mut suppressed_findings_by_rule = BTreeMap::new();
    for result in &results {
        for finding in &result.findings {
            *findings_by_rule.entry(finding.rule_id.clone()).or_insert(0) += 1;
        }
        let suppressed = suppressed_rule_counts(&result.findings, result.suppressed_findings_count);
        for (rule_id, count) in suppressed {
            *suppressed_findings_by_rule.entry(rule_id).or_insert(0) += count;
        }
    }

    let receipt = NativeCriticCheckReceipt {
        kind: "native_critic_check",
        schema_version: SCHEMA_VERSION,
        generated_at: Utc::now(),
        commit: current_commit(),
        roots: roots.iter().map(|root| root.display().to_string()).collect(),
        profile: "recommended",
        engine: "native",
        severity: config.severity,
        include: config.include,
        exclude: config.exclude,
        rules_run: rule_ids.len(),
        rule_ids,
        files_checked: results.len(),
        files_with_parse_errors: results.iter().filter(|result| !result.parse_ok).count(),
        findings_count: results.iter().map(|result| result.findings_count).sum(),
        suppressed_findings_count: results
            .iter()
            .map(|result| result.suppressed_findings_count)
            .sum(),
        fixable_findings_count: results.iter().map(|result| result.fixable_findings_count).sum(),
        findings_by_rule,
        suppressed_findings_by_rule,
        files: results,
    };

    if let Some(parent) = config.receipt.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        fs::create_dir_all(parent)
            .wrap_err_with(|| format!("failed to create {}", parent.display()))?;
    }
    if let Some(parent) = config.summary.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        fs::create_dir_all(parent)
            .wrap_err_with(|| format!("failed to create {}", parent.display()))?;
    }
    write_json(&config.receipt, &receipt)?;
    write_summary(&config.summary, &receipt)?;

    println!(
        "native critic check: {} finding(s), {} suppressed, {} fixable across {} file(s); receipt: {}",
        receipt.findings_count,
        receipt.suppressed_findings_count,
        receipt.fixable_findings_count,
        receipt.files_checked,
        config.receipt.display()
    );

    Ok(())
}

fn check_file(
    registry: &NativeCriticRegistry,
    config: &CriticConfig,
    file: &Path,
) -> Result<NativeCriticFileResult> {
    let source =
        fs::read_to_string(file).wrap_err_with(|| format!("failed to read {}", file.display()))?;
    let parse_result = Parser::new(&source).parse();
    let Ok(ast) = parse_result else {
        return Ok(NativeCriticFileResult {
            path: file.display().to_string(),
            parse_ok: false,
            parse_error: Some("native parser failed to parse source".to_string()),
            findings_count: 0,
            suppressed_findings_count: 0,
            fixable_findings_count: 0,
            findings: Vec::new(),
        });
    };

    let ctx = CriticContext::new(&source, &ast, config);
    let findings = registry.check(&ctx);
    let unsuppressed_source = strip_native_suppression_lines(&source);
    let unsuppressed_count = if unsuppressed_source == source {
        findings.len()
    } else {
        match Parser::new(&unsuppressed_source).parse() {
            Ok(ast) => {
                let ctx = CriticContext::new(&unsuppressed_source, &ast, config);
                registry.check(&ctx).len()
            }
            Err(_) => findings.len(),
        }
    };
    let suppressed_findings_count = unsuppressed_count.saturating_sub(findings.len());
    let fixable_findings_count = findings.iter().filter(|finding| finding.fix.is_some()).count();

    Ok(NativeCriticFileResult {
        path: file.display().to_string(),
        parse_ok: true,
        parse_error: None,
        findings_count: findings.len(),
        suppressed_findings_count,
        fixable_findings_count,
        findings,
    })
}

fn strip_native_suppression_lines(source: &str) -> String {
    source
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with("## no critic ") || trimmed.starts_with("## no perl-lsp-critic ")
            {
                ""
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn suppressed_rule_counts(
    findings: &[CriticFinding],
    suppressed_findings_count: usize,
) -> BTreeMap<String, usize> {
    if suppressed_findings_count == 0 {
        return BTreeMap::new();
    }

    let mut counts = BTreeMap::new();
    let rules = findings.iter().map(|finding| finding.rule_id.clone()).collect::<BTreeSet<_>>();
    if rules.len() == 1 {
        if let Some(rule_id) = rules.into_iter().next() {
            counts.insert(rule_id, suppressed_findings_count);
        }
    }
    counts
}

fn write_summary(path: &Path, receipt: &NativeCriticCheckReceipt) -> Result<()> {
    let mut markdown = String::new();
    markdown.push_str("# Native Critic Check\n\n");
    markdown.push_str(&format!("- Engine: `{}`\n", receipt.engine));
    markdown.push_str(&format!("- Profile: `{}`\n", receipt.profile));
    markdown.push_str(&format!("- Severity: `{}`\n", receipt.severity));
    markdown.push_str(&format!("- Files checked: `{}`\n", receipt.files_checked));
    markdown
        .push_str(&format!("- Files with parse errors: `{}`\n", receipt.files_with_parse_errors));
    markdown.push_str(&format!("- Rules run: `{}`\n", receipt.rules_run));
    markdown.push_str(&format!("- Findings: `{}`\n", receipt.findings_count));
    markdown.push_str(&format!("- Suppressed findings: `{}`\n", receipt.suppressed_findings_count));
    markdown.push_str(&format!("- Fixable findings: `{}`\n\n", receipt.fixable_findings_count));

    markdown.push_str("## Findings By Rule\n\n");
    markdown.push_str("| Rule | Findings |\n");
    markdown.push_str("| --- | ---: |\n");
    for (rule, count) in &receipt.findings_by_rule {
        markdown.push_str(&format!("| `{rule}` | {count} |\n"));
    }
    if receipt.findings_by_rule.is_empty() {
        markdown.push_str("| _none_ | 0 |\n");
    }

    markdown.push_str("\n## Files\n\n");
    markdown.push_str("| File | Parse ok | Findings | Suppressed | Fixable |\n");
    markdown.push_str("| --- | --- | ---: | ---: | ---: |\n");
    for file in &receipt.files {
        markdown.push_str(&format!(
            "| `{}` | {} | {} | {} | {} |\n",
            file.path,
            file.parse_ok,
            file.findings_count,
            file.suppressed_findings_count,
            file.fixable_findings_count
        ));
    }

    fs::write(path, markdown).wrap_err_with(|| format!("failed to write {}", path.display()))
}

fn default_roots() -> Vec<PathBuf> {
    vec![
        PathBuf::from("examples/perl"),
        PathBuf::from("tests/perl-corpus"),
        PathBuf::from("crates/perl-corpus/fixtures/parser_accuracy"),
    ]
}

fn collect_paths(roots: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for root in roots {
        if root.is_file() {
            if is_perl_source(root) {
                paths.push(root.to_path_buf());
            }
            continue;
        }
        if !root.exists() {
            continue;
        }
        for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
            if entry.file_type().is_file() && is_perl_source(entry.path()) {
                paths.push(entry.path().to_path_buf());
            }
        }
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn is_perl_source(path: &Path) -> bool {
    let filename = path.file_name().and_then(|name| name.to_str()).unwrap_or_default();
    if filename.ends_with(".expected.pl") || filename.ends_with(".expected-diagnostics.txt") {
        return false;
    }
    matches!(path.extension().and_then(|ext| ext.to_str()), Some("pl" | "pm" | "t"))
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let json = serde_json::to_string_pretty(value)?;
    fs::write(path, format!("{json}\n"))
        .wrap_err_with(|| format!("failed to write {}", path.display()))
}

fn current_commit() -> String {
    Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|stdout| stdout.trim().to_string())
        .filter(|commit| !commit.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn native_critic_check_writes_receipt_and_summary() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let source = temp.path().join("App.pm");
        fs::write(&source, "package App;\nsub run { my $unused = 1; return 1; }\n1;\n")?;
        let receipt = temp.path().join("native-critic-check.json");
        let summary = temp.path().join("native-critic-check.md");

        check(NativeCriticCheckConfig {
            roots: vec![source],
            severity: 1,
            include: vec!["native.variables.unused_lexical".to_string()],
            exclude: Vec::new(),
            receipt: receipt.clone(),
            summary: summary.clone(),
        })?;

        let value: Value = serde_json::from_str(&fs::read_to_string(receipt)?)?;
        assert_eq!(value["kind"], "native_critic_check");
        assert_eq!(value["engine"], "native");
        assert_eq!(value["profile"], "recommended");
        assert_eq!(value["files_checked"], 1);
        assert_eq!(value["rules_run"], 27);
        assert_eq!(value["findings_count"], 1);
        assert_eq!(value["fixable_findings_count"], 1);
        assert_eq!(value["findings_by_rule"]["native.variables.unused_lexical"], 1);

        let summary = fs::read_to_string(summary)?;
        assert!(summary.contains("# Native Critic Check"));
        assert!(summary.contains("| `native.variables.unused_lexical` | 1 |"));
        Ok(())
    }

    #[test]
    fn native_critic_check_counts_suppressed_findings() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let source = temp.path().join("App.pm");
        fs::write(
            &source,
            "## no perl-lsp-critic native.variables.unused_lexical -- generated\npackage App;\nsub run { my $unused = 1; return 1; }\n1;\n",
        )?;
        let receipt = temp.path().join("native-critic-check.json");
        let summary = temp.path().join("native-critic-check.md");

        check(NativeCriticCheckConfig {
            roots: vec![source],
            severity: 1,
            include: vec!["native.variables.unused_lexical".to_string()],
            exclude: Vec::new(),
            receipt: receipt.clone(),
            summary,
        })?;

        let value: Value = serde_json::from_str(&fs::read_to_string(receipt)?)?;
        assert_eq!(value["findings_count"], 0);
        assert_eq!(value["suppressed_findings_count"], 1);
        assert_eq!(value["files"][0]["suppressed_findings_count"], 1);
        Ok(())
    }
}
