# Acceptance Criteria: #1362 — Nested variable list declarations

## §Behavior

| Input / Condition | Expected Result | Notes |
|---|---|---|
| `my ($a, ($b)) = (1, 2);` | Single item in nested list parses cleanly to AST with no Error nodes | Baseline: already works |
| `my ($a, ($b, $c)) = (1, 2, 3);` | Multiple items in nested list parse cleanly to AST with all items captured | Bug fix: previously threw "expected ')', found ','"; now captures both $b and $c |
| `my ($outer1, $outer2, ($inner1, $inner2)) = (1, 2, (3, 4));` | Deeply nested with assignment parses cleanly; all four variables captured | Reproduction case from issue |
| `my ($a, ($b, ($c, $d))) = (1, 2, 3, 4);` | Arbitrarily deep nesting parses cleanly | Edge case: nested within nested |
| `my ($a, (undef, $b)) = (1, 2, 3);` | undef as placeholder in nested list parses cleanly | undef is valid list item |
| `my ($a, (undef, undef, $b)) = (1, 2, 3, 4);` | Multiple undefs in nested list parses cleanly | Edge case: multiple placeholders |
| `my (@arr, (@inner)) = @list;` | Array variables in nested list parse cleanly | Slurpy variables (also valid in nested context) |

All tests pass: `cargo test -p perl-parser-core`
No clippy warnings: `cargo clippy -p perl-parser-core`
Formatted: `cargo xtask fmt`

## §Hazards

**Subsystem-specific defaults consulted**: [SUBSYSTEM_HAZARD_DEFAULTS.md — Parser](../reference/SUBSYSTEM_HAZARD_DEFAULTS.md#parser--scanner-subsystem)

| Class | Invariant | Surface (file:fn) | Required adversarial test |
|---|---|---|---|
| PARSER-1: Literal/comment blindness | Nested list item parsing must skip delimiters inside string literals, comments, and heredocs. A scanner that counts commas in bare source is insufficient. | `crates/perl-parser-core/src/engine/parser/variables.rs:parse_variable_list_item()` | `test_nested_varlist_comma_in_string` — input `my ($a, ("string with, comma")) = ...` should NOT treat the comma inside the string as a list separator. Also test heredoc and comment variants. |
| PARSER-2: Delimiter pairing | Nested list parsing must handle unbalanced parens, nested parens, and parens inside literals without panic. | `crates/perl-parser-core/src/engine/parser/variables.rs:parse_variable_list_item()` lines 163-180 | `test_nested_varlist_unbalanced_parens` — input `my ($a, ($b)` (missing close) must produce error AST node, not panic. `test_nested_varlist_nested_parens` — input `my ($a, (($b)))` must parse correctly. |
| PARSER-3: Grammar-ambiguity positive + negative oracles | Nested variable lists are valid Perl syntax; confirm via `perl -MO=Terse` that both the positive case (my ($a, ($b, $c))) and negative case (syntax that should fail) match Perl's acceptance/rejection. | Grammar rule in `parse_variable_list_item()` | `test_nested_varlist_matches_perl_oracle` — run `perl -cw` on all test inputs in this acceptance.md to confirm Perl accepts them. Document the perl command used. |
| PARSER-4: Recovery honesty | Any error-recovery path (e.g., missing comma between nested items) must produce Error/Invalid nodes that are actually reachable by the current parser, not hypothetical. | `crates/perl-parser-core/src/engine/parser/variables.rs:parse_variable_list_item()` | `test_nested_varlist_missing_comma` — input `my ($a, ($b $c))` (missing comma between $b and $c) must produce a real error path that exists in the code, not a snapshot-only fiction. |
| ID/ref-space collision | N/A — nested variable lists do not allocate numeric IDs or reference spaces; no collision hazard. | — | — |
| Test-encodes-the-bug | Red-TDD tests must encode the actual bug (multiple items in nested list failing) before implementation, not a secondary artifact. | Test file: `crates/perl-parser-core/tests/nested_varlist_tests.rs` | `test_nested_varlist_multiple_inner_items` must fail with "expected ')', found ','" error message before the fix, then pass after. |

## §Contracts

| Contract | Source document + section | How this change satisfies or extends it |
|---|---|---|
| Variable-list parsing | [PARSER_CONTRACTS.md](../reference/PARSER_CONTRACTS.md) — destructuring / variable-list section (if present) | Extends the grammar to permit nested lists with multiple items; ensures all items in nested lists are captured in the AST for semantic analysis. |
| NodeKind exhaustiveness | [PARSER_CONTRACTS.md](../reference/PARSER_CONTRACTS.md) — NodeKind classification | Introduces new `NodeKind::NestedVariableList` variant; any code doing exhaustive matching on NodeKind (semantic analyzer, LSP handlers) will be caught by compiler and must explicitly handle the new variant. |

## §API-Shape

| Item | Kind | Signature / Range | Dup-risk (grep result) | Caller count |
|---|---|---|---|---|
| `NodeKind::NestedVariableList` | enum variant | `NestedVariableList { items: Vec<Node> }` | 0 — newly introduced | 0 (new) |
| `parse_variable_list_item` | function | `fn parse_variable_list_item(&mut self) -> ParseResult<Node>` — signature unchanged, behavior extended | 1 match (existing definition in variables.rs); 3 callers unchanged | 3 existing callers: 2 in variables.rs, 1 in calls.rs |

No breaking changes to existing callers — the function maintains its signature; the fix is internal to the `LeftParen` branch.

## §Test-Grid

| Scenario | Kind | Test name | Invariant discharged |
|---|---|---|---|
| Single item in nested list | positive | `test_nested_varlist_single_inner_item` | Baseline: existing behavior preserved |
| Multiple items in nested list (main bug) | positive | `test_nested_varlist_multiple_inner_items` | PARSER-3 oracle: multiple items parse correctly |
| Deeply nested lists | positive | `test_nested_varlist_deeper_nesting` | No depth limit; arbitrary nesting works |
| Assignment with nested list | positive | `test_nested_varlist_with_assignment` | Issue reproduction case parses cleanly |
| undef as placeholder | positive | `test_nested_varlist_undef_in_nested` | undef (valid list item) works in nested context |
| Multiple undef items | positive | `test_nested_varlist_multiple_undef` | Edge case: all-undef nested lists |
| Array variables in nested list | positive | `test_nested_varlist_array_in_nested` | Slurpy variables (@, %) in nested context |
| Comma inside string literal | adversarial | `test_nested_varlist_comma_in_string` | PARSER-1: scanner blindness — comma in "string" is not a list separator |
| Unbalanced nested parens | negative | `test_nested_varlist_unbalanced_parens` | PARSER-2: delimiter pairing — missing close paren produces error AST, no panic |
| Deeply nested unbalanced | negative | `test_nested_varlist_unbalanced_deep` | PARSER-2: unbalanced at any depth |
| Missing comma between items | negative | `test_nested_varlist_missing_comma` | PARSER-4: recovery produces real error node; test does not encode false expectations |
| Perl oracle validation | oracle | `test_nested_varlist_perl_oracle` | PARSER-3: all test inputs confirmed with `perl -cw` |

## §Blast-Radius

| Consumer | Crate | Dependency type | Impact | Required update |
|---|---|---|---|---|
| `parse_variable_list_item()` callers in variables.rs | perl-parser-core | direct call (recursive + internal) | None — function signature unchanged; internal branch fix applies to all callers automatically. | None |
| `parse_variable_list_item()` caller in expressions/calls.rs | perl-parser-core | direct call | None — function signature unchanged; benefit from shared fix automatically. | None |
| NodeKind exhaustive matches in semantic analyzer | perl-semantic-analyzer | transitive (reads AST) | Potential: any code doing exhaustive pattern match on NodeKind will not compile until it handles `NestedVariableList`. Compiler will flag all sites. | Review and add handling for new variant (likely: treat as passthrough / forward to child items) |
| NodeKind exhaustive matches in LSP handlers | perl-lsp-rs | transitive (reads AST) | Potential: same as above — compiler will flag. | Review and add handling (likely: same as semantic analyzer) |
| NodeKind exhaustive matches in workspace indexer | perl-workspace | transitive (reads AST) | Potential: same as above. | Review and add handling (likely: recurse into items for variable collection) |
| Test corpus (CPAN modules) | test_corpus | none — parser-only change | None — no test corpus modifications needed. Parser fixes transparently improve corpus coverage. | None |

Must-not-touch boundary:
- `crates/perl-lexer/` — tokenizer is unchanged; parser fix does not require lexer changes
- `crates/perl-dap/` — debugger is unchanged
- `crates/perl-lsp-rs/src/dap*` — DAP bridge unchanged
- `docs/reference/PARSER_CONTRACTS.md` — no need to update existing contracts; new variant is additive
- Feature flags / capabilities — no LSP capability changes
