---
description: Open-ended issue discovery — scan for improvement opportunities across the codebase
argument-hint: "[--limit <N>] [--dry-run]"
---

# Find Issues: Open-Ended Discovery

Scan the codebase for improvement opportunities without a specific focus. Context: **$ARGUMENTS**

## What It Checks

Run these checks in parallel and collect findings:

### 1. Clippy warnings
```bash
cargo clippy --workspace --lib 2>&1 | grep "warning\[" | sort | uniq -c | sort -rn | head -20
```

### 2. TODO/FIXME/HACK comments
```bash
grep -rn "TODO\|FIXME\|HACK\|XXX\|WORKAROUND" crates/*/src/ --include="*.rs" | head -50
```

### 3. Ignored tests
```bash
grep -rn "#\[ignore" crates/*/tests/ crates/*/src/ --include="*.rs" | head -30
```

### 4. Dead code signals
```bash
cargo machete 2>&1 | head -30
```

### 5. Missing documentation
```bash
# Public items without doc comments
cargo doc --workspace --no-deps 2>&1 | grep "warning.*missing" | head -20
```

### 6. Banned patterns in production code
```bash
# Check for unwrap/expect/panic outside tests
grep -rn "\.unwrap()\|\.expect(\|panic!\|todo!\|unimplemented!\|dbg!" crates/*/src/ --include="*.rs" | grep -v "test" | grep -v "#\[allow" | head -30
```

### 7. Large files (complexity signal)
```bash
find crates/*/src/ -name "*.rs" -exec wc -l {} \; | sort -rn | head -15
```

### 8. Test coverage gaps
```bash
# Crates with src/ but no tests/
for crate in crates/*/; do
  if [ -d "$crate/src" ] && [ ! -d "$crate/tests" ]; then
    echo "NO TESTS: $crate"
  fi
done
```

## Process

1. **Run all checks** above (in parallel where possible).

2. **Group findings by category**:
   - `clippy` — lint warnings
   - `todo-fixme` — unfinished work markers
   - `ignored-tests` — tests that could be un-ignored
   - `dead-code` — unused dependencies or dead code
   - `doc-gaps` — missing documentation
   - `banned-patterns` — coding standard violations
   - `complexity` — files that may need refactoring
   - `test-gaps` — crates missing tests

3. **For each non-empty category**, invoke `/scout-report` to create a GitHub issue:
   - Title: `<category>: <N> findings across <M> crates`
   - Body: list of specific findings with file paths
   - Label: `swarm-discovered`

4. **Print summary** to stdout:
   ```
   === Find Issues Summary ===
   clippy:          12 warnings across 5 crates
   todo-fixme:       8 markers in 4 files
   ignored-tests:    3 tests may be un-ignorable
   dead-code:        2 unused deps
   doc-gaps:         0
   banned-patterns:  1 unwrap in production code
   complexity:       3 files >500 lines
   test-gaps:        4 crates with no tests
   ---
   Created 6 GitHub issues.
   ```

## Options

- `--limit <N>` — Cap findings per category at N (default: 10)
- `--dry-run` — Print findings but do not create GitHub issues

## When to Use

- "What should we work on next?" — run `/find-issues`
- Before a swarm cycle — run `/find-issues --dry-run` to preview
- After a release — run `/find-issues` to find new improvement areas
