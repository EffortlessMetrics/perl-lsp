//! Corpus differential test — walks the project's real-world Perl corpora and
//! asserts that v3 does not regress below a recorded baseline.
//!
//! # What this tests
//!
//! This test walks the same corpus paths used by the project's parser tests and
//! runs all three parsers (v1/v2/v3) on every `.pl`, `.pm`, and `.t` file.  It
//! records per-parser outcomes and asserts:
//!
//! 1. **No crashes** — no parser should panic on any corpus file.
//! 2. **v3 clean rate does not regress** — v3 is the production parser; its
//!    clean-parse count must meet or exceed the recorded baseline.
//!
//! # Baseline
//!
//! The baseline is determined empirically by running this test and recording the
//! actual numbers.  When v3 improves, update `BASELINE_V3_CLEAN_MIN` upward.
//! Do not lower it without a comment explaining why.
//!
//! # Performance
//!
//! With ~1 200 corpus files and three parsers per file, expect ~30-60s on a
//! developer machine.  The full test is annotated with `#[ignore]` so it does
//! not run in the default `cargo test` invocation.  Run explicitly with:
//!
//! ```bash
//! cargo test -p perl-parser-comparison --test corpus_differential -- --ignored --nocapture
//! ```
//!
//! The smoke test (top-level test_corpus only) runs automatically.

use std::collections::HashMap;
use std::path::PathBuf;

use perl_parser_comparison::{AggregateStats, DisagreementKind, Verdict, classify, walk_corpora};

// ─── Baselines ────────────────────────────────────────────────────────────────
//
// Baselines established from corpus run (2026-05-16) on 1268-file corpus:
//   v3 clean: 1242, v3 errors: 26, crashes: 0
//
// Use 95% of empirical value as floor to allow minor variance without
// false-positive failures.  Update the constant upward when v3 improves.

/// Minimum number of files v3 must parse cleanly (Correct verdict).
/// Empirical baseline: 1242; floor set at 95% = 1179.
const BASELINE_V3_CLEAN_MIN: usize = 1179;

/// Maximum allowed crashes across all parsers.
const BASELINE_MAX_CRASHES: usize = 0;

// ─── Corpus root helpers ──────────────────────────────────────────────────────

/// Locate the workspace root from CARGO_MANIFEST_DIR or current directory.
fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is set by cargo when running tests
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let manifest = PathBuf::from(manifest_dir);
        // crates/perl-parser-comparison -> workspace root is two levels up
        if let Some(root) = manifest.parent().and_then(|p| p.parent()) {
            if root.join("Cargo.toml").exists() {
                return root.to_owned();
            }
        }
    }
    // Fallback: walk up from cwd looking for workspace Cargo.toml
    let mut dir = match std::env::current_dir() {
        Ok(d) => d,
        Err(_) => return PathBuf::from("."),
    };
    loop {
        let candidate = dir.join("Cargo.toml");
        if candidate.exists() {
            if let Ok(content) = std::fs::read_to_string(&candidate) {
                if content.contains("[workspace]") {
                    return dir;
                }
            }
        }
        match dir.parent() {
            Some(p) => dir = p.to_owned(),
            None => return PathBuf::from("."),
        }
    }
}

fn full_corpus_roots() -> Vec<PathBuf> {
    let root = workspace_root();
    ["test_corpus", "tree-sitter-perl/test/highlight"]
        .iter()
        .map(|rel| root.join(rel))
        .filter(|p| p.exists())
        .collect()
}

// ─── Tests ────────────────────────────────────────────────────────────────────

/// Full corpus differential run.  Marked `#[ignore]` — run with `-- --ignored`.
///
/// Produces a human-readable report on stdout when run with `--nocapture`.
#[test]
#[ignore]
fn corpus_differential_full() {
    let roots = full_corpus_roots();
    assert!(!roots.is_empty(), "No corpus roots found — is the workspace root reachable?");

    println!("\n=== Corpus Differential: Full Run ===");
    for root in &roots {
        println!("  root: {}", root.display());
    }

    let records = walk_corpora(&roots);
    let stats = AggregateStats::from_records(&records);

    // Print the report
    let report = perl_parser_comparison::format_report(&records, &stats);
    println!("{report}");

    // ─── Assertion 1: no crashes ──────────────────────────────────────────────
    let total_crashes = stats.v1_crashes + stats.v2_crashes + stats.v3_crashes;
    assert!(
        total_crashes == BASELINE_MAX_CRASHES,
        "Parser crashes detected: v1={} v2={} v3={} (total={}, expected_max={})\n\
         Crashing files:\n{}",
        stats.v1_crashes,
        stats.v2_crashes,
        stats.v3_crashes,
        total_crashes,
        BASELINE_MAX_CRASHES,
        records
            .iter()
            .filter(|r| {
                matches!(r.v1, Verdict::Crashes)
                    || matches!(r.v2, Verdict::Crashes)
                    || matches!(r.v3, Verdict::Crashes)
            })
            .map(|r| format!("  {} v1={} v2={} v3={}", r.path.display(), r.v1, r.v2, r.v3))
            .collect::<Vec<_>>()
            .join("\n"),
    );

    // ─── Assertion 2: v3 clean rate does not regress ──────────────────────────
    assert!(
        stats.v3_clean >= BASELINE_V3_CLEAN_MIN,
        "v3 clean-parse count regressed: got {} but baseline requires >= {}\n\
         (run with --nocapture to see the full disagreement table)",
        stats.v3_clean,
        BASELINE_V3_CLEAN_MIN,
    );

    println!("\n  Summary:");
    println!("    Total files:          {}", stats.total);
    println!("    Total disagreements:  {}", stats.total_disagreements());
    println!("    v3 clean:             {}", stats.v3_clean);
    println!("    v3 errors:            {}", stats.v3_errors);
    println!("    v3 crashes:           {}", stats.v3_crashes);
    println!("\n  Disagreement breakdown:");
    println!("    all_agree:             {}", stats.all_agree);
    println!("    recovery_disagreement: {}", stats.recovery_disagreement);
    println!("    v3_only_clean:         {}", stats.v3_only_clean);
    println!("    v2_only_clean:         {}", stats.v2_only_clean);
    println!("    v1_only_clean:         {}", stats.v1_only_clean);
    println!("    each_disagrees:        {}", stats.each_disagrees);
}

/// Lightweight smoke test — only walks the top-level test_corpus directory.
/// Runs in the default `cargo test` invocation (no `--ignored` needed).
///
/// Asserts:
/// - At least 10 files were found (sanity check that corpus is reachable)
/// - No parser crashes
/// - Disagrement counts are tracked and printed
#[test]
fn corpus_differential_smoke() {
    let root = workspace_root();
    // Walk the full test_corpus directory (fast: ~1256 files, ~4s total)
    let smoke_root = root.join("test_corpus");
    if !smoke_root.exists() {
        println!("Skipping smoke test: test_corpus not found at {}", smoke_root.display());
        return;
    }

    let records = walk_corpora(&[smoke_root]);

    // Basic sanity: corpus was reachable
    assert!(records.len() >= 10, "Expected at least 10 corpus files, found {}", records.len());

    let stats = AggregateStats::from_records(&records);

    println!("\n=== Corpus Differential: Smoke Run ===");
    println!("  Files processed:      {}", stats.total);
    println!("  Disagreements:        {}", stats.total_disagreements());
    println!("  v3 clean:             {}", stats.v3_clean);
    println!("  v3 errors:            {}", stats.v3_errors);
    println!("  v3 crashes:           {}", stats.v3_crashes);

    // Per-disagreement-kind counts
    let mut kind_counts: HashMap<String, usize> = HashMap::new();
    for r in &records {
        *kind_counts.entry(r.disagreement.to_string()).or_default() += 1;
    }
    let mut kinds: Vec<_> = kind_counts.iter().collect();
    kinds.sort_by_key(|(k, _)| k.as_str());
    println!("\n  Disagreement breakdown:");
    for (kind, count) in &kinds {
        println!("    {:30} {}", kind, count);
    }

    // No crashes allowed — a crash on corpus is always a parser bug
    let total_crashes = stats.v1_crashes + stats.v2_crashes + stats.v3_crashes;
    assert!(
        total_crashes == 0,
        "Parser crashes in smoke run: v1={} v2={} v3={}; crashing files:\n{}",
        stats.v1_crashes,
        stats.v2_crashes,
        stats.v3_crashes,
        records
            .iter()
            .filter(|r| {
                matches!(r.v1, Verdict::Crashes)
                    || matches!(r.v2, Verdict::Crashes)
                    || matches!(r.v3, Verdict::Crashes)
            })
            .map(|r| format!("  {}", r.path.display()))
            .collect::<Vec<_>>()
            .join("\n"),
    );
}

/// Unit test for the classify() function.
#[test]
fn classify_unit_tests() {
    // All agree — clean
    assert_eq!(
        classify(&Verdict::Correct, &Verdict::Correct, &Verdict::Correct),
        DisagreementKind::AllAgree
    );
    // All agree — same error kind
    assert_eq!(
        classify(&Verdict::Errors, &Verdict::Errors, &Verdict::Errors),
        DisagreementKind::AllAgree
    );

    // V3-only clean
    assert_eq!(
        classify(&Verdict::Errors, &Verdict::Errors, &Verdict::Correct),
        DisagreementKind::V3OnlyClean
    );

    // V2-only clean
    assert_eq!(
        classify(&Verdict::Errors, &Verdict::Correct, &Verdict::Errors),
        DisagreementKind::V2OnlyClean
    );

    // V1-only clean
    assert_eq!(
        classify(&Verdict::Correct, &Verdict::Errors, &Verdict::Errors),
        DisagreementKind::V1OnlyClean
    );

    // Recovery disagreement: mixed non-clean kinds
    assert_eq!(
        classify(&Verdict::Errors, &Verdict::WrongButPlausible, &Verdict::Errors),
        DisagreementKind::RecoveryDisagreement
    );

    // Recovery disagreement: two clean, one not
    assert_eq!(
        classify(&Verdict::Correct, &Verdict::Correct, &Verdict::Errors),
        DisagreementKind::RecoveryDisagreement
    );

    // Recovery disagreement: one clean, two not (same kind)
    assert_eq!(
        classify(&Verdict::Correct, &Verdict::Errors, &Verdict::Correct),
        DisagreementKind::RecoveryDisagreement
    );

    // Crashes count as non-clean
    assert_eq!(
        classify(&Verdict::Crashes, &Verdict::Errors, &Verdict::Correct),
        DisagreementKind::V3OnlyClean
    );
}
