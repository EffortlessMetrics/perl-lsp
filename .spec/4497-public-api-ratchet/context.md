# Context: Issue #4497 — Facade-Only Public API Ratchet

**Issue:** https://github.com/EffortlessMetrics/perl-lsp/issues/4497
**Parent:** #4410 (Microcrate Collapse — 135 → 31 crates)
**Scope:** Facade-only (5 crates)
**Status:** Builder-ready with critical spec correction from plan-review

---

## Problem Statement

The microcrate collapse (issue #4410) reduces the published crate set from 135 down to 31. The 5 primary user-facing facade crates are:
- `perl-lsp-rs` — LSP server library
- `perl-parser` — Parser library
- `perl-uri` — URI utilities
- `perl-dap` — DAP server library
- `perllsp` — LSP binary wrapper

Currently, **there is no hard-fail CI guardrail** that prevents accidental public API surface changes in these 5 facades. The existing `semver-check:` job in ci-nightly.yml only covers 3 crates (perl-parser, perl-lexer, perl-parser-core) and has `continue-on-error: true`, so it does not fail CI.

**Risk:** After the microcrate collapse goes public in v0.13.0, external Perl developers will depend on these 5 library APIs. An accidental export change (e.g., re-export visibility shift, function signature change) would scatter imports and break downstream code.

**Mitigation:** Lock the public API surface of the 5 facade crates with a per-crate text baseline. Fail CI hard when the surface drifts without an explicit update.

---

## Key Decisions

### Scope: 5 crates, not 74 or 31

**Original proposal:** Guard all 74 published crates.
**Revised scope:** Guard only the 5 primary facades.
**Rationale:**
- Facades are **designed not to churn** during internal refactors (Wave D/F split achieved this)
- Internal satellites (e.g., perl-lexer-core) can refactor freely; facades stay stable
- Baseline maintenance cost drops from ~93 files to 5 — sustainable for v0.13.0 launch
- Coverage is complete for user-facing APIs; internal churn does not leak to consumers

**Who decided:** Architecture-reviewer and maintainer-issue agent confirmed this is the right grain.

### Mechanism: Text baselines + diff, not `cargo public-api diff` subcommand

**Original spec said:** Use `cargo public-api diff --package <crate>` to compare against baselines.
**Problem:** This command does not exist. The `diff` subcommand only compares against crates.io or git commits — there is no file-based diffing mode.

**Corrected mechanism:** 
1. Capture: `cargo public-api -p <crate> --simplified 2>/dev/null | grep "^pub " > .ci/public-api-baselines/<crate>.txt`
2. Check: Fresh capture to temp file, then use standard `diff -u baseline.txt current.txt`
3. CI integration: Redirect diff output to PR comment or log, fail on non-empty diff

**Correction made by:** Plan-reviewer agent (verified locally before finalizing spec).

### `--simplified` flag is mandatory

**Original question:** Should we use `--simplified` or full output?
**Answer:** Always use `--simplified` (`-s` short form).

**Evidence:**
- Without `-s`: perl-dap baseline is ~10,246 lines
- With `-s`: perl-dap baseline is ~3,184 lines
- Impact: A cosmetic change like adding `#[derive(Debug)]` to a DAP struct changes ~100 lines without `-s`
- Verdict: `-s` reduces noise and baseline churn; use consistently in both capture and check

**Who verified:** Plan-reviewer tested locally on stable 1.92.0.

### `perllsp.txt` will be ~2 lines

**Question:** Is a 2-line baseline a bug or correct?
**Answer:** Correct and expected.

**Context:**
- `perllsp` is a thin binary wrapper around `perl-lsp-rs`
- It has both `[lib]` and `[[bin]]` targets
- `cargo public-api` reports only the lib target (which is the public API)
- The lib target is just re-exports from `perl-lsp-rs`
- Result: ~2 public items in the baseline

**Implication:** Builders must not interpret this as an error or attempt to expand it. Document it in the checklist as expected.

### Hard-fail CI, not warning-only

**Question:** Should `public-api-check:` inherit `continue-on-error: true` from the existing `semver-check:` job?
**Answer:** No. The new job must hard-fail.

**Rationale:**
- Existing `semver-check:` is warning-only (continues on error) because it runs nightly and only on schedule — not a merge gate
- The new `public-api-check:` is also nightly+schedule, but different purpose: lock facades post-collapse
- For v0.13.0 launch, facade stability is critical — hard-fail ensures developers see drift immediately
- Labels: Use the same pattern as `ci:semver` — introduce `ci:public-api` to trigger the check on specific PRs

**Implication:** The job will fail CI if surface changes without baseline update. This is intentional and correct.

### Semver-check job sync

**Finding:** The `justfile:semver-check-all` recipe covers 5 crates, but the CI `semver-check:` job only hardcodes 3.
**Action:** Sync the CI job to cover all 5 crates (add perl-lsp-rs and perllsp steps).
**Why:** Consistency between local (`just semver-check-all`) and CI (`semver-check:` job) prevents hidden failures.

---

## Alternatives Considered and Rejected

### Option A: Text baselines + capture-and-diff (CHOSEN)
- **Cost:** 5 baseline files, justfile commands, CI job integration
- **Benefit:** Stable, verifiable, works with any tool
- **Chosen:** Yes — matches existing `.ci/` pattern (coverage-baseline.txt, cpan-corpus-baseline.json)

### Option B: Direct `cargo public-api diff` against crates.io
- **Cost:** Lower (no baselines to store)
- **Benefit:** Always compares against latest published version
- **Rejected:** Does not work for pre-release v0.13.0; would need to publish first. Also, tool's `diff` subcommand is git/semver-based, not file-based.

### Option C: Justfile-only, no CI enforcement
- **Cost:** Near zero (one recipe)
- **Benefit:** Local check only
- **Rejected:** Too easy to skip; v0.13.0 public launch needs hard guarantee. Plan-reviewer marked this as high-risk.

### Option D: Merge-blocking gate in `ci.yml` (DEFERRED)
- **Cost:** New workflow, different trigger pattern
- **Benefit:** True merge gate (not just nightly)
- **Status:** Deferred to post-Wave G1a completion. v0.13.0 will use nightly + label gate.

---

## Related Issues and Decisions

### #4410 — Microcrate Collapse Tracker (Parent)
- Defines the 31 surviving crates
- Lists 7 deferred CI guardrails (of which this is one)
- Links to wave PRs (D, F, G1a, etc.)

### #4499 — Wave A1 (Dependency)
- Manifest-check ratchet (sister issue)
- Also uses `.ci/` text baselines pattern
- Builder should look at #4499 for parallel reference

### v0.13.0 Release Story
- Public alpha announcement planned post-collapse
- These 5 facades are the "published surface" users will depend on
- Baseline ratchet locks this surface for the release
- Must land before GA announcement

---

## Builder Notes and Gotchas

### Baseline capture is build-dependent
- Running `cargo public-api -p <crate>` triggers a full build
- On first run, expect 2-5 minutes per crate
- Subsequent runs are cached (faster)
- The 5 captures will take ~15-20 minutes total in CI

### Temp file cleanup
- The justfile recipes create `/tmp/<crate>-current.txt` and `/tmp/<crate>-diff.txt`
- On Windows (using MSYS2), `/tmp` maps to the MSYS2 mount point
- On Linux CI, `/tmp` is the actual temp filesystem
- Builder should ensure these temp files do not accumulate (they're regenerated each run)

### Grep pattern is strict
- The grep pattern `^pub ` (must start with "pub ") filters:
  - `pub fn foo()` ✓
  - `pub struct Foo` ✓
  - `pub use bar;` ✓
  - `    pub fn bar()` ✗ (indented; not captured)
  - `// pub fn commented()` ✗ (not at line start)
- This is intentional — baseline should only contain module-level public items

### Version pin: cargo-public-api 0.50.1
- Tested on Rust stable 1.92.0
- No nightly features required
- Pinned via `--locked` flag to ensure deterministic installs
- Verify locally: `cargo install cargo-public-api --locked --version 0.50.1`

### CI label trigger: `ci:public-api`
- Mirrors existing `ci:semver` pattern
- Not a required label for merge (optional trigger)
- If builder forgets to document this, PRs won't trigger the check on label — but nightly schedule will still catch drifts
- Plan-reviewer verified this in the acceptance criteria

---

## Confidence and Risks

### HIGH confidence areas
- Root cause verified (no hard-fail gate exists)
- Command correctness verified (capture-and-diff pattern tested locally)
- Baseline size estimates measured (`perl-dap` with/without `-s`)
- Tool stability confirmed (cargo-public-api 0.50.1 on stable 1.92.0)

### MEDIUM confidence areas
- Perllsp baseline at 2 lines (documented as expected, but unusual)
- Integration with nightly workflow timing (builder may need to adjust timeout if baselines are large)

### LOW risks
- Facade churn during Waves G1-G3 (design of collapse prevents this — facades don't change during satellite refactors)
- Baseline drift post-v0.13.0 (only happens if facades are extended; then update is intentional and documented)

---

## Retrospective Notes

### What went right
- Plan-reviewer caught the `cargo public-api diff` subcommand error immediately by testing locally
- Accurate baseline size measurements (perl-dap 10,246 → 3,184 lines with `-s`) prevented spec underestimation
- Architecture-reviewer confirmed the 5-crate scope is aligned with facade/core split (Waves D/F) — will remain stable through G-waves

### What could improve
- Scout should have tested the tool locally before filing the spec (would have caught the diff command error)
- The inconsistency between `justfile:semver-check-all` (5 crates) and CI job (3 crates) should have been flagged earlier (found in architecture-review, could have been caught in initial scout)

### For future CI guardrails
- Always test tool commands locally before finalizing spec (run `tool --help`, test subcommands)
- Check for feature flag or library API changes that could affect diff output (e.g., `--simplified` stability across versions)
- Verify baseline idempotency: run capture twice, diff the files — they must be identical

---

## Files Modified

| File | Change | Reason |
|------|--------|--------|
| `.github/workflows/ci-nightly.yml` | Add `public-api-check:` job (lines 346+); add 2 semver steps to existing job (lines 330+) | New hard-fail gate; sync CI with justfile crate coverage |
| `justfile` | Add 3 recipes: `_public-api-install`, `public-api-check`, `public-api-update` (lines 1796+) | Local dev commands |
| `CONTRIBUTING.md` | Add "### Public API Surface Ratchet" subsection (line 303+) | Onboarding for facade API changes |
| `.ci/public-api-baselines/` | Create directory + 5 `.txt` files | Baseline storage |
| `.spec/4497-public-api-ratchet/` | Create spec files: checklist.md, acceptance.md, context.md | Implementation planning |

---

## No-Touch Zones

**Builders must NOT modify:**
- Any `crates/*/src/` files
- Any `Cargo.toml` files
- `xtask/` directory or generated files
- `features.toml`
- Other CI workflows (`.github/workflows/*.yml` except ci-nightly.yml)
- Test code (tests are spec-planner's job, not builder's — unless spec explicitly calls for new tests)

---

**End of Context**

Spec written: 2026-04-18
Plan-reviewer: EffortlessSteven
Builder: TBD
Status: Ready for implementation branch creation and red-TDD
