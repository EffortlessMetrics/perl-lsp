---
tags: [coverage-integrity, lcov, scanner-blindness, cfg-test, ci]
repos: [perl-lsp-swarm]
related: ["#1326", "#1327", "#1321", "#1282"]
portable: false
article_asset: true
search_terms: [cfg_test_line_numbers, strip_cfg_test_lines, LcovSummary, quality_baseline.rs, cfg(test), brace-scanner, LCOV-filter, DA:, line_hit, line_found, literal-blind]
---

# LCOV brace scanner was blind to string/char/comment literals

**Date**: 2026-06
**Hazard class**: scanner-blindness + coverage-integrity
**Portable lesson**: [docs/concepts/hazard-class-invariants.md](../concepts/hazard-class-invariants.md) (Class 4 + Class 6)

## What happened

The patch-coverage LCOV post-processor in  used
 to find -gated scopes by counting brace depth.
The scanner did not skip braces inside string literals, character literals, or block
comments. A brace inside a literal within a cfg(test) block could cause the scanner to
miscalculate the block boundary, excluding production lines from LCOV and masking real
coverage gaps. Discovered during the 2026-06-11 campaign when PR #1321 (test-only) failed
Codecov/Patch-95 on its own test lines (a dead branch inside a cfg(test) block).

## Why

The scanner was correct for the common case (source with no brace characters inside
literals). The adversarial case was not tested. Coverage of the scanner itself was
measured against simple inputs only, so the gap was invisible in the test suite.

## Fix

PR #1327:  in  was replaced with a state
machine that tracks whether the current character is inside a string literal, character
literal, or block comment, and ignores braces encountered in those states. Two tests:
one proving production lines survive the transformation, one proving test-only lines
are excluded.

## Spec impact

Motivated Class 4 (Scanner Literal/Comment Blindness) and Class 6 (Coverage/Measurement
Integrity) in . Added to
 section 8.

## Portable lesson

Scanners that count delimiters must treat literal regions as opaque. Testing only with
clean inputs (no delimiters in literals) is insufficient; the adversarial input is a
source where the target delimiter appears exclusively inside a string/char/comment.

- **Pattern**: [docs/concepts/hazard-class-invariants.md](../concepts/hazard-class-invariants.md)
- **Class**: Class 4 -- Scanner Literal/Comment Blindness; Class 6 -- Coverage/Measurement Integrity
- **Generalization**: Any brace/delimiter scanner must be tested with delimiters inside literals.

## Related PRs

- [#1326](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/1326) -- issue: Codecov measures coverage of test lines added by test-only PRs
- [#1327](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1327) -- fix: replace brace scanner with literal-aware state machine
- [#1321](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1321) -- trigger PR: dead cfg(test) branch failed Patch-95
