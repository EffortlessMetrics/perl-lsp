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
    /// Per-option classifications in source order.
    pub options: Vec<PerltidyCompatOption>,
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
    /// Per-item classifications in source order.
    pub items: Vec<PerlcriticCompatItem>,
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
    PerltidyCompatReport {
        option_count: options.len(),
        supported_count: perltidy_count(&options, "supported"),
        approximated_count: perltidy_count(&options, "approximated"),
        unsupported_safe_count: perltidy_count(&options, "unsupported_safe"),
        external_only_count: perltidy_count(&options, "external_only"),
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
            "maps to native critic include/exclude rule filtering for native rule IDs",
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

        let markdown = render_perltidy_compat_markdown(".perltidyrc", &report);
        assert!(markdown.contains("# Native Format Perltidy Compatibility"));
        assert!(markdown.contains("| `-l` | 100 | supported | format.line_width |"));
    }

    #[test]
    fn perltidy_profile_keeps_unknown_options_external_only() {
        let report = classify_perltidy_profile("--unknown-style\n");

        assert_eq!(report.option_count, 1);
        assert_eq!(report.external_only_count, 1);
        assert_eq!(report.options[0].option, "--unknown-style");
        assert_eq!(report.options[0].classification, "external_only");
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

        let markdown = render_perlcritic_compat_markdown(".perlcriticrc", &report);
        assert!(markdown.contains("# Native Critic Perlcritic Compatibility"));
        assert!(markdown.contains("| policy | `InputOutput::RequireCheckedOpen` |  | native_superset | native.io.unchecked_open_close |"));
    }
}
