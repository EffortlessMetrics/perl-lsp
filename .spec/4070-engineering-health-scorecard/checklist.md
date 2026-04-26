# Implementation Checklist — Engineering Health Scorecard (#4070)

## Step 1: Create quality.md Status File
**File**: `docs/project/status/quality.md`
**Action**: CREATE — new markdown file with scorecard tables
**Dependencies**: None
**Template content** (~40 lines):

```markdown
# Engineering Health Scorecard

## Test Coverage

<!-- BEGIN: TEST_COUNT_TABLE -->
(per-crate test counts will be inserted here)
<!-- END: TEST_COUNT_TABLE -->

## Mutation Testing

<!-- BEGIN: MUTATION_SCORE_TABLE -->
(per-crate mutation scores will be inserted here)
<!-- END: MUTATION_SCORE_TABLE -->

## Metrics Collection

Test counts are extracted from `cargo test --list` output, grouped by crate.
Mutation scores are extracted from `cargo mutants --json --output <dir>` results.

## Phase 2 (Future)

- Flaky test tracking from `.ci/debt-ledger.yaml`
- Ignored test tracking via `cargo xtask ignored-tests`
- Release smoke test pass rate
- Memory usage trends
- Post-release defect rate
```

**Verify**: `ls docs/project/status/quality.md`

---

## Step 2: Refactor count_tier_a_lib_tests()
**File**: `xtask/src/tasks/update_status.rs`
**Action**: MODIFY existing function (around line 213)
**Dependencies**: None
**Current behavior**: Returns `usize` (total count across all crates)
**New behavior**: Return `BTreeMap<String, usize>` (crate name → count)

**Implementation** (~30 lines):
1. Parse `cargo test --list` output line-by-line
2. Extract crate name prefix (before `::` in test path) via regex
3. Insert/increment counter in BTreeMap
4. Return BTreeMap
5. Update callers (likely just `generate_quality_status()`) to use the map

**Verify**: `cargo build -p xtask` compiles

---

## Step 3: Add parse_mutants_json Function
**File**: `xtask/src/tasks/update_status.rs`
**Action**: CREATE — new private function (~40 lines)
**Dependencies**: Step 2 complete
**Signature**:
```rust
fn parse_mutants_json(path: &Path) -> Result<BTreeMap<String, usize>> {
    // 1. Read file
    // 2. Parse JSON array via serde_json
    // 3. Iterate, extract "package" field, count per crate
    // 4. Return BTreeMap
}
```

**Error handling**: Return `Result<_>` (not unwrap/expect)
- File missing: OK (fallback to aggregate 87% in caller)
- JSON parse error: OK (fallback to aggregate)

**Verify**: `cargo build -p xtask` compiles

---

## Step 4: Update generate_quality_status()
**File**: `xtask/src/tasks/update_status.rs`
**Action**: MODIFY existing function (around line 712)
**Dependencies**: Steps 2-3 complete
**Current behavior**: Hard-codes mutation score to 87%
**New behavior**:

1. Call `count_tier_a_lib_tests()` (refactored in Step 2) to get per-crate test counts
2. Call `parse_mutants_json()` (new in Step 3) to get per-crate mutation scores
3. Format both into markdown tables with columns: crate | count | trend / crate | score | status
4. Replace markers in original file:
   - `<!-- BEGIN: TEST_COUNT_TABLE -->` / `<!-- END: TEST_COUNT_TABLE -->`
   - `<!-- BEGIN: MUTATION_SCORE_TABLE -->` / `<!-- END: MUTATION_SCORE_TABLE -->`

**Table format**:
```
| Crate | Tests | Status |
|-------|-------|--------|
| perl-parser | 234 | — |
| perl-lsp-rs | 156 | — |
```

**Fallback behavior**: If mutation JSON missing, show aggregate 87% per original behavior (no error, graceful)

**Estimated lines**: ~100 lines total (including formatting and error handling)

**Verify**: `cargo build -p xtask`, `cargo xtask status-update` succeeds

---

## Step 5: Integration Test
**File**: `xtask/src/tasks/update_status.rs` (add unit test near bottom)
**Action**: ADD test function (~30 lines)
**Dependencies**: Steps 1-4 complete
**Test cases**:
1. Parse synthetic `cargo test --list` output, verify per-crate grouping
2. Parse synthetic mutants JSON, verify per-crate aggregation
3. Format per-crate tables, verify markdown structure

**Verify**: `cargo test -p xtask -- quality_status` passes

---

## Step 6: Verify Full Flow
**File**: None (execution only)
**Action**: Run status generation and check output
**Dependencies**: Steps 1-5 complete
**Commands**:
```bash
cargo xtask status-update
# Check that quality.md is updated
cat docs/project/status/quality.md | grep -E "^##|Test|Mutation"
# Verify table is populated
cat docs/project/status/quality.md | grep -c "perl-parser\|perl-lsp"
```

**Verify**: Output contains per-crate test counts and mutation score lines

---

## Step 7: Clippy and Formatting
**File**: None (verification only)
**Action**: Lint and format check
**Dependencies**: Steps 1-6 complete
**Commands**:
```bash
cargo clippy -p xtask -- -D warnings
cargo xtask fmt
```

**Verify**: Exit code 0 (no errors)

---

## Summary

| File | Action | Lines | Step |
|------|--------|-------|------|
| `docs/project/status/quality.md` | CREATE | 40 | 1 |
| `xtask/src/tasks/update_status.rs` | MODIFY count_tier_a_lib_tests() | 30 | 2 |
| `xtask/src/tasks/update_status.rs` | ADD parse_mutants_json() | 40 | 3 |
| `xtask/src/tasks/update_status.rs` | MODIFY generate_quality_status() | 100 | 4 |
| `xtask/src/tasks/update_status.rs` | ADD test function | 30 | 5 |

**Total scope**: ~240 lines across 2 files

**Compilation gates** (verify at each step):
- Step 2: `cargo build -p xtask`
- Step 3: `cargo build -p xtask`
- Step 4: `cargo build -p xtask`, `cargo xtask status-update`
- Step 5: `cargo test -p xtask`
- Step 6: Integration check
- Step 7: `cargo clippy`, `cargo xtask fmt`

**Builder next**: Implement Steps 1-7 in order, verify quality.md is populated with tables, commit and push.
