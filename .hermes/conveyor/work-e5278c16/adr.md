# ADR: Redirect `unexpected_comma_expr` Fix from `hashes.rs` to `postfix.rs`

## Status

**Proposed** — Redirecting fix location based on verification and plan review findings.

## Context

The work item `work-e5278c16` was created to fix `unexpected_comma_expr` errors in 18 CPAN corpus files. The issue title describes it as "list vs hash disambiguation" and the initial plan targets `parse_hash_or_block_inner()` in `crates/perl-parser-core/src/engine/parser/expressions/hashes.rs`.

However, verification and plan review agents found that:

1. **The issue title and description mischaracterize the root cause.** The actual failing pattern in `overload.pm:27` is:
   ```perl
   @ops_seen{ map split(/ /), values %ops } = ();
   ```
   This is a **hash slice** (`@hash{...}`), not a hash literal vs block disambiguation issue.

2. **The plan targets the wrong file.** The plan focuses on `hashes.rs` but the bug is in `postfix.rs`. The `LeftBrace` handling at line 267 of `postfix.rs` is ONLY inside the `Arrow` match arm. There is no case for `LeftBrace` directly after a variable (like `@hash{...}` or `%hash{...}`).

3. **The scope is smaller than claimed.** The issue claims 18 files but verification found ~6 unique files (10 error instances).

## Decision

**Redirect the fix to `postfix.rs` as a new standalone `LeftBrace` postfix operation.**

The fix should:

1. Add `LeftBrace` (and `RightBrace`) handling as a standalone postfix operation at the top-level loop of `parse_postfix_chain()` (around line 28), NOT inside the `Arrow` arm.

2. When `parse_postfix_chain()` sees `LeftBrace` after a variable with `@` or `%` sigil, treat it as a hash/array slice postfix and delegate to the existing hash parsing logic.

3. Ensure existing `->{...}` (arrow-based hash dereference) behavior remains unchanged — the `Arrow` arm should continue to handle that case.

## Consequences

### Benefits

- Fixes the actual bug: `@hash{...}` and `%hash{...}` will be recognized as postfix subscripts without requiring `->`
- Aligns with Perl semantics: hash/array slices are standard Perl syntax
- Advances CPAN coverage goal (90% target)
- Fixes errors in the 6 affected corpus files

### Tradeoffs / Risks

1. **Conflict with existing `Arrow` arm**: Adding `LeftBrace` at the top level must not interfere with `->{...}` handling inside the `Arrow` arm. The `Arrow` arm's `LeftBrace` handling should take precedence when preceded by `->`.

2. **Sigil checking required**: The fix needs to check the base expression's sigil (`@` or `%`) to distinguish `@hash{...}` (slice) from plain `{...}` (block or hash literal). If the base is a bare `{`, it should still be handled by `hashes.rs`.

3. **CLI build error**: The non-exhaustive match on `ParseError::Recovered` at `perl-parse.rs:332` prevents direct verification. This should be fixed first or an alternative verification method used.

4. **Scope uncertainty**: The issue claims 18 files but only ~6 are affected. If the corpus scope is different, the fix may not address all intended files.

## Alternatives Considered

### Alternative 1: Keep the original plan (target `hashes.rs`)

**Rejected** — The plan review and vision alignment agents confirmed this would be ineffective. The `parse_hash_or_block_inner()` function handles `{expr}` disambiguation, but `@ops_seen{...}` is never parsed as a hash literal because the postfix chain doesn't consume `{`. The `{...}` is parsed as a separate expression by a different code path.

### Alternative 2: Fix `hashes.rs` AND `postfix.rs`

**Partial adoption** — The `hashes.rs` changes may still be valuable for improving hash/block disambiguation generally, but they won't fix the `unexpected_comma_expr` errors caused by hash slices. The `postfix.rs` fix is necessary; `hashes.rs` changes are orthogonal.

### Alternative 3: Improve lookahead in `parse_expression()`

**Rejected** — This would be a more invasive change that could affect many other parsing paths. The postfix chain approach is more targeted and aligns with how other postfix operators are handled.

## Architecture Notes

The postfix chain architecture in `postfix.rs` has a gap: it only handles `{` as a postfix subscript when preceded by `->`. Perl allows `@hash{...}` and `%hash{...}` without the arrow (standard hash/array slice syntax). Adding `LeftBrace` as a standalone postfix op fills this gap.

The fix should:
- Check if the base expression is a Variable with sigil `@` or `%`
- If so, consume `{...}` as a hash/array slice postfix
- Delegate to existing hash parsing infrastructure
- Ensure the `Arrow` arm's `LeftBrace` handling still works for `->{...}`