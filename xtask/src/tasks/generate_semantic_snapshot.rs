//! Generate or check deterministic HIR semantic snapshots over a corpus slice.
//!
//! # Purpose — SNAPSHOT rail, never "gold"
//!
//! This subcommand exercises `lower_ast()` over a small set of corpus fixtures
//! and records the resulting HIR structure as a JSON manifest. It proves that
//! the lowering pipeline is *deterministic and stable* across commits — not
//! that the output is semantically correct.
//!
//! **Curated-gold correctness** (human-labeled expected facts) is a separate,
//! independent schema and is NOT built here.
//!
//! # KPI
//!
//! `semantic_snapshot_stability_rate` — fraction of snapshot entries that match
//! the recorded reference. NOT `semantic_gold_pass_rate`.
//!
//! # Modes
//!
//! - **generate** (default): run `lower_ast()` over the fixture slice, write
//!   the snapshot manifest to `--output`.
//! - **check**: re-run `lower_ast()` and compare against the recorded manifest
//!   at `--snapshot`. Exits non-zero on any drift.

use color_eyre::eyre::{Context, Result, bail};
use perl_corpus::snapshot::{
    HIR_SCHEMA_VERSION, HirSummary, SnapshotEntry, SnapshotManifest, source_hash,
};
use perl_parser_core::hir::{HirFile, HirKind, lower_ast};
use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Public API (called from main.rs)
// ---------------------------------------------------------------------------

/// Arguments for the `generate-semantic-snapshot` subcommand.
pub struct GenerateSemanticSnapshotArgs {
    /// Directory containing the corpus fixture `.pl` files.
    pub fixture_dir: PathBuf,
    /// Path to write (generate mode) or read (check mode) the snapshot manifest.
    pub output: PathBuf,
    /// When true, compare against `output` and fail on drift. When false, write.
    pub check: bool,
}

/// Entry point called from `main.rs`.
pub fn run(args: GenerateSemanticSnapshotArgs) -> Result<()> {
    let fixtures = collect_fixtures(&args.fixture_dir)?;
    if fixtures.is_empty() {
        bail!("No .pl fixture files found in {}", args.fixture_dir.display());
    }

    let fresh_entries = compute_snapshot_entries(&fixtures)?;

    if args.check {
        run_check(&args.output, &fresh_entries)
    } else {
        run_generate(&args.output, fresh_entries)
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Collect `.pl` fixture files from `dir`, sorted by name for determinism.
fn collect_fixtures(dir: &Path) -> Result<Vec<PathBuf>> {
    if !dir.exists() {
        bail!("Fixture directory not found: {}", dir.display());
    }

    let mut paths: Vec<PathBuf> = fs::read_dir(dir)
        .with_context(|| format!("reading fixture dir {}", dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "pl"))
        .collect();

    paths.sort();
    Ok(paths)
}

/// Run `lower_ast()` over each fixture and compute one `SnapshotEntry` per file.
fn compute_snapshot_entries(fixtures: &[PathBuf]) -> Result<Vec<SnapshotEntry>> {
    fixtures.iter().map(|path| compute_one_entry(path)).collect()
}

fn compute_one_entry(path: &Path) -> Result<SnapshotEntry> {
    let source =
        fs::read_to_string(path).with_context(|| format!("reading fixture {}", path.display()))?;

    let fixture_id = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unknown".to_string());

    let hash = source_hash(&source);
    let hir = lower_source(&source);
    let summary = summarize_hir(&hir);

    Ok(SnapshotEntry {
        fixture_id,
        source_hash: hash,
        hir_schema_version: HIR_SCHEMA_VERSION.to_string(),
        hir_summary: summary,
    })
}

/// Parse source and lower AST to HIR.
fn lower_source(source: &str) -> HirFile {
    let mut parser = perl_parser_core::Parser::new(source);
    let output = parser.parse_with_recovery();
    lower_ast(&output.ast)
}

/// Build a deterministic structural summary from a `HirFile`.
///
/// Excludes raw source offsets (which change on whitespace edits) so that
/// semantics-preserving formatting changes do not cause snapshot drift.
fn summarize_hir(file: &HirFile) -> HirSummary {
    let item_kind_sequence: Vec<String> =
        file.items.iter().map(|item| item_kind_name(&item.kind).to_string()).collect();

    let item_count = item_kind_sequence.len();
    let scope_count = file.scope_graph.scopes.len();
    let binding_count = file.scope_graph.bindings.len();
    let package_count = file.stash_graph.packages.len();
    let slot_count: usize = file.stash_graph.packages.iter().map(|p| p.slots.len()).sum();
    let directive_count = file.compile_environment.directives.len();
    let module_request_count = file.compile_environment.module_requests.len();
    let dynamic_boundary_count = file.compile_environment.dynamic_boundaries.len();

    HirSummary {
        item_count,
        item_kind_sequence,
        scope_count,
        binding_count,
        package_count,
        slot_count,
        directive_count,
        module_request_count,
        dynamic_boundary_count,
    }
}

/// Map a `HirKind` to its stable string name for snapshot keying.
fn item_kind_name(kind: &HirKind) -> &'static str {
    match kind {
        HirKind::PackageDecl(_) => "PackageDecl",
        HirKind::SubDecl(_) => "SubDecl",
        HirKind::MethodDecl(_) => "MethodDecl",
        HirKind::UseDecl(_) => "UseDecl",
        HirKind::RequireDecl(_) => "RequireDecl",
        HirKind::VariableDecl(_) => "VariableDecl",
        HirKind::CallExpr(_) => "CallExpr",
        HirKind::MethodCallExpr(_) => "MethodCallExpr",
        HirKind::IndirectCallExpr(_) => "IndirectCallExpr",
        HirKind::LiteralExpr(_) => "LiteralExpr",
        HirKind::BarewordExpr(_) => "BarewordExpr",
        HirKind::BlockShell(_) => "BlockShell",
        HirKind::BranchShell(_) => "BranchShell",
        HirKind::LoopShell(_) => "LoopShell",
        HirKind::StatementModifierShell(_) => "StatementModifierShell",
        HirKind::ControlTransfer(_) => "ControlTransfer",
        HirKind::DerefExpr(_) => "DerefExpr",
        HirKind::DynamicBoundary(_) => "DynamicBoundary",
        // Catch-all for any variants added in future HIR model versions.
        _ => "Unknown",
    }
}

// ---------------------------------------------------------------------------
// Generate mode
// ---------------------------------------------------------------------------

fn run_generate(output: &Path, entries: Vec<SnapshotEntry>) -> Result<()> {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let mut manifest = SnapshotManifest::new(today);
    manifest.entries = entries;

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating output directory {}", parent.display()))?;
    }

    let json = serde_json::to_string_pretty(&manifest).context("serializing snapshot manifest")?;
    fs::write(output, &json)
        .with_context(|| format!("writing snapshot manifest to {}", output.display()))?;

    println!(
        "generate-semantic-snapshot: wrote {} entries to {}",
        manifest.entries.len(),
        output.display()
    );
    println!(
        "  kpi={} schema={} hir_schema_version={}",
        manifest.kpi, manifest.schema, manifest.hir_schema_version
    );
    println!("  claim_boundary: {}", manifest.claim_boundary);

    Ok(())
}

// ---------------------------------------------------------------------------
// Check mode
// ---------------------------------------------------------------------------

fn run_check(snapshot_path: &Path, fresh: &[SnapshotEntry]) -> Result<()> {
    if !snapshot_path.exists() {
        bail!(
            "Snapshot manifest not found at {}; run without --check to generate it",
            snapshot_path.display()
        );
    }

    let json = fs::read_to_string(snapshot_path)
        .with_context(|| format!("reading snapshot manifest {}", snapshot_path.display()))?;
    let recorded: SnapshotManifest =
        serde_json::from_str(&json).context("parsing snapshot manifest")?;

    // Schema version check.
    if recorded.hir_schema_version != HIR_SCHEMA_VERSION {
        bail!(
            "HIR schema version mismatch: recorded={} current={}; regenerate snapshot",
            recorded.hir_schema_version,
            HIR_SCHEMA_VERSION,
        );
    }

    let (stable, total, rate) = recorded.stability_rate(fresh);

    // Report per-entry results.
    let mut drift_found = false;
    for recorded_entry in &recorded.entries {
        let fresh_match = fresh.iter().find(|f| f.fixture_id == recorded_entry.fixture_id);
        match fresh_match {
            None => {
                eprintln!(
                    "  MISSING fixture: {} (no fresh entry computed)",
                    recorded_entry.fixture_id
                );
                drift_found = true;
            }
            Some(fresh_entry) => {
                if fresh_entry.source_hash != recorded_entry.source_hash {
                    eprintln!(
                        "  SOURCE CHANGED: {} (hash {} -> {})",
                        recorded_entry.fixture_id,
                        recorded_entry.source_hash,
                        fresh_entry.source_hash,
                    );
                    drift_found = true;
                } else if fresh_entry.hir_summary != recorded_entry.hir_summary {
                    eprintln!(
                        "  HIR DRIFT: {} (same source, different HIR structure)",
                        recorded_entry.fixture_id
                    );
                    eprintln!(
                        "    recorded item_count={} fresh item_count={}",
                        recorded_entry.hir_summary.item_count, fresh_entry.hir_summary.item_count,
                    );
                    drift_found = true;
                } else {
                    println!("  OK: {}", recorded_entry.fixture_id);
                }
            }
        }
    }

    println!("semantic_snapshot_stability_rate: {}/{} = {:.1}%", stable, total, rate * 100.0);

    if drift_found || stable < total {
        bail!(
            "Snapshot check failed: {}/{} stable. \
             Run without --check to regenerate the snapshot manifest.",
            stable,
            total
        );
    }

    println!("Snapshot check passed: all {} entries stable.", total);
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;
    use tempfile::TempDir;

    fn write_fixture(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        let mut f = fs::File::create(&path).expect("create fixture");
        f.write_all(content.as_bytes()).expect("write fixture");
        path
    }

    #[test]
    fn generates_snapshot_for_minimal_source() {
        let tmp = TempDir::new().expect("tempdir");
        let src = "package Foo;\nsub bar { return 1; }\n1;\n";
        write_fixture(tmp.path(), "minimal.pl", src);

        let out = tmp.path().join("snapshot.json");
        let args = GenerateSemanticSnapshotArgs {
            fixture_dir: tmp.path().to_owned(),
            output: out.clone(),
            check: false,
        };
        run(args).expect("generate should succeed");

        let json = fs::read_to_string(&out).expect("read output");
        let manifest: SnapshotManifest = serde_json::from_str(&json).expect("deserialize manifest");

        assert_eq!(manifest.schema, "semantic_snapshot.v1");
        assert_eq!(manifest.kpi, "semantic_snapshot_stability_rate");
        assert_eq!(manifest.hir_schema_version, HIR_SCHEMA_VERSION);
        assert_eq!(manifest.entries.len(), 1);

        let entry = &manifest.entries[0];
        assert_eq!(entry.fixture_id, "minimal");
        assert_eq!(entry.source_hash, source_hash(src));
        // The lowerer must produce at least one item (PackageDecl or SubDecl).
        assert!(entry.hir_summary.item_count > 0, "expected HIR items from non-trivial source");
    }

    #[test]
    fn snapshot_names_typed_dereference_items() {
        let tmp = TempDir::new().expect("tempdir");
        let src = "my @arr = @{\"foo\"};\n";
        write_fixture(tmp.path(), "deref.pl", src);

        let out = tmp.path().join("snapshot.json");
        run(GenerateSemanticSnapshotArgs {
            fixture_dir: tmp.path().to_owned(),
            output: out.clone(),
            check: false,
        })
        .expect("generate should succeed");

        let json = fs::read_to_string(&out).expect("read output");
        let manifest: SnapshotManifest = serde_json::from_str(&json).expect("deserialize manifest");
        assert!(
            manifest.entries[0]
                .hir_summary
                .item_kind_sequence
                .iter()
                .any(|kind| kind == "DerefExpr"),
            "typed dereference items must have a stable snapshot name"
        );
    }

    #[test]
    fn check_mode_passes_when_snapshot_matches() {
        let tmp = TempDir::new().expect("tempdir");
        let src = "package Bar;\nsub baz { 0 }\n1;\n";
        write_fixture(tmp.path(), "bar.pl", src);
        let out = tmp.path().join("snap.json");

        // Generate.
        run(GenerateSemanticSnapshotArgs {
            fixture_dir: tmp.path().to_owned(),
            output: out.clone(),
            check: false,
        })
        .expect("generate");

        // Check — should pass because source hasn't changed.
        run(GenerateSemanticSnapshotArgs {
            fixture_dir: tmp.path().to_owned(),
            output: out.clone(),
            check: true,
        })
        .expect("check should pass on unchanged source");
    }

    #[test]
    fn check_mode_fails_on_source_change() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("ch.pl");
        fs::write(&path, "package Ch; sub a { 1 } 1;").expect("write v1");
        let out = tmp.path().join("snap.json");

        // Generate with v1 source.
        run(GenerateSemanticSnapshotArgs {
            fixture_dir: tmp.path().to_owned(),
            output: out.clone(),
            check: false,
        })
        .expect("generate v1");

        // Overwrite with different source.
        fs::write(&path, "package Ch; sub a { 1 } sub b { 2 } 1;").expect("write v2");

        // Check should fail because source hash changed.
        let result = run(GenerateSemanticSnapshotArgs {
            fixture_dir: tmp.path().to_owned(),
            output: out.clone(),
            check: true,
        });
        assert!(result.is_err(), "check must fail when source changes");
    }

    #[test]
    fn snapshot_covers_slice_fixtures() {
        // Validate that all three bundled slice fixtures produce non-empty HIR.
        let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../crates/perl-corpus/fixtures/snapshot-slice");

        if !fixture_dir.exists() {
            // Fixture dir only present in the workspace; skip in isolated builds.
            return;
        }

        let fixtures = collect_fixtures(&fixture_dir).expect("collect fixtures");
        assert!(fixtures.len() >= 3, "expected at least 3 slice fixtures, got {}", fixtures.len());

        let entries = compute_snapshot_entries(&fixtures).expect("compute entries");
        for entry in &entries {
            assert!(
                entry.hir_summary.item_count > 0,
                "fixture {} produced empty HIR",
                entry.fixture_id
            );
            assert_eq!(entry.hir_schema_version, HIR_SCHEMA_VERSION);
        }
    }
}
