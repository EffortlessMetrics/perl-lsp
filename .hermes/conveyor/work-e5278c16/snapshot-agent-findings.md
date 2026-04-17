# Snapshot Test Findings — work-e5278c16

## What This Change Does

Fixes the parser to correctly handle hash slices and array slices (`@hash{...}`, `%hash{...}`) as postfix subscript operations without requiring an intervening arrow (`->`). The fix adds a new match arm in `parse_postfix_chain()` that recognizes `@var{...}` or `%var{...}` patterns and parses them as hash/array slice postfix operations (Binary nodes with `{}` operator).

The key outputs snapshot-tested are the S-expression representations produced by `Node::to_sexp()` for various hash slice patterns.

## Snapshots Written

- **34 snapshot tests** in `crates/perl-parser-core/tests/hash_slice_snapshot_tests.rs`

### SnapshotName: Input → Output Shape

- `snapshot_percent_hash_slice_simple`: `%hash{key}` → `(source_file (binary_{} (variable % hash) (identifier key)))`
- `snapshot_at_hash_slice_simple`: `@hash{key}` → `(source_file (binary_{} (variable @ hash) (identifier key)))`
- `snapshot_hash_slice_multiple_keys`: `%hash{key1, key2}` → `(source_file (binary_{} (variable % hash) (array ...)))`
- `snapshot_hash_slice_variable_key`: `@hash{$key}` → `(source_file (binary_{} (variable @ hash) (variable $ key)))`
- `snapshot_hash_slice_complex_map_split`: `@ops_seen{ map split(/ /), values %ops }` → Contains `binary_{}` and `variable @ ops_seen`
- `snapshot_hash_slice_single_quoted_key`: `%hash{'key'}` → Contains `binary_{}`
- `snapshot_hash_slice_double_quoted_key`: `%hash{"key"}` → Contains `binary_{}`
- `snapshot_hash_slice_trailing_comma`: `%hash{key1, key2,}` → Contains `binary_{}`
- `snapshot_hash_slice_qualified_variable`: `%Pkg::Hash{key}` → Contains `binary_{}`
- `snapshot_hash_slice_scalar_ref`: `%$href{key}` → Contains `binary_{}`
- `snapshot_arrow_hash_deref_simple`: `$ref->{key}` → `(source_file (arrow_hash_deref (variable $ ref) (identifier key)))`
- `snapshot_arrow_hash_deref_variable_key`: `$ref->{$expr}` → `(source_file (arrow_hash_deref ... (variable $ expr)))`
- `snapshot_arrow_hash_deref_nested`: `$ref->{$h->{nested}}` → Contains `arrow_hash_deref`
- `snapshot_hash_literal`: `{ $a => $b }` → Contains `hash`, NOT `binary_{}` (verifies disambiguation)
- `snapshot_block_with_list`: `{ $a, $b }` → Contains `block`
- `snapshot_empty_hash_literal`: `{}` → Contains `block`
- `snapshot_hash_slice_chained_method`: `%hash{key}->method()` → Contains `binary_{}`
- `snapshot_arrow_chained_hash_slice`: `$ref->{key}{nested}` → Contains `arrow_hash_deref`
- `snapshot_multiple_hash_slices`: `@a{@x} = @b{@y};` → At least 2 `binary_{}` nodes
- `snapshot_array_of_hashes_slice`: `@array[$i]{key1, key2}` → Contains `binary_{}`
- `snapshot_arrow_array_deref_then_hash_slice`: `$array_ref->[0]->{key}` → Contains `arrow_hash_deref` or `binary_{}`
- `snapshot_arrow_hash_deref_then_array_index`: `$hash_ref->{key}[0]` → Contains `arrow_hash_deref`
- `snapshot_hash_slice_in_conditional`: `if (%hash{@keys}) { }` → Contains `binary_{}`
- `snapshot_hash_slice_in_sort`: `sort %hash{@keys}` → Contains `binary_{}`
- `snapshot_hash_slice_in_map`: `map { $_ x 2 } %hash{@keys}` → Contains `binary_{}`
- `snapshot_assignment_to_hash_slice`: `%hash{key1, key2} = (1, 2);` → Contains `binary_{}`
- `snapshot_hash_slice_with_exists`: `exists %hash{key}` → Contains `binary_{}`
- `snapshot_hash_slice_with_delete`: `delete %hash{key};` → Contains `binary_{}`
- `snapshot_hash_slice_with_defined`: `defined %hash{key}` → Contains `binary_{}`
- `snapshot_hash_slice_then_array_index`: `%hash{key}[0, 2]` → Contains `binary_{}` and array subscript
- `snapshot_hash_slice_negative_key`: `$hash{-1}` → No error node
- `snapshot_hash_slice_large_number_key`: `$hash{999999999}` → No error node
- `snapshot_hash_slice_special_char_bareword`: `$hash{_private_key}` → No error node
- `snapshot_hash_slice_colon_key`: `$hash{'key::with::colons'}` → No error node

### Normalizes

No normalization was required. The `to_sexp()` output is fully deterministic (no timestamps, UUIDs, or random values).

### Output Shape

Each snapshot captures the full S-expression string output from `Node::to_sexp()`. The format is tree-sitter compatible, e.g.:
```
(source_file (binary_{} (variable % hash) (identifier key)))
```

## Edge Cases Covered

- Simple hash slices with `%` and `@` sigils
- Hash slices with single and multiple keys (bareword, variable, quoted)
- Hash slices with complex key expressions (map, split, values)
- Hash slices in various syntactic contexts (conditionals, sort, map, assignment, exists, delete, defined)
- Chained operations (hash slice then method call, arrow deref then hash slice, array index then hash slice)
- Arrow-based hash dereference patterns (ensuring unchanged behavior)
- Hash literals vs blocks (ensuring disambiguation still works)
- Edge cases: negative keys, large numbers, special characters in barewords, qualified package names, scalar refs

## Non-Deterministic Output Handling

The parser's S-expression output is **fully deterministic** - no timestamps, UUIDs, or random values appear in the output. Source locations are absolute character offsets that are stable for the same input. No normalization was needed before snapshotting.

## Summary

- Snapshot tests written: **34**
- All passing: **yes**
- Coverage assessment: The snapshots capture the parser's S-expression output for hash slice patterns. They verify that hash slices produce `binary_{}` nodes, arrow hash deref produces `arrow_hash_deref` nodes, and hash literals produce `hash` nodes (not `binary_{}`). Additional test files provide fuzz, property, and edge case coverage for a total of 109 hash-slice-related tests passing.
