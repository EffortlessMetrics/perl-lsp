# Acceptance Criteria: Issue #4513 Red-TDD API-Read Prompt Update

This file enumerates the exact acceptance criteria that define when issue #4513 is complete. Red-TDD uses these as test assertions (where applicable). Builder verifies all are met before closing.

---

## Primary Acceptance Criteria

- [ ] **A1.** `.claude/commands/red-tdd-read.md` has a new Step 5 describing the absorption API-read procedure, with sub-steps a–e explicitly covering:
  - Check source crate existence (`ls crates/<absorbed-crate>/src/lib.rs`)
  - Read source crate if exists (`cat crates/<absorbed-crate>/src/lib.rs` + pub use chains)
  - Read destination module if source already absorbed (`cat crates/<dest-crate>/src/providers/<module>.rs`)
  - Record exact signatures (no inference)
  - Fallback to `// TODO: signature unclear — API shape TBD` if unlocatable

- [ ] **A2.** `.claude/commands/red-tdd-write.md` has a guard note immediately before Step 1 with text "Absorption issues — API-shape guard" that instructs agents to confirm API signatures were read in `/red-tdd-read` Step 5

- [ ] **A3.** `.claude/commands/red-tdd-write.md` Step 4 has a third bullet point stating "If testing an absorbed type: use only signatures confirmed during the read step — never add a `Default::default()` or `::new()` call based on Rust convention alone"

- [ ] **A4.** `.claude/agents/red-tdd.md` Principles section has a fifth bullet (after "Read existing tests first") with text containing "For absorption issues, read actual APIs first" that covers the key points:
  - Read `src/lib.rs` and follow `pub use` chains
  - Handle "source crate no longer exists" case (read destination module instead)
  - Do not infer `Default`, no-arg `new()`, field shapes
  - Use `// TODO: signature unclear — API shape TBD` for unlocatable signatures

- [ ] **A5.** Zero changes to `.claude/commands/spec-planner-plan.md` or `.claude/agents/spec-planner.md` (Fix B is DEFERRED — diaboli verdict mandates this scope boundary)

---

## Non-Blocking Measurement Criteria

- [ ] **G2 Measurement Gate:** When Wave G2 ships, count `NOTE(G2-API-fix)` comments in the red-TDD commit.
  - **Success target:** ≤2 API-shape fixes
  - **Re-evaluation trigger:** If ≥4 `NOTE(G2-API-fix)` comments appear, open a follow-up issue referencing:
    - oppositional-planner alternative A2 (rustdoc-JSON auto-generation)
    - oppositional-planner alternative A3 (skip red-TDD for absorption work)
  - **Non-blocking:** This gate does not prevent merge; it informs the decision to defer/undefer Fix B in a follow-up issue

---

## Scope Exclusions (What NOT to Change)

- [ ] No changes to production code under `crates/`
- [ ] No changes to test code under `crates/*/tests/`
- [ ] No changes to `.spec/` files (those are spec-planner's responsibility; this checklist itself is the only new `.spec/` content for this issue)

---

## Verification

**Builder:** After committing, run:
```bash
# Verify the three files have the exact content per checklist
grep -n "absorbed crate" .claude/commands/red-tdd-read.md
grep -n "API-shape guard" .claude/commands/red-tdd-write.md
grep -n "For absorption issues, read actual APIs first" .claude/agents/red-tdd.md

# Confirm spec-planner files are untouched
git diff HEAD -- .claude/commands/spec-planner-plan.md .claude/agents/spec-planner.md
# Expected: empty output

# Show final diff
git diff --stat HEAD
```

**Success:** All grep commands find the text, `git diff` on spec-planner files is empty, and `git diff --stat` shows only the three red-tdd files modified.

---

## Known Edge Cases (Handled in Spec)

1. **Post-collapse crate absence:** Step 5c of the new red-tdd-read.md explicitly handles the case where a source crate no longer exists (already absorbed by a prior wave). Agent reads destination module instead. Handled: ✓

2. **Re-export chains:** Step 5b specifies "follow any `pub use` chains into sub-modules" so agent does not stop at a bare re-export in lib.rs. Handled: ✓

3. **Non-absorption scoping:** The instruction is explicitly gated on "issues involving crate absorption or module refactoring" so it does not fire on parser bug fixes or feature adds. Handled: ✓

4. **Blocking on missing signatures:** The instruction says "do not block"; `// TODO: signature unclear — API shape TBD` is the escape hatch, not a hard stop. Handled: ✓

5. **Graduated fallback:** Read steps are ordered (check existence → read source → read destination → record → TODO if unclear), providing a clear decision tree. Handled: ✓

---

## Related Context

- **Prior wave evidence:** G1a=3 API-shape fixes, G1b=6 API-shape fixes (doubling pattern)
- **Root cause:** Red-TDD reads spec (abstract "what should exist") rather than code (actual `pub struct` / `pub fn` signatures)
- **Fix A (this issue):** Update red-TDD prompts to require API-read step before writing tests
- **Fix B (deferred):** Require spec-planner to include "Public API surfaces" enumeration in `context.md` (diaboli verdict: defer pending G2 measurement)
- **Decision rationale:** Fix A is low-risk, high-ROI process change; Fix B adds spec-planner maintenance burden, so defer until G2 shows whether Fix A alone is sufficient

---

## Handoff

**Red-TDD:** No Rust tests to write for this issue. This is a prompt-update verification. Red-TDD reads this checklist and acceptance.md, confirms all three files are correctly edited (via grep), and signs off with a comment on the issue rather than a test commit. Recommend: Post a sign-off comment and hand off to builder with a note that "acceptance is via file inspection (grep commands above) rather than cargo tests."

**Builder:** Edit the three markdown files exactly as specified in checklist.md. Verify with grep commands. Done when all acceptance criteria are checked.

**Next agent (G2 red-TDD):** When the next absorption issue arrives, use the updated prompts in this issue. This spec-planner checklist becomes a reference for the new red-TDD process.
