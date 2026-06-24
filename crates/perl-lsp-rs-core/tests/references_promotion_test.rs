//! Integration tests for PIR-A guarded reference promotion (#2651 PR3b contract).
//!
//! Tests the corrected [`references_pir_promote`] entry point with the new
//! [`PromotionMode`] + sigil-identity contract against curated hand-verified
//! fixture sets (the correct, scope-aware answers — NOT legacy as ground truth,
//! since legacy is the scope-blind baseline being superseded).
//!
//! Drives the real pipeline: `Parser → lower_ast → extract_lexical_facts`.
//! `LexicalExtractorReceipt` is `#[non_exhaustive]` so receipts cannot be
//! hand-constructed; we always go through the real pipeline.
//!
//! The `Exact` branch is exercised via `PromotionMode::PromoteExact`. The
//! `Off`/`Shadow`/fallback branches are exercised directly.
//!
//! ## Fixture inventory
//!
//! | # | Name | What it asserts |
//! |---|------|-----------------|
//! | F1 | scope_exact_outer_x | PromoteExact returns exactly the 2 scope-correct ranges for outer `$x` |
//! | F2 | flag_off_fallback | Off → FeatureDisabled regardless of receipt |
//! | F3 | package_qualified_refused | `$Foo::x` → LegacyFallback(NotSameFileLexical) |
//! | F4 | single_scope_compiler_equals_legacy | simple same-scope: compiler returns exact 2 ranges |
//! | F5 | subroutine_references_unaffected | `find_references_single_file` still returns ≥2 refs for subs |
//! | F7 | utf16_astral_column_non_bmp | emoji (2 UTF-16 units) before `$v` on same line: column is UTF-16 code-unit count |
//! | F8 | utf16_bmp_multibyte_column | `é` (2 UTF-8 bytes, 1 UTF-16 unit) before `$v`: column counts 1 UTF-16 unit, not 2 bytes |
//! | F9 | crlf_line_endings | CRLF source: line/character correct; `\r` not miscounted as column |
//! | F10 | scope_shadow_exact_range_set | outer `$x` + inner `my $x`: find-refs on outer returns only outer ranges, as exact set |
//! | F11 | include_declaration_note | promotion returns all occurrences incl. declaration when opts.include_declaration=true |
//! | F12 | cross_file_fallback | package-qualified target → LegacyFallback, not Exact |
//!
//! Note: latency is tracked via benchmarks/receipts, not wall-clock unit tests.

use perl_lsp_rs_core::providers::navigation::references_pir_shadow::{
    PirShadowRefusalReason, PromotionMode, ReferenceOptions,
};
use perl_lsp_rs_core::providers::navigation::{
    DEFAULT_PROMOTION_MODE, ReferencesPirPromoteOutcome, find_references_single_file,
    references_pir_promote,
};
use perl_parser_core::{Parser, hir::lower_ast, pir::extract_lexical_facts};
use perl_position_tracking::PositionMapper;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Identity URI mapper: converts byte offsets to a trivial single-line `lsp_types::Range`.
///
/// Used by F1-F4 which test scope/promotion logic, not encoding correctness.
/// F7-F12 use a `PositionMapper`-backed closure for correct UTF-16 encoding.
fn byte_mapper(start: usize, end: usize) -> lsp_types::Range {
    lsp_types::Range {
        start: lsp_types::Position { line: 0, character: start as u32 },
        end: lsp_types::Position { line: 0, character: end as u32 },
    }
}

/// Build a `lsp_types::Range` from byte offsets using the production UTF-16 encoder.
///
/// The `PositionMapper` internally uses a rope and counts UTF-16 code units,
/// matching the LSP protocol requirement.
fn lsp_range_from_bytes(
    mapper: &PositionMapper,
    start_byte: usize,
    end_byte: usize,
) -> lsp_types::Range {
    let start = mapper.byte_to_lsp_pos(start_byte);
    let end = mapper.byte_to_lsp_pos(end_byte);
    lsp_types::Range {
        start: lsp_types::Position { line: start.line, character: start.character },
        end: lsp_types::Position { line: end.line, character: end.character },
    }
}

fn receipt_for(source: &str) -> perl_parser_core::pir::LexicalExtractorReceipt {
    let mut parser = Parser::new(source);
    let output = parser.parse_with_recovery();
    let hir = lower_ast(&output.ast);
    extract_lexical_facts(&hir)
}

/// Sort ranges by (line, character, end_line, end_character) for deterministic
/// comparison independent of the order the compiler emits them.
fn sorted_ranges(mut ranges: Vec<lsp_types::Range>) -> Vec<lsp_types::Range> {
    ranges.sort_by_key(|r| (r.start.line, r.start.character, r.end.line, r.end.character));
    ranges
}

fn opts_all() -> ReferenceOptions {
    ReferenceOptions { include_declaration: true }
}

// ─────────────────────────────────────────────────────────────────────────────
// Confirm the rollback anchor is off by default
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn default_promotion_mode_is_off() -> TestResult {
    assert_eq!(DEFAULT_PROMOTION_MODE, PromotionMode::Off);
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// F1: Scope-exact outer $x — the key correctness win
//
// Source (6 lines):
//   my $x = 1;        ← outer write (body0)
//   {
//       my $x = 2;    ← inner write in a block scope
//       print $x;     ← inner read
//   }
//   print $x;         ← outer read
//
// Curated expected for outer $x in body0: exactly 2 ranges (outer write + outer read).
// The scope-blind legacy arm returns 3 (all $x occurrences regardless of scope).
// The compiler returns only the 2 outer ones → correctness win.
//
// We use the identity byte_mapper here (testing scope-exact logic, not encoding).
// The expected ranges are derived from the byte positions we assert on.
// ─────────────────────────────────────────────────────────────────────────────

const F1_SOURCE: &str = "my $x = 1;\n{\n    my $x = 2;\n    print $x;\n}\nprint $x;\n";

/// Returns the exact lsp_types::Range values expected for the TWO outer-scope
/// $x occurrences in F1_SOURCE, using the identity byte_mapper.
///
/// Both outer $x occurrences must be present; the two inner ones must be absent.
/// Byte positions for $x occurrences (verified via F1_SOURCE.match_indices("$x")):
///   outer write: byte  3.. 5  → identity mapper: char  3.. 5 (line 0)
///   outer read:  byte 50..52  → identity mapper: char 50..52 (line 0)
///
/// (Inner occurrences: byte 20..22, byte 38..40 — MUST be excluded.)
fn f1_expected_outer_x_ranges() -> Vec<lsp_types::Range> {
    // Compute byte positions from the source to keep this in sync.
    let positions: Vec<usize> = F1_SOURCE.match_indices("$x").map(|(i, _)| i).collect();
    assert_eq!(positions.len(), 4, "F1_SOURCE must have exactly 4 $x occurrences");
    // Outer = first (byte 3) and last (byte 50).
    let outer_write_start = positions[0];
    let outer_read_start = positions[3];

    // With the identity byte_mapper, byte offset == character.
    vec![
        lsp_types::Range {
            start: lsp_types::Position { line: 0, character: outer_write_start as u32 },
            end: lsp_types::Position { line: 0, character: (outer_write_start + 2) as u32 },
        },
        lsp_types::Range {
            start: lsp_types::Position { line: 0, character: outer_read_start as u32 },
            end: lsp_types::Position { line: 0, character: (outer_read_start + 2) as u32 },
        },
    ]
}

#[test]
fn f1_scope_exact_outer_x_returns_exact_range_set() -> TestResult {
    let expected = sorted_ranges(f1_expected_outer_x_ranges());
    let receipt = receipt_for(F1_SOURCE);

    let outcome = references_pir_promote(
        PromotionMode::PromoteExact,
        "$",
        "x",
        &receipt,
        &[],
        0,
        &byte_mapper,
        opts_all(),
    );

    match outcome {
        ReferencesPirPromoteOutcome::Exact(ranges) => {
            let got = sorted_ranges(ranges);
            assert_eq!(
                got, expected,
                "scope-exact compiler must return exactly the 2 outer $x ranges;\
                 \nexpected: {expected:?}\ngot:      {got:?}"
            );
        }
        other => return Err(format!("expected Exact, got {other:?}").into()),
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// F2: Flag-off fallback — the merge-safe default
//
// With DEFAULT_PROMOTION_MODE = Off, references_pir_promote must return
// LegacyFallback(FeatureDisabled) regardless of receipt content or target name.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn f2_flag_off_always_returns_legacy_fallback() -> TestResult {
    assert_eq!(DEFAULT_PROMOTION_MODE, PromotionMode::Off, "flag must be off at merge time");

    let receipt = receipt_for(F1_SOURCE);
    // Curated legacy byte-offset pairs for outer $x (as the legacy arm would return).
    let positions: Vec<usize> = F1_SOURCE.match_indices("$x").map(|(i, _)| i).collect();
    let legacy: Vec<(usize, usize)> =
        vec![(positions[0], positions[0] + 2), (positions[3], positions[3] + 2)];
    let outcome = references_pir_promote(
        DEFAULT_PROMOTION_MODE,
        "$",
        "x",
        &receipt,
        &legacy,
        0,
        &byte_mapper,
        opts_all(),
    );

    match outcome {
        ReferencesPirPromoteOutcome::LegacyFallback { result, reason } => {
            assert_eq!(result, legacy, "legacy result must be returned unmodified");
            assert_eq!(
                reason,
                PirShadowRefusalReason::FeatureDisabled,
                "Off mode must set FeatureDisabled reason"
            );
        }
        other => return Err(format!("expected LegacyFallback on Off mode, got {other:?}").into()),
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// F3: Package-qualified name refused (PromoteExact path)
//
// A `::`-qualified name must return LegacyFallback(NotSameFileLexical) because
// package variables are not same-file lexical bindings.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn f3_package_qualified_name_is_refused() -> TestResult {
    let receipt = receipt_for("my $x = 1;\n");
    let legacy = vec![(3usize, 5usize)];
    let outcome = references_pir_promote(
        PromotionMode::PromoteExact,
        "$",
        "Foo::x",
        &receipt,
        &legacy,
        0,
        &byte_mapper,
        opts_all(),
    );

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
// F4: Single-scope variable — compiler returns exact 2-range set
//
// Source: `my $a = 1;\nprint $a;\n`
// Curated expected: exactly 2 ranges ($a write + $a read). No scope ambiguity.
// ─────────────────────────────────────────────────────────────────────────────

const F4_SOURCE: &str = "my $a = 1;\nprint $a;\n";

#[test]
fn f4_single_scope_exact_range_set() -> TestResult {
    // $a byte positions in F4_SOURCE:
    //   "my $a = 1;\n" → $a at byte 3..5 (line 0 with identity mapper)
    //   "print $a;\n"  → $a at byte 17..19 (line 0 with identity mapper)
    let positions: Vec<usize> = F4_SOURCE.match_indices("$a").map(|(i, _)| i).collect();
    assert_eq!(positions.len(), 2, "F4_SOURCE must have exactly 2 $a occurrences");

    let expected = sorted_ranges(vec![
        lsp_types::Range {
            start: lsp_types::Position { line: 0, character: positions[0] as u32 },
            end: lsp_types::Position { line: 0, character: (positions[0] + 2) as u32 },
        },
        lsp_types::Range {
            start: lsp_types::Position { line: 0, character: positions[1] as u32 },
            end: lsp_types::Position { line: 0, character: (positions[1] + 2) as u32 },
        },
    ]);

    let receipt = receipt_for(F4_SOURCE);
    let outcome = references_pir_promote(
        PromotionMode::PromoteExact,
        "$",
        "a",
        &receipt,
        &[],
        0,
        &byte_mapper,
        opts_all(),
    );

    match outcome {
        ReferencesPirPromoteOutcome::Exact(ranges) => {
            let got = sorted_ranges(ranges);
            assert_eq!(
                got, expected,
                "single-scope $a must yield exactly the 2 ranges (write + read);\
                 \nexpected: {expected:?}\ngot:      {got:?}"
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
// correctly — the sub arms in references.rs are not touched by this PR.
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

// Note: latency is tracked via benchmarks and latency receipts, not wall-clock
// unit tests (which are flaky on variable CI hardware for small fixtures).

// ─────────────────────────────────────────────────────────────────────────────
// F7: Astral / non-BMP (UTF-16 surrogate pair) before target variable
//
// Source: `my $v = "😀"; my $w = $v;\nprint $v;\n`
//
// The emoji 😀 (U+1F600) occupies 4 UTF-8 bytes but 2 UTF-16 code units (a
// surrogate pair). It is on the SAME line as $v to make the encoding test
// non-trivial.
//
// The key assertion: $v at byte 24 on line 0 must have character=22 (UTF-16),
// NOT character=24 (byte offset) or character=23 (codepoint offset).
// ─────────────────────────────────────────────────────────────────────────────

const F7_SOURCE: &str = "my $v = \"\u{1F600}\"; my $w = $v;\nprint $v;\n";

#[test]
fn f7_utf16_astral_column_non_bmp() -> TestResult {
    // 😀 = U+1F600, 4 bytes UTF-8, 2 UTF-16 code units.
    let emoji_byte = F7_SOURCE.find('\u{1F600}').ok_or("emoji not found in F7_SOURCE")?;
    assert_eq!(emoji_byte, 9, "emoji must start at byte 9 in F7_SOURCE");

    let mapper = PositionMapper::new(F7_SOURCE);
    let uri_mapper = |start: usize, end: usize| lsp_range_from_bytes(&mapper, start, end);

    let receipt = receipt_for(F7_SOURCE);
    let outcome = references_pir_promote(
        PromotionMode::PromoteExact,
        "$",
        "v",
        &receipt,
        &[],
        0,
        &uri_mapper,
        opts_all(),
    );

    let ranges = match outcome {
        ReferencesPirPromoteOutcome::Exact(r) => r,
        other => return Err(format!("expected Exact, got {other:?}").into()),
    };

    let expected_decl = lsp_range_from_bytes(&mapper, 3, 5);
    let expected_after_emoji = lsp_range_from_bytes(&mapper, 24, 26);
    let expected_line1 = lsp_range_from_bytes(&mapper, 34, 36);

    // Assert the UTF-16 column for the post-emoji occurrence: must be 22, not 24.
    assert_eq!(
        expected_after_emoji.start.character, 22,
        "UTF-16 character for $v after 😀 must be 22 (emoji = 4 UTF-8 bytes = 2 UTF-16 units)"
    );
    assert_eq!(expected_after_emoji.start.line, 0, "$v after emoji must be on line 0");

    assert_eq!(
        expected_line1.start.character, 6,
        "$v in 'print $v' on line 1 must be at UTF-16 col 6 (pure ASCII)"
    );

    let got = sorted_ranges(ranges);
    let expected = sorted_ranges(vec![expected_decl, expected_after_emoji, expected_line1]);

    assert_eq!(
        got, expected,
        "F7: exact UTF-16 range set mismatch;\nexpected: {expected:?}\ngot:      {got:?}"
    );

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// F8: Multi-byte BMP — `é` (U+00E9) before target variable on same line
//
// é = U+00E9: 2 UTF-8 bytes, 1 UTF-16 code unit (BMP, not a surrogate pair).
// Key: col = 21 (not 22 = byte offset).
// ─────────────────────────────────────────────────────────────────────────────

const F8_SOURCE: &str = "my $v = \"\u{00E9}\"; my $w = $v;\nprint $v;\n";

#[test]
fn f8_utf16_bmp_multibyte_column() -> TestResult {
    let e_acute_byte = F8_SOURCE.find('\u{00E9}').ok_or("é not found in F8_SOURCE")?;
    assert_eq!(e_acute_byte, 9, "é must start at byte 9 in F8_SOURCE");

    let mapper = PositionMapper::new(F8_SOURCE);
    let uri_mapper = |start: usize, end: usize| lsp_range_from_bytes(&mapper, start, end);

    let pos_after_e = mapper.byte_to_lsp_pos(22);
    assert_eq!(pos_after_e.line, 0, "$v use must be on line 0");
    assert_eq!(
        pos_after_e.character, 21,
        "UTF-16 col for $v after é must be 21: é occupies 2 UTF-8 bytes but 1 UTF-16 unit"
    );

    let receipt = receipt_for(F8_SOURCE);
    let outcome = references_pir_promote(
        PromotionMode::PromoteExact,
        "$",
        "v",
        &receipt,
        &[],
        0,
        &uri_mapper,
        opts_all(),
    );

    let ranges = match outcome {
        ReferencesPirPromoteOutcome::Exact(r) => r,
        other => return Err(format!("expected Exact, got {other:?}").into()),
    };

    // Line 0 ends at byte 26 (25 content bytes + \n).
    let line1_v_byte =
        F8_SOURCE[26..].find("$v").map(|i| i + 26).ok_or("$v not found on line 1 of F8_SOURCE")?;

    let expected_decl = lsp_range_from_bytes(&mapper, 3, 5);
    let expected_use_line0 = lsp_range_from_bytes(&mapper, 22, 24);
    let expected_use_line1 = lsp_range_from_bytes(&mapper, line1_v_byte, line1_v_byte + 2);

    let got = sorted_ranges(ranges);
    let expected = sorted_ranges(vec![expected_decl, expected_use_line0, expected_use_line1]);

    assert_eq!(
        got, expected,
        "F8: BMP multibyte UTF-16 range mismatch;\nexpected: {expected:?}\ngot:      {got:?}"
    );

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// F9: CRLF line endings
//
// Source (CRLF): `my $v = 1;\r\nprint $v;\r\n`
// The `\r` must NOT be counted as a column on the following line.
// ─────────────────────────────────────────────────────────────────────────────

const F9_SOURCE_CRLF: &str = "my $v = 1;\r\nprint $v;\r\n";

#[test]
fn f9_crlf_line_endings_correct_columns() -> TestResult {
    assert!(F9_SOURCE_CRLF.contains("\r\n"), "F9_SOURCE_CRLF must use CRLF line endings");
    assert_eq!(F9_SOURCE_CRLF.as_bytes()[10], b'\r', "byte 10 must be CR");
    assert_eq!(F9_SOURCE_CRLF.as_bytes()[11], b'\n', "byte 11 must be LF");

    let mapper = PositionMapper::new(F9_SOURCE_CRLF);
    let uri_mapper = |start: usize, end: usize| lsp_range_from_bytes(&mapper, start, end);

    let pos_decl_start = mapper.byte_to_lsp_pos(3);
    assert_eq!(pos_decl_start.line, 0, "$v decl must be on line 0");
    assert_eq!(pos_decl_start.character, 3, "$v decl col must be 3");

    let line1_start = 12usize;
    let v_use_byte = line1_start + 6;

    let pos_use_start = mapper.byte_to_lsp_pos(v_use_byte);
    assert_eq!(pos_use_start.line, 1, "$v use must be on line 1");
    assert_eq!(pos_use_start.character, 6, "$v use col must be 6 (\\r must not be miscounted)");

    let receipt = receipt_for(F9_SOURCE_CRLF);
    let outcome = references_pir_promote(
        PromotionMode::PromoteExact,
        "$",
        "v",
        &receipt,
        &[],
        0,
        &uri_mapper,
        opts_all(),
    );

    let ranges = match outcome {
        ReferencesPirPromoteOutcome::Exact(r) => r,
        other => return Err(format!("expected Exact for CRLF source, got {other:?}").into()),
    };

    let expected = sorted_ranges(vec![
        lsp_range_from_bytes(&mapper, 3, 5),
        lsp_range_from_bytes(&mapper, v_use_byte, v_use_byte + 2),
    ]);
    let got = sorted_ranges(ranges);

    assert_eq!(
        got, expected,
        "F9: CRLF range mismatch;\nexpected: {expected:?}\ngot:      {got:?}"
    );

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// F10: Scope-exact shadowing — exact range set with UTF-16 mapper
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn f10_scope_shadow_exact_range_set_with_utf16_mapper() -> TestResult {
    let mapper = PositionMapper::new(F1_SOURCE);
    let uri_mapper = |start: usize, end: usize| lsp_range_from_bytes(&mapper, start, end);

    let receipt = receipt_for(F1_SOURCE);
    let outcome = references_pir_promote(
        PromotionMode::PromoteExact,
        "$",
        "x",
        &receipt,
        &[],
        0,
        &uri_mapper,
        opts_all(),
    );

    let ranges = match outcome {
        ReferencesPirPromoteOutcome::Exact(r) => r,
        other => return Err(format!("expected Exact, got {other:?}").into()),
    };

    let outer_write = lsp_range_from_bytes(&mapper, 3, 5);
    let outer_read = {
        let positions: Vec<usize> = F1_SOURCE.match_indices("$x").map(|(i, _)| i).collect();
        lsp_range_from_bytes(&mapper, positions[3], positions[3] + 2)
    };

    assert_eq!(outer_write.start.line, 0);
    assert_eq!(outer_write.start.character, 3);
    assert_eq!(outer_write.end.character, 5);
    assert_eq!(outer_read.start.line, 5, "final $x must be on line 5 (0-indexed)");
    assert_eq!(outer_read.start.character, 6, "final $x must start at col 6 on line 5");
    assert_eq!(outer_read.end.character, 8);

    let expected = sorted_ranges(vec![outer_write, outer_read]);
    let got = sorted_ranges(ranges);

    let inner_decl = lsp_range_from_bytes(&mapper, 20, 22);
    let inner_read = lsp_range_from_bytes(&mapper, 38, 40);

    assert_eq!(
        got, expected,
        "F10: scope-shadow exact range set mismatch (inner ranges must be excluded);\
         \nexpected: {expected:?}\ngot:      {got:?}\
         \ninner_decl (must be absent): {inner_decl:?}\
         \ninner_read (must be absent): {inner_read:?}"
    );

    assert!(
        !got.contains(&inner_decl),
        "inner-scope $x declaration (line 2) must NOT appear in outer-scope result"
    );
    assert!(
        !got.contains(&inner_read),
        "inner-scope $x read (line 3) must NOT appear in outer-scope result"
    );

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// F11: includeDeclaration=true includes both declaration and use
//
// With `include_declaration: true`, the promote path returns ALL anchored
// occurrences for the target name. Callers that want to suppress the declaration
// must pass `include_declaration: false`.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn f11_include_declaration_filtering_is_above_promote_layer() -> TestResult {
    let source = "my $a = 1;\nprint $a;\n";
    let mapper = PositionMapper::new(source);
    let uri_mapper = |start: usize, end: usize| lsp_range_from_bytes(&mapper, start, end);

    let receipt = receipt_for(source);
    let outcome = references_pir_promote(
        PromotionMode::PromoteExact,
        "$",
        "a",
        &receipt,
        &[],
        0,
        &uri_mapper,
        ReferenceOptions { include_declaration: true },
    );

    let ranges = match outcome {
        ReferencesPirPromoteOutcome::Exact(r) => r,
        other => return Err(format!("expected Exact, got {other:?}").into()),
    };

    let decl_range = lsp_range_from_bytes(&mapper, 3, 5);
    let read_range = lsp_range_from_bytes(&mapper, 17, 19);

    assert_eq!(decl_range.start.line, 0);
    assert_eq!(decl_range.start.character, 3);
    assert_eq!(read_range.start.line, 1);
    assert_eq!(read_range.start.character, 6, "$a in 'print $a' on line 1 must be at col 6");

    let expected = sorted_ranges(vec![decl_range, read_range]);
    let got = sorted_ranges(ranges);

    assert_eq!(
        got, expected,
        "F11: promote layer with include_declaration=true must return BOTH declaration and use site;\
         \nNote: callers can pass include_declaration=false to suppress the declaration;\
         \nexpected: {expected:?}\ngot:      {got:?}"
    );

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// F12: Cross-file / package-qualified target → LegacyFallback (not Exact)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn f12_cross_file_package_qualified_returns_legacy_fallback() -> TestResult {
    let receipt = receipt_for("my $x = 1;\nprint $x;\n");
    let legacy = vec![(3usize, 5usize), (12usize, 14usize)];

    let outcome = references_pir_promote(
        PromotionMode::PromoteExact,
        "$",
        "Foo::bar",
        &receipt,
        &legacy,
        0,
        &byte_mapper,
        opts_all(),
    );

    match outcome {
        ReferencesPirPromoteOutcome::LegacyFallback { result, reason } => {
            assert_eq!(
                reason,
                PirShadowRefusalReason::NotSameFileLexical,
                "package-qualified name must refuse with NotSameFileLexical, not {reason:?}"
            );
            assert_eq!(result, legacy, "legacy result must be returned unmodified on refusal");
        }
        ReferencesPirPromoteOutcome::Exact(r) => {
            return Err(format!(
                "F12: package-qualified name must NOT produce Exact result;\
                 \ngot Exact({r:?}) — the guard failed to refuse"
            )
            .into());
        }
    }

    Ok(())
}
