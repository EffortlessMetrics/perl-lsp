---
tags: [config-read-scope, predicate-drift, vscode-config, component-vs-system, external-truth-gate]
repos: [perl-lsp-swarm]
related: ["#3308", "#3276"]
portable: true
article_asset: true
search_terms: [getConfiguration, languageId, globalLanguageValue, workspaceLanguageValue, inspect, hasExplicitOverride, handle_did_change_configuration, perlcritic, config predicate, second parse, config drift, scope mismatch]
---

# Config-read scope and predicate-drift in VS Code settings integration

**Date**: 2026-07
**Hazard class**: Component-proved ≠ system-proved / external-truth-gate / predicate drift
**Portable lesson**: [docs/concepts/external-truth-gate.md](../concepts/external-truth-gate.md) and [docs/concepts/verify-the-instrument.md](../concepts/verify-the-instrument.md)

## What happened

Two distinct bugs in native perlcritic config integration landed on PR #3308 and were caught only by deep-review and Codex secondary review:

1. **Config-read scope mismatch**: A fix added `globalLanguageValue` and `workspaceLanguageValue` checks to the `hasExplicitOverride` predicate to determine if a user had explicitly set a perlcritic severity override. However, VS Code's `inspect()` API only populates those fields when `getConfiguration()` is called WITH a language scope (`{uri, languageId}`); a bare `Uri` scope leaves them undefined. So the `[perl]`-block support was implemented but inert — a unit test passed, but the feature never worked in VS Code because the read itself was scoped wrong.

2. **Predicate drift from config re-parse**: The `handle_did_change_configuration` handler decided whether to reset the cached CriticAnalyzer by re-parsing ONLY the `perlcritic.*` block from the config. A user changing `critic.severity` (the new native key) would update the global config, but the handler's detection code only looked for the old `perlcritic.*` key, so the change went undetected. The config was updated, but the cache invalidation predicate diverged.

Both were verified fixes in the integration once Codex inspected the VS Code runtime behavior and the config snapshot logic.

## Why

1. **Scope mismatch**: Unit tests exercise code in isolation. A test can assert that a function works correctly when called with the right scope, but not prove that *all callers* use the right scope. The feature author implemented the check correctly but did not verify that the actual VS Code API call (in a different code path) passed the right scope. This is the component-vs-system gap: the component is correct, but its integration point is not.

2. **Predicate drift**: When a change-detection predicate re-parses the same input that a writer already parses, the two versions can diverge. The writer calls `update_from_value` on the full config; the predicate called it on a subset (`perlcritic.*` only). The fix is: snapshot state before the write, let the writer do its thing, then diff the snapshot against the new state — one parser, no divergence.

## Fix

In PR #3308:

1. **Scope fix** (`vscode-extension/src/extension.ts`, the `getPerlCriticConfiguration` helper): call `getConfiguration('perl-lsp', {uri, languageId: 'perl'})` (or `{languageId: 'perl'}` when there is no document) so that `globalLanguageValue` / `workspaceLanguageValue` are populated for `"[perl]"`-block overrides. This is the TypeScript extension read path — not the Rust server handler.

2. **Predicate-drift fix** (`crates/perl-lsp-rs/src/runtime/workspace.rs`, `handle_did_change_configuration`): replaced the re-parse predicate with a snapshot-diff — capture the relevant config fields (e.g. `perlcritic_severity`, native critic profile/include/exclude) before calling `update_from_value`, then compare after. The single writer path is now the only parser, so no divergence.

The two fixes live in **different** layers: the scope fix in the TS extension (`vscode-extension/src/extension.ts`), the snapshot-diff in the Rust runtime (`crates/perl-lsp-rs/src/runtime/workspace.rs`). Neither is in `crates/perl-lsp-rs/src/config/` — that path does not exist.

## Spec impact

None — one-off integration bugs. However, the pattern generalizes:

- Any VS Code config-read integration must be tested with the exact scope (language-aware or not) that the handler uses.
- Any config change-detection that re-parses a subset of input is a divergence hazard; prefer snapshot-diff over subset-re-parse.

A follow-up in [docs/reference/SUBSYSTEM_HAZARD_DEFAULTS.md](../reference/SUBSYSTEM_HAZARD_DEFAULTS.md) should add a row under "Config integration: scope-aware reads and single-parser change detection."

## Portable lesson

**Scope mismatch in external APIs**: A function may be implemented correctly in isolation but called with the wrong scope from the integration point. Unit tests + integration tests (end-to-end VS Code runtime) are required to prove the integration.

- **Pattern**: [docs/concepts/external-truth-gate.md](../concepts/external-truth-gate.md) and [docs/concepts/verify-the-instrument.md](../concepts/verify-the-instrument.md)
- **Class**: Component-proved ≠ system-proved. The component (the check itself) is correct, but the system (the scope the caller provides) is wrong.
- **Generalization**: When integrating with an external API (VS Code, LSP, Perl), verify the integration call with the exact scope the API requires. A component test can never prove system correctness when the system's integration point is external.

**Predicate drift**: When a predicate re-parses input a writer already parses, maintain one parser by comparing state snapshots, not by re-parsing a subset.

- **Pattern**: [docs/concepts/verify-the-instrument.md](../concepts/verify-the-instrument.md) (single instrument, not dual measurement)
- **Generalization**: If you implement a change-detection predicate and a config writer, do not let the predicate re-parse; let the writer be the single source of parsed config, and let the predicate observe state before and after.

## Related PRs

- [#3308](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/3308) — native perlcritic config integration with scope and predicate-drift fixes
- [#3276](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/3276) — epic: native-stack product-surface campaign
