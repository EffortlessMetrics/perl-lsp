//! Native formatter fixture receipts.

use chrono::{DateTime, Utc};
use color_eyre::eyre::{Context, Result, eyre};
use perl_lsp_rs_core::config::{ServerConfig, load_project_config};
use perl_lsp_rs_core::tooling::perltidy::{
    BracePlacement, ElsePlacement, FormatConfig, FormatterMode, KeywordSpacing, NativeFormatter,
    PerlFormatter, TrailingComma,
};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

const SCHEMA_VERSION: u32 = 1;

/// Options for `cargo xtask native-format check`.
pub struct NativeFormatCheckConfig {
    /// Directory containing native formatter fixtures.
    pub fixtures: PathBuf,
    /// Directory for JSON receipts.
    pub receipt_dir: PathBuf,
}

/// Options for `cargo xtask native-format corpus`.
pub struct NativeFormatCorpusConfig {
    /// Files or directories containing corpus Perl sources.
    pub roots: Vec<PathBuf>,
    /// Output JSON receipt path.
    pub receipt: PathBuf,
    /// Output markdown summary path.
    pub summary: PathBuf,
}

/// Options for `cargo xtask native-format perltidy-compat`.
pub struct NativeFormatPerltidyCompatConfig {
    /// Path to the `.perltidyrc`-style profile to classify.
    pub profile: PathBuf,
    /// Output JSON receipt path.
    pub receipt: PathBuf,
    /// Output markdown summary path.
    pub summary: PathBuf,
}

/// Options for `cargo xtask native-format config`.
pub struct NativeFormatConfigReceiptConfig {
    /// Workspace root used to discover `.perl-lsp.toml`.
    pub workspace_root: PathBuf,
    /// Output JSON receipt path.
    pub receipt: PathBuf,
    /// Output markdown summary path.
    pub summary: PathBuf,
}

#[derive(Debug, Serialize)]
struct NativeFormatFixturesReceipt {
    kind: &'static str,
    schema_version: u32,
    generated_at: DateTime<Utc>,
    commit: String,
    fixture_root: String,
    fixture_count: usize,
    passed_count: usize,
    failed_count: usize,
    changed_count: usize,
    expected_match_count: usize,
    idempotent_count: usize,
    parse_preserved_count: usize,
    diagnostics_count: usize,
    bailout_count: usize,
    expected_diagnostics_fixture_count: usize,
    expected_diagnostics_match_count: usize,
    fixtures: Vec<NativeFormatFixtureResult>,
    passed: bool,
}

#[derive(Debug, Serialize)]
struct NativeFormatCorpusReceipt {
    kind: &'static str,
    schema_version: u32,
    generated_at: DateTime<Utc>,
    commit: String,
    roots: Vec<String>,
    files_checked: usize,
    files_changed: usize,
    source_parse_clean_count: usize,
    formatted_parse_clean_count: usize,
    idempotence_passed_count: usize,
    parse_preserved_count: usize,
    literal_bailout_count: usize,
    unsupported_patterns_count: usize,
    unsupported_parse_clean_count: usize,
    parse_error_count: usize,
    diagnostics_count: usize,
    passed: bool,
    files: Vec<NativeFormatCorpusFileResult>,
}

#[derive(Debug, Serialize)]
struct NativeFormatCorpusFileResult {
    path: String,
    changed: bool,
    edit_count: usize,
    diagnostic_codes: Vec<String>,
    idempotent: bool,
    source_parse_clean: bool,
    formatted_parse_clean: bool,
    parse_preserved: bool,
    literal_bailout: bool,
    unsupported_pattern: bool,
    unsupported_parse_clean: bool,
    parse_error: bool,
    passed: bool,
}

#[derive(Debug, Serialize)]
struct NativeFormatPerltidyCompatReceipt {
    kind: &'static str,
    schema_version: u32,
    generated_at: DateTime<Utc>,
    commit: String,
    profile: String,
    option_count: usize,
    supported_count: usize,
    approximated_count: usize,
    unsupported_safe_count: usize,
    external_only_count: usize,
    options: Vec<PerltidyCompatOptionResult>,
}

#[derive(Debug, Serialize)]
struct NativeFormatConfigReceipt {
    kind: &'static str,
    schema_version: u32,
    generated_at: DateTime<Utc>,
    commit: String,
    workspace_root: String,
    config_source: &'static str,
    config_profile: Option<String>,
    formatting_enabled: bool,
    engine_selected: FormatterMode,
    external_adapter_requested: bool,
    line_width: u32,
    indent_width: u32,
    use_tabs: bool,
    brace_placement: BracePlacement,
    else_placement: ElsePlacement,
    keyword_spacing: KeywordSpacing,
    trailing_comma: TrailingComma,
    final_newline: &'static str,
}

#[derive(Debug, Serialize)]
struct PerltidyCompatOptionResult {
    option: String,
    value: Option<String>,
    classification: &'static str,
    native_field: Option<&'static str>,
    note: &'static str,
}

#[derive(Debug, Serialize)]
struct NativeFormatMetricReceipt {
    kind: &'static str,
    schema_version: u32,
    generated_at: DateTime<Utc>,
    commit: String,
    fixture_root: String,
    fixture_count: usize,
    passed_count: usize,
    rate: f64,
    passed: bool,
}

#[derive(Debug, Serialize)]
struct NativeFormatFixtureResult {
    path: String,
    expected_path: Option<String>,
    expected_diagnostics_path: Option<String>,
    changed: bool,
    edit_count: usize,
    diagnostic_codes: Vec<String>,
    expected_diagnostic_codes: Vec<String>,
    expected_matched: bool,
    expected_diagnostics_matched: bool,
    idempotent: bool,
    source_parse_clean: bool,
    formatted_parse_clean: bool,
    parse_preserved: bool,
    bailout: bool,
    passed: bool,
}

/// Check native formatter fixtures and write formatter receipts.
pub fn check(config: NativeFormatCheckConfig) -> Result<()> {
    let fixtures = collect_fixture_paths(&config.fixtures)?;
    if fixtures.is_empty() {
        return Err(eyre!("no native formatter fixtures found in {}", config.fixtures.display()));
    }

    let formatter = NativeFormatter::new();
    let format_config = FormatConfig::default();
    let mut results = Vec::new();

    for fixture in fixtures {
        results.push(check_fixture(&formatter, &format_config, &fixture)?);
    }

    let generated_at = Utc::now();
    let commit = current_commit();
    let passed_count = results.iter().filter(|result| result.passed).count();
    let failed_count = results.len() - passed_count;
    let fixture_count = results.len();
    let receipt = NativeFormatFixturesReceipt {
        kind: "native_format_fixtures",
        schema_version: SCHEMA_VERSION,
        generated_at,
        commit: commit.clone(),
        fixture_root: config.fixtures.display().to_string(),
        fixture_count,
        passed_count,
        failed_count,
        changed_count: results.iter().filter(|result| result.changed).count(),
        expected_match_count: results.iter().filter(|result| result.expected_matched).count(),
        idempotent_count: results.iter().filter(|result| result.idempotent).count(),
        parse_preserved_count: results.iter().filter(|result| result.parse_preserved).count(),
        diagnostics_count: results.iter().map(|result| result.diagnostic_codes.len()).sum(),
        bailout_count: results.iter().filter(|result| result.bailout).count(),
        expected_diagnostics_fixture_count: results
            .iter()
            .filter(|result| result.expected_diagnostics_path.is_some())
            .count(),
        expected_diagnostics_match_count: results
            .iter()
            .filter(|result| {
                result.expected_diagnostics_path.is_some() && result.expected_diagnostics_matched
            })
            .count(),
        passed: failed_count == 0,
        fixtures: results,
    };

    fs::create_dir_all(&config.receipt_dir)
        .wrap_err_with(|| format!("failed to create {}", config.receipt_dir.display()))?;
    write_json(&config.receipt_dir.join("native-format-fixtures.json"), &receipt)?;
    write_json(
        &config.receipt_dir.join("native-format-idempotence.json"),
        &metric_receipt(
            "native_format_idempotence",
            generated_at,
            &commit,
            &config.fixtures,
            fixture_count,
            receipt.idempotent_count,
        ),
    )?;
    write_json(
        &config.receipt_dir.join("native-format-parse-preservation.json"),
        &metric_receipt(
            "native_format_parse_preservation",
            generated_at,
            &commit,
            &config.fixtures,
            fixture_count,
            receipt.parse_preserved_count,
        ),
    )?;

    println!(
        "native formatter fixtures: {passed_count}/{fixture_count} passed; receipts: {}",
        config.receipt_dir.display()
    );

    if receipt.passed {
        Ok(())
    } else {
        Err(eyre!(
            "{failed_count} native formatter fixture(s) failed; see {}",
            config.receipt_dir.join("native-format-fixtures.json").display()
        ))
    }
}

/// Check corpus files and write native formatter corpus receipts.
pub fn corpus(config: NativeFormatCorpusConfig) -> Result<()> {
    let roots = if config.roots.is_empty() { default_corpus_roots() } else { config.roots };
    let files = collect_corpus_paths(&roots)?;
    if files.is_empty() {
        return Err(eyre!(
            "no native formatter corpus files found under {}",
            roots.iter().map(|root| root.display().to_string()).collect::<Vec<_>>().join(", ")
        ));
    }

    let formatter = NativeFormatter::new();
    let format_config = FormatConfig::default();
    let mut results = Vec::new();
    for file in files {
        results.push(check_corpus_file(&formatter, &format_config, &file)?);
    }

    let generated_at = Utc::now();
    let commit = current_commit();
    let passed = results.iter().all(|result| result.passed);
    let receipt = NativeFormatCorpusReceipt {
        kind: "native_format_corpus",
        schema_version: SCHEMA_VERSION,
        generated_at,
        commit,
        roots: roots.iter().map(|root| root.display().to_string()).collect(),
        files_checked: results.len(),
        files_changed: results.iter().filter(|result| result.changed).count(),
        source_parse_clean_count: results.iter().filter(|result| result.source_parse_clean).count(),
        formatted_parse_clean_count: results
            .iter()
            .filter(|result| result.formatted_parse_clean)
            .count(),
        idempotence_passed_count: results.iter().filter(|result| result.idempotent).count(),
        parse_preserved_count: results.iter().filter(|result| result.parse_preserved).count(),
        literal_bailout_count: results.iter().filter(|result| result.literal_bailout).count(),
        unsupported_patterns_count: results
            .iter()
            .filter(|result| result.unsupported_pattern)
            .count(),
        unsupported_parse_clean_count: results
            .iter()
            .filter(|result| result.unsupported_parse_clean)
            .count(),
        parse_error_count: results.iter().filter(|result| result.parse_error).count(),
        diagnostics_count: results.iter().map(|result| result.diagnostic_codes.len()).sum(),
        passed,
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
    write_corpus_summary(&config.summary, &receipt)?;

    println!(
        "native formatter corpus: {}/{} parse-preserved, {}/{} idempotent; receipt: {}",
        receipt.parse_preserved_count,
        receipt.files_checked,
        receipt.idempotence_passed_count,
        receipt.files_checked,
        config.receipt.display()
    );

    if receipt.passed {
        Ok(())
    } else {
        Err(eyre!(
            "native formatter corpus found {} file(s) without idempotence or parse preservation; see {}",
            receipt.files.iter().filter(|result| !result.passed).count(),
            config.receipt.display()
        ))
    }
}

/// Classify a perltidy profile against current native formatter compatibility.
pub fn perltidy_compat(config: NativeFormatPerltidyCompatConfig) -> Result<()> {
    let raw = fs::read_to_string(&config.profile)
        .wrap_err_with(|| format!("failed to read {}", config.profile.display()))?;
    let options = tokenize_perltidy_profile(&raw);
    let results = classify_perltidy_options(&options);
    let receipt = NativeFormatPerltidyCompatReceipt {
        kind: "native_format_perltidy_compat",
        schema_version: SCHEMA_VERSION,
        generated_at: Utc::now(),
        commit: current_commit(),
        profile: config.profile.display().to_string(),
        option_count: results.len(),
        supported_count: results
            .iter()
            .filter(|result| result.classification == "supported")
            .count(),
        approximated_count: results
            .iter()
            .filter(|result| result.classification == "approximated")
            .count(),
        unsupported_safe_count: results
            .iter()
            .filter(|result| result.classification == "unsupported_safe")
            .count(),
        external_only_count: results
            .iter()
            .filter(|result| result.classification == "external_only")
            .count(),
        options: results,
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
    write_perltidy_compat_summary(&config.summary, &receipt)?;

    println!(
        "native formatter perltidy compatibility: {} supported, {} approximated, {} external-only; receipt: {}",
        receipt.supported_count,
        receipt.approximated_count,
        receipt.external_only_count,
        config.receipt.display()
    );
    Ok(())
}

/// Write a receipt for the effective native formatter configuration surface.
pub fn config(config: NativeFormatConfigReceiptConfig) -> Result<()> {
    let mut server_config = ServerConfig::default();
    let project_config = load_project_config(&config.workspace_root)
        .map_err(|err| eyre!(err))
        .wrap_err_with(|| {
            format!(
                "failed to load project config from {}",
                config.workspace_root.join(".perl-lsp.toml").display()
            )
        })?;
    let config_source = if let Some(project_config) = project_config {
        project_config.apply_to_server_config(&mut server_config);
        "project"
    } else {
        "defaults"
    };

    let format_config = format_config_from_server_config(&server_config);
    let receipt = NativeFormatConfigReceipt {
        kind: "native_format_config",
        schema_version: SCHEMA_VERSION,
        generated_at: Utc::now(),
        commit: current_commit(),
        workspace_root: config.workspace_root.display().to_string(),
        config_source,
        config_profile: server_config.perltidy_profile.clone(),
        formatting_enabled: server_config.perltidy_enabled,
        engine_selected: format_config.mode,
        external_adapter_requested: format_config.mode == FormatterMode::ExternalLegacy,
        line_width: format_config.line_width,
        indent_width: format_config.indent_width,
        use_tabs: format_config.use_tabs,
        brace_placement: format_config.brace_placement,
        else_placement: format_config.else_placement,
        keyword_spacing: format_config.keyword_spacing,
        trailing_comma: format_config.trailing_comma,
        final_newline: "preserve",
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
    write_config_summary(&config.summary, &receipt)?;

    println!(
        "native formatter config: engine {:?}, source {}; receipt: {}",
        receipt.engine_selected,
        receipt.config_source,
        config.receipt.display()
    );
    Ok(())
}

fn format_config_from_server_config(server_config: &ServerConfig) -> FormatConfig {
    let mut config = FormatConfig {
        mode: server_config.formatting_engine,
        line_width: server_config
            .perltidy_maximum_line_length
            .unwrap_or_else(|| FormatConfig::default().line_width),
        indent_width: server_config
            .perltidy_indent_columns
            .unwrap_or_else(|| FormatConfig::default().indent_width),
        use_tabs: server_config.perltidy_tabs.unwrap_or_else(|| FormatConfig::default().use_tabs),
        ..FormatConfig::default()
    };
    if let Some(opening_brace_on_new_line) = server_config.perltidy_opening_brace_on_new_line {
        config.brace_placement = if opening_brace_on_new_line {
            BracePlacement::NextLine
        } else {
            BracePlacement::SameLine
        };
    }
    if let Some(cuddled_else) = server_config.perltidy_cuddled_else {
        config.else_placement =
            if cuddled_else { ElsePlacement::Cuddled } else { ElsePlacement::SeparateLine };
    }
    if let Some(space_after_keyword) = server_config.perltidy_space_after_keyword {
        config.keyword_spacing =
            if space_after_keyword { KeywordSpacing::Space } else { KeywordSpacing::Compact };
    }
    if let Some(add_trailing_commas) = server_config.perltidy_add_trailing_commas {
        config.trailing_comma = if add_trailing_commas {
            TrailingComma::AddWhenWrapped
        } else {
            TrailingComma::Preserve
        };
    }
    config
}

fn collect_fixture_paths(fixtures: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in WalkDir::new(fixtures).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let filename = path.file_name().and_then(|name| name.to_str()).unwrap_or_default();
        if path.extension().and_then(|ext| ext.to_str()) == Some("pl")
            && !filename.ends_with(".expected.pl")
        {
            paths.push(path.to_path_buf());
        }
    }
    paths.sort();
    Ok(paths)
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
        if option_requires_value(token)
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

fn option_requires_value(option: &str) -> bool {
    matches!(
        option,
        "-l" | "--maximum-line-length"
            | "-i"
            | "--indent-columns"
            | "-ci"
            | "--block-comment-indentation"
    )
}

fn classify_perltidy_options(
    options: &[(String, Option<String>)],
) -> Vec<PerltidyCompatOptionResult> {
    options.iter().map(|(option, value)| classify_perltidy_option(option, value.clone())).collect()
}

fn classify_perltidy_option(option: &str, value: Option<String>) -> PerltidyCompatOptionResult {
    match option {
        "-l" | "--maximum-line-length" => compat_result(
            option,
            value,
            "supported",
            Some("format.line_width"),
            "maps directly to the native formatter line width",
        ),
        "-i" | "--indent-columns" => compat_result(
            option,
            value,
            "supported",
            Some("format.indent_width"),
            "maps directly to native formatter indentation width",
        ),
        "-t" | "--tabs" | "-nt" | "--notabs" => compat_result(
            option,
            value,
            "supported",
            Some("format.use_tabs"),
            "maps directly to native formatter tab indentation",
        ),
        "-ce" | "--cuddled-else" | "-nce" | "--nocuddled-else" => compat_result(
            option,
            value,
            "supported",
            Some("format.else_placement"),
            "maps to native formatter else placement for supported simple block layouts",
        ),
        "-sok" | "--space-after-keyword" | "-nsok" | "--nospace-after-keyword" => compat_result(
            option,
            value,
            "supported",
            Some("format.keyword_spacing"),
            "maps to native formatter keyword spacing for supported simple control-flow headers",
        ),
        "-bl" | "--opening-brace-on-new-line" | "-bar" | "--opening-brace-always-on-right" => {
            compat_result(
                option,
                value,
                "supported",
                Some("format.brace_placement"),
                "maps to native formatter brace placement for supported simple block layouts",
            )
        }
        "-atc" | "--add-trailing-commas" | "-natc" | "--no-add-trailing-commas" => compat_result(
            option,
            value,
            "supported",
            Some("format.trailing_comma"),
            "maps to native formatter trailing comma policy for wrapped calls, lists, and hashes",
        ),
        "-ci" | "--block-comment-indentation" => compat_result(
            option,
            value,
            "external_only",
            None,
            "comment-aware native formatting is not yet configurable",
        ),
        "-val" | "--vertical-alignment" | "-nval" | "--novertical-alignment" => compat_result(
            option,
            value,
            "external_only",
            None,
            "native formatter intentionally avoids alignment policy today",
        ),
        "-pbp" | "--perl-best-practices" | "-gnu" | "--gnu-style" => compat_result(
            option,
            value,
            "approximated",
            None,
            "native formatter can map individual style settings but not full preset profiles yet",
        ),
        "-q" | "--quiet" | "-st" | "--standard-output" | "-se" | "--standard-error-output" => {
            compat_result(
                option,
                value,
                "unsupported_safe",
                None,
                "perltidy execution/output flag does not affect native formatting style",
            )
        }
        _ => compat_result(
            option,
            value,
            "external_only",
            None,
            "unknown style option is not applied by native formatter and may require external compatibility mode",
        ),
    }
}

fn compat_result(
    option: &str,
    value: Option<String>,
    classification: &'static str,
    native_field: Option<&'static str>,
    note: &'static str,
) -> PerltidyCompatOptionResult {
    PerltidyCompatOptionResult {
        option: option.to_string(),
        value,
        classification,
        native_field,
        note,
    }
}

fn default_corpus_roots() -> Vec<PathBuf> {
    vec![
        PathBuf::from("examples/perl"),
        PathBuf::from("tests/perl-corpus"),
        PathBuf::from("crates/perl-corpus/fixtures/parser_accuracy"),
    ]
}

fn collect_corpus_paths(roots: &[PathBuf]) -> Result<Vec<PathBuf>> {
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

fn check_corpus_file(
    formatter: &NativeFormatter,
    config: &FormatConfig,
    file: &Path,
) -> Result<NativeFormatCorpusFileResult> {
    let source =
        fs::read_to_string(file).wrap_err_with(|| format!("failed to read {}", file.display()))?;
    let result = formatter.format_document(&source, config);
    let second = formatter.format_document(&result.formatted, config);
    let diagnostic_codes =
        result.diagnostics.iter().map(|diagnostic| diagnostic.code.clone()).collect::<Vec<_>>();
    let source_parse_clean = parses_cleanly(&source);
    let formatted_parse_clean = parses_cleanly(&result.formatted);
    let idempotent = second.formatted == result.formatted && !second.changed;
    let parse_preserved = if source_parse_clean {
        formatted_parse_clean && !has_parse_preservation_diagnostic(&result)
    } else {
        result.formatted == source && !has_parse_preservation_diagnostic(&result)
    };
    let literal_bailout = !result.changed
        && result.edits.is_empty()
        && result.formatted == source
        && diagnostic_codes.iter().any(|code| code == "native.format.literal_preserve_region");
    let unsupported_pattern =
        diagnostic_codes.iter().any(|code| code != "native.format.literal_preserve_region");
    let parse_error = diagnostic_codes.iter().any(|code| code == "native.format.parse_error");
    let unsupported_parse_clean = unsupported_pattern && source_parse_clean;
    let passed = idempotent && parse_preserved;

    Ok(NativeFormatCorpusFileResult {
        path: file.display().to_string(),
        changed: result.changed,
        edit_count: result.edits.len(),
        diagnostic_codes,
        idempotent,
        source_parse_clean,
        formatted_parse_clean,
        parse_preserved,
        literal_bailout,
        unsupported_pattern,
        unsupported_parse_clean,
        parse_error,
        passed,
    })
}

fn check_fixture(
    formatter: &NativeFormatter,
    config: &FormatConfig,
    fixture: &Path,
) -> Result<NativeFormatFixtureResult> {
    let source = fs::read_to_string(fixture)
        .wrap_err_with(|| format!("failed to read {}", fixture.display()))?;
    let expected_path = expected_path_for(fixture);
    let expected = match expected_path.as_ref().filter(|path| path.exists()) {
        Some(path) => Some(
            fs::read_to_string(path)
                .wrap_err_with(|| format!("failed to read {}", path.display()))?,
        ),
        None => None,
    };
    let expected_diagnostics_path = expected_diagnostics_path_for(fixture);
    let expected_diagnostic_codes =
        match expected_diagnostics_path.as_ref().filter(|path| path.exists()) {
            Some(path) => read_expected_diagnostic_codes(path)?,
            None => Vec::new(),
        };

    let result = formatter.format_document(&source, config);
    let second = formatter.format_document(&result.formatted, config);
    let expected_matched = expected.as_ref().is_none_or(|expected| expected == &result.formatted);
    let diagnostic_codes =
        result.diagnostics.iter().map(|diagnostic| diagnostic.code.clone()).collect::<Vec<_>>();
    let second_diagnostic_codes =
        second.diagnostics.iter().map(|diagnostic| diagnostic.code.clone()).collect::<Vec<_>>();
    let expected_diagnostics_matched = diagnostic_codes == expected_diagnostic_codes;
    let expects_diagnostics = expected_diagnostics_path.as_ref().is_some_and(|path| path.exists());
    let source_parse_clean = parses_cleanly(&source);
    let formatted_parse_clean = parses_cleanly(&result.formatted);
    let idempotent = second.formatted == result.formatted
        && !second.changed
        && if expects_diagnostics {
            second_diagnostic_codes == diagnostic_codes
        } else {
            second.diagnostics.is_empty()
        };
    let bailout = expects_diagnostics
        && !result.changed
        && result.edits.is_empty()
        && result.formatted == source;
    let parse_preserved = if expects_diagnostics {
        result.formatted == source && !has_parse_preservation_diagnostic(&result)
    } else {
        source_parse_clean && formatted_parse_clean && !has_parse_preservation_diagnostic(&result)
    };
    let passed = expected_matched
        && idempotent
        && parse_preserved
        && expected_diagnostics_matched
        && if expects_diagnostics {
            bailout
        } else {
            source_parse_clean && formatted_parse_clean && diagnostic_codes.is_empty()
        };

    Ok(NativeFormatFixtureResult {
        path: fixture.display().to_string(),
        expected_path: expected_path
            .filter(|path| path.exists())
            .map(|path| path.display().to_string()),
        expected_diagnostics_path: expected_diagnostics_path
            .filter(|path| path.exists())
            .map(|path| path.display().to_string()),
        changed: result.changed,
        edit_count: result.edits.len(),
        diagnostic_codes,
        expected_diagnostic_codes,
        expected_matched,
        expected_diagnostics_matched,
        idempotent,
        source_parse_clean,
        formatted_parse_clean,
        parse_preserved,
        bailout,
        passed,
    })
}

fn expected_path_for(path: &Path) -> Option<PathBuf> {
    let stem = path.file_stem()?.to_str()?;
    Some(path.with_file_name(format!("{stem}.expected.pl")))
}

fn expected_diagnostics_path_for(path: &Path) -> Option<PathBuf> {
    let stem = path.file_stem()?.to_str()?;
    Some(path.with_file_name(format!("{stem}.expected-diagnostics.txt")))
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

fn parses_cleanly(source: &str) -> bool {
    let mut parser = perl_parser_core::Parser::new(source);
    let output = parser.parse_with_recovery();
    !output.terminated_early && output.diagnostics.is_empty()
}

fn has_parse_preservation_diagnostic(
    result: &perl_lsp_rs_core::tooling::perltidy::FormatResult,
) -> bool {
    result
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "native.format.parse_preservation")
}

fn metric_receipt(
    kind: &'static str,
    generated_at: DateTime<Utc>,
    commit: &str,
    fixture_root: &Path,
    fixture_count: usize,
    passed_count: usize,
) -> NativeFormatMetricReceipt {
    NativeFormatMetricReceipt {
        kind,
        schema_version: SCHEMA_VERSION,
        generated_at,
        commit: commit.to_string(),
        fixture_root: fixture_root.display().to_string(),
        fixture_count,
        passed_count,
        rate: if fixture_count == 0 { 0.0 } else { passed_count as f64 / fixture_count as f64 },
        passed: fixture_count == passed_count,
    }
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let json = serde_json::to_string_pretty(value)?;
    fs::write(path, format!("{json}\n"))
        .wrap_err_with(|| format!("failed to write {}", path.display()))
}

fn write_corpus_summary(path: &Path, receipt: &NativeFormatCorpusReceipt) -> Result<()> {
    let mut markdown = String::new();
    markdown.push_str("# Native Format Corpus\n\n");
    markdown.push_str(&format!("- Commit: `{}`\n", receipt.commit));
    markdown.push_str(&format!("- Files checked: {}\n", receipt.files_checked));
    markdown.push_str(&format!("- Files changed: {}\n", receipt.files_changed));
    markdown.push_str(&format!(
        "- Idempotence: {}/{}\n",
        receipt.idempotence_passed_count, receipt.files_checked
    ));
    markdown.push_str(&format!(
        "- Parse preservation: {}/{}\n",
        receipt.parse_preserved_count, receipt.files_checked
    ));
    markdown.push_str(&format!("- Literal bailouts: {}\n", receipt.literal_bailout_count));
    markdown
        .push_str(&format!("- Unsupported diagnostics: {}\n", receipt.unsupported_patterns_count));
    markdown.push_str(&format!(
        "- Unsupported diagnostics on parse-clean files: {}\n",
        receipt.unsupported_parse_clean_count
    ));
    markdown.push_str(&format!("- Parse-error diagnostics: {}\n", receipt.parse_error_count));
    markdown.push_str(&format!("- Passed: {}\n\n", receipt.passed));
    markdown.push_str("| File | Changed | Idempotent | Parse Preserved | Diagnostics |\n");
    markdown.push_str("| --- | --- | --- | --- | --- |\n");
    for file in &receipt.files {
        let diagnostics = if file.diagnostic_codes.is_empty() {
            String::new()
        } else {
            file.diagnostic_codes.join(", ")
        };
        markdown.push_str(&format!(
            "| `{}` | {} | {} | {} | {} |\n",
            file.path, file.changed, file.idempotent, file.parse_preserved, diagnostics
        ));
    }
    let unsupported_files =
        receipt.files.iter().filter(|file| file.unsupported_pattern).collect::<Vec<_>>();
    if !unsupported_files.is_empty() {
        markdown.push_str("\n## Unsupported Diagnostics\n\n");
        markdown.push_str("| File | Source Parse Clean | Parse Error | Diagnostics |\n");
        markdown.push_str("| --- | --- | --- | --- |\n");
        for file in unsupported_files {
            markdown.push_str(&format!(
                "| `{}` | {} | {} | {} |\n",
                file.path,
                file.source_parse_clean,
                file.parse_error,
                file.diagnostic_codes.join(", ")
            ));
        }
    }
    fs::write(path, markdown).wrap_err_with(|| format!("failed to write {}", path.display()))
}

fn write_perltidy_compat_summary(
    path: &Path,
    receipt: &NativeFormatPerltidyCompatReceipt,
) -> Result<()> {
    let mut markdown = String::new();
    markdown.push_str("# Native Format Perltidy Compatibility\n\n");
    markdown.push_str(&format!("- Profile: `{}`\n", receipt.profile));
    markdown.push_str(&format!("- Options checked: {}\n", receipt.option_count));
    markdown.push_str(&format!("- Supported: {}\n", receipt.supported_count));
    markdown.push_str(&format!("- Approximated: {}\n", receipt.approximated_count));
    markdown.push_str(&format!("- Unsupported safe: {}\n", receipt.unsupported_safe_count));
    markdown.push_str(&format!("- External-only: {}\n\n", receipt.external_only_count));
    markdown.push_str("| Option | Value | Classification | Native field | Note |\n");
    markdown.push_str("| --- | --- | --- | --- | --- |\n");
    for option in &receipt.options {
        markdown.push_str(&format!(
            "| `{}` | {} | {} | {} | {} |\n",
            option.option,
            option.value.as_deref().unwrap_or(""),
            option.classification,
            option.native_field.unwrap_or(""),
            option.note
        ));
    }
    fs::write(path, markdown).wrap_err_with(|| format!("failed to write {}", path.display()))
}

fn write_config_summary(path: &Path, receipt: &NativeFormatConfigReceipt) -> Result<()> {
    let mut markdown = String::new();
    markdown.push_str("# Native Format Config\n\n");
    markdown.push_str(&format!("- Workspace root: `{}`\n", receipt.workspace_root));
    markdown.push_str(&format!("- Config source: `{}`\n", receipt.config_source));
    markdown.push_str(&format!("- Formatting enabled: `{}`\n", receipt.formatting_enabled));
    markdown.push_str(&format!("- Engine selected: `{:?}`\n", receipt.engine_selected));
    markdown.push_str(&format!(
        "- External adapter requested: `{}`\n\n",
        receipt.external_adapter_requested
    ));
    markdown.push_str("| Field | Value |\n");
    markdown.push_str("| --- | --- |\n");
    markdown.push_str(&format!(
        "| config_profile | {} |\n",
        receipt.config_profile.as_deref().unwrap_or("")
    ));
    markdown.push_str(&format!("| line_width | {} |\n", receipt.line_width));
    markdown.push_str(&format!("| indent_width | {} |\n", receipt.indent_width));
    markdown.push_str(&format!("| use_tabs | {} |\n", receipt.use_tabs));
    markdown.push_str(&format!("| brace_placement | {:?} |\n", receipt.brace_placement));
    markdown.push_str(&format!("| else_placement | {:?} |\n", receipt.else_placement));
    markdown.push_str(&format!("| keyword_spacing | {:?} |\n", receipt.keyword_spacing));
    markdown.push_str(&format!("| trailing_comma | {:?} |\n", receipt.trailing_comma));
    markdown.push_str(&format!("| final_newline | {} |\n", receipt.final_newline));
    fs::write(path, markdown).wrap_err_with(|| format!("failed to write {}", path.display()))
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
    fn native_format_check_writes_fixture_receipts() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let fixtures = temp.path().join("fixtures");
        let receipts = temp.path().join("receipts");
        fs::create_dir_all(&fixtures)?;
        fs::write(fixtures.join("simple.pl"), "my$x=1;\n")?;
        fs::write(fixtures.join("simple.expected.pl"), "my $x = 1;\n")?;

        check(NativeFormatCheckConfig { fixtures, receipt_dir: receipts.clone() })?;

        let fixture_receipt: Value = serde_json::from_str(&fs::read_to_string(
            receipts.join("native-format-fixtures.json"),
        )?)?;
        assert_eq!(fixture_receipt["kind"], "native_format_fixtures");
        assert_eq!(fixture_receipt["fixture_count"], 1);
        assert_eq!(fixture_receipt["passed"], true);

        let idempotence: Value = serde_json::from_str(&fs::read_to_string(
            receipts.join("native-format-idempotence.json"),
        )?)?;
        assert_eq!(idempotence["kind"], "native_format_idempotence");
        assert_eq!(idempotence["passed"], true);

        Ok(())
    }

    #[test]
    fn native_format_check_accepts_expected_literal_preserve_bailouts() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let fixtures = temp.path().join("fixtures");
        let receipts = temp.path().join("receipts");
        fs::create_dir_all(&fixtures)?;
        let source = "=pod\n\n=head1 NAME\n\n=cut\n\nmy $x = 1;\n";
        fs::write(fixtures.join("pod.pl"), source)?;
        fs::write(fixtures.join("pod.expected.pl"), source)?;
        fs::write(
            fixtures.join("pod.expected-diagnostics.txt"),
            "native.format.literal_preserve_region\n",
        )?;

        check(NativeFormatCheckConfig { fixtures, receipt_dir: receipts.clone() })?;

        let fixture_receipt: Value = serde_json::from_str(&fs::read_to_string(
            receipts.join("native-format-fixtures.json"),
        )?)?;
        assert_eq!(fixture_receipt["fixture_count"], 1);
        assert_eq!(fixture_receipt["passed"], true);
        assert_eq!(fixture_receipt["bailout_count"], 1);
        assert_eq!(fixture_receipt["expected_diagnostics_fixture_count"], 1);
        assert_eq!(fixture_receipt["expected_diagnostics_match_count"], 1);
        assert_eq!(
            fixture_receipt["fixtures"][0]["diagnostic_codes"][0],
            "native.format.literal_preserve_region"
        );
        assert_eq!(fixture_receipt["fixtures"][0]["bailout"], true);

        Ok(())
    }

    #[test]
    fn native_format_corpus_writes_receipt_and_summary() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let corpus_root = temp.path().join("corpus");
        let receipts = temp.path().join("receipts");
        fs::create_dir_all(&corpus_root)?;
        fs::write(corpus_root.join("simple.pl"), "my$x=1;\n")?;
        fs::write(corpus_root.join("pod.pl"), "=pod\n\n=head1 NAME\n\n=cut\n\nmy $x = 1;\n")?;

        corpus(NativeFormatCorpusConfig {
            roots: vec![corpus_root],
            receipt: receipts.join("native-format-corpus.json"),
            summary: receipts.join("native-format-corpus-summary.md"),
        })?;

        let receipt: Value =
            serde_json::from_str(&fs::read_to_string(receipts.join("native-format-corpus.json"))?)?;
        assert_eq!(receipt["kind"], "native_format_corpus");
        assert_eq!(receipt["files_checked"], 2);
        assert_eq!(receipt["files_changed"], 1);
        assert_eq!(receipt["idempotence_passed_count"], 2);
        assert_eq!(receipt["parse_preserved_count"], 2);
        assert_eq!(receipt["literal_bailout_count"], 1);
        assert_eq!(receipt["unsupported_parse_clean_count"], 0);
        assert_eq!(receipt["parse_error_count"], 0);
        assert_eq!(receipt["passed"], true);

        let summary = fs::read_to_string(receipts.join("native-format-corpus-summary.md"))?;
        assert!(summary.contains("# Native Format Corpus"));
        assert!(summary.contains("Files checked: 2"));
        assert!(summary.contains("Literal bailouts: 1"));
        assert!(summary.contains("Unsupported diagnostics on parse-clean files: 0"));
        assert!(summary.contains("Parse-error diagnostics: 0"));

        Ok(())
    }

    #[test]
    fn native_format_perltidy_compat_classifies_common_options() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let profile = temp.path().join(".perltidyrc");
        let receipts = temp.path().join("receipts");
        fs::write(&profile, "# common profile\n-l=100\n-i 2\n-nt\n-ce\n-nsok\n-q\n-atc\n-bl\n")?;

        perltidy_compat(NativeFormatPerltidyCompatConfig {
            profile,
            receipt: receipts.join("native-format-perltidy-compat.json"),
            summary: receipts.join("native-format-perltidy-compat.md"),
        })?;

        let receipt: Value = serde_json::from_str(&fs::read_to_string(
            receipts.join("native-format-perltidy-compat.json"),
        )?)?;
        assert_eq!(receipt["kind"], "native_format_perltidy_compat");
        assert_eq!(receipt["option_count"], 8);
        assert_eq!(receipt["supported_count"], 7);
        assert_eq!(receipt["approximated_count"], 0);
        assert_eq!(receipt["external_only_count"], 0);
        assert_eq!(receipt["unsupported_safe_count"], 1);
        assert_eq!(receipt["options"][0]["native_field"], "format.line_width");
        assert_eq!(receipt["options"][1]["value"], "2");
        assert_eq!(receipt["options"][3]["classification"], "supported");
        assert_eq!(receipt["options"][3]["native_field"], "format.else_placement");
        assert_eq!(receipt["options"][4]["classification"], "supported");
        assert_eq!(receipt["options"][4]["native_field"], "format.keyword_spacing");
        assert_eq!(receipt["options"][5]["classification"], "unsupported_safe");
        assert_eq!(receipt["options"][6]["classification"], "supported");
        assert_eq!(receipt["options"][6]["native_field"], "format.trailing_comma");
        assert_eq!(receipt["options"][7]["classification"], "supported");
        assert_eq!(receipt["options"][7]["native_field"], "format.brace_placement");

        let summary = fs::read_to_string(receipts.join("native-format-perltidy-compat.md"))?;
        assert!(summary.contains("# Native Format Perltidy Compatibility"));
        assert!(summary.contains("| `-l` | 100 | supported | format.line_width |"));

        Ok(())
    }

    #[test]
    fn native_format_perltidy_compat_keeps_unknown_options_external_only() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let profile = temp.path().join(".perltidyrc");
        let receipts = temp.path().join("receipts");
        fs::write(&profile, "--unknown-style\n")?;

        perltidy_compat(NativeFormatPerltidyCompatConfig {
            profile,
            receipt: receipts.join("native-format-perltidy-compat.json"),
            summary: receipts.join("native-format-perltidy-compat.md"),
        })?;

        let receipt: Value = serde_json::from_str(&fs::read_to_string(
            receipts.join("native-format-perltidy-compat.json"),
        )?)?;
        assert_eq!(receipt["option_count"], 1);
        assert_eq!(receipt["external_only_count"], 1);
        assert_eq!(receipt["options"][0]["option"], "--unknown-style");
        assert_eq!(receipt["options"][0]["classification"], "external_only");

        let summary = fs::read_to_string(receipts.join("native-format-perltidy-compat.md"))?;
        assert!(summary.contains("| `--unknown-style` |  | external_only |"));

        Ok(())
    }

    #[test]
    fn native_format_config_writes_default_receipt_and_summary() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let receipts = temp.path().join("receipts");

        config(NativeFormatConfigReceiptConfig {
            workspace_root: temp.path().to_path_buf(),
            receipt: receipts.join("native-format-config.json"),
            summary: receipts.join("native-format-config.md"),
        })?;

        let receipt: Value =
            serde_json::from_str(&fs::read_to_string(receipts.join("native-format-config.json"))?)?;
        assert_eq!(receipt["kind"], "native_format_config");
        assert_eq!(receipt["config_source"], "defaults");
        assert_eq!(receipt["formatting_enabled"], true);
        assert_eq!(receipt["engine_selected"], "native");
        assert_eq!(receipt["external_adapter_requested"], false);
        assert_eq!(receipt["line_width"], 80);
        assert_eq!(receipt["indent_width"], 4);
        assert_eq!(receipt["use_tabs"], false);
        assert_eq!(receipt["brace_placement"], "same-line");
        assert_eq!(receipt["else_placement"], "cuddled");
        assert_eq!(receipt["keyword_spacing"], "space");
        assert_eq!(receipt["trailing_comma"], "preserve");
        assert_eq!(receipt["final_newline"], "preserve");

        let summary = fs::read_to_string(receipts.join("native-format-config.md"))?;
        assert!(summary.contains("# Native Format Config"));
        assert!(summary.contains("Config source: `defaults`"));
        assert!(summary.contains("| line_width | 80 |"));

        Ok(())
    }

    #[test]
    fn native_format_config_applies_project_formatting_policy() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let receipts = temp.path().join("receipts");
        fs::write(
            temp.path().join(".perl-lsp.toml"),
            r#"
[formatting]
enabled = true
engine = "native"
perltidy_profile = ".perltidyrc"
perltidy_maximum_line_length = 88
perltidy_indent_columns = 2
perltidy_tabs = true
perltidy_opening_brace_on_new_line = true
perltidy_cuddled_else = false
perltidy_space_after_keyword = false
perltidy_add_trailing_commas = true
"#,
        )?;

        config(NativeFormatConfigReceiptConfig {
            workspace_root: temp.path().to_path_buf(),
            receipt: receipts.join("native-format-config.json"),
            summary: receipts.join("native-format-config.md"),
        })?;

        let receipt: Value =
            serde_json::from_str(&fs::read_to_string(receipts.join("native-format-config.json"))?)?;
        assert_eq!(receipt["config_source"], "project");
        assert_eq!(receipt["config_profile"], ".perltidyrc");
        assert_eq!(receipt["engine_selected"], "native");
        assert_eq!(receipt["external_adapter_requested"], false);
        assert_eq!(receipt["line_width"], 88);
        assert_eq!(receipt["indent_width"], 2);
        assert_eq!(receipt["use_tabs"], true);
        assert_eq!(receipt["brace_placement"], "next-line");
        assert_eq!(receipt["else_placement"], "separate-line");
        assert_eq!(receipt["keyword_spacing"], "compact");
        assert_eq!(receipt["trailing_comma"], "add-when-wrapped");

        Ok(())
    }

    #[test]
    fn native_format_config_marks_external_adapter_requests() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let receipts = temp.path().join("receipts");
        fs::write(
            temp.path().join(".perl-lsp.toml"),
            r#"
[formatting]
engine = "external-perltidy"
"#,
        )?;

        config(NativeFormatConfigReceiptConfig {
            workspace_root: temp.path().to_path_buf(),
            receipt: receipts.join("native-format-config.json"),
            summary: receipts.join("native-format-config.md"),
        })?;

        let receipt: Value =
            serde_json::from_str(&fs::read_to_string(receipts.join("native-format-config.json"))?)?;
        assert_eq!(receipt["config_source"], "project");
        assert_eq!(receipt["engine_selected"], "external-legacy");
        assert_eq!(receipt["external_adapter_requested"], true);

        Ok(())
    }
}
