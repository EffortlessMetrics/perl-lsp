---
description: Accuracy-scout step 2 — check files exist, line numbers in range, function signatures match
user-invocable: false
---

# Accuracy: Verify Files

For every file path and function name claim from /accuracy-read-issue, verify
it against the current HEAD of the worktree (which tracks master).

## Steps

1. **Verify each file path exists:**

   ```bash
   # For each claimed path F1, F2, ...
   ls <claimed_path> 2>&1 || echo "MISSING: <claimed_path>"
   ```

   If the file is missing, search for a likely correct location:
   ```bash
   find crates/ -name "$(basename <claimed_path>)" 2>/dev/null | head -5
   ```

2. **Verify line numbers are in range** (if issue gave `file.rs:42`):

   ```bash
   wc -l <file_path>
   ```

   If the claimed line number exceeds the actual line count — stale reference.
   If in range, check what is actually at that line:
   ```bash
   sed -n '<line>p' <file_path>
   ```

3. **Verify every function/symbol exists:**

   ```bash
   # Search for exact function definition
   grep -rn "fn <function_name>" crates/ --include="*.rs" | head -10

   # Search for struct/trait/enum
   grep -rn "struct\|trait\|enum\|impl" crates/ --include="*.rs" | grep "<symbol_name>" | head -10
   ```

   If not found under the claimed name, search variants:
   ```bash
   # Try partial name in case it was renamed
   grep -rn "<partial_name>" crates/ --include="*.rs" | grep "fn " | head -10
   ```

4. **Record results for each claim:**

   - `VERIFIED` — file exists, line in range, function found at claimed location
   - `STALE PATH` — file moved; include correct path
   - `STALE FUNCTION` — function renamed or removed; include correct name or "not found"
   - `LINE MISMATCH` — function found in file but at different line
   - `NOT FOUND` — searched broadly, nothing found; likely removed or never existed

## Output

```
File/symbol verification for issue #NNN:

  F1: crates/perl-parser/src/expressions.rs — VERIFIED (exists, 842 lines)
  F2: crates/perl-parser/src/old_module.rs — STALE PATH (moved to crates/perl-parser/src/core/expressions.rs)
  S1: fn parse_hash_or_block — VERIFIED at crates/perl-parser/src/expressions.rs:417
  S2: fn parse_method_call — STALE FUNCTION (renamed to parse_method_invocation at expressions.rs:382)
  C1: target/cpan-corpus/lib/perl5/YAML/XS.pm — VERIFIED

Errors found: 2
```

Pass all results (including VERIFIED ones) to /accuracy-verify-claims.
