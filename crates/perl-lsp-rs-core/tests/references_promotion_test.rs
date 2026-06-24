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
//! | F1 | scope_exact_outer_x | compiler returns exactly the 2 scope-correct ranges for outer `$x` |
//! | F2 | flag_off_fallback | flag=false → always LegacyFallback regardless of receipt |
//! | F3 | package_qualified_refused | `Foo::x` → LegacyFallback(NotSameFileLexical) via unguarded path |
//! | F4 | single_scope_compiler_equals_legacy | simple same-scope: compiler returns exact 2 ranges |
//! | F5 | subroutine_references_unaffected | `find_references_single_file` still returns ≥2 refs for subs |
//! | F7 | utf16_astral_column_non_bmp | emoji (2 UTF-16 units) before `$v` on same line: column is UTF-16 code-unit count |
//! | F8 | utf16_bmp_multibyte_column | `é` (2 UTF-8 bytes, 1 UTF-16 unit) before `$v`: column counts 1 UTF-16 unit, not 2 bytes |
//! | F9 | crlf_line_endings | CRLF source: line/character correct; `\r` not miscounted as column |
//! | F10 | scope_shadow_exact_range_set | outer `$x` + inner `my $x`: find-refs on outer returns only outer ranges, as exact set |
//! | F11 | include_declaration_note | promotion returns all occurrences incl. declaration; filtering is above promote layer |
//! | F12 | cross_file_fallback | package-qualified target → LegacyFallback, not Exact |
//!
//! Note: latency is tracked via benchmarks/receipts, not wall-clock unit tests.

use perl_lsp_rs_core::providers::navigation::references_pir_shadow::PirShadowRefusalReason;
use perl_lsp_rs_core::providers::navigation::{
    ENABLE_PIR_LEXICAL_REFERENCES, ReferencesPirPromoteOutcome, find_references_single_file,
    references_pir_promote, references_pir_promote_unguarded,
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

    let outcome = references_pir_promote_unguarded(&receipt, &[], "x", 0, &byte_mapper);

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
// With ENABLE_PIR_LEXICAL_REFERENCES = false, references_pir_promote must
// always return LegacyFallback regardless of receipt content or target name.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn f2_flag_off_always_returns_legacy_fallback() -> TestResult {
    const { assert!(!ENABLE_PIR_LEXICAL_REFERENCES, "flag must be off at merge time") };

    let receipt = receipt_for(F1_SOURCE);
    // Curated legacy byte-offset pairs for outer $x (as the legacy arm would return).
    let positions: Vec<usize> = F1_SOURCE.match_indices("$x").map(|(i, _)| i).collect();
    let legacy: Vec<(usize, usize)> =
        vec![(positions[0], positions[0] + 2), (positions[3], positions[3] + 2)];
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
    let outcome = references_pir_promote_unguarded(&receipt, &[], "a", 0, &byte_mapper);

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
// Source: `# 😀 hello\nmy $v = 1;\nprint $v;\n`
//
// The emoji 😀 (U+1F600) is in the comment on line 0 — it occupies 4 UTF-8
// bytes but 2 UTF-16 code units (a surrogate pair). Lines 1 and 2 are pure
// ASCII, so the character column for $v is trivial there.
//
// To make the encoding test non-trivial, we place 😀 on the SAME line as $v:
//
//   Source: `my $v = "😀"; my $w = $v;\nprint $v;\n`
//
// $v positions (curated):
//   Line 0: `my $v = "😀"; my $w = $v;\n`
//     - bytes: m(0)y(1) (2)$(3)v(4) (5)=(6) (7)"(8)😀(9..12)"(13);(14) (15)m(16)y(17) (18)$(19)w(20) (21)=(22) (23)$(24)v(25);(26)\n(27)
//     - `$v` at byte 3..5 → UTF-16 col: 3 (all ASCII before it)
//     - `$v` at byte 24..26 → UTF-16 col: bytes 0..23 decoded:
//         `my $v = "` = 9 bytes = 9 UTF-16 units
//         `😀` = 4 bytes = 2 UTF-16 units
//         `"; my $w = ` = 10 bytes = 10 UTF-16 units
//         total = 21 UTF-16 units → col 21..23
//   Line 1: `print $v;\n`
//     - `$v` at byte (28+6)=34..36 → UTF-16 col 6..8 (pure ASCII line)
//
// The key assertion: $v at byte 24 on line 0 must have character=21 (UTF-16),
// NOT character=24 (byte offset) or character=22 (codepoint offset).
// ─────────────────────────────────────────────────────────────────────────────

const F7_SOURCE: &str = "my $v = \"\u{1F600}\"; my $w = $v;\nprint $v;\n";

#[test]
fn f7_utf16_astral_column_non_bmp() -> TestResult {
    // 😀 = U+1F600, 4 bytes UTF-8, 2 UTF-16 code units.
    // Verify our source string has the emoji in the expected position.
    let emoji_byte = F7_SOURCE.find('\u{1F600}').ok_or("emoji not found in F7_SOURCE")?;
    assert_eq!(emoji_byte, 9, "emoji must start at byte 9 in F7_SOURCE");

    let mapper = PositionMapper::new(F7_SOURCE);

    // Closure using the production UTF-16 mapper — this is what the real
    // provider would use. The promote path passes byte offsets to it; we
    // verify it produces correct UTF-16 columns.
    let uri_mapper = |start: usize, end: usize| lsp_range_from_bytes(&mapper, start, end);

    let receipt = receipt_for(F7_SOURCE);
    let outcome = references_pir_promote_unguarded(&receipt, &[], "v", 0, &uri_mapper);

    let ranges = match outcome {
        ReferencesPirPromoteOutcome::Exact(r) => r,
        other => return Err(format!("expected Exact, got {other:?}").into()),
    };

    // The compiler must find occurrences of $v (not $w).
    // We expect 3: the declaration ($v decl), the $v in the rhs, and the print.
    // The key property: find the occurrence on line 0 that comes AFTER the emoji.
    // Its UTF-16 character column must be 22 (not 24=byte offset, not 23=codepoint offset).
    //
    // Expected ranges (sorted):
    //   $v decl  → byte 3..5  → line 0, char 3..5    (before emoji, trivial)
    //   $v use1  → byte 24..26 → line 0, char 22..24  (AFTER emoji: 9 ASCII + 2 UTF-16 + 11 ASCII = 22)
    //   $v use2  → byte 34..36 → line 1, char 6..8    (pure ASCII line 1)
    //
    // Line 0 byte count: "my $v = \"😀\"; my $w = $v;\n"
    //   9 ASCII + 4 bytes(😀) + 15 ASCII = 28 bytes total.
    //   Line 1 = "print $v;\n" starts at byte 28. $v at byte 28+6=34.

    let expected_decl = lsp_range_from_bytes(&mapper, 3, 5);
    let expected_after_emoji = lsp_range_from_bytes(&mapper, 24, 26);
    let expected_line1 = lsp_range_from_bytes(&mapper, 34, 36);

    // Assert the UTF-16 column for the post-emoji occurrence: must be 22, not 24.
    // Breakdown: 9 (before quote+emoji) + 2 (emoji as 2 UTF-16 units) + 11 ("; my $w = ") = 22.
    assert_eq!(
        expected_after_emoji.start.character, 22,
        "UTF-16 character for $v after 😀 must be 22 (emoji = 4 UTF-8 bytes = 2 UTF-16 units)"
    );
    assert_eq!(expected_after_emoji.start.line, 0, "$v after emoji must be on line 0");

    // Assert the line-1 occurrence column is pure-ASCII (byte == UTF-16 unit).
    assert_eq!(
        expected_line1.start.character, 6,
        "$v in 'print $v' on line 1 must be at UTF-16 col 6 (pure ASCII)"
    );

    // The produce path must include the expected ranges in its Exact result.
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
//
// Source: `my $v = "é"; my $w = $v;\nprint $v;\n`
//
// The key assertion: $v (use) at byte 22 on line 0 must have character=21.
// The é replaces 1 ASCII byte with 2 UTF-8 bytes but still counts as 1 UTF-16 unit,
// so the column shifts by +1 byte but NOT by +1 UTF-16 unit vs a pure-ASCII string.
//
// Byte layout of line 0: `my $v = "é"; my $w = $v;\n`
//   m(0)y(1) (2)$(3)v(4) (5)=(6) (7)"(8)é(9..10)"(11);(12) (13)m(14)y(15) (16)$(17)w(18)
//    (19)=(20) (21)$(22)v(23);(24)\n(25)
//   `$v` (decl) at byte 3..5  → UTF-16 col 3..5   (before é, trivial)
//   `$v` (use)  at byte 22..24 → UTF-16 col:
//       `my $v = "` = 9 bytes = 9 UTF-16 units
//       `é`         = 2 bytes = 1 UTF-16 unit    ← saves 1 byte vs 2-unit surrogate
//       `"; my $w = ` = 11 bytes = 11 UTF-16 units
//       total = 21 UTF-16 units → col 21..23
//
// Key: col = 21 (not 22 = byte offset of $v in F8, not 20 = miscount).
// Contrast with F7 (emoji 😀): 9 + 2 (surrogate) + 11 = 22, two UTF-16 units for the emoji.
// ─────────────────────────────────────────────────────────────────────────────

const F8_SOURCE: &str = "my $v = \"\u{00E9}\"; my $w = $v;\nprint $v;\n";

#[test]
fn f8_utf16_bmp_multibyte_column() -> TestResult {
    // é = U+00E9, 2 UTF-8 bytes, 1 UTF-16 code unit.
    let e_acute_byte = F8_SOURCE.find('\u{00E9}').ok_or("é not found in F8_SOURCE")?;
    assert_eq!(e_acute_byte, 9, "é must start at byte 9 in F8_SOURCE");

    let mapper = PositionMapper::new(F8_SOURCE);
    let uri_mapper = |start: usize, end: usize| lsp_range_from_bytes(&mapper, start, end);

    // Byte layout of line 0: `my $v = "é"; my $w = $v;\n`
    //   m(0)y(1) (2)$(3)v(4) (5)=(6) (7)"(8) é(9-10) "(11);(12) (13)m(14)y(15)
    //    (16)$(17)w(18) (19)=(20) (21)$(22)v(23);(24)\n(25)
    //   $v decl at byte 3..5  (before é, trivial)
    //   $v use  at byte 22..24
    //   UTF-16 col for byte 22: 9 (ASCII before é) + 1 (é = 1 UTF-16 unit) + 11 (`"; my $w = `) = 21

    // Verify using PositionMapper (production encoder):
    let pos_after_e = mapper.byte_to_lsp_pos(22);
    assert_eq!(pos_after_e.line, 0, "$v use must be on line 0");
    // UTF-16 col: 9 (before é) + 1 (é as 1 UTF-16 unit) + 11 (after é, before $v) = 21
    // NOT 22 (raw byte offset), NOT 20 (miscounting é as 0 units).
    assert_eq!(
        pos_after_e.character, 21,
        "UTF-16 col for $v after é must be 21: é occupies 2 UTF-8 bytes but 1 UTF-16 unit"
    );

    let receipt = receipt_for(F8_SOURCE);
    let outcome = references_pir_promote_unguarded(&receipt, &[], "v", 0, &uri_mapper);

    let ranges = match outcome {
        ReferencesPirPromoteOutcome::Exact(r) => r,
        other => return Err(format!("expected Exact, got {other:?}").into()),
    };

    // Line 1: "print $v;\n" — $v at line 1 col 6 (pure ASCII).
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
// The same logical Perl source with `\r\n` line endings — positions must be
// identical in line/character to the `\n` version: the `\r` is part of the
// line terminator and must NOT be counted as a column on the FOLLOWING line.
//
// Source (CRLF): `my $v = 1;\r\nprint $v;\r\n`
//
// Expected:
//   $v decl → byte 3..5  → line 0, character 3..5
//   $v use  → byte 18..20 → line 1, character 6..8
//             (line 1 starts at byte 12, after \r\n; "print " = 6 bytes → $v at 12+6=18)
// ─────────────────────────────────────────────────────────────────────────────

const F9_SOURCE_CRLF: &str = "my $v = 1;\r\nprint $v;\r\n";

#[test]
fn f9_crlf_line_endings_correct_columns() -> TestResult {
    // Verify the source has CRLF.
    assert!(F9_SOURCE_CRLF.contains("\r\n"), "F9_SOURCE_CRLF must use CRLF line endings");
    assert_eq!(F9_SOURCE_CRLF.as_bytes()[10], b'\r', "byte 10 must be CR");
    assert_eq!(F9_SOURCE_CRLF.as_bytes()[11], b'\n', "byte 11 must be LF");

    let mapper = PositionMapper::new(F9_SOURCE_CRLF);
    let uri_mapper = |start: usize, end: usize| lsp_range_from_bytes(&mapper, start, end);

    // $v decl: byte 3..5 → line 0, col 3..5
    let pos_decl_start = mapper.byte_to_lsp_pos(3);
    assert_eq!(pos_decl_start.line, 0, "$v decl must be on line 0");
    assert_eq!(pos_decl_start.character, 3, "$v decl col must be 3");

    // Line 1 starts at byte 12 (after \r\n).
    // "print $v;\r\n" → "print " = 6 bytes, so $v at byte 12+6=18.
    let line1_start = 12usize; // after \r\n
    let v_use_byte = line1_start + 6; // "print " = 6 bytes

    let pos_use_start = mapper.byte_to_lsp_pos(v_use_byte);
    assert_eq!(pos_use_start.line, 1, "$v use must be on line 1");
    assert_eq!(pos_use_start.character, 6, "$v use col must be 6 (\\r must not be miscounted)");

    let receipt = receipt_for(F9_SOURCE_CRLF);
    let outcome = references_pir_promote_unguarded(&receipt, &[], "v", 0, &uri_mapper);

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
// F10: Scope-exact shadowing — exact range set
//
// This is the core correctness win: outer `my $x` and inner-block `my $x`.
// Find-refs on the OUTER `$x` must return Exact with Ranges ONLY at the
// outer-scope occurrences. The inner ones are excluded.
//
// We reuse F1_SOURCE (same structure) but use the production PositionMapper
// to assert the actual LSP Ranges (line+character) rather than identity offsets.
//
// Outer $x occurrences in F1_SOURCE with PositionMapper (LF line endings):
//   byte 3..5   → line 0, character 3..5  (declaration: `my $x = 1;`)
//   byte 50..52 → line 5, character 6..8  (final read: `print $x;`)
//
// Wait: let me compute line 5. F1_SOURCE:
//   line 0: `my $x = 1;\n`   (11 bytes, starts at 0)
//   line 1: `{\n`            ( 2 bytes, starts at 11)
//   line 2: `    my $x = 2;\n` (15 bytes, starts at 13)
//   line 3: `    print $x;\n` (14 bytes, starts at 28)
//   line 4: `}\n`            ( 2 bytes, starts at 42)
//   line 5: `print $x;\n`   (10 bytes, starts at 44)
//
// Byte 50 = 44 + 6, on line 5 (starts at byte 44), character 6.
// ($x = bytes 50..52, so character 6..8.)
//
// Inner $x occurrences (MUST be absent):
//   byte 20..22 → line 2, character 7..9  (`my $x = 2;` — `$x` after `    my `)
//   byte 38..40 → line 3, character 10..12 (`    print $x;`)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn f10_scope_shadow_exact_range_set_with_utf16_mapper() -> TestResult {
    let mapper = PositionMapper::new(F1_SOURCE);
    let uri_mapper = |start: usize, end: usize| lsp_range_from_bytes(&mapper, start, end);

    let receipt = receipt_for(F1_SOURCE);
    let outcome = references_pir_promote_unguarded(&receipt, &[], "x", 0, &uri_mapper);

    let ranges = match outcome {
        ReferencesPirPromoteOutcome::Exact(r) => r,
        other => return Err(format!("expected Exact, got {other:?}").into()),
    };

    // Verify line numbers of expected positions using the production mapper.
    let outer_write = lsp_range_from_bytes(&mapper, 3, 5);
    let outer_read = {
        // Find the last $x in F1_SOURCE.
        let positions: Vec<usize> = F1_SOURCE.match_indices("$x").map(|(i, _)| i).collect();
        lsp_range_from_bytes(&mapper, positions[3], positions[3] + 2)
    };

    // Assert the concrete LSP positions we expect:
    // outer_write: line 0, character 3..5
    assert_eq!(outer_write.start.line, 0);
    assert_eq!(outer_write.start.character, 3);
    assert_eq!(outer_write.end.character, 5);

    // outer_read: line 5, character 6..8
    assert_eq!(outer_read.start.line, 5, "final $x must be on line 5 (0-indexed)");
    assert_eq!(outer_read.start.character, 6, "final $x must start at col 6 on line 5");
    assert_eq!(outer_read.end.character, 8);

    let expected = sorted_ranges(vec![outer_write, outer_read]);
    let got = sorted_ranges(ranges);

    // Inner $x ranges that MUST NOT appear:
    let inner_decl = lsp_range_from_bytes(&mapper, 20, 22); // line 2, col 7
    let inner_read = lsp_range_from_bytes(&mapper, 38, 40); // line 3, col 10

    assert_eq!(
        got, expected,
        "F10: scope-shadow exact range set mismatch (inner ranges must be excluded);\
         \nexpected: {expected:?}\ngot:      {got:?}\
         \ninner_decl (must be absent): {inner_decl:?}\
         \ninner_read (must be absent): {inner_read:?}"
    );

    // Explicit exclusion check for inner ranges.
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
// F11: includeDeclaration filtering is above the promote layer
//
// The promote path (`references_pir_promote_unguarded`) returns ALL anchored
// occurrences for the target name — both the declaration site and use sites.
// The LSP includeDeclaration filter is applied by the provider layer (the
// textDocument/references handler), NOT inside the promote function.
//
// This test asserts that the promote layer returns BOTH the declaration and all
// reads for a simple two-occurrence source, confirming the contract: callers
// must filter the declaration if includeDeclaration=false.
//
// Source: `my $a = 1;\nprint $a;\n`
//   $a decl: byte 3..5 (the `my $a =` declaration)
//   $a use:  byte 17..19 (the `print $a` read)
//
// Both must appear in the Exact result — the promote layer makes no distinction.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn f11_include_declaration_filtering_is_above_promote_layer() -> TestResult {
    // Use the production PositionMapper to assert real LSP positions.
    let source = "my $a = 1;\nprint $a;\n";
    let mapper = PositionMapper::new(source);
    let uri_mapper = |start: usize, end: usize| lsp_range_from_bytes(&mapper, start, end);

    let receipt = receipt_for(source);
    let outcome = references_pir_promote_unguarded(&receipt, &[], "a", 0, &uri_mapper);

    let ranges = match outcome {
        ReferencesPirPromoteOutcome::Exact(r) => r,
        other => return Err(format!("expected Exact, got {other:?}").into()),
    };

    // Both declaration and read must be present.
    let decl_range = lsp_range_from_bytes(&mapper, 3, 5); // `my $a` → line 0, col 3..5
    let read_range = lsp_range_from_bytes(&mapper, 17, 19); // `print $a` → line 1, col 6..8

    // Assert concrete positions.
    assert_eq!(decl_range.start.line, 0);
    assert_eq!(decl_range.start.character, 3);
    assert_eq!(read_range.start.line, 1);
    assert_eq!(read_range.start.character, 6, "$a in 'print $a' on line 1 must be at col 6");

    let expected = sorted_ranges(vec![decl_range, read_range]);
    let got = sorted_ranges(ranges);

    assert_eq!(
        got, expected,
        "F11: promote layer must return BOTH declaration and use site;\
         \nNote: includeDeclaration filtering happens in the provider layer above;\
         \nexpected: {expected:?}\ngot:      {got:?}"
    );

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// F12: Cross-file / package-qualified target → LegacyFallback (not Exact)
//
// A package-qualified name (e.g. `Foo::bar`) is not a same-file lexical binding.
// The refusal guard must fire and return LegacyFallback(NotSameFileLexical),
// even through the unguarded path that bypasses the feature flag.
//
// This confirms the guard refuses rather than producing a bogus Exact result
// for targets the promote machinery doesn't handle.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn f12_cross_file_package_qualified_returns_legacy_fallback() -> TestResult {
    // Any source with a real lexical receipt will do; we just want a non-empty
    // receipt so the guard's refusal is clearly from the name check, not from
    // empty bodies.
    let receipt = receipt_for("my $x = 1;\nprint $x;\n");
    let legacy = vec![(3usize, 5usize), (12usize, 14usize)];

    let outcome = references_pir_promote_unguarded(&receipt, &legacy, "Foo::bar", 0, &byte_mapper);

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
        ReferencesPirPromoteOutcome::Stale { .. } => {
            return Err("F12: expected LegacyFallback, got Stale".into());
        }
    }

    Ok(())
}
