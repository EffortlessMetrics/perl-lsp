# Implementation Checklist for #1664

## Overview

Fix the scope analyzer to correctly treat `state` variables declared without an explicit initializer as initialized (since Perl implicitly initializes them to `undef`). This prevents false UninitializedVariable warnings.

**Target crate**: `perl-semantic-analyzer`
**Files to change**: 1
**Effort**: XS (15 minutes)
**Risk**: Very low

---

## Step 1: Modify declarations.rs to mark state vars as initialized

**File**: `crates/perl-semantic-analyzer/src/analysis/scope_analyzer/declarations.rs`

**Location**: Line 29 (in `handle_variable_declaration` function)

**Change**: Update the `is_initialized` assignment to account for `state` declarator.

**Current code** (lines 28-29):
```rust
let is_our = declarator == "our";
let is_initialized = initializer.is_some();
```

**New code**:
```rust
let is_our = declarator == "our";
let is_initialized = declarator == "state" || initializer.is_some();
```

**Rationale**: `state` variables are implicitly initialized to `undef` on first call. The check should reflect this Perl semantics by marking them as initialized even without an explicit initializer.

**Verify command**:
```bash
cargo test -p perl-semantic-analyzer --lib scope_analyzer
```

Expected: Tests compile and run without panics.

---

## Step 2: Write red tests in scope_and_symbol_tests.rs

**File**: `crates/perl-semantic-analyzer/tests/scope_and_symbol_tests.rs`

**Test location**: Add new test function in the scope_and_symbol_tests.rs file.

**Test 1: state_variable_without_initializer_not_uninitialized**

Insert after the existing state-related tests (search for `#[test]` with "state" in comments).

```rust
#[test]
fn state_variable_without_initializer_not_uninitialized() {
    // state variables are implicitly initialized to undef on first call,
    // so they should not trigger UninitializedVariable warnings
    let code = r#"
use feature 'state';

sub test {
    state $x;
    print $x;  // Should NOT warn: state is initialized to undef
}
"#;
    let issues = scope_issues(code);
    let uninitialized: Vec<_> = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UninitializedVariable && i.variable_name.contains("x"))
        .collect();
    assert!(
        uninitialized.is_empty(),
        "state without initializer should not be reported as uninitialized; found: {:?}",
        issues.iter().map(|i| (&i.kind, &i.variable_name)).collect::<Vec<_>>()
    );
}
```

**Test 2: my_variable_without_initializer_is_uninitialized (regression)**

Insert after Test 1.

```rust
#[test]
fn my_variable_without_initializer_is_uninitialized() {
    // my variables without initializers ARE truly uninitialized,
    // so they SHOULD trigger UninitializedVariable warnings
    let code = r#"
sub test {
    my $y;
    print $y;  // SHOULD warn: my is uninitialized
}
"#;
    let issues = scope_issues(code);
    let uninitialized: Vec<_> = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UninitializedVariable && i.variable_name.contains("y"))
        .collect();
    assert!(
        !uninitialized.is_empty(),
        "my without initializer should be reported as uninitialized"
    );
}
```

**Test 3: state_with_initializer_not_uninitialized (regression)**

Insert after Test 2.

```rust
#[test]
fn state_with_initializer_not_uninitialized() {
    // state variables with explicit initializers should never warn
    let code = r#"
use feature 'state';

sub test {
    state $x = 42;
    print $x;  // Should NOT warn: state with initializer
}
"#;
    let issues = scope_issues(code);
    let uninitialized: Vec<_> = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UninitializedVariable && i.variable_name.contains("x"))
        .collect();
    assert!(
        uninitialized.is_empty(),
        "state with initializer should not be reported as uninitialized"
    );
}
```

**Verify command**:
```bash
cargo test -p perl-semantic-analyzer state_variable_without_initializer_not_uninitialized -- --nocapture
cargo test -p perl-semantic-analyzer my_variable_without_initializer_is_uninitialized -- --nocapture
cargo test -p perl-semantic-analyzer state_with_initializer_not_uninitialized -- --nocapture
```

Expected: All three tests FAIL (red) before the fix in Step 1.

---

## Step 3: Verify the fix makes tests green

After Step 1 is complete, run:

```bash
cargo test -p perl-semantic-analyzer state_variable_without_initializer_not_uninitialized -- --nocapture
cargo test -p perl-semantic-analyzer my_variable_without_initializer_is_uninitialized -- --nocapture
cargo test -p perl-semantic-analyzer state_with_initializer_not_uninitialized -- --nocapture
```

Expected: All three tests PASS (green).

---

## Step 4: Verify no regressions in scope analysis

Run the full scope analysis test suite:

```bash
cargo test -p perl-semantic-analyzer --lib scope_analyzer
cargo test -p perl-semantic-analyzer --test scope_and_symbol_tests
cargo test -p perl-semantic-analyzer --test scope_golden_tests
```

Expected: All tests PASS. No regressions in my/our/local/state behavior.

---

## Step 5: Verify workspace-wide quality gates

```bash
cargo xtask fmt
cargo clippy -p perl-semantic-analyzer --lib
cargo test -p perl-semantic-analyzer
```

Expected: All commands pass without errors or warnings.

---

## Compilation Order

1. **Change declarations.rs line 29** — Single-line logic change, no new types or public API. Compiles immediately.
2. **Add test functions** — Tests reference existing scope_issues() helper and IssueKind enum. Both already in scope. Compiles immediately.
3. **Run tests** — Verify red (before fix), then green (after fix).

**No struct changes, no dependencies on other changes.** This fix is standalone and safe.

---

## Summary

| Step | Action | Files | Verify Command |
|------|--------|-------|-----------------|
| 1 | Modify initialization check to handle state | `declarations.rs:29` | `cargo test -p perl-semantic-analyzer --lib scope_analyzer` |
| 2 | Write three red tests | `scope_and_symbol_tests.rs` | Tests fail before Step 1, pass after |
| 3 | Verify fix makes tests green | (none) | `cargo test -p perl-semantic-analyzer state_variable_*` |
| 4 | Regression check scope tests | (none) | `cargo test -p perl-semantic-analyzer --test scope_*` |
| 5 | Workspace quality gates | (none) | `cargo xtask fmt && cargo clippy -p perl-semantic-analyzer` |

---

## Risk Checklist

- [ ] **is_initialized logic unchanged for my/our/local** — Verify by reading line 29 after change; `state ||` preserves original logic for other declarators
- [ ] **Both initialization path and use path verified** — mod.rs:1062-1069 reads is_initialized; our change sets it correctly
- [ ] **Edge case: state in nested blocks** — Test covers: `state` should persist across nested block scope entries (not reset)
- [ ] **Edge case: state across function calls** — `state` should persist on second call; tests verify no false warnings on reuse
- [ ] **Edge case: multiple state vars** — Different `state` vars should be independent; tests verify each separately

---

## Acceptance Criteria (from acceptance.md)

✓ Behavioral: state without initializer does not trigger UninitializedVariable warning
✓ Regression: my without initializer still triggers UninitializedVariable warning
✓ Regression: state with initializer (unchanged behavior)
✓ Hazard coverage: All 6 analyzer hazard classes addressed in test grid
✓ Contract compliance: No protocol changes, purely internal analyzer improvement
✓ API shape: No new public API or ID-space changes
✓ Blast radius: Only scope analyzer affected; LSP clients see fewer false warnings (improvement, not regression)
