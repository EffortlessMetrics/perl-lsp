# Implementation Checklist: #4513 Red-TDD API-Read Prompt Update

**Issue:** #4513 Red-TDD agents write tests against imagined API shapes; fix via red-TDD prompt enhancement.

**Scope:** Fix A only (red-TDD prompt updates). Fix B (spec-planner API enumeration) is DEFERRED (see diaboli verdict in issue comments).

**Branch:** `impl/4513-red-tdd-api-read`

**Note on testing:** This is a **markdown-only process change** — no Rust code to test. Red-TDD writes the tests against the updated prompt as part of issue #4513's acceptance. The builder verifies the three files were updated via grep (see "Verify Commands" below), not via cargo tests. After builder completes, red-TDD reads this issue again (next cycle) and writes absorption-issue tests using the new prompt.

---

## Implementation Steps

### Step 1: Update `.claude/commands/red-tdd-read.md` — Add Step 5 (Absorption API Read)

**File path:** `/h/Code/Rust/perl-lsp/.claude/commands/red-tdd-read.md`

**Current state:** File has 5 steps (1–5) in the "## Steps" section.

**Change:** 
- Renumber current "5. Identify from acceptance.md..." to become "6. Identify from acceptance.md..."
- Insert new Step 5 immediately after current Step 4 (which reads existing test patterns)

**New Step 5 to add:**

```markdown
5. For issues involving crate absorption or module refactoring (any issue where a crate's symbols move into a new module), read each absorbed crate's actual public API **before** writing any test:

   a. Check whether the source crate still exists on this branch:
      ```bash
      ls crates/<absorbed-crate>/src/lib.rs
      ```
   b. If it exists, read it:
      ```bash
      cat crates/<absorbed-crate>/src/lib.rs
      ```
      Then follow any `pub use` chains into sub-modules to locate the actual struct/fn/trait declarations.
   c. If the source crate has already been absorbed (file not found — prior wave merged it), read the destination module instead:
      ```bash
      cat crates/<dest-crate>/src/providers/<module>.rs
      ```
      Inspect its `pub struct`, `pub fn`, `pub trait`, and `pub use` items for exact signatures.
   d. Record the exact signatures you will test against. Do not infer `Default`, no-arg `new()`, field types, or trait bounds — use only what you read.
   e. If a signature cannot be located after checking both source and destination, write the test stub with a prominent comment:
      ```rust
      // TODO: signature unclear — API shape TBD. Builder: verify before making this green.
      ```
```

**Dependency:** None — this is a new step, no prior changes required.

**Verify command:**
```bash
grep -n "absorbed crate" /h/Code/Rust/perl-lsp/.claude/commands/red-tdd-read.md
# Expected: Line number found in output
```

---

### Step 2: Update `.claude/commands/red-tdd-write.md` — Add Guard Note and Signature Accuracy Reminder

**File path:** `/h/Code/Rust/perl-lsp/.claude/commands/red-tdd-write.md`

**Current state:** File has section "## Steps" with 5 numbered steps.

**Changes:**

#### 2a: Add guard note before Step 1

**Location:** Immediately before the line "1. For each acceptance criterion..." in the "## Steps" section.

**Guard note to add:**

```markdown
> **Absorption issues — API-shape guard:** Before writing any test that references a symbol from an absorbed crate, confirm you read that crate's actual `pub struct` / `pub fn` / `pub trait` / `pub use` declarations in `/red-tdd-read` Step 5. Do not infer `Default`, no-arg `new()`, or field shapes. If you did not capture the exact signature during the read step, go back and read it now. If a signature cannot be located, use `// TODO: signature unclear — API shape TBD` and continue — do not block.
```

#### 2b: Harden Step 4 with signature-accuracy reminder

**Location:** In Step 4, the bullet point section that starts with "- If testing a function that doesn't exist yet..."

**Current bullets in Step 4:**
```
   - If testing a function that doesn't exist yet, test against the existing API and assert the *absence* of desired behavior
   - If testing a new type, add a minimal stub (empty struct) that compiles but has no implementation
   - Never use `todo!()` or `unimplemented!()` in test code
```

**Add a third bullet after "Never use `todo!()` or `unimplemented!()` in test code":**

```markdown
   - If testing an absorbed type: use only signatures confirmed during the read step — never add a `Default::default()` or `::new()` call based on Rust convention alone
```

**Dependency:** Step 2a (guard note) should be added before this, but they don't conflict. Complete 2a first.

**Verify command:**
```bash
grep -n "API-shape guard" /h/Code/Rust/perl-lsp/.claude/commands/red-tdd-write.md
grep -n "If testing an absorbed type" /h/Code/Rust/perl-lsp/.claude/commands/red-tdd-write.md
# Expected: Both patterns found
```

---

### Step 3: Update `.claude/agents/red-tdd.md` — Add Principle Bullet

**File path:** `/h/Code/Rust/perl-lsp/.claude/agents/red-tdd.md`

**Current state:** File has "## Principles" section with 4 bullets:
1. "Tests define done..."
2. "Read existing tests first..."
3. "One commit, one push..."
4. "Comment on the issue..."

**Change:** Add a fifth bullet after bullet 2 ("Read existing tests first...").

**New bullet 5 to add:**

```markdown
- **For absorption issues, read actual APIs first.** Before writing any test that references a symbol in an absorbed crate, read that crate's `src/lib.rs` and follow `pub use` chains to confirm exact signatures. If the source crate no longer exists (absorbed by a prior wave), read the destination module instead. Do not infer `Default`, no-arg `new()`, or field shapes — test only what the actual code declares. Unlocatable signatures get `// TODO: signature unclear — API shape TBD`, not a guess.
```

**Dependency:** None — this is a new bullet, no prior changes required.

**Verify command:**
```bash
grep -n "For absorption issues, read actual APIs first" /h/Code/Rust/perl-lsp/.claude/agents/red-tdd.md
# Expected: Line number found in output
```

---

### Step 4: Verify No Changes to Spec-Planner Files (Fix B is DEFERRED)

**File paths that must NOT be changed:**
- `/h/Code/Rust/perl-lsp/.claude/commands/spec-planner-plan.md`
- `/h/Code/Rust/perl-lsp/.claude/agents/spec-planner.md`

**Action:** Do not edit these files. Fix B (API enumeration requirement) is DEFERRED pending G2 measurement.

**Verify command:**
```bash
git diff HEAD -- .claude/commands/spec-planner-plan.md .claude/agents/spec-planner.md
# Expected: Empty (no changes)
```

---

### Step 5: Verify No Production Code Changes

**File patterns that must NOT be changed:**
- Any file under `crates/`
- Any test file under `crates/*/tests/`

**Action:** Do not edit any production code. This issue is markdown/prompt-only.

**Verify command:**
```bash
git status | grep "^.*crates/"
# Expected: No output (no modified files in crates/)
```

---

## Acceptance Criteria Checklist

- [ ] **A1.** `.claude/commands/red-tdd-read.md` has a new Step 5 with the absorption API-read procedure (sub-steps a–e), covering both "source crate exists" and "source crate already absorbed" branches
- [ ] **A2.** `.claude/commands/red-tdd-write.md` has a guard note before Step 1 and a third bullet in Step 4, both referencing the absorption API-read requirement
- [ ] **A3.** `.claude/agents/red-tdd.md` Principles section has one new bullet (fifth) explicitly naming the absorption API-read pattern
- [ ] **A4.** Zero changes to any `spec-planner-*.md` file (Fix B is DEFERRED — diaboli verdict)
- [ ] **A5.** Zero changes to any production `crates/` code

---

## Full Verify Command (Builder: Run after committing)

```bash
# Verify the three files were updated
grep -n "absorbed crate" /h/Code/Rust/perl-lsp/.claude/commands/red-tdd-read.md
grep -n "API-shape guard" /h/Code/Rust/perl-lsp/.claude/commands/red-tdd-write.md
grep -n "For absorption issues, read actual APIs first" /h/Code/Rust/perl-lsp/.claude/agents/red-tdd.md

# Confirm spec-planner files are untouched
git diff HEAD -- .claude/commands/spec-planner-plan.md .claude/agents/spec-planner.md
# Expected: empty (no changes)

# Confirm no crates/ changes
git status | grep "crates/" || echo "OK: No changes in crates/"

# Show the diff summary
git diff --stat HEAD
```

---

## Change Order and Compilation Notes

**Change order:**
1. Step 1: `.claude/commands/red-tdd-read.md` — Add Step 5
2. Step 2: `.claude/commands/red-tdd-write.md` — Add guard note + bullet
3. Step 3: `.claude/agents/red-tdd.md` — Add principle bullet
4. Step 4: Verify no spec-planner changes
5. Step 5: Verify no production code changes

**Compilation:** N/A — this is markdown-only. No cargo tests to run. Red-TDD will write Rust tests against this updated prompt in future absorption issues (e.g., Wave G2). The builder's work is complete when the three markdown files have been edited as specified and all grep verify commands pass.

---

## What Red-TDD Will Do Next (Informational)

When the next absorption issue comes through (e.g., Wave G2), red-TDD will:
1. Read this issue #4513 to understand the updated process
2. Follow the new Step 5 in `/red-tdd-read` before writing any test
3. Apply the guard note and signature checks from `/red-tdd-write` and `red-tdd.md`
4. Write absorption-issue tests with exact API shapes (no inference)
5. Document any unlocatable signatures with `// TODO: signature unclear — API shape TBD`

The metric (G2 wave should have ≤2 `NOTE(G2-API-fix)` comments) is non-blocking but will inform whether Fix B (spec-planner API enumeration) should be undeferred in a follow-up.

---

## Handoff Summary

**Builder:** Three markdown files to edit. No Rust code. No cargo tests. Verify with grep commands above. Done when all five acceptance criteria are checked and the verify commands pass clean.

**Next agent (Red-TDD):** Will read this issue and the updated prompt files before writing tests for the next absorption issue. This spec-planner checklist remains in `.spec/4513-red-tdd-api-read/` as project history.
