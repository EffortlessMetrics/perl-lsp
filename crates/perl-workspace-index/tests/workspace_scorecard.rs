//! Workspace & Indexing Scorecard — stage 1 metric harness.
//!
//! Measures three operational signals against the SLO targets from
//! `perl-workspace-index-slo`:
//!
//! 1. **Stale-index defect rate** — after a file is deleted, its symbols
//!    must no longer appear in completion/reference results. Target: 0%.
//!
//! 2. **Incremental reindex correctness** — after content is replaced, only
//!    the new symbols are present. Target: 0 stale symbols.
//!
//! 3. **Multi-file isolation** — removing file A must not erase symbols from
//!    file B. Target: 100% of B's symbols survive.
//!
//! These tests surface to `docs/project/status/workspace.md` via
//! `cargo xtask update-status --only workspace`.
//!
//! Related: issue #4068, PR #4137 (CI wiring), SLO spec in
//! `crates/perl-workspace-index-slo/src/lib.rs`.

use perl_workspace_index::workspace::workspace_index::WorkspaceIndex;
use std::time::Instant;
use url::Url;
use walkdir::WalkDir;

// ---------------------------------------------------------------------------
// Helper: build a file URI without panic
// ---------------------------------------------------------------------------

fn uri(path: &str) -> Url {
    Url::parse(&format!("file://{path}")).expect("valid test URI")
}

// ---------------------------------------------------------------------------
// Metric 1 — Stale-index defect rate (file delete)
//
// Scenario: index a file with a known symbol, delete the file, then query.
// Expected: find_definition and find_references both return empty / None.
// ---------------------------------------------------------------------------

/// After a file is removed, find_definition must return None.
#[test]
fn scorecard_stale_definition_after_file_delete() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let file_uri = uri("/workspace/scorecard/stale_def_test.pl");

    // Phase 1: index a file containing `sub stale_helper`
    index.index_file(
        file_uri.clone(),
        "package Scorecard;\nsub stale_helper { return 42; }\n".to_string(),
    )?;

    // Verify it is indexed
    assert!(
        index.find_definition("Scorecard::stale_helper").is_some(),
        "symbol should be present after indexing"
    );
    assert!(
        index.find_definition("stale_helper").is_some(),
        "bare-name symbol should be present after indexing"
    );

    // Phase 2: delete the file (simulates user deleting the file on disk)
    index.remove_file(&file_uri.to_string());

    // Phase 3: assert no stale results — defect rate must be 0
    assert!(
        index.find_definition("Scorecard::stale_helper").is_none(),
        "STALE INDEX DEFECT: qualified symbol persists after file deletion — \
         this degrades goto-definition and rename"
    );
    assert!(
        index.find_definition("stale_helper").is_none(),
        "STALE INDEX DEFECT: bare-name symbol persists after file deletion"
    );

    Ok(())
}

/// After a file is removed, find_references must return empty.
#[test]
fn scorecard_stale_references_after_file_delete() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let file_uri = uri("/workspace/scorecard/stale_refs_test.pl");

    index.index_file(
        file_uri.clone(),
        "package RefTest;\nsub helper { 1 }\nhelper();\nRefTest::helper();\n".to_string(),
    )?;

    // Baseline: references exist
    assert!(
        !index.find_references("RefTest::helper").is_empty(),
        "references should exist before deletion"
    );

    index.remove_file(&file_uri.to_string());

    // After deletion: no stale references
    assert!(
        index.find_references("RefTest::helper").is_empty(),
        "STALE INDEX DEFECT: references persist after file deletion (qualified)"
    );
    assert!(
        index.find_references("helper").is_empty(),
        "STALE INDEX DEFECT: references persist after file deletion (bare)"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Metric 2 — Incremental reindex correctness (content replacement)
//
// Scenario: index a file with symbol A, replace its content with symbol B,
// re-index. Expect: A is gone, B is present.
// ---------------------------------------------------------------------------

/// After content replacement, only the new symbol is present.
#[test]
fn scorecard_incremental_reindex_removes_old_symbol() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let file_uri = uri("/workspace/scorecard/incremental_test.pl");

    // Index v1: contains `old_symbol`
    index
        .index_file(file_uri.clone(), "package Inc;\nsub old_symbol { return 1; }\n".to_string())?;
    assert!(index.find_definition("Inc::old_symbol").is_some(), "old_symbol should be indexed");

    // Re-index the same URI with v2: `old_symbol` removed, `new_symbol` added
    // Simulates user saving a file after renaming the function
    index
        .index_file(file_uri.clone(), "package Inc;\nsub new_symbol { return 2; }\n".to_string())?;

    // new_symbol must be present
    assert!(
        index.find_definition("Inc::new_symbol").is_some(),
        "new_symbol should appear after re-index"
    );

    // old_symbol must be gone — the most common stale-index failure mode
    assert!(
        index.find_definition("Inc::old_symbol").is_none(),
        "STALE INDEX DEFECT: old_symbol persists after content replacement — \
         incremental reindex did not clear previous symbols for this file"
    );

    Ok(())
}

/// Incremental reindex latency is within the SLO target of 100ms P95.
#[test]
fn scorecard_incremental_reindex_latency_within_slo() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    // Take 20 samples. With n=20, `(20 * 0.95) as usize = 19`, which selects
    // the maximum value — this is intentionally conservative: for a small smoke
    // sample the SLO assertion "every single reindex fits in 100ms" is strictly
    // stronger than a true P95 and appropriate for a unit-scale test.
    let mut latencies_ms = Vec::with_capacity(20);

    for i in 0..20u32 {
        let file_uri = uri(&format!("/workspace/scorecard/latency_slo_{i}.pl"));
        // Initial index
        index.index_file(
            file_uri.clone(),
            format!("package SloTest{i};\nsub func_{i} {{ return {i}; }}\n"),
        )?;

        // Measure incremental re-index (content change)
        let t0 = Instant::now();
        index.index_file(
            file_uri.clone(),
            format!("package SloTest{i};\nsub func_{i}_v2 {{ return {}; }}\n", i + 100),
        )?;
        latencies_ms.push(t0.elapsed().as_millis() as u64);
    }

    latencies_ms.sort_unstable();
    // With n=20 this resolves to index 19 (the maximum); see comment above.
    let p95_idx = (latencies_ms.len() as f64 * 0.95) as usize;
    let p95_ms = latencies_ms[p95_idx.min(latencies_ms.len() - 1)];

    // SLO target from perl-workspace-index-slo: incremental_update_p95_ms = 100ms
    assert!(p95_ms <= 100, "SLO BREACH: incremental reindex P95 = {p95_ms}ms exceeds 100ms target");

    Ok(())
}

// ---------------------------------------------------------------------------
// Metric 3 — Multi-file isolation (cross-file symbol independence)
//
// Scenario: two files are indexed; removing file A must not affect symbols
// from file B.
// ---------------------------------------------------------------------------

/// Removing file A preserves all symbols from file B (100% isolation).
#[test]
fn scorecard_file_removal_isolation() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    let uri_a = uri("/workspace/scorecard/isolation_a.pl");
    let uri_b = uri("/workspace/scorecard/isolation_b.pl");

    index.index_file(uri_a.clone(), "package IsoA;\nsub func_a { return 'a'; }\n".to_string())?;
    index.index_file(uri_b.clone(), "package IsoB;\nsub func_b { return 'b'; }\n".to_string())?;

    // Both symbols present
    assert!(index.find_definition("IsoA::func_a").is_some());
    assert!(index.find_definition("IsoB::func_b").is_some());

    // Remove only file A
    index.remove_file(&uri_a.to_string());

    // A's symbol must be gone
    assert!(
        index.find_definition("IsoA::func_a").is_none(),
        "IsoA::func_a should be removed after deleting isolation_a.pl"
    );

    // B's symbol must still be present — this tests cross-file isolation
    assert!(
        index.find_definition("IsoB::func_b").is_some(),
        "ISOLATION FAILURE: IsoB::func_b disappeared after removing a different file — \
         cross-file index corruption"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Metric 4 — Symbol count integrity
//
// After a clear(), the index must return to a clean empty state.
// ---------------------------------------------------------------------------

/// Index state is clean after clear().
#[test]
fn scorecard_clear_returns_to_empty() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();

    // Index several files
    for i in 0..5 {
        index.index_file(
            uri(&format!("/workspace/scorecard/clear_test_{i}.pl")),
            format!("package Clear{i};\nsub sub_{i} {{ {i} }}\n"),
        )?;
    }

    assert!(index.file_count() > 0, "files should be indexed before clear");
    assert!(index.symbol_count() > 0, "symbols should exist before clear");

    index.clear();

    assert_eq!(index.file_count(), 0, "file_count should be 0 after clear()");
    assert_eq!(index.symbol_count(), 0, "symbol_count should be 0 after clear()");
    assert!(!index.has_symbols(), "has_symbols should be false after clear()");

    Ok(())
}

// ---------------------------------------------------------------------------
// Metric 5 — Fixture workspace scale coverage
//
// Verify that the 4-scale fixture workspaces exist with the expected file
// counts. This test catches regressions where fixture generation is skipped
// or files are accidentally deleted.
// ---------------------------------------------------------------------------

#[test]
fn scorecard_fixture_workspaces_exist_at_expected_scales() {
    // Locate the workspace root relative to this test file.
    // CARGO_MANIFEST_DIR is set by cargo to the crate root.
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // crates/perl-workspace-index -> ../.. -> repo root -> test_corpus/workspaces
    let workspaces_dir = manifest_dir.join("../../test_corpus/workspaces");

    // small/medium/large are committed; xlarge is generated on demand (see README).
    // The file-count assertion only applies to committed fixtures.
    let committed_scales = [("small", 10usize), ("medium", 100), ("large", 1000)];

    for (name, min_count) in committed_scales {
        let dir = workspaces_dir.join(name);
        assert!(
            dir.exists(),
            "fixture workspace 'test_corpus/workspaces/{name}/' is missing — \
             this is a committed fixture and should always exist"
        );
        assert!(dir.is_dir(), "test_corpus/workspaces/{name} exists but is not a directory");

        // Count .pl and .pm files recursively
        let count = WalkDir::new(&dir)
            .into_iter()
            .filter_map(|e: Result<walkdir::DirEntry, _>| e.ok())
            .filter(|e: &walkdir::DirEntry| {
                e.file_type().is_file()
                    && e.path().extension().map(|ext| ext == "pl" || ext == "pm").unwrap_or(false)
            })
            .count();

        assert!(
            count >= min_count,
            "fixture workspace '{name}' has only {count} files, expected >= {min_count}"
        );
    }

    // xlarge (10k files) is generated on demand; only verify the directory structure exists.
    // Run `bash scripts/gen-xlarge-workspace.sh` to regenerate.
    // File count is not asserted here because committing 10k files to git is impractical.
    let xlarge_dir = workspaces_dir.join("xlarge");
    assert!(
        xlarge_dir.exists(),
        "fixture workspace 'test_corpus/workspaces/xlarge/' is missing — \
         run `bash scripts/gen-xlarge-workspace.sh` to generate it"
    );
}
