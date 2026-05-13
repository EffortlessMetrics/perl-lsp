//! Scenario 29 - Mojolicious hover provenance receipt.
//!
//! This receipt exercises the committed Mojolicious skeleton workspace and
//! records hover quality signals without changing provider behavior.
//!
//! Receipt signals:
//! - hover result availability and payload shape
//! - source/provenance/confidence/freshness labels when live compiler facts answer
//! - legacy hover coverage when the provider has no compiler-backed fact
//! - range shape when the provider includes one

use anyhow::{Context, Result};
use perl_lsp_ux_tests::{
    ScenarioConfig, UxCiTier, UxComponent, UxHarness, UxScenarioSkip, run_ux_scenario,
};
use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

const SCENARIO_FILE: &str = "ux_scenario_29_mojolicious_hover_provenance.rs";

#[derive(Debug)]
struct FixtureFile {
    relative_path: String,
    content: String,
}

#[derive(Debug)]
struct HoverProbe {
    name: &'static str,
    file: &'static str,
    line: u32,
    character: u32,
    expected_substrings: &'static [&'static str],
}

#[derive(Debug, Serialize)]
struct HoverProbeReport {
    name: &'static str,
    file: &'static str,
    line: u32,
    character: u32,
    has_result: bool,
    has_contents: bool,
    has_range: bool,
    markdown: String,
    expected_hits: Vec<String>,
    missing_expected_substrings: Vec<String>,
    source_label_hits: Vec<String>,
    confidence_label_hits: Vec<String>,
    legacy_hover_hits: Vec<String>,
    fallback_label_hits: Vec<String>,
}

fn binary_available() -> bool {
    perl_lsp_ux_tests::resolve_binary().is_ok()
}

fn missing_binary_skip() -> UxScenarioSkip {
    UxScenarioSkip::infra("PERL_LSP_BIN not set and target/debug/perl-lsp not found")
}

fn workspace_root() -> Result<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .context("CARGO_MANIFEST_DIR must be nested under the workspace root")
}

fn mojolicious_fixture_root() -> Result<PathBuf> {
    Ok(workspace_root()?.join("test_corpus").join("real_projects").join("mojolicious_skeleton"))
}

fn is_perl_source(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| matches!(extension, "pm" | "pl" | "t"))
}

fn collect_perl_files(root: &Path, dir: &Path, files: &mut Vec<FixtureFile>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let entry = entry.with_context(|| format!("reading an entry under {}", dir.display()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_perl_files(root, &path, files)?;
        } else if is_perl_source(&path) {
            let relative_path = path
                .strip_prefix(root)
                .with_context(|| format!("stripping fixture root from {}", path.display()))?
                .to_string_lossy()
                .replace('\\', "/");
            let content =
                fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
            files.push(FixtureFile { relative_path, content });
        }
    }
    Ok(())
}

fn load_mojolicious_fixture_files() -> Result<Vec<FixtureFile>> {
    let root = mojolicious_fixture_root()?;
    let mut files = Vec::new();
    collect_perl_files(&root, &root, &mut files)?;
    files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(files)
}

fn create_mojolicious_harness(files: &[FixtureFile]) -> Result<UxHarness> {
    let mut config = ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
        .env("PERL_LSP_WORKSPACE", "1");

    for file in files {
        config = config.with_file(&file.relative_path, &file.content);
    }

    UxHarness::new(config)
}

fn open_all_fixture_files(harness: &UxHarness, files: &[FixtureFile]) -> Result<()> {
    for file in files {
        harness.open_file(&file.relative_path, &file.content)?;
    }
    Ok(())
}

fn hover_markdown(hover: &Value) -> Option<String> {
    let contents = hover.get("contents")?;
    contents
        .as_str()
        .or_else(|| contents.get("value").and_then(Value::as_str))
        .map(ToOwned::to_owned)
}

fn has_hover_shape(hover: &Value) -> bool {
    hover.get("contents").is_some()
}

fn matching_needles(markdown: &str, needles: &[&str]) -> Vec<String> {
    needles
        .iter()
        .filter(|needle| markdown.contains(**needle))
        .map(|needle| (*needle).to_string())
        .collect()
}

fn missing_needles(markdown: &str, needles: &[&str]) -> Vec<String> {
    needles
        .iter()
        .filter(|needle| !markdown.contains(**needle))
        .map(|needle| (*needle).to_string())
        .collect()
}

fn run_probe(harness: &UxHarness, probe: &HoverProbe) -> Result<HoverProbeReport> {
    let hover = harness.hover(probe.file, probe.line, probe.character)?;
    let Some(hover) = hover else {
        return Ok(HoverProbeReport {
            name: probe.name,
            file: probe.file,
            line: probe.line,
            character: probe.character,
            has_result: false,
            has_contents: false,
            has_range: false,
            markdown: String::new(),
            expected_hits: Vec::new(),
            missing_expected_substrings: probe
                .expected_substrings
                .iter()
                .map(|needle| (*needle).to_string())
                .collect(),
            source_label_hits: Vec::new(),
            confidence_label_hits: Vec::new(),
            legacy_hover_hits: Vec::new(),
            fallback_label_hits: Vec::new(),
        });
    };

    anyhow::ensure!(
        has_hover_shape(&hover),
        "hover probe {} must return a Hover-shaped payload with contents: {hover:?}",
        probe.name
    );

    let markdown = hover_markdown(&hover).with_context(|| {
        format!("hover probe {} must include markdown or string contents", probe.name)
    })?;

    Ok(HoverProbeReport {
        name: probe.name,
        file: probe.file,
        line: probe.line,
        character: probe.character,
        has_result: true,
        has_contents: true,
        has_range: hover.get("range").is_some(),
        expected_hits: matching_needles(&markdown, probe.expected_substrings),
        missing_expected_substrings: missing_needles(&markdown, probe.expected_substrings),
        source_label_hits: matching_needles(
            &markdown,
            &["Source:", "compiler fact", "semantic fact", "framework adapter", "dynamic boundary"],
        ),
        confidence_label_hits: matching_needles(
            &markdown,
            &["high confidence", "medium confidence", "low confidence", "fresh"],
        ),
        legacy_hover_hits: matching_needles(&markdown, &["**Perl**:", "**Subroutine**"]),
        fallback_label_hits: matching_needles(&markdown, &["fallback", "legacy", "unknown"]),
        markdown,
    })
}

fn hover_probes() -> Vec<HoverProbe> {
    vec![
        HoverProbe {
            name: "imported_croak_symbol",
            file: "lib/Mojolicious.pm",
            line: 72,
            character: 5,
            expected_substrings: &[
                "croak",
                "Source:",
                "compiler fact",
                "import/export inference",
                "high confidence",
                "fresh",
            ],
        },
        HoverProbe {
            name: "accessor_declaration_legacy_hover",
            file: "lib/Mojolicious.pm",
            line: 20,
            character: 5,
            expected_substrings: &["commands"],
        },
        HoverProbe {
            name: "local_exact_method",
            file: "lib/Mojolicious.pm",
            line: 53,
            character: 6,
            expected_substrings: &["dispatch"],
        },
    ]
}

#[test]
fn scenario_29_mojolicious_hover_provenance_receipt() {
    run_ux_scenario(
        "mojolicious_hover_provenance",
        SCENARIO_FILE,
        "scenario_29_mojolicious_hover_provenance_receipt",
        UxCiTier::Pr,
        Some(UxComponent::Hover),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let fixture_files = load_mojolicious_fixture_files()?;
            recorder
                .check("mojolicious fixture has committed Perl files", !fixture_files.is_empty())?;

            let harness = create_mojolicious_harness(&fixture_files)?;
            open_all_fixture_files(&harness, &fixture_files)?;
            std::thread::sleep(Duration::from_millis(500));

            let probes = hover_probes();
            let mut reports = Vec::new();
            for probe in &probes {
                recorder.mark_request_start(probe.name);
                let report = run_probe(&harness, probe)?;
                if report.has_result {
                    recorder.mark_first_useful_result(probe.name);
                }
                eprintln!(
                    "hover_probe={} markdown={:?} expected_hits={:?} missing={:?}",
                    report.name,
                    report.markdown,
                    report.expected_hits,
                    report.missing_expected_substrings
                );
                reports.push(report);
            }

            let missing_expected_probe_count = reports
                .iter()
                .filter(|report| !report.missing_expected_substrings.is_empty())
                .count();
            let source_label_total: usize =
                reports.iter().map(|report| report.source_label_hits.len()).sum();
            let confidence_label_total: usize =
                reports.iter().map(|report| report.confidence_label_hits.len()).sum();
            let legacy_hover_total: usize =
                reports.iter().map(|report| report.legacy_hover_hits.len()).sum();
            let fallback_label_total: usize =
                reports.iter().map(|report| report.fallback_label_hits.len()).sum();

            let receipt = serde_json::json!({
                "schema_version": 1,
                "project": "mojolicious",
                "surface": "hover",
                "claim_boundary": "real-workspace hover quality receipt only; no provider behavior changed or promoted",
                "fixture_file_count": fixture_files.len(),
                "probe_count": reports.len(),
                "missing_expected_probe_count": missing_expected_probe_count,
                "source_label_total": source_label_total,
                "confidence_label_total": confidence_label_total,
                "legacy_hover_total": legacy_hover_total,
                "fallback_label_total": fallback_label_total,
                "reports": reports,
            });
            eprintln!(
                "mojolicious_hover_provenance_receipt={}",
                serde_json::to_string_pretty(&receipt)?
            );

            recorder.check("all hover probes produced reports", reports.len() == probes.len())?;
            recorder.check(
                "all hover probes returned non-null results",
                reports.iter().all(|report| report.has_result),
            )?;
            recorder.check(
                "all hover probes returned expected provenance or symbol text",
                missing_expected_probe_count == 0,
            )?;
            recorder.check(
                "at least one hover probe exposed source/provenance text",
                source_label_total > 0,
            )?;
            recorder.check(
                "at least one hover probe exposed confidence/freshness text",
                confidence_label_total > 0,
            )?;
            recorder.check(
                "at least one hover probe preserved legacy hover output",
                legacy_hover_total > 0,
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}
