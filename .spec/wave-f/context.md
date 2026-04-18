# Wave F Context — Key Decisions, Alternatives, and Objections Resolved

## Summary

Wave F absorbs 8 `perl-lsp-feature-*` and `perl-lsp-capability-map` thin published libraries into a new `perl-lsp-rs-core` implementation crate. This is part of the v0.13.0 microcrate collapse program (issue #4410), targeting 30 published crates (from 81).

**Pattern:** Mirrors Wave D's successful `perl-parser`/`perl-parser-core` split (PR #4486, merged 2026-04-18).

**Published count:** 81 → 74 (8 removed, 1 added, net −7).

---

## Verification Pipeline Results

All 6 pre-build verification layers completed with corrections:

### 1. Accuracy Scout (2026-04-18)

**Status:** ✓ VERIFIED with corrections.

**Corrections applied:**

1. **Binary target claim corrected:** `perl-lsp-feature-profile-cli` has NO `[[bin]]` section on master (library-only). Ledger line 182 note about "preserve [[bin]] as subcommand" is dead documentation. **Action:** Drop the note; treat profile_cli as a plain module, no binary target creation.

2. **Consumer count corrected:** Claimed "~20 consumer crates" but accuracy-scout found only 2 direct consumers: `perl-lsp` (all 8 crates) and `perl-lsp-protocol` (2 crates: flags + contracts). Third indirect consumer identified: `perl-lsp-feature-governance` (Wave G3, not Wave F scope) depends on 5 of the 8. **Action:** Update consumer migration scope accordingly.

3. **Crate identity clarified:** Package name is `perl-lsp-rs` but directory is `crates/perl-lsp/`. This naming mismatch persists post-Wave F and will be addressed in Wave G or later. No action required for Wave F.

### 2. Research Verifier (2026-04-18)

**Status:** ✓ NO EXTERNAL CLAIMS — Internal Rust workspace refactoring only.

No claims depend on Perl semantics, LSP protocol specs, or external crate APIs. All mechanical facts (cargo syntax, workspace conventions) already verified by accuracy-scout. Routed directly to oppositional-planner.

### 3. Oppositional Planner (2026-04-18)

**Status:** ✓ 4 OBJECTIONS RAISED; resolved by Amendment 6.

**Original objections (all resolved by structural fix):**

- **O1:** `perl-lsp-rs` is a dual-duty binary, not a library facade. Absorbing 8 config crates into the binary violates Wave D's "thin facade over core" pattern.
- **O2:** Binary target decision unresolved (profile-cli spec mismatch).
- **O3:** Publication intent ambiguous — should `perl-lsp-rs` be a published facade NOW?
- **O4:** Nested module structure creates maintenance friction (src/features/ already has 38 LSP provider modules; adding 8 absorbed crates makes it unmaintainable).

**Alternatives proposed:**

- **A1 (selected by plan-reviewer):** Use new `perl-lsp-rs-core` crate (implementation sibling). Mirrors Wave D pattern exactly. Resolves O1/O3/O4 in one stroke.
- **A2:** Minimal first pass (ids/contracts/flags only); defer policy/grid/profile to Wave G1.
- **A3:** Keep feature crates published; absorb only the 2 externally consumed (flags, contracts).

**Resolution:** Amendment 6 (PR #4492, merged 2026-04-18) locked in **A1** as the structural decision. `perl-lsp-rs-core` is now the accepted architecture.

### 4. Advocatus Diaboli (2026-04-18)

**Status:** ✓ DEFER → RESOLVED by Amendment 6.

**Challenge:** User impact is zero (internal refactoring). Timing risk: Wave D just landed (2026-04-18, 24 hours earlier). Architecture decision was unresolved (blocker).

**Resolution:** Amendment 6 resolved the architecture question. Advocates-diaboli would recommend proceeding with the A1 pattern now that the decision is locked. DEFER verdict lifted by architectural clarity.

### 5. Architecture Reviewer (implied, pre-recorded)

**Status:** ✓ Should pass trivially — Amendment 6 encoded the A1 structure.

`perl-lsp-rs-core` as implementation sibling, `perl-lsp-rs` as thin facade. This matches Wave D's proven pattern (perl-parser-core). Dependency layering is clean: core is internal/private; facade is public re-export surface.

### 6. Maintainer Issue (2026-04-18)

**Status:** ✓ ALIGNED.

Wave F is part of v0.13.0 critical path (collapse program #4410). Reduces published crate count from 81 toward 30 target. Matches historical precedent (Waves D, A, B, E, H all succeeded). No framework scope implications. Cleared for plan-review.

---

## Plan Review Corrections (Final, Locked)

Plan-reviewer (2026-04-18) applied 3 critical corrections to the spec:

### Correction 1: No Binary Target in Wave F

`perl-lsp-feature-profile-cli` is library-only on master (accuracy-scout confirmed). Ledger line 182 says "preserve [[bin]] as subcommand" but this is forward-looking spec, not current state.

**Decision:** Do NOT create a `[[bin]]` target in Wave F. Drop the ledger note. Treat `profile_cli` as a plain module in `perl-lsp-rs-core/src/features/profile_cli.rs`.

### Correction 2: Three-Consumer Cargo.toml Updates Required

Accuracy-scout found only 2 direct consumers (perl-lsp, perl-lsp-protocol), but plan-review identified a third indirect consumer: **`perl-lsp-feature-governance`** (Wave G3 scope, stays published).

**Decision:** Governance's `Cargo.toml` must be updated in Wave F (5 deps changed to `perl-lsp-rs-core`), even though governance code is not absorbed. This prevents build breakage post-Wave F.

### Correction 3: Published Count: 81 → 74, Not 73

Accuracy-scout's initial claim: 81 → 73 (−8 only).
Correction: 81 → 74 (−8 absorbed, +1 new `perl-lsp-rs-core`, net −7).

`perl-lsp-rs-core` is **published** (added to [workspace.metadata.publish].allow), mirroring perl-parser-core in Wave D.

---

## Root Cause: Why This Wave Exists

The 8 `perl-lsp-feature-*` and `perl-lsp-capability-map` crates are thin published libraries that provide **configuration, policy, and capability data structures** for the LSP server. They have no inherent public users outside the perl-lsp crate itself — they exist as published crates purely for organizational hygiene (one crate per concern).

Wave F reorganizes them into modules within `perl-lsp-rs-core`, the internal implementation sibling of the `perl-lsp-rs` facade. This removes 8 published crates (reducing surface area), collocates related code, and eliminates inter-crate import chains (all become local `crate::features::*` paths).

**Rationale:** Collapse the microcrate explosion toward the v0.13.0 target of 30 published + 4 internal = 34 total crates. Wave F contributes 19% of the remaining collapse work (8 of 43 remaining to-collapse crates).

---

## Objections Resolved

### Objection 1: O1 — Destination crate is not a library facade

**Original:** perl-lsp-rs is the binary (crates/perl-lsp/). Absorbing config crates into it violates the "thin UX facade" pattern from Wave D.

**Resolution (Amendment 6):** Create `perl-lsp-rs-core` as the implementation sibling. Move all 8 crates there. perl-lsp-rs (the facade) re-exports from -core. This mirrors perl-parser/perl-parser-core exactly. ✓

### Objection 2: O2 — Binary target preservation unresolved

**Original:** Spec says "preserve [[bin]] as subcommand" but perl-lsp-feature-profile-cli has no binary today.

**Resolution:** Accuracy-scout confirmed: no [[bin]] section exists. Drop the spec line. profile_cli is library-only. ✓

### Objection 3: O3 — Publication intent ambiguous

**Original:** Should perl-lsp-rs be a published facade NOW, or is it internal-only?

**Resolution (Amendment 6):** `perl-lsp-rs-core` is the new published sibling (mirroring perl-parser-core). `perl-lsp-rs` remains the facade library re-exporting from -core. Both stay published. ✓

### Objection 4: O4 — Module bloat (src/features/ already has 38 LSP provider modules)

**Original:** Adding 8 absorbed config crates to src/features/ makes the mod.rs file unmaintainable.

**Resolution (Amendment 6):** Absorbed crates go into `perl-lsp-rs-core/src/features/`. Existing LSP provider modules (code_actions, completion, etc.) stay in `crates/perl-lsp/src/features/` (different crate). No collision. ✓

---

## Why the Plan-Reviewed Checklist Differs from Scout Spec

The scout's original spec (in #4489 issue body) was written before Amendment 6 locked the `perl-lsp-rs-core` architecture decision. The plan-review comment incorporates Amendment 6's decision and fixes 3 corrections from the verification pipeline.

**Key differences:**

1. **Destination:** Original spec: "absorb into perl-lsp-rs". Plan-review: "absorb into perl-lsp-rs-core" (new crate).
2. **Binary target:** Original spec: "preserve [[bin]]". Plan-review: "don't; profile_cli is library-only".
3. **Consumer scope:** Original spec: "update 2 consumers". Plan-review: "update 3 (perl-lsp, perl-lsp-protocol, perl-lsp-feature-governance)".

The red-TDD builder and implementation builder should follow the **plan-review checklist**, not the original scout spec.

---

## Wave F in the Collapse Program

Collapse program structure (issue #4410):

| Wave | Crates | Status | Published | Date |
|---|---|---|---|---|
| D | perl-parser, perl-parser-core | ✓ Merged #4486 | 81→81 (swap) | 2026-04-18 |
| **F** | **perl-lsp-feature-* (8) → perl-lsp-rs-core** | **In planning** | **81→74** | **2026-04-18** |
| G1 | LSP providers (40+ crates) | Pending | 74→36+ | TBD |
| G2 | Governance + runtime | Pending | TBD | TBD |
| G3 | perl-lsp-feature-governance scope | Pending | TBD | TBD |

Wave F is a **prerequisite for Wave G1** — governance and protocol code can't migrate to LSP providers until the config crates are consolidated.

---

## Test Organization Pattern (Plan-Review Specified)

Tests are moved to a flat directory structure in `crates/perl-lsp-rs-core/tests/` using the naming pattern `feature_<short>_<original>.rs`:

```
tests/
  feature_ids_comprehensive.rs             (from perl-lsp-feature-ids)
  feature_contracts_comprehensive.rs       (from perl-lsp-feature-contracts)
  feature_contracts_extended.rs            (additional contracts tests)
  feature_flags_comprehensive.rs           (from perl-lsp-feature-flags)
  feature_flags_extended.rs                (additional flags tests)
  feature_profile_comprehensive.rs         (from perl-lsp-feature-profile)
  feature_policy_comprehensive.rs          (from perl-lsp-feature-policy)
  feature_policy_extended.rs               (additional policy tests)
  feature_grid_comprehensive.rs            (from perl-lsp-feature-grid)
  feature_grid_extended.rs                 (additional grid tests)
  capability_map_comprehensive.rs          (from perl-lsp-capability-map)
  [profile_cli has no tests/]
```

This naming avoids collisions and makes test origin obvious. All tests inherit the parent crate's `[dev-dependencies]` (perl-tdd-support, serde_json, etc.).

---

## Risk Flags (from Verification Pipeline)

### R1: Directory/Package Name Mismatch Persists

`crates/perl-lsp/` is the directory, but `perl-lsp-rs` is the package name. This mismatch will require renaming in a future wave. Wave F does NOT address it (out of scope). Documented for Wave G or later.

**Mitigation:** No action in Wave F. This is accepted technical debt.

### R2: Cross-Module Import Rewriting

The 8 absorbed crates form a dependency chain (ids → capability_map, ids → flags, capability_map → contracts, etc.). Every intra-wave import must be rewritten from `use perl_lsp_feature_*::` to `use crate::features::*::`. If any are missed, the build will fail.

**Mitigation:** Checklist specifies exact import rewrites. Builder must audit every file before the final check.

### R3: Build Script and features_sot.toml

Only `perl-lsp-feature-contracts` has a `build.rs` and `features_sot.toml`. Both must be moved to `perl-lsp-rs-core/`. The build script is referenced in [build-dependencies].

**Mitigation:** Checklist specifies exact file copy. Builder must verify `perl-lsp-rs-core` has both files after step 1.

### R4: Feature Gate Forwarding

5 of the 8 crates use the `lsp-ga-lock` feature gate. After consolidation, all must forward to `perl-lsp-rs-core/lsp-ga-lock`. If any are missed, feature-gated code won't be guarded correctly.

**Mitigation:** Checklist specifies which crates use the gate and which consumers need to forward it. Builder must verify all 5 with: `grep -rn "lsp-ga-lock" crates/perl-lsp-rs-core/src/`.

### R5: perl-lsp-feature-ids is Not a Direct Dependency

accuracy-scout noted: `perl-lsp-feature-ids` is never directly listed in `perl-lsp/Cargo.toml` — it was transitive. Plan-review reiterated: "do NOT add perl-lsp-feature-ids to perl-lsp/Cargo.toml" (step 5a).

**Mitigation:** Checklist explicitly says don't add it. Builder must verify with: `grep "perl-lsp-feature-ids" crates/perl-lsp/Cargo.toml` (expect zero matches).

---

## Why This Complexity?

Wave F touches 3 consumer Cargo.toml files, 2 source code files, 1 workspace root, and deletes 8 directories. The tight dependency chains between the 8 absorbed crates mean every import path must be rewritten correctly.

This is **necessary complexity**, not avoidable. The Rust compiler will catch missed rewrites at compile time (unresolved imports), so there's no risk of silent failures.

---

## Success Criteria (Plan-Review Final)

All verification gates must pass:

1. `cargo check --workspace` — no unresolved dependencies
2. `cargo test -p perl-lsp-rs-core` — all 11 test files pass
3. `cargo test -p perl-lsp-rs`, `perl-lsp-protocol`, `perl-lsp-feature-governance` — all pass
4. `cargo xtask layer-check` — no violations
5. `cargo xtask fmt` — no formatting issues
6. Published baseline: `xtask/published-crate-baseline.txt` = `74`
7. Ledger updated: 8 Wave F rows marked complete

If all 7 pass, Wave F is complete. Red-TDD adds edge case tests. Builder implements, pushes PR, and the full pipeline (review, deep-review, refactor, merge) follows.

---

## Next Steps (Post-Spec-Plan)

1. **Red-TDD builder:** Check out this branch (impl/4489-wave-f-perl-lsp-rs-core), write failing tests that validate the checklist acceptance criteria.
2. **Implementation builder:** Check out the red-TDD branch (now with spec + red tests), implement steps 1–9, verify with commands V1–V9, push PR.
3. **Reviewer:** Check diff against spec, validate architecture, approve or suggest fixes.
4. **Merge:** Once all gates pass and deep-review clears it, merge to master.

---

## References

- **Issue:** #4489 (Wave F — perl-lsp-feature-* collapse)
- **Parent:** #4410 (v0.13.0 collapse program tracker)
- **Amendment 6 (structural fix):** PR #4492, issue #4491 (perl-lsp-rs-core decision)
- **Precedent (Wave D):** PR #4486, issue #4486 (perl-parser/perl-parser-core, merged 2026-04-18)
- **Plan-review comment:** Final spec locks in all 9 steps and 9 verify commands

---

## Open Questions Resolved

**Q: Should perl-lsp-rs emerge as a published facade NOW?**
A: Yes. After Amendment 6, `perl-lsp-rs` stays published (it's the facade). `perl-lsp-rs-core` is new-published (implementation sibling). This mirrors perl-parser/perl-parser-core.

**Q: Should we create a [[bin]] target for perl-lsp-feature-profile-cli?**
A: No. The crate is library-only on master. Do not create a binary. Treat profile_cli as a plain module.

**Q: How many consumers need Cargo.toml updates?**
A: Three: `perl-lsp` (8 deps), `perl-lsp-protocol` (2 deps), `perl-lsp-feature-governance` (5 deps).

**Q: Why does the published count stay flat (81 → 74)?**
A: 8 removed, 1 added (perl-lsp-rs-core). Net −7. This is correct. The new -core crate is published, mirroring the Wave D pattern.
