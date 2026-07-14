---
tags: [guard-trigger-coverage, substring-matching, scanner-blindness, enforcement-over-doctrine, cross-pr-coupling, strict-surface, product-surface]
repos: [perl-lsp-swarm]
related: ["#3315", "#3308", "#3319", "#3276"]
portable: true
article_asset: true
search_terms: [live_strict_surface_is_clean, policy_checks, scanner qualification, incompatibility, compatibility, contains, whole-word matching, native-first qualifier, cross-pr coupling, deferred guard, sibling PR]
---

# Guard trigger coverage and substring-matching in strict-product-surface scanner

**Date**: 2026-07
**Hazard class**: Scanner-blindness / enforcement-over-doctrine / guard-trigger-coverage
**Portable lesson**: [docs/concepts/enforcement-over-doctrine.md](../concepts/enforcement-over-doctrine.md) and [docs/concepts/non-exhaustive-check-silent-drop.md](../concepts/non-exhaustive-check-silent-drop.md)

## What happened

PR #3315 introduced a strict native-product-surface scanner to enforce that all native-surface APIs are properly marked and localized, preventing leaks. Three distinct bugs were caught by deep-review and Codex:

1. **Guard trigger coverage**: The `live_strict_surface_is_clean` unit test only ran via diff-scoped `-p xtask` test selection. A documentation-only PR that reintroduced a native-surface leak would never trigger the test, defeating the guard's purpose. The test existed but its trigger condition excluded the exact change-shape it was designed to catch.

2. **Substring matching in qualifiers**: The scanner used raw `str::contains()` to check for exemption qualifiers (e.g., "optional", "legacy"). This caused false negatives: `incompatibility` embed-matched `compatibility`, and a bare `native` or `default` qualifier wrongly exempted the phrase "install perltidy for native support" (where "native" was a substring of a longer sentence, not a standalone qualifier).

3. **Cross-PR coupling deferral**: The PR introduced a JSON scan of `package.json` that correctly identified native-surface leaks in command definitions — but those leaks were owned by PR #3308 (the sibling config-integration PR). Rather than reach across PR boundaries and edit another PR's files, the findings were deferred to a clean follow-up after #3308 landed.

Deep-review caught #1 and #2. The cross-PR analysis in #3 was the scanner author's correct judgment: coupling merge order would force edits to another PR's files.

## Why

1. **Trigger coverage**: A guard is enforcement; its trigger condition must be co-located with the change it guards. A unit test for a guard that runs only under specific conditions (diff scope, crate selection) will miss the exact scenario that should trigger it. Doctrine-only guards are theater; mechanical guards must run automatically and on the shape they're designed to catch.

2. **Substring matching**: Allow/deny lists in text scanners require whole-word matching, not substring matching. When you allow "compatibility" as a qualifier, you do not intend to allow "incompatibility". When you check for a "native" marker, you mean the standalone marker, not the word "native" embedded in "native support". This is a classic false-negative generator in string-based filtering.

3. **Cross-PR coupling**: When a new guard flags content another in-flight PR owns, reaching into that PR to fix it couples merge order and violates the principle of PR-local edits. Deferral is the correct judgment: let each PR own its own files, and run the guard as a clean follow-up.

## Fix

In PR #3315:

1. **Trigger coverage fix**: Wired the `live_strict_surface_is_clean` test into the always-run `policy_checks` gate, so it executes on every PR, including doc-only PRs. The gate runs unconditionally and covers the change-shape the guard exists to catch.

2. **Substring matching fix**: Replaced `str::contains()` with `contains_word()` (a helper that checks for whole-word boundaries) and narrowed the qualifier list to words that themselves signal optionality/legacy (e.g., "optional", "legacy", "experimental", not "native" or "default" without context). The list was made explicit and the matching was made precise.

3. **Cross-PR coupling deferral**: A prototyped JSON-prose scan of `package.json` correctly flagged leaks in command titles that sibling PR #3308 owned, so it was **reverted from #3315** and deferred to a follow-up that lands after #3308 (only the reusable `unqualified_markers` core was kept). Reaching across PR boundaries to fix another PR's files would have coupled merge order.

The scanner logic lives in `xtask/src/tasks/native_product_surface.rs`; the always-run gate wiring is the `policy_checks` entry in `.ci/gate-policy.yaml`.

## Spec impact

Follow-up tracked (not yet written — this PR only touched `docs/learnings/`):

- Add an acceptance criterion to [docs/agents/SPEC_UPDATE_CHECKLIST.md](../../agents/SPEC_UPDATE_CHECKLIST.md): "Enforcement guards: the trigger condition must cover the exact change-shape the guard exists to police; a guard's enforcing test must run unconditionally, not only under a diff-scoped/crate-scoped selection."
- Add a row to [docs/reference/SUBSYSTEM_HAZARD_DEFAULTS.md](../reference/SUBSYSTEM_HAZARD_DEFAULTS.md): "Text-scanner guards: whole-word matching on qualifiers/markers, explicit allow/deny lists, and an always-on trigger across all PRs."

## Portable lesson

**Enforcement gates must be mechanical and always-on**: Doctrine-only rules are theater. If a rule must be enforced, wire it into an unconditional gate that triggers on the exact change-shape it guards.

- **Pattern**: [docs/concepts/enforcement-over-doctrine.md](../concepts/enforcement-over-doctrine.md)
- **Class**: Enforcement gap. A rule exists (doctrine), but the gate that enforces it is optional or too narrow.
- **Generalization**: A guard's trigger condition is the complement of its intent. If the intent is "all doc-only PRs must not introduce native-surface leaks," then the trigger must be "doc-only PRs." A guard that only runs under diff-scoped crate selection will miss doc-only PRs.

**Substring matching is a false-negative generator in allow/deny lists**: Text-based filtering requires whole-word matching on both the list entry and the checked input.

- **Pattern**: [docs/concepts/non-exhaustive-check-silent-drop.md](../concepts/non-exhaustive-check-silent-drop.md)
- **Generalization**: When you scan text for allowed/denied terms, use whole-word matching (word boundaries or explicit delimiters). Raw `contains()` produces silent false negatives that silently exempt unintended cases.

**Cross-PR coupling**: When a guard flags content another in-flight PR owns, defer rather than reach across boundaries. Each PR owns its files; guards should report, not edit other PRs.

- **Pattern**: None yet — this is a procedural principle, not a code hazard.
- **Generalization**: Merge order is a free variable; don't couple it by reaching into other PRs. If a sibling PR owns the content, deferred follow-up is the right call.

## Related PRs

- [#3315](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/3315) — strict native-product-surface scanner with trigger, matching, and cross-PR coupling fixes
- [#3308](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/3308) — native perlcritic config integration (sibling PR, owns command definitions)
- [#3276](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/3276) — epic: native-stack product-surface campaign
