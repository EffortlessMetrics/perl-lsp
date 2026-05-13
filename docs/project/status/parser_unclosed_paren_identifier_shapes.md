# Unclosed Paren Identifier Shape Analysis

This is a hand-owned analysis note for the raw
`unclosed_paren_identifier` parser bucket. It groups repeated source-backed
fixture shapes so the next parser lane can choose a narrow grammar repair
instead of adding isolated fixtures by default.

## Claim Boundary

Evidence sources:

- [parser accuracy next](parser_accuracy_next.md)
- [parser raw failure buckets](parser.md#raw-failure-buckets)
- `.ci/parser-corpus-baseline.json`
- `crates/perl-parser-core/tests/unclosed_paren_identifier_tests.rs`

Current generated status reports:

- denominator: 50 fixtures / 29 families; 139 scored lines; 117 scored symbols
- failure packets: 0 active
- capability pointer: `parser.md#raw-failure-buckets`
- stale raw bucket route: `heredoc / delimiter handling` ->
  `unclosed_paren_identifier`

The raw bucket receipt is stale: profile `system`, commit `3c287d7db`,
generated `2026-04-28`, Perl `5.038002`, and `86` resolved roots. This note
does not claim current corpus movement or bucket reduction. Linux corpus
refresh remains the proof step before any bucket-count movement is claimed.

## Stale Source Families

The stale receipt lists these duplicated source families under
`unclosed_paren_identifier`:

- `Unicode/Collate.pm`
- `Unicode/Normalize.pm`
- `Carp.pm`
- `ExtUtils/MM_Unix.pm`
- `Pod/Simple/XHTML.pm`
- `Regexp/Common/comment.pm`

These names are discovery pointers, not current failure proof.

## Repeated Shapes

### 1. Block-list pipelines in expression contexts

Shape:

- `map BLOCK`, `grep BLOCK`, and `sort BLOCK` chains inside returns,
  assignments, for-lists, and argument lists
- nested block-list operators followed by another source list expression

Representative locked tests:

- `unicode_collate_sort_map_arrayref_pipeline`
- `dbi_registry_map_block_over_grep_block`
- `regexp_common_comment_combine_parenthesized_map_args`
- `extutils_mm_unix_map_over_grep_substitution`

Repair hypothesis:

Strengthen list-operator parsing so block-list terms and their following source
lists terminate consistently inside parenthesized and comma-list contexts.

### 2. `map EXPR, LIST` with quote-like or call expressions

Shape:

- `map EXPR, LIST` where the mapped expression is a quote-like operator,
  method call, builtin call, coderef call, or regex constructor
- caller contexts such as `join`, `pack`, `return`, lexical assignment, and
  parenthesized assignment

Representative locked tests:

- `extutils_mm_unix_ldrun_join_map_qq_rpath`
- `extutils_mm_unix_mpl_args_join_map_qq_brackets`
- `unicode_normalize_printable_map_sprintf_split`
- `pod_simple_xhtml_entity_regex_map_assignment`

Repair hypothesis:

Clarify map-expression boundaries for quote-like expressions (`qq{}`, `qq[]`,
`qr{}`) and call expressions so the following list operand is not mistaken for
an unclosed caller expression.

### 3. Pair and tuple construction from map

Shape:

- map bodies that return key/value pairs or parenthesized tuples
- unary-plus pair expressions in argument lists
- tuple-producing map blocks in `for` lists or hash assignments

Representative locked tests:

- `extutils_mm_unix_ignore_map_tuple_qw`
- `extutils_mm_unix_split_command_map_quote_literal_pair`
- `capture_tiny_stash_map_list_assignment`
- `extutils_mm_unix_to_inst_pm_wraplist_map_sort`

Repair hypothesis:

Recognize parenthesized tuple returns and unary-plus pair expressions as map
bodies without treating the tuple close as the caller's grouping boundary.

### 4. Hash/list subscripts and source-list builtins

Shape:

- source lists built from `keys`, `split`, `unpack`, and sorted key lists
- dereferenced array or hash sources
- hash-slice index lists that contain map expressions

Representative locked tests:

- `extutils_mm_unix_hash_slice_map_lc_keys_assignment`
- `unicode_collate_hst_join_map_split_expr`
- `unicode_collate_unpack_u_coderef_map_expr`
- `extutils_mm_unix_grep_parens_over_map_arrayref_default`

Repair hypothesis:

Tighten source-list termination after `keys`, `split`, `unpack`, dereferenced
array sources, and hash-slice index lists so map/grep source operands do not
consume the parent expression incorrectly.

### 5. Non-map neighboring shapes

Shape:

- main-package variable parsing inside parenthesized expressions
- typeglob assignment with ternary anonymous subs
- scalar ternary/caller expressions
- prefix-decrement in repetition expressions
- filehandle and builtin-call parentheses

Representative locked tests:

- `main_package_variable_in_paren_expr`
- `unicode_normalize_typeglob_ternary_native_subs`
- `local_carp_not_scalar_ternary_caller`
- `x_repetition_prefix_decrement_in_parens`
- `print_filehandle_in_unless`
- `print_block_filehandle_in_if`

Repair hypothesis:

Keep these out of the first map/grep/sort grammar repair. They are adjacent
`unclosed_paren_identifier` regressions or older locked false-positive shapes,
but they do not share the same list-operator boundary as the repeated map/grep
fixture train.

## Recommended Next Parser PR

Start with the list-operator boundary repair:

```text
fix(parser): repair repeated map/grep/sort expression boundary
```

Scope:

- one parser behavior change
- existing `unclosed_paren_identifier_tests` fixtures as the safety net
- no generated status hand edits
- no bucket-count movement claim without the Linux corpus refresh

Fixture-only PRs should continue only when a new real-Perl source shape is not
covered by the existing groups above.

## Verification

For this analysis note:

```bash
cargo test -p perl-parser-core --test unclosed_paren_identifier_tests --profile agent --locked -- --nocapture
cargo xtask metrics parser-accuracy --check
cargo xtask update-status --only parser --check
cargo xtask metrics ratchet-check parser_accuracy
cargo xtask fmt --check
git diff --check
```
