//! Native formatter and critic replacement status receipts.

use chrono::{DateTime, Utc};
use color_eyre::eyre::{Context, Result, eyre};
use perl_lsp_rs_core::config::{CriticEngine, FormatterMode, ServerConfig};
use perl_lsp_rs_core::tooling::native_compat::{
    PerlcriticCompatItem, PerlcriticCompatReport, PerlcriticNativeConfigSuggestion,
    classify_perlcritic_profile, render_perlcritic_compat_markdown,
};
use perl_lsp_rs_core::tooling::perl_critic::NativeCriticRegistry;
use perl_lsp_rs_core::tooling::perltidy::FormatConfig;
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
    /// Existing native-format config receipt, when available.
    pub format_config_receipt: PathBuf,
    /// Existing native critic perlcritic compatibility receipt, when available.
    pub critic_perlcritic_compat_receipt: PathBuf,
    /// Existing native critic check receipt, when available.
    pub critic_check_receipt: PathBuf,
    /// Existing native critic false-positive fixture receipt, when available.
    pub critic_false_positive_receipt: PathBuf,
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

/// Options for `cargo xtask native-tooling check-defaults`.
pub struct NativeToolingDefaultsConfig {
    /// Repository root used for source-policy checks.
    pub root: PathBuf,
}

/// Options for `cargo xtask native-tooling readiness`.
pub struct NativeToolingReadinessConfig {
    /// Native-tooling status receipt to evaluate.
    pub status_receipt: PathBuf,
    /// Output path for the native-tooling readiness JSON receipt.
    pub receipt: PathBuf,
    /// Optional markdown readiness output path.
    pub markdown: Option<PathBuf>,
}

#[derive(Debug)]
struct DefaultCheck {
    name: &'static str,
    passed: bool,
    detail: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct NativeToolingStatusReceipt {
    kind: String,
    schema_version: u32,
    generated_at: DateTime<Utc>,
    commit: String,
    formatter: FormatterStatus,
    critic: CriticStatus,
}

#[derive(Debug, Serialize, Deserialize)]
struct NativeToolingReadinessReceipt {
    kind: String,
    schema_version: u32,
    generated_at: DateTime<Utc>,
    commit: String,
    status_receipt: String,
    verdict: &'static str,
    ready_count: usize,
    blocker_count: usize,
    warning_count: usize,
    unverified_count: usize,
    criteria: Vec<ReadinessCriterion>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ReadinessCriterion {
    area: &'static str,
    name: &'static str,
    status: &'static str,
    required_for_default: bool,
    evidence: String,
    next: &'static str,
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
    corpus_unsupported_parse_clean_count: Option<usize>,
    corpus_parse_error_count: Option<usize>,
    corpus_diagnostics_count: Option<usize>,
    corpus_passed: Option<bool>,
    format_perltidy_compat_receipt: String,
    format_perltidy_compat_receipt_present: bool,
    perltidy_compat_option_count: Option<usize>,
    perltidy_compat_supported_count: Option<usize>,
    perltidy_compat_approximated_count: Option<usize>,
    perltidy_compat_unsupported_safe_count: Option<usize>,
    perltidy_compat_external_only_count: Option<usize>,
    format_config_receipt: String,
    format_config_receipt_present: bool,
    format_config_source: Option<String>,
    format_engine_selected: Option<String>,
    format_external_adapter_requested: Option<bool>,
    format_line_width: Option<usize>,
    format_indent_width: Option<usize>,
    format_use_tabs: Option<bool>,
    format_brace_placement: Option<String>,
    format_else_placement: Option<String>,
    format_keyword_spacing: Option<String>,
    format_trailing_comma: Option<String>,
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
    critic_check_receipt: String,
    critic_check_receipt_present: bool,
    critic_check_profile: Option<String>,
    critic_check_files_checked: Option<usize>,
    critic_check_files_with_parse_errors: Option<usize>,
    critic_check_rules_run: Option<usize>,
    critic_check_findings_count: Option<usize>,
    critic_check_suppressed_findings_count: Option<usize>,
    critic_check_fixable_findings_count: Option<usize>,
    critic_false_positive_receipt: String,
    critic_false_positive_receipt_present: bool,
    critic_false_positive_files_checked: Option<usize>,
    critic_false_positive_files_with_parse_errors: Option<usize>,
    critic_false_positive_rules_run: Option<usize>,
    critic_false_positive_findings_count: Option<usize>,
    critic_false_positive_suppressed_findings_count: Option<usize>,
    critic_false_positive_fixable_findings_count: Option<usize>,
    critic_perlcritic_compat_receipt: String,
    critic_perlcritic_compat_receipt_present: bool,
    perlcritic_compat_item_count: Option<usize>,
    perlcritic_compat_native_equivalent_count: Option<usize>,
    perlcritic_compat_native_superset_count: Option<usize>,
    perlcritic_compat_approximated_count: Option<usize>,
    perlcritic_compat_unsupported_safe_count: Option<usize>,
    perlcritic_compat_external_only_count: Option<usize>,
}

#[derive(Debug, Serialize)]
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
    suggested_config: PerlcriticNativeConfigSuggestion,
    items: Vec<PerlcriticCompatItem>,
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
    let report = classify_perlcritic_profile(&raw);
    let receipt = PerlcriticCompatReceipt {
        kind: "native_tooling_perlcritic_compat",
        schema_version: SCHEMA_VERSION,
        generated_at: Utc::now(),
        commit: current_commit(),
        profile: config.profile.display().to_string(),
        item_count: report.item_count,
        native_equivalent_count: report.native_equivalent_count,
        native_superset_count: report.native_superset_count,
        approximated_count: report.approximated_count,
        unsupported_safe_count: report.unsupported_safe_count,
        external_only_count: report.external_only_count,
        suggested_config: report.suggested_config,
        items: report.items,
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

/// Verify native tooling defaults and native paths do not silently shell out.
pub fn check_defaults(config: NativeToolingDefaultsConfig) -> Result<()> {
    let checks = native_tooling_default_checks(&config.root)?;
    for check in &checks {
        let status = if check.passed { "pass" } else { "fail" };
        println!("native-tooling defaults {status}: {} - {}", check.name, check.detail);
    }

    let failures: Vec<_> = checks.iter().filter(|check| !check.passed).collect();
    if failures.is_empty() {
        println!("native-tooling defaults: all checks passed");
        return Ok(());
    }

    Err(eyre!("native-tooling default guard failed: {} check(s) failed", failures.len()))
}

/// Render a native tooling default-cutover readiness report from existing receipts.
pub fn readiness(config: NativeToolingReadinessConfig) -> Result<()> {
    let status: NativeToolingStatusReceipt = serde_json::from_str(
        &fs::read_to_string(&config.status_receipt)
            .wrap_err_with(|| format!("failed to read {}", config.status_receipt.display()))?,
    )
    .wrap_err_with(|| format!("failed to parse {}", config.status_receipt.display()))?;
    let receipt = build_readiness_receipt(&config.status_receipt, &status);

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
        fs::write(markdown, render_readiness_markdown(&receipt))
            .wrap_err_with(|| format!("failed to write {}", markdown.display()))?;
    }

    println!(
        "native tooling readiness: {} ({} ready, {} blocker, {} warning, {} unverified); receipt: {}",
        receipt.verdict,
        receipt.ready_count,
        receipt.blocker_count,
        receipt.warning_count,
        receipt.unverified_count,
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
        &config.format_config_receipt,
    )?;
    let critic = critic_status(
        &config.critic_perlcritic_compat_receipt,
        &config.critic_check_receipt,
        &config.critic_false_positive_receipt,
    )?;
    Ok(NativeToolingStatusReceipt {
        kind: "native_tooling_status".to_string(),
        schema_version: SCHEMA_VERSION,
        generated_at: Utc::now(),
        commit: current_commit(),
        formatter,
        critic,
    })
}

fn build_readiness_receipt(
    status_receipt: &Path,
    status: &NativeToolingStatusReceipt,
) -> NativeToolingReadinessReceipt {
    let mut criteria = Vec::new();
    let formatter = &status.formatter;
    let critic = &status.critic;
    let server_defaults = ServerConfig::default();

    criteria.push(readiness_criterion(
        "formatter",
        "native formatter default",
        formatter.format_engine_selected.as_deref() == Some("native")
            && formatter.format_external_adapter_requested == Some(false),
        formatter.format_config_receipt_present,
        true,
        format!(
            "engine={}, external_adapter_requested={}",
            optional_text_metric(
                formatter.format_engine_selected.as_deref(),
                formatter.format_config_receipt_present,
            ),
            optional_bool_metric(
                formatter.format_external_adapter_requested,
                formatter.format_config_receipt_present,
            )
        ),
        "regenerate `cargo xtask native-format config` and keep external adapter opt-in",
    ));
    criteria.push(readiness_criterion(
        "formatter",
        "fixture suite passes",
        formatter.fixture_failed_count == Some(0)
            && formatter.fixture_passed_count == Some(formatter.fixture_count),
        formatter.format_receipt_present,
        true,
        format!(
            "passed={}/{} failed={}",
            optional_metric(formatter.fixture_passed_count, formatter.format_receipt_present),
            formatter.fixture_count,
            optional_metric(formatter.fixture_failed_count, formatter.format_receipt_present)
        ),
        "run `cargo xtask native-format check` and fix any fixture failures",
    ));
    criteria.push(readiness_criterion(
        "formatter",
        "corpus idempotent and parse-preserving",
        formatter.corpus_passed == Some(true)
            && formatter.corpus_idempotence_passed_count == formatter.corpus_files_checked
            && formatter.corpus_parse_preserved_count == formatter.corpus_files_checked,
        formatter.format_corpus_receipt_present,
        true,
        format!(
            "files={} idempotence={} parse_preservation={} passed={}",
            optional_metric(formatter.corpus_files_checked, formatter.format_corpus_receipt_present),
            optional_pair_metric(
                formatter.corpus_idempotence_passed_count,
                formatter.corpus_files_checked,
                formatter.format_corpus_receipt_present,
            ),
            optional_pair_metric(
                formatter.corpus_parse_preserved_count,
                formatter.corpus_files_checked,
                formatter.format_corpus_receipt_present,
            ),
            optional_bool_metric(formatter.corpus_passed, formatter.format_corpus_receipt_present)
        ),
        "run `cargo xtask native-format corpus` and investigate non-idempotent or non-preserved files",
    ));
    criteria.push(readiness_criterion(
        "formatter",
        "corpus parse-clean unsupported formatter diagnostics are cleared",
        formatter.corpus_unsupported_parse_clean_count == Some(0),
        formatter.format_corpus_receipt_present,
        false,
        format!(
            "unsupported_diagnostics={} parse_clean_unsupported={} parse_error_diagnostics={} literal_bailouts={} diagnostics={}",
            optional_metric(
                formatter.corpus_unsupported_patterns_count,
                formatter.format_corpus_receipt_present,
            ),
            optional_metric(
                formatter.corpus_unsupported_parse_clean_count,
                formatter.format_corpus_receipt_present,
            ),
            optional_metric(
                formatter.corpus_parse_error_count,
                formatter.format_corpus_receipt_present,
            ),
            optional_metric(
                formatter.corpus_literal_bailout_count,
                formatter.format_corpus_receipt_present,
            ),
            optional_metric(
                formatter.corpus_diagnostics_count,
                formatter.format_corpus_receipt_present
            )
        ),
        "clear parse-clean unsupported diagnostics before claiming broad format-on-save coverage; parse-error diagnostics remain recovery-fixture signal",
    ));
    criteria.push(readiness_criterion(
        "formatter",
        "dangerous surfaces visible",
        formatter.expected_diagnostics_fixture_count > 0
            && formatter.literal_preserve_fixture_count > 0
            && formatter.bailout_count.unwrap_or_default() > 0,
        formatter.format_receipt_present,
        true,
        format!(
            "expected_diagnostic_fixtures={} literal_preserve_fixtures={} bailout_count={}",
            formatter.expected_diagnostics_fixture_count,
            formatter.literal_preserve_fixture_count,
            optional_metric(formatter.bailout_count, formatter.format_receipt_present)
        ),
        "expand literal/comment preservation fixtures before treating format-on-save as broad coverage",
    ));
    criteria.push(readiness_criterion(
        "formatter",
        "perltidy compatibility has no external-only gaps",
        formatter.perltidy_compat_external_only_count == Some(0),
        formatter.format_perltidy_compat_receipt_present,
        true,
        format!(
            "options={} supported={} approximated={} external_only={}",
            optional_metric(
                formatter.perltidy_compat_option_count,
                formatter.format_perltidy_compat_receipt_present,
            ),
            optional_metric(
                formatter.perltidy_compat_supported_count,
                formatter.format_perltidy_compat_receipt_present,
            ),
            optional_metric(
                formatter.perltidy_compat_approximated_count,
                formatter.format_perltidy_compat_receipt_present,
            ),
            optional_metric(
                formatter.perltidy_compat_external_only_count,
                formatter.format_perltidy_compat_receipt_present,
            )
        ),
        "map, approximate, or document remaining external-only perltidy options",
    ));
    criteria.push(readiness_criterion(
        "critic",
        "native critic default",
        server_defaults.perlcritic_enabled
            && server_defaults.critic_engine == CriticEngine::Native
            && server_defaults.native_critic_profile == "recommended",
        true,
        true,
        format!(
            "default perlcritic_enabled={} critic_engine={:?} profile={}",
            server_defaults.perlcritic_enabled,
            server_defaults.critic_engine,
            server_defaults.native_critic_profile
        ),
        "keep default critic path on the low-noise native recommended profile",
    ));
    criteria.push(readiness_criterion(
        "critic",
        "native rule surface has LSP and bridge coverage",
        critic.native_rule_count > 0
            && critic.rules_surfaced_in_pull_diagnostics == critic.native_rule_count
            && critic.rules_surfaced_in_push_diagnostics == critic.native_rule_count
            && critic.rules_surfaced_in_workspace_diagnostics == critic.native_rule_count
            && critic.rules_with_violation_bridge == critic.native_rule_count,
        true,
        true,
        format!(
            "rules={} pull={} push={} workspace={} bridge={}",
            critic.native_rule_count,
            critic.rules_surfaced_in_pull_diagnostics,
            critic.rules_surfaced_in_push_diagnostics,
            critic.rules_surfaced_in_workspace_diagnostics,
            critic.rules_with_violation_bridge
        ),
        "route every recommended rule through pull, push, workspace diagnostics, and violation bridge",
    ));
    criteria.push(readiness_criterion(
        "critic",
        "native critic check receipt is parse-clean",
        critic.critic_check_files_checked.unwrap_or_default() > 0
            && critic.critic_check_files_with_parse_errors == Some(0),
        critic.critic_check_receipt_present,
        true,
        format!(
            "profile={} files={} parse_errors={} findings={} fixable={}",
            optional_text_metric(
                critic.critic_check_profile.as_deref(),
                critic.critic_check_receipt_present,
            ),
            optional_metric(critic.critic_check_files_checked, critic.critic_check_receipt_present),
            optional_metric(
                critic.critic_check_files_with_parse_errors,
                critic.critic_check_receipt_present,
            ),
            optional_metric(critic.critic_check_findings_count, critic.critic_check_receipt_present),
            optional_metric(
                critic.critic_check_fixable_findings_count,
                critic.critic_check_receipt_present,
            )
        ),
        "run `cargo xtask native-critic check` and fix parse errors before relying on critic receipts",
    ));
    criteria.push(readiness_criterion(
        "critic",
        "native critic false-positive fixtures are clean",
        critic.critic_false_positive_files_checked.unwrap_or_default() > 0
            && critic.critic_false_positive_files_with_parse_errors == Some(0)
            && critic.critic_false_positive_findings_count == Some(0)
            && critic.critic_false_positive_suppressed_findings_count == Some(0),
        critic.critic_false_positive_receipt_present,
        true,
        format!(
            "files={} parse_errors={} findings={} suppressed={}",
            optional_metric(
                critic.critic_false_positive_files_checked,
                critic.critic_false_positive_receipt_present,
            ),
            optional_metric(
                critic.critic_false_positive_files_with_parse_errors,
                critic.critic_false_positive_receipt_present,
            ),
            optional_metric(
                critic.critic_false_positive_findings_count,
                critic.critic_false_positive_receipt_present,
            ),
            optional_metric(
                critic.critic_false_positive_suppressed_findings_count,
                critic.critic_false_positive_receipt_present,
            )
        ),
        "run the native critic false-positive fixture receipt and fix any emitted finding",
    ));
    criteria.push(readiness_criterion(
        "critic",
        "native critic has fixes and suppression coverage",
        critic.rules_with_suppression == critic.native_rule_count && critic.rules_with_fixes > 0,
        true,
        true,
        format!(
            "rules={} suppression={} fixes={}",
            critic.native_rule_count, critic.rules_with_suppression, critic.rules_with_fixes
        ),
        "add suppression/config/fix tests for new rules before enabling them in recommended profile",
    ));
    criteria.push(readiness_criterion(
        "critic",
        "perlcritic compatibility has no external-only gaps",
        critic.perlcritic_compat_external_only_count == Some(0),
        critic.critic_perlcritic_compat_receipt_present,
        true,
        format!(
            "items={} equivalent={} superset={} approximated={} external_only={}",
            optional_metric(
                critic.perlcritic_compat_item_count,
                critic.critic_perlcritic_compat_receipt_present,
            ),
            optional_metric(
                critic.perlcritic_compat_native_equivalent_count,
                critic.critic_perlcritic_compat_receipt_present,
            ),
            optional_metric(
                critic.perlcritic_compat_native_superset_count,
                critic.critic_perlcritic_compat_receipt_present,
            ),
            optional_metric(
                critic.perlcritic_compat_approximated_count,
                critic.critic_perlcritic_compat_receipt_present,
            ),
            optional_metric(
                critic.perlcritic_compat_external_only_count,
                critic.critic_perlcritic_compat_receipt_present,
            )
        ),
        "map high-value perlcritic policies or keep external mode documented for remaining policies",
    ));

    let ready_count = criteria.iter().filter(|criterion| criterion.status == "ready").count();
    let blocker_count = criteria.iter().filter(|criterion| criterion.status == "blocked").count();
    let warning_count = criteria.iter().filter(|criterion| criterion.status == "warning").count();
    let unverified_count =
        criteria.iter().filter(|criterion| criterion.status == "unverified").count();
    let verdict = if blocker_count == 0 && unverified_count == 0 {
        if warning_count == 0 { "ready" } else { "provisional" }
    } else {
        "not_ready"
    };

    NativeToolingReadinessReceipt {
        kind: "native_tooling_readiness".to_string(),
        schema_version: SCHEMA_VERSION,
        generated_at: Utc::now(),
        commit: current_commit(),
        status_receipt: status_receipt.display().to_string(),
        verdict,
        ready_count,
        blocker_count,
        warning_count,
        unverified_count,
        criteria,
    }
}

fn readiness_criterion(
    area: &'static str,
    name: &'static str,
    passed: bool,
    verified: bool,
    required_for_default: bool,
    evidence: String,
    next: &'static str,
) -> ReadinessCriterion {
    let status = match (verified, passed, required_for_default) {
        (false, _, _) => "unverified",
        (true, true, _) => "ready",
        (true, false, true) => "blocked",
        (true, false, false) => "warning",
    };
    ReadinessCriterion { area, name, status, required_for_default, evidence, next }
}

fn native_tooling_default_checks(root: &Path) -> Result<Vec<DefaultCheck>> {
    let server_defaults = ServerConfig::default();
    let format_defaults = FormatConfig::default();
    let formatting_provider_source =
        read_source(root, "crates/perl-lsp-rs-core/src/providers/formatting/formatting.rs")?;
    let diagnostics_source = read_source(root, "crates/perl-lsp-rs/src/runtime/diagnostics.rs")?;
    let configuration_docs = read_source(root, "docs/reference/CONFIGURATION.md")?;

    let native_critic_skip_count = diagnostics_source
        .matches("critic_engine == perl_lsp_rs_core::config::CriticEngine::Native")
        .count();

    Ok(vec![
        DefaultCheck {
            name: "formatter_server_default_native",
            passed: server_defaults.formatting_engine == FormatterMode::Native,
            detail: format!(
                "ServerConfig default formatter engine is {:?}",
                server_defaults.formatting_engine
            ),
        },
        DefaultCheck {
            name: "formatter_core_default_native",
            passed: format_defaults.mode == FormatterMode::Native,
            detail: format!("FormatConfig default formatter mode is {:?}", format_defaults.mode),
        },
        DefaultCheck {
            name: "critic_default_no_shell_out",
            passed: server_defaults.perlcritic_enabled
                && server_defaults.critic_engine == CriticEngine::Native
                && server_defaults.native_critic_profile == "recommended",
            detail: format!(
                "ServerConfig default critic enabled={} engine={:?} profile={}",
                server_defaults.perlcritic_enabled,
                server_defaults.critic_engine,
                server_defaults.native_critic_profile
            ),
        },
        DefaultCheck {
            name: "native_formatter_branch_uses_native_provider",
            passed: formatting_provider_source
                .contains("FormatterMode::Native | FormatterMode::Compat")
                && formatting_provider_source.contains("Ok(native_format_document")
                && formatting_provider_source.contains("Ok(native_format_range"),
            detail: "native/compat formatter branches render through native_format_*".to_string(),
        },
        DefaultCheck {
            name: "external_formatter_requires_external_legacy_mode",
            passed: formatting_provider_source
                .contains("FormatterMode::ExternalLegacy => self.format_document_with_perltidy")
                && formatting_provider_source.contains("self.format_range_with_perltidy"),
            detail: "perltidy formatter calls are isolated behind ExternalLegacy".to_string(),
        },
        DefaultCheck {
            name: "native_critic_skips_external_collectors",
            passed: native_critic_skip_count >= 2,
            detail: format!(
                "found {native_critic_skip_count} native critic skip guards in diagnostics runtime"
            ),
        },
        DefaultCheck {
            name: "configuration_docs_mark_native_format_default",
            passed: configuration_docs
                .contains("| `[formatting]` | `engine` | string | `\"native\"` |")
                && configuration_docs
                    .contains("Use `\"external-perltidy\"` for legacy shell-out compatibility"),
            detail:
                "configuration docs describe native formatter default and explicit legacy adapter"
                    .to_string(),
        },
        DefaultCheck {
            name: "configuration_docs_mark_native_critic_default",
            passed: configuration_docs
                .contains("| `[critic]` | `engine` | string | `\"native\"` |")
                && configuration_docs.contains(
                    "Use `\"legacy\"` or `\"external\"` for Perl::Critic shell-out compatibility",
                ),
            detail: "configuration docs describe native critic default and explicit legacy adapter"
                .to_string(),
        },
    ])
}

fn read_source(root: &Path, relative: &str) -> Result<String> {
    let path = root.join(relative);
    fs::read_to_string(&path).wrap_err_with(|| format!("failed to read {}", path.display()))
}

fn formatter_status(
    fixtures: &Path,
    format_receipt: &Path,
    format_corpus_receipt: &Path,
    format_perltidy_compat_receipt: &Path,
    format_config_receipt: &Path,
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
    let config_receipt =
        if format_config_receipt.exists() { Some(read_json(format_config_receipt)?) } else { None };

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
        corpus_unsupported_parse_clean_count: optional_usize(
            &corpus_receipt,
            "unsupported_parse_clean_count",
        ),
        corpus_parse_error_count: optional_usize(&corpus_receipt, "parse_error_count"),
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
        format_config_receipt: format_config_receipt.display().to_string(),
        format_config_receipt_present: config_receipt.is_some(),
        format_config_source: optional_string(&config_receipt, "config_source"),
        format_engine_selected: optional_string(&config_receipt, "engine_selected"),
        format_external_adapter_requested: optional_bool(
            &config_receipt,
            "external_adapter_requested",
        ),
        format_line_width: optional_usize(&config_receipt, "line_width"),
        format_indent_width: optional_usize(&config_receipt, "indent_width"),
        format_use_tabs: optional_bool(&config_receipt, "use_tabs"),
        format_brace_placement: optional_string(&config_receipt, "brace_placement"),
        format_else_placement: optional_string(&config_receipt, "else_placement"),
        format_keyword_spacing: optional_string(&config_receipt, "keyword_spacing"),
        format_trailing_comma: optional_string(&config_receipt, "trailing_comma"),
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

fn critic_status(
    critic_perlcritic_compat_receipt: &Path,
    critic_check_receipt: &Path,
    critic_false_positive_receipt: &Path,
) -> Result<CriticStatus> {
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
    let check_receipt =
        if critic_check_receipt.exists() { Some(read_json(critic_check_receipt)?) } else { None };
    let false_positive_receipt = if critic_false_positive_receipt.exists() {
        Some(read_json(critic_false_positive_receipt)?)
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
        critic_check_receipt: critic_check_receipt.display().to_string(),
        critic_check_receipt_present: check_receipt.is_some(),
        critic_check_profile: optional_string(&check_receipt, "profile"),
        critic_check_files_checked: optional_usize(&check_receipt, "files_checked"),
        critic_check_files_with_parse_errors: optional_usize(
            &check_receipt,
            "files_with_parse_errors",
        ),
        critic_check_rules_run: optional_usize(&check_receipt, "rules_run"),
        critic_check_findings_count: optional_usize(&check_receipt, "findings_count"),
        critic_check_suppressed_findings_count: optional_usize(
            &check_receipt,
            "suppressed_findings_count",
        ),
        critic_check_fixable_findings_count: optional_usize(
            &check_receipt,
            "fixable_findings_count",
        ),
        critic_false_positive_receipt: critic_false_positive_receipt.display().to_string(),
        critic_false_positive_receipt_present: false_positive_receipt.is_some(),
        critic_false_positive_files_checked: optional_usize(
            &false_positive_receipt,
            "files_checked",
        ),
        critic_false_positive_files_with_parse_errors: optional_usize(
            &false_positive_receipt,
            "files_with_parse_errors",
        ),
        critic_false_positive_rules_run: optional_usize(&false_positive_receipt, "rules_run"),
        critic_false_positive_findings_count: optional_usize(
            &false_positive_receipt,
            "findings_count",
        ),
        critic_false_positive_suppressed_findings_count: optional_usize(
            &false_positive_receipt,
            "suppressed_findings_count",
        ),
        critic_false_positive_fixable_findings_count: optional_usize(
            &false_positive_receipt,
            "fixable_findings_count",
        ),
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

fn optional_string(receipt: &Option<Value>, key: &str) -> Option<String> {
    receipt.as_ref().and_then(|value| value.get(key)).and_then(Value::as_str).map(ToOwned::to_owned)
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
    let corpus_unsupported_parse_clean = optional_metric(
        formatter.corpus_unsupported_parse_clean_count,
        formatter.format_corpus_receipt_present,
    );
    let corpus_parse_errors = optional_metric(
        formatter.corpus_parse_error_count,
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
    let format_config_source = optional_text_metric(
        formatter.format_config_source.as_deref(),
        formatter.format_config_receipt_present,
    );
    let format_engine_selected = optional_text_metric(
        formatter.format_engine_selected.as_deref(),
        formatter.format_config_receipt_present,
    );
    let format_external_adapter = optional_bool_metric(
        formatter.format_external_adapter_requested,
        formatter.format_config_receipt_present,
    );
    let format_line_width =
        optional_metric(formatter.format_line_width, formatter.format_config_receipt_present);
    let format_indent_width =
        optional_metric(formatter.format_indent_width, formatter.format_config_receipt_present);
    let format_use_tabs =
        optional_bool_metric(formatter.format_use_tabs, formatter.format_config_receipt_present);
    let format_brace_placement = optional_text_metric(
        formatter.format_brace_placement.as_deref(),
        formatter.format_config_receipt_present,
    );
    let format_else_placement = optional_text_metric(
        formatter.format_else_placement.as_deref(),
        formatter.format_config_receipt_present,
    );
    let format_keyword_spacing = optional_text_metric(
        formatter.format_keyword_spacing.as_deref(),
        formatter.format_config_receipt_present,
    );
    let format_trailing_comma = optional_text_metric(
        formatter.format_trailing_comma.as_deref(),
        formatter.format_config_receipt_present,
    );
    let critic_check_files_checked =
        optional_metric(critic.critic_check_files_checked, critic.critic_check_receipt_present);
    let critic_check_profile = optional_text_metric(
        critic.critic_check_profile.as_deref(),
        critic.critic_check_receipt_present,
    );
    let critic_check_parse_errors = optional_metric(
        critic.critic_check_files_with_parse_errors,
        critic.critic_check_receipt_present,
    );
    let critic_check_rules_run =
        optional_metric(critic.critic_check_rules_run, critic.critic_check_receipt_present);
    let critic_check_findings =
        optional_metric(critic.critic_check_findings_count, critic.critic_check_receipt_present);
    let critic_check_suppressed = optional_metric(
        critic.critic_check_suppressed_findings_count,
        critic.critic_check_receipt_present,
    );
    let critic_check_fixable = optional_metric(
        critic.critic_check_fixable_findings_count,
        critic.critic_check_receipt_present,
    );
    let critic_false_positive_files_checked = optional_metric(
        critic.critic_false_positive_files_checked,
        critic.critic_false_positive_receipt_present,
    );
    let critic_false_positive_parse_errors = optional_metric(
        critic.critic_false_positive_files_with_parse_errors,
        critic.critic_false_positive_receipt_present,
    );
    let critic_false_positive_rules_run = optional_metric(
        critic.critic_false_positive_rules_run,
        critic.critic_false_positive_receipt_present,
    );
    let critic_false_positive_findings = optional_metric(
        critic.critic_false_positive_findings_count,
        critic.critic_false_positive_receipt_present,
    );
    let critic_false_positive_suppressed = optional_metric(
        critic.critic_false_positive_suppressed_findings_count,
        critic.critic_false_positive_receipt_present,
    );
    let critic_false_positive_fixable = optional_metric(
        critic.critic_false_positive_fixable_findings_count,
        critic.critic_false_positive_receipt_present,
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
| Corpus unsupported parse-clean diagnostics | {} |
| Corpus parse-error diagnostics | {} |
| Corpus passed | {} |
| Perltidy compatibility options | {} |
| Perltidy compatibility supported | {} |
| Perltidy compatibility approximated | {} |
| Perltidy compatibility unsupported-safe | {} |
| Perltidy compatibility external-only | {} |
| Config source | {} |
| Selected formatter engine | {} |
| External formatter adapter requested | {} |
| Config line width | {} |
| Config indent width | {} |
| Config uses tabs | {} |
| Config brace placement | {} |
| Config else placement | {} |
| Config keyword spacing | {} |
| Config trailing comma | {} |

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
| Native critic check profile | {} |
| Native critic check files | {} |
| Native critic check parse errors | {} |
| Native critic check rules run | {} |
| Native critic check findings | {} |
| Native critic check suppressed | {} |
| Native critic check fixable | {} |
| Native critic false-positive files | {} |
| Native critic false-positive parse errors | {} |
| Native critic false-positive rules run | {} |
| Native critic false-positive findings | {} |
| Native critic false-positive suppressed | {} |
| Native critic false-positive fixable | {} |
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
        corpus_unsupported_parse_clean,
        corpus_parse_errors,
        corpus_passed,
        perltidy_compat_options,
        perltidy_compat_supported,
        perltidy_compat_approximated,
        perltidy_compat_unsupported_safe,
        perltidy_compat_external_only,
        format_config_source,
        format_engine_selected,
        format_external_adapter,
        format_line_width,
        format_indent_width,
        format_use_tabs,
        format_brace_placement,
        format_else_placement,
        format_keyword_spacing,
        format_trailing_comma,
        critic.native_rule_count,
        critic.rules_with_suppression,
        critic.rules_with_fixes,
        critic.rules_surfaced_in_pull_diagnostics,
        critic.rules_surfaced_in_push_diagnostics,
        critic.rules_surfaced_in_workspace_diagnostics,
        critic.rules_with_violation_bridge,
        critic_check_profile,
        critic_check_files_checked,
        critic_check_parse_errors,
        critic_check_rules_run,
        critic_check_findings,
        critic_check_suppressed,
        critic_check_fixable,
        critic_false_positive_files_checked,
        critic_false_positive_parse_errors,
        critic_false_positive_rules_run,
        critic_false_positive_findings,
        critic_false_positive_suppressed,
        critic_false_positive_fixable,
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

fn render_readiness_markdown(receipt: &NativeToolingReadinessReceipt) -> String {
    let mut markdown = String::new();
    markdown.push_str("# Native Tooling Readiness\n\n");
    markdown.push_str("> Generated by `cargo xtask native-tooling readiness`.\n\n");
    markdown.push_str(&format!("- Verdict: `{}`\n", receipt.verdict));
    markdown.push_str(&format!("- Ready: `{}`\n", receipt.ready_count));
    markdown.push_str(&format!("- Blockers: `{}`\n", receipt.blocker_count));
    markdown.push_str(&format!("- Warnings: `{}`\n", receipt.warning_count));
    markdown.push_str(&format!("- Unverified: `{}`\n\n", receipt.unverified_count));
    markdown.push_str("| Area | Criterion | Status | Required | Evidence | Next |\n");
    markdown.push_str("| --- | --- | --- | --- | --- | --- |\n");
    for criterion in &receipt.criteria {
        markdown.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            criterion.area,
            criterion.name,
            criterion.status,
            criterion.required_for_default,
            criterion.evidence.replace('|', "\\|"),
            criterion.next.replace('|', "\\|")
        ));
    }
    markdown
}

fn write_perlcritic_compat_summary(path: &Path, receipt: &PerlcriticCompatReceipt) -> Result<()> {
    let report = PerlcriticCompatReport {
        item_count: receipt.item_count,
        native_equivalent_count: receipt.native_equivalent_count,
        native_superset_count: receipt.native_superset_count,
        approximated_count: receipt.approximated_count,
        unsupported_safe_count: receipt.unsupported_safe_count,
        external_only_count: receipt.external_only_count,
        suggested_config: receipt.suggested_config.clone(),
        items: receipt.items.clone(),
    };
    let markdown = render_perlcritic_compat_markdown(&receipt.profile, &report);
    fs::write(path, markdown).wrap_err_with(|| format!("failed to write {}", path.display()))
}

fn optional_metric(value: Option<usize>, receipt_present: bool) -> String {
    value.map(|value| value.to_string()).unwrap_or_else(|| {
        if receipt_present { "unknown".to_string() } else { "UNVERIFIED".to_string() }
    })
}

fn optional_bool_metric(value: Option<bool>, receipt_present: bool) -> String {
    value.map(|value| value.to_string()).unwrap_or_else(|| {
        if receipt_present { "unknown".to_string() } else { "UNVERIFIED".to_string() }
    })
}

fn optional_text_metric(value: Option<&str>, receipt_present: bool) -> String {
    value.map(ToOwned::to_owned).unwrap_or_else(|| {
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
  "unsupported_parse_clean_count": 0,
  "parse_error_count": 0,
  "diagnostics_count": 1,
  "passed": true
}
"#,
        )?;
        let format_perltidy_compat_receipt = receipts.join("native-format-perltidy-compat.json");
        fs::write(
            &format_perltidy_compat_receipt,
            r#"{
  "option_count": 9,
  "supported_count": 7,
  "approximated_count": 0,
  "unsupported_safe_count": 1,
  "external_only_count": 1
}
"#,
        )?;
        let format_config_receipt = receipts.join("native-format-config.json");
        fs::write(
            &format_config_receipt,
            r#"{
  "config_source": "project",
  "engine_selected": "native",
  "external_adapter_requested": false,
  "line_width": 88,
  "indent_width": 2,
  "use_tabs": false,
  "brace_placement": "next-line",
  "else_placement": "separate-line",
  "keyword_spacing": "compact",
  "trailing_comma": "add-when-wrapped"
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
        let critic_check_receipt = receipts.join("native-critic-check.json");
        fs::write(
            &critic_check_receipt,
            r#"{
  "profile": "recommended",
  "files_checked": 3,
  "files_with_parse_errors": 0,
  "rules_run": 28,
  "findings_count": 4,
  "suppressed_findings_count": 1,
  "fixable_findings_count": 2
}
"#,
        )?;
        let critic_false_positive_receipt = receipts.join("native-critic-false-positive.json");
        fs::write(
            &critic_false_positive_receipt,
            r#"{
  "files_checked": 3,
  "files_with_parse_errors": 0,
  "rules_run": 28,
  "findings_count": 0,
  "suppressed_findings_count": 0,
  "fixable_findings_count": 0
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
            format_config_receipt: format_config_receipt.clone(),
            critic_perlcritic_compat_receipt: critic_perlcritic_compat_receipt.clone(),
            critic_check_receipt: critic_check_receipt.clone(),
            critic_false_positive_receipt: critic_false_positive_receipt.clone(),
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
        assert_eq!(value["formatter"]["corpus_unsupported_parse_clean_count"], 0);
        assert_eq!(value["formatter"]["corpus_parse_error_count"], 0);
        assert_eq!(value["formatter"]["corpus_passed"], true);
        assert_eq!(value["formatter"]["format_perltidy_compat_receipt_present"], true);
        assert_eq!(value["formatter"]["perltidy_compat_option_count"], 9);
        assert_eq!(value["formatter"]["perltidy_compat_supported_count"], 7);
        assert_eq!(value["formatter"]["perltidy_compat_approximated_count"], 0);
        assert_eq!(value["formatter"]["perltidy_compat_unsupported_safe_count"], 1);
        assert_eq!(value["formatter"]["perltidy_compat_external_only_count"], 1);
        assert_eq!(value["formatter"]["format_config_receipt_present"], true);
        assert_eq!(value["formatter"]["format_config_source"], "project");
        assert_eq!(value["formatter"]["format_engine_selected"], "native");
        assert_eq!(value["formatter"]["format_external_adapter_requested"], false);
        assert_eq!(value["formatter"]["format_line_width"], 88);
        assert_eq!(value["formatter"]["format_indent_width"], 2);
        assert_eq!(value["formatter"]["format_use_tabs"], false);
        assert_eq!(value["formatter"]["format_brace_placement"], "next-line");
        assert_eq!(value["formatter"]["format_else_placement"], "separate-line");
        assert_eq!(value["formatter"]["format_keyword_spacing"], "compact");
        assert_eq!(value["formatter"]["format_trailing_comma"], "add-when-wrapped");
        assert!(value["critic"]["native_rule_count"].as_u64().unwrap_or_default() > 0);
        assert_eq!(value["critic"]["critic_check_receipt_present"], true);
        assert_eq!(value["critic"]["critic_check_profile"], "recommended");
        assert_eq!(value["critic"]["critic_check_files_checked"], 3);
        assert_eq!(value["critic"]["critic_check_files_with_parse_errors"], 0);
        assert_eq!(value["critic"]["critic_check_rules_run"], 28);
        assert_eq!(value["critic"]["critic_check_findings_count"], 4);
        assert_eq!(value["critic"]["critic_check_suppressed_findings_count"], 1);
        assert_eq!(value["critic"]["critic_check_fixable_findings_count"], 2);
        assert_eq!(value["critic"]["critic_false_positive_receipt_present"], true);
        assert_eq!(value["critic"]["critic_false_positive_files_checked"], 3);
        assert_eq!(value["critic"]["critic_false_positive_files_with_parse_errors"], 0);
        assert_eq!(value["critic"]["critic_false_positive_rules_run"], 28);
        assert_eq!(value["critic"]["critic_false_positive_findings_count"], 0);
        assert_eq!(value["critic"]["critic_false_positive_suppressed_findings_count"], 0);
        assert_eq!(value["critic"]["critic_false_positive_fixable_findings_count"], 0);
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
        assert!(markdown.contains("| Corpus unsupported parse-clean diagnostics | 0 |"));
        assert!(markdown.contains("| Corpus parse-error diagnostics | 0 |"));
        assert!(markdown.contains("| Corpus passed | true |"));
        assert!(markdown.contains("| Perltidy compatibility options | 9 |"));
        assert!(markdown.contains("| Perltidy compatibility supported | 7 |"));
        assert!(markdown.contains("| Perltidy compatibility approximated | 0 |"));
        assert!(markdown.contains("| Perltidy compatibility unsupported-safe | 1 |"));
        assert!(markdown.contains("| Perltidy compatibility external-only | 1 |"));
        assert!(markdown.contains("| Config source | project |"));
        assert!(markdown.contains("| Selected formatter engine | native |"));
        assert!(markdown.contains("| External formatter adapter requested | false |"));
        assert!(markdown.contains("| Config line width | 88 |"));
        assert!(markdown.contains("| Config indent width | 2 |"));
        assert!(markdown.contains("| Config uses tabs | false |"));
        assert!(markdown.contains("| Config brace placement | next-line |"));
        assert!(markdown.contains("| Config else placement | separate-line |"));
        assert!(markdown.contains("| Config keyword spacing | compact |"));
        assert!(markdown.contains("| Config trailing comma | add-when-wrapped |"));
        assert!(markdown.contains("| Native critic check profile | recommended |"));
        assert!(markdown.contains("| Native critic check files | 3 |"));
        assert!(markdown.contains("| Native critic check parse errors | 0 |"));
        assert!(markdown.contains("| Native critic check rules run | 28 |"));
        assert!(markdown.contains("| Native critic check findings | 4 |"));
        assert!(markdown.contains("| Native critic check suppressed | 1 |"));
        assert!(markdown.contains("| Native critic check fixable | 2 |"));
        assert!(markdown.contains("| Native critic false-positive files | 3 |"));
        assert!(markdown.contains("| Native critic false-positive parse errors | 0 |"));
        assert!(markdown.contains("| Native critic false-positive rules run | 28 |"));
        assert!(markdown.contains("| Native critic false-positive findings | 0 |"));
        assert!(markdown.contains("| Native critic false-positive suppressed | 0 |"));
        assert!(markdown.contains("| Native critic false-positive fixable | 0 |"));
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
            format_config_receipt,
            critic_perlcritic_compat_receipt,
            critic_check_receipt,
            critic_false_positive_receipt,
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
        assert_eq!(receipt["approximated_count"], 3);
        assert_eq!(receipt["unsupported_safe_count"], 2);
        assert_eq!(receipt["external_only_count"], 0);
        assert_eq!(receipt["items"][0]["classification"], "native_equivalent");
        assert_eq!(receipt["items"][2]["name"], "exclude");
        assert_eq!(receipt["items"][5]["native_rule"], "native.io.two_arg_open");
        assert_eq!(receipt["items"][6]["native_rule"], "native.io.unchecked_open_close");
        assert_eq!(receipt["items"][8]["classification"], "approximated");
        assert_eq!(receipt["items"][9]["classification"], "approximated");
        assert_eq!(receipt["items"][10]["classification"], "approximated");
        assert_eq!(receipt["items"][11]["name"], "color");
        assert_eq!(receipt["items"][11]["classification"], "unsupported_safe");
        assert_eq!(receipt["suggested_config"]["engine"], "native");
        assert_eq!(receipt["suggested_config"]["profile"], "recommended");
        assert_eq!(receipt["suggested_config"]["perlcritic_severity"], 3);
        assert_eq!(receipt["suggested_config"]["include"][0], "native.testing.require_use_strict");
        assert_eq!(
            receipt["suggested_config"]["exclude"][0],
            "native.documentation.require_pod_sections"
        );

        let summary = fs::read_to_string(receipts.join("perlcritic-compat.md"))?;
        assert!(summary.contains("# Native Critic Perlcritic Compatibility"));
        assert!(summary.contains("## Suggested Native Critic Config"));
        assert!(summary.contains("perlcritic_severity = 3"));
        assert!(summary.contains("include = [\"native.testing.require_use_strict\"]"));
        assert!(summary.contains("exclude = [\"native.documentation.require_pod_sections\"]"));
        assert!(summary.contains(
            "| setting | `exclude` | Documentation::RequirePodSections | native_equivalent |"
        ));
        assert!(summary.contains("| policy | `InputOutput::ProhibitTwoArgOpen` |  | native_equivalent | native.io.two_arg_open |"));
        assert!(summary.contains("| policy | `InputOutput::RequireCheckedOpen` |  | native_superset | native.io.unchecked_open_close |"));
        assert!(summary.contains("| policy | `Documentation::RequirePodSections` |  | approximated | native.documentation.require_pod_sections |"));
        assert!(summary.contains("| setting | `profile-strictness` | quiet | unsupported_safe |"));
        assert!(summary.contains("| setting | `theme` | core | approximated |"));
        assert!(summary.contains("| setting | `color` | 1 | unsupported_safe |"));

        Ok(())
    }

    #[test]
    fn native_tooling_readiness_marks_default_cutover_blockers() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let status_receipt = temp.path().join("status.json");
        let readiness_receipt = temp.path().join("readiness.json");
        let readiness_markdown = temp.path().join("readiness.md");
        let status = NativeToolingStatusReceipt {
            kind: "native_tooling_status".to_string(),
            schema_version: SCHEMA_VERSION,
            generated_at: Utc::now(),
            commit: "test".to_string(),
            formatter: FormatterStatus {
                fixture_root: "fixtures".to_string(),
                fixture_count: 2,
                expected_diagnostics_fixture_count: 1,
                literal_preserve_fixture_count: 1,
                format_receipt: "native-format-fixtures.json".to_string(),
                format_receipt_present: true,
                fixture_passed_count: Some(2),
                fixture_failed_count: Some(0),
                idempotent_count: Some(2),
                parse_preserved_count: Some(2),
                diagnostics_count: Some(1),
                bailout_count: Some(1),
                expected_diagnostics_match_count: Some(1),
                format_corpus_receipt: "native-format-corpus.json".to_string(),
                format_corpus_receipt_present: true,
                corpus_files_checked: Some(3),
                corpus_files_changed: Some(1),
                corpus_idempotence_passed_count: Some(3),
                corpus_parse_preserved_count: Some(3),
                corpus_literal_bailout_count: Some(1),
                corpus_unsupported_patterns_count: Some(2),
                corpus_unsupported_parse_clean_count: Some(0),
                corpus_parse_error_count: Some(2),
                corpus_diagnostics_count: Some(1),
                corpus_passed: Some(true),
                format_perltidy_compat_receipt: "native-format-perltidy-compat.json".to_string(),
                format_perltidy_compat_receipt_present: true,
                perltidy_compat_option_count: Some(9),
                perltidy_compat_supported_count: Some(7),
                perltidy_compat_approximated_count: Some(0),
                perltidy_compat_unsupported_safe_count: Some(1),
                perltidy_compat_external_only_count: Some(1),
                format_config_receipt: "native-format-config.json".to_string(),
                format_config_receipt_present: true,
                format_config_source: Some("defaults".to_string()),
                format_engine_selected: Some("native".to_string()),
                format_external_adapter_requested: Some(false),
                format_line_width: Some(80),
                format_indent_width: Some(4),
                format_use_tabs: Some(false),
                format_brace_placement: Some("same-line".to_string()),
                format_else_placement: Some("cuddled".to_string()),
                format_keyword_spacing: Some("space".to_string()),
                format_trailing_comma: Some("preserve".to_string()),
            },
            critic: CriticStatus {
                native_rule_count: 2,
                native_rules: vec![
                    "native.testing.require_use_strict".to_string(),
                    "native.variables.unused_lexical".to_string(),
                ],
                rules_with_suppression: 2,
                rules_with_fixes: 1,
                fixable_rules: vec!["native.variables.unused_lexical".to_string()],
                rules_surfaced_in_pull_diagnostics: 2,
                rules_surfaced_in_push_diagnostics: 2,
                rules_surfaced_in_workspace_diagnostics: 2,
                rules_with_violation_bridge: 2,
                critic_check_receipt: "native-critic-check.json".to_string(),
                critic_check_receipt_present: true,
                critic_check_profile: Some("recommended".to_string()),
                critic_check_files_checked: Some(3),
                critic_check_files_with_parse_errors: Some(0),
                critic_check_rules_run: Some(2),
                critic_check_findings_count: Some(1),
                critic_check_suppressed_findings_count: Some(0),
                critic_check_fixable_findings_count: Some(1),
                critic_false_positive_receipt: "native-critic-false-positive.json".to_string(),
                critic_false_positive_receipt_present: true,
                critic_false_positive_files_checked: Some(2),
                critic_false_positive_files_with_parse_errors: Some(0),
                critic_false_positive_rules_run: Some(2),
                critic_false_positive_findings_count: Some(0),
                critic_false_positive_suppressed_findings_count: Some(0),
                critic_false_positive_fixable_findings_count: Some(0),
                critic_perlcritic_compat_receipt: "perlcritic-compat.json".to_string(),
                critic_perlcritic_compat_receipt_present: true,
                perlcritic_compat_item_count: Some(4),
                perlcritic_compat_native_equivalent_count: Some(1),
                perlcritic_compat_native_superset_count: Some(1),
                perlcritic_compat_approximated_count: Some(0),
                perlcritic_compat_unsupported_safe_count: Some(0),
                perlcritic_compat_external_only_count: Some(2),
            },
        };
        write_json(&status_receipt, &status)?;

        readiness(NativeToolingReadinessConfig {
            status_receipt,
            receipt: readiness_receipt.clone(),
            markdown: Some(readiness_markdown.clone()),
        })?;

        let value: Value = serde_json::from_str(&fs::read_to_string(readiness_receipt)?)?;
        assert_eq!(value["kind"], "native_tooling_readiness");
        assert_eq!(value["verdict"], "not_ready");
        assert_eq!(value["blocker_count"].as_u64().unwrap_or_default(), 2);
        assert_eq!(value["warning_count"].as_u64().unwrap_or_default(), 0);
        assert!(value["ready_count"].as_u64().unwrap_or_default() > 0);
        let criteria = value["criteria"].as_array().ok_or_else(|| eyre!("criteria array"))?;
        assert!(criteria.iter().any(|criterion| {
            criterion["name"] == "corpus parse-clean unsupported formatter diagnostics are cleared"
                && criterion["status"] == "ready"
                && criterion["required_for_default"] == false
        }));
        assert!(criteria.iter().any(|criterion| {
            criterion["name"] == "perltidy compatibility has no external-only gaps"
                && criterion["status"] == "blocked"
        }));
        assert!(criteria.iter().any(|criterion| {
            criterion["name"] == "perlcritic compatibility has no external-only gaps"
                && criterion["status"] == "blocked"
        }));
        assert!(criteria.iter().any(|criterion| {
            criterion["name"] == "native critic default" && criterion["status"] == "ready"
        }));
        assert!(criteria.iter().any(|criterion| {
            criterion["name"] == "native critic false-positive fixtures are clean"
                && criterion["status"] == "ready"
        }));

        let markdown = fs::read_to_string(readiness_markdown)?;
        assert!(markdown.contains("# Native Tooling Readiness"));
        assert!(markdown.contains("- Verdict: `not_ready`"));
        assert!(markdown.contains("- Warnings: `0`"));
        assert!(markdown.contains(
            "| formatter | corpus parse-clean unsupported formatter diagnostics are cleared | ready | false |"
        ));
        assert!(markdown.contains(
            "| formatter | perltidy compatibility has no external-only gaps | blocked |"
        ));
        assert!(
            markdown.contains(
                "| critic | perlcritic compatibility has no external-only gaps | blocked |"
            )
        );
        assert!(markdown.contains("| critic | native critic default | ready |"));
        assert!(
            markdown
                .contains("| critic | native critic false-positive fixtures are clean | ready |")
        );
        Ok(())
    }

    #[test]
    fn native_tooling_default_checks_require_native_paths_to_avoid_shell_out() -> Result<()> {
        let temp = tempfile::tempdir()?;
        write_default_guard_sources(temp.path())?;

        let checks = native_tooling_default_checks(temp.path())?;

        assert!(
            checks.iter().all(|check| check.passed),
            "expected all checks to pass: {checks:#?}"
        );
        assert!(
            checks
                .iter()
                .any(|check| check.name == "external_formatter_requires_external_legacy_mode")
        );
        assert!(checks.iter().any(|check| check.name == "native_critic_skips_external_collectors"));

        Ok(())
    }

    #[test]
    fn native_tooling_default_checks_fail_when_native_critic_skip_guard_is_missing() -> Result<()> {
        let temp = tempfile::tempdir()?;
        write_default_guard_sources(temp.path())?;
        fs::write(
            temp.path().join("crates/perl-lsp-rs/src/runtime/diagnostics.rs"),
            "fn collect() { if !enabled { return; } }\n",
        )?;

        let checks = native_tooling_default_checks(temp.path())?;
        let critic_guard = checks
            .iter()
            .find(|check| check.name == "native_critic_skips_external_collectors")
            .ok_or_else(|| eyre!("missing native critic guard check"))?;

        assert!(!critic_guard.passed);
        Ok(())
    }

    fn write_default_guard_sources(root: &Path) -> Result<()> {
        let formatting_path =
            root.join("crates/perl-lsp-rs-core/src/providers/formatting/formatting.rs");
        let diagnostics_path = root.join("crates/perl-lsp-rs/src/runtime/diagnostics.rs");
        let docs_path = root.join("docs/reference/CONFIGURATION.md");
        fs::create_dir_all(formatting_path.parent().ok_or_else(|| eyre!("missing parent"))?)?;
        fs::create_dir_all(diagnostics_path.parent().ok_or_else(|| eyre!("missing parent"))?)?;
        fs::create_dir_all(docs_path.parent().ok_or_else(|| eyre!("missing parent"))?)?;
        fs::write(
            formatting_path,
            r#"
match self.mode {
    FormatterMode::Native | FormatterMode::Compat => {
        Ok(native_format_document(content, options, self.perltidy_config.as_ref()))
    }
    FormatterMode::ExternalLegacy => self.format_document_with_perltidy(content, options),
    FormatterMode::Off => Ok(FormattedDocument { text: content.to_string(), edits: vec![] }),
}
match self.mode {
    FormatterMode::Native | FormatterMode::Compat => {
        Ok(native_format_range(content, range, options, self.perltidy_config.as_ref()))
    }
    FormatterMode::ExternalLegacy => {
                self.format_range_with_perltidy(content, options, &lines, start_line, end_line)
            }
    FormatterMode::Off => Ok(FormattedDocument { text: content.to_string(), edits: vec![] }),
}
"#,
        )?;
        fs::write(
            diagnostics_path,
            r#"
if !enabled || critic_engine == perl_lsp_rs_core::config::CriticEngine::Native {
    return;
}
if !enabled || critic_engine == perl_lsp_rs_core::config::CriticEngine::Native {
    return;
}
"#,
        )?;
        fs::write(
            docs_path,
            r#"
| `[critic]` | `engine` | string | `"native"` | Critic engine |
| `[formatting]` | `engine` | string | `"native"` | Formatter engine |
| `[formatting] engine = "native"` | `"formatting": {"engine": "native"}` | Use `"external-perltidy"` for legacy shell-out compatibility |
| `[critic] engine = "native"` | `"critic": {"engine": "native"}` | Use `"legacy"` or `"external"` for Perl::Critic shell-out compatibility |
"#,
        )?;
        Ok(())
    }
}
