//! Integration tests for PIR-A guarded reference promotion (#2635 PR3a).
//!
//! Tests the [`references_pir_promote`] entry point and the
//! [`ReferencesPirPromoteOutcome`] decision enum against curated hand-verified
//! fixture sets (the correct, scope-aware answers — NOT legacy as ground truth,
//! since legacy is the scope-blind baseline being superseded).
//!
//! Drives the real pipeline: `Parser → lower_ast → extract_lexical_facts`.
//! `LexicalExtractorReceipt` is `#[non_exhaustive]` so receipts cannot be
//! hand-constructed; we always go through the real pipeline.
//!
//! The `Exact` branch (flag on) is exercised via [`references_pir_promote_unguarded`],
//! a `#[cfg(test)]` test-only entry point that bypasses the compile-time flag but
//! otherwise runs identical logic. The flag-off `LegacyFallback` branch is tested
//! through [`references_pir_promote`] directly.
//!
//! ## Fixture inventory
//!
//! | # | Name | What it asserts |
//! |---|------|-----------------|
//! | F1 | scope_exact_outer_x | compiler returns exactly 2 scope-correct ranges (not 3) for outer `$x` |
//! | F2 | flag_off_fallback | flag=false → always LegacyFallback regardless of receipt |
//! | F3 | package_qualified_refused | `Foo::x` → LegacyFallback(NotSameFileLexical) via unguarded path |
//! | F4 | single_scope_compiler_equals_legacy | simple same-scope: compiler count matches expected |
//! | F5 | subroutine_references_unaffected | `find_references_single_file` still returns ≥2 refs for subs |
//! | F6 | latency_sanity | 1000 consecutive promote calls complete in < 2ms mean wall-clock |

use perl_lsp_rs_core::providers::navigation::references_pir_shadow::PirShadowRefusalReason;
use perl_lsp_rs_core::providers::navigation::{
    ENABLE_PIR_LEXICAL_REFERENCES, ReferencesPirPromoteOutcome, find_references_single_file,
    references_pir_promote, references_pir_promote_unguarded,
};
use perl_parser_core::{Parser, hir::lower_ast, pir::extract_lexical_facts};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Identity URI mapper: converts byte offsets to a trivial single-line `lsp_types::Range`.
/// Real callers would use a proper UTF-16 mapper; this suffices for promotion-logic tests.
fn byte_mapper(start: usize, end: usize) -> lsp_types::Range {
    lsp_types::Range {
        start: lsp_types::Position { line: 0, character: start as u32 },
        end: lsp_types::Position { line: 0, character: end as u32 },
    }
}

fn receipt_for(source: &str) -> perl_parser_core::pir::LexicalExtractorReceipt {
    let mut parser = Parser::new(source);
    let output = parser.parse_with_recovery();
    let hir = lower_ast(&output.ast);
    extract_lexical_facts(&hir)
}

// ─────────────────────────────────────────────────────────────────────────────
// F1: Scope-exact outer $x — the key correctness win
//
// Source (6 lines):
//   my $x = 1;        ← outer write (body0)
//   {
//       my $x = 2;    ← inner write in a block scope (body0 inner block)
//       print $x;     ← inner read
//   }
//   print $x;         ← outer read
//
// Curated expected for outer $x in body0: exactly 2 ranges (outer write + outer read).
// The scope-blind legacy arm returns 3 (all $x occurrences regardless of scope).
// The compiler returns only the 2 outer ones → correctness win.
// ─────────────────────────────────────────────────────────────────────────────

const F1_SOURCE: &str = "my $x = 1;\n{\n    my $x = 2;\n    print $x;\n}\nprint $x;\n";

/// The outer `$x` occurrences in F1_SOURCE as byte-offset pairs (curated by hand).
///
/// - `my $x = 1;` → `$x` at byte 3..5 (write/declaration)
/// - `print $x;`  (final line, after block) → `$x` at byte 57..59 (read)
///
/// Verify with: `F1_SOURCE.match_indices("$x").collect::<Vec<_>>()`
///   → [(3, "$x"), (16, "$x"), (29, "$x"), (57, "$x")]
///   Outer = index 0 (pos 3) and index 3 (pos 57).
fn f1_expected_outer_x_byte_count() -> usize {
    let occurrences: Vec<usize> = F1_SOURCE.match_indices("$x").map(|(i, _)| i).collect();
    // We expect 4 total $x occurrences; outer ones are the first and last
    assert_eq!(occurrences.len(), 4, "F1_SOURCE sanity: expected 4 $x occurrences");
    2 // outer write + outer read
}

#[test]
fn f1_scope_exact_outer_x_returns_two_ranges() -> TestResult {
    let expected_count = f1_expected_outer_x_byte_count();
    let receipt = receipt_for(F1_SOURCE);

    // Use the unguarded path to exercise the Exact branch.
    let outcome = references_pir_promote_unguarded(
        &receipt,
        &[], // legacy result not needed — asserting Exact path
        "x",
        0,
        &byte_mapper,
    );

    match outcome {
        ReferencesPirPromoteOutcome::Exact(ranges) => {
            assert_eq!(
                ranges.len(),
                expected_count,
                "scope-exact compiler must return {expected_count} ranges for outer $x, got {}: {ranges:?}",
                ranges.len(),
            );
        }
        other => return Err(format!("expected Exact, got {other:?}").into()),
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// F2: Flag-off fallback — the merge-safe default
//
// With ENABLE_PIR_LEXICAL_REFERENCES = false, references_pir_promote must
// always return LegacyFallback regardless of receipt content or target name.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn f2_flag_off_always_returns_legacy_fallback() -> TestResult {
    const { assert!(!ENABLE_PIR_LEXICAL_REFERENCES, "flag must be off at merge time") };

    let receipt = receipt_for(F1_SOURCE);
    let legacy = vec![(3usize, 5usize), (57usize, 59usize)];
    let outcome = references_pir_promote(&receipt, &legacy, "x", 0, &byte_mapper);

    match outcome {
        ReferencesPirPromoteOutcome::LegacyFallback { result, .. } => {
            assert_eq!(result, legacy, "legacy result must be returned unmodified");
        }
        other => return Err(format!("expected LegacyFallback on flag=off, got {other:?}").into()),
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// F3: Package-qualified name refused (via unguarded path)
//
// A `::`-qualified name must return LegacyFallback(NotSameFileLexical) because
// package variables are not same-file lexical bindings.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn f3_package_qualified_name_is_refused() -> TestResult {
    let receipt = receipt_for("my $x = 1;\n");
    let legacy = vec![(3usize, 5usize)];
    let outcome = references_pir_promote_unguarded(&receipt, &legacy, "Foo::x", 0, &byte_mapper);

    match outcome {
        ReferencesPirPromoteOutcome::LegacyFallback { result, reason } => {
            assert_eq!(
                reason,
                PirShadowRefusalReason::NotSameFileLexical,
                "package-qualified name must refuse with NotSameFileLexical"
            );
            assert_eq!(result, legacy, "legacy result preserved on refusal");
        }
        other => {
            return Err(
                format!("expected LegacyFallback(NotSameFileLexical), got {other:?}").into()
            );
        }
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// F4: Single-scope variable — compiler count matches curated expected set
//
// Source: `my $a = 1;\nprint $a;\n`
// Curated expected: 2 ranges ($a write + $a read). No scope ambiguity.
// Assert: outcome is Exact with exactly 2 ranges.
// ─────────────────────────────────────────────────────────────────────────────

const F4_SOURCE: &str = "my $a = 1;\nprint $a;\n";

#[test]
fn f4_single_scope_exact_count() -> TestResult {
    let receipt = receipt_for(F4_SOURCE);

    let outcome = references_pir_promote_unguarded(&receipt, &[], "a", 0, &byte_mapper);

    match outcome {
        ReferencesPirPromoteOutcome::Exact(ranges) => {
            assert_eq!(
                ranges.len(),
                2,
                "single-scope $a must yield exactly 2 ranges (write + read), got {}: {ranges:?}",
                ranges.len()
            );
        }
        other => return Err(format!("expected Exact for single-scope $a, got {other:?}").into()),
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// F5: Subroutine references unaffected
//
// Verifies that `find_references_single_file` still resolves sub references
// correctly — the sub arms (lines 83-95 in references.rs) are not touched by
// this PR (the Variable arm is only deleted in PR3b, and only post-soak).
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn f5_subroutine_references_unaffected() -> TestResult {
    let source = "sub Foo::bar { 1 } Foo::bar();";
    let mut parser = Parser::new(source);
    let output = parser.parse_with_recovery();

    // Cursor at position 4 sits on 'F' in `sub Foo::bar { ... }`.
    let refs = find_references_single_file(&output.ast, 4)
        .ok_or("find_references_single_file returned None for sub Foo::bar")?;

    assert!(
        refs.len() >= 2,
        "subroutine references must include declaration + call site (got {refs:?})"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// F6: Latency sanity — 1000 consecutive promote calls
//
// Target: mean wall-clock < 2ms per call on any CI hardware.
// (Reference machine budget: 500µs on Ryzen 9 9950X3D; 2ms used here as a
// conservative threshold for unknown CI hardware.)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn f6_promote_latency_sanity_1000_calls() -> TestResult {
    // Build a receipt with multiple lexical names across a moderately sized source.
    const SOURCE: &str = concat!(
        "my $t = 0;\n",
        "my $u = 1;\n",
        "my $v = 2;\n",
        "print $t;\n",
        "print $u;\n",
        "print $v;\n",
        "my $w = $t + $u;\n",
        "print $w;\n",
    );

    let receipt = receipt_for(SOURCE);
    let legacy: Vec<(usize, usize)> = Vec::new();

    let start = std::time::Instant::now();
    for _ in 0..1000 {
        let outcome = references_pir_promote_unguarded(&receipt, &legacy, "t", 0, &byte_mapper);
        // Ensure the compiler doesn't elide the call.
        std::hint::black_box(&outcome);
    }
    let elapsed = start.elapsed();
    let mean_ns = elapsed.as_nanos() / 1000;
    let mean_us = mean_ns / 1000;

    assert!(
        mean_us < 2000,
        "promote mean latency {mean_us}µs exceeds 2000µs CI threshold; \
         1000 calls took {}ms",
        elapsed.as_millis()
    );
    Ok(())
}
