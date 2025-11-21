# Semantic Analyzer Phase 1 Validation Plan (Band 1)

**Target**: Prove semantic stack works on resource-constrained WSL hardware
**Timeline**: TODAY (1-2 hours maximum)
**Environment**: WSL with limited CPU/RAM, no GitHub Actions available
**Status**: DRAFT - Ready for execution

---

## 🎯 Validation Objectives

1. **Prove SemanticAnalyzer Phase 1 is functional** (12/12 handlers work)
2. **Validate SemanticModel API** (`build()`, `definition_at()`, `symbol_table()`)
3. **Establish baseline metrics** for later full-scale testing
4. **Capture evidence** for "90% complete" claim

**NOT in scope**: Full LSP integration tests (those require more resources and will come later)

---

## ⚠️ Known Issue: Workspace Configuration

**Problem**: The workspace has a configuration conflict:
- `xtask` is in `workspace.members` but depends on `tree-sitter-perl`
- `tree-sitter-perl-rs` is in `workspace.exclude` but tries to inherit `edition.workspace`
- This blocks ALL cargo commands including test execution

**Workaround Options**:

### Option A: Temporary xtask Exclusion (RECOMMENDED)
```bash
# Backup current Cargo.toml
cp Cargo.toml Cargo.toml.backup

# Edit Cargo.toml to move xtask from members to exclude
# Change line 9: "xtask" → move to exclude section around line 14

# Restore after testing
cp Cargo.toml.backup Cargo.toml
```

### Option B: Fix tree-sitter-perl-rs Cargo.toml
```bash
# Edit crates/tree-sitter-perl-rs/Cargo.toml
# Line 3: edition.workspace = true → edition = "2024"
# Line 5: rust-version.workspace = true → rust-version = "1.89"
```

**Choose Option A** for minimal disruption and easy rollback.

---

## 📋 Phase 1: Minimal Smoke Tests (perl-parser only, <30 seconds)

These tests run **entirely in the perl-parser crate** with NO LSP overhead.

### Test Category 1: Core SemanticAnalyzer Unit Tests

**Location**: `crates/perl-parser/src/semantic.rs` (lines 1117-1460+)

**What to run**:
```bash
# After applying workspace fix, run these tests ONE AT A TIME:

# 1. Basic semantic tokens (fastest, ~50ms)
RUSTC_WRAPPER="" RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
  cargo test -p perl-parser --lib semantic::tests::test_semantic_tokens -- --exact --nocapture

# 2. Hover information (fast, ~60ms)
RUSTC_WRAPPER="" RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
  cargo test -p perl-parser --lib semantic::tests::test_hover_info -- --exact --nocapture

# 3. Cross-package navigation (medium, ~100ms)
RUSTC_WRAPPER="" RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
  cargo test -p perl-parser --lib semantic::tests::test_cross_package_navigation -- --exact --nocapture

# 4. Scope identification (medium, ~120ms)
RUSTC_WRAPPER="" RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
  cargo test -p perl-parser --lib semantic::tests::test_scope_identification -- --exact --nocapture

# 5. **CRITICAL**: find_definition() for scalars (NEW, Phase 1 validation)
RUSTC_WRAPPER="" RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
  cargo test -p perl-parser --lib semantic::tests::test_analyzer_find_definition_scalar -- --exact --nocapture

# 6. **CRITICAL**: SemanticModel.definition_at() API (NEW, Phase 1 validation)
RUSTC_WRAPPER="" RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
  cargo test -p perl-parser --lib semantic::tests::test_semantic_model_definition_at -- --exact --nocapture
```

**Expected runtime**: 6 tests × ~100ms each = **~600ms total** (under 1 second)

**Expected output**:
```
running 1 test
test semantic::tests::test_semantic_tokens ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; X filtered out
```

**If tests fail**:
- Check error message for specific assertion failures
- Look for "panicked at" lines indicating which validation failed
- Common issues: Symbol not found, wrong location, scope mismatch

---

### Test Category 2: Semantic Smoke Tests (Phase 1 only)

**Location**: `crates/perl-parser/tests/semantic_smoke_tests.rs`

**What to run** (Phase 1 tests only, ignore Phase 2/3):
```bash
# Run ALL Phase 1 tests (12 tests, should complete in <3 seconds)
RUSTC_WRAPPER="" RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
  cargo test -p perl-parser --test semantic_smoke_tests \
  -- --nocapture \
  --skip substitution \
  --skip method_call \
  --skip reference \
  --skip use_require \
  --skip given_when \
  --skip control_flow \
  --skip postfix_loop \
  --skip file_test

# OR run individual Phase 1 tests:
cargo test -p perl-parser --test semantic_smoke_tests test_expression_statement_semantic -- --exact --nocapture
cargo test -p perl-parser --test semantic_smoke_tests test_try_block_semantic -- --exact --nocapture
cargo test -p perl-parser --test semantic_smoke_tests test_eval_block_semantic -- --exact --nocapture
cargo test -p perl-parser --test semantic_smoke_tests test_do_block_semantic -- --exact --nocapture
cargo test -p perl-parser --test semantic_smoke_tests test_variable_list_declaration_semantic -- --exact --nocapture
cargo test -p perl-parser --test semantic_smoke_tests test_ternary_expression_semantic -- --exact --nocapture
cargo test -p perl-parser --test semantic_smoke_tests test_array_literal_semantic -- --exact --nocapture
cargo test -p perl-parser --test semantic_smoke_tests test_hash_literal_semantic -- --exact --nocapture
cargo test -p perl-parser --test semantic_smoke_tests test_phase_block_semantic -- --exact --nocapture
cargo test -p perl-parser --test semantic_smoke_tests test_unary_operators_semantic -- --exact --nocapture
cargo test -p perl-parser --test semantic_smoke_tests test_readline_operator_semantic -- --exact --nocapture
cargo test -p perl-parser --test semantic_smoke_tests test_variable_with_attributes_semantic -- --exact --nocapture
```

**Expected runtime**: 12 tests × ~200ms = **~2.4 seconds total**

**Tests validate**:
- ✅ All 12 Phase 1 node handlers (`ExpressionStatement`, `Try`, `Eval`, `Do`, `VariableListDeclaration`, `VariableWithAttributes`, `Ternary`, `Unary`, `Readline`, `ArrayLiteral`, `HashLiteral`, `PhaseBlock`)
- ✅ Semantic tokens generated correctly
- ✅ Symbol table populated
- ✅ No crashes or panics

---

## 📋 Phase 2: Semantic Model Integration Tests (OPTIONAL, <10 seconds)

**Only run if Phase 1 passes** and you have time/resources.

```bash
# SemanticModel build and tokens
RUSTC_WRAPPER="" RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
  cargo test -p perl-parser --lib semantic::tests::test_semantic_model_build_and_tokens -- --exact --nocapture

# SemanticModel symbol table access
RUSTC_WRAPPER="" RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
  cargo test -p perl-parser --lib semantic::tests::test_semantic_model_symbol_table_access -- --exact --nocapture

# SemanticModel hover info
RUSTC_WRAPPER="" RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
  cargo test -p perl-parser --lib semantic::tests::test_semantic_model_hover_info -- --exact --nocapture
```

**Expected runtime**: 3 tests × ~150ms = **~450ms total**

---

## 📋 Phase 3: Integration Test (OPTIONAL, <5 seconds)

**Only run if you have confidence and resources.**

```bash
# Complex real-world scenario
RUSTC_WRAPPER="" RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
  cargo test -p perl-parser --test semantic_smoke_tests test_complex_real_world_semantic -- --exact --nocapture
```

**Expected runtime**: ~800ms

**Validates**: Multi-package code with error handling, method calls, and complex scoping

---

## 🚨 If Tests Hang or Fail

### If tests hang (>30 seconds):
1. **Kill with Ctrl+C**
2. **Check system resources**: `free -h`, `top`
3. **Try without CARGO_BUILD_JOBS**: Remove `CARGO_BUILD_JOBS=1` flag
4. **Skip this test**: Mark as "needs investigation on better hardware"

### If tests fail with errors:
1. **Capture full output**: Add `2>&1 | tee test_output.log`
2. **Check for**:
   - Assertion failures → semantic logic bug
   - Symbol not found → indexing issue
   - Location mismatches → position tracking bug
3. **Document failure** in validation report (see below)

### If compilation fails:
1. **Workspace issue**: Go back and apply Option A or B fix
2. **Dependency issue**: Run `cargo clean` then retry
3. **Out of memory**: Reduce test batch size (run one at a time)

---

## 📊 Validation Report Template

Create this file: `SEMANTIC_VALIDATION_BAND1_RESULTS.md`

```markdown
# Semantic Analyzer Phase 1 Validation Results

**Date**: 2025-11-21
**Environment**: WSL on [your system specs]
**Duration**: [actual time]

## Phase 1: Core Unit Tests (6 tests)

| Test | Status | Time | Notes |
|------|--------|------|-------|
| test_semantic_tokens | ✅/❌ | XXms | [any issues] |
| test_hover_info | ✅/❌ | XXms | [any issues] |
| test_cross_package_navigation | ✅/❌ | XXms | [any issues] |
| test_scope_identification | ✅/❌ | XXms | [any issues] |
| test_analyzer_find_definition_scalar | ✅/❌ | XXms | [CRITICAL] |
| test_semantic_model_definition_at | ✅/❌ | XXms | [CRITICAL] |

**Pass rate**: X/6 (XX%)

## Phase 2: Smoke Tests (12 tests)

| Test | Status | Time | Node Type |
|------|--------|------|-----------|
| test_expression_statement_semantic | ✅/❌ | XXms | ExpressionStatement |
| test_try_block_semantic | ✅/❌ | XXms | Try |
| test_eval_block_semantic | ✅/❌ | XXms | Eval |
| test_do_block_semantic | ✅/❌ | XXms | Do |
| test_variable_list_declaration_semantic | ✅/❌ | XXms | VariableListDeclaration |
| test_variable_with_attributes_semantic | ✅/❌ | XXms | VariableWithAttributes |
| test_ternary_expression_semantic | ✅/❌ | XXms | Ternary |
| test_unary_operators_semantic | ✅/❌ | XXms | Unary |
| test_readline_operator_semantic | ✅/❌ | XXms | Readline |
| test_array_literal_semantic | ✅/❌ | XXms | ArrayLiteral |
| test_hash_literal_semantic | ✅/❌ | XXms | HashLiteral |
| test_phase_block_semantic | ✅/❌ | XXms | PhaseBlock |

**Pass rate**: X/12 (XX%)

## Summary

- **Total tests run**: XX
- **Pass rate**: XX%
- **Total time**: XXXms
- **Conclusion**: Phase 1 semantic analyzer is [VALIDATED ✅ / NEEDS WORK ❌]

## Evidence for "90% Complete" Claim

✅ **If 16/18 tests pass (89%+)**:
- Semantic analyzer Phase 1 (12/12 handlers) is **functionally complete**
- SemanticModel API is **stable and working**
- Core LSP foundation (definition, hover, tokens) is **validated**
- **Ready to claim 80-85% overall completion** (pending full LSP integration)

❌ **If <14/18 tests pass (<78%)**:
- Semantic analyzer needs more work
- Document specific failures for later investigation
- **Cannot claim 90% complete yet**

## Next Steps

1. [x] Band 1 validation complete
2. [ ] Band 2: Full LSP integration tests (requires better hardware or CI)
3. [ ] Band 3: Cross-file navigation and workspace features
```

---

## 🎯 Success Criteria

**Minimum for "Phase 1 Complete" claim**:
- ✅ 5/6 core unit tests passing (83%+)
- ✅ 10/12 smoke tests passing (83%+)
- ✅ **Both** CRITICAL tests passing (find_definition_scalar + semantic_model_definition_at)
- ✅ Total runtime <5 seconds
- ✅ No crashes or panics

**Ideal outcome**:
- ✅ 6/6 core unit tests (100%)
- ✅ 12/12 smoke tests (100%)
- ✅ Total runtime <3 seconds
- ✅ Clean validation report ready to commit

---

## 📝 Execution Checklist

```bash
# 1. Fix workspace issue (5 minutes)
[ ] Apply Option A (exclude xtask temporarily) OR Option B (fix tree-sitter-perl-rs)
[ ] Verify: cargo check -p perl-parser succeeds

# 2. Run Phase 1 core tests (1 minute)
[ ] test_semantic_tokens
[ ] test_hover_info
[ ] test_cross_package_navigation
[ ] test_scope_identification
[ ] test_analyzer_find_definition_scalar ⭐ CRITICAL
[ ] test_semantic_model_definition_at ⭐ CRITICAL

# 3. Run Phase 1 smoke tests (3 minutes)
[ ] All 12 Phase 1 tests (or individually if needed)

# 4. Document results (5 minutes)
[ ] Fill in validation report template
[ ] Commit report to docs/
[ ] Revert workspace changes

# 5. Celebrate or debug (variable)
[ ] If 16/18 pass: Phase 1 VALIDATED ✅
[ ] If <16/18 pass: Document issues for investigation
```

**Total estimated time**: 10-15 minutes execution + 5-10 minutes documentation = **~20 minutes**

---

## 🔄 Rollback Plan

After testing, restore workspace configuration:

```bash
# If using Option A:
cp Cargo.toml.backup Cargo.toml

# If using Option B:
git checkout crates/tree-sitter-perl-rs/Cargo.toml

# Verify rollback:
cargo check --workspace
```

---

## 📌 Related Documentation

- **Semantic Analyzer Design**: `docs/design/semantic_analyzer.md`
- **Issue #188**: Tracking for Phase 1/2/3 implementation
- **CURRENT_STATUS.md**: Project health dashboard
- **ISSUE_STATUS_2025-11-12.md**: Detailed issue analysis

---

**Status**: Ready for execution
**Owner**: [Your Name]
**Review**: [Pending]
**Validation Date**: [TBD]
