//! Scenario 28 - Mojolicious completion ranking receipt.
//!
//! This receipt exercises the committed Mojolicious skeleton workspace and
//! records completion quality signals without changing provider behavior.
//!
//! Receipt signals:
//! - candidate count and repeated-request candidate-count delta
//! - top-N ranking churn across repeated requests
//! - useful-hit and top-N noise counts for representative probes
//! - generated-candidate and provenance-label coverage
//! - dynamic/fallback label coverage when the provider exposes it

use anyhow::{Context, Result};
use perl_lsp_ux_tests::{
    ScenarioConfig, UxCiTier, UxComponent, UxHarness, UxScenarioSkip, run_ux_scenario,
};
use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

const SCENARIO_FILE: &str = "ux_scenario_28_mojolicious_completion_ranking.rs";
const TOP_N: usize = 10;

#[derive(Debug)]
struct FixtureFile {
    relative_path: String,
    content: String,
}

#[derive(Debug)]
struct CompletionProbe {
    name: &'static str,
    file: &'static str,
    line: u32,
    character: u32,
    useful_substrings: &'static [&'static str],
    generated_candidate_substrings: &'static [&'static str],
}

#[derive(Debug, Serialize)]
struct CompletionProbeReport {
    name: &'static str,
    file: &'static str,
    line: u32,
    character: u32,
    first_count: usize,
    second_count: usize,
    candidate_count_delta: usize,
    top_n_churn: usize,
    top_labels: Vec<String>,
    useful_hits: Vec<String>,
    top_n_noise_count: usize,
    noise_delta: isize,
    generated_candidate_hits: Vec<String>,
    generated_provenance_label_hits: Vec<String>,
    dynamic_or_fallback_label_hits: Vec<String>,
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

fn completion_label(item: &Value) -> Option<String> {
    item.get("label")
        .and_then(Value::as_str)
        .or_else(|| item.get("insertText").and_then(Value::as_str))
        .or_else(|| item.get("filterText").and_then(Value::as_str))
        .map(ToOwned::to_owned)
}

fn completion_text(item: &Value) -> String {
    let mut parts = Vec::new();
    for key in ["label", "insertText", "filterText", "detail"] {
        if let Some(value) = item.get(key).and_then(Value::as_str) {
            parts.push(value.to_string());
        }
    }
    if let Some(documentation) = item.get("documentation") {
        if let Some(value) = documentation.as_str() {
            parts.push(value.to_string());
        } else if let Some(value) = documentation.get("value").and_then(Value::as_str) {
            parts.push(value.to_string());
        }
    }
    parts.join("\n")
}

fn item_has_completion_shape(item: &Value) -> bool {
    item.get("label").and_then(Value::as_str).is_some()
        || item.get("insertText").and_then(Value::as_str).is_some()
        || item.get("filterText").and_then(Value::as_str).is_some()
}

fn labels_for(items: &[Value]) -> Vec<String> {
    items.iter().filter_map(completion_label).collect()
}

fn matching_labels(labels: &[String], substrings: &[&str]) -> Vec<String> {
    labels
        .iter()
        .filter(|label| substrings.iter().any(|needle| label.contains(needle)))
        .cloned()
        .collect()
}

fn matching_item_labels(items: &[Value], substrings: &[&str]) -> Vec<String> {
    items
        .iter()
        .filter(|item| {
            let haystack = completion_text(item);
            substrings.iter().any(|needle| haystack.contains(needle))
        })
        .filter_map(completion_label)
        .collect()
}

fn top_n_churn(first: &[String], second: &[String]) -> usize {
    let compared = first.len().max(second.len()).min(TOP_N);
    (0..compared).filter(|idx| first.get(*idx) != second.get(*idx)).count()
}

fn run_probe(harness: &UxHarness, probe: &CompletionProbe) -> Result<CompletionProbeReport> {
    let _warmup = harness.completion(probe.file, probe.line, probe.character)?;
    let first = harness.completion(probe.file, probe.line, probe.character)?;
    let second = harness.completion(probe.file, probe.line, probe.character)?;

    for item in first.iter().chain(second.iter()) {
        anyhow::ensure!(
            item_has_completion_shape(item),
            "completion item for probe {} must include label, insertText, or filterText: {item:?}",
            probe.name
        );
    }

    let labels = labels_for(&first);
    let second_labels = labels_for(&second);
    let top_labels = labels.iter().take(TOP_N).cloned().collect::<Vec<_>>();
    let useful_hits = matching_labels(&labels, probe.useful_substrings);
    let top_n_noise_count = top_labels
        .iter()
        .filter(|label| !probe.useful_substrings.iter().any(|needle| label.contains(needle)))
        .count();
    let noise_delta = top_n_noise_count as isize - useful_hits.len() as isize;

    let generated_candidate_hits = matching_labels(&labels, probe.generated_candidate_substrings);
    let generated_provenance_label_hits =
        matching_item_labels(&first, &["generated", "framework", "Mojo::Base", "has"]);
    let dynamic_or_fallback_label_hits =
        matching_item_labels(&first, &["dynamic", "fallback", "low confidence", "unknown"]);

    Ok(CompletionProbeReport {
        name: probe.name,
        file: probe.file,
        line: probe.line,
        character: probe.character,
        first_count: first.len(),
        second_count: second.len(),
        candidate_count_delta: first.len().abs_diff(second.len()),
        top_n_churn: top_n_churn(&labels, &second_labels),
        top_labels,
        useful_hits,
        top_n_noise_count,
        noise_delta,
        generated_candidate_hits,
        generated_provenance_label_hits,
        dynamic_or_fallback_label_hits,
    })
}

fn completion_probes() -> Vec<CompletionProbe> {
    vec![
        CompletionProbe {
            name: "module_prefix_mojolicious",
            file: "lib/Mojolicious.pm",
            line: 8,
            character: 17,
            useful_substrings: &[
                "Mojolicious::Commands",
                "Mojolicious::Controller",
                "Mojolicious::Renderer",
                "Mojolicious::Routes",
                "Commands",
                "Controller",
                "Renderer",
                "Routes",
            ],
            generated_candidate_substrings: &[],
        },
        CompletionProbe {
            name: "imported_croak_prefix",
            file: "lib/Mojolicious.pm",
            line: 72,
            character: 7,
            useful_substrings: &["croak"],
            generated_candidate_substrings: &[],
        },
        CompletionProbe {
            name: "self_accessor_arrow",
            file: "lib/Mojolicious.pm",
            line: 55,
            character: 25,
            useful_substrings: &["plugins", "renderer", "routes", "static", "sessions", "types"],
            generated_candidate_substrings: &[
                "commands",
                "controller_class",
                "log",
                "plugins",
                "renderer",
                "routes",
                "sessions",
                "static",
                "types",
                "mode",
            ],
        },
    ]
}

#[test]
fn scenario_28_mojolicious_visible_symbol_ranking_receipt() {
    run_ux_scenario(
        "mojolicious_completion_ranking",
        SCENARIO_FILE,
        "scenario_28_mojolicious_visible_symbol_ranking_receipt",
        UxCiTier::Pr,
        Some(UxComponent::Completion),
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

            let probes = completion_probes();
            let mut reports = Vec::new();

            for probe in &probes {
                recorder.mark_request_start(probe.name);
                let report = run_probe(&harness, probe)?;
                recorder.mark_first_useful_result(probe.name);
                reports.push(report);
            }

            recorder
                .check("all completion probes produced reports", reports.len() == probes.len())?;
            recorder.check(
                "completion probes returned at least one candidate somewhere",
                reports.iter().any(|report| report.first_count > 0),
            )?;
            recorder.check(
                "repeated completion requests kept candidate counts stable",
                reports.iter().all(|report| report.candidate_count_delta == 0),
            )?;
            recorder.check(
                "repeated completion requests kept top-N ranking stable",
                reports.iter().all(|report| report.top_n_churn == 0),
            )?;

            let useful_hit_total: usize =
                reports.iter().map(|report| report.useful_hits.len()).sum();
            let generated_candidate_total: usize =
                reports.iter().map(|report| report.generated_candidate_hits.len()).sum();
            let generated_provenance_label_total: usize =
                reports.iter().map(|report| report.generated_provenance_label_hits.len()).sum();
            let dynamic_or_fallback_label_total: usize =
                reports.iter().map(|report| report.dynamic_or_fallback_label_hits.len()).sum();

            let receipt = serde_json::json!({
                "schema_version": 1,
                "project": "mojolicious",
                "surface": "completion",
                "claim_boundary": "real-workspace quality receipt only; no provider behavior changed or promoted",
                "top_n": TOP_N,
                "fixture_file_count": fixture_files.len(),
                "probe_count": reports.len(),
                "useful_hit_total": useful_hit_total,
                "generated_candidate_total": generated_candidate_total,
                "generated_provenance_label_total": generated_provenance_label_total,
                "dynamic_or_fallback_label_total": dynamic_or_fallback_label_total,
                "reports": reports,
            });
            eprintln!(
                "mojolicious_completion_ranking_receipt={}",
                serde_json::to_string_pretty(&receipt)?
            );

            harness.assert_no_crash();
            Ok(())
        },
    );
}
