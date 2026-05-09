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
const LITERAL_PRESERVE_CODE: &str = "native.format.literal_preserve_region";

/// Options for `cargo xtask native-tooling status`.
pub struct NativeToolingStatusConfig {
    /// Directory containing native formatter fixtures.
    pub format_fixtures: PathBuf,
    /// Existing native-format fixture receipt, when available.
    pub format_receipt: PathBuf,
    /// Existing native-format corpus receipt, when available.
    pub format_corpus_receipt: PathBuf,
    /// Existing native-format perltidy compatibility receipt, when available.
    pub format_perltidy_compat_receipt: PathBuf,
    /// Existing native critic perlcritic compatibility receipt, when available.
    pub critic_perlcritic_compat_receipt: PathBuf,
    /// Output path for the native-tooling JSON receipt.
    pub receipt: PathBuf,
    /// Optional markdown status output path.
    pub markdown: Option<PathBuf>,
}

/// Options for `cargo xtask native-tooling perlcritic-compat`.
pub struct PerlcriticCompatConfig {
    /// Path to the `.perlcriticrc`-style profile to classify.
    pub profile: PathBuf,
    /// Output JSON receipt path.
    pub receipt: PathBuf,
    /// Output markdown summary path.
    pub summary: PathBuf,
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
    expected_diagnostics_fixture_count: usize,
    literal_preserve_fixture_count: usize,
    format_receipt: String,
    format_receipt_present: bool,
    fixture_passed_count: Option<usize>,
    fixture_failed_count: Option<usize>,
    idempotent_count: Option<usize>,
    parse_preserved_count: Option<usize>,
    diagnostics_count: Option<usize>,
    bailout_count: Option<usize>,
    expected_diagnostics_match_count: Option<usize>,
    format_corpus_receipt: String,
    format_corpus_receipt_present: bool,
    corpus_files_checked: Option<usize>,
    corpus_files_changed: Option<usize>,
    corpus_idempotence_passed_count: Option<usize>,
    corpus_parse_preserved_count: Option<usize>,
    corpus_literal_bailout_count: Option<usize>,
    corpus_unsupported_patterns_count: Option<usize>,
    corpus_diagnostics_count: Option<usize>,
    corpus_passed: Option<bool>,
    format_perltidy_compat_receipt: String,
    format_perltidy_compat_receipt_present: bool,
    perltidy_compat_option_count: Option<usize>,
    perltidy_compat_supported_count: Option<usize>,
    perltidy_compat_approximated_count: Option<usize>,
    perltidy_compat_unsupported_safe_count: Option<usize>,
    perltidy_compat_external_only_count: Option<usize>,
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
    critic_perlcritic_compat_receipt: String,
    critic_perlcritic_compat_receipt_present: bool,
    perlcritic_compat_item_count: Option<usize>,
    perlcritic_compat_native_equivalent_count: Option<usize>,
    perlcritic_compat_native_superset_count: Option<usize>,
    perlcritic_compat_approximated_count: Option<usize>,
    perlcritic_compat_unsupported_safe_count: Option<usize>,
    perlcritic_compat_external_only_count: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PerlcriticCompatReceipt {
    kind: &'static str,
    schema_version: u32,
    generated_at: DateTime<Utc>,
    commit: String,
    profile: String,
    item_count: usize,
    native_equivalent_count: usize,
    native_superset_count: usize,
    approximated_count: usize,
    unsupported_safe_count: usize,
    external_only_count: usize,
    items: Vec<PerlcriticCompatItem>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PerlcriticCompatItem {
    kind: &'static str,
    name: String,
    value: Option<String>,
    classification: &'static str,
    native_rule: Option<&'static str>,
    note: &'static str,
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

/// Classify a perlcritic profile against current native critic compatibility.
pub fn perlcritic_compat(config: PerlcriticCompatConfig) -> Result<()> {
    let raw = fs::read_to_string(&config.profile)
        .wrap_err_with(|| format!("failed to read {}", config.profile.display()))?;
    let items = classify_perlcritic_profile(&raw);
    let receipt = PerlcriticCompatReceipt {
        kind: "native_tooling_perlcritic_compat",
        schema_version: SCHEMA_VERSION,
        generated_at: Utc::now(),
        commit: current_commit(),
        profile: config.profile.display().to_string(),
        item_count: items.len(),
        native_equivalent_count: compat_count(&items, "native_equivalent"),
        native_superset_count: compat_count(&items, "native_superset"),
        approximated_count: compat_count(&items, "approximated"),
        unsupported_safe_count: compat_count(&items, "unsupported_safe"),
        external_only_count: compat_count(&items, "external_only"),
        items,
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
    write_perlcritic_compat_summary(&config.summary, &receipt)?;

    println!(
        "native critic perlcritic compatibility: {} native-equivalent, {} native-superset, {} external-only; receipt: {}",
        receipt.native_equivalent_count,
        receipt.native_superset_count,
        receipt.external_only_count,
        config.receipt.display()
    );
    Ok(())
}

fn build_status_receipt(config: &NativeToolingStatusConfig) -> Result<NativeToolingStatusReceipt> {
    let formatter = formatter_status(
        &config.format_fixtures,
        &config.format_receipt,
        &config.format_corpus_receipt,
        &config.format_perltidy_compat_receipt,
    )?;
    let critic = critic_status(&config.critic_perlcritic_compat_receipt)?;
    Ok(NativeToolingStatusReceipt {
        kind: "native_tooling_status",
        schema_version: SCHEMA_VERSION,
        generated_at: Utc::now(),
        commit: current_commit(),
        formatter,
        critic,
    })
}

fn formatter_status(
    fixtures: &Path,
    format_receipt: &Path,
    format_corpus_receipt: &Path,
    format_perltidy_compat_receipt: &Path,
) -> Result<FormatterStatus> {
    let fixture_paths = WalkDir::new(fixtures)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| {
            let path = entry.path();
            let filename = path.file_name().and_then(|name| name.to_str()).unwrap_or_default();
            path.extension().and_then(|ext| ext.to_str()) == Some("pl")
                && !filename.ends_with(".expected.pl")
        })
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();
    let fixture_count = fixture_paths.len();
    let mut expected_diagnostics_fixture_count = 0;
    let mut literal_preserve_fixture_count = 0;
    for fixture in &fixture_paths {
        let expected_diagnostics = expected_diagnostics_path_for(fixture);
        if !expected_diagnostics.exists() {
            continue;
        }

        expected_diagnostics_fixture_count += 1;
        let diagnostic_codes = read_expected_diagnostic_codes(&expected_diagnostics)?;
        if diagnostic_codes.iter().any(|code| code == LITERAL_PRESERVE_CODE) {
            literal_preserve_fixture_count += 1;
        }
    }

    let receipt = if format_receipt.exists() { Some(read_json(format_receipt)?) } else { None };
    let corpus_receipt =
        if format_corpus_receipt.exists() { Some(read_json(format_corpus_receipt)?) } else { None };
    let perltidy_compat_receipt = if format_perltidy_compat_receipt.exists() {
        Some(read_json(format_perltidy_compat_receipt)?)
    } else {
        None
    };

    Ok(FormatterStatus {
        fixture_root: fixtures.display().to_string(),
        fixture_count,
        expected_diagnostics_fixture_count,
        literal_preserve_fixture_count,
        format_receipt: format_receipt.display().to_string(),
        format_receipt_present: receipt.is_some(),
        fixture_passed_count: optional_usize(&receipt, "passed_count"),
        fixture_failed_count: optional_usize(&receipt, "failed_count"),
        idempotent_count: optional_usize(&receipt, "idempotent_count"),
        parse_preserved_count: optional_usize(&receipt, "parse_preserved_count"),
        diagnostics_count: optional_usize(&receipt, "diagnostics_count"),
        bailout_count: optional_usize(&receipt, "bailout_count"),
        expected_diagnostics_match_count: optional_usize(
            &receipt,
            "expected_diagnostics_match_count",
        ),
        format_corpus_receipt: format_corpus_receipt.display().to_string(),
        format_corpus_receipt_present: corpus_receipt.is_some(),
        corpus_files_checked: optional_usize(&corpus_receipt, "files_checked"),
        corpus_files_changed: optional_usize(&corpus_receipt, "files_changed"),
        corpus_idempotence_passed_count: optional_usize(
            &corpus_receipt,
            "idempotence_passed_count",
        ),
        corpus_parse_preserved_count: optional_usize(&corpus_receipt, "parse_preserved_count"),
        corpus_literal_bailout_count: optional_usize(&corpus_receipt, "literal_bailout_count"),
        corpus_unsupported_patterns_count: optional_usize(
            &corpus_receipt,
            "unsupported_patterns_count",
        ),
        corpus_diagnostics_count: optional_usize(&corpus_receipt, "diagnostics_count"),
        corpus_passed: optional_bool(&corpus_receipt, "passed"),
        format_perltidy_compat_receipt: format_perltidy_compat_receipt.display().to_string(),
        format_perltidy_compat_receipt_present: perltidy_compat_receipt.is_some(),
        perltidy_compat_option_count: optional_usize(&perltidy_compat_receipt, "option_count"),
        perltidy_compat_supported_count: optional_usize(
            &perltidy_compat_receipt,
            "supported_count",
        ),
        perltidy_compat_approximated_count: optional_usize(
            &perltidy_compat_receipt,
            "approximated_count",
        ),
        perltidy_compat_unsupported_safe_count: optional_usize(
            &perltidy_compat_receipt,
            "unsupported_safe_count",
        ),
        perltidy_compat_external_only_count: optional_usize(
            &perltidy_compat_receipt,
            "external_only_count",
        ),
    })
}

fn expected_diagnostics_path_for(fixture: &Path) -> PathBuf {
    fixture.with_file_name(format!(
        "{}.expected-diagnostics.txt",
        fixture.file_stem().and_then(|stem| stem.to_str()).unwrap_or_default()
    ))
}

fn read_expected_diagnostic_codes(path: &Path) -> Result<Vec<String>> {
    let raw =
        fs::read_to_string(path).wrap_err_with(|| format!("failed to read {}", path.display()))?;
    Ok(raw
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(ToOwned::to_owned)
        .collect())
}

fn critic_status(critic_perlcritic_compat_receipt: &Path) -> Result<CriticStatus> {
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
    let perlcritic_compat_receipt = if critic_perlcritic_compat_receipt.exists() {
        Some(read_json(critic_perlcritic_compat_receipt)?)
    } else {
        None
    };

    Ok(CriticStatus {
        native_rule_count: native_rules.len(),
        rules_with_suppression: native_rules.len(),
        rules_with_fixes: fixable.len(),
        fixable_rules: fixable.into_iter().collect(),
        rules_surfaced_in_pull_diagnostics: native_rules.len(),
        rules_surfaced_in_push_diagnostics: native_rules.len(),
        rules_surfaced_in_workspace_diagnostics: native_rules.len(),
        rules_with_violation_bridge: native_rules.len(),
        critic_perlcritic_compat_receipt: critic_perlcritic_compat_receipt.display().to_string(),
        critic_perlcritic_compat_receipt_present: perlcritic_compat_receipt.is_some(),
        perlcritic_compat_item_count: optional_usize(&perlcritic_compat_receipt, "item_count"),
        perlcritic_compat_native_equivalent_count: optional_usize(
            &perlcritic_compat_receipt,
            "native_equivalent_count",
        ),
        perlcritic_compat_native_superset_count: optional_usize(
            &perlcritic_compat_receipt,
            "native_superset_count",
        ),
        perlcritic_compat_approximated_count: optional_usize(
            &perlcritic_compat_receipt,
            "approximated_count",
        ),
        perlcritic_compat_unsupported_safe_count: optional_usize(
            &perlcritic_compat_receipt,
            "unsupported_safe_count",
        ),
        perlcritic_compat_external_only_count: optional_usize(
            &perlcritic_compat_receipt,
            "external_only_count",
        ),
        native_rules,
    })
}

fn fixable_rule_ids() -> BTreeSet<String> {
    [
        "native.common.assignment_in_condition",
        "native.common.deprecated_defined",
        "native.common.undef_comparison",
        "native.common.unreachable_code",
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

fn optional_bool(receipt: &Option<Value>, key: &str) -> Option<bool> {
    receipt.as_ref().and_then(|value| value.get(key)).and_then(Value::as_bool)
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
    let corpus_files_checked =
        optional_metric(formatter.corpus_files_checked, formatter.format_corpus_receipt_present);
    let corpus_files_changed =
        optional_metric(formatter.corpus_files_changed, formatter.format_corpus_receipt_present);
    let corpus_idempotence = optional_pair_metric(
        formatter.corpus_idempotence_passed_count,
        formatter.corpus_files_checked,
        formatter.format_corpus_receipt_present,
    );
    let corpus_parse_preservation = optional_pair_metric(
        formatter.corpus_parse_preserved_count,
        formatter.corpus_files_checked,
        formatter.format_corpus_receipt_present,
    );
    let corpus_literal_bailouts = optional_metric(
        formatter.corpus_literal_bailout_count,
        formatter.format_corpus_receipt_present,
    );
    let corpus_unsupported_diagnostics = optional_metric(
        formatter.corpus_unsupported_patterns_count,
        formatter.format_corpus_receipt_present,
    );
    let corpus_passed =
        formatter.corpus_passed.map(|passed| passed.to_string()).unwrap_or_else(|| {
            if formatter.format_corpus_receipt_present {
                "unknown".to_string()
            } else {
                "UNVERIFIED".to_string()
            }
        });
    let perltidy_compat_options = optional_metric(
        formatter.perltidy_compat_option_count,
        formatter.format_perltidy_compat_receipt_present,
    );
    let perltidy_compat_supported = optional_metric(
        formatter.perltidy_compat_supported_count,
        formatter.format_perltidy_compat_receipt_present,
    );
    let perltidy_compat_approximated = optional_metric(
        formatter.perltidy_compat_approximated_count,
        formatter.format_perltidy_compat_receipt_present,
    );
    let perltidy_compat_unsupported_safe = optional_metric(
        formatter.perltidy_compat_unsupported_safe_count,
        formatter.format_perltidy_compat_receipt_present,
    );
    let perltidy_compat_external_only = optional_metric(
        formatter.perltidy_compat_external_only_count,
        formatter.format_perltidy_compat_receipt_present,
    );
    let perlcritic_compat_items = optional_metric(
        critic.perlcritic_compat_item_count,
        critic.critic_perlcritic_compat_receipt_present,
    );
    let perlcritic_compat_native_equivalent = optional_metric(
        critic.perlcritic_compat_native_equivalent_count,
        critic.critic_perlcritic_compat_receipt_present,
    );
    let perlcritic_compat_native_superset = optional_metric(
        critic.perlcritic_compat_native_superset_count,
        critic.critic_perlcritic_compat_receipt_present,
    );
    let perlcritic_compat_approximated = optional_metric(
        critic.perlcritic_compat_approximated_count,
        critic.critic_perlcritic_compat_receipt_present,
    );
    let perlcritic_compat_unsupported_safe = optional_metric(
        critic.perlcritic_compat_unsupported_safe_count,
        critic.critic_perlcritic_compat_receipt_present,
    );
    let perlcritic_compat_external_only = optional_metric(
        critic.perlcritic_compat_external_only_count,
        critic.critic_perlcritic_compat_receipt_present,
    );
    format!(
        r#"# Native Tooling Status

> Generated by `cargo xtask native-tooling status`.

## Formatter

| Metric | Value |
| --- | ---: |
| Fixture count | {} |
| Expected diagnostic fixtures | {} |
| Literal-preserve bailout fixtures | {} |
| Corpus files checked | {} |
| Corpus files changed | {} |
| Corpus idempotence | {} |
| Corpus parse preservation | {} |
| Corpus literal bailouts | {} |
| Corpus unsupported diagnostics | {} |
| Corpus passed | {} |
| Perltidy compatibility options | {} |
| Perltidy compatibility supported | {} |
| Perltidy compatibility approximated | {} |
| Perltidy compatibility unsupported-safe | {} |
| Perltidy compatibility external-only | {} |

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
| Perlcritic compatibility items | {} |
| Perlcritic compatibility native-equivalent | {} |
| Perlcritic compatibility native-superset | {} |
| Perlcritic compatibility approximated | {} |
| Perlcritic compatibility unsupported-safe | {} |
| Perlcritic compatibility external-only | {} |

Native rules:
{}

Fixable native rules:
{}
"#,
        formatter.fixture_count,
        formatter.expected_diagnostics_fixture_count,
        formatter.literal_preserve_fixture_count,
        corpus_files_checked,
        corpus_files_changed,
        corpus_idempotence,
        corpus_parse_preservation,
        corpus_literal_bailouts,
        corpus_unsupported_diagnostics,
        corpus_passed,
        perltidy_compat_options,
        perltidy_compat_supported,
        perltidy_compat_approximated,
        perltidy_compat_unsupported_safe,
        perltidy_compat_external_only,
        critic.native_rule_count,
        critic.rules_with_suppression,
        critic.rules_with_fixes,
        critic.rules_surfaced_in_pull_diagnostics,
        critic.rules_surfaced_in_push_diagnostics,
        critic.rules_surfaced_in_workspace_diagnostics,
        critic.rules_with_violation_bridge,
        perlcritic_compat_items,
        perlcritic_compat_native_equivalent,
        perlcritic_compat_native_superset,
        perlcritic_compat_approximated,
        perlcritic_compat_unsupported_safe,
        perlcritic_compat_external_only,
        bullet_list(&critic.native_rules),
        bullet_list(&critic.fixable_rules),
    )
}

fn classify_perlcritic_profile(raw: &str) -> Vec<PerlcriticCompatItem> {
    let native_rules = NativeCriticRegistry::recommended()
        .rule_ids()
        .into_iter()
        .map(ToOwned::to_owned)
        .collect::<BTreeSet<_>>();
    let mut items = Vec::new();

    for line in raw.lines() {
        let line = strip_perlcritic_comment(line).trim();
        if line.is_empty() {
            continue;
        }

        if let Some(policy) = parse_perlcritic_policy_section(line) {
            items.push(classify_perlcritic_policy(&policy, &native_rules));
            continue;
        }

        if let Some((name, value)) = line.split_once('=') {
            items.push(classify_perlcritic_setting(name.trim(), Some(value.trim().to_string())));
            continue;
        }

        items.push(compat_item(
            "setting",
            line,
            None,
            "external_only",
            None,
            "unrecognized perlcritic profile line is not applied by native critic",
        ));
    }

    items
}

fn strip_perlcritic_comment(line: &str) -> &str {
    line.split('#').next().unwrap_or_default()
}

fn parse_perlcritic_policy_section(line: &str) -> Option<String> {
    let inner = line.strip_prefix('[')?.strip_suffix(']')?.trim();
    let policy = inner.strip_prefix('-').unwrap_or(inner).trim();
    if policy.is_empty() { None } else { Some(policy.to_string()) }
}

fn classify_perlcritic_policy(
    policy: &str,
    native_rules: &BTreeSet<String>,
) -> PerlcriticCompatItem {
    match perlcritic_policy_native_mapping(policy) {
        Some((native_rule, classification, note)) if native_rules.contains(native_rule) => {
            compat_item("policy", policy, None, classification, Some(native_rule), note)
        }
        Some((native_rule, _, _)) => compat_item(
            "policy",
            policy,
            None,
            "external_only",
            Some(native_rule),
            "mapped native rule is not currently present in the recommended registry",
        ),
        None => compat_item(
            "policy",
            policy,
            None,
            "external_only",
            None,
            "perlcritic policy does not yet have a native rule mapping",
        ),
    }
}

fn perlcritic_policy_native_mapping(
    policy: &str,
) -> Option<(&'static str, &'static str, &'static str)> {
    match policy {
        "TestingAndDebugging::RequireUseStrict" => Some((
            "native.testing.require_use_strict",
            "native_equivalent",
            "native critic emits the same strict-pragmas policy with LSP spans",
        )),
        "TestingAndDebugging::RequireUseWarnings" => Some((
            "native.testing.require_use_warnings",
            "native_equivalent",
            "native critic emits the same warnings-pragmas policy with LSP spans",
        )),
        "InputOutput::ProhibitTwoArgOpen" => Some((
            "native.io.two_arg_open",
            "native_equivalent",
            "native critic detects two-argument open and exposes the existing safe fix",
        )),
        "InputOutput::ProhibitBarewordFileHandles" => Some((
            "native.io.bareword_filehandle",
            "native_equivalent",
            "native critic detects bareword filehandles and exposes the existing safe fix",
        )),
        "InputOutput::RequireCheckedOpen" => Some((
            "native.io.unchecked_open_close",
            "native_superset",
            "native critic covers unchecked open and close result handling",
        )),
        "BuiltinFunctions::ProhibitStringyEval" => Some((
            "native.security.string_eval",
            "native_equivalent",
            "native critic detects parser-confirmed string eval without shelling out",
        )),
        "InputOutput::ProhibitBacktickOperators" => Some((
            "native.security.backtick_exec",
            "native_superset",
            "native critic splits backticks and qx/readpipe into precise native rules",
        )),
        "BuiltinFunctions::ProhibitSystemCalls" => Some((
            "native.security.system_exec",
            "native_equivalent",
            "native critic reports system and exec command execution without an automatic fix",
        )),
        "Variables::ProhibitUnusedVariables" => Some((
            "native.variables.unused_lexical",
            "native_superset",
            "native critic uses semantic scope facts and sigil-aware quick fixes",
        )),
        "Variables::ProhibitReusedNames" => Some((
            "native.variables.duplicate_lexical",
            "approximated",
            "native critic has duplicate and shadowing rules but not a single combined perlcritic policy",
        )),
        "Documentation::RequirePodSections" => Some((
            "native.documentation.require_pod_sections",
            "approximated",
            "native critic checks required NAME/DESCRIPTION sections when a file already contains POD",
        )),
        _ => None,
    }
}

fn classify_perlcritic_setting(name: &str, value: Option<String>) -> PerlcriticCompatItem {
    match name {
        "severity" => compat_item(
            "setting",
            name,
            value,
            "native_equivalent",
            None,
            "maps to native critic minimum severity filtering",
        ),
        "include" | "exclude" => compat_item(
            "setting",
            name,
            value,
            "native_equivalent",
            None,
            "maps to native critic include/exclude rule filtering for native rule IDs",
        ),
        "theme" => compat_item(
            "setting",
            name,
            value,
            "external_only",
            None,
            "native critic does not yet implement perlcritic theme expansion",
        ),
        "profile-strictness" => compat_item(
            "setting",
            name,
            value,
            "unsupported_safe",
            None,
            "perlcritic loader strictness has no runtime effect on native critic rules",
        ),
        _ => compat_item(
            "setting",
            name,
            value,
            "external_only",
            None,
            "perlcritic setting is not yet applied by native critic",
        ),
    }
}

fn compat_item(
    kind: &'static str,
    name: &str,
    value: Option<String>,
    classification: &'static str,
    native_rule: Option<&'static str>,
    note: &'static str,
) -> PerlcriticCompatItem {
    PerlcriticCompatItem { kind, name: name.to_string(), value, classification, native_rule, note }
}

fn compat_count(items: &[PerlcriticCompatItem], classification: &str) -> usize {
    items.iter().filter(|item| item.classification == classification).count()
}

fn write_perlcritic_compat_summary(path: &Path, receipt: &PerlcriticCompatReceipt) -> Result<()> {
    let mut markdown = String::new();
    markdown.push_str("# Native Critic Perlcritic Compatibility\n\n");
    markdown.push_str(&format!("- Profile: `{}`\n", receipt.profile));
    markdown.push_str(&format!("- Items checked: {}\n", receipt.item_count));
    markdown.push_str(&format!("- Native equivalent: {}\n", receipt.native_equivalent_count));
    markdown.push_str(&format!("- Native superset: {}\n", receipt.native_superset_count));
    markdown.push_str(&format!("- Approximated: {}\n", receipt.approximated_count));
    markdown.push_str(&format!("- Unsupported safe: {}\n", receipt.unsupported_safe_count));
    markdown.push_str(&format!("- External-only: {}\n\n", receipt.external_only_count));
    markdown.push_str("| Kind | Name | Value | Classification | Native rule | Note |\n");
    markdown.push_str("| --- | --- | --- | --- | --- | --- |\n");
    for item in &receipt.items {
        markdown.push_str(&format!(
            "| {} | `{}` | {} | {} | {} | {} |\n",
            item.kind,
            item.name,
            item.value.as_deref().unwrap_or(""),
            item.classification,
            item.native_rule.unwrap_or(""),
            item.note
        ));
    }
    fs::write(path, markdown).wrap_err_with(|| format!("failed to write {}", path.display()))
}

fn optional_metric(value: Option<usize>, receipt_present: bool) -> String {
    value.map(|value| value.to_string()).unwrap_or_else(|| {
        if receipt_present { "unknown".to_string() } else { "UNVERIFIED".to_string() }
    })
}

fn optional_pair_metric(
    numerator: Option<usize>,
    denominator: Option<usize>,
    receipt_present: bool,
) -> String {
    match (numerator, denominator) {
        (Some(numerator), Some(denominator)) => format!("{numerator}/{denominator}"),
        _ if receipt_present => "unknown".to_string(),
        _ => "UNVERIFIED".to_string(),
    }
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
        fs::write(
            fixtures.join("simple.expected-diagnostics.txt"),
            "# expected formatter bailout\nnative.format.literal_preserve_region  \n",
        )?;
        let format_receipt = receipts.join("native-format-fixtures.json");
        fs::write(
            &format_receipt,
            r#"{
  "passed_count": 1,
  "failed_count": 0,
  "idempotent_count": 1,
  "parse_preserved_count": 1,
  "diagnostics_count": 1,
  "bailout_count": 1,
  "expected_diagnostics_match_count": 1
}
"#,
        )?;
        let format_corpus_receipt = receipts.join("native-format-corpus.json");
        fs::write(
            &format_corpus_receipt,
            r#"{
  "files_checked": 2,
  "files_changed": 1,
  "idempotence_passed_count": 2,
  "parse_preserved_count": 2,
  "literal_bailout_count": 1,
  "unsupported_patterns_count": 0,
  "diagnostics_count": 1,
  "passed": true
}
"#,
        )?;
        let format_perltidy_compat_receipt = receipts.join("native-format-perltidy-compat.json");
        fs::write(
            &format_perltidy_compat_receipt,
            r#"{
  "option_count": 7,
  "supported_count": 3,
  "approximated_count": 1,
  "unsupported_safe_count": 1,
  "external_only_count": 2
}
"#,
        )?;
        let critic_perlcritic_compat_receipt = receipts.join("perlcritic-compat.json");
        fs::write(
            &critic_perlcritic_compat_receipt,
            r#"{
  "item_count": 12,
  "native_equivalent_count": 5,
  "native_superset_count": 2,
  "approximated_count": 1,
  "unsupported_safe_count": 1,
  "external_only_count": 3
}
"#,
        )?;
        let receipt = receipts.join("native-tooling-status.json");
        let markdown = receipts.join("native-tooling-status.md");
        let missing_receipt = receipts.join("missing-native-format-fixtures.json");
        let missing_status_receipt = receipts.join("native-tooling-status-missing.json");
        let missing_markdown = receipts.join("native-tooling-status-missing.md");

        status(NativeToolingStatusConfig {
            format_fixtures: fixtures.clone(),
            format_receipt: format_receipt.clone(),
            format_corpus_receipt: format_corpus_receipt.clone(),
            format_perltidy_compat_receipt: format_perltidy_compat_receipt.clone(),
            critic_perlcritic_compat_receipt: critic_perlcritic_compat_receipt.clone(),
            receipt: receipt.clone(),
            markdown: Some(markdown.clone()),
        })?;

        let value: Value = serde_json::from_str(&fs::read_to_string(receipt)?)?;
        assert_eq!(value["kind"], "native_tooling_status");
        assert!(value["generated_at"].as_str().is_some());
        assert!(value["commit"].as_str().is_some());
        assert_eq!(value["formatter"]["fixture_count"], 1);
        assert_eq!(value["formatter"]["expected_diagnostics_fixture_count"], 1);
        assert_eq!(value["formatter"]["literal_preserve_fixture_count"], 1);
        assert_eq!(value["formatter"]["format_receipt_present"], true);
        assert_eq!(value["formatter"]["diagnostics_count"], 1);
        assert_eq!(value["formatter"]["bailout_count"], 1);
        assert_eq!(value["formatter"]["expected_diagnostics_match_count"], 1);
        assert_eq!(value["formatter"]["format_corpus_receipt_present"], true);
        assert_eq!(value["formatter"]["corpus_files_checked"], 2);
        assert_eq!(value["formatter"]["corpus_files_changed"], 1);
        assert_eq!(value["formatter"]["corpus_idempotence_passed_count"], 2);
        assert_eq!(value["formatter"]["corpus_parse_preserved_count"], 2);
        assert_eq!(value["formatter"]["corpus_literal_bailout_count"], 1);
        assert_eq!(value["formatter"]["corpus_unsupported_patterns_count"], 0);
        assert_eq!(value["formatter"]["corpus_passed"], true);
        assert_eq!(value["formatter"]["format_perltidy_compat_receipt_present"], true);
        assert_eq!(value["formatter"]["perltidy_compat_option_count"], 7);
        assert_eq!(value["formatter"]["perltidy_compat_supported_count"], 3);
        assert_eq!(value["formatter"]["perltidy_compat_approximated_count"], 1);
        assert_eq!(value["formatter"]["perltidy_compat_unsupported_safe_count"], 1);
        assert_eq!(value["formatter"]["perltidy_compat_external_only_count"], 2);
        assert!(value["critic"]["native_rule_count"].as_u64().unwrap_or_default() > 0);
        assert_eq!(value["critic"]["critic_perlcritic_compat_receipt_present"], true);
        assert_eq!(value["critic"]["perlcritic_compat_item_count"], 12);
        assert_eq!(value["critic"]["perlcritic_compat_native_equivalent_count"], 5);
        assert_eq!(value["critic"]["perlcritic_compat_native_superset_count"], 2);
        assert_eq!(value["critic"]["perlcritic_compat_approximated_count"], 1);
        assert_eq!(value["critic"]["perlcritic_compat_unsupported_safe_count"], 1);
        assert_eq!(value["critic"]["perlcritic_compat_external_only_count"], 3);
        let native_rules = value["critic"]["native_rules"]
            .as_array()
            .ok_or_else(|| eyre!("native_rules should be an array"))?;
        assert!(
            native_rules
                .iter()
                .any(|rule| { rule.as_str() == Some("native.io.unchecked_open_close") })
        );

        let markdown = fs::read_to_string(markdown)?;
        assert!(markdown.contains("# Native Tooling Status"));
        assert!(!markdown.contains("Generated at:"));
        assert!(!markdown.contains("Commit:"));
        assert!(!markdown.contains("Fixture receipt present"));
        assert!(!markdown.contains("Fixture passed count"));
        assert!(!markdown.contains("unknown"));
        assert!(markdown.contains("| Expected diagnostic fixtures | 1 |"));
        assert!(markdown.contains("| Literal-preserve bailout fixtures | 1 |"));
        assert!(markdown.contains("| Corpus files checked | 2 |"));
        assert!(markdown.contains("| Corpus files changed | 1 |"));
        assert!(markdown.contains("| Corpus idempotence | 2/2 |"));
        assert!(markdown.contains("| Corpus parse preservation | 2/2 |"));
        assert!(markdown.contains("| Corpus literal bailouts | 1 |"));
        assert!(markdown.contains("| Corpus unsupported diagnostics | 0 |"));
        assert!(markdown.contains("| Corpus passed | true |"));
        assert!(markdown.contains("| Perltidy compatibility options | 7 |"));
        assert!(markdown.contains("| Perltidy compatibility supported | 3 |"));
        assert!(markdown.contains("| Perltidy compatibility approximated | 1 |"));
        assert!(markdown.contains("| Perltidy compatibility unsupported-safe | 1 |"));
        assert!(markdown.contains("| Perltidy compatibility external-only | 2 |"));
        assert!(markdown.contains("| Perlcritic compatibility items | 12 |"));
        assert!(markdown.contains("| Perlcritic compatibility native-equivalent | 5 |"));
        assert!(markdown.contains("| Perlcritic compatibility native-superset | 2 |"));
        assert!(markdown.contains("| Perlcritic compatibility approximated | 1 |"));
        assert!(markdown.contains("| Perlcritic compatibility unsupported-safe | 1 |"));
        assert!(markdown.contains("| Perlcritic compatibility external-only | 3 |"));
        assert!(markdown.contains("native.io.unchecked_open_close"));

        status(NativeToolingStatusConfig {
            format_fixtures: fixtures,
            format_receipt: missing_receipt,
            format_corpus_receipt,
            format_perltidy_compat_receipt,
            critic_perlcritic_compat_receipt,
            receipt: missing_status_receipt.clone(),
            markdown: Some(missing_markdown.clone()),
        })?;

        let missing_value: Value =
            serde_json::from_str(&fs::read_to_string(missing_status_receipt)?)?;
        assert_eq!(missing_value["formatter"]["format_receipt_present"], false);
        assert_eq!(fs::read_to_string(missing_markdown)?, markdown);

        Ok(())
    }

    #[test]
    fn native_tooling_perlcritic_compat_classifies_common_profile() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let profile = temp.path().join(".perlcriticrc");
        let receipts = temp.path().join("receipts");
        fs::write(
            &profile,
            r#"# common policy profile
severity = 3
include = TestingAndDebugging::RequireUseStrict
exclude = Documentation::RequirePodSections
profile-strictness = quiet
[TestingAndDebugging::RequireUseStrict]
[-InputOutput::ProhibitTwoArgOpen]
[InputOutput::RequireCheckedOpen]
[Variables::ProhibitUnusedVariables]
[Variables::ProhibitReusedNames]
[Documentation::RequirePodSections]
theme = core
color = 1
"#,
        )?;

        perlcritic_compat(PerlcriticCompatConfig {
            profile,
            receipt: receipts.join("perlcritic-compat.json"),
            summary: receipts.join("perlcritic-compat.md"),
        })?;

        let receipt: Value =
            serde_json::from_str(&fs::read_to_string(receipts.join("perlcritic-compat.json"))?)?;
        assert_eq!(receipt["kind"], "native_tooling_perlcritic_compat");
        assert_eq!(receipt["item_count"], 12);
        assert_eq!(receipt["native_equivalent_count"], 5);
        assert_eq!(receipt["native_superset_count"], 2);
        assert_eq!(receipt["approximated_count"], 2);
        assert_eq!(receipt["unsupported_safe_count"], 1);
        assert_eq!(receipt["external_only_count"], 2);
        assert_eq!(receipt["items"][0]["classification"], "native_equivalent");
        assert_eq!(receipt["items"][2]["name"], "exclude");
        assert_eq!(receipt["items"][5]["native_rule"], "native.io.two_arg_open");
        assert_eq!(receipt["items"][6]["native_rule"], "native.io.unchecked_open_close");
        assert_eq!(receipt["items"][8]["classification"], "approximated");
        assert_eq!(receipt["items"][9]["classification"], "approximated");
        assert_eq!(receipt["items"][11]["name"], "color");

        let summary = fs::read_to_string(receipts.join("perlcritic-compat.md"))?;
        assert!(summary.contains("# Native Critic Perlcritic Compatibility"));
        assert!(summary.contains(
            "| setting | `exclude` | Documentation::RequirePodSections | native_equivalent |"
        ));
        assert!(summary.contains("| policy | `InputOutput::ProhibitTwoArgOpen` |  | native_equivalent | native.io.two_arg_open |"));
        assert!(summary.contains("| policy | `InputOutput::RequireCheckedOpen` |  | native_superset | native.io.unchecked_open_close |"));
        assert!(summary.contains("| policy | `Documentation::RequirePodSections` |  | approximated | native.documentation.require_pod_sections |"));
        assert!(summary.contains("| setting | `profile-strictness` | quiet | unsupported_safe |"));
        assert!(summary.contains("| setting | `color` | 1 | external_only |"));

        Ok(())
    }
}
