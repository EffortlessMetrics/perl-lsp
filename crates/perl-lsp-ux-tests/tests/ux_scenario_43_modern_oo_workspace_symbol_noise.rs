//! Scenario 43 - Modern OO workspace-symbol noise receipt.
//!
//! This receipt exercises `workspace/symbol` over the committed Modern OO
//! corpus fixture. It adds project-shaped generated-symbol rank/noise coverage
//! for Moose/Moo roles, accessors, delegated handles, and method modifiers
//! without expanding workspace-symbol live behavior.
//!
//! Receipt signals:
//! - query latency and repeated-request candidate-count delta
//! - useful source-backed hits versus unrelated/noisy hits for representative queries
//! - generated/framework candidate names plus explicitly labeled generated pilot symbols
//! - generated/no-source candidate names kept out of live exact workspace-symbol output
//! - role/delegation/modifier boundary shapes observed separately from exact symbols
//! - stale/fresh query behavior after editing an open document

use anyhow::{Context, Result};
use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::missing_binary_skip;
use perl_lsp_ux_tests::{ScenarioConfig, UxCiTier, UxComponent, UxHarness, run_ux_scenario};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const SCENARIO_FILE: &str = "ux_scenario_43_modern_oo_workspace_symbol_noise.rs";
const FIXTURE_RELATIVE_PATH: &str = "modern_oo_frameworks.pl";
const TOP_N: usize = 12;

#[derive(Debug)]
struct FixtureFile {
    relative_path: String,
    content: String,
}

#[derive(Debug)]
struct WorkspaceSymbolProbe {
    name: &'static str,
    category: &'static str,
    query: &'static str,
    useful_substrings: &'static [&'static str],
    generated_candidate_names: &'static [&'static str],
    generated_no_source_candidate_names: &'static [&'static str],
    boundary_source_shapes: &'static [&'static str],
}

#[derive(Debug, Serialize)]
struct WorkspaceSymbolProbeReport {
    name: &'static str,
    category: &'static str,
    query: &'static str,
    first_count: usize,
    second_count: usize,
    candidate_count_delta: usize,
    first_latency_ms: u128,
    second_latency_ms: u128,
    valid_shape_count: usize,
    invalid_shape_count: usize,
    useful_hits: Vec<String>,
    useful_hit_count: usize,
    top_n_noise_count: usize,
    unrelated_hit_count: usize,
    generated_candidate_count: usize,
    generated_candidate_live_hits: Vec<String>,
    generated_no_source_candidate_count: usize,
    generated_no_source_live_hits: Vec<String>,
    generated_label_hits: Vec<String>,
    first_useful_source_rank: Option<usize>,
    first_generated_label_rank: Option<usize>,
    generated_label_after_useful_source: bool,
    generated_label_before_useful_source_count: usize,
    boundary_source_shape_count: usize,
    stale_or_fallback_label_hits: Vec<String>,
    symbol_name_sample: Vec<String>,
}

#[derive(Debug, Serialize)]
struct FreshnessReport {
    file: &'static str,
    query: &'static str,
    before_count: usize,
    after_count: usize,
    stale_hits_after_edit: Vec<String>,
    fresh_hits_after_edit: Vec<String>,
    stale_symbol_absent: bool,
    fresh_symbol_present: bool,
    before_latency_ms: u128,
    after_latency_ms: u128,
}

struct TimedSymbols {
    symbols: Vec<Value>,
    latency_ms: u128,
}

fn workspace_root() -> Result<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .context("CARGO_MANIFEST_DIR must be nested under the workspace root")
}

fn modern_oo_fixture_path() -> Result<PathBuf> {
    Ok(workspace_root()?.join("test_corpus").join("real_world").join(FIXTURE_RELATIVE_PATH))
}

fn load_modern_oo_fixture_file() -> Result<FixtureFile> {
    let path = modern_oo_fixture_path()?;
    let content =
        fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    Ok(FixtureFile { relative_path: FIXTURE_RELATIVE_PATH.to_string(), content })
}

fn create_modern_oo_harness(file: &FixtureFile) -> Result<UxHarness> {
    let config = ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
        .env("PERL_LSP_WORKSPACE", "1")
        .with_file(&file.relative_path, &file.content);

    UxHarness::new(config)
}

fn timed_workspace_symbols(harness: &UxHarness, query: &str) -> Result<TimedSymbols> {
    let started = Instant::now();
    let symbols = harness.workspace_symbols(query)?;
    Ok(TimedSymbols { symbols, latency_ms: started.elapsed().as_millis() })
}

fn timed_workspace_symbols_until(
    harness: &UxHarness,
    query: &str,
    predicate: impl FnMut(&[Value]) -> bool,
) -> Result<TimedSymbols> {
    let started = Instant::now();
    let symbols = harness.wait_for_workspace_symbols(
        query,
        Duration::from_secs(5),
        Duration::from_millis(100),
        predicate,
    )?;
    Ok(TimedSymbols { symbols, latency_ms: started.elapsed().as_millis() })
}

fn workspace_symbol_name(symbol: &Value) -> Option<String> {
    symbol.get("name").and_then(Value::as_str).map(ToOwned::to_owned)
}

fn workspace_symbol_text(symbol: &Value) -> String {
    let mut parts = Vec::new();
    for key in ["name", "detail", "containerName"] {
        if let Some(value) = symbol.get(key).and_then(Value::as_str) {
            parts.push(value.to_string());
        }
    }
    parts.join("\n")
}

fn workspace_symbol_has_generated_label(symbol: &Value) -> bool {
    workspace_symbol_text(symbol).contains("[generated/framework]")
}

fn workspace_symbol_matches_any(symbol: &Value, substrings: &[&str]) -> bool {
    let haystack = workspace_symbol_text(symbol);
    substrings.iter().any(|needle| haystack.contains(needle))
}

fn first_symbol_rank_matching<F>(symbols: &[Value], predicate: F) -> Option<usize>
where
    F: Fn(&Value) -> bool,
{
    symbols.iter().position(predicate)
}

fn is_valid_position(position: &Value) -> bool {
    position.get("line").and_then(Value::as_u64).is_some()
        && position.get("character").and_then(Value::as_u64).is_some()
}

fn is_valid_range(range: &Value) -> bool {
    let Some(start) = range.get("start") else {
        return false;
    };
    let Some(end) = range.get("end") else {
        return false;
    };
    is_valid_position(start) && is_valid_position(end)
}

fn is_valid_workspace_symbol_shape(symbol: &Value) -> bool {
    let has_name = symbol.get("name").and_then(Value::as_str).is_some();
    let has_kind =
        symbol.get("kind").and_then(Value::as_u64).is_some_and(|kind| (1..=26).contains(&kind));
    let Some(location) = symbol.get("location") else {
        return false;
    };
    let has_uri =
        location.get("uri").or_else(|| location.get("targetUri")).and_then(Value::as_str).is_some();
    let has_range = location
        .get("range")
        .or_else(|| location.get("targetSelectionRange"))
        .is_some_and(is_valid_range);
    has_name && has_kind && has_uri && has_range
}

fn symbol_name_sample(symbols: &[Value]) -> Vec<String> {
    symbols.iter().take(TOP_N).filter_map(workspace_symbol_name).collect()
}

fn exact_unproven_generated_name_hits(
    symbols: &[Value],
    candidates: &[&str],
    useful_substrings: &[&str],
) -> Vec<String> {
    let candidate_names = candidates.iter().copied().collect::<BTreeSet<_>>();
    let mut names = symbols
        .iter()
        .filter_map(workspace_symbol_name)
        .filter(|name| candidate_names.contains(name.as_str()))
        .filter(|name| !useful_substrings.iter().any(|useful| name.eq_ignore_ascii_case(useful)))
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    names
}

fn matching_symbol_names(symbols: &[Value], substrings: &[&str]) -> Vec<String> {
    let mut names = symbols
        .iter()
        .filter(|symbol| {
            let haystack = workspace_symbol_text(symbol);
            substrings.iter().any(|needle| haystack.contains(needle))
        })
        .filter_map(workspace_symbol_name)
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    names
}

fn count_unrelated_hits(symbols: &[Value], query: &str, useful_substrings: &[&str]) -> usize {
    symbols
        .iter()
        .filter(|symbol| {
            let haystack = workspace_symbol_text(symbol);
            !haystack.contains(query)
                && !useful_substrings.iter().any(|needle| haystack.contains(needle))
        })
        .count()
}

fn count_top_n_noise(symbols: &[Value], useful_substrings: &[&str]) -> usize {
    symbols
        .iter()
        .take(TOP_N)
        .filter(|symbol| {
            let haystack = workspace_symbol_text(symbol);
            !useful_substrings.iter().any(|needle| haystack.contains(needle))
        })
        .count()
}

fn source_shape_hits<I>(source: &str, candidates: I) -> Vec<String>
where
    I: IntoIterator<Item = &'static str>,
{
    let mut hits = candidates
        .into_iter()
        .filter(|candidate| source.contains(candidate))
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    hits.sort();
    hits.dedup();
    hits
}

fn missing_source_shapes<I>(source: &str, candidates: I) -> Vec<String>
where
    I: IntoIterator<Item = &'static str>,
{
    let mut missing = candidates
        .into_iter()
        .filter(|candidate| !source.contains(candidate))
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    missing.sort();
    missing.dedup();
    missing
}

fn run_probe(
    harness: &UxHarness,
    probe: &WorkspaceSymbolProbe,
) -> Result<WorkspaceSymbolProbeReport> {
    let first = timed_workspace_symbols_until(harness, probe.query, |symbols| {
        symbols.iter().any(|symbol| workspace_symbol_matches_any(symbol, probe.useful_substrings))
            && symbols.iter().any(workspace_symbol_has_generated_label)
    })?;
    let second = timed_workspace_symbols(harness, probe.query)?;

    let valid_shape_count =
        first.symbols.iter().filter(|symbol| is_valid_workspace_symbol_shape(symbol)).count();
    let invalid_shape_count = first.symbols.len().saturating_sub(valid_shape_count);
    let useful_hits = matching_symbol_names(&first.symbols, probe.useful_substrings);
    let generated_candidate_live_hits = exact_unproven_generated_name_hits(
        &first.symbols,
        probe.generated_candidate_names,
        probe.useful_substrings,
    );
    let generated_no_source_live_hits = exact_unproven_generated_name_hits(
        &first.symbols,
        probe.generated_no_source_candidate_names,
        probe.useful_substrings,
    );
    let generated_label_hits = matching_symbol_names(
        &first.symbols,
        &["generated", "framework", "virtual", "FrameworkAdapter"],
    );
    let stale_or_fallback_label_hits = matching_symbol_names(
        &first.symbols,
        &["stale", "fallback", "low confidence", "dynamic boundary"],
    );
    let first_useful_source_rank = first_symbol_rank_matching(&first.symbols, |symbol| {
        !workspace_symbol_has_generated_label(symbol)
            && workspace_symbol_matches_any(symbol, probe.useful_substrings)
    });
    let first_generated_label_rank =
        first_symbol_rank_matching(&first.symbols, workspace_symbol_has_generated_label);
    let generated_label_after_useful_source = matches!(
        (first_useful_source_rank, first_generated_label_rank),
        (Some(source_rank), Some(generated_rank)) if source_rank < generated_rank
    );
    let generated_label_before_useful_source_count = first
        .symbols
        .iter()
        .enumerate()
        .filter(|(rank, symbol)| {
            workspace_symbol_has_generated_label(symbol)
                && first_useful_source_rank.is_none_or(|source_rank| *rank < source_rank)
        })
        .count();

    Ok(WorkspaceSymbolProbeReport {
        name: probe.name,
        category: probe.category,
        query: probe.query,
        first_count: first.symbols.len(),
        second_count: second.symbols.len(),
        candidate_count_delta: first.symbols.len().abs_diff(second.symbols.len()),
        first_latency_ms: first.latency_ms,
        second_latency_ms: second.latency_ms,
        valid_shape_count,
        invalid_shape_count,
        useful_hit_count: useful_hits.len(),
        useful_hits,
        top_n_noise_count: count_top_n_noise(&first.symbols, probe.useful_substrings),
        unrelated_hit_count: count_unrelated_hits(
            &first.symbols,
            probe.query,
            probe.useful_substrings,
        ),
        generated_candidate_count: probe.generated_candidate_names.len(),
        generated_candidate_live_hits,
        generated_no_source_candidate_count: probe.generated_no_source_candidate_names.len(),
        generated_no_source_live_hits,
        generated_label_hits,
        first_useful_source_rank,
        first_generated_label_rank,
        generated_label_after_useful_source,
        generated_label_before_useful_source_count,
        boundary_source_shape_count: probe.boundary_source_shapes.len(),
        stale_or_fallback_label_hits,
        symbol_name_sample: symbol_name_sample(&first.symbols),
    })
}

fn freshness_report(harness: &UxHarness, fixture: &FixtureFile) -> Result<FreshnessReport> {
    let before = timed_workspace_symbols(harness, "display_")?;
    let updated = fixture.content.replace("sub display_info", "sub display_summary");
    anyhow::ensure!(
        updated != fixture.content,
        "freshness fixture must rename display_info to display_summary"
    );

    harness.change_file_full(FIXTURE_RELATIVE_PATH, &updated)?;
    std::thread::sleep(Duration::from_millis(500));

    let after = timed_workspace_symbols(harness, "display_")?;
    let stale_hits_after_edit = matching_symbol_names(&after.symbols, &["display_info"]);
    let fresh_hits_after_edit = matching_symbol_names(&after.symbols, &["display_summary"]);

    Ok(FreshnessReport {
        file: FIXTURE_RELATIVE_PATH,
        query: "display_",
        before_count: before.symbols.len(),
        after_count: after.symbols.len(),
        stale_symbol_absent: stale_hits_after_edit.is_empty(),
        fresh_symbol_present: !fresh_hits_after_edit.is_empty(),
        stale_hits_after_edit,
        fresh_hits_after_edit,
        before_latency_ms: before.latency_ms,
        after_latency_ms: after.latency_ms,
    })
}

fn workspace_symbol_probes() -> Vec<WorkspaceSymbolProbe> {
    vec![
        WorkspaceSymbolProbe {
            name: "package_prefix_modern_oo",
            category: "package_prefix_quality",
            query: "MooseExample",
            useful_substrings: &[
                "MooseExample::User",
                "display_info",
                "has_admin_role",
                "validate",
            ],
            generated_candidate_names: &[
                "id",
                "name",
                "email",
                "set_email",
                "clear_email",
                "has_email",
            ],
            generated_no_source_candidate_names: &[
                "MooseExample::User::meta_generated_accessor",
                "MooseExample::User::runtime_role_method",
            ],
            boundary_source_shapes: &[
                "before 'set_email'",
                "after 'set_email'",
                "around 'set_email'",
            ],
        },
        WorkspaceSymbolProbe {
            name: "delegated_handles_quality",
            category: "delegated_handles_quality",
            query: "role",
            useful_substrings: &["has_admin_role", "RoleExample", "roles"],
            generated_candidate_names: &["add_role", "has_role", "role_count", "all_roles"],
            generated_no_source_candidate_names: &[
                "RoleExample::Generated::role_runtime_handle",
                "RoleExample::Generated::autoloaded_role_method",
            ],
            boundary_source_shapes: &["handles  => {", "add_role     =>", "role_count   =>"],
        },
        WorkspaceSymbolProbe {
            name: "moo_product_quality",
            category: "moo_product_quality",
            query: "price",
            useful_substrings: &["MooExample::Product", "set_price", "calculate_discount_price"],
            generated_candidate_names: &[
                "sku",
                "name",
                "price",
                "in_stock",
                "categories",
                "attributes",
            ],
            generated_no_source_candidate_names: &[
                "MooExample::Product::price_runtime_coercion",
                "MooExample::Product::generated_price_writer",
            ],
            boundary_source_shapes: &["die \"Price must be numeric\"", "$self->price($price)"],
        },
        WorkspaceSymbolProbe {
            name: "role_composition_boundary",
            category: "role_composition_boundary",
            query: "config",
            useful_substrings: &[
                "RoleExample::Configurable",
                "get_config",
                "set_config",
                "load_config",
            ],
            generated_candidate_names: &["config", "log_level", "name"],
            generated_no_source_candidate_names: &[
                "RoleExample::Configurable::config_runtime_accessor",
                "RoleExample::Configurable::generated_config_loader",
            ],
            boundary_source_shapes: &[
                "with 'RoleExample::Loggable'",
                "requires 'load_config'",
                "$self->config({ debug => 1, timeout => 30 });",
            ],
        },
    ]
}

#[test]
fn scenario_43_modern_oo_workspace_symbol_noise_receipt() {
    run_ux_scenario(
        "modern_oo_workspace_symbol_noise",
        SCENARIO_FILE,
        "scenario_43_modern_oo_workspace_symbol_noise_receipt",
        UxCiTier::Pr,
        Some(UxComponent::WorkspaceSymbols),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let fixture = load_modern_oo_fixture_file()?;
            recorder.check(
                "modern OO fixture has committed source",
                !fixture.content.trim().is_empty(),
            )?;

            let harness = create_modern_oo_harness(&fixture)?;
            harness.open_file(&fixture.relative_path, &fixture.content)?;
            std::thread::sleep(Duration::from_millis(500));

            let probes = workspace_symbol_probes();
            let mut reports = Vec::new();

            for probe in &probes {
                recorder.mark_request_start(probe.name);
                let report = run_probe(&harness, probe)?;
                if report.first_count > 0 {
                    recorder.mark_first_useful_result(probe.name);
                }
                eprintln!(
                    "workspace_symbol_probe={} query={} count={} useful_hits={:?} top_noise={} unrelated={} generated_live_hits={:?} generated_no_source_live_hits={:?} generated_labels={:?}",
                    report.name,
                    report.query,
                    report.first_count,
                    report.useful_hits,
                    report.top_n_noise_count,
                    report.unrelated_hit_count,
                    report.generated_candidate_live_hits,
                    report.generated_no_source_live_hits,
                    report.generated_label_hits
                );
                reports.push(report);
            }

            recorder.mark_request_start("freshness_after_edit");
            let freshness = freshness_report(&harness, &fixture)?;
            recorder.mark_first_useful_result("freshness_after_edit");
            eprintln!(
                "workspace_symbol_freshness file={} query={} stale_absent={} fresh_present={} stale_hits={:?} fresh_hits={:?}",
                freshness.file,
                freshness.query,
                freshness.stale_symbol_absent,
                freshness.fresh_symbol_present,
                freshness.stale_hits_after_edit,
                freshness.fresh_hits_after_edit
            );

            let categories = reports.iter().map(|report| report.category).collect::<BTreeSet<_>>();
            let live_symbol_total: usize = reports.iter().map(|report| report.first_count).sum();
            let invalid_shape_total: usize =
                reports.iter().map(|report| report.invalid_shape_count).sum();
            let useful_hit_total: usize =
                reports.iter().map(|report| report.useful_hit_count).sum();
            let top_n_noise_total: usize =
                reports.iter().map(|report| report.top_n_noise_count).sum();
            let unrelated_hit_total: usize =
                reports.iter().map(|report| report.unrelated_hit_count).sum();
            let candidate_count_delta_total: usize =
                reports.iter().map(|report| report.candidate_count_delta).sum();
            let generated_candidate_total: usize =
                reports.iter().map(|report| report.generated_candidate_count).sum();
            let generated_live_symbol_total: usize =
                reports.iter().map(|report| report.generated_candidate_live_hits.len()).sum();
            let generated_no_source_candidate_total: usize =
                reports.iter().map(|report| report.generated_no_source_candidate_count).sum();
            let generated_no_source_live_symbol_total: usize =
                reports.iter().map(|report| report.generated_no_source_live_hits.len()).sum();
            let generated_label_total: usize =
                reports.iter().map(|report| report.generated_label_hits.len()).sum();
            let generated_label_names_are_labeled = reports.iter().all(|report| {
                report
                    .generated_label_hits
                    .iter()
                    .all(|name| name.contains("[generated/framework]"))
            });
            let generated_label_rank_proof_count =
                reports.iter().filter(|report| report.generated_label_after_useful_source).count();
            let generated_label_before_useful_source_total: usize = reports
                .iter()
                .map(|report| report.generated_label_before_useful_source_count)
                .sum();
            let boundary_source_candidate_total: usize =
                reports.iter().map(|report| report.boundary_source_shape_count).sum();
            let boundary_source_hits = source_shape_hits(
                &fixture.content,
                probes.iter().flat_map(|probe| probe.boundary_source_shapes.iter().copied()),
            );
            let boundary_source_missing = missing_source_shapes(
                &fixture.content,
                probes.iter().flat_map(|probe| probe.boundary_source_shapes.iter().copied()),
            );
            let generated_source_hits = source_shape_hits(
                &fixture.content,
                probes.iter().flat_map(|probe| probe.generated_candidate_names.iter().copied()),
            );
            let generated_source_missing = missing_source_shapes(
                &fixture.content,
                probes.iter().flat_map(|probe| probe.generated_candidate_names.iter().copied()),
            );
            let generated_no_source_source_hits = source_shape_hits(
                &fixture.content,
                probes
                    .iter()
                    .flat_map(|probe| probe.generated_no_source_candidate_names.iter().copied()),
            );
            let generated_no_source_source_missing = missing_source_shapes(
                &fixture.content,
                probes
                    .iter()
                    .flat_map(|probe| probe.generated_no_source_candidate_names.iter().copied()),
            );
            let stale_or_fallback_label_total: usize =
                reports.iter().map(|report| report.stale_or_fallback_label_hits.len()).sum();
            let max_latency_ms = reports
                .iter()
                .flat_map(|report| [report.first_latency_ms, report.second_latency_ms])
                .chain([freshness.before_latency_ms, freshness.after_latency_ms])
                .max()
                .unwrap_or(0);

            let receipt = serde_json::json!({
                "schema_version": 1,
                "project": "modern_oo_frameworks",
                "surface": "workspace_symbols",
                "claim_boundary": "Modern OO workspace-symbol rank/noise receipt only; generated labels may appear only for source-backed pilot symbols and do not promote broad generated/dynamic behavior",
                "fixture_file": FIXTURE_RELATIVE_PATH,
                "probe_count": reports.len(),
                "live_symbol_total": live_symbol_total,
                "useful_hit_total": useful_hit_total,
                "top_n_noise_total": top_n_noise_total,
                "unrelated_hit_total": unrelated_hit_total,
                "candidate_count_delta_total": candidate_count_delta_total,
                "invalid_shape_total": invalid_shape_total,
                "generated_candidate_total": generated_candidate_total,
                "generated_live_symbol_total": generated_live_symbol_total,
                "generated_no_source_candidate_total": generated_no_source_candidate_total,
                "generated_no_source_live_symbol_total": generated_no_source_live_symbol_total,
                "generated_no_source_source_hits": generated_no_source_source_hits,
                "generated_no_source_source_missing": generated_no_source_source_missing,
                "generated_label_total": generated_label_total,
                "generated_label_rank_proof_count": generated_label_rank_proof_count,
                "generated_label_before_useful_source_total": generated_label_before_useful_source_total,
                "generated_source_hits": generated_source_hits,
                "generated_source_missing": generated_source_missing,
                "boundary_source_candidate_total": boundary_source_candidate_total,
                "boundary_source_hits": boundary_source_hits,
                "boundary_source_missing": boundary_source_missing,
                "stale_or_fallback_label_total": stale_or_fallback_label_total,
                "max_latency_ms": max_latency_ms,
                "freshness": freshness,
                "reports": reports,
            });
            eprintln!(
                "modern_oo_workspace_symbol_noise_receipt={}",
                serde_json::to_string_pretty(&receipt)?
            );

            recorder.check(
                "all workspace-symbol probes produced reports",
                reports.len() == probes.len(),
            )?;
            recorder.check(
                "workspace-symbol probes covered intended Modern OO receipt categories",
                categories
                    == BTreeSet::from([
                        "delegated_handles_quality",
                        "moo_product_quality",
                        "package_prefix_quality",
                        "role_composition_boundary",
                    ]),
            )?;
            recorder.check(
                "workspace-symbol probes returned useful Modern OO hits",
                useful_hit_total > 0,
            )?;
            recorder.check(
                "workspace-symbol entries used valid LSP shapes",
                invalid_shape_total == 0,
            )?;
            recorder.check(
                "receipt recorded repeated-query candidate count stability",
                candidate_count_delta_total == 0,
            )?;
            recorder.check(
                "receipt recorded generated candidates without exact generated-name promotion",
                generated_candidate_total > 0 && generated_live_symbol_total == 0,
            )?;
            recorder.check(
                "receipt recorded generated/no-source candidates without live exact promotion",
                generated_no_source_candidate_total > 0
                    && generated_no_source_live_symbol_total == 0,
            )?;
            recorder.check(
                "generated-label pilot surfaced explicitly labeled Modern OO symbols",
                generated_label_total > 0 && generated_label_names_are_labeled,
            )?;
            recorder.check(
                "generated-label pilot ranks after useful source-backed Modern OO symbols",
                generated_label_rank_proof_count > 0
                    && generated_label_before_useful_source_total == 0,
            )?;
            recorder.check(
                "configured generated candidates are backed by Modern OO source",
                !generated_source_hits.is_empty() && generated_source_missing.is_empty(),
            )?;
            recorder.check(
                "configured generated/no-source candidates have no Modern OO source anchor",
                generated_no_source_source_hits.is_empty()
                    && !generated_no_source_source_missing.is_empty(),
            )?;
            recorder.check(
                "receipt covered role/delegation/modifier boundary shapes",
                boundary_source_candidate_total > 0,
            )?;
            recorder.check(
                "configured boundary shapes are backed by Modern OO source",
                !boundary_source_hits.is_empty() && boundary_source_missing.is_empty(),
            )?;
            recorder.check(
                "workspace-symbol freshness after edit removed stale name and surfaced fresh name",
                freshness.stale_symbol_absent && freshness.fresh_symbol_present,
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}
