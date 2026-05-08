//! Native formatter fixture receipts.

use chrono::{DateTime, Utc};
use color_eyre::eyre::{Context, Result, eyre};
use perl_lsp_rs_core::tooling::perltidy::{FormatConfig, NativeFormatter, PerlFormatter};
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
}
