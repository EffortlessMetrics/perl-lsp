# ADR-2026-0420: Test Code Quality Baseline and Panic Burn-Down

## Status
Proposed

## Context

GitHub issue #3237 tracks ~2,800 findings of modern-Rust idiom violations in perl-lsp test code, mirroring the production-code quality standards already enforced via `perl-ci-hygiene` baselines. The goal is test-code quality parity with production: `panic!`, `unwrap`, `expect`, `todo!` banned in test code just as they are in production.

**Critical problems with the initial issue data:**
- Issue counts are materially wrong (27%+ discrepancy on panic, 7x on unreachable, 3 of 5 listed crates don't exist in the workspace)
- No test baseline exists — without one, "done" is unmeasurable
- Phase 3 (3 unlinked TODOs) is empty — the only unlinked TODO is in production code; the 4 "todo!" in `missing_docs_ac_tests.rs` are raw string literals (fixture data), not real macro calls
- The `#[allow(clippy::panic)]` lint opt-out only exists in `perl-test-must` and `perl-tdd-support` themselves — crates that add `perl-tdd-support` as dev-dependency and use `must()` will have `clippy::panic = deny` active on their test code

The workspace MSRV is 1.92, which means `std::assert_matches!` (stabilized Rust 1.73) is available.

## Decision

Establish test baseline infrastructure **before** any remediation, and scope the remediation to only the mechanically verified actionable items:

### Decision 1: Baseline-First Ordering
Establish `ci/panic_test_baseline.txt` with the **current** (pre-remediation) `panic!` count in test code before any fixes are applied. This makes "done" measurable: the baseline IS the target, and the work is complete when the count reaches (or falls below) that number. The same pattern applies to `ci/todo_test_baseline.txt`.

**Rejected alternative:** Post-remediation baseline (mitigates "done" ambiguity by asserting "we went down" rather than "we reached a target").

### Decision 2: Exclude Unverified Categories
Limit this work item to the mechanically verified subset:
- `panic!` in match-arm catches (use `assert_matches!`)
- Verified crate dependency additions (only crates that actually exist in the workspace)

Exclude from this work item (require scout triage first):
- ~620 no-assertion test functions (judgment-heavy)
- ~2,000 `println!`/`eprintln!` calls (judgment-heavy — benchmark vs. leftover)
- ~133 hardcoded absolute paths (filter to fs-writing subset needed first)
- `unreachable!()` findings (paired with `must(Err)` for type-checker exhaustivity; not a simple replacement)

**Rejected alternative:** Batch all categories into one PR — unreviewable and risky given the judgment required for prints, paths, and no-assertion tests.

### Decision 3: `#[allow(clippy::panic)]` Propagation as First-Class Constraint
Every crate that adds `perl-tdd-support` as a dev-dependency and uses `must()`/`must_some()`/`must_err()` in its test modules **must** add `#![allow(clippy::panic)]` to its test module (or opt-out in `lints.rust`/`[lints.rust]`). This is not optional — without it, the workspace's `panic = "deny"` lint will cause a compile failure when `must()` is called.

**Rejected alternative:** Relying on the caller to discover the need organically — leads to compile failures during implementation.

### Decision 4: Use `assert_matches!` as Replacement Idiom
`std::assert_matches!` (Rust 1.73, workspace MSRV is 1.92) replaces `match` + `panic!` in match-arm catches. It produces better assertion output on failure than `assert!(matches!(expr, pattern))`.

**Rejected alternative:** `matches!()` + `assert!()` — worse failure output, no diagnostic benefit.

## Consequences

### Benefits
- "Done" is measurable: the baseline IS the target
- Zero new external dependencies (uses existing `perl-tdd-support` crate and std)
- Incremental gate enforcement via existing `perl-ci-hygiene` infrastructure
- Better test diagnostics (assert_matches! shows which pattern arm failed)

### Tradeoffs
- The `panic!` burn-down count is approximate (~132, not the 180 in the issue) — a fresh scan during Phase 1 will establish the precise count
- The `perl-tdd-support` dev-dep list is limited to 2 verified crates (perl-dead-code, perl-lsp-feature-policy), not the 5 listed in issue #3237
- Phase 3 (unlinked TODOs) is effectively empty — no test-code TODO work exists

### Risks
1. **Stale crate list blocks Phase 2** — mitigated by limiting to verified crates only
2. **Missing `#[allow(clippy::panic)]` causes compile failures** — documented as first-class constraint; every crate adding `perl-tdd-support` must include the allow attribute
3. **Production gate already red** — 7 `unreachable!()` in `perl-parser/src/heredoc_anti_patterns.rs` against baseline 0; this is a separate production concern, not test-code work

## Alternatives Considered

| Alternative | Why Rejected |
|---|---|
| Post-remediation baseline | "Done" is ambiguous — asserts "went down" not "reached target" |
| Batch all 7 categories | Unreviewable PR; judgment-heavy categories need scout triage first |
| `matches!()` + `assert!()` | Worse failure output than `assert_matches!` |
| Don't document `#[allow(clippy::panic)]` propagation | Compile failures during implementation; discoverable only by running cargo check |
| Include `unreachable!()` in burn-down | Paired with `must(Err)` for type-checker exhaustivity; not a simple replacement |
