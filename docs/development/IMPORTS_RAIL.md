# Literal `require`/`import` Imports Burndown

> **Substrate (already built)**: static `use Foo` imports are fully analyzed; the workspace import surface and bareword resolution use it; completion tiers (project / workspace / vendor) read from it.
> **Connector gap**: literal `require Foo; Foo->import(qw(a b));` extraction. Today this pattern is dropped on the floor, so users typing it lose completion, diagnostics, and goto on the imported symbols.
> **0.14.0 upside**: legacy and pragmatic Perl code that uses `require ...; ->import(...)` (commonly used for lazy loading and conditional pulls) gains the same editor intelligence as `use Foo qw(...)`.

## Status

| Phase | Issue | Builder-ready? | PR | Receipt |
|---|---|---|---|---|
| 1. Spec — closeout shape for literal require/import | [#8616](https://github.com/EffortlessMetrics/perl-lsp/issues/8616) | spec land | [#8618](https://github.com/EffortlessMetrics/perl-lsp/pull/8618) (docs-only) | `cargo xtask semantic-scorecard --check` |
| 2. Implementation — track literal `require Module; Module->import(qw(...))` symbols | [#8623](https://github.com/EffortlessMetrics/perl-lsp/issues/8623) | yes (`builder-ready`) | _pending_ | `cargo test -p perl-module --locked` |

## Exit criteria

- [ ] All phases land or are explicitly deferred with a successor.
- [ ] Receipt commands in this doc reproduce the closeout proof.
- [ ] Status doc updated (`docs/project/status/module_resolution.md` regenerated post-merge).
- [ ] Claim boundary recorded.

## Claim boundary

This rail proves that **literal `require Module; Module->import(qw(symA symB));` adjacent to each other in lexical order produces the same import surface as `use Module qw(symA symB);`**: completion, hover, and goto on `symA` work identically.

This rail does **NOT** prove:

- Dynamic require (`require $name`, `require Module::Variant->variant_for($x)`) is analyzed. That remains an explicit non-goal — it requires runtime evaluation we will not perform.
- `Module->import()` calls separated from the `require` by control flow (loops, conditionals, function bodies) are tracked. Only the literal-adjacent-pair form is in scope.
- The PL701 diagnostic firing or non-firing for require'd modules — that lives in the module-resolution rail, not here.

## Receipts

```bash
# Phase 1 — spec land (docs only, no code receipt beyond merge)
# Phase 2 — implementation closeout:
cargo test -p perl-module --locked
cargo xtask semantic-scorecard --check
```

Note: the original rail spec referenced `perl-module-import`; the actual workspace crate is `perl-module`. Confirm against `cargo metadata --no-deps`. If a `perl-module-import` crate is introduced as part of the implementation, update this receipt and the status doc together.

## Related

- Umbrella issue: [#4280 — ux-journey: module workflow gaps — from use to debug](https://github.com/EffortlessMetrics/perl-lsp/issues/4280)
- Tracker for this rollout doc: #8626
- Spec issue: [#8616](https://github.com/EffortlessMetrics/perl-lsp/issues/8616); spec PR: [#8618](https://github.com/EffortlessMetrics/perl-lsp/pull/8618)
- Implementation issue: [#8623](https://github.com/EffortlessMetrics/perl-lsp/issues/8623)
- Architecture / spec docs: `crates/perl-module/` (import-surface owner)
- Status doc: [docs/project/status/module_resolution.md](../project/status/module_resolution.md)
- Adjacent rails:
  - `MODULE_COMPLETION_RAIL.md` — completion latency; this rail expands what completion has to offer
  - `REAL_WORKSPACE_BASELINE_RAIL.md` — must include a literal-require fixture once phase 2 lands

## Do not combine

Do **not** roll this rail's PRs into:

- The dynamic-require analysis discussion. Dynamic require is out of scope here and combining the two will produce a spec that pleases nobody.
- Any other module-resolution PR (use chains, qualifier resolution, bareword chains). Keep this rail strictly about the literal `require ...; ->import(...)` adjacency.
- The `use Module ()` empty-import work, even though they share the import surface. Different code paths, different test surfaces.

## Lane assignment

**Builder (sonnet)** — phase 2 implementation contract lives in #8623 with explicit scope, file paths, and acceptance criteria. Phase 1 spec PR #8618 is docs-only and lands before phase 2 builder starts (per the `Depends on: #8618 merge` line in #8623).
