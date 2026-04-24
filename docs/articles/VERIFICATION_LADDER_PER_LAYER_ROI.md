# Verification Ladder ROI by Layer: Concrete Catch Data from One Session

**Date**: 2026-04-19
**Session**: Wave G1 collapse on perl-lsp (5 PRs, 74 → 49 published crates, ~11 hours)
**Cross-references**: [DOWNSTREAM_CATCHES_UPSTREAM.md](DOWNSTREAM_CATCHES_UPSTREAM.md), [COST_ROI.md](COST_ROI.md), [SESSION_7_ECONOMICS.md](SESSION_7_ECONOMICS.md)

---

## TL;DR

perl-lsp's verification ladder routes claims and PRs through seven sequential layers: accuracy-scout, research-verifier, oppositional-planner, advocatus-diaboli, architecture-reviewer, maintainer-{issue,pr}, and reviewer-deep (with green-tdd and diff-auditor as post-build gates). Each layer is a different kind of check against a different kind of error.

The standard argument for a deep ladder is economic — each downstream layer is more expensive than the previous, so you want the cheap layers to catch what they can before handing off. This article provides **concrete per-layer catch data from a single 11-hour session** on perl-lsp to show the ladder isn't redundancy theater: each layer caught bugs the previous layers couldn't have caught, at costs roughly proportional to the severity of the catch.

The data:

| Layer | Catches in this session | Notable |
|---|---|---|
| accuracy-scout | 5 corrections (file paths, symbol locations shifted post-G1a) | Mechanical; required before any downstream layer could trust facts |
| research-verifier | 1 **false-premise kill** (#4498 closed, #4499 filed as pivot) | Closest to catastrophic — entire issue premise was wrong |
| oppositional-planner | 2 intra-dep corrections; 1 "80% caught" overclaim flagged | Scope/claim calibration |
| advocatus-diaboli | 2 valid DEFERs (scope-pivoted, see companion article); 1 clean BUILD | Judgment on existence |
| architecture-reviewer | 1 consolidation opportunity (existing Python check → xtask) | Prevented duplicate implementation |
| maintainer-{issue,pr} | Alignment checks — 0 blockers, 3 notes (wrapper-type audit watchlist) | Strategic fit |
| reviewer-deep | **3 real bugs on #4504 alone** (missing `\|\| true` on grep pipeline, MSRV toolchain mismatch, vacuous test assertion) | Would have shipped otherwise |
| green-tdd | 1 regression (3-import miss on #4510); 4 new edge cases | Builder-level catch after builder was "done" |
| diff-auditor | 0 caught bugs; verified R100 snapshot byte-identity | Clean-diff gate, scope drift |

Total layer-distinct catches: **~19 across 9 agents**. No two layers caught the same kind of bug.

---

## What Each Layer Caught, Specifically

### accuracy-scout (5 corrections)

Example: on issue #4501 (Wave G1b), the issue body was written pre-G1a-merge. Post-G1a, the 10 G1b provider crates' `Cargo.toml` files had been updated to depend on `perl-lsp-rs-core` instead of the now-absorbed G1a crates. The issue body still referenced the pre-G1a dependency structure. accuracy-scout caught the shift, wrote a corrected post-G1a dependency map in its verification comment, and all downstream layers worked from the corrected version.

Cost: ~5 minutes of haiku time per accuracy pass.

Severity of uncaught-hypothetical: downstream agents would have worked from stale dependency facts, producing subtly wrong plans and tests. Probably recoverable but expensive.

### research-verifier (1 false-premise kill)

Issue #4498 proposed adding `cargo publish --dry-run` per crate to catch registry-side validation failures (keyword limits, wildcard deps, auth). Research-verifier fetched [cargo docs](https://doc.rust-lang.org/cargo/commands/cargo-publish.html), [RFC 1241](https://rust-lang.github.io/rfcs/1241-no-wildcard-deps.html), and [cargo issue #5941](https://github.com/rust-lang/cargo/issues/5941) and established that `cargo publish --dry-run` is entirely local — zero network contact, zero registry validation. The issue's core premise was false.

Diaboli acted on research-verifier's finding and returned CLOSE. The issue was closed; a pivot issue #4499 was filed with the correct premise (consolidate existing Python `--check-drift` + add LICENSE-present). #4499 later merged as PR #4505.

Cost: ~8 minutes of haiku + 3 web fetches.

Severity of uncaught-hypothetical: the original #4498 would have shipped a CI gate that **looked like** it caught registry failures but didn't. Worse than no gate, because it creates false confidence. Could have caused a real publish-failure at v0.13.0 release time, weeks later, when the actual source of the failure would be very expensive to diagnose.

### oppositional-planner (3 flags across 2 issues)

Two intra-G1a dependency pairs (`perl-lsp-file-completion` → `perl-lsp-completion-item`; `perl-lsp-workspace-symbols` → `perl-lsp-symbol-query`) were missed by the original scout's "no inter-provider dependencies" framing. Oppositional-planner caught both by reading each crate's Cargo.toml and flagging the dependencies for plan-reviewer to bake into the builder checklist (as "helper-before-consumer" sequencing).

Also on #4499, oppositional-planner challenged the issue body's "80% of failures caught" claim and computed the real catch rate at ~33% (allowlist drift + missing LICENSE; the other four checks were already caught by `cargo package`). Plan-reviewer corrected the framing.

Cost: ~6 minutes of haiku per pass.

Severity of uncaught-hypothetical: builder would have absorbed `perl-lsp-file-completion` before `perl-lsp-completion-item` existed at its new path, triggering a mid-PR compile break. Recoverable (builders do figure it out) but costs ~15 minutes of in-PR thrashing per pair.

### advocatus-diaboli (2 valid DEFERs, 1 BUILD)

On #4497 and #4499, diaboli returned DEFER with rationale anchored to the original scope. Both were scope-pivoted by the orchestrator and reversed — see [SCOPE_PIVOT_ON_DEFER.md](SCOPE_PIVOT_ON_DEFER.md).

On #4500 (G1a), diaboli returned BUILD after the DEFER-worthy 48h soak period on Wave F was calculated to have elapsed. Diaboli explicitly named the 48h-soak-risk and said "proceed, soak period met."

Cost: ~5 minutes of haiku per verdict.

Severity of uncaught-hypothetical: without the scope-pivot companion pattern, diaboli's conservative bias would have delayed 2 PRs by days to weeks. The DEFERs themselves weren't wrong — they were reading conservative-at-the-proposed-scope. The orchestrator's scope-pivot tool turned those DEFERs into productive scope corrections.

### architecture-reviewer (1 consolidation opportunity)

On #4499, architecture-reviewer noted that the existing Python `scripts/publish-topo.py --check-drift` invocation in `.github/workflows/publish-dry-run.yml` covered the same ground as the proposed xtask `publish-manifest-check`. Rather than duplicate, architecture-reviewer recommended consolidating — move the Python logic into the new xtask subcommand.

Plan-reviewer adopted the recommendation. Final scope: net LOC reduction, not addition.

Cost: ~5 minutes of haiku.

Severity of uncaught-hypothetical: would have shipped a duplicate check. Not a bug per se, but long-term cleanliness cost. Each future addition would have had to update both the Python script and the xtask subcommand.

### maintainer-{issue,pr} (alignment, watchlist items)

No blockers. But the maintainer-pr on #4510 flagged that G1a had 3 builder API-shape fixes and G1b had 6 — "wrapper-type audit watchlist, revisit after G2." That observation became issue #4513 and the article [LLM_READS_SPEC_NOT_CODE.md](LLM_READS_SPEC_NOT_CODE.md).

Cost: ~3-5 minutes of haiku per PR.

Severity of uncaught-hypothetical: soft. The G2 wave's red-TDD would have continued the pattern — 12 projected API-shape fixes instead of 2 — accumulating silent cost that eventually breaks the growth curve.

### reviewer-deep (3 real bugs on #4504)

The headline catch of the session. On PR #4504 (facade-only `cargo public-api` ratchet), reviewer-deep found three bugs that every preceding layer missed:

1. **Missing `|| true` on grep pipeline.** The justfile recipe ran `cargo public-api -p <crate> --simplified 2>/dev/null | grep "^pub "`. Under `set -euo pipefail`, `grep` exits 1 when there's no match. If any crate's `cargo public-api` invocation silently failed (stderr redirected to /dev/null), grep would get empty input and exit 1, aborting the script before the `FAILED` counter was evaluated. This means a silent cargo-public-api failure would present as "CI passed" while actually having skipped crates.

2. **Toolchain mismatch (MSRV vs stable).** The baselines were captured at the workspace MSRV (1.92.0). The new CI job's checkout used `toolchain: stable`. When stable rustc advances and changes rustdoc JSON output format, the baseline check would false-positive against master for a reason unrelated to API changes.

3. **Test D's `--simplified` assertion was vacuous.** The green-TDD test asserted that `--simplified` appeared in `ci-nightly.yml`. The string appeared — but in a YAML **comment**, not in any executable code. The test would pass even if the actual step dropped the flag. Reviewer-deep replaced with a real behavioral check (asserting `set -euo pipefail`, `|| true` guard, `diff -u`, and `FAILED=1 / exit 1` in the recipe body).

All three bugs would have shipped. All three would have silently degraded the gate — the gate's alert would fire on spurious inputs, or miss real drift. Once that happens, engineers train themselves to ignore the gate. Gate becomes noise. Silent.

Cost: ~12 minutes of sonnet time, 3 fix commits, 1 force-push to the PR branch.

Severity of uncaught-hypothetical: the whole point of the ratchet gate is to catch drift. A ratchet with silent partial coverage, false positives, and vacuous tests isn't a ratchet — it's decoration. Catching this at reviewer-deep is the difference between shipping a useful tool and shipping theater.

### green-tdd (regression on #4510)

Builder on #4510 (Wave G1b) completed ~60 minutes of work collapsing 10 crates, then green-tdd ran a hardening pass and found that three imports in `crates/perl-lsp-rs/tests/wired_crates_integration_test.rs` still referenced `perl_lsp_inline_completion` (the crate being absorbed). Builder had missed them. Green-tdd flagged `needs-builder-fix`; builder pushed a 3-line correction commit.

Cost: ~10 minutes of haiku.

Severity of uncaught-hypothetical: `cargo check -p perl-lsp --tests` would have failed post-merge. Not catastrophic (master's push-triggered CI doesn't compile integration tests — see [forensics §1](../forensics/2026-04-19-wave-g1-collapse-retrospective.md)) but would have surfaced in the next full-test run. Fast-follow fix.

### diff-auditor (0 bugs, cleanliness verification)

Final gate. Verified the 5 PRs' cumulative diffs were scope-clean — no accidental file changes outside spec, no leftover debug artifacts, no `println!` or `dbg!` in production code, snapshots were git R100 renames with byte-identical content. All clean. 0 catches.

Cost: ~3 minutes of haiku per PR.

Severity of uncaught-hypothetical: unknown — diff-auditor is the final safety net. If there had been scope drift or leftover artifacts, this is where we'd have caught them. None this session.

## ROI By Layer

Aggregating roughly:

| Layer | Session cost (~agent-minutes) | Catches | Cost per catch |
|---|---|---|---|
| accuracy-scout | ~25 (5 passes) | 5 mechanical corrections | 5 min/catch |
| research-verifier | ~20 (3 passes) | 1 false premise | 20 min/catch |
| oppositional-planner | ~20 (4 passes) | 3 substantive flags | 7 min/catch |
| advocatus-diaboli | ~20 (4 passes) | 3 verdicts (2 scope-pivoted, 1 BUILD) | 7 min/verdict |
| architecture-reviewer | ~15 (3 passes) | 1 consolidation | 15 min/catch |
| maintainer-{issue,pr} | ~20 (5 passes) | 1 watchlist (growth pattern) | 20 min/catch |
| reviewer-deep | ~60 (5 passes) | 3 real bugs on #4504 + correctness OK on 4 others | 20 min/bug |
| green-tdd | ~30 (5 passes) | 1 regression + hardening on all | 30 min/catch |
| diff-auditor | ~15 (5 passes) | 0 | — (safety net) |

Total agent-minutes: ~225 for the full ladder across 5 PRs.

Total PR count shipped: 5.

Total unique bugs caught at layers where they were catchable: ~16.

**Per-PR agent cost for ladder: ~45 agent-minutes.** Per the session's haiku/sonnet mix and the model-pricing context, this is a small fraction of the total shipping cost, and it prevents bugs that would have cost multiples more to catch after merge.

## What The Data Says

Three observations:

1. **Every layer catches a different *kind* of error.** No two layers caught the same kind of bug. The ladder isn't redundancy; it's orthogonal coverage. Each agent specializes in an error class the others can't see.

2. **reviewer-deep is the highest-cost / highest-severity layer**, and the session's most expensive catches happened there. That's correct. Cheap layers filter cheap bugs; expensive bugs survive to the expensive layer. The ladder's cost increases monotonically with severity, and so does the catch-severity distribution.

3. **Research-verifier's single catch was the highest-consequence.** It saved an entire false-premise issue from shipping a broken gate. The 20-minute cost bought a cancel-and-refile that would have cost days to diagnose and fix at release time.

There's no single layer that can replace the ladder. A pipeline that tries to compress the ladder into "standards review + CI" would have shipped the 3 reviewer-deep bugs, the false-premise ratchet, the green-tdd regression, and the intra-dep sequencing errors. Probably not all in the same session — but over enough sessions, all of them. The ladder is the cost of not shipping them.

## Related

- [DOWNSTREAM_CATCHES_UPSTREAM.md](DOWNSTREAM_CATCHES_UPSTREAM.md) — companion pattern (reversed catch direction)
- [COST_ROI.md](COST_ROI.md) — broader economics
- [SESSION_6_ECONOMICS.md](SESSION_6_ECONOMICS.md) / [SESSION_7_ECONOMICS.md](SESSION_7_ECONOMICS.md) — prior session retrospectives
- [SCOPE_PIVOT_ON_DEFER.md](SCOPE_PIVOT_ON_DEFER.md) — companion pattern (orchestrator judgment on DEFER verdicts)
- [LLM_READS_SPEC_NOT_CODE.md](LLM_READS_SPEC_NOT_CODE.md) — companion pattern (red-TDD failure mode)
- [forensics/2026-04-19-wave-g1-collapse-retrospective.md](../forensics/2026-04-19-wave-g1-collapse-retrospective.md) — full session
