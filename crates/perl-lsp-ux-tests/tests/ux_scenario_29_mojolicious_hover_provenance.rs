//! Scenario 29 - Mojolicious hover provenance receipt.
//!
//! This receipt exercises hover over the committed Mojolicious skeleton
//! workspace and records provenance-quality signals without changing provider
//! behavior.
//!
//! Receipt signals:
//! - hover result shape and null/fallback state per probe
//! - expected text hits for exact, imported, generated, dynamic-shaped, and
//!   fallback/module-resolution surfaces
//! - source/provenance/confidence/freshness label coverage where the provider
//!   exposes compiler-backed hover explanations
//! - generated, dynamic-boundary, and fallback label coverage when present

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

const SCENARIO_FILE: &str = "ux_scenario_29_mojolicious_hover_provenance.rs";

#[derive(Debug)]
struct HoverProbe {
    name: &'static str,
    category: &'static str,
    file: &'static str,
    line: usize,
    needle: &'static str,
    cursor_offset: usize,
    expected_substrings: &'static [&'static str],
}

#[derive(Debug)]
struct ProbePosition {
    line: u32,
    character: u32,
}

#[derive(Debug)]
struct HoverContent {
    shape: &'static str,
    text: String,
}

#[derive(Debug, Serialize)]
struct HoverProbeReport {
    name: &'static str,
    category: &'static str,
    file: &'static str,
    line: u32,
    character: u32,
    result_state: &'static str,
    content_shape: Option<&'static str>,
    content_len: usize,
    content_excerpt: String,
    expected_hits: Vec<String>,
    missing_expected_substrings: Vec<String>,
    source_label_hits: Vec<String>,
    provenance_label_hits: Vec<String>,
    confidence_label_hits: Vec<String>,
    freshness_label_hits: Vec<String>,
    generated_label_hits: Vec<String>,
    dynamic_boundary_label_hits: Vec<String>,
    fallback_label_hits: Vec<String>,
}

fn resolve_probe_position(files: &[FixtureFile], probe: &HoverProbe) -> Result<ProbePosition> {
    let content = fixture_content(files, probe.file)?;
    let line_text = content
        .lines()
        .nth(probe.line)
        .with_context(|| format!("missing line {} in {}", probe.line, probe.file))?;
    let needle_start = line_text.find(probe.needle).with_context(|| {
        format!("missing needle `{}` on {}:{}", probe.needle, probe.file, probe.line)
    })?;
    let character =
        needle_start.checked_add(probe.cursor_offset).context("probe cursor offset overflow")?;
    Ok(ProbePosition {
        line: u32::try_from(probe.line).context("probe line does not fit in u32")?,
        character: u32::try_from(character).context("probe character does not fit in u32")?,
    })
}

fn hover_contents(hover: &Value) -> Result<HoverContent> {
    let contents = hover.get("contents").context("hover result missing contents")?;
    if let Some(text) = contents.as_str() {
        return Ok(HoverContent { shape: "marked_string", text: text.to_string() });
    }
    if let Some(value) = contents.get("value").and_then(Value::as_str) {
        return Ok(HoverContent { shape: "markup_content", text: value.to_string() });
    }
    if let Some(entries) = contents.as_array() {
        let mut text_parts = Vec::new();
        for entry in entries {
            if let Some(text) = entry.as_str() {
                text_parts.push(text.to_string());
            } else if let Some(value) = entry.get("value").and_then(Value::as_str) {
                text_parts.push(value.to_string());
            }
        }
        return Ok(HoverContent { shape: "marked_string_array", text: text_parts.join("\n") });
    }
    anyhow::bail!("hover contents must be MarkupContent, MarkedString, or array: {contents:?}")
}

fn matching_needles(text: &str, needles: &[&str]) -> Vec<String> {
    needles.iter().copied().filter(|needle| text.contains(needle)).map(str::to_string).collect()
}

fn missing_needles(text: &str, needles: &[&str]) -> Vec<String> {
    needles.iter().copied().filter(|needle| !text.contains(needle)).map(str::to_string).collect()
}

fn content_excerpt(text: &str) -> String {
    const MAX_CHARS: usize = 240;
    let mut excerpt = text.chars().take(MAX_CHARS).collect::<String>();
    if text.chars().count() > MAX_CHARS {
        excerpt.push_str("...");
    }
    excerpt
}

fn run_probe(
    harness: &UxHarness,
    files: &[FixtureFile],
    probe: &HoverProbe,
) -> Result<HoverProbeReport> {
    let position = resolve_probe_position(files, probe)?;
    let result = harness.hover(probe.file, position.line, position.character)?;

    let Some(hover) = result else {
        return Ok(HoverProbeReport {
            name: probe.name,
            category: probe.category,
            file: probe.file,
            line: position.line,
            character: position.character,
            result_state: "null",
            content_shape: None,
            content_len: 0,
            content_excerpt: String::new(),
            expected_hits: Vec::new(),
            missing_expected_substrings: probe
                .expected_substrings
                .iter()
                .copied()
                .map(str::to_string)
                .collect(),
            source_label_hits: Vec::new(),
            provenance_label_hits: Vec::new(),
            confidence_label_hits: Vec::new(),
            freshness_label_hits: Vec::new(),
            generated_label_hits: Vec::new(),
            dynamic_boundary_label_hits: Vec::new(),
            fallback_label_hits: Vec::new(),
        });
    };

    let content = hover_contents(&hover)?;
    let text = content.text;
    Ok(HoverProbeReport {
        name: probe.name,
        category: probe.category,
        file: probe.file,
        line: position.line,
        character: position.character,
        result_state: "content",
        content_shape: Some(content.shape),
        content_len: text.len(),
        content_excerpt: content_excerpt(&text),
        expected_hits: matching_needles(&text, probe.expected_substrings),
        missing_expected_substrings: missing_needles(&text, probe.expected_substrings),
        source_label_hits: matching_needles(
            &text,
            &["Source:", "compiler fact", "semantic fact", "framework adapter"],
        ),
        provenance_label_hits: matching_needles(
            &text,
            &["exact AST", "import/export inference", "framework synthesis", "dynamic boundary"],
        ),
        confidence_label_hits: matching_needles(
            &text,
            &["high confidence", "medium confidence", "low confidence"],
        ),
        freshness_label_hits: matching_needles(&text, &["fresh", "stale", "not applicable"]),
        generated_label_hits: matching_needles(&text, &["Generated by", "generated"]),
        dynamic_boundary_label_hits: matching_needles(&text, &["Dynamic boundary", "dynamic"]),
        fallback_label_hits: matching_needles(&text, &["fallback", "SearchFallback"]),
    })
}

fn hover_probes() -> Vec<HoverProbe> {
    vec![
        HoverProbe {
            name: "exact_sub_start",
            category: "exact_syntax",
            file: "lib/Mojolicious.pm",
            line: 88,
            needle: "start",
            cursor_offset: 1,
            expected_substrings: &["start"],
        },
        HoverProbe {
            name: "imported_croak_call",
            category: "imported_symbol",
            file: "lib/Mojolicious.pm",
            line: 72,
            needle: "croak",
            cursor_offset: 1,
            expected_substrings: &["croak"],
        },
        HoverProbe {
            name: "generated_accessor_call",
            category: "generated_or_framework",
            file: "lib/Mojolicious.pm",
            line: 55,
            needle: "->plugins",
            cursor_offset: 2,
            expected_substrings: &["plugins"],
        },
        HoverProbe {
            name: "dynamic_callable_shape",
            category: "dynamic_boundary_shape",
            file: "lib/Mojolicious/Controller.pm",
            line: 37,
            needle: "->$cb",
            cursor_offset: 2,
            expected_substrings: &["$cb", "cb"],
        },
        HoverProbe {
            name: "module_import_hover",
            category: "module_resolution",
            file: "lib/Mojolicious.pm",
            line: 8,
            needle: "Mojolicious::Commands",
            cursor_offset: 3,
            expected_substrings: &["Mojolicious::Commands"],
        },
        HoverProbe {
            name: "fallback_missing_module_shape",
            category: "fallback_or_missing_fact",
            file: "lib/Mojolicious.pm",
            line: 40,
            needle: "Mojo::Transaction::HTTP",
            cursor_offset: 6,
            expected_substrings: &["Mojo::Transaction::HTTP", "Mojo::Transaction"],
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

            let harness = create_fixture_harness(&fixture_files)?;
            open_all_fixture_files(&harness, &fixture_files)?;
            std::thread::sleep(Duration::from_millis(500));

            let probes = hover_probes();
            let mut reports = Vec::new();

            for probe in &probes {
                recorder.mark_request_start(probe.name);
                let report = run_probe(&harness, &fixture_files, probe)?;
                if report.result_state == "content" {
                    recorder.mark_first_useful_result(probe.name);
                }
                eprintln!(
                    "hover_probe={} category={} state={} hits={:?} source_labels={:?}",
                    report.name,
                    report.category,
                    report.result_state,
                    report.expected_hits,
                    report.source_label_hits
                );
                reports.push(report);
            }

            let categories = reports.iter().map(|report| report.category).collect::<BTreeSet<_>>();
            let content_count =
                reports.iter().filter(|report| report.result_state == "content").count();
            let expected_hit_total: usize =
                reports.iter().map(|report| report.expected_hits.len()).sum();
            let source_label_total: usize =
                reports.iter().map(|report| report.source_label_hits.len()).sum();
            let provenance_label_total: usize =
                reports.iter().map(|report| report.provenance_label_hits.len()).sum();
            let confidence_label_total: usize =
                reports.iter().map(|report| report.confidence_label_hits.len()).sum();
            let freshness_label_total: usize =
                reports.iter().map(|report| report.freshness_label_hits.len()).sum();
            let generated_label_total: usize =
                reports.iter().map(|report| report.generated_label_hits.len()).sum();
            let dynamic_boundary_label_total: usize =
                reports.iter().map(|report| report.dynamic_boundary_label_hits.len()).sum();
            let fallback_label_total: usize =
                reports.iter().map(|report| report.fallback_label_hits.len()).sum();

            let receipt = serde_json::json!({
                "schema_version": 1,
                "project": "mojolicious",
                "surface": "hover",
                "claim_boundary": "real-workspace hover quality receipt only; no provider behavior changed or promoted",
                "fixture_file_count": fixture_files.len(),
                "probe_count": reports.len(),
                "content_result_count": content_count,
                "expected_hit_total": expected_hit_total,
                "source_label_total": source_label_total,
                "provenance_label_total": provenance_label_total,
                "confidence_label_total": confidence_label_total,
                "freshness_label_total": freshness_label_total,
                "generated_label_total": generated_label_total,
                "dynamic_boundary_label_total": dynamic_boundary_label_total,
                "fallback_label_total": fallback_label_total,
                "reports": reports,
            });
            eprintln!(
                "mojolicious_hover_provenance_receipt={}",
                serde_json::to_string_pretty(&receipt)?
            );

            recorder.check("all hover probes produced reports", reports.len() == probes.len())?;
            recorder.check(
                "hover probes covered all intended receipt categories",
                categories
                    == BTreeSet::from([
                        "dynamic_boundary_shape",
                        "exact_syntax",
                        "fallback_or_missing_fact",
                        "generated_or_framework",
                        "imported_symbol",
                        "module_resolution",
                    ]),
            )?;
            recorder.check("at least one hover probe returned content", content_count > 0)?;
            recorder.check(
                "hover receipt recorded at least one expected text hit",
                expected_hit_total > 0,
            )?;
            recorder.check(
                "hover receipt recorded provenance or source labels when exposed",
                source_label_total
                    + provenance_label_total
                    + confidence_label_total
                    + freshness_label_total
                    > 0,
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}
