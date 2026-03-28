---
description: Scout for DAP test gaps and improvement opportunities
argument-hint: "[crate] e.g. 'perl-dap-value', 'perl-dap-security', or empty for all"
---

# DAP Scout

Scout for DAP improvement opportunities. READ ONLY — returns findings, does not modify code.

Target: **$ARGUMENTS** (default: all DAP crates with low coverage)

## Steps

1. **Check test counts** for each DAP crate:
   ```bash
   cargo test -p <crate> -- --list 2>/dev/null | grep 'test$' | wc -l
   ```

2. **Identify gaps** — focus on these known low-coverage crates:
   - `perl-dap-value` — 316 LOC, low tests
   - `perl-dap-security` — 310 LOC, low tests
   - `perl-dap-shell` — 76 LOC, low tests
   - `perl-dap-command-args` — 47 LOC

3. **Review related issues**:
   - #420 — DAP forward work
   - #435 — DAP tests

4. **Return a SLICE definition** for each gap found:
   - `crate`: which DAP crate
   - `current_test_count`: number of existing tests
   - `loc`: lines of code
   - `suggested_tests`: what to test
   - `related_issues`: linked GitHub issues

## Output

Use the **Full Scout Report** variant from `/scout-issue`. Include DAP metadata subsection after Root Cause:

- `crate`: which DAP crate
- `current_test_count`: number of existing tests
- `loc`: lines of code
- `suggested_tests`: what to test
- `related_issues`: linked GitHub issues
