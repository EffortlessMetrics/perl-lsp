---
description: Scout step 7 — verify all file paths and function names before filing
user-invocable: false
---

# Scout Verify

Verify every file path, function name, and test pattern in your findings before filing the issue. This catches the most common scout errors.

## Steps

1. **Verify every file path exists:**
   - Glob for each `crates/*/src/*.rs` path you plan to cite
   - If a path doesn't exist, find the correct one
   - Common mistakes:
     - `crates/perl-parser/src/` vs `crates/perl-parser-core/src/engine/parser/`
     - `test_corpus/cpan/MOOSE/` does not exist — CPAN corpus lives at `target/cpan-corpus/lib/perl5/`

2. **Verify every function name:**
   - Grep for each function name you plan to cite
   - If a function doesn't exist, find the correct name
   - Common mistakes: inverted names (e.g., `parse_block_or_hash` vs `parse_hash_or_block`)

3. **Verify test patterns compile:**
   - If you wrote test code in /scout-test-spec, confirm the helpers and imports exist
   - Check `crates/perl-parser-core/tests/cpan_test_helpers/mod.rs` for available helpers

## Output

Update your task notes with corrections. If you found errors in your own references, fix them before proceeding to /scout-report.
