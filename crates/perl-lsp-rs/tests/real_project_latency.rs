//! Real-project latency suite — 3 project baselines with p50/p95/p99
//!
//! Measures LSP response latencies on representative Perl project fixtures
//! extracted from real open-source projects (Mojolicious, Dancer2, Catalyst).
//!
//! ## Running
//!
//! ```bash
//! # Run all latency tests (nightly)
//! cargo test -p perl-lsp-rs --test real_project_latency -- --include-ignored --nocapture
//!
//! # Run a single project
//! cargo test -p perl-lsp-rs --test real_project_latency mojolicious -- --include-ignored --nocapture
//! ```
//!
//! ## Metrics
//!
//! For each project, 5 operations are timed over N samples:
//! 1. Cold-start-to-hover: server init + first hover response
//! 2. First completion (after `->`)
//! 3. First goto-definition on an imported symbol
//! 4. Incremental reparse (after a 1-line didChange)
//! 5. Workspace symbol query latency
//!
//! Output is written to `.ci/metrics/real_project_latency.json`.

#![allow(clippy::panic)] // Test file: panics in assertion helpers are intentional
#![allow(clippy::manual_is_multiple_of)] // `% 4 == 0` is clearer than `.is_multiple_of(4)` for calendar math

mod common;

use common::{initialize_lsp, send_notification, send_request, start_lsp_server};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

// ---- Constants ----------------------------------------------------------------

/// Number of samples per metric — p50/p95/p99 are computed from this distribution.
const LATENCY_SAMPLES: usize = 10;

/// JSON output path (relative to workspace root).
const OUTPUT_PATH: &str = ".ci/metrics/real_project_latency.json";

/// Fixture base directory (relative to workspace root).
const FIXTURE_BASE: &str = "test_corpus/real_projects";

// ---- Data types ---------------------------------------------------------------

/// Per-metric latency summary: p50, p95, p99 (in milliseconds) + sample count.
#[derive(Debug, Clone)]
struct LatencySummary {
    p50_ms: u64,
    p95_ms: u64,
    p99_ms: u64,
    samples: usize,
}

/// All 5 metrics for a single project.
#[derive(Debug)]
struct ProjectMetrics {
    name: String,
    file_count: usize,
    cold_start_to_hover: LatencySummary,
    first_completion: LatencySummary,
    first_goto_definition: LatencySummary,
    incremental_reparse: LatencySummary,
    workspace_symbol_query: LatencySummary,
}

/// A project fixture definition.
struct ProjectFixture {
    /// Short name used in JSON output and test names.
    name: &'static str,
    /// Subdirectory under `test_corpus/real_projects/`.
    dir: &'static str,
    /// A `.pm` file within the fixture that contains a package declaration.
    entry_file: &'static str,
    /// Line (0-indexed) in `entry_file` where a symbol useful for hover exists.
    hover_line: u32,
    hover_col: u32,
    /// Line for method-call completion trigger.
    completion_line: u32,
    completion_col: u32,
    /// Line for goto-definition of an imported symbol.
    definition_line: u32,
    definition_col: u32,
}

// ---- Fixture definitions -------------------------------------------------------

const MOJOLICIOUS_FIXTURE: ProjectFixture = ProjectFixture {
    name: "mojolicious",
    dir: "mojolicious_skeleton",
    entry_file: "lib/Mojolicious.pm",
    hover_line: 10,
    hover_col: 6,
    completion_line: 14,
    completion_col: 10,
    definition_line: 5,
    definition_col: 4,
};

const DANCER2_FIXTURE: ProjectFixture = ProjectFixture {
    name: "dancer2",
    dir: "dancer2_skeleton",
    entry_file: "lib/Dancer2.pm",
    hover_line: 10,
    hover_col: 6,
    completion_line: 14,
    completion_col: 10,
    definition_line: 5,
    definition_col: 4,
};

const CATALYST_FIXTURE: ProjectFixture = ProjectFixture {
    name: "catalyst",
    dir: "catalyst_skeleton",
    entry_file: "lib/Catalyst.pm",
    hover_line: 10,
    hover_col: 6,
    completion_line: 14,
    completion_col: 10,
    definition_line: 5,
    definition_col: 4,
};

// ---- Helpers ------------------------------------------------------------------

/// Return workspace root (directory that contains Cargo.lock).
fn workspace_root() -> PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let mut dir = Path::new(&manifest).to_path_buf();
    loop {
        if dir.join("Cargo.lock").exists() {
            return dir;
        }
        match dir.parent() {
            Some(p) => dir = p.to_path_buf(),
            None => return Path::new(&manifest).to_path_buf(),
        }
    }
}

/// Return absolute path to a fixture directory.
fn fixture_path(fixture: &ProjectFixture) -> PathBuf {
    workspace_root().join(FIXTURE_BASE).join(fixture.dir)
}

/// Return absolute path to the fixture entry file.
fn entry_file_path(fixture: &ProjectFixture) -> PathBuf {
    fixture_path(fixture).join(fixture.entry_file)
}

/// Count `.pm` and `.pl` files recursively in a directory.
fn count_perl_files(dir: &Path) -> usize {
    if !dir.exists() {
        return 0;
    }
    let mut count = 0;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                count += count_perl_files(&path);
            } else if let Some(ext) = path.extension() {
                if ext == "pm" || ext == "pl" || ext == "t" {
                    count += 1;
                }
            }
        }
    }
    count
}

/// Compute percentile (0–100) from a sorted sample slice (values in ms).
fn percentile(sorted: &[u64], pct: usize) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((pct as f64 / 100.0) * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

/// Summarise a raw sample vec: sort, then extract p50/p95/p99.
fn summarise(mut samples: Vec<u64>) -> LatencySummary {
    let n = samples.len();
    samples.sort_unstable();
    LatencySummary {
        p50_ms: percentile(&samples, 50),
        p95_ms: percentile(&samples, 95),
        p99_ms: percentile(&samples, 99),
        samples: n,
    }
}

/// Read a file as string, returning empty string if missing.
fn read_file_or_empty(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

/// Build an LSP file URI for an absolute path.
fn file_uri(path: &Path) -> String {
    let s = path.to_string_lossy();
    if s.starts_with('/') {
        format!("file://{s}")
    } else {
        // Windows: replace backslashes, prepend authority slash
        let forward = s.replace('\\', "/");
        format!("file:///{forward}")
    }
}

/// Return an ISO-8601 UTC timestamp string using std::time only.
fn utc_now_iso8601() -> String {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO).as_secs();
    // Manual breakdown into date/time components
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    let days = secs / 86400;
    // Compute year/month/day from days since epoch (1970-01-01)
    let (year, month, day) = days_to_ymd(days);
    format!("{year:04}-{month:02}-{day:02}T{h:02}:{m:02}:{s:02}Z")
}

/// Convert days since Unix epoch (1970-01-01) to (year, month, day).
fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    // 400-year Gregorian cycle = 146097 days
    let year400 = days / 146097;
    days %= 146097;
    let year100 = (days / 36524).min(3);
    days -= year100 * 36524;
    let year4 = days / 1461;
    days %= 1461;
    let year1 = (days / 365).min(3);
    days -= year1 * 365;
    let year = year400 * 400 + year100 * 100 + year4 * 4 + year1 + 1970;
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let month_days: &[u64] = if leap {
        &[31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        &[31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1u64;
    for &md in month_days {
        if days < md {
            break;
        }
        days -= md;
        month += 1;
    }
    (year, month, days + 1)
}

// ---- Core measurement functions -----------------------------------------------

/// Helper: open a document on an already-initialised server.
fn open_document(server: &common::LspServer, uri: &str, content: &str) {
    send_notification(
        server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": content
                }
            }
        }),
    );
}

/// Measure cold-start-to-hover: includes server start + initialize + first hover.
fn measure_cold_start_to_hover(fixture: &ProjectFixture, entry_content: &str) -> Vec<u64> {
    let uri = file_uri(&entry_file_path(fixture));
    let mut samples = Vec::with_capacity(LATENCY_SAMPLES);

    for _ in 0..LATENCY_SAMPLES {
        let start = Instant::now();
        let server = start_lsp_server();
        initialize_lsp(&server);
        open_document(&server, &uri, entry_content);

        let _hover = send_request(
            &server,
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/hover",
                "params": {
                    "textDocument": { "uri": uri },
                    "position": {
                        "line": fixture.hover_line,
                        "character": fixture.hover_col
                    }
                }
            }),
        );
        samples.push(start.elapsed().as_millis() as u64);
        // Server drops here (graceful shutdown via Drop)
    }

    samples
}

/// Measure first-completion latency (server already warmed).
fn measure_first_completion(fixture: &ProjectFixture, entry_content: &str) -> Vec<u64> {
    let uri = file_uri(&entry_file_path(fixture));
    let server = start_lsp_server();
    initialize_lsp(&server);
    open_document(&server, &uri, entry_content);
    std::thread::sleep(Duration::from_millis(50));

    let mut samples = Vec::with_capacity(LATENCY_SAMPLES);
    for _ in 0..LATENCY_SAMPLES {
        let start = Instant::now();
        let _resp = send_request(
            &server,
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/completion",
                "params": {
                    "textDocument": { "uri": uri },
                    "position": {
                        "line": fixture.completion_line,
                        "character": fixture.completion_col
                    },
                    "context": {
                        "triggerKind": 2,
                        "triggerCharacter": ">"
                    }
                }
            }),
        );
        samples.push(start.elapsed().as_millis() as u64);
    }
    samples
}

/// Measure goto-definition latency (server already warmed).
fn measure_goto_definition(fixture: &ProjectFixture, entry_content: &str) -> Vec<u64> {
    let uri = file_uri(&entry_file_path(fixture));
    let server = start_lsp_server();
    initialize_lsp(&server);
    open_document(&server, &uri, entry_content);
    std::thread::sleep(Duration::from_millis(50));

    let mut samples = Vec::with_capacity(LATENCY_SAMPLES);
    for _ in 0..LATENCY_SAMPLES {
        let start = Instant::now();
        let _resp = send_request(
            &server,
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/definition",
                "params": {
                    "textDocument": { "uri": uri },
                    "position": {
                        "line": fixture.definition_line,
                        "character": fixture.definition_col
                    }
                }
            }),
        );
        samples.push(start.elapsed().as_millis() as u64);
    }
    samples
}

/// Measure incremental-reparse latency: didChange + re-request hover.
fn measure_incremental_reparse(fixture: &ProjectFixture, entry_content: &str) -> Vec<u64> {
    let uri = file_uri(&entry_file_path(fixture));
    let server = start_lsp_server();
    initialize_lsp(&server);
    open_document(&server, &uri, entry_content);
    std::thread::sleep(Duration::from_millis(50));

    let base_lines: Vec<String> = entry_content.lines().map(|l| l.to_string()).collect();
    let mut samples = Vec::with_capacity(LATENCY_SAMPLES);

    for i in 0..LATENCY_SAMPLES {
        let version = (i + 2) as i32;
        let mut lines = base_lines.clone();
        if !lines.is_empty() {
            lines[0] = format!("{}  # edit {i}", lines[0]);
        }
        let new_content = lines.join("\n");

        let start = Instant::now();
        send_notification(
            &server,
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": { "uri": uri, "version": version },
                    "contentChanges": [{ "text": new_content }]
                }
            }),
        );
        let _resp = send_request(
            &server,
            json!({
                "jsonrpc": "2.0",
                "method": "textDocument/hover",
                "params": {
                    "textDocument": { "uri": uri },
                    "position": {
                        "line": fixture.hover_line,
                        "character": fixture.hover_col
                    }
                }
            }),
        );
        samples.push(start.elapsed().as_millis() as u64);
    }
    samples
}

/// Measure workspace/symbol query latency.
fn measure_workspace_symbol(fixture: &ProjectFixture, entry_content: &str) -> Vec<u64> {
    let uri = file_uri(&entry_file_path(fixture));
    let server = start_lsp_server();
    initialize_lsp(&server);
    open_document(&server, &uri, entry_content);
    std::thread::sleep(Duration::from_millis(100));

    let mut samples = Vec::with_capacity(LATENCY_SAMPLES);
    for _ in 0..LATENCY_SAMPLES {
        let start = Instant::now();
        let _resp = send_request(
            &server,
            json!({
                "jsonrpc": "2.0",
                "method": "workspace/symbol",
                "params": { "query": "new" }
            }),
        );
        samples.push(start.elapsed().as_millis() as u64);
    }
    samples
}

/// Build a synthetic Catalyst-style app with at least `target_lines` lines.
fn synthetic_catalyst_app(target_lines: usize) -> String {
    let mut content = String::from(
        "package MyApp;\n\
         use strict;\n\
         use warnings;\n\
         use Catalyst qw/-Debug ConfigLoader Static::Simple/;\n\
         extends 'Catalyst';\n\n",
    );

    let mut line_count = content.lines().count();
    let mut i = 0usize;
    while line_count < target_lines.saturating_sub(2) {
        let block = format!(
            "sub action_{i} : Path('/route_{i}') Args(1) {{\n\
             \x20\x20my ($self, $c, $arg) = @_;\n\
             \x20\x20my $value = $c->req->params->{{value}} // $arg;\n\
             \x20\x20$c->stash->{{result}} = uc($value);\n\
             \x20\x20$c->response->body($c->stash->{{result}});\n\
             }}\n\n"
        );
        line_count += block.lines().count();
        content.push_str(&block);
        i += 1;
    }

    content.push_str("__PACKAGE__->setup();\n1;\n");
    content
}

// ---- JSON output --------------------------------------------------------------

/// Serialise a LatencySummary to a serde_json Value.
fn summary_to_json(s: &LatencySummary) -> Value {
    json!({
        "p50_ms": s.p50_ms,
        "p95_ms": s.p95_ms,
        "p99_ms": s.p99_ms,
        "samples": s.samples
    })
}

/// Write the baseline JSON file (creates parent directories if needed).
fn write_baseline(projects: &[ProjectMetrics]) {
    let root = workspace_root();
    let output_path = root.join(OUTPUT_PATH);

    if let Some(parent) = output_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let now = utc_now_iso8601();
    let commit = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| "unknown".to_string())
        .trim()
        .to_string();

    let mut projects_map = serde_json::Map::new();
    for p in projects {
        let metrics = json!({
            "cold_start_to_hover": summary_to_json(&p.cold_start_to_hover),
            "first_completion": summary_to_json(&p.first_completion),
            "first_goto_definition": summary_to_json(&p.first_goto_definition),
            "incremental_reparse": summary_to_json(&p.incremental_reparse),
            "workspace_symbol_query": summary_to_json(&p.workspace_symbol_query)
        });
        projects_map.insert(
            p.name.clone(),
            json!({
                "file_count": p.file_count,
                "metrics": metrics
            }),
        );
    }

    let output = json!({
        "schema_version": 1,
        "measured_at": now,
        "commit": commit,
        "projects": projects_map,
        "tolerance_pct": 10
    });

    match serde_json::to_string_pretty(&output) {
        Ok(s) => {
            if let Err(e) = fs::write(&output_path, &s) {
                eprintln!("Warning: failed to write baseline to {output_path:?}: {e}");
            } else {
                eprintln!("Baseline written to {output_path:?}");
            }
        }
        Err(e) => eprintln!("Warning: failed to serialise baseline: {e}"),
    }
}

/// Print a human-readable summary of a project's metrics.
fn print_metrics(p: &ProjectMetrics) {
    eprintln!("\n=== {} ({} files) ===", p.name, p.file_count);
    eprintln!(
        "  cold_start_to_hover  : p50={:>5}ms  p95={:>5}ms  p99={:>5}ms",
        p.cold_start_to_hover.p50_ms, p.cold_start_to_hover.p95_ms, p.cold_start_to_hover.p99_ms
    );
    eprintln!(
        "  first_completion     : p50={:>5}ms  p95={:>5}ms  p99={:>5}ms",
        p.first_completion.p50_ms, p.first_completion.p95_ms, p.first_completion.p99_ms
    );
    eprintln!(
        "  first_goto_definition: p50={:>5}ms  p95={:>5}ms  p99={:>5}ms",
        p.first_goto_definition.p50_ms,
        p.first_goto_definition.p95_ms,
        p.first_goto_definition.p99_ms
    );
    eprintln!(
        "  incremental_reparse  : p50={:>5}ms  p95={:>5}ms  p99={:>5}ms",
        p.incremental_reparse.p50_ms, p.incremental_reparse.p95_ms, p.incremental_reparse.p99_ms
    );
    eprintln!(
        "  workspace_symbol     : p50={:>5}ms  p95={:>5}ms  p99={:>5}ms",
        p.workspace_symbol_query.p50_ms,
        p.workspace_symbol_query.p95_ms,
        p.workspace_symbol_query.p99_ms
    );
}

// ---- Core measurement harness ------------------------------------------------

/// Run all 5 metrics for a single fixture and return the results.
/// Panics if the fixture directory does not exist.
fn measure_project(fixture: &ProjectFixture) -> ProjectMetrics {
    let path = fixture_path(fixture);
    assert!(
        path.exists(),
        "Fixture directory not found: {path:?}\n\
        Expected: test_corpus/real_projects/{}/\n\
        Create the fixture skeleton to satisfy this test.",
        fixture.dir
    );

    let entry = entry_file_path(fixture);
    let entry_content = read_file_or_empty(&entry);
    assert!(!entry_content.is_empty(), "Fixture entry file is empty or missing: {entry:?}");

    let file_count = count_perl_files(&path);
    eprintln!("[{name}] measuring (file_count={file_count})", name = fixture.name);

    let cold_start = summarise(measure_cold_start_to_hover(fixture, &entry_content));
    let completion = summarise(measure_first_completion(fixture, &entry_content));
    let definition = summarise(measure_goto_definition(fixture, &entry_content));
    let reparse = summarise(measure_incremental_reparse(fixture, &entry_content));
    let ws_symbol = summarise(measure_workspace_symbol(fixture, &entry_content));

    ProjectMetrics {
        name: fixture.name.to_string(),
        file_count,
        cold_start_to_hover: cold_start,
        first_completion: completion,
        first_goto_definition: definition,
        incremental_reparse: reparse,
        workspace_symbol_query: ws_symbol,
    }
}

// ---- Tests -------------------------------------------------------------------

/// Sanity check: fixture directories exist before any latency tests run.
/// This test is NOT ignored — it runs in the normal test suite to verify
/// the fixture skeleton has been committed.
#[test]
fn test_real_project_fixtures_exist() {
    let root = workspace_root();
    let base = root.join(FIXTURE_BASE);

    for dir in &["mojolicious_skeleton", "dancer2_skeleton", "catalyst_skeleton"] {
        let path = base.join(dir);
        assert!(
            path.exists(),
            "Real project fixture directory missing: {path:?}\n\
            Expected: test_corpus/real_projects/{dir}/\n\
            Create the fixture skeleton to satisfy this test."
        );
    }
}

/// Sanity check: each fixture entry file exists and contains valid Perl.
#[test]
fn test_real_project_entry_files_are_valid_perl() {
    for fixture in &[&MOJOLICIOUS_FIXTURE, &DANCER2_FIXTURE, &CATALYST_FIXTURE] {
        let entry = entry_file_path(fixture);
        assert!(entry.exists(), "Entry file missing for fixture '{}': {entry:?}", fixture.name);
        let content = fs::read_to_string(&entry)
            .unwrap_or_else(|e| panic!("Cannot read entry file {entry:?}: {e}"));
        assert!(
            !content.is_empty(),
            "Entry file is empty for fixture '{}': {entry:?}",
            fixture.name
        );
        // Basic Perl sanity: should contain 'package' or shebang
        assert!(
            content.contains("package") || content.contains("#!/"),
            "Entry file for '{}' does not look like Perl (no 'package' or shebang): {entry:?}",
            fixture.name
        );
    }
}

/// Sanity check: baseline JSON schema is valid once file exists.
#[test]
fn test_real_project_latency_baseline_schema() {
    let output_path = workspace_root().join(OUTPUT_PATH);
    if !output_path.exists() {
        // Baseline doesn't exist yet — expected before first nightly run.
        eprintln!(
            "Baseline not yet generated (expected before first nightly run): {output_path:?}"
        );
        return;
    }

    let content = fs::read_to_string(&output_path)
        .unwrap_or_else(|e| panic!("Cannot read baseline at {output_path:?}: {e}"));

    let parsed: Value = serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Baseline JSON is malformed at {output_path:?}: {e}"));

    assert_eq!(
        parsed.get("schema_version").and_then(Value::as_u64),
        Some(1),
        "Baseline must have schema_version=1"
    );

    assert!(parsed.get("projects").is_some(), "Baseline must have a 'projects' key");

    let projects = &parsed["projects"];
    for name in &["mojolicious", "dancer2", "catalyst"] {
        assert!(projects.get(name).is_some(), "Baseline missing project '{name}'");
        let proj = &projects[name];
        let metrics = proj
            .get("metrics")
            .unwrap_or_else(|| panic!("Project '{name}' missing 'metrics' in baseline"));

        for metric in &[
            "cold_start_to_hover",
            "first_completion",
            "first_goto_definition",
            "incremental_reparse",
            "workspace_symbol_query",
        ] {
            let m = metrics.get(metric).unwrap_or_else(|| {
                panic!("Project '{name}' missing metric '{metric}' in baseline")
            });
            assert!(m.get("p50_ms").is_some(), "'{name}.{metric}' missing p50_ms");
            assert!(m.get("p95_ms").is_some(), "'{name}.{metric}' missing p95_ms");
            assert!(m.get("p99_ms").is_some(), "'{name}.{metric}' missing p99_ms");
            assert!(m.get("samples").is_some(), "'{name}.{metric}' missing samples");
        }
    }
}

/// Latency benchmark: Mojolicious skeleton.
///
/// Run with:
/// ```bash
/// cargo test -p perl-lsp-rs --test real_project_latency mojolicious -- --include-ignored --nocapture
/// ```
#[test]
#[ignore = "nightly only — requires fixtures and extended runtime"]
fn real_project_latency_mojolicious() {
    let metrics = measure_project(&MOJOLICIOUS_FIXTURE);
    print_metrics(&metrics);
    write_baseline(&[metrics]);
}

/// Latency benchmark: Dancer2 skeleton.
///
/// Run with:
/// ```bash
/// cargo test -p perl-lsp-rs --test real_project_latency dancer2 -- --include-ignored --nocapture
/// ```
#[test]
#[ignore = "nightly only — requires fixtures and extended runtime"]
fn real_project_latency_dancer2() {
    let metrics = measure_project(&DANCER2_FIXTURE);
    print_metrics(&metrics);
    write_baseline(&[metrics]);
}

/// Latency benchmark: Catalyst skeleton.
///
/// Run with:
/// ```bash
/// cargo test -p perl-lsp-rs --test real_project_latency catalyst -- --include-ignored --nocapture
/// ```
#[test]
#[ignore = "nightly only — requires fixtures and extended runtime"]
fn real_project_latency_catalyst() {
    let metrics = measure_project(&CATALYST_FIXTURE);
    print_metrics(&metrics);
    write_baseline(&[metrics]);
}

/// Full suite: all 3 projects, writes a combined baseline.
///
/// Run with:
/// ```bash
/// cargo test -p perl-lsp-rs --test real_project_latency full_suite -- --include-ignored --nocapture
/// ```
#[test]
#[ignore = "nightly only — requires fixtures and extended runtime"]
fn real_project_latency_full_suite() {
    let fixtures = [&MOJOLICIOUS_FIXTURE, &DANCER2_FIXTURE, &CATALYST_FIXTURE];
    let mut results = Vec::new();
    for fixture in &fixtures {
        let m = measure_project(fixture);
        print_metrics(&m);
        results.push(m);
    }
    write_baseline(&results);
    eprintln!("\nBaseline written to {OUTPUT_PATH}");
}

/// User-facing SLO guard:
/// first publishDiagnostics for a 5,000-line Catalyst app should arrive in <5s.
///
/// Run with:
/// ```bash
/// cargo test -p perl-lsp-rs --test real_project_latency first_diagnostics_5000_line_catalyst -- --include-ignored --nocapture
/// ```
#[test]
#[ignore = "nightly/perf lane — synthetic 5k-line fixture and wall-clock budget"]
fn first_diagnostics_5000_line_catalyst() -> Result<(), Box<dyn std::error::Error>> {
    let app = synthetic_catalyst_app(5000);
    let lines = app.lines().count();
    assert!(lines >= 5000, "Synthetic Catalyst fixture must be >=5000 lines, got {lines}");

    let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO).as_millis();
    let temp_path = std::env::temp_dir().join(format!("perl_lsp_real_perf_{unique}.pm"));
    fs::write(&temp_path, &app)?;

    let uri = file_uri(&temp_path);
    let server = start_lsp_server();
    initialize_lsp(&server);

    let start = Instant::now();
    open_document(&server, &uri, &app);

    let notification = common::read_notification_method(
        &server,
        "textDocument/publishDiagnostics",
        Duration::from_secs(6),
    );
    let elapsed = start.elapsed().as_millis() as u64;

    let _ = fs::remove_file(&temp_path);

    assert!(
        notification.is_some(),
        "No publishDiagnostics received within 6s for 5000-line Catalyst app (elapsed={elapsed}ms)"
    );
    assert!(
        elapsed < 5000,
        "SLO breach: first diagnostics took {elapsed}ms for 5000-line Catalyst app (target <5000ms)"
    );
    Ok(())
}
