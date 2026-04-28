# Specification: Hash Slice Parsing Fix (`work-e5278c16`)

## Feature/Behavior Description

Fix the parser to correctly handle hash slices and array slices (`@hash{...}`, `%hash{...}`, `@array{...}`) as postfix subscript operations without requiring an intervening arrow (`->`).

### Problem Statement

The parser produces `unexpected_comma_expr` errors on Perl code like:
```perl
@ops_seen{ map split(/ /), values %ops } = ();
```

This is a **hash slice** (accessing multiple hash values at once). The parser incorrectly treats `@ops_seen` and `{ map split(/ /), values %ops }` as two separate expressions, causing the comma inside `map split(/ /), values %ops` to be flagged as unexpected.

### Root Cause

In `postfix.rs`, the `parse_postfix_chain()` function only handles `LeftBrace` as a subscript inside the `Arrow` arm (line 267). There is no case for `LeftBrace` directly after a variable with `@` or `%` sigil.

### Expected Behavior

| Perl Code | Parsed As | Currently? |
|----------|-----------|------------|
| `$ref->{key}` | Arrow hash dereference | ✅ Works |
| `@hash{key1, key2}` | Hash slice (postfix) | ❌ Treated as separate expressions |
| `%hash{key1, key2}` | Hash slice (postfix) | ❌ Treated as separate expressions |
| `@array{0, 1}` | Array hash-slice alias | ❌ Treated as separate expressions |
| `{ $a => $b }` | Hash literal | ✅ Works |

## Acceptance Criteria

### AC1: Hash Slice Without Arrow
```perl
%hash{key1, key2}   # Should parse as HashSlice, not separate expressions
@array{0, 1}        # Should parse as HashSlice (Perl alias), not separate expressions
```
**Test**: Parse these snippets and verify they produce a single postfix operation, not two separate expressions.

### AC2: Complex Hash Slice Expressions
```perl
@ops_seen{ map split(/ /), values %ops }
%seen{$key1, $key2}
```
**Test**: Parse these from actual corpus files and verify no `unexpected_comma_expr` errors.

### AC3: Arrow-Based Hash Dereference Unchanged
```perl
$ref->{key}         # Should still work via Arrow arm
$ref->{ $expr }     # Should still work
```
**Test**: Verify existing `->{...}` patterns continue to parse correctly.

### AC4: Hash Literal vs Block Unchanged
```perl
{ $a => $b }        # Hash literal — should still parse correctly
{ $a, $b }          # Block with list — should still parse correctly
{ expr }            # Block — should still parse correctly
```
**Test**: Verify existing hash/block parsing is not affected.

### AC5: Corpus Coverage
The 6 affected files should show 0 `unexpected_comma_expr` errors after the fix:
- `/usr/share/perl/5.38/App/Cpan.pm`
- `/usr/share/perl/5.38/overload.pm`
- `/usr/share/perl/5.38/unicore/Name.pm`
- `/usr/share/perl/5.38.2/App/Cpan.pm`
- `/usr/share/perl/5.38.2/overload.pm`
- `/usr/share/perl/5.38.2/unicore/Name.pm`

## Non-Goals

1. **Do not fix all `unexpected_comma_expr` errors** — this fix addresses only the hash slice pattern. Other `unexpected_comma_expr` errors (e.g., from genuine comma operator issues) are out of scope.

2. **Do not change hash literal vs block disambiguation** — the `hashes.rs` logic should remain unchanged for expressions that start with `{`.

3. **Do not add support for `[]` subscripts without arrow** — array subscript `[]` may have different handling and is out of scope.

4. **Do not fix the CLI build error** — the `non-exhaustive patterns: Recovered` error at `perl-parse.rs:332` is a pre-existing issue unrelated to this fix.

## Dependencies

1. **Parser infrastructure**: The fix requires access to the postfix chain parsing logic in `postfix.rs`.

2. **Token kinds**: Requires `TokenKind::LeftBrace` and `TokenKind::RightBrace` handling.

3. **Expression parsing**: Needs to delegate to existing hash/array parsing logic for the slice contents.

4. **Test infrastructure**: Uses `perl-parser-core` test framework and corpus sweep tooling for verification.

## Implementation Hint

The fix should add a new match arm at the top-level loop of `parse_postfix_chain()` (around line 28 in `postfix.rs`), not inside the `Arrow` arm. When `peek_kind()` returns `LeftBrace` and the base expression has `@` or `%` sigil, consume `{...}` as a hash/array slice postfix.

The existing `Arrow` arm's `LeftBrace` handling (line 267) should remain unchanged — it handles `->{...}` and takes precedence when preceded by arrow.

## Files to Modify

| File | Change |
|------|--------|
| `crates/perl-parser-core/src/engine/parser/expressions/postfix.rs` | Add `LeftBrace` as standalone postfix op for hash/array slices |
| `crates/perl-parser-core/tests/` | Add test file for hash slice patterns |

## Files NOT to Modify (for this fix)

- `crates/perl-parser-core/src/engine/parser/expressions/hashes.rs` — hash literal vs block disambiguation is a separate issue
- `xtask/src/tasks/parser_corpus_sweep.rs` — error bucket mapping is correct
- `crates/perl-parser-core/src/engine/parser/expressions/precedence.rs` — comma handling is correct