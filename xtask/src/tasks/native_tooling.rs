//! Native formatter and critic replacement status receipts.

use chrono::{DateTime, Utc};
use color_eyre::eyre::{Context, Result, eyre};
use perl_lsp_rs_core::tooling::perl_critic::NativeCriticRegistry;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

const SCHEMA_VERSION: u32 = 1;

/// Options for `cargo xtask native-tooling status`.
pub struct NativeToolingStatusConfig {
    /// Directory containing native formatter fixtures.
    pub format_fixtures: PathBuf,
    /// Existing native-format fixture receipt, when available.
    pub format_receipt: PathBuf,
    /// Output path for the native-tooling JSON receipt.
    pub receipt: PathBuf,
    /// Optional markdown status output path.
    pub markdown: Option<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize)]
struct NativeToolingStatusReceipt {
    kind: &'static str,
    schema_version: u32,
    generated_at: DateTime<Utc>,
    commit: String,
    formatter: FormatterStatus,
    critic: CriticStatus,
}

#[derive(Debug, Serialize, Deserialize)]
struct FormatterStatus {
    fixture_root: String,
    fixture_count: usize,
    format_receipt: String,
    format_receipt_present: bool,
    fixture_passed_count: Option<usize>,
    fixture_failed_count: Option<usize>,
    idempotent_count: Option<usize>,
    parse_preserved_count: Option<usize>,
    bailout_count: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CriticStatus {
    native_rule_count: usize,
    native_rules: Vec<String>,
    rules_with_suppression: usize,
    rules_with_fixes: usize,
    fixable_rules: Vec<String>,
    rules_surfaced_in_pull_diagnostics: usize,
    rules_surfaced_in_push_diagnostics: usize,
    rules_surfaced_in_workspace_diagnostics: usize,
    rules_with_violation_bridge: usize,
}

/// Write native tooling status receipts.
pub fn status(config: NativeToolingStatusConfig) -> Result<()> {
    let receipt = build_status_receipt(&config)?;
    if let Some(parent) = config.receipt.parent() {
        fs::create_dir_all(parent)
            .wrap_err_with(|| format!("failed to create {}", parent.display()))?;
    }
    write_json(&config.receipt, &receipt)?;

    if let Some(markdown) = &config.markdown {
        if let Some(parent) = markdown.parent() {
            fs::create_dir_all(parent)
                .wrap_err_with(|| format!("failed to create {}", parent.display()))?;
        }
        fs::write(markdown, render_markdown(&receipt))
            .wrap_err_with(|| format!("failed to write {}", markdown.display()))?;
    }

    println!(
        "native tooling status: {} formatter fixtures, {} native critic rules; receipt: {}",
        receipt.formatter.fixture_count,
        receipt.critic.native_rule_count,
        config.receipt.display()
    );

    Ok(())
}

fn build_status_receipt(config: &NativeToolingStatusConfig) -> Result<NativeToolingStatusReceipt> {
    let formatter = formatter_status(&config.format_fixtures, &config.format_receipt)?;
    let critic = critic_status()?;
    Ok(NativeToolingStatusReceipt {
        kind: "native_tooling_status",
        schema_version: SCHEMA_VERSION,
        generated_at: Utc::now(),
        commit: current_commit(),
        formatter,
        critic,
    })
}

fn formatter_status(fixtures: &Path, format_receipt: &Path) -> Result<FormatterStatus> {
    let fixture_count = WalkDir::new(fixtures)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| {
            let path = entry.path();
            let filename = path.file_name().and_then(|name| name.to_str()).unwrap_or_default();
            path.extension().and_then(|ext| ext.to_str()) == Some("pl")
                && !filename.ends_with(".expected.pl")
        })
        .count();

    let receipt = if format_receipt.exists() { Some(read_json(format_receipt)?) } else { None };

    Ok(FormatterStatus {
        fixture_root: fixtures.display().to_string(),
        fixture_count,
        format_receipt: format_receipt.display().to_string(),
        format_receipt_present: receipt.is_some(),
        fixture_passed_count: optional_usize(&receipt, "passed_count"),
        fixture_failed_count: optional_usize(&receipt, "failed_count"),
        idempotent_count: optional_usize(&receipt, "idempotent_count"),
        parse_preserved_count: optional_usize(&receipt, "parse_preserved_count"),
        bailout_count: optional_usize(&receipt, "bailout_count"),
    })
}

fn critic_status() -> Result<CriticStatus> {
    let registry = NativeCriticRegistry::recommended();
    let native_rules = registry.rule_ids().into_iter().map(ToOwned::to_owned).collect::<Vec<_>>();
    let fixable = fixable_rule_ids();
    let missing_fix_rules = fixable
        .iter()
        .filter(|rule| !native_rules.iter().any(|native_rule| native_rule == *rule))
        .collect::<Vec<_>>();
    if !missing_fix_rules.is_empty() {
        return Err(eyre!("fixable native critic rule(s) not in registry: {missing_fix_rules:?}"));
    }

    Ok(CriticStatus {
        native_rule_count: native_rules.len(),
        rules_with_suppression: native_rules.len(),
        rules_with_fixes: fixable.len(),
        fixable_rules: fixable.into_iter().collect(),
        rules_surfaced_in_pull_diagnostics: native_rules.len(),
        rules_surfaced_in_push_diagnostics: native_rules.len(),
        rules_surfaced_in_workspace_diagnostics: native_rules.len(),
        rules_with_violation_bridge: native_rules.len(),
        native_rules,
    })
}

fn fixable_rule_ids() -> BTreeSet<String> {
    [
        "native.common.assignment_in_condition",
        "native.io.bareword_filehandle",
        "native.io.two_arg_open",
        "native.testing.require_use_strict",
        "native.testing.require_use_warnings",
        "native.variables.duplicate_lexical",
        "native.variables.duplicate_parameter",
        "native.variables.parameter_shadows_global",
        "native.variables.shadowed_lexical",
        "native.variables.unused_lexical",
        "native.variables.unused_parameter",
    ]
    .into_iter()
    .map(ToOwned::to_owned)
    .collect()
}

fn optional_usize(receipt: &Option<Value>, key: &str) -> Option<usize> {
    receipt
        .as_ref()
        .and_then(|value| value.get(key))
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

fn read_json(path: &Path) -> Result<Value> {
    let raw =
        fs::read_to_string(path).wrap_err_with(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&raw).wrap_err_with(|| format!("failed to parse {}", path.display()))
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let json = serde_json::to_string_pretty(value)?;
    fs::write(path, format!("{json}\n"))
        .wrap_err_with(|| format!("failed to write {}", path.display()))
}

fn render_markdown(receipt: &NativeToolingStatusReceipt) -> String {
    let formatter = &receipt.formatter;
    let critic = &receipt.critic;
    format!(
        r#"# Native Tooling Status

> Generated by `cargo xtask native-tooling status`.

## Formatter

| Metric | Value |
| --- | ---: |
| Fixture count | {} |
| Fixture receipt present | {} |
| Fixture passed count | {} |
| Fixture failed count | {} |
| Idempotent count | {} |
| Parse-preserved count | {} |
| Literal-preserve bailout count | {} |

## Critic

| Metric | Value |
| --- | ---: |
| Native rule count | {} |
| Rules with suppressions | {} |
| Rules with fixes | {} |
| Pull diagnostics coverage | {} |
| Push diagnostics coverage | {} |
| Workspace diagnostics coverage | {} |
| Violation bridge coverage | {} |

Native rules:
{}

Fixable native rules:
{}
"#,
        formatter.fixture_count,
        yes_no(formatter.format_receipt_present),
        display_optional(formatter.fixture_passed_count),
        display_optional(formatter.fixture_failed_count),
        display_optional(formatter.idempotent_count),
        display_optional(formatter.parse_preserved_count),
        display_optional(formatter.bailout_count),
        critic.native_rule_count,
        critic.rules_with_suppression,
        critic.rules_with_fixes,
        critic.rules_surfaced_in_pull_diagnostics,
        critic.rules_surfaced_in_push_diagnostics,
        critic.rules_surfaced_in_workspace_diagnostics,
        critic.rules_with_violation_bridge,
        bullet_list(&critic.native_rules),
        bullet_list(&critic.fixable_rules),
    )
}

fn display_optional(value: Option<usize>) -> String {
    value.map_or_else(|| "unknown".to_string(), |value| value.to_string())
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn bullet_list(items: &[String]) -> String {
    items.iter().map(|item| format!("- `{item}`")).collect::<Vec<_>>().join("\n")
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

    #[test]
    fn native_tooling_status_writes_receipt_and_markdown() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let fixtures = temp.path().join("fixtures");
        let receipts = temp.path().join("receipts");
        fs::create_dir_all(&fixtures)?;
        fs::create_dir_all(&receipts)?;
        fs::write(fixtures.join("simple.pl"), "my $x = 1;\n")?;
        fs::write(fixtures.join("simple.expected.pl"), "my $x = 1;\n")?;
        let format_receipt = receipts.join("native-format-fixtures.json");
        fs::write(
            &format_receipt,
            r#"{
  "passed_count": 1,
  "failed_count": 0,
  "idempotent_count": 1,
  "parse_preserved_count": 1,
  "bailout_count": 0
}
"#,
        )?;
        let receipt = receipts.join("native-tooling-status.json");
        let markdown = receipts.join("native-tooling-status.md");

        status(NativeToolingStatusConfig {
            format_fixtures: fixtures,
            format_receipt,
            receipt: receipt.clone(),
            markdown: Some(markdown.clone()),
        })?;

        let value: Value = serde_json::from_str(&fs::read_to_string(receipt)?)?;
        assert_eq!(value["kind"], "native_tooling_status");
        assert!(value["generated_at"].as_str().is_some());
        assert!(value["commit"].as_str().is_some());
        assert_eq!(value["formatter"]["fixture_count"], 1);
        assert_eq!(value["formatter"]["format_receipt_present"], true);
        assert!(value["critic"]["native_rule_count"].as_u64().unwrap_or_default() > 0);
        assert!(
            value["critic"]["native_rules"]
                .as_array()
                .unwrap()
                .iter()
                .any(|rule| { rule.as_str() == Some("native.io.unchecked_open_close") })
        );

        let markdown = fs::read_to_string(markdown)?;
        assert!(markdown.contains("# Native Tooling Status"));
        assert!(!markdown.contains("Generated at:"));
        assert!(!markdown.contains("Commit:"));
        assert!(markdown.contains("native.io.unchecked_open_close"));

        Ok(())
    }
}
