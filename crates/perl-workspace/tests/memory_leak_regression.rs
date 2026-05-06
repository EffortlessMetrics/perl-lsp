//! Regression tests for memory leaks in `WorkspaceIndex`.
//!
//! These tests pin down the leak vectors fixed in the
//! `claude/investigate-memory-leak-H0LGP` investigation. Each test isolates
//! one growth path and asserts that memory accounting returns to (or stays
//! near) baseline after the corresponding cleanup.
//!
//! Requires the `memory-profiling` feature so `MemorySnapshot::capture` and
//! `DocumentStore::total_text_bytes` are available.

#![cfg(feature = "memory-profiling")]

use perl_workspace::workspace::memory::MemorySnapshot;
use perl_workspace::workspace_index::WorkspaceIndex;
use url::Url;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn module(idx: usize) -> Result<(Url, String), url::ParseError> {
    let uri = Url::parse(&format!("file:///lib/Leak/Mod{idx}.pm"))?;
    let src = format!(
        r#"package Leak::Mod{idx};
use strict;
use warnings;

sub new {{ return bless {{}}, shift }}
sub run {{ return {idx} }}
sub helper_{idx} {{ my ($self, $x) = @_; return $x + {idx} }}

1;
"#
    );
    Ok((uri, src))
}

/// After indexing N files and then removing all of them, the index must
/// return to the empty-baseline footprint. A leak in any secondary index
/// (`symbols`, `global_references`, semantic shards, document store) shows
/// up as residual bytes after the remove loop.
#[test]
fn remove_all_returns_index_to_empty_baseline() -> TestResult {
    let index = WorkspaceIndex::new();
    let baseline = MemorySnapshot::capture(&index);
    assert_eq!(baseline.total_estimated_bytes(), 0, "fresh index must be zero");

    let count = 200;
    let mut uris = Vec::with_capacity(count);
    for i in 0..count {
        let (uri, src) = module(i)?;
        index.index_file(uri.clone(), src).ok();
        uris.push(uri);
    }

    let after_index = MemorySnapshot::capture(&index);
    assert!(
        after_index.total_estimated_bytes() > 0,
        "index must hold bytes after indexing {count} files"
    );
    assert_eq!(after_index.file_count, count, "all files indexed");

    for uri in &uris {
        index.remove_file(uri.as_str());
    }

    let after_remove = MemorySnapshot::capture(&index);
    assert_eq!(after_remove.file_count, 0, "no files should remain");
    assert_eq!(after_remove.symbol_count, 0, "no symbols should remain");
    assert_eq!(
        after_remove.symbols_bytes, 0,
        "symbols map must be drained on full remove (regression: find_definition fallback or sweep miss)"
    );
    assert_eq!(
        after_remove.global_refs_bytes, 0,
        "global_references must be drained on full remove (regression: defensive sweep miss)"
    );
    assert_eq!(
        after_remove.document_store_bytes, 0,
        "document store must be drained on full remove"
    );
    // files_bytes counts the underlying HashMap keys/values; with all entries
    // removed it must be zero. Allocator bucket capacity is not measured.
    assert_eq!(after_remove.files_bytes, 0, "files map must be empty");
    Ok(())
}

/// `find_definition` must NOT cache misses. Repeatedly looking up names that
/// do not resolve should leave the `symbols` map size unchanged. Before the
/// fix, each fallback inserted an entry that was never invalidated.
#[test]
fn find_definition_does_not_grow_symbols_on_miss() -> TestResult {
    let index = WorkspaceIndex::new();
    for i in 0..50 {
        let (uri, src) = module(i)?;
        index.index_file(uri, src).ok();
    }
    let before = MemorySnapshot::capture(&index);

    // Issue 1000 lookups for names that don't exist anywhere in the index.
    for i in 0..1000 {
        let _ = index.find_definition(&format!("Definitely::Not::Real::sym_{i}"));
    }

    let after = MemorySnapshot::capture(&index);
    assert_eq!(
        after.symbols_bytes, before.symbols_bytes,
        "find_definition cache miss must not grow the symbols map (regression: fallback insertion was reintroduced)"
    );
    assert_eq!(
        after.global_refs_bytes, before.global_refs_bytes,
        "find_definition must not grow the global_references map"
    );
    Ok(())
}

/// Repeated index/remove cycles for the same set of files must not accumulate
/// memory across cycles. This catches subtle leaks where the second remove
/// path cleans less than the first (e.g. shape-dependent sweep skips).
#[test]
fn repeated_index_remove_cycles_do_not_accumulate() -> TestResult {
    let index = WorkspaceIndex::new();

    // Warmup cycle to establish baseline after the first round of allocations.
    for i in 0..40 {
        let (uri, src) = module(i)?;
        index.index_file(uri, src).ok();
    }
    for i in 0..40 {
        let (uri, _) = module(i)?;
        index.remove_file(uri.as_str());
    }
    let baseline = MemorySnapshot::capture(&index);
    assert_eq!(baseline.symbols_bytes, 0, "warmup must drain symbols");
    assert_eq!(baseline.global_refs_bytes, 0, "warmup must drain global_refs");

    for _cycle in 0..20 {
        for i in 0..40 {
            let (uri, src) = module(i)?;
            index.index_file(uri, src).ok();
        }
        for i in 0..40 {
            let (uri, _) = module(i)?;
            index.remove_file(uri.as_str());
        }
    }

    let after = MemorySnapshot::capture(&index);
    assert_eq!(
        after.symbols_bytes, 0,
        "20 index/remove cycles must leave symbols empty (regression: cycle accumulation)"
    );
    assert_eq!(
        after.global_refs_bytes, 0,
        "20 index/remove cycles must leave global_references empty"
    );
    assert_eq!(after.file_count, 0, "no files should remain after cycles");
    Ok(())
}

/// Indexing the same files into two indexes — one in a single shot, one via
/// many cycles — must produce the same memory footprint. If cycle accumulates
/// secondary state, the cycled index will be measurably larger.
#[test]
fn cycle_footprint_matches_single_shot_footprint() -> TestResult {
    let single_shot = WorkspaceIndex::new();
    for i in 0..60 {
        let (uri, src) = module(i)?;
        single_shot.index_file(uri, src).ok();
    }
    let single = MemorySnapshot::capture(&single_shot);

    let cycled = WorkspaceIndex::new();
    for _ in 0..5 {
        for i in 0..60 {
            let (uri, src) = module(i)?;
            cycled.index_file(uri, src).ok();
        }
        for i in 0..60 {
            let (uri, _) = module(i)?;
            cycled.remove_file(uri.as_str());
        }
    }
    // Final indexing pass: leave the cycled index populated.
    for i in 0..60 {
        let (uri, src) = module(i)?;
        cycled.index_file(uri, src).ok();
    }
    let cycled_snap = MemorySnapshot::capture(&cycled);

    assert_eq!(
        cycled_snap.file_count, single.file_count,
        "file counts must match across cycled and single-shot"
    );
    assert_eq!(
        cycled_snap.symbol_count, single.symbol_count,
        "symbol counts must match across cycled and single-shot"
    );
    // Allow modest variance for HashMap re-allocation differences in the
    // measured byte fields, but the cycled index must not be substantially
    // larger than the single-shot one. A 25% upper bound catches gross leaks
    // without flaking on benign re-allocation noise.
    let single_total = single.total_estimated_bytes();
    let cycled_total = cycled_snap.total_estimated_bytes();
    let upper = single_total + single_total / 4;
    assert!(
        cycled_total <= upper,
        "cycled total {cycled_total} exceeds single-shot upper bound {upper} \
         (regression: per-cycle accumulation)"
    );
    Ok(())
}

/// The `bytes_per_symbol` ratio must stay within a sane envelope at moderate
/// scale. Catches gross blow-ups (e.g. accidentally storing the full source
/// text per symbol, or an unbounded per-key candidate cache).
#[test]
fn bytes_per_symbol_stays_under_threshold_at_500_files() -> TestResult {
    let index = WorkspaceIndex::new();
    for i in 0..500 {
        let (uri, src) = module(i)?;
        index.index_file(uri, src).ok();
    }

    let snap = MemorySnapshot::capture(&index);
    assert!(snap.symbol_count >= 500, "expected at least 500 symbols");

    // Current baseline measured on this corpus is well under 2_000 bytes/symbol.
    // The threshold is loose to avoid flaking on minor refactors but tight
    // enough to catch a 5x regression.
    let bps = snap.bytes_per_symbol();
    assert!(
        bps < 5_000,
        "bytes-per-symbol {bps} exceeded the 5_000 byte regression threshold \
         (likely a new unbounded clone or a per-key cache without eviction)"
    );
    Ok(())
}
