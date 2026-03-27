# Call Hierarchy Receiver-Inference Plan

This document translates the post-`#2962` call-hierarchy gap analysis into an implementation checklist that can be shipped in small PRs.

## Scope and baseline

- Baseline: cross-file call-hierarchy resolution via workspace index is already in place.
- Remaining high-value gap: dynamic object dispatch where receiver class is not explicit at the call site.
- Non-goal: full Perl type inference or whole-program evaluation.

Guiding rule for ambiguous dispatch: **omit, do not guess**.

## Design goal

For each method call site, derive:

1. candidate receiver package/class set, and
2. called method name.

Resolve only confirmed call targets; keep unresolved when inference is ambiguous.

## PR-by-PR execution plan

### PR 1: Receiver-aware outgoing call resolution (intraprocedural only)

**Goal:** Resolve common `$obj->method(...)` calls by inferring receiver classes inside one callable.

**Primary files**

- `crates/perl-lsp/src/call_hierarchy_provider.rs`
- `crates/perl-lsp/src/runtime/language/hierarchy.rs`
- `crates/perl-lsp/tests/lsp_call_hierarchy_tests.rs`

**Implementation checklist**

- Add structured method-call facts from provider to resolver path:
  - receiver expression kind (variable, package literal, other),
  - receiver text (for diagnostics/detail),
  - method name.
- Build request-scoped receiver environment per enclosing callable:
  - `$x = Foo->new(...)` => `{Foo}`
  - `$x = bless ..., 'Foo'` => `{Foo}`
  - aliasing (`$y = $x`) propagation
  - reassignment kills prior facts
  - branch joins union candidate sets
- Use inferred receiver class set to produce qualified method candidates before workspace-index lookup.
- Preserve existing fallback behavior for unresolved calls.

**Suggested tests**

- `call_hierarchy_outgoing_method_resolves_constructor_receiver_cross_file`
- `call_hierarchy_outgoing_method_resolves_bless_receiver_cross_file`
- `call_hierarchy_outgoing_method_alias_propagation`
- `call_hierarchy_outgoing_method_branch_join_ambiguous_omits_target`

---

### PR 2: Method-aware identity and incoming references in workspace index

**Goal:** Make incoming method queries precise across closed files.

**Primary files**

- `crates/perl-workspace-index/src/workspace/workspace_index.rs`
- `crates/perl-workspace-index/src/lib.rs`
- `crates/perl-lsp/src/runtime/language/hierarchy.rs`
- `crates/perl-lsp/tests/lsp_call_hierarchy_tests.rs`

**Implementation checklist**

- Extend callable identity model for methods (method kind or dedicated callable key).
- Index method-call references with receiver-class-aware identity.
- Add workspace query API for method incoming refs.
- Update hierarchy incoming fast path to use method-aware queries.
- Keep open-document AST scan only as dirty-buffer overlay/fallback.

**Suggested tests**

- `call_hierarchy_incoming_method_cross_file_index_only`
- `call_hierarchy_incoming_method_same_name_different_packages_disambiguates`
- `call_hierarchy_incoming_method_closed_file_found_via_index`

---

### PR 3: Inheritance-aware method lookup

**Goal:** Resolve inherited method dispatch.

**Primary files**

- `crates/perl-workspace-index/src/workspace/workspace_index.rs`
- `crates/perl-lsp/src/runtime/language/hierarchy.rs`
- `crates/perl-lsp/tests/lsp_call_hierarchy_tests.rs`

**Implementation checklist**

- Index class graph edges:
  - `@ISA` assignments
  - `use parent`
  - `use base`
- Add lookup flow:
  1. receiver class method table
  2. parent chain walk
  3. `SUPER::method` handling
- Mark resolution status as exact / ambiguous / unresolved.

**Suggested tests**

- `call_hierarchy_method_lookup_finds_parent_via_isa`
- `call_hierarchy_method_lookup_finds_parent_via_use_parent`
- `call_hierarchy_super_call_resolves_parent_method`

---

### PR 4: Bounded dynamic fallback and return hints

**Goal:** Cover high-value dynamic tail without runtime evaluation.

**Primary files**

- `crates/perl-workspace-index/src/workspace/workspace_index.rs`
- `crates/perl-lsp/src/call_hierarchy_provider.rs`
- `crates/perl-lsp/src/runtime/language/hierarchy.rs`
- `crates/perl-lsp/tests/lsp_call_hierarchy_tests.rs`

**Implementation checklist**

- `AUTOLOAD` fallback when normal method lookup fails.
- Constant-string dynamic call handling:
  - `$obj->$m(...)` only when `$m` folds to string literal.
- Lightweight interprocedural return hints at index time:
  - `return Foo->new(...)`
  - `return bless ..., 'Foo'`
- Keep arbitrary symbolic refs and runtime-generated names unresolved.

**Suggested tests**

- `call_hierarchy_method_falls_back_to_autoload`
- `call_hierarchy_dynamic_method_name_constant_string_resolves`
- `call_hierarchy_dynamic_method_name_nonconstant_omits_target`
- `call_hierarchy_receiver_hint_from_simple_factory`

## Data/serialization invariants

Call hierarchy is a two-step protocol (`prepare` then `incoming`/`outgoing`).
Any identity needed later must survive round-trip serialization in `CallHierarchyItem.data`.

Near-term requirements:

- Preserve compatibility with existing `packageName` and `qualifiedName` fields.
- Add canonical callable identity payload when method-aware identity is introduced.
- Ensure degraded-mode fallback does not clobber resolved workspace-index targets.

## Definition of done

- Cross-file outgoing/incoming method results are receiver-aware for common OO patterns.
- Ambiguous method targets are omitted rather than guessed.
- Inheritance dispatch works for indexed parent edges.
- Dynamic handling remains explicitly bounded and test-covered.
