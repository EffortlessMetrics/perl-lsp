# Context & Decision Log — Wave G1a Collapse (Issue #4500)

This file documents key decisions, objections raised, alternatives considered, and how they were resolved. It provides historical context for future maintainers and future spec updates.

---

## What is G1a?

Wave G1a collapses 15 low-risk LSP provider crates into a single `perl_lsp_rs_core::providers::*` module namespace. This is a pure structural refactoring — no feature additions, no protocol changes, no behavior changes. Users see zero impact.

The 15 crates are:
1. `perl-lsp-completion-item` (helper utility)
2. `perl-lsp-file-completion` (depends on completion-item)
3. `perl-lsp-code-lens`
4. `perl-lsp-document-highlight`
5. `perl-lsp-folding`
6. `perl-lsp-selection-range`
7. `perl-lsp-inlay-hints`
8. `perl-lsp-type-hierarchy`
9. `perl-lsp-formatting-types`
10. `perl-lsp-on-type-formatting`
11. `perl-lsp-color-provider`
12. `perl-lsp-symbol-query` (helper utility)
13. `perl-lsp-import-management`
14. `perl-lsp-document-links`
15. `perl-lsp-workspace-symbols` (depends on symbol-query)

**Impact:** Reduces published crate count from 74 → 59 (−15).

**Why:** Supporting v0.13.0 critical path to "public alpha announcement" by reducing dependency-graph complexity and semver tax. The microcrate collapse goal is 135 → 31 published crates. Wave F achieved 135 → 74. G1a achieves 74 → 59. G1b will further reduce this.

---

## Objections Raised & Resolutions

### O1 — Test Migration Complexity (Oppositional Planner)

**Objection:**
> "G1a contains 6549 lines of source across 15 crates with mixed test layouts (8 crates use `tests/` dirs, 7 use inline `#[cfg(test)]`). The spec says 'test module paths change' but doesn't detail how. After collapse, test discovery will break if the builder treats all test files as one monolithic unit."

**Risk:** Silent test loss. 75+ test files scattered across 15 crates. No consistent naming. Collision detection is manual.

**Plan-Reviewer Resolution:**
- Enumerated all 20 test files (not 75; some crates have inline tests only, not separate `tests/` dirs).
- Adopted prefix naming to resolve collisions: `provider_CRATE_DESCRIPTOR.rs` (e.g., `provider_completion_item_dedup_sort.rs`).
- Inline `#[cfg(test)]` blocks stay in place — they move with source modules automatically.
- Added test count baseline verification (Step 0.1 and Step 9.2 of checklist).
- **Mitigation:** Explicit file mapping in checklist APPENDIX B with all 20 files listed. Builder copies each file, updates imports, runs `cargo test` after each group.

---

### O2 — Intra-G1a Dependency Sequencing (Oppositional Planner)

**Objection:**
> "Two dependency pairs exist: completion-item→file-completion and symbol-query→workspace-symbols. Architecture-reviewer said 'place helper modules first, consumers second,' but the spec's step-by-step collapse doesn't enforce this. If the builder processes crates in the issue's table order (completion-item #1, file-completion #2, symbol-query #12, workspace-symbols #15), compilation will fail."

**Risk:** Builder tries to collapse `file-completion` before `completion-item` is visible as a submodule, gets forward-reference errors or worse.

**Plan-Reviewer Resolution:**
- Explicitly grouped collapse into 3 ordered phases (PART 2, PART 3, PART 4 of checklist).
- **Group 1 (helpers):** `completion-item`, `symbol-query` collapsed first. Verification gate at end of Group 1.
- **Group 2 (consumers):** `file-completion`, `workspace-symbols` collapsed after Group 1 visible. Intra-module imports rewritten to `crate::providers::HELPER::`.
- **Group 3 (independents):** 11 crates with no inter-dependencies, collapsed in any order.
- **Mitigation:** Explicit ordering enforced by checklist sections. Builder cannot skip to Group 3 without completing Groups 1–2.

---

### O3 — wired_crates_integration_test.rs Brittleness (Oppositional Planner)

**Objection:**
> "This 364-line test file has 19 direct `use perl_lsp_*` imports. After collapse, every import must change. The file is a mechanically-generated artifact of 'which crates are wired,' not hand-edited logic. Spec says manually patch it. Risk: typo in one import, provider silently unwired."

**Risk:** Manual patching error → provider registration fails at runtime post-merge.

**Plan-Reviewer Resolution:**
- Investigated the file and found only **6 of 15 G1a crates** have import lines in it (not all 19).
- Provided explicit diff table (PART 6, Step 6.1 of checklist) with exact line matches and replacements.
- Added post-patch verification gate: `grep -c "perl_lsp_CRATE"` must return `0`, followed by running the test.
- **Mitigation:** Not auto-generated (not worth tooling for 6 lines), but heavily validated. Explicit table + verification gate replaces manual patching uncertainty.

---

### O4 — Wave F Soak Overlap (Oppositional Planner)

**Objection:**
> "Wave F merged ~46h ago. Parallel collapses (Waves A/E/H) are in-build simultaneously. If any merge during G1a's soak window, G1a inherits unstable snapshots. Risk: snap conflicts, test fixture debt."

**Risk:** G1a's test suite inherits snapshot drift from concurrent work.

**Plan-Reviewer Resolution:**
- This is an **ops scheduling problem**, not a codebase-readiness problem.
- G1a's implementation checklist is independent of soak window.
- Mitigation: **Builder should monitor master during 48–72h soak. If Waves A/E/H merge, rebase G1a branch and re-run `cargo check --workspace` before opening PR.** (Added as risk flag R3 in checklist.)
- Deferral was conditional on plan-review hardening O1/O2/O3, not on soak completion. Plan-review completed 2026-04-19 (2 days post-Wave-F); G1a can be built now with awareness of O4.

---

## Alternatives Considered

### A1 — Split G1a Further (Helper-First Two-PR Approach)

**Proposal (from Oppositional Planner):**
> Split G1a into G1a.1 (13 independents + 2 helpers) and G1a.2 (2 consumers). Collapse G1a.1, verify tests, then G1a.2.

**Pros:**
- Safer sequencing: helpers proven visible before consumers depend on them.
- Easier for junior builders: split the risk.

**Cons:**
- Two separate PRs → longer closure time (1–2 days extra).
- Delays G1b unblocking (G1b blocked on G1a merge).
- Adds ops scheduling overhead (two separate merges, two separate CI gates).

**Decision:** REJECTED. Plan-reviewer's explicit Group 1 → Group 2 → Group 3 sequencing in a single PR is safer with clear gates. Splitting is over-engineering for low-risk helper dependencies.

---

### A2 — Monolithic Module with Feature Flags

**Proposal (from Oppositional Planner):**
> Instead of 15 submodules, create `crates/perl-lsp-rs-core/src/providers.rs` (single file, not directory) with conditional compilation:
> ```rust
> #[cfg(feature = "provider-completion-item")]
> pub mod completion_item { /* 15 collapsed crates */ }
> ```

**Pros:**
- Test discovery trivial (all in one file/module).
- Can enable providers selectively.

**Cons:**
- Violates microcrate philosophy (goal is expanding published surface, not reducing it).
- Feature flags add maintenance burden.
- Doesn't prepare for eventual `perl-lsp-rs` facade re-exports (Wave F pattern requires public submodule boundaries).
- Precedent violation: Wave F used 15 submodules for capability crates, not monolithic.

**Decision:** REJECTED. G1a uses the Wave F pattern (submodules under `providers/`) consistently.

---

## Verification Chain & Sign-Offs

All six verification stages completed before plan-review:

1. **Accuracy-scout** (2026-04-18):
   - Verified all 15 crate paths exist.
   - Corrected the issue's claim "no inter-provider deps" → Found 2 internal dependency pairs.
   - Status: CORRECTED

2. **Research-verifier** (2026-04-18):
   - No new external claims (Perl, LSP spec, CPAN, crate API).
   - Inherits parent #4496's research verdict.
   - Status: VERIFIED

3. **Architecture-reviewer** (2026-04-18):
   - No upward layer violations.
   - G1a crates are leaf providers, correct dependency direction.
   - Module hierarchy (Group 1 → Group 2 via submodule visibility) handles internal deps cleanly.
   - Status: ALIGNED

4. **Oppositional-planner** (2026-04-18):
   - Challenged approach on 4 execution risks (O1–O4).
   - Verdict: QUESTIONABLE on **implementation spec**, SOUND on approach.
   - Identified 2 alternatives (A1, A2); both rejected.
   - Flagged 3 risk flags (R1, R2, R3).
   - Status: CHALLENGES FILED

5. **Advocatus diaboli** (2026-04-18):
   - Verdict: DEFER 24–48h for plan-review spec-hardening on O1/O2/O3.
   - Execution risks identified but fixable.
   - Approach is sound.
   - Status: DEFER (conditional BUILD after plan-review)

6. **Maintainer vision** (2026-04-18):
   - G1a advances critical v0.13.0 goal (135 → 31 published crates).
   - Zero user impact (pure refactoring).
   - Scope fits project plumbing, not feature creep.
   - Status: ALIGNED

7. **Plan-reviewer** (2026-04-19):
   - Resolved O1 with test enumeration + prefix naming + baseline verification.
   - Resolved O2 with explicit 3-group sequencing + verification gates.
   - Resolved O3 with exact diff table + post-patch grep gate.
   - Addressed R1, R2, R3 in checklist.
   - Status: READY FOR BUILDER

---

## Key Insights for Future G1b/G1c

1. **Test migration at scale:** 20 files with prefix naming is manageable. Future waves with 30+ test files might benefit from a shell script to auto-generate the manifest.

2. **Intra-module dependencies:** Module hierarchy (submodules) naturally handles helper → consumer visibility. No shared `providers::shared` submodule needed. This pattern works well.

3. **Registry file pattern:** `wired_crates_integration_test.rs` is a manually-maintained registry. For larger waves, a `cargo xtask` command to auto-enumerate submodules and generate import statements would reduce typo risk.

4. **Soak window ops:** Wave F's 48–72h soak exposed that parallel merges (Waves A/E/H) can introduce test fixture debt. Future waves should clarify in the spec: "If parallel work merges during soak, rebase and verify CI gates independently."

---

## Builder Notes

**You are the builder.** This checklist is for you. Here's what matters:

1. **Work in order.** Groups 1 → 2 → 3. The gates are there for a reason.
2. **Verify after each part.** `cargo check` and `cargo test` are your friends. Don't skip.
3. **Test file naming:** Use the prefix pattern `provider_MODULE_DESCRIPTOR.rs` consistently. It prevents collisions and makes them easy to find.
4. **Intra-module imports:** `crate::providers::HELPER::` is the correct syntax when a consumer submodule imports from a helper submodule.
5. **The wired_crates file:** Exactly 6 imports to change. Use the diff table. Run the grep gate after patching.
6. **Consumer crates are your canary.** If consumer imports break compilation, you've missed a public re-export or the import path is wrong.

**What could go wrong:**
- Test file collision (if you don't use the prefix pattern).
- Forward-reference error (if you collapse file-completion before completion-item is visible).
- Typo in wired_crates (grep gate catches this).
- Silent test loss (baseline count verification catches this).
- Forgotten import site in a consumer crate (cargo check catches this).

**Red-TDD should write tests for:**
- All 15 submodules compile.
- All 20 test files import correctly.
- All 6 consumer crates import from `perl_lsp_rs_core::providers::*` successfully.
- `wired_crates_integration_test.rs` compiles and runs with new imports.
- Test count ≥ baseline (no silent test loss).

---

## Decision Summary

| Decision | Status | Rationale |
|----------|--------|-----------|
| Collapse into `perl_lsp_rs_core::providers` submodules (15 separate) | APPROVED | Wave F pattern, scalable, matches facade/core split |
| Group 1 → Group 2 → Group 3 sequencing | APPROVED | Respects intra-module dependencies, gates prevent forward-ref errors |
| Prefix naming for test files: `provider_MODULE_DESCRIPTOR.rs` | APPROVED | Avoids collisions, clear naming, easy discovery |
| Migrate inline tests with source, no separate move needed | APPROVED | Simplifies test discovery, reduces manual work |
| Manual patch of `wired_crates_integration_test.rs` with diff table | APPROVED | Only 6 imports, explicit diff table + grep gate adequate |
| No split into G1a.1/G1a.2 PRs | APPROVED | Single PR with clear gates safer than parallel PRs |
| No monolithic module with feature flags | APPROVED | Violates microcrate philosophy, precedent against it |
| Monitor parallel waves during soak, rebase if needed | APPROVED | Ops concern, not implementation blocker; handled by builder awareness |

---

## Glossary

- **G1a:** Wave 1a of the second microcrate collapse batch (first batch = Waves A–F). Contains 15 provider crates.
- **G1b:** Wave 1b (blocked on G1a). Contains 10+ remaining LSP and diagnostic crates.
- **Helper:** A submodule that other submodules depend on (e.g., `completion_item`, `symbol_query`).
- **Consumer:** A submodule that depends on helpers (e.g., `file_completion`, `workspace_symbols`).
- **Independent:** A submodule with no inter-provider dependencies (11 in Group 3).
- **Test discovery:** Cargo's ability to find and run test files/modules. Mixed layouts (inline + `tests/` dir) complicate this.
- **Intra-module import:** An import within the same parent module (e.g., one submodule importing from sibling submodule using `crate::providers::`).
- **Wired:** Registered and available for LSP protocol execution (providers are "wired" in the LSP server's provider registry).

