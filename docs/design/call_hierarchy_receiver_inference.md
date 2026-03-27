# Call Hierarchy Receiver-Inference Delivery Plan

This document translates the post-#2962 call-hierarchy gap analysis into an implementation checklist.

## Scope and Non-goals

### In scope

- Receiver-class inference for object method calls (`$obj->method(...)`) in call hierarchy.
- Inheritance-aware method lookup for resolved receiver classes.
- Bounded dynamic handling (`AUTOLOAD`, constant-foldable dynamic method names).
- Workspace-index support for incoming method references across closed files.

### Out of scope

- Whole-program Perl type inference.
- Arbitrary symbolic reference evaluation.
- Runtime package-name construction and full symbol-table mutation tracking.

## Guiding rule

When receiver/method resolution is ambiguous, omit unresolved targets instead of guessing.

## PR-by-PR sequence

## PR 1: Canonical callable identity plumbing (no behavior expansion)

### Goal

Stabilize internal identity across `prepareCallHierarchy` -> `incomingCalls`/`outgoingCalls` round trips before adding method-specific logic.

### Changes

- Add a canonical internal callable key in hierarchy resolution paths.
- Keep wire compatibility by preserving existing `CallHierarchyItem.data` fields.
- Add optional forward-compatible metadata slot(s) in `data` for dispatch semantics.
- Update stale comments to reflect post-#2962 workspace-index behavior.

### Likely touch points

- `crates/perl-lsp/src/runtime/language/hierarchy.rs`
- `crates/perl-lsp/src/call_hierarchy_provider.rs`

### Tests to add/update

- `call_hierarchy_item_data_round_trip_preserves_identity`
- `incoming_uses_data_identity_when_available`

---

## PR 2: Intraprocedural receiver inference for outgoing method calls

### Goal

Resolve common object calls in outgoing hierarchy by inferring candidate receiver classes inside a single callable.

### Receiver facts to support first

- Constructor pattern: `my $x = Foo->new(...)`
- Bless pattern: `my $x = bless $ref, 'Foo'`
- Aliasing: `$y = $x`
- Reassignment kill/narrow behavior
- Branch joins with candidate-set union

### Changes

- Extend call-site collection to carry structured method-call facts (receiver form + method name).
- Build a request-scoped receiver environment per enclosing callable.
- Populate qualified target identity when receiver class resolves uniquely.
- Continue to omit when candidate class set remains ambiguous.

### Likely touch points

- `crates/perl-lsp/src/call_hierarchy_provider.rs`
- `crates/perl-lsp/src/runtime/language/hierarchy.rs`

### Tests to add

- `outgoing_object_method_cross_file_constructor_receiver`
- `outgoing_object_method_cross_file_bless_receiver`
- `outgoing_object_method_alias_propagation`
- `outgoing_object_method_branch_join_ambiguous_omits_target`

---

## PR 3: Workspace-index method-aware incoming references

### Goal

Support incoming method calls for closed files via indexed method-call reference facts.

### Changes

- Introduce method-aware callable identity in workspace index (either `SymKind::Method` or dedicated callable key).
- Index method-call references with receiver-class context where available.
- Add method-aware reference query API used by hierarchy in Ready mode.
- Keep open-document AST scan as dirty-buffer/degraded fallback only.

### Likely touch points

- `crates/perl-workspace-index/src/workspace/workspace_index.rs`
- `crates/perl-workspace-index/src/lib.rs`
- `crates/perl-lsp/src/runtime/language/hierarchy.rs`

### Tests to add

- `incoming_object_method_cross_file_uses_workspace_index`
- `incoming_object_method_same_name_different_receiver_class_disambiguates`
- `incoming_object_method_closed_file_index_coverage`

---

## PR 4: Inheritance-aware method lookup

### Goal

Resolve inherited methods using a class graph rather than direct class-only lookup.

### Class graph inputs

- `@ISA`
- `use parent`
- `use base`
- `SUPER::method` explicit dispatch

### Changes

- Index inheritance edges in workspace index.
- Add lookup routine: class method -> parent chain fallback.
- Surface result state (`exact`, `inherited`, `ambiguous`, `unknown`) for hierarchy decisions.

### Likely touch points

- `crates/perl-workspace-index/src/workspace/workspace_index.rs`
- `crates/perl-lsp/src/runtime/language/hierarchy.rs`

### Tests to add

- `outgoing_object_method_inherited_from_parent`
- `incoming_object_method_inherited_from_parent`
- `super_dispatch_resolves_parent_method`

---

## PR 5: Bounded dynamic fallback and return hints

### Goal

Improve practical coverage for deferred dynamic cases without unbounded analysis.

### Dynamic handling tiers

- `AUTOLOAD` fallback only after normal method lookup fails.
- Dynamic method/function names only when constant-foldable to string literals.
- Keep non-constant dynamic dispatch unresolved.

### Return hints

- Index callable return hints for direct
  - `return Foo->new(...)`
  - `return bless ..., 'Foo'`
- Consume hints only for low-risk, unique-class propagation.

### Likely touch points

- `crates/perl-workspace-index/src/workspace/workspace_index.rs`
- `crates/perl-lsp/src/runtime/language/hierarchy.rs`
- `crates/perl-lsp/src/call_hierarchy_provider.rs`

### Tests to add

- `outgoing_autoload_fallback_when_method_missing`
- `outgoing_dynamic_method_constant_name_resolves`
- `outgoing_dynamic_method_nonconstant_omits_target`
- `outgoing_factory_return_hint_propagates_receiver_class`

## Test lane recommendations per PR

Start narrow, then expand:

1. Targeted call-hierarchy tests:
   - `cargo test -p perl-lsp --test lsp_call_hierarchy_tests`
2. Crate-level checks when index APIs change:
   - `cargo test -p perl-workspace-index`
3. Fast workspace regression sweep before merge:
   - `just pr-fast`

## Acceptance criteria summary

- Cross-file incoming/outgoing method hierarchy works for common OO Perl (`new`, `bless`, aliases, inheritance).
- Closed-file coverage relies on workspace index, not open-doc fallback.
- Dynamic behavior is explicitly bounded and documented.
- Ambiguous dispatch produces omission, not invented edges.
