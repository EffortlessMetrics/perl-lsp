# Context and Decision Rationale: Issue #4513

This document captures the key decisions, alternatives considered, and objections resolved during the planning of issue #4513 (Red-TDD API-Read Prompt Update).

---

## Problem Summary

Red-TDD agents write tests against **inferred** API shapes rather than **actual** code. This creates a predictable pattern: the builder receives a branch with failing tests, implements the feature, and then "fixes" multiple tests because they were testing against the wrong constructor signatures, missing trait bounds, or incorrect field types.

**Evidence trail:**
- **Wave G1a (PR #4506):** 3 red-TDD API-shape fixes by builder
- **Wave G1b (PR #4510):** 6 red-TDD API-shape fixes by builder
- **Trajectory:** Roughly doubling per wave (G2 projected ~12, G3 ~24)

Each fix was documented with `NOTE(<wave>-API-fix)` or equivalent to mark them as signature corrections, not semantic drift (verified by deep-reviewer). The pattern signals a process gap that compounds over time.

---

## Root Cause

Red-TDD reads the **specification** (which describes what should exist in abstract terms) rather than the **actual codebase** (which has concrete `pub struct`/`pub fn`/`pub use` declarations).

When an issue specifies crate absorption (e.g., "move symbols from perl-foo-bar into a new module under perl-lsp-rs-core"), the spec says:
- "Module X absorbs crate Y"
- "Constructor Foo should be callable as `core::Foo::new()`"

But the spec often omits:
- Whether `Foo::new()` takes arguments
- Whether `Foo` implements `Default`
- Whether `Foo` is `Clone` or `Send + Sync`
- The exact field types (`Option<_>` vs. plain value, `Vec<T>` vs. `&[T]`)
- What's `pub use`'d from sub-modules vs. what requires deep import paths

Red-TDD then assumes idiomatic Rust defaults (every type has `Default::default()`, constructors take no args, field names are inferred from context). When those assumptions don't match the actual code, the builder has to fix the test.

---

## Solution: Two Complementary Fixes

### Fix A — Update Red-TDD Prompt (THIS ISSUE)

**Scope:** Update three files in `.claude/` to add an explicit API-read step before red-TDD writes any test.

**Files:**
1. `.claude/commands/red-tdd-read.md` — Add Step 5 describing how to read an absorbed crate's actual API
2. `.claude/commands/red-tdd-write.md` — Add a guard note and signature-accuracy reminder
3. `.claude/agents/red-tdd.md` — Add a principle bullet about absorption API-reads

**Approach:**
- Red-TDD reads the actual `src/lib.rs` and `pub use` chains before writing tests
- Red-TDD follows a graduated fallback: check if source exists → read source → read destination if already absorbed → record exact signatures → use `// TODO: signature unclear` if unlocatable
- Tests are written against **actual** signatures, not inferred ones

**Risk profile:** LOW — it's a process change (no code changes). The instruction is explicit and handles edge cases (post-collapse crate absence, non-absorption scoping, fallback for missing signatures).

**ROI:** HIGH — if G2 shows ≤2 API-shape fixes (vs. G1b's 6), the process change alone solved the problem.

---

### Fix B — Spec-Planner API Enumeration (DEFERRED)

**Scope:** NOT included in this issue. Deferred pending G2 measurement (see diaboli verdict below).

**Original proposal:**
- Require spec-planner to include a "Public API surfaces" section in `context.md`
- Enumerate each absorbed crate's constructors, trait implementations, type shapes
- Give red-TDD one consolidated reference instead of multiple source trees

**Why deferred:**
1. **Maintenance burden:** Spec-planner files are currently thin and maintainable. Adding API enumeration creates hand-maintained documentation that can rot if the implementation changes.
2. **Measurement gate:** If Fix A alone reduces API-shape fixes to ≤2 (G2 target), Fix B is unnecessary. If Fix A isn't enough (≥4 `NOTE(G2-API-fix)` comments), then Fix B becomes compelling.
3. **Diaboli verdict:** Advocatus-diaboli explicitly said BUILD-AT-REDUCED-SCOPE (Fix A only). (See issue comments for full objections.)

**Follow-up:**
If G2 red-TDD commit contains ≥4 `NOTE(G2-API-fix)` comments, open a follow-up issue referencing:
- oppositional-planner alternative A2: rustdoc-JSON auto-generation (tooling to extract API shapes)
- oppositional-planner alternative A3: skip red-TDD for absorption work (change the pipeline, not the prompt)

---

## Verification Pipeline (Before This Issue)

This issue went through the full verification pipeline:

1. **Accuracy-scout:** REVIEWED
   - Verified file paths (.claude/commands/red-tdd-*.md, .claude/agents/red-tdd.md exist)
   - Verified issue history (G1a 3 fixes, G1b 6 fixes documented)
   - CLEAN

2. **Research-verifier:** REVIEWED
   - No external claims to verify (Perl, LSP spec, crate APIs)
   - VERIFIED_CLEAN

3. **Oppositional-planner:** REVIEWED
   - Objection O1: Root cause diagnosis may be incomplete (alternative: spec is intentionally abstract)
   - Objection O2: Red-TDD may not have cognitive capacity for API-read + test-write in one pass
   - Objection O3: Fix B's API enumeration will rot silently in `context.md`
   - Alternative A1: Tooling (rustdoc-JSON extraction)
   - Alternative A2: Skip red-TDD for absorption work entirely
   - Verdict: Objections are real but Fix A addresses O1/O2; O3 triggers deferral of Fix B

4. **Architecture-reviewer:** REVIEWED
   - Verified no crate-boundary violations
   - Verified skill dependency graph (red-TDD → builder flow respects layer contracts)
   - COMPLIANT

5. **Advocatus-diaboli:** REVIEWED
   - Pattern is real (G1a=3, G1b=6 verified)
   - Pattern is growing (doubling trajectory)
   - **Verdict: BUILD-AT-REDUCED-SCOPE**
   - Fix A is worth doing (low risk, measurable impact)
   - Fix B is worth deferring (no evidence it's needed yet, adds maintenance)

6. **Maintainer-issue:** REVIEWED
   - Aligns with v0.13.0 microcrate collapse goals (reducing API-shape churn = faster builds)
   - Part of broader "red-TDD quality" theme from G1 waves
   - ALIGNED

7. **Plan-review:** COMPLETED (Sonnet)
   - Refined spec to exact 3 files, exact locations
   - Stress-tested wording (handles post-collapse absence, non-absorption scoping, fallback for missing signatures)
   - Specified exact acceptance criteria
   - Verdict: READY FOR BUILDER

---

## Stress-Test Findings (From Plan-Review)

These edge cases were found during spec refinement and are **explicitly handled** in the checklist:

1. **Non-absorption scoping:** The instruction in red-tdd-read.md Step 5 is gated on "issues involving crate absorption or module refactoring" so it doesn't fire on unrelated work (parser bugs, feature adds). ✓

2. **Post-collapse crate absence:** Step 5c explicitly handles "source crate has already been absorbed (file not found — prior wave merged it)"; agent reads destination module instead. ✓

3. **Re-export chain traversal:** Step 5b says "follow any `pub use` chains into sub-modules" so agent doesn't stop at a bare re-export in lib.rs. ✓

4. **Blocking on missing signatures:** "do not block" is explicit; `// TODO: signature unclear — API shape TBD` is the escape hatch. ✓

5. **Graduated fallback:** Read steps are ordered (existence check → read source → read dest → record → TODO if unclear), providing a clear decision tree. ✓

---

## Memory References

Related context from project memory files (see `~/.claude/projects/H--Code-Rust-perl-lsp/memory/MEMORY.md`):

- **[Red-TDD must read actual APIs](feedback_red_tdd_needs_api_read.md)** — Original problem diagnosis from G1 waves. "Red-tdd writes tests against imagined API shapes; builders keep fixing them (G1a: 3, G1b: 6, growing). Add explicit API-read to red-tdd prompt; document public surfaces upstream."

- **[Thick grounded agent definitions](feedback_thick_grounded_agents.md)** — Design pattern for agent files: should be repo-specific, not generic. This spec-planner checklist is an example of thick grounding (enumerates exact files, paths, lines, procedures).

- **[Verification pipeline is sequential](feedback_verification_is_sequential.md)** — Each agent reads previous output. Accuracy-scout → research → oppositional → architecture → diaboli → maintainer → plan-review → spec-planner.

- **[Microcrate collapse v0.13.0](project_microcrate_collapse_v014.md)** — Wave G1a (perl-module) merged #4422, G1b (perl-workspace) shipped #4510. G2 is the next absorption wave. This issue (Fix A) is a process improvement to reduce G2 API-shape friction. Fix B (deferred) is a follow-up if G2 data shows it's needed.

- **[Scope-pivot on DEFER](feedback_scope_pivot_on_defer.md)** — When diaboli/maintainer says DEFER, first question is "does this still apply at reduced scope?" The answer here is yes: Fix A is independently valuable, Fix B can wait for G2 measurement.

---

## Decision Rationale: Why Fix A, Not Fix B?

**Principle: Defer when measurement can decide.**

Fix B (spec-planner API enumeration) adds documentation burden to the spec-planner agent. Today, `.spec/<issue#>/context.md` is handwritten context for reviewers and future maintainers. Requiring an enumerated "Public API surfaces" section turns it into a quasi-API reference that must stay in sync with actual code.

The Advocatus-diaboli verdict explicitly raised objection O3: **"Fix B's API enumeration will rot silently"** (context.md changes are optional post-implementation, so old enumerations can linger).

**Solution: Measurement gate.**

- Do Fix A now (low risk, high ROI process change)
- Measure G2 wave: count `NOTE(G2-API-fix)` comments in red-TDD commit
- If ≤2 (vs. G1b's 6): Fix A is sufficient, close the follow-up and move on
- If ≥4: Open a follow-up issue referencing alternatives (rustdoc-JSON tooling or pipeline redesign)

This way, Fix B only gets built if the data shows it's needed, and the scope is right-sized.

---

## No "Public API Surfaces" Section (Fix B Deferred)

Per the plan-reviewer's explicit note: "Builder must NOT touch any `spec-planner-*.md` file." And per diaboli: Fix B is deferred.

**This `.spec/4513-red-tdd-api-read/context.md` file is intentionally thin on public API documentation** because Fix B (which would enumerate APIs) is not in scope. Future agents reading this can see the trade-off decision and the measurement gate in the diaboli comment.

---

## Related PRs and Waves

- **PR #4506 (Wave G1a):** First absorption wave (perl-module collapse). Red-TDD had 3 API-shape fixes documented with `NOTE(G1a-API-fix)`.
- **PR #4510 (Wave G1b):** Second absorption wave (perl-workspace collapse). Red-TDD had 6 API-shape fixes, doubling.
- **This issue (#4513):** Process improvement to reduce API-shape fixes in G2 and beyond.
- **Wave G2 (future):** Will test the updated red-TDD prompts. If ≤2 API-shape fixes, Fix A wins. If ≥4, opens discussion of Fix B or alternatives.

---

## Handoff Notes

**For Red-TDD (when this spec is handed off):**
- No Rust tests to write here; this is a prompt-update verification
- Acceptance is via file inspection (grep commands in checklist.md)
- Sign off with a comment on the issue rather than a test commit
- Next absorption issue you encounter (e.g., Wave G2) should follow the updated prompts in this issue

**For Builder:**
- Three markdown files to edit
- No production code changes
- Verify with grep commands in checklist.md
- Done when all acceptance criteria checked

**For Future Agents (G2 red-TDD and beyond):**
- This `.spec/4513-red-tdd-api-read/` folder is permanent project history
- Shows the decision to defer Fix B pending measurement
- Shows the exact wording of the API-read procedure
- If G2 data suggests Fix B is needed, reference this issue's diaboli comment for alternatives

---

## Summary

**Issue #4513** is a targeted process improvement to the red-TDD agent prompt. Fix A (this issue) adds an explicit API-read step. Fix B (spec-planner enhancement) is deferred pending G2 measurement.

The scope is tight, the risk is low, and the ROI is potentially high (halving API-shape fixes from 6 to ≤2). Accepted by all verification layers. Ready for builder.
