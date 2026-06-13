---
tags: [id-collision, dap, debugger, ref-space, protocol-safety]
repos: [perl-lsp-swarm]
related: ["#1219", "#1246", "#1340"]
portable: false
article_asset: true
search_terms: [variablesReference, variables_reference, frame_id, scope_type, EvaluateResult, VariableCacheKind, allocate_evaluate_result_ref, 50_000, 1_000_000, dap_evaluate_comprehensive_tests]
---

# DAP variablesReference base 50_000 collided with scope-ref formula

**Date**: 2026-06
**Hazard class**: id-collision
**Portable lesson**: [docs/concepts/hazard-class-invariants.md](../concepts/hazard-class-invariants.md) (Class 1)

## What happened

PR #1219 allocated a new  range starting at 50_000 for expandable
DAP evaluate results. Existing scope references used the formula . For a  of 5_000 the scope ref is exactly 50_000 -- a direct
collision. The debugger would silently fetch the wrong container when the client tried
to expand the evaluate result. Deep-review caught it; the builder had to push a fix.

## Why

The new range was chosen without documenting or checking existing range allocations.
The collision is latent (requires frame_id >= 5_000) and not caught by happy-path tests
that use small frame IDs (0, 1, 2). No named constant documented the boundary; the value
50_000 was chosen by inspection, not by proof of disjointness.

## Fix

Bump the base to 1_000_000. This is provably beyond the 
range for any realistic frame count. Name the constant; document why the value was chosen.
Adversarial test: allocate one ID from each pool and assert they are never equal.

## Spec impact

Motivated Class 1 (ID/Reference-Space Collision) in
 and the corresponding acceptance-criteria row
in  section 8. Follow-up PR #1246 (DAP frameId
validation) was the first PR built with these invariants front-loaded -- it got 0-fix
deep-review. See .

## Portable lesson

Concrete instance of ID/reference-space collision: two independent allocators sharing
an untyped integer space, each unaware of the other's range. The structural fix is a
single allocator owning all ranges, or distinct newtypes per ref-space, making collisions
a compile error.

- **Pattern**: [docs/concepts/hazard-class-invariants.md](../concepts/hazard-class-invariants.md)
- **Class**: Class 1 -- ID/Reference-Space Collision
- **Generalization**: New numeric ID pools must prove disjointness from all existing pools; choose-by-inspection is insufficient.

## Related PRs

- [#1219](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1219) -- allocate variablesReference for structured results
- [#1246](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1246) -- DAP frameId validation (first PR with hazard invariants front-loaded)
- [#1340](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1340) -- added hazard-class invariants to spec system
