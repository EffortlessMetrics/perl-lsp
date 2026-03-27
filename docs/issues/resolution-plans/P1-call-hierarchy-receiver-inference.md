# P1 Call Hierarchy Receiver-Inference Plan

This plan defines the next implementation tranche after cross-file call hierarchy baseline work (for example, issue `#2962`) so that object-method hierarchy becomes reliable in normal Perl OO code without claiming full Perl type inference.

## Goal

Improve call hierarchy precision for object method calls by adding:

1. intraprocedural receiver-class inference,
2. inheritance-aware method lookup,
3. bounded dynamic fallback (`AUTOLOAD` and constant-folded dynamic names), and
4. lightweight return hints for simple factories.

## Non-goals

- Full whole-program type inference.
- Runtime-complete support for symbolic references and arbitrary dynamic dispatch.
- Program execution or emulation to discover call targets.

## Delivery Strategy

### PR 1 — Receiver-aware outgoing calls (80/20)

Scope:

- `crates/perl-lsp/src/call_hierarchy_provider.rs`
- `crates/perl-lsp/src/runtime/language/hierarchy.rs`
- `crates/perl-lsp/tests/lsp_call_hierarchy_tests.rs`

Work:

- Extend method call-site facts captured from AST walks with structured receiver data.
- Build a request-scoped receiver environment inside one callable:
  - `my $x = Foo->new(...)`
  - `my $x = bless ..., 'Foo'`
  - aliasing (`$y = $x`)
  - branch joins (candidate-set union)
  - reassignment (kill/narrow set)
- Resolve outgoing method targets from inferred receiver class candidates; emit only confirmed targets.
- Preserve semantic identity in `CallHierarchyItem.data` round-trip.

Acceptance tests:

- cross-file outgoing object method target (`$obj->method`) from constructor-created receiver,
- outgoing resolution through alias (`$y = $x`),
- ambiguous receiver candidates omit target instead of guessing.

### PR 2 — Method-aware incoming workspace references

Scope:

- `crates/perl-workspace-index/src/workspace/workspace_index.rs`
- `crates/perl-lsp/src/runtime/language/hierarchy.rs`
- `crates/perl-lsp/tests/lsp_call_hierarchy_tests.rs`

Work:

- Make callable identity method-aware in workspace lookup paths.
- Add index facts for method-call references keyed by receiver class + method.
- Keep open-document AST traversal as overlay/fallback only.

Acceptance tests:

- cross-file incoming method calls in closed files resolve via workspace index,
- two classes with same method name (`Database::connect` vs `Cache::connect`) stay disambiguated.

### PR 3 — Inheritance-aware dispatch lookup

Scope:

- `crates/perl-workspace-index/src/workspace/workspace_index.rs`
- `crates/perl-lsp/src/runtime/language/hierarchy.rs`
- `crates/perl-lsp/tests/lsp_call_hierarchy_tests.rs`

Work:

- Index class graph edges from:
  - `@ISA`,
  - `use parent`,
  - `use base`.
- Method lookup order:
  1. receiver class,
  2. parent chain,
  3. unknown/ambiguous result if unresolved.
- Add explicit handling for `SUPER::method`.

Acceptance tests:

- `Child` receiver resolves to inherited `Base::method`,
- explicit `SUPER::method` is routed to parent implementation.

### PR 4 — Bounded dynamic fallback + return hints

Scope:

- `crates/perl-workspace-index/src/workspace/workspace_index.rs`
- `crates/perl-lsp/src/runtime/language/hierarchy.rs`
- `crates/perl-lsp/tests/lsp_call_hierarchy_tests.rs`

Work:

- `AUTOLOAD` fallback when direct/inherited method lookup misses.
- Resolve dynamic method/function names only when constant-folded to literal strings.
- Add lightweight return hints for:
  - direct `bless` returns,
  - direct `Class->new(...)` returns,
  - unique simple factory wrappers.

Acceptance tests:

- unresolved normal lookup falls back to package/inherited `AUTOLOAD`,
- `$obj->$m()` resolves only for constant `$m`,
- simple factory return hint enables downstream `$obj->method` resolution.

## Risk Controls

- Keep request-scoped inference in LSP handlers; avoid persistent whole-workspace call graph construction.
- Preserve readiness routing (`Ready` index path first, degraded/open-doc fallback second).
- On ambiguity, omit results rather than invent targets.

## Suggested Execution Order

1. PR 1 (receiver-aware outgoing),
2. PR 2 (method-aware incoming index),
3. PR 3 (inheritance),
4. PR 4 (dynamic fallback + return hints).

## Validation Commands

```bash
# Fast checks during iteration
cargo test -p perl-lsp --test lsp_call_hierarchy_tests

# Repo-standard fast gate
just pr-fast

# Canonical local gate
nix develop -c just ci-gate
```
