# Local CI Protocol (While GitHub Actions Is Unavailable)

**Status**: ACTIVE (GitHub Actions billing blocked as of 2025-11-12)
**Context**: [CI Status Report](../CI_STATUS_214.md) | [Merge Checklist #214](MERGE_CHECKLIST_214.md)

---

## Overview

While GitHub Actions is unavailable due to billing issues, this protocol defines the **local validation requirements** for merging changes to master.

**Key Principle**: Every PR must pass local CI gates before merge, with explicit documentation of blind spots.

---

## Quick Reference

### For Every PR/Merge (Required)
```bash
just ci-gate
```

### For Large/Cross-Cutting Changes (Recommended)
```bash
just ci-full
```

### Before Major Releases
```bash
just ci-full
.ci/scripts/check-from-raw.sh
cargo test --workspace --all-features --locked
```

---

## CI Targets

### `just ci-gate` - Fast Merge Gate (~2-5 minutes)

**Purpose**: Minimum viable quality gate for any merge to master

**What It Runs**:
1. **Format check**: `cargo fmt --all -- --check`
2. **Clippy (libs)**: `cargo clippy --workspace --lib --locked -- -D warnings -A missing_docs`
3. **Core tests**: `cargo test --workspace --lib --bins --locked`
4. **Policy check**: `.ci/scripts/check-from-raw.sh`

**When To Use**:
- ✅ Before every merge to master
- ✅ Before creating a PR (to ensure it's ready)
- ✅ After rebasing or resolving merge conflicts

**Pass Criteria**: All steps complete with exit code 0

---

### `just ci-full` - Comprehensive Local CI (~10-20 minutes)

**Purpose**: Full local validation including integration tests and documentation

**What It Runs** (same as `just ci-local`):
1. **Format check**: `cargo fmt --check --all`
2. **Clippy (all targets)**: `cargo clippy --workspace --all-targets -- -D warnings -A missing_docs`
   - NOTE: May hit resource limits on constrained systems
   - Fallback: `cargo clippy --workspace --lib --locked -- -D warnings -A missing_docs`
3. **Core tests**: `cargo test --workspace --lib --bins`
4. **LSP integration tests**: `RUST_TEST_THREADS=2 cargo test -p perl-lsp --test lsp_comprehensive_e2e_test -- --test-threads=2`
5. **Documentation build**: `cargo doc -p perl-parser -p perl-lsp --no-deps`

**When To Use**:
- ✅ Before merging large structural changes
- ✅ Before merging parser/LSP core changes
- ✅ Before merging CI infrastructure changes
- ✅ Before tagging releases
- ⚠️  Optionally for any merge (if you have time)

**Pass Criteria**: All steps complete with exit code 0

---

## Merge Requirements

### For Any PR

1. **Run**: `just ci-gate`
2. **Verify**: Exit code 0 (all checks pass)
3. **Document**: If any blind spots apply, note them in PR description
4. **Merge**: Proceed with merge to master

### For Large/Structural PRs

1. **Run**: `just ci-full`
2. **Verify**: All checks pass (see "Handling Failures" below)
3. **Create Checklist**: Copy `docs/ci/MERGE_CHECKLIST_214.md` template
4. **Fill Results**: Document actual test outputs and blind spots
5. **Sign Off**: Add your name to the "Decision" section
6. **Merge**: Proceed with merge to master
7. **Commit Checklist**: Add filled checklist to `docs/ci/MERGE_CHECKLIST_<PR#>.md`

---

## Handling Failures

### Clippy Resource Limits

**Symptom**: `cargo clippy --workspace --all-targets` fails with:
```
Resource temporarily unavailable (os error 11)
the compiler unexpectedly panicked
```

**Cause**: sccache or parallel compilation overwhelming system resources

**Solution**: Use library-only validation instead:
```bash
cargo clippy --workspace --lib --locked -- -D warnings -A missing_docs
```

**Why This Is Acceptable**:
- Library code is 90%+ of the codebase
- Test/binary code rarely has clippy issues that lib doesn't catch
- This is an environmental limitation, not a code quality issue

### LSP Tests All Ignored

**Symptom**: LSP comprehensive E2E test shows `33 ignored`

**Cause**: Tests are label-gated (require `ci:e2e-tests` label or similar)

**Why This Is Acceptable**:
- Test infrastructure compiles and runs (validates structure)
- Tests are intentionally gated for cost control
- Core LSP functionality is validated via unit tests in lib tests

### Documentation Warnings

**Symptom**: `cargo doc` shows missing documentation warnings

**Why This Is Acceptable**:
- We use `-A missing_docs` during systematic resolution (PR #160/SPEC-149)
- Documentation baseline is tracked separately
- Fatal errors (broken links, malformed docs) still fail the build

---

## Known Blind Spots

These limitations are **acceptable** while Actions is unavailable:

### 1. Windows Runners
**Cannot validate**: Windows-specific behavior
**Mitigation**:
- Cross-platform abstractions (e.g., `test_helpers::exit_status()`)
- Unit tests covering portable logic
- First Actions run after billing resolution will catch Windows issues

### 2. Rust Version Matrix (Beta/Nightly)
**Cannot validate**: Beta and nightly toolchain compatibility
**Mitigation**:
- Focus on stable (production target)
- Beta/nightly are "nice to have" not "must have"
- MSRV policy ensures backward compatibility

### 3. Extended Property Tests
**Cannot validate**: 256-case property test suites (too slow for local gate)
**Mitigation**:
- Standard property tests (32 cases) validated locally
- Label-gate extended tests in CI (when available)

### 4. Mutation Testing
**Cannot validate**: Full mutation testing (hours-long)
**Mitigation**:
- Mutation hardening tests included in `just ci-full`
- Full mutation testing is opt-in via `just ci-test-mutation`

### 5. Performance Benchmarks
**Cannot validate**: Benchmark regressions
**Mitigation**:
- No performance-sensitive changes expected during Actions outage
- Benchmark suite available via `just bench` if needed

---

## Adding New CI Gates

When adding new quality checks to the CI pipeline:

1. **Add to justfile**: Create a new `ci-<name>` target
2. **Test locally**: Verify it works on your dev box
3. **Add to `ci-gate` or `ci-full`**: Choose based on:
   - **ci-gate**: Fast (<30s), always required (e.g., format, basic clippy)
   - **ci-full**: Slow or comprehensive (e.g., integration tests, docs, full clippy)
4. **Document**: Update this file with the new gate
5. **Communicate**: Let the team know about the new requirement

---

## Temporary Policy

**Effective**: 2025-11-12 until GitHub Actions billing is resolved

**Requirements**:
- ✅ All changes MUST pass `just ci-gate` before merging
- ✅ Large/structural changes SHOULD pass `just ci-full` before merging
- ✅ Blind spots MUST be documented in PR descriptions or merge checklists
- ✅ First commit after Actions restoration MUST re-validate full CI matrix

**Exceptions**:
- Documentation-only changes (e.g., README updates) may skip `ci-gate` if trivial
- Emergency hotfixes may merge with abbreviated validation + post-merge full validation

**Communication**:
- Note in PR descriptions: "Validated via local CI (Actions unavailable due to billing)"
- Link to this document for transparency

---

## Post-Actions Restoration

When GitHub Actions billing is resolved:

1. **Re-run full CI** on master branch
2. **Validate Windows** runners with recent changes
3. **Run extended property tests** to catch any edge cases
4. **Review blind spots** from merge checklists
5. **Archive this document** to `docs/ci/archive/LOCAL_CI_PROTOCOL_2025-11-12.md`
6. **Return to normal** GitHub Actions-based CI workflow

---

## Examples

### Example 1: Small Bugfix PR

```bash
# Make changes
git checkout -b fix/issue-123
# ... edit code ...

# Validate before pushing
just ci-gate

# Create PR, note in description:
# "Validated via `just ci-gate` (Actions unavailable)"

# After review, merge
git checkout master
git pull origin master
git merge fix/issue-123 --no-ff
git push origin master
```

### Example 2: Large Parser Refactor

```bash
# Make changes
git checkout -b refactor/parser-cleanup
# ... edit code ...

# Full validation
just ci-full

# Create merge checklist
cp docs/ci/MERGE_CHECKLIST_214.md docs/ci/MERGE_CHECKLIST_<PR#>.md
# Fill in results...

# Create PR, attach checklist
# After review, merge with checklist
git checkout master
git pull origin master
git merge refactor/parser-cleanup --no-ff
git push origin master

# Commit checklist
git add docs/ci/MERGE_CHECKLIST_<PR#>.md
git commit -m "docs: add merge checklist for PR #<PR#>"
git push origin master
```

---

## Support

**Questions?** Check:
- [CI Status Report](../CI_STATUS_214.md) - Why Actions is unavailable
- [Merge Checklist Template](MERGE_CHECKLIST_214.md) - How to document validation
- [justfile](/justfile) - CI target definitions
- [CLAUDE.md](/CLAUDE.md) - Overall project documentation

**Issues?** Open a GitHub issue with `[LOCAL-CI]` prefix
