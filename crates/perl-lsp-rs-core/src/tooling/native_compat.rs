//! Native tooling compatibility reports for user-facing migration commands.
//!
//! These helpers classify legacy `perltidy` and `perlcritic` profile files
//! against the Rust-native formatter and critic surfaces without invoking the
//! external tools. Developer receipt commands in `xtask` render richer CI
//! artifacts; this module provides the small stable report model needed by the
//! installed `perllsp` binary.

use super::perl_critic::NativeCriticRegistry;
use serde::Serialize;
use std::collections::BTreeSet;

/// Native formatter compatibility report for a `.perltidyrc` profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PerltidyCompatReport {
    /// Number of perltidy options found in the profile.
    pub option_count: usize,
    /// Options that map directly to native formatter config.
    pub supported_count: usize,
    /// Options approximated by current native formatter behavior.
    pub approximated_count: usize,
    /// Execution/output options that are safe to ignore for native formatting.
    pub unsupported_safe_count: usize,
    /// Options that still require external perltidy compatibility mode.
    pub external_only_count: usize,
    /// Suggested native formatter configuration derived from compatible profile options.
    pub suggested_config: PerltidyNativeConfigSuggestion,
    /// Per-option classifications in source order.
    pub options: Vec<PerltidyCompatOption>,
}

/// Native formatter config values that can be migrated from a `.perltidyrc` profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PerltidyNativeConfigSuggestion {
    /// Native formatter engine to use for the migrated config.
    pub engine: &'static str,
    /// Maximum line length mapped from `-l`, when present and valid.
    pub perltidy_maximum_line_length: Option<u32>,
    /// Indentation width mapped from `-i`, when present and valid.
    pub perltidy_indent_columns: Option<u32>,
    /// Tab indentation setting mapped from `-t` / `-nt`.
    pub perltidy_tabs: Option<bool>,
    /// Opening brace placement mapped from `-bl` / `-bar`.
    pub perltidy_opening_brace_on_new_line: Option<bool>,
    /// Else placement mapped from `-ce` / `-nce`.
    pub perltidy_cuddled_else: Option<bool>,
    /// Keyword spacing mapped from `-sok` / `-nsok`.
    pub perltidy_space_after_keyword: Option<bool>,
    /// Trailing comma behavior mapped from `-atc` / `-natc`.
    pub perltidy_add_trailing_commas: Option<bool>,
    /// Supported options whose values could not be parsed.
    pub invalid_options: Vec<String>,
    /// Execution/output options ignored by native formatting.
    pub ignored_options: Vec<String>,
    /// Profile or preset options approximated by native settings.
    pub approximated_options: Vec<String>,
    /// Options that still need explicit external perltidy compatibility mode.
    pub external_only_options: Vec<String>,
}

/// Per-option compatibility classification for a `.perltidyrc` entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PerltidyCompatOption {
    /// Raw perltidy option token, such as `-l`.
    pub option: String,
    /// Optional value associated with the option.
    pub value: Option<String>,
    /// Classification: supported, approximated, unsupported_safe, or external_only.
    pub classification: &'static str,
    /// Native formatter config field when the option maps directly.
    pub native_field: Option<&'static str>,
    /// Human explanation for the classification.
    pub note: &'static str,
}

/// Native critic compatibility report for a `.perlcriticrc` profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PerlcriticCompatReport {
    /// Number of settings and policy sections found in the profile.
    pub item_count: usize,
    /// Items with direct native critic equivalents.
    pub native_equivalent_count: usize,
    /// Items where native critic behavior is broader or more precise.
    pub native_superset_count: usize,
    /// Items approximated by the current native recommended profile.
    pub approximated_count: usize,
    /// Settings that are safe to ignore for structured native diagnostics.
    pub unsupported_safe_count: usize,
    /// Items that still require external perlcritic compatibility mode.
    pub external_only_count: usize,
    /// Suggested native critic configuration derived from compatible profile settings.
    pub suggested_config: PerlcriticNativeConfigSuggestion,
    /// Per-item classifications in source order.
    pub items: Vec<PerlcriticCompatItem>,
}

/// Native critic config values that can be migrated from a `.perlcriticrc` profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PerlcriticNativeConfigSuggestion {
    /// Native critic engine to use for the migrated config.
    pub engine: &'static str,
    /// Conservative native profile used as the migration target.
    pub profile: &'static str,
    /// Project config severity derived from `severity`, when present and valid.
    pub perlcritic_severity: Option<u8>,
    /// Native rule IDs derived from compatible `include` policies.
    pub include: Vec<String>,
    /// Native rule IDs derived from compatible `exclude` policies.
    pub exclude: Vec<String>,
    /// `include` policy names that could not be mapped to active native rules.
    pub unmapped_include: Vec<String>,
    /// `exclude` policy names that could not be mapped to active native rules.
    pub unmapped_exclude: Vec<String>,
}

/// Per-setting or per-policy compatibility classification for `.perlcriticrc`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PerlcriticCompatItem {
    /// Item kind: setting or policy.
    pub kind: &'static str,
    /// Setting name or policy name.
    pub name: String,
    /// Setting value, when present.
    pub value: Option<String>,
    /// Classification: native_equivalent, native_superset, approximated,
    /// unsupported_safe, or external_only.
    pub classification: &'static str,
    /// Native rule ID when the item maps to a rule.
    pub native_rule: Option<&'static str>,
    /// Human explanation for the classification.
    pub note: &'static str,
}

/// Classify a `.perltidyrc`-style profile against native formatter support.
#[must_use]
pub fn classify_perltidy_profile(raw: &str) -> PerltidyCompatReport {
    let options = tokenize_perltidy_profile(raw)
        .iter()
        .map(|(option, value)| classify_perltidy_option(option, value.clone()))
        .collect::<Vec<_>>();
    let suggested_config = suggested_perltidy_config(&options);
    PerltidyCompatReport {
        option_count: options.len(),
        supported_count: perltidy_count(&options, "supported"),
        approximated_count: perltidy_count(&options, "approximated"),
        unsupported_safe_count: perltidy_count(&options, "unsupported_safe"),
        external_only_count: perltidy_count(&options, "external_only"),
        suggested_config,
        options,
    }
}

/// Classify a `.perlcriticrc`-style profile against native critic support.
#[must_use]
pub fn classify_perlcritic_profile(raw: &str) -> PerlcriticCompatReport {
    let native_rules = NativeCriticRegistry::recommended()
        .rule_ids()
        .into_iter()
        .map(ToOwned::to_owned)
        .collect::<BTreeSet<_>>();
    let mut items = Vec::new();
    let mut suggested_config = PerlcriticNativeConfigSuggestion {
        engine: "native",
        profile: "recommended",
        perlcritic_severity: None,
        include: Vec::new(),
        exclude: Vec::new(),
        unmapped_include: Vec::new(),
        unmapped_exclude: Vec::new(),
    };

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
            let name = name.trim();
            let value = value.trim().to_string();
            apply_perlcritic_setting_to_suggestion(
                name,
                &value,
                &native_rules,
                &mut suggested_config,
            );
            items.push(classify_perlcritic_setting(name, Some(value)));
            continue;
        }

        items.push(perlcritic_item(
            "setting",
            line,
            None,
            "external_only",
            None,
            "unrecognized perlcritic profile line is not applied by native critic",
        ));
    }

    PerlcriticCompatReport {
        item_count: items.len(),
        native_equivalent_count: perlcritic_count(&items, "native_equivalent"),
        native_superset_count: perlcritic_count(&items, "native_superset"),
        approximated_count: perlcritic_count(&items, "approximated"),
        unsupported_safe_count: perlcritic_count(&items, "unsupported_safe"),
        external_only_count: perlcritic_count(&items, "external_only"),
        suggested_config,
        items,
    }
}

/// Render a human-readable Markdown summary for a perltidy compatibility report.
#[must_use]
pub fn render_perltidy_compat_markdown(profile: &str, report: &PerltidyCompatReport) -> String {
    let mut markdown = String::new();
    markdown.push_str("# Native Format Perltidy Compatibility\n\n");
    markdown.push_str(&format!("- Profile: `{profile}`\n"));
    markdown.push_str(&format!("- Options checked: {}\n", report.option_count));
    markdown.push_str(&format!("- Supported: {}\n", report.supported_count));
    markdown.push_str(&format!("- Approximated: {}\n", report.approximated_count));
    markdown.push_str(&format!("- Unsupported safe: {}\n", report.unsupported_safe_count));
    markdown.push_str(&format!("- External-only: {}\n\n", report.external_only_count));
    markdown.push_str("## Suggested Native Formatter Config\n\n");
    markdown.push_str("```toml\n");
    markdown.push_str("[formatting]\n");
    markdown.push_str(&format!("engine = \"{}\"\n", report.suggested_config.engine));
    if let Some(value) = report.suggested_config.perltidy_maximum_line_length {
        markdown.push_str(&format!("perltidy_maximum_line_length = {value}\n"));
    }
    if let Some(value) = report.suggested_config.perltidy_indent_columns {
        markdown.push_str(&format!("perltidy_indent_columns = {value}\n"));
    }
    if let Some(value) = report.suggested_config.perltidy_tabs {
        markdown.push_str(&format!("perltidy_tabs = {value}\n"));
    }
    if let Some(value) = report.suggested_config.perltidy_opening_brace_on_new_line {
        markdown.push_str(&format!("perltidy_opening_brace_on_new_line = {value}\n"));
    }
    if let Some(value) = report.suggested_config.perltidy_cuddled_else {
        markdown.push_str(&format!("perltidy_cuddled_else = {value}\n"));
    }
    if let Some(value) = report.suggested_config.perltidy_space_after_keyword {
        markdown.push_str(&format!("perltidy_space_after_keyword = {value}\n"));
    }
    if let Some(value) = report.suggested_config.perltidy_add_trailing_commas {
        markdown.push_str(&format!("perltidy_add_trailing_commas = {value}\n"));
    }
    markdown.push_str("```\n\n");
    render_perltidy_option_notes(&mut markdown, &report.suggested_config);
    markdown.push_str("| Option | Value | Classification | Native field | Note |\n");
    markdown.push_str("| --- | --- | --- | --- | --- |\n");
    for option in &report.options {
        markdown.push_str(&format!(
            "| `{}` | {} | {} | {} | {} |\n",
            option.option,
            option.value.as_deref().unwrap_or(""),
            option.classification,
            option.native_field.unwrap_or(""),
            option.note
        ));
    }
    markdown
}

fn render_perltidy_option_notes(markdown: &mut String, config: &PerltidyNativeConfigSuggestion) {
    if config.invalid_options.is_empty()
        && config.ignored_options.is_empty()
        && config.approximated_options.is_empty()
        && config.external_only_options.is_empty()
    {
        return;
    }

    markdown.push_str("Migration notes:\n");
    if !config.invalid_options.is_empty() {
        markdown
            .push_str(&format!("- invalid values: `{}`\n", config.invalid_options.join("`, `")));
    }
    if !config.ignored_options.is_empty() {
        markdown.push_str(&format!(
            "- ignored execution/output options: `{}`\n",
            config.ignored_options.join("`, `")
        ));
    }
    if !config.approximated_options.is_empty() {
        markdown.push_str(&format!(
            "- approximated presets: `{}`\n",
            config.approximated_options.join("`, `")
        ));
    }
    if !config.external_only_options.is_empty() {
        markdown.push_str(&format!(
            "- external-only options: `{}`\n",
            config.external_only_options.join("`, `")
        ));
    }
    markdown.push('\n');
}

/// Render a human-readable Markdown summary for a perlcritic compatibility report.
#[must_use]
pub fn render_perlcritic_compat_markdown(profile: &str, report: &PerlcriticCompatReport) -> String {
    let mut markdown = String::new();
    markdown.push_str("# Native Critic Perlcritic Compatibility\n\n");
    markdown.push_str(&format!("- Profile: `{profile}`\n"));
    markdown.push_str(&format!("- Items checked: {}\n", report.item_count));
    markdown.push_str(&format!("- Native equivalent: {}\n", report.native_equivalent_count));
    markdown.push_str(&format!("- Native superset: {}\n", report.native_superset_count));
    markdown.push_str(&format!("- Approximated: {}\n", report.approximated_count));
    markdown.push_str(&format!("- Unsupported safe: {}\n", report.unsupported_safe_count));
    markdown.push_str(&format!("- External-only: {}\n\n", report.external_only_count));
    markdown.push_str("## Suggested Native Critic Config\n\n");
    markdown.push_str("```toml\n");
    if let Some(severity) = report.suggested_config.perlcritic_severity {
        markdown.push_str("[diagnostics]\n");
        markdown.push_str(&format!("perlcritic_severity = {severity}\n\n"));
    }
    markdown.push_str("[critic]\n");
    markdown.push_str(&format!("engine = \"{}\"\n", report.suggested_config.engine));
    markdown.push_str(&format!("profile = \"{}\"\n", report.suggested_config.profile));
    if !report.suggested_config.include.is_empty() {
        markdown
            .push_str(&format!("include = [{}]\n", quoted_list(&report.suggested_config.include)));
    }
    if !report.suggested_config.exclude.is_empty() {
        markdown
            .push_str(&format!("exclude = [{}]\n", quoted_list(&report.suggested_config.exclude)));
    }
    markdown.push_str("```\n\n");
    if !report.suggested_config.unmapped_include.is_empty()
        || !report.suggested_config.unmapped_exclude.is_empty()
    {
        markdown.push_str("Unmapped legacy filters:\n");
        if !report.suggested_config.unmapped_include.is_empty() {
            markdown.push_str(&format!(
                "- include: `{}`\n",
                report.suggested_config.unmapped_include.join("`, `")
            ));
        }
        if !report.suggested_config.unmapped_exclude.is_empty() {
            markdown.push_str(&format!(
                "- exclude: `{}`\n",
                report.suggested_config.unmapped_exclude.join("`, `")
            ));
        }
        markdown.push('\n');
    }
    markdown.push_str("| Kind | Name | Value | Classification | Native rule | Note |\n");
    markdown.push_str("| --- | --- | --- | --- | --- | --- |\n");
    for item in &report.items {
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
    markdown
}

fn quoted_list(values: &[String]) -> String {
    values.iter().map(|value| format!("\"{value}\"")).collect::<Vec<_>>().join(", ")
}

fn tokenize_perltidy_profile(raw: &str) -> Vec<(String, Option<String>)> {
    let tokens = raw
        .lines()
        .filter_map(|line| line.split('#').next())
        .flat_map(str::split_whitespace)
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    let mut options = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        let token = &tokens[index];
        if !token.starts_with('-') {
            index += 1;
            continue;
        }
        if let Some((option, value)) = token.split_once('=') {
            options.push((option.to_string(), Some(value.to_string())));
            index += 1;
            continue;
        }
        if perltidy_option_requires_value(token)
            && tokens.get(index + 1).is_some_and(|value| !value.starts_with('-'))
        {
            options.push((token.to_string(), tokens.get(index + 1).cloned()));
            index += 2;
            continue;
        }
        options.push((token.to_string(), None));
        index += 1;
    }
    options
}

fn perltidy_option_requires_value(option: &str) -> bool {
    matches!(
        option,
        "-l" | "--maximum-line-length"
            | "-i"
            | "--indent-columns"
            | "-ci"
            | "--block-comment-indentation"
    )
}

fn classify_perltidy_option(option: &str, value: Option<String>) -> PerltidyCompatOption {
    match option {
        "-l" | "--maximum-line-length" => perltidy_option(
            option,
            value,
            "supported",
            Some("format.line_width"),
            "maps directly to the native formatter line width",
        ),
        "-i" | "--indent-columns" => perltidy_option(
            option,
            value,
            "supported",
            Some("format.indent_width"),
            "maps directly to native formatter indentation width",
        ),
        "-t" | "--tabs" | "-nt" | "--notabs" => perltidy_option(
            option,
            value,
            "supported",
            Some("format.use_tabs"),
            "maps directly to native formatter tab indentation",
        ),
        "-ce" | "--cuddled-else" | "-nce" | "--nocuddled-else" => perltidy_option(
            option,
            value,
            "supported",
            Some("format.else_placement"),
            "maps to native formatter else placement for supported simple block layouts",
        ),
        "-sok" | "--space-after-keyword" | "-nsok" | "--nospace-after-keyword" => perltidy_option(
            option,
            value,
            "supported",
            Some("format.keyword_spacing"),
            "maps to native formatter keyword spacing for supported simple control-flow headers",
        ),
        "-bl" | "--opening-brace-on-new-line" | "-bar" | "--opening-brace-always-on-right" => {
            perltidy_option(
                option,
                value,
                "supported",
                Some("format.brace_placement"),
                "maps to native formatter brace placement for supported simple block layouts",
            )
        }
        "-atc" | "--add-trailing-commas" | "-natc" | "--no-add-trailing-commas" => perltidy_option(
            option,
            value,
            "supported",
            Some("format.trailing_comma"),
            "maps to native formatter trailing comma policy for wrapped calls, lists, and hashes",
        ),
        "-ci" | "--block-comment-indentation" => perltidy_option(
            option,
            value,
            "external_only",
            None,
            "comment-aware native formatting is not yet configurable",
        ),
        "-val" | "--vertical-alignment" | "-nval" | "--novertical-alignment" => perltidy_option(
            option,
            value,
            "external_only",
            None,
            "native formatter intentionally avoids alignment policy today",
        ),
        "-pbp" | "--perl-best-practices" | "-gnu" | "--gnu-style" => perltidy_option(
            option,
            value,
            "approximated",
            None,
            "native formatter can map individual style settings but not full preset profiles yet",
        ),
        "-q" | "--quiet" | "-st" | "--standard-output" | "-se" | "--standard-error-output" => {
            perltidy_option(
                option,
                value,
                "unsupported_safe",
                None,
                "perltidy execution/output flag does not affect native formatting style",
            )
        }
        _ => perltidy_option(
            option,
            value,
            "external_only",
            None,
            "unknown style option is not applied by native formatter and may require external compatibility mode",
        ),
    }
}

fn perltidy_option(
    option: &str,
    value: Option<String>,
    classification: &'static str,
    native_field: Option<&'static str>,
    note: &'static str,
) -> PerltidyCompatOption {
    PerltidyCompatOption { option: option.to_string(), value, classification, native_field, note }
}

fn suggested_perltidy_config(options: &[PerltidyCompatOption]) -> PerltidyNativeConfigSuggestion {
    let mut suggestion = PerltidyNativeConfigSuggestion {
        engine: "native",
        perltidy_maximum_line_length: None,
        perltidy_indent_columns: None,
        perltidy_tabs: None,
        perltidy_opening_brace_on_new_line: None,
        perltidy_cuddled_else: None,
        perltidy_space_after_keyword: None,
        perltidy_add_trailing_commas: None,
        invalid_options: Vec::new(),
        ignored_options: Vec::new(),
        approximated_options: Vec::new(),
        external_only_options: Vec::new(),
    };

    for option in options {
        match option.classification {
            "supported" => apply_supported_perltidy_option(option, &mut suggestion),
            "unsupported_safe" => {
                push_unique(&mut suggestion.ignored_options, option.option.clone())
            }
            "approximated" => {
                push_unique(&mut suggestion.approximated_options, option.option.clone());
            }
            "external_only" => {
                push_unique(&mut suggestion.external_only_options, option.option.clone());
            }
            _ => {}
        }
    }

    suggestion
}

fn apply_supported_perltidy_option(
    option: &PerltidyCompatOption,
    suggestion: &mut PerltidyNativeConfigSuggestion,
) {
    match option.option.as_str() {
        "-l" | "--maximum-line-length" => {
            apply_perltidy_u32(
                option,
                &mut suggestion.perltidy_maximum_line_length,
                &mut suggestion.invalid_options,
            );
        }
        "-i" | "--indent-columns" => {
            apply_perltidy_u32(
                option,
                &mut suggestion.perltidy_indent_columns,
                &mut suggestion.invalid_options,
            );
        }
        "-t" | "--tabs" => suggestion.perltidy_tabs = Some(true),
        "-nt" | "--notabs" => suggestion.perltidy_tabs = Some(false),
        "-ce" | "--cuddled-else" => suggestion.perltidy_cuddled_else = Some(true),
        "-nce" | "--nocuddled-else" => suggestion.perltidy_cuddled_else = Some(false),
        "-sok" | "--space-after-keyword" => {
            suggestion.perltidy_space_after_keyword = Some(true);
        }
        "-nsok" | "--nospace-after-keyword" => {
            suggestion.perltidy_space_after_keyword = Some(false);
        }
        "-bl" | "--opening-brace-on-new-line" => {
            suggestion.perltidy_opening_brace_on_new_line = Some(true);
        }
        "-bar" | "--opening-brace-always-on-right" => {
            suggestion.perltidy_opening_brace_on_new_line = Some(false);
        }
        "-atc" | "--add-trailing-commas" => suggestion.perltidy_add_trailing_commas = Some(true),
        "-natc" | "--no-add-trailing-commas" => {
            suggestion.perltidy_add_trailing_commas = Some(false);
        }
        _ => {}
    }
}

fn apply_perltidy_u32(
    option: &PerltidyCompatOption,
    target: &mut Option<u32>,
    invalid_options: &mut Vec<String>,
) {
    match option.value.as_deref().and_then(|value| value.parse::<u32>().ok()) {
        Some(value) => *target = Some(value),
        None => push_unique(invalid_options, option.option.clone()),
    }
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
            perlcritic_item("policy", policy, None, classification, Some(native_rule), note)
        }
        Some((native_rule, _, _)) => perlcritic_item(
            "policy",
            policy,
            None,
            "external_only",
            Some(native_rule),
            "mapped native rule is not currently present in the recommended registry",
        ),
        None => perlcritic_item(
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
        "severity" => perlcritic_item(
            "setting",
            name,
            value,
            "native_equivalent",
            None,
            "maps to native critic minimum severity filtering",
        ),
        "include" | "exclude" => perlcritic_item(
            "setting",
            name,
            value,
            "native_equivalent",
            None,
            "native critic supports include/exclude filters; map policy names to native rule IDs before configuring them",
        ),
        "theme" => classify_perlcritic_theme_setting(value),
        "profile-strictness" => perlcritic_item(
            "setting",
            name,
            value,
            "unsupported_safe",
            None,
            "perlcritic loader strictness has no runtime effect on native critic rules",
        ),
        "color" => perlcritic_item(
            "setting",
            name,
            value,
            "unsupported_safe",
            None,
            "perlcritic terminal color setting has no effect on structured native diagnostics",
        ),
        _ => perlcritic_item(
            "setting",
            name,
            value,
            "external_only",
            None,
            "perlcritic setting is not yet applied by native critic",
        ),
    }
}

fn apply_perlcritic_setting_to_suggestion(
    name: &str,
    value: &str,
    native_rules: &BTreeSet<String>,
    suggestion: &mut PerlcriticNativeConfigSuggestion,
) {
    match name {
        "severity" => {
            suggestion.perlcritic_severity = value.parse::<u8>().ok();
        }
        "include" => {
            let (mapped, unmapped) = map_perlcritic_policy_list(value, native_rules);
            push_unique_many(&mut suggestion.include, mapped);
            push_unique_many(&mut suggestion.unmapped_include, unmapped);
        }
        "exclude" => {
            let (mapped, unmapped) = map_perlcritic_policy_list(value, native_rules);
            push_unique_many(&mut suggestion.exclude, mapped);
            push_unique_many(&mut suggestion.unmapped_exclude, unmapped);
        }
        _ => {}
    }
}

fn map_perlcritic_policy_list(
    value: &str,
    native_rules: &BTreeSet<String>,
) -> (Vec<String>, Vec<String>) {
    let mut mapped = Vec::new();
    let mut unmapped = Vec::new();
    for policy in value.split(|ch: char| ch == ',' || ch.is_whitespace()) {
        let policy = policy.trim();
        if policy.is_empty() {
            continue;
        }
        match perlcritic_policy_native_mapping(policy) {
            Some((native_rule, _, _)) if native_rules.contains(native_rule) => {
                push_unique(&mut mapped, native_rule.to_string());
            }
            _ => push_unique(&mut unmapped, policy.to_string()),
        }
    }
    (mapped, unmapped)
}

fn push_unique_many(target: &mut Vec<String>, values: Vec<String>) {
    for value in values {
        push_unique(target, value);
    }
}

fn push_unique(target: &mut Vec<String>, value: String) {
    if !target.iter().any(|existing| existing == &value) {
        target.push(value);
    }
}

fn classify_perlcritic_theme_setting(value: Option<String>) -> PerlcriticCompatItem {
    let Some(theme) = value.as_deref() else {
        return perlcritic_item(
            "setting",
            "theme",
            value,
            "unsupported_safe",
            None,
            "empty perlcritic theme does not change native critic rule selection",
        );
    };
    let known_themes = [
        "bugs",
        "certrec",
        "certrule",
        "core",
        "cosmetic",
        "maintenance",
        "pbp",
        "performance",
        "security",
        "tests",
        "unicode",
    ];
    if known_themes.contains(&theme.trim()) {
        perlcritic_item(
            "setting",
            "theme",
            value,
            "approximated",
            None,
            "native critic recommended profile approximates common perlcritic themes with currently implemented native rules",
        )
    } else {
        perlcritic_item(
            "setting",
            "theme",
            value,
            "external_only",
            None,
            "unrecognized perlcritic theme is not expanded by native critic",
        )
    }
}

fn perlcritic_item(
    kind: &'static str,
    name: &str,
    value: Option<String>,
    classification: &'static str,
    native_rule: Option<&'static str>,
    note: &'static str,
) -> PerlcriticCompatItem {
    PerlcriticCompatItem { kind, name: name.to_string(), value, classification, native_rule, note }
}

fn perltidy_count(options: &[PerltidyCompatOption], classification: &str) -> usize {
    options.iter().filter(|option| option.classification == classification).count()
}

fn perlcritic_count(items: &[PerlcriticCompatItem], classification: &str) -> usize {
    items.iter().filter(|item| item.classification == classification).count()
}

#[cfg(test)]
mod tests {
    use super::{
        classify_perlcritic_profile, classify_perltidy_profile, render_perlcritic_compat_markdown,
        render_perltidy_compat_markdown,
    };

    #[test]
    fn perltidy_profile_classifies_native_supported_options() {
        let report = classify_perltidy_profile(
            "# common profile\n-l=100\n-i 2\n-nt\n-ce\n-nsok\n-q\n-atc\n-bl\n",
        );

        assert_eq!(report.option_count, 8);
        assert_eq!(report.supported_count, 7);
        assert_eq!(report.approximated_count, 0);
        assert_eq!(report.unsupported_safe_count, 1);
        assert_eq!(report.external_only_count, 0);
        assert_eq!(report.options[0].native_field, Some("format.line_width"));
        assert_eq!(report.options[4].native_field, Some("format.keyword_spacing"));
        assert_eq!(report.options[6].native_field, Some("format.trailing_comma"));
        assert_eq!(report.suggested_config.engine, "native");
        assert_eq!(report.suggested_config.perltidy_maximum_line_length, Some(100));
        assert_eq!(report.suggested_config.perltidy_indent_columns, Some(2));
        assert_eq!(report.suggested_config.perltidy_tabs, Some(false));
        assert_eq!(report.suggested_config.perltidy_cuddled_else, Some(true));
        assert_eq!(report.suggested_config.perltidy_space_after_keyword, Some(false));
        assert_eq!(report.suggested_config.perltidy_add_trailing_commas, Some(true));
        assert_eq!(report.suggested_config.perltidy_opening_brace_on_new_line, Some(true));
        assert_eq!(report.suggested_config.ignored_options, vec!["-q"]);

        let markdown = render_perltidy_compat_markdown(".perltidyrc", &report);
        assert!(markdown.contains("# Native Format Perltidy Compatibility"));
        assert!(markdown.contains("## Suggested Native Formatter Config"));
        assert!(markdown.contains("perltidy_maximum_line_length = 100"));
        assert!(markdown.contains("perltidy_space_after_keyword = false"));
        assert!(markdown.contains("| `-l` | 100 | supported | format.line_width |"));
    }

    #[test]
    fn perltidy_profile_keeps_unknown_options_external_only() {
        let report = classify_perltidy_profile("--unknown-style\n");

        assert_eq!(report.option_count, 1);
        assert_eq!(report.external_only_count, 1);
        assert_eq!(report.options[0].option, "--unknown-style");
        assert_eq!(report.options[0].classification, "external_only");
        assert_eq!(report.suggested_config.external_only_options, vec!["--unknown-style"]);
    }

    #[test]
    fn perltidy_profile_reports_invalid_and_approximated_config_suggestions() {
        let report = classify_perltidy_profile("-l nope -pbp -ci=3 -st\n");

        assert_eq!(report.suggested_config.perltidy_maximum_line_length, None);
        assert_eq!(report.suggested_config.invalid_options, vec!["-l"]);
        assert_eq!(report.suggested_config.approximated_options, vec!["-pbp"]);
        assert_eq!(report.suggested_config.external_only_options, vec!["-ci"]);
        assert_eq!(report.suggested_config.ignored_options, vec!["-st"]);

        let markdown = render_perltidy_compat_markdown(".perltidyrc", &report);
        assert!(markdown.contains("- invalid values: `-l`"));
        assert!(markdown.contains("- approximated presets: `-pbp`"));
        assert!(markdown.contains("- external-only options: `-ci`"));
        assert!(markdown.contains("- ignored execution/output options: `-st`"));
    }

    #[test]
    fn perlcritic_profile_classifies_common_policy_surface() {
        let report = classify_perlcritic_profile(
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
        );

        assert_eq!(report.item_count, 12);
        assert_eq!(report.native_equivalent_count, 5);
        assert_eq!(report.native_superset_count, 2);
        assert_eq!(report.approximated_count, 3);
        assert_eq!(report.unsupported_safe_count, 2);
        assert_eq!(report.external_only_count, 0);
        assert_eq!(report.items[5].native_rule, Some("native.io.two_arg_open"));
        assert_eq!(report.items[6].native_rule, Some("native.io.unchecked_open_close"));
        assert_eq!(report.suggested_config.engine, "native");
        assert_eq!(report.suggested_config.profile, "recommended");
        assert_eq!(report.suggested_config.perlcritic_severity, Some(3));
        assert_eq!(report.suggested_config.include, vec!["native.testing.require_use_strict"]);
        assert_eq!(
            report.suggested_config.exclude,
            vec!["native.documentation.require_pod_sections"]
        );

        let markdown = render_perlcritic_compat_markdown(".perlcriticrc", &report);
        assert!(markdown.contains("# Native Critic Perlcritic Compatibility"));
        assert!(markdown.contains("include = [\"native.testing.require_use_strict\"]"));
        assert!(markdown.contains("exclude = [\"native.documentation.require_pod_sections\"]"));
        assert!(markdown.contains("| policy | `InputOutput::RequireCheckedOpen` |  | native_superset | native.io.unchecked_open_close |"));
    }

    #[test]
    fn perlcritic_profile_suggests_native_filter_ids_and_reports_unmapped_filters() {
        let report = classify_perlcritic_profile(
            r#"
include = TestingAndDebugging::RequireUseWarnings, Unknown::Policy
exclude = InputOutput::ProhibitTwoArgOpen Variables::ProhibitUnusedVariables
"#,
        );

        assert_eq!(report.suggested_config.include, vec!["native.testing.require_use_warnings"]);
        assert_eq!(report.suggested_config.unmapped_include, vec!["Unknown::Policy"]);
        assert_eq!(
            report.suggested_config.exclude,
            vec!["native.io.two_arg_open", "native.variables.unused_lexical"]
        );
        assert!(report.suggested_config.unmapped_exclude.is_empty());

        let markdown = render_perlcritic_compat_markdown(".perlcriticrc", &report);
        assert!(markdown.contains("include = [\"native.testing.require_use_warnings\"]"));
        assert!(markdown.contains(
            "exclude = [\"native.io.two_arg_open\", \"native.variables.unused_lexical\"]"
        ));
        assert!(markdown.contains("- include: `Unknown::Policy`"));
    }
}
