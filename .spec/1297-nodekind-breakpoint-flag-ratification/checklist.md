# Implementation Checklist: #1297 — Validate NodeKind safe_for_breakpoint/introduces_scope flag values before Phase 7/8 DAP consumption

## Overview

This is a **ratification-only PR**: encode ChatGPT-Pro + perl-debugger probe results into code (flip 2 flags, document instance-dependent rows) with no enum changes, no consumer migration, and no DAP integration. The ratification is authoritative; this PR makes the code match the evidence.

## Change order (compiles at each step)

### Step 1: Flip Use.safe_for_breakpoint from true to false
- **File:** `crates/perl-ast/src/classification.rs` (line 773–780)
- **Change:** Modify the flags!() arm for `NodeKind::Use`
- **Details:** Change `bp = true` to `bp = false`
  ```
  NodeKind::Use { .. } => flags!(
      exec = true,
      scope = false,
      decl = false,
      refs = false,
      children = true,
      recovery = false,
      bp = false    // <- flip from true
  ),
  ```
- **Rationale:** `use Module LIST` is `BEGIN { require; import }` — compile-time pragma not breakable in runtime debugger. Probe on perl 5.40.1 reports "not breakable".
- **Verify:** `cargo check -p perl-ast`

### Step 2: Flip No.safe_for_breakpoint from true to false
- **File:** `crates/perl-ast/src/classification.rs` (line 782–789)
- **Change:** Modify the flags!() arm for `NodeKind::No`
- **Details:** Change `bp = true` to `bp = false`
  ```
  NodeKind::No { .. } => flags!(
      exec = true,
      scope = false,
      decl = false,
      refs = false,
      children = true,
      recovery = false,
      bp = false    // <- flip from true
  ),
  ```
- **Rationale:** `no` is compile-time unimport; probe reports "not breakable".
- **Verify:** `cargo check -p perl-ast`

### Step 3: Add doc comment to module-level safe_for_breakpoint semantics
- **File:** `crates/perl-ast/src/classification.rs` (lines 21–27, module-level doc comment)
- **Change:** Extend the existing `safe_for_breakpoint` doc comment to note instance-dependent behavior
- **Details:** Add a note after the existing semantics doc:
  ```rust
  //! - **Instance-dependent flags (see docs/reference/PARSER_CONTRACTS.md §Breakpoint):**
  //!   - `Eval.introduces_scope`: true only if child is `NodeKind::Block`; `eval STRING`/`eval EXPR` have no static block scope.
  //!   - `Package.introduces_scope` & `Package.safe_for_breakpoint`: true only if `block.is_some()`; `package Foo;` has no block scope.
  //!   - `PhaseBlock.safe_for_breakpoint`: variant-level true; DAP layer checks phase name (BEGIN/CHECK/UNITCHECK compile-time → not breakable in runtime; END/INIT phase-dependent).
  ```
- **Verify:** `cargo check -p perl-ast`

### Step 4: Update SAFE_FOR_BREAKPOINT_TRUE pinned set in tests
- **File:** `crates/perl-ast/tests/classification_tests.rs` (line 245–289)
- **Change:** Remove "Use" and "No" from the SAFE_FOR_BREAKPOINT_TRUE array
- **Details:** Current array has 41 items; remove 2, leaving 39
- **Verify:** `cargo test -p perl-ast` (should pass; Use/No are now in FALSE set)

### Step 5: Update SAFE_FOR_BREAKPOINT_FALSE pinned set in tests
- **File:** `crates/perl-ast/tests/classification_tests.rs` (line 293–321)
- **Change:** Add "Use" and "No" to the SAFE_FOR_BREAKPOINT_FALSE array
- **Details:** Current array has 26 items; add 2, making 28
- **Verify:** `cargo test -p perl-ast` (should pass)

### Step 6: Add instance-dependent assertions in tests
- **File:** `crates/perl-ast/tests/classification_tests.rs` (after the safe_for_breakpoint tests, around line 360+)
- **Change:** Add a new test documenting the instance-dependent contract
- **Details:** Add a `#[test]` function with inline doc comments explaining Eval/Package/PhaseBlock instance checks
- **Verify:** `cargo test -p perl-ast` (new test should pass)

### Step 7: Document instance-dependent rows in PARSER_CONTRACTS.md
- **File:** `docs/reference/PARSER_CONTRACTS.md` (add new section or extend existing classification section)
- **Change:** Add subsection documenting safe_for_breakpoint/introduces_scope contract with instance-dependent rows and consumer guidance
- **Details:** Table with static (variant-level only) and instance-dependent rows, plus consumer implementation guidance
- **Verify:** `cargo doc -p perl-ast` (docs build; no syntax errors)

### Step 8: Final verification
- **File:** N/A (verification only)
- **Change:** N/A
- **Verify:** 
  ```
  cargo check -p perl-ast
  cargo test -p perl-ast
  cargo xtask fmt
  cargo clippy -p perl-ast
  ```

## Callers and consumers

**Primary:** None yet (this is a prefilter for Phase 8 DAP integration). No existing callers read these flags outside tests.

**Future consumers (Phase 8):** DAP breakpoint validator will read `safe_for_breakpoint` as a prefilter and perform instance-aware verification per phase name and block structure.

## Scope boundary

**Files IN scope:**
- `crates/perl-ast/src/classification.rs` (flags values + doc comments)
- `crates/perl-ast/tests/classification_tests.rs` (pinned sets + instance-dependent assertions)
- `docs/reference/PARSER_CONTRACTS.md` (breakpoint/scope contract section)

**Files OUT of scope:**
- No consumer migration (DAP/LSP code is untouched)
- No enum changes (NodeKind is untouched)
- No new types (NodeKindFlags structure unchanged)
- No parser changes (classification is pure metadata)

## Flags for builder

1. **Instance-dependent semantics are in DOCS only** — variant-level flags for Eval/Package/PhaseBlock are conservative prefilters. The actual behavior is encoded in doc comments and PARSER_CONTRACTS.md table.

2. **Drift-guard compatibility** — flags() macro is exhaustive with no wildcard. Flipping Use/No requires no new match arms (they already exist). Verify compilation succeeds.

3. **Test pinned sets must be exact** — SAFE_FOR_BREAKPOINT_TRUE and SAFE_FOR_BREAKPOINT_FALSE must partition all variants with no gaps. After flipping Use/No, both constants must be updated.

4. **No consumer regressions** — grep confirms no DAP/LSP code reads safe_for_breakpoint yet. Safe to flip. Phase 8 (separate PR) adds the consumer.

5. **DAP_CONTRACTS.md status unknown** — PR #1446 unknown status. If DAP_CONTRACTS.md lands first, extend it. Otherwise, document contract in PARSER_CONTRACTS.md.
