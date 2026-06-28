//! Integration tests for the PIR-A references shadow compare (#2577 PR2, #2634).
//!
//! `LexicalExtractorReceipt` and its types are `#[non_exhaustive]`, so an
//! external test crate cannot hand-construct a receipt. These fixtures drive the
//! real pipeline (`Parser` → `lower_ast` → `extract_lexical_facts`) to obtain a
//! genuine receipt, then feed synthetic legacy byte-offset slices to
//! `shadow_references_with_pir` and assert the comparison.
//!
//! The two refusal guards the real pipeline never reaches (empty bodies,
//! `provider_behavior_changed == true`) are covered by the module's inline unit
//! tests over the pure `evaluate_refusal` helper; here we cover the two
//! pipeline-reachable refusals (package-qualified name, out-of-range body) plus
//! the full comparison surface.

use perl_lsp_rs_core::providers::navigation::references_pir_shadow::{
    PirShadowRefusalReason, shadow_references_with_pir,
};
use perl_parser_core::pir::LexicalExtractorReceipt;
use perl_parser_core::{Parser, hir::lower_ast, pir::extract_lexical_facts};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Source with four distinct `$x` sites in the program-root body (body 0):
/// `my $x = 1` (write), `print $x` (read), `$x = 10` (write), `say $x` (read).
const SRC: &str = "my $x = 1;\nprint $x;\n$x = 10;\nsay $x;\n";

/// Drive the real pipeline to a genuine extractor receipt.
fn receipt_for(source: &str) -> LexicalExtractorReceipt {
    let mut parser = Parser::new(source);
    let output = parser.parse_with_recovery();
    let hir = lower_ast(&output.ast);
    extract_lexical_facts(&hir)
}

/// The compiler's anchored `(start, end)` byte ranges for `name` in `body_idx`,
/// deduplicated and sorted ascending — identical to the set the function builds
/// internally, so it is the ground truth for constructing legacy slices.
fn compiler_ranges(
    receipt: &LexicalExtractorReceipt,
    body_idx: usize,
    name: &str,
) -> Vec<(usize, usize)> {
    let Some(body) = receipt.bodies.get(body_idx) else {
        return Vec::new();
    };
    body.facts
        .iter()
        .filter(|f| f.name.name == name && f.source_anchor.is_anchored())
        .filter_map(|f| f.source_anchor.range.as_ref().map(|r| (r.start, r.end)))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// Fixture 1: identical compiler/legacy sets → no disagreement of any kind.
#[test]
fn fixture_1_perfect_agreement() -> TestResult {
    let receipt = receipt_for(SRC);
    let ranges = compiler_ranges(&receipt, 0, "x");
    assert!(ranges.len() >= 2, "sanity: expected multiple $x sites, got {ranges:?}");

    let cmp = shadow_references_with_pir(&receipt, &ranges, "x", 0);

    assert_eq!(cmp.refusal_reason, None, "valid request must not refuse");
    assert_eq!(cmp.compiler_candidate_count, ranges.len());
    assert_eq!(cmp.legacy_candidate_count, ranges.len());
    assert!(cmp.missing_from_compiler.is_empty(), "no missing on agreement");
    assert!(cmp.extra_in_compiler.is_empty(), "no extra on agreement");
    assert!(cmp.range_disagreements.is_empty(), "no disagreements on agreement");
    assert!(!cmp.provider_behavior_changed, "shadow never changes behavior");
    Ok(())
}

/// Fixture 2: a legacy-only site far from any compiler site → `missing_from_compiler`.
#[test]
fn fixture_2_missing_from_compiler() -> TestResult {
    let receipt = receipt_for(SRC);
    let ranges = compiler_ranges(&receipt, 0, "x");

    let mut legacy = ranges.clone();
    legacy.push((10_000, 10_002)); // far from any real site → not a near-match

    let cmp = shadow_references_with_pir(&receipt, &legacy, "x", 0);

    assert_eq!(cmp.refusal_reason, None);
    assert_eq!(cmp.missing_from_compiler, vec![(10_000, 10_002)]);
    assert!(cmp.extra_in_compiler.is_empty());
    assert!(cmp.range_disagreements.is_empty());
    assert_eq!(cmp.legacy_candidate_count, ranges.len() + 1);
    Ok(())
}

/// Fixture 3: dropping a compiler site from legacy → `extra_in_compiler`.
#[test]
fn fixture_3_extra_in_compiler() -> TestResult {
    let receipt = receipt_for(SRC);
    let ranges = compiler_ranges(&receipt, 0, "x");
    let dropped = *ranges.last().ok_or("expected at least one $x site")?;
    let legacy: Vec<(usize, usize)> = ranges.iter().copied().filter(|&r| r != dropped).collect();

    let cmp = shadow_references_with_pir(&receipt, &legacy, "x", 0);

    assert_eq!(cmp.refusal_reason, None);
    assert_eq!(cmp.extra_in_compiler, vec![dropped]);
    assert!(cmp.missing_from_compiler.is_empty());
    assert!(cmp.range_disagreements.is_empty());
    Ok(())
}

/// Fixture 4: a 1-byte-shifted legacy site → one `range_disagreement`, and the
/// paired sites are absent from `missing`/`extra` (the categories are disjoint).
#[test]
fn fixture_4_range_disagreement_is_disjoint() -> TestResult {
    let receipt = receipt_for(SRC);
    let ranges = compiler_ranges(&receipt, 0, "x");
    let &(s, e) = ranges.first().ok_or("expected at least one $x site")?;

    // Shift the smallest site's start by +1 byte in the legacy view.
    let mut legacy = ranges.clone();
    legacy[0] = (s + 1, e + 1);

    let cmp = shadow_references_with_pir(&receipt, &legacy, "x", 0);

    assert_eq!(cmp.refusal_reason, None);
    assert_eq!(cmp.range_disagreements.len(), 1, "exactly one near-match pair");
    let rd = &cmp.range_disagreements[0];
    assert_eq!(rd.variable, "x");
    assert_eq!(rd.compiler_range, (s, e));
    assert_eq!(rd.legacy_range, (s + 1, e + 1));

    // Disjointness: the paired ranges are not double-counted.
    assert!(cmp.missing_from_compiler.is_empty(), "near-match not in missing");
    assert!(cmp.extra_in_compiler.is_empty(), "near-match not in extra");
    assert!(!cmp.missing_from_compiler.contains(&(s + 1, e + 1)));
    assert!(!cmp.extra_in_compiler.contains(&(s, e)));
    Ok(())
}

/// Fixture 5: a `::`-qualified target refuses as not-same-file-lexical.
#[test]
fn fixture_5_refuse_package_qualified_name() -> TestResult {
    let receipt = receipt_for(SRC);
    let cmp = shadow_references_with_pir(&receipt, &[], "Foo::bar", 0);

    assert_eq!(cmp.refusal_reason, Some(PirShadowRefusalReason::NotSameFileLexical));
    assert_eq!(cmp.compiler_candidate_count, 0);
    assert_eq!(cmp.legacy_candidate_count, 0);
    assert!(cmp.missing_from_compiler.is_empty());
    assert!(cmp.extra_in_compiler.is_empty());
    assert!(cmp.range_disagreements.is_empty());
    assert!(!cmp.provider_behavior_changed);
    Ok(())
}

/// Fixture 6: an out-of-range body index refuses with zeroed counts even when a
/// legacy slice was supplied.
#[test]
fn fixture_6_refuse_body_index_out_of_range() -> TestResult {
    let receipt = receipt_for(SRC);
    let oob = receipt.bodies.len() + 5;
    let cmp = shadow_references_with_pir(&receipt, &[(0, 2)], "x", oob);

    assert_eq!(cmp.refusal_reason, Some(PirShadowRefusalReason::NoAnchoredFacts));
    assert_eq!(cmp.legacy_candidate_count, 0, "refusal zeroes counts");
    assert_eq!(cmp.compiler_candidate_count, 0);
    Ok(())
}

/// Fixture 7: an unknown name is NOT a refusal — the comparison runs and finds
/// zero compiler candidates, so the legacy site is reported as missing.
#[test]
fn fixture_7_unknown_name_runs_with_no_compiler_facts() -> TestResult {
    let receipt = receipt_for(SRC);
    let cmp = shadow_references_with_pir(&receipt, &[(0, 2)], "zzz", 0);

    assert_eq!(cmp.refusal_reason, None, "unknown name runs, not refuses");
    assert_eq!(cmp.compiler_candidate_count, 0);
    assert_eq!(cmp.legacy_candidate_count, 1);
    assert_eq!(cmp.missing_from_compiler, vec![(0, 2)]);
    assert!(cmp.extra_in_compiler.is_empty());
    Ok(())
}

/// Fixture 8: deterministic output, and the diff vectors are sorted ascending.
#[test]
fn fixture_8_deterministic_and_sorted() -> TestResult {
    let receipt = receipt_for(SRC);
    let ranges = compiler_ranges(&receipt, 0, "x");

    // Two far-apart legacy-only sites pushed out of order.
    let mut legacy = ranges.clone();
    legacy.push((20_000, 20_002));
    legacy.push((10_000, 10_002));

    let a = shadow_references_with_pir(&receipt, &legacy, "x", 0);
    let b = shadow_references_with_pir(&receipt, &legacy, "x", 0);
    assert_eq!(a, b, "same inputs must yield identical receipts");

    let mut sorted = a.missing_from_compiler.clone();
    sorted.sort_unstable();
    assert_eq!(a.missing_from_compiler, sorted, "missing must be sorted ascending");
    assert_eq!(a.missing_from_compiler, vec![(10_000, 10_002), (20_000, 20_002)]);
    Ok(())
}

/// Fixture 9: byte offsets, not UTF-16 code units. A 2-byte `é` sits between the
/// two `$x` sites; the compiler anchors must align to `str::match_indices` byte
/// offsets (which a UTF-16 count would not).
#[test]
fn fixture_9_non_ascii_byte_offsets() -> TestResult {
    const UNICODE_SRC: &str = "my $x = \"caf\u{e9}\";\nprint $x;\n";
    let receipt = receipt_for(UNICODE_SRC);
    let ranges = compiler_ranges(&receipt, 0, "x");
    assert!(ranges.len() >= 2, "expected decl + read $x sites, got {ranges:?}");

    // Byte offsets of each `$x` literal in the source (`match_indices` is byte-based).
    let occurrences: Vec<usize> = UNICODE_SRC.match_indices("$x").map(|(i, _)| i).collect();
    assert_eq!(occurrences.len(), 2, "two `$x` occurrences expected");

    // Every anchor is a valid BYTE slice of the source covering the `$x` token.
    // A UTF-16-based offset would, past the 2-byte é, land on a non-char boundary
    // (`get` → None) or slice the wrong bytes.
    for &(start, end) in &ranges {
        let slice = UNICODE_SRC.get(start..end);
        assert!(
            slice.is_some_and(|s| s.contains("$x")),
            "range ({start},{end}) must be a valid byte slice covering `$x`, got {slice:?}"
        );
    }

    // The post-é read site is anchored at its BYTE offset, which strictly exceeds
    // its char/UTF-16 offset — the decisive byte-not-UTF-16 proof.
    let read_byte = *occurrences.get(1).ok_or("expected a second `$x` occurrence")?;
    let read_char_offset = UNICODE_SRC.get(..read_byte).map_or(0, |s| s.chars().count());
    assert!(
        read_byte > read_char_offset,
        "sanity: byte offset {read_byte} must exceed char offset {read_char_offset} past é"
    );
    let covers_read = ranges.iter().any(|&(s, e)| s <= read_byte && read_byte < e);
    assert!(
        covers_read,
        "a compiler anchor must cover the post-é `$x` at byte {read_byte} \
         (not its UTF-16 offset {read_char_offset})"
    );
    Ok(())
}
