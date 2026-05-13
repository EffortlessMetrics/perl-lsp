//! Scenario 32 - Mojolicious document-symbol quality receipt.
//!
//! This receipt exercises `textDocument/documentSymbol` over the committed
//! Mojolicious skeleton workspace after the source-backed live document-symbol
//! slice. It records project-shaped symbol quality without broadening generated
//! or dynamic behavior.
//!
//! Receipt signals:
//! - document-symbol payload shape for selected real-workspace files
//! - expected source-backed package and subroutine symbols
//! - generated/dynamic route-method labels that must not become live exact
//!   source-backed document symbols

use anyhow::{Context, Result};
use perl_lsp_ux_tests::{
    ScenarioConfig, UxCiTier, UxComponent, UxHarness, UxScenarioSkip, run_ux_scenario,
};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

const SCENARIO_FILE: &str = "ux_scenario_32_mojolicious_document_symbols_quality.rs";

#[derive(Debug)]
struct FixtureFile {
    relative_path: String,
    content: String,
}

#[derive(Debug)]
struct DocumentSymbolProbe {
    name: &'static str,
    category: &'static str,
    file: &'static str,
    expected_names: &'static [&'static str],
    forbidden_names: &'static [&'static str],
}

#[derive(Debug, Default)]
struct FlattenedSymbol {
    name: String,
    detail: String,
    shape: &'static str,
    shape_valid: bool,
}

#[derive(Debug, Serialize)]
struct DocumentSymbolProbeReport {
    name: &'static str,
    category: &'static str,
    file: &'static str,
    top_level_count: usize,
    flattened_count: usize,
    document_symbol_shape_count: usize,
    symbol_information_shape_count: usize,
    invalid_shape_count: usize,
    expected_hits: Vec<String>,
    missing_expected_names: Vec<String>,
    forbidden_name_hits: Vec<String>,
    generated_or_dynamic_label_hits: Vec<String>,
    symbol_name_excerpt: Vec<String>,
    fallback_or_empty: bool,
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

fn document_symbol_probes() -> Vec<DocumentSymbolProbe> {
    vec![
        DocumentSymbolProbe {
            name: "mojolicious_app_surface",
            category: "source_backed_package_and_subs",
            file: "lib/Mojolicious.pm",
            expected_names: &["Mojolicious", "dispatch", "helper", "plugin", "start", "startup"],
            forbidden_names: &[],
        },
        DocumentSymbolProbe {
            name: "controller_surface",
            category: "source_backed_package_and_subs",
            file: "lib/Mojolicious/Controller.pm",
            expected_names: &["Mojolicious::Controller", "render", "rendered", "url_for"],
            forbidden_names: &[],
        },
        DocumentSymbolProbe {
            name: "routes_dynamic_boundary_surface",
            category: "dynamic_generated_boundary",
            file: "lib/Mojolicious/Routes.pm",
            expected_names: &["Mojolicious::Routes", "add_shortcut", "get", "post", "websocket"],
            forbidden_names: &["Mojolicious::Routes::Route::$name", "$name"],
        },
        DocumentSymbolProbe {
            name: "mojo_base_typeglob_boundary_surface",
            category: "typeglob_generated_boundary",
            file: "lib/Mojo/Base.pm",
            expected_names: &["Mojo::Base", "import", "new", "attr", "with_roles", "_attr"],
            forbidden_names: &["${class}::${name}"],
        },
    ]
}

fn is_valid_position(position: &Value) -> bool {
    position.get("line").and_then(Value::as_u64).is_some()
        && position.get("character").and_then(Value::as_u64).is_some()
}

fn is_valid_range(range: &Value) -> bool {
    range.get("start").is_some_and(is_valid_position)
        && range.get("end").is_some_and(is_valid_position)
}

fn is_valid_symbol_kind(value: &Value) -> bool {
    value.get("kind").and_then(Value::as_u64).is_some_and(|kind| (1..=26).contains(&kind))
}

fn symbol_name(value: &Value) -> Option<&str> {
    value.get("name").and_then(Value::as_str)
}

fn symbol_detail(value: &Value) -> &str {
    value.get("detail").and_then(Value::as_str).unwrap_or_default()
}

fn symbol_shape(value: &Value) -> (&'static str, bool) {
    let has_name = symbol_name(value).is_some();
    let has_kind = is_valid_symbol_kind(value);
    let document_symbol_shape = value.get("range").is_some_and(is_valid_range)
        && value.get("selectionRange").is_some_and(is_valid_range);
    let symbol_information_shape = value
        .get("location")
        .and_then(|location| location.get("range"))
        .is_some_and(is_valid_range);

    if has_name && has_kind && document_symbol_shape {
        ("document_symbol", true)
    } else if has_name && has_kind && symbol_information_shape {
        ("symbol_information", true)
    } else {
        ("invalid", false)
    }
}

fn flatten_symbols(symbols: &[Value]) -> Vec<FlattenedSymbol> {
    let mut flattened = Vec::new();
    for symbol in symbols {
        flatten_symbol(symbol, &mut flattened);
    }
    flattened
}

fn flatten_symbol(symbol: &Value, flattened: &mut Vec<FlattenedSymbol>) {
    let (shape, shape_valid) = symbol_shape(symbol);
    if let Some(name) = symbol_name(symbol) {
        flattened.push(FlattenedSymbol {
            name: name.to_string(),
            detail: symbol_detail(symbol).to_string(),
            shape,
            shape_valid,
        });
    } else {
        flattened.push(FlattenedSymbol {
            name: String::new(),
            detail: String::new(),
            shape,
            shape_valid,
        });
    }

    if let Some(children) = symbol.get("children").and_then(Value::as_array) {
        for child in children {
            flatten_symbol(child, flattened);
        }
    }
}

fn expected_hits(names: &[String], expected_names: &[&str]) -> Vec<String> {
    expected_names
        .iter()
        .copied()
        .filter(|expected| names.iter().any(|name| name == expected))
        .map(str::to_string)
        .collect()
}

fn missing_expected_names(names: &[String], expected_names: &[&str]) -> Vec<String> {
    expected_names
        .iter()
        .copied()
        .filter(|expected| !names.iter().any(|name| name == expected))
        .map(str::to_string)
        .collect()
}

fn forbidden_name_hits(symbols: &[FlattenedSymbol], forbidden_names: &[&str]) -> Vec<String> {
    forbidden_names
        .iter()
        .copied()
        .filter(|forbidden| symbols.iter().any(|symbol| symbol.name.contains(forbidden)))
        .map(str::to_string)
        .collect()
}

fn generated_or_dynamic_label_hits(symbols: &[FlattenedSymbol]) -> Vec<String> {
    symbols
        .iter()
        .filter(|symbol| {
            let text = format!("{}\n{}", symbol.name, symbol.detail).to_ascii_lowercase();
            ["generated", "dynamic", "autoload", "fallback", "low confidence"]
                .iter()
                .any(|needle| text.contains(needle))
        })
        .map(|symbol| symbol.name.clone())
        .collect()
}

fn symbol_name_excerpt(names: &[String]) -> Vec<String> {
    const MAX_NAMES: usize = 18;
    names.iter().take(MAX_NAMES).cloned().collect()
}

fn run_probe(
    harness: &UxHarness,
    probe: &DocumentSymbolProbe,
) -> Result<DocumentSymbolProbeReport> {
    let symbols = harness.document_symbols(probe.file)?;
    let flattened = flatten_symbols(&symbols);
    let names = flattened.iter().map(|symbol| symbol.name.clone()).collect::<Vec<_>>();
    let document_symbol_shape_count =
        flattened.iter().filter(|symbol| symbol.shape == "document_symbol").count();
    let symbol_information_shape_count =
        flattened.iter().filter(|symbol| symbol.shape == "symbol_information").count();
    let invalid_shape_count = flattened.iter().filter(|symbol| !symbol.shape_valid).count();
    let expected = expected_hits(&names, probe.expected_names);
    let missing = missing_expected_names(&names, probe.expected_names);
    let forbidden = forbidden_name_hits(&flattened, probe.forbidden_names);
    let generated_or_dynamic = generated_or_dynamic_label_hits(&flattened);

    Ok(DocumentSymbolProbeReport {
        name: probe.name,
        category: probe.category,
        file: probe.file,
        top_level_count: symbols.len(),
        flattened_count: flattened.len(),
        document_symbol_shape_count,
        symbol_information_shape_count,
        invalid_shape_count,
        expected_hits: expected,
        missing_expected_names: missing,
        forbidden_name_hits: forbidden,
        generated_or_dynamic_label_hits: generated_or_dynamic,
        symbol_name_excerpt: symbol_name_excerpt(&names),
        fallback_or_empty: symbols.is_empty(),
    })
}

#[test]
fn scenario_32_mojolicious_document_symbols_quality_receipt() {
    run_ux_scenario(
        "mojolicious_document_symbols_quality",
        SCENARIO_FILE,
        "scenario_32_mojolicious_document_symbols_quality_receipt",
        UxCiTier::Pr,
        Some(UxComponent::WorkspaceSymbols),
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

            let probes = document_symbol_probes();
            let mut reports = Vec::new();

            for probe in &probes {
                recorder.mark_request_start(probe.name);
                let report = run_probe(&harness, probe)?;
                if report.flattened_count > 0 {
                    recorder.mark_first_useful_result(probe.name);
                }
                eprintln!(
                    "document_symbol_probe={} category={} flattened={} expected_hits={:?} missing={:?} forbidden={:?}",
                    report.name,
                    report.category,
                    report.flattened_count,
                    report.expected_hits,
                    report.missing_expected_names,
                    report.forbidden_name_hits
                );
                reports.push(report);
            }

            let categories = reports.iter().map(|report| report.category).collect::<BTreeSet<_>>();
            let invalid_shape_total: usize =
                reports.iter().map(|report| report.invalid_shape_count).sum();
            let expected_hit_total: usize =
                reports.iter().map(|report| report.expected_hits.len()).sum();
            let missing_expected_total: usize =
                reports.iter().map(|report| report.missing_expected_names.len()).sum();
            let forbidden_name_total: usize =
                reports.iter().map(|report| report.forbidden_name_hits.len()).sum();
            let document_symbol_shape_total: usize =
                reports.iter().map(|report| report.document_symbol_shape_count).sum();
            let symbol_information_shape_total: usize =
                reports.iter().map(|report| report.symbol_information_shape_count).sum();
            let fallback_or_empty_count =
                reports.iter().filter(|report| report.fallback_or_empty).count();
            let generated_or_dynamic_label_total: usize =
                reports.iter().map(|report| report.generated_or_dynamic_label_hits.len()).sum();

            let receipt = serde_json::json!({
                "schema_version": 1,
                "project": "mojolicious",
                "surface": "document_symbols",
                "claim_boundary": "real-workspace document-symbol quality receipt only; no generated, dynamic, stale, low-confidence, or ambiguous symbol cutover",
                "fixture_file_count": fixture_files.len(),
                "probe_count": reports.len(),
                "expected_hit_total": expected_hit_total,
                "missing_expected_total": missing_expected_total,
                "forbidden_name_total": forbidden_name_total,
                "invalid_shape_total": invalid_shape_total,
                "document_symbol_shape_total": document_symbol_shape_total,
                "symbol_information_shape_total": symbol_information_shape_total,
                "fallback_or_empty_count": fallback_or_empty_count,
                "generated_or_dynamic_label_total": generated_or_dynamic_label_total,
                "reports": reports,
            });
            eprintln!(
                "mojolicious_document_symbols_quality_receipt={}",
                serde_json::to_string_pretty(&receipt)?
            );

            recorder.check(
                "all document-symbol probes produced reports",
                reports.len() == probes.len(),
            )?;
            recorder.check(
                "document-symbol probes covered all intended receipt categories",
                categories
                    == BTreeSet::from([
                        "dynamic_generated_boundary",
                        "source_backed_package_and_subs",
                        "typeglob_generated_boundary",
                    ]),
            )?;
            recorder.check(
                "document-symbol responses used valid LSP payload shape",
                invalid_shape_total == 0,
            )?;
            recorder.check(
                "document-symbol receipt recorded source-backed symbols",
                document_symbol_shape_total + symbol_information_shape_total > 0,
            )?;
            recorder.check(
                "selected Mojolicious files returned expected source-backed symbols",
                expected_hit_total > 0 && missing_expected_total == 0,
            )?;
            recorder.check(
                "generated or dynamic labels were not promoted as exact document symbols",
                forbidden_name_total == 0,
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}
