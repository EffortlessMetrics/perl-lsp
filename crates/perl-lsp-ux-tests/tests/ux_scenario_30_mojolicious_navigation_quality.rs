//! Scenario 30 - Mojolicious navigation quality receipt.
//!
//! This receipt exercises the committed Mojolicious skeleton workspace and
//! records current `textDocument/definition` and `textDocument/references`
//! quality signals without changing provider behavior.
//!
//! Receipt signals:
//! - definition/reference result counts for exact, imported, module, dynamic,
//!   and declaration-including probe shapes
//! - shape-valid LSP Location / LocationLink payloads
//! - expected workspace target hits where current live behavior can prove them
//! - fallback/empty counts for uncertain dynamic or unsupported surfaces

use anyhow::{Context, Result};
use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::missing_binary_skip;
use perl_lsp_ux_tests::{
    ProjectFixtureFile as FixtureFile, UxCiTier, UxComponent, UxHarness, create_fixture_harness,
    fixture_content, load_mojolicious_fixture_files, open_all_fixture_files, run_ux_scenario,
};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeSet;
use std::time::Duration;

const SCENARIO_FILE: &str = "ux_scenario_30_mojolicious_navigation_quality.rs";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum NavigationSurface {
    Definition,
    References,
}

#[derive(Debug)]
struct NavigationProbe {
    name: &'static str,
    category: &'static str,
    surface: NavigationSurface,
    file: &'static str,
    zero_based_line: usize,
    needle: &'static str,
    cursor_offset: usize,
    include_declaration: bool,
    expected_uri_suffixes: &'static [&'static str],
}

#[derive(Debug)]
struct ProbePosition {
    line: u32,
    character: u32,
}

#[derive(Debug, Serialize)]
struct NavigationProbeReport {
    name: &'static str,
    category: &'static str,
    surface: NavigationSurface,
    file: &'static str,
    zero_based_line: u32,
    character: u32,
    include_declaration: bool,
    result_count: usize,
    shape_valid_count: usize,
    invalid_shape_count: usize,
    result_uris: Vec<String>,
    result_lines: Vec<u64>,
    expected_target_hits: Vec<String>,
    missing_expected_uri_suffixes: Vec<String>,
    fallback_or_empty: bool,
}

fn resolve_probe_position(files: &[FixtureFile], probe: &NavigationProbe) -> Result<ProbePosition> {
    let content = fixture_content(files, probe.file)?;
    let line_text = content
        .lines()
        .nth(probe.zero_based_line)
        .with_context(|| format!("missing line {} in {}", probe.zero_based_line, probe.file))?;
    let needle_start = line_text.find(probe.needle).with_context(|| {
        format!("missing needle `{}` on {}:{}", probe.needle, probe.file, probe.zero_based_line)
    })?;
    let character =
        needle_start.checked_add(probe.cursor_offset).context("probe cursor offset overflow")?;
    Ok(ProbePosition {
        line: u32::try_from(probe.zero_based_line).context("probe line does not fit in u32")?,
        character: u32::try_from(character).context("probe character does not fit in u32")?,
    })
}

fn is_lsp_location_shape(entry: &Value) -> bool {
    let is_location = entry.get("uri").is_some() && entry.get("range").is_some();
    let is_location_link = entry.get("targetUri").is_some() && entry.get("targetRange").is_some();
    is_location || is_location_link
}

fn entry_uri(entry: &Value) -> Option<&str> {
    entry.get("uri").or_else(|| entry.get("targetUri")).and_then(Value::as_str)
}

fn entry_line(entry: &Value) -> Option<u64> {
    entry
        .get("range")
        .or_else(|| entry.get("targetRange"))
        .and_then(|range| range.get("start"))
        .and_then(|start| start.get("line"))
        .and_then(Value::as_u64)
}

fn expected_target_hits(uris: &[String], expected_suffixes: &[&str]) -> Vec<String> {
    expected_suffixes
        .iter()
        .copied()
        .filter(|suffix| uris.iter().any(|uri| uri.ends_with(suffix)))
        .map(str::to_string)
        .collect()
}

fn missing_expected_targets(uris: &[String], expected_suffixes: &[&str]) -> Vec<String> {
    expected_suffixes
        .iter()
        .copied()
        .filter(|suffix| !uris.iter().any(|uri| uri.ends_with(suffix)))
        .map(str::to_string)
        .collect()
}

fn run_probe(
    harness: &UxHarness,
    files: &[FixtureFile],
    probe: &NavigationProbe,
) -> Result<NavigationProbeReport> {
    let position = resolve_probe_position(files, probe)?;
    let results = match probe.surface {
        NavigationSurface::Definition => harness.definition_with_retry(
            probe.file,
            position.line,
            position.character,
            5,
            Duration::from_millis(200),
        )?,
        NavigationSurface::References => harness.references(
            probe.file,
            position.line,
            position.character,
            probe.include_declaration,
        )?,
    };

    let shape_valid_count = results.iter().filter(|entry| is_lsp_location_shape(entry)).count();
    let invalid_shape_count = results.len().saturating_sub(shape_valid_count);
    let result_uris = results.iter().filter_map(entry_uri).map(str::to_string).collect::<Vec<_>>();
    let result_lines = results.iter().filter_map(entry_line).collect::<Vec<_>>();
    let expected_hits = expected_target_hits(&result_uris, probe.expected_uri_suffixes);
    let missing_expected = missing_expected_targets(&result_uris, probe.expected_uri_suffixes);

    Ok(NavigationProbeReport {
        name: probe.name,
        category: probe.category,
        surface: probe.surface,
        file: probe.file,
        zero_based_line: position.line,
        character: position.character,
        include_declaration: probe.include_declaration,
        result_count: results.len(),
        shape_valid_count,
        invalid_shape_count,
        result_uris,
        result_lines,
        expected_target_hits: expected_hits,
        missing_expected_uri_suffixes: missing_expected,
        fallback_or_empty: results.is_empty(),
    })
}

fn navigation_probes() -> Vec<NavigationProbe> {
    vec![
        NavigationProbe {
            name: "definition_module_import_commands",
            category: "definition_module_resolution",
            surface: NavigationSurface::Definition,
            file: "lib/Mojolicious.pm",
            zero_based_line: 8,
            needle: "Mojolicious::Commands",
            cursor_offset: 3,
            include_declaration: false,
            expected_uri_suffixes: &["lib/Mojolicious/Commands.pm"],
        },
        NavigationProbe {
            name: "definition_local_startup_call",
            category: "definition_exact_local",
            surface: NavigationSurface::Definition,
            file: "lib/Mojolicious.pm",
            zero_based_line: 34,
            needle: "startup",
            cursor_offset: 1,
            include_declaration: false,
            expected_uri_suffixes: &["lib/Mojolicious.pm"],
        },
        NavigationProbe {
            name: "definition_imported_croak",
            category: "definition_imported_symbol",
            surface: NavigationSurface::Definition,
            file: "lib/Mojolicious.pm",
            zero_based_line: 72,
            needle: "croak",
            cursor_offset: 1,
            include_declaration: false,
            expected_uri_suffixes: &[],
        },
        NavigationProbe {
            name: "definition_dynamic_callable_shape",
            category: "definition_dynamic_boundary_shape",
            surface: NavigationSurface::Definition,
            file: "lib/Mojolicious/Controller.pm",
            zero_based_line: 37,
            needle: "->$cb",
            cursor_offset: 2,
            include_declaration: false,
            expected_uri_suffixes: &[],
        },
        NavigationProbe {
            name: "references_local_dispatch_without_declaration",
            category: "references_exact_local",
            surface: NavigationSurface::References,
            file: "lib/Mojolicious.pm",
            zero_based_line: 53,
            needle: "dispatch",
            cursor_offset: 1,
            include_declaration: false,
            expected_uri_suffixes: &["lib/Mojolicious.pm"],
        },
        NavigationProbe {
            name: "references_imported_croak_without_declaration",
            category: "references_imported_symbol",
            surface: NavigationSurface::References,
            file: "lib/Mojolicious.pm",
            zero_based_line: 72,
            needle: "croak",
            cursor_offset: 1,
            include_declaration: false,
            expected_uri_suffixes: &["lib/Mojolicious.pm"],
        },
        NavigationProbe {
            name: "references_dispatch_with_declaration_boundary",
            category: "references_include_declaration_boundary",
            surface: NavigationSurface::References,
            file: "lib/Mojolicious.pm",
            zero_based_line: 53,
            needle: "dispatch",
            cursor_offset: 1,
            include_declaration: true,
            expected_uri_suffixes: &["lib/Mojolicious.pm"],
        },
    ]
}

#[test]
fn scenario_30_mojolicious_navigation_quality_receipt() {
    run_ux_scenario(
        "mojolicious_navigation_quality",
        SCENARIO_FILE,
        "scenario_30_mojolicious_navigation_quality_receipt",
        UxCiTier::Pr,
        Some(UxComponent::GotoDefinition),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let fixture_files = load_mojolicious_fixture_files()?;
            recorder
                .check("mojolicious fixture has committed Perl files", !fixture_files.is_empty())?;

            let harness = create_fixture_harness(&fixture_files)?;
            open_all_fixture_files(&harness, &fixture_files)?;
            std::thread::sleep(Duration::from_millis(500));

            let probes = navigation_probes();
            let mut reports = Vec::new();

            for probe in &probes {
                recorder.mark_request_start(probe.name);
                let report = run_probe(&harness, &fixture_files, probe)?;
                if report.result_count > 0 {
                    recorder.mark_first_useful_result(probe.name);
                }
                eprintln!(
                    "navigation_probe={} surface={:?} category={} count={} hits={:?} uris={:?}",
                    report.name,
                    report.surface,
                    report.category,
                    report.result_count,
                    report.expected_target_hits,
                    report.result_uris
                );
                reports.push(report);
            }

            let categories = reports.iter().map(|report| report.category).collect::<BTreeSet<_>>();
            let definition_count = reports
                .iter()
                .filter(|report| report.surface == NavigationSurface::Definition)
                .count();
            let references_count = reports
                .iter()
                .filter(|report| report.surface == NavigationSurface::References)
                .count();
            let invalid_shape_total: usize =
                reports.iter().map(|report| report.invalid_shape_count).sum();
            let expected_hit_total: usize =
                reports.iter().map(|report| report.expected_target_hits.len()).sum();
            let fallback_or_empty_count =
                reports.iter().filter(|report| report.fallback_or_empty).count();
            let include_declaration_probe_count =
                reports.iter().filter(|report| report.include_declaration).count();

            let receipt = serde_json::json!({
                "schema_version": 1,
                "project": "mojolicious",
                "surface": "navigation",
                "claim_boundary": "real-workspace navigation quality receipt only; no provider behavior changed or promoted",
                "fixture_file_count": fixture_files.len(),
                "probe_count": reports.len(),
                "definition_probe_count": definition_count,
                "references_probe_count": references_count,
                "expected_hit_total": expected_hit_total,
                "fallback_or_empty_count": fallback_or_empty_count,
                "include_declaration_probe_count": include_declaration_probe_count,
                "invalid_shape_total": invalid_shape_total,
                "reports": reports,
            });
            eprintln!(
                "mojolicious_navigation_quality_receipt={}",
                serde_json::to_string_pretty(&receipt)?
            );

            recorder
                .check("all navigation probes produced reports", reports.len() == probes.len())?;
            recorder.check(
                "navigation probes covered all intended receipt categories",
                categories
                    == BTreeSet::from([
                        "definition_dynamic_boundary_shape",
                        "definition_exact_local",
                        "definition_imported_symbol",
                        "definition_module_resolution",
                        "references_exact_local",
                        "references_imported_symbol",
                        "references_include_declaration_boundary",
                    ]),
            )?;
            recorder.check("definition probes were exercised", definition_count >= 4)?;
            recorder.check("references probes were exercised", references_count >= 3)?;
            recorder.check(
                "all non-empty navigation results used valid LSP shapes",
                invalid_shape_total == 0,
            )?;
            recorder.check(
                "navigation receipt recorded at least one expected workspace target",
                expected_hit_total > 0,
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}
