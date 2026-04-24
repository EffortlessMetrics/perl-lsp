# Deep Review Fix-Forward Catalog: Session 2026-04-24

*Concrete record of correctness bugs caught by the sonnet deep-review pass in the
2026-04-24 session. Each entry shows what would have shipped without the review gate.*

---

## Why This Document Exists

The SESSION_7_ECONOMICS article established a pattern: deep review (sonnet tier) catches
semantic and integration-scale bugs that haiku standards review cannot reach. This catalog
extends that record with findings from the 2026-04-24 session.

The pattern continues to hold: no deep review in any Era 7 session has returned without
finding something to fix. This document is the evidence record.

---

## The Two-Tier Design Is Load-Bearing

Before the catalog, the architecture point:

**Haiku standards review** operates on surface properties: banned patterns (`unwrap()`,
`expect()`, `panic!()`), clippy cleanliness, fmt compliance, scope containment, branch
contamination. It is fast, cheap, and mechanical. Hit rate this session: ~40% of PRs had
something haiku caught.

**Sonnet deep review** operates on semantic correctness: does the logic work? Are there
coordinate-space bugs? Do the tests actually constrain the code's behavior? Are there
missing edge cases? Hit rate this session: every PR reviewed had at least one finding.

These are not redundant. The haiku pass is a precondition (clean code is easier to reason
about correctly), but it is not a substitute. The 2-pass design is the reason correctness
bugs caught in this session did not reach master.

---

## The Catalog

### Entry 1 — PR #5881: Lexicographic Sort Key Boundary

**Area**: Semantic completion ranking (`perl-semantic-analyzer`)
**Root cause**: Sort key generation used `{:02}` zero-padded decimal formatting for
hop-count prefixes. At tens boundary (e.g., hop 11 vs. hop 9), `"09"` sorts after `"11"`
lexicographically but 9 < 11 numerically. The error is invisible for fewer than 10 ancestor
scopes and surfaces when deep call chains exist.
**Fix**: Widen the padding to ensure lexicographic and numeric ordering agree across the
expected range of hop counts. Verified by new tests spanning the tens boundary.
**Test added**: Boundary comparison tests for hop-count sort keys at 9, 10, 11, and 20.
**ROI**: Completion ranking silently wrong for users in deeply nested Perl class hierarchies.
No error, no warning — just subtly wrong suggestions. Would have been reported as a UX bug
with no obvious root cause.

---

### Entry 2 — PR #5882: Multiline `@INC` Path Truncation

**Area**: Module path resolution (`perl-module`)
**Root cause**: `extract_quoted_list` (or equivalent) iterated over raw source lines,
not over logical Perl statements. A multiline `use lib` form like:

```perl
use lib (
    "/opt/local/lib",  # some comment
    "/usr/local/lib",
);
```

would truncate at the inline `#` comment on the first path, producing an incorrect path
and missing all subsequent paths in the list.
**Fix**: Add a quote-aware statement splitter (`split_perl_statements`) that handles
semicolons inside strings correctly and treats parenthesized multiline forms as one unit.
**Test added**: `multiline_use_lib_is_extracted`, `multiline_use_and_no_lib_are_ordered`,
`quoted_semicolons_in_path_not_split`.
**ROI**: `@INC` paths from multiline `use lib` declarations would be silently dropped,
causing spurious "module not found" diagnostics for modules that are genuinely available.

---

### Entry 3 — PR #5894: Dead Symlink Guard Semantics

**Area**: Completion provider for `@INC` module scanning (`perl-completion`)
**Root cause**: The symlink guard used `is_dir()` to decide whether a path should be
walked for module names. On a symlink-to-directory, `std::fs::DirEntry::is_dir()` follows
the symlink and returns true — but a dead symlink (symlink pointing to a non-existent
target) returns false for `is_dir()` and also false for `is_file()`. The guard for dead
symlinks was written as `!entry.path().is_dir()`, which is always true for dead symlinks
regardless of whether they are dead. The branch that should have skipped dead symlinks
never ran.

Separately, `lstat` semantics: `is_symlink()` is true for the symlink itself (regardless
of target), while `is_dir()` follows the symlink. The correct guard for "skip dead symlinks"
requires checking `is_symlink() && !is_dir()`.
**Fix**: Replace `is_dir()` guard with `is_symlink() && !metadata().is_ok()` (or
equivalent lstat-aware check).
**Test added**: Symlink-aware directory scan test with a dead symlink present.
**ROI**: Without the fix, dead symlinks in `@INC` directories cause the scan to either
panic (if path operations are called on an invalid target) or silently skip valid modules
that appear after the dead symlink in the directory listing.

---

### Entry 4 — PR #5927: `agent_receipt` Schema vs. Runtime Mismatch

**Area**: CI receipt emission (`perl-ci-hygiene`)
**Root cause**: The JSON schema for the agent-facing gate receipt declared `agent_receipt`
as an optional field (nullable). The Rust struct had it as non-optional (`AgentReceipt`,
not `Option<AgentReceipt>`). Any consumer that received a receipt without the field would
fail to deserialize, breaking backward compat against clients built to the schema contract.
**Fix**: Align either the struct (wrap in `Option`) or the schema (make required) to
create a consistent contract. The fix made it optional in Rust to match the schema
flexibility claim.
**Test added**: Round-trip deserialization test with and without the field present.
**ROI**: This is a schema-vs-runtime mismatch of the type that causes silent breakage in
automated consumers. An agent reading a receipt expecting the field to be optional would
receive a deserialization error with an unrecognized error message.

---

### Entry 5 — PR #5932: SLO Tracking Gap on Reference Query

**Area**: Workspace index cross-file query (`perl-workspace-index`)
**Root cause**: `ProductionIndexCoordinator::query_symbol_references` delegated to the
inner index implementation but did not wrap the call with SLO timing instrumentation. All
other production query methods had SLO tracking; this one was a raw passthrough. It would
produce no latency metrics, making p95 analysis blind to any regression in this code path.
**Fix**: Wrap the implementation call with the existing SLO instrument pattern.
**Test added**: Observation that SLO metrics are emitted after a reference query (integration
test asserting the counter increments).
**ROI**: Reference queries are a hot path (used by find-references, workspace rename
planning). Invisible to SLO dashboards means invisible to performance regression detection.

---

### Entry 6 — PR #5938: Initialize-Before-Shutdown Guard vs. Error Recovery

**Area**: LSP lifecycle dispatch (`perl-lsp-rs`)
**Root cause**: The PR added an "initialize-before-shutdown" guard: the server would reject
`shutdown` if it had not received `initialize`. This is correct per the LSP spec in the
happy path. However, the LSP router has a documented error-recovery exemption: it allows
certain lifecycle-adjacent messages to proceed even without full initialization, to support
crash-recovery and reconnect scenarios. The new guard conflicted with this exemption,
causing the server to reject `shutdown` in error-recovery paths that were previously handled.
**Fix**: Remove the guard (the spec-correct behavior is that shutdown can be called to
terminate a session regardless of initialization state — the server should exit cleanly
rather than reject with an error).
**Test added**: Test asserting `shutdown` succeeds in a session that never completed
`initialize`.
**ROI**: Without the fix, editor crash-recovery paths (reconnect after server crash) would
fail to clean up the server process, leaving orphaned server instances.

---

### Entry 7 — PR #5939: Malformed Trace Notification Resets to `"off"`

**Area**: LSP trace level handling (`perl-lsp-rs`)
**Root cause**: The `$/setTrace` handler used a collapsed `if let Some(value)` pattern
that, on the non-matching arm (when the trace value was not recognized), silently reset
the trace level to `"off"`. This is specified behavior in the LSP spec (unknown values
fall back to `"off"`), but the implementation was doing this via an implicit fallthrough
rather than an explicit assignment. The deep reviewer found that the `"off"` constant
was not tested in isolation from the wildcard-default arm — a mutant that changed `"off"`
to any other value would have survived, because the existing tests did not distinguish
the explicit `"off"` arm from the wildcard branch.
**Fix**: Centralize trace level values as named constants; make `normalize_trace_level`
explicit; test each arm independently.
**Test added**: Per-arm tests that distinguish `"off"`, `"messages"`, `"verbose"`, and
an unrecognized value — each independently asserting the output.
**ROI**: Mutant-survival gap in production trace handling. A code change that broke `"off"`
handling specifically would not be detected by CI.

---

### Entry 8 — PR #5946: `assert_eq!` Inside `proptest!` Bypasses Shrinking

**Area**: Lifecycle state machine property tests (`perl-lsp-rs`)
**Root cause**: Property-based tests in `proptest` should use `prop_assert!` and
`prop_assert_eq!`, not the standard `assert_eq!` macro. The difference: `assert_eq!`
panics immediately on failure, stopping shrinking. `prop_assert_eq!` returns a `TestCaseError`,
allowing proptest to shrink the failing input to its minimal form. Tests using `assert_eq!`
inside `proptest!` blocks produce large, unshrunken counterexamples that are hard to debug.

Additionally, the `initialize_requested` flag was mutated during test but never asserted,
making the state machine test vacuous with respect to that flag.
**Fix**: Replace `assert_eq!` with `prop_assert_eq!` throughout proptest blocks; add
assertions on `initialize_requested`.
**Test added**: Proptest block now correctly shrinks on failure; flag assertions added.
**ROI**: Tests that cannot shrink produce noise on failure. A proptest that uses `assert_eq!`
is half-broken: it finds failures but makes them hard to diagnose.

---

### Entry 9 — PR #5952: Rebase Would Drop BDD Tests; 3 Vacuous Invariants

**Area**: Lifecycle dispatch fuzz tests (`perl-lsp-rs`)
**Root cause**: Two separate findings:

1. The PR branch, if rebased naively onto master at that point, would have dropped 4 BDD-
   style lifecycle tests that had been added by a sibling PR. The rebase conflict was not
   a git merge conflict (no line-level overlap) but a semantic conflict: both PRs added
   tests to the same module, and a rebase would silently discard the sibling's additions.
   Deep review caught this by checking the before/after state of the test module.

2. Three of the fuzzing invariants were vacuous: they asserted properties that could not
   be falsified by any input the fuzzer would generate (e.g., asserting that a counter is
   non-negative when the type is `u32`). These tests add no coverage signal.

**Fix**: Repair the test module to preserve both PR's tests; replace vacuous invariants
with genuine behavioral assertions.
**Test added**: 3 new non-vacuous invariants; BDD tests preserved.
**ROI**: Vacuous tests create false coverage confidence. A test that cannot fail is not a
test — it is documentation pretending to be verification.

---

### Entry 10 — PR #5956: Mutant-Survival Gap in `"off"` Match Arm

**Area**: Lifecycle dispatch mutation coverage (`perl-lsp-rs`)
**Root cause**: The `"off"` arm of the trace-level match had no test that distinguished
it from the wildcard default arm. A mutant that changed `"off"` to `"verbose"` in the
match arm would survive because the existing tests did not verify which arm handled an
explicit `"off"` input.
**Fix**: Add a targeted mutation-hardening test for `"off"` specifically, verifying it
produces a distinct outcome from the wildcard case.
**Test added**: Mutation-hardening test isolating the `"off"` arm.
**ROI**: Mutation-surviving code paths are the class of bugs that survive all existing
tests. Filling this gap means a future regression in `"off"` handling will be caught by
CI rather than reported by a user.

---

### Entry 11 — PR #5985: Coordinate-Space Mixing in Position Mapping

**Area**: Incremental parsing shifted-node reuse (`perl-incremental-parsing`)
**Root cause**: `map_old_position_to_new` compared edit boundaries from old-source space
against position shifts computed in new-source space. The mismatch caused incorrect
decisions about which nodes were safe to reuse after batch edits, producing either over-
reuse (wrong parse tree returned) or under-reuse (unnecessary full re-parse).

A separate bug: the test helper for multi-line edit scenarios was constructing the
expected position incorrectly, masking the coordinate-space bug in the existing tests.
**Fix**: Ensure all position comparisons are performed in the same coordinate space;
fix the test helper.
**Test added**: Explicit multi-line batch edit test with position assertions that would
have caught the original bug.
**ROI**: Coordinate-space mixing in incremental parsers produces wrong AST content
silently — the parser appears to succeed but returns incorrect parse trees. This would
manifest as incorrect completions, wrong go-to-definition targets, and false diagnostics
for edited files.

---

### Entry 12 — PR #6001: `adjust_positions` Segment-Envelope Invariant

**Area**: Incremental checkpoint position adjustment (`perl-incremental-parsing`)
**Root cause**: The `adjust_positions` function operated on token coordinates relative
to segment envelopes. A double-shift could occur when a token's position was adjusted
both by the segment-offset logic and by the token-offset logic, producing positions outside
the valid range. The invariant "no double-shift" was not pinned by any test.
**Fix**: Add a guard that detects and rejects double-shift; pin the invariant in a test.
**Test added**: Invariant test asserting positions are monotonically non-decreasing after
adjustment and within the valid segment envelope.
**ROI**: Silent position drift in incremental checkpoint data produces wrong line/column
numbers for all subsequent LSP responses for affected files, until the file is fully
re-parsed. The corruption is invisible (no error, no warning) and would be reported as
wrong hover positions or wrong diagnostic locations.

---

### Entry 13 — PR #6018 (First Pass): Vacuous Assertions and Stale Documentation

**Area**: Batch edit normalization and UTF-8 boundary handling (`perl-incremental-parsing`)
**Root cause** (first pass): Three findings:

1. A vacuous `assert!(true)` assertion in a boundary test — the assertion could never fail.
2. Doc comment claiming `<1ms` for a code path that benchmarks at 3-8ms under realistic
   load — stale documentation.
3. Three adjacency edge case tests were missing (consecutive zero-width insertions at
   the same offset; edit at the exact boundary of a cached segment).

**Fix**: Remove vacuous assertion; update documentation; add three edge case tests.
**Test added**: `test_consecutive_zero_width_insertions`, `test_segment_boundary_edit`,
`test_adjacent_edits_at_same_offset`.
**ROI**: Vacuous assertions create false test coverage signals; stale `<1ms` docs become
user-facing performance claims that fail in practice.

---

### Entry 14 — PR #6018 (Second Pass): SERIOUS — Double-Parse Regression

**Area**: Batch edit normalization (`perl-incremental-parsing`)
**Root cause** (second pass, after first pass fixes): A SERIOUS correctness and performance
regression was found in the core batch edit path:

1. **Double-parse on success**: The implementation called `parse_full()` twice on every
   successful batch edit — once to validate and once to produce the result. This means
   every client of the incremental parser was paying 2× full parse cost per edit cycle,
   completely defeating the purpose of incremental parsing. This bug was invisible in unit
   tests (the tests verified correctness, not performance) and invisible in CI (no benchmark
   gate). It would have been reported as a performance regression by users.

2. **Sort non-determinism**: The fallback path and the happy path sorted results differently,
   meaning the ordering of tokens/nodes could differ between a full parse and an incremental
   parse for identical inputs. Non-deterministic ordering breaks snapshot tests and produces
   flaky test failures.

3. **Fragile Debug-string assertions**: Three tests asserted behavior via `format!("{:?}")`,
   which depends on the `Debug` implementation of the type. Any change to the `Debug` impl
   (adding fields, reordering) would break the tests for reasons unrelated to the behavior
   being tested.

**Fix**: Eliminate the duplicate `parse_full()` call; normalize sort order between paths;
replace Debug-string assertions with structural assertions.
**Test added**: Performance regression test asserting `parse_full()` is called at most once
per incremental edit; sort-order determinism test.
**ROI**: The double-parse bug was a correctness-class performance regression: the incremental
parser was producing correct results at full-parse cost, making the feature economically
pointless. This is the most consequential fix of the session.

---

### Entry 15 — PR #6022: p95 Floor-Division Bug + Warmup Contamination

**Area**: Parser performance scorecard (`perl-parser-bench`)
**Root cause**: Two bugs in the scorecard computation:

1. **p95 floor-division**: For sample counts N ≤ 20, the p95 index computation used
   integer floor-division in a way that returned the maximum sample value rather than the
   95th percentile sample. For a 20-sample benchmark, p95 should be sample 19 (0-indexed);
   the bug returned sample 20 (off-by-one: max value instead of 95th percentile).

2. **Warmup contamination**: Round 0 of the benchmark (the first warm-up invocation) was
   included in the p95 computation. Round 0 consistently shows a 20× latency spike due to
   page faults and instruction cache priming. Including it in p95 produces an inflated
   result that was committed to the baseline JSON and would be reported as the "normal" p95
   in all future comparisons.

**Fix**: Fix the floor-division formula; exclude round 0 from all percentile computations.
**Test added**: Boundary tests for p95 computation at N=1, N=10, N=20, N=21.
**ROI**: A wrong p95 baseline makes all future regressions invisible (they appear normal
against an inflated baseline) or all future improvements invisible (everything looks like
a regression relative to a deflated baseline). The warmup contamination specifically
produces baselines 20× higher than the steady-state latency, making regression detection
meaningless.

---

### Entry 16 — PR #6031/#6032 Pair: `From<&Symbol>` Silently Drops Fields

**Area**: Semantic query facade (`perl-semantic-analyzer`)
**Root cause**: The `From<&Symbol>` implementation that converts internal `Symbol` values
to the public `ResolvedSymbol` projection type silently dropped the `declaration` and
`documentation` fields. These fields were present on `Symbol` but not mapped into the
`From` impl. Any consumer using the facade to read symbol documentation or find the
declaration site would receive `None` values for both fields, with no error indication.

Additionally, an architectural layer violation was identified: PR #6032 placed the facade
at a layer (`perl-parser`) that should not have access to workspace-index types. The correct
placement was `perl-semantic-analyzer`, which has appropriate upward visibility.
**Fix**: Complete the `From` impl; move the facade to the correct crate; add 5 edge tests
for the dropped fields.
**Test added**: Tests asserting `declaration` and `documentation` fields round-trip correctly
through the `From` conversion; test asserting the facade is in the correct module.
**ROI**: A type converter that silently drops fields violates the type system's implicit
contract. Users of the facade API would see "no documentation" for all symbols even when
documentation was present in the source — a correctness failure that looks like a missing
feature.

---

### Entry 17 — PR #6014: 400-LOC Module Size Gate Exceeded

**Area**: Parser corpus status and failure clustering (`xtask`)
**Root cause**: The implementation exceeded the 400-LOC module size guideline (from
`feedback_no_loc_caps.md`: organize by coherence, not line count — but there is a practical
limit beyond which single-module files become harder to navigate). The module required
refactoring into three focused submodules.
**Fix**: Extract into `corpus_audit/clusters.rs`, `corpus_audit/node_coverage.rs`, and
`corpus_audit/report.rs` with the parent module as coordinator.
**Test added**: No behavior change; tests carried over to new module locations.
**ROI**: This is a maintainability catch, not a correctness catch. A 400+ LOC single module
in xtask becomes a merge-conflict hotspot (multiple future changes touch the same file)
and a navigation burden for future agents reading the codebase.

---

## Summary Statistics

| Category | Count |
|----------|-------|
| Performance bugs (silent, correctness-class) | 3 (double-parse, p95 floor-div, coordinate-space) |
| Semantic correctness bugs (wrong behavior, no error) | 5 (sort key, @INC truncation, symlink guard, schema mismatch, SLO gap) |
| Test quality issues (vacuous, wrong framework, shrinking disabled) | 5 |
| Lifecycle/spec compliance | 2 (shutdown guard, trace reset) |
| Architectural / maintainability | 2 (layer violation, module size) |
| **Total distinct findings** | **17** |

All findings were pushed directly to the PR branch by the deep reviewer (fix-forward mode).
No builder round-trip was needed for any single finding.

---

## The Pattern

Across Era 7 sessions (7, Session 6, and now 2026-04-24), the deep review hit rate has
been 100%: no PR reviewed by the sonnet deep-review pass has come back clean. The
distribution of finding types is consistent:

- ~30% are performance bugs that are invisible to correctness tests
- ~30% are semantic correctness bugs (wrong field dropped, wrong coordinate space)
- ~20% are test quality issues (vacuous, wrong framework, inadequate coverage)
- ~20% are spec compliance or architectural issues

The haiku standards pass catches none of these categories. The two tiers are not redundant;
they are complementary. Removing either tier reduces the quality of the output materially.

---

_Related: `docs/articles/SESSION_7_ECONOMICS.md`, `docs/articles/VERIFICATION_LADDER_PER_LAYER_ROI.md`, `docs/articles/CONTINUOUS_REVIEW_PATTERNS.md`_
