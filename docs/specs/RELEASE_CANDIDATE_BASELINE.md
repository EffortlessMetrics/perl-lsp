# Release Candidate Baseline

**Target**: `v0.9.0-rc1`
**Baseline Commit**: Post-PR #245 (merged 2025-12-29)

---

## 1. Local Gate Receipt Contract

Every PR description must include:

```markdown
## Gate Receipt

**Commit**: `<sha>`
**Command**: `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=1 ./scripts/gate-local.sh`

<details>
<summary>Last ~30 lines of gate output</summary>

```
>>> perl-lsp tests (workspace feature)
running 3 tests
test test_utf16_cjk_position_handling ... ok
test test_utf16_emoji_position_handling ... ok
test test_utf16_mixed_content_position_handling ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured

>>> semantic_hover tests
running 14 tests - all passed

>>> perl-dap tests
running 37 tests - all passed

╔═══════════════════════════════════════════════════════════════════════════════╗
║ ✓ Gate passed                                                                  ║
╚═══════════════════════════════════════════════════════════════════════════════╝
```

</details>
```

### Optional: Receipt Archive

```bash
# After running gate-local.sh, archive the full log
./scripts/gate-local.sh 2>&1 | tee docs/receipts/$(date +%Y%m%d)-$(git rev-parse --short HEAD).log
git add docs/receipts/
```

---

## 2. Tagging the Release Candidate

### Prerequisites

```bash
# Ensure you're on master with latest
git checkout master
git pull origin master

# Verify clean state
git status  # Should show no uncommitted changes

# Run full gate
CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=1 ./scripts/gate-local.sh
```

### Create the Tag

```bash
# Tag the release candidate
git tag -a v0.9.0-rc1 -m "Release candidate 1 for v0.9.0

Baseline for production polish phase.

Changes since 0.8.8:
- PR #245: Snapshot scan paths, UTF-16 regression tests, extended gate
- PR #244: Strict params and UTF-16 correctness across handlers
- PR #243: UTF-16 position handling fixes

Freeze scope:
- Correctness fixes only
- Install/packaging improvements
- Performance caps (no new features)

Gate receipt: ./scripts/gate-local.sh passing at $(git rev-parse --short HEAD)"

# Push the tag
git push origin v0.9.0-rc1
```

### Verify the Tag

```bash
# Confirm tag exists
git tag -l 'v0.9.*'

# View tag details
git show v0.9.0-rc1
```

---

## 3. Freeze Semantics

From `v0.9.0-rc1` forward, **only accept**:

### ✅ Allowed

| Category | Examples |
|----------|----------|
| **Correctness** | Bug fixes, UTF-16 edge cases, parse failures |
| **Install Story** | Docs, editor setup, config reference, install scripts |
| **Performance Caps** | Result limiting, latency bounds, resource limits |
| **Security** | Path traversal fixes, input validation |
| **Test Coverage** | Regression tests, edge case coverage |

### ❌ Not Allowed

| Category | Why Not |
|----------|---------|
| **New Features** | Scope creep, risk of regression |
| **Refactoring** | Unnecessary churn during stabilization |
| **API Changes** | Breaking consumers |
| **New Dependencies** | Supply chain risk |

---

## 4. RC Progression

```
v0.9.0-rc1  (baseline)
    │
    │  bug fixes only
    ▼
v0.9.0-rc2  (if needed)
    │
    │  stability confirmation
    ▼
v0.9.0      (release)
```

### Promotion Criteria to v0.9.0

- [ ] All RC issues resolved
- [ ] Gate passing on all supported platforms (Linux, macOS, Windows/WSL)
- [ ] Editor setup docs tested with VS Code, Neovim
- [ ] No new issues for 72 hours
- [ ] Performance SLOs validated

---

## 5. Quick Reference

```bash
# Run gate before any PR
CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=1 ./scripts/gate-local.sh

# Archive receipt (optional)
./scripts/gate-local.sh 2>&1 | tee docs/receipts/$(date +%Y%m%d)-$(git rev-parse --short HEAD).log

# Tag RC (maintainer only)
git tag -a v0.9.0-rc1 -m "Release candidate 1"
git push origin v0.9.0-rc1

# Cherry-pick fix to RC (if needed)
git checkout -b fix/rc1-issue-NNN v0.9.0-rc1
# ... make fix ...
git push origin fix/rc1-issue-NNN
# ... PR, review, merge to master first ...
# ... then tag rc2 from master ...
```
