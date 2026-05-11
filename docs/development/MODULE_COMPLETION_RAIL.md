# Module Completion Latency Burndown

> **Substrate (already built)**: prefix-directed module scan landed in PR #8498 (closed parent #8491); shared `EffectiveIncContext` is now threaded through module-resolution consumers in `crates/perl-module/` and `crates/perl-lsp-rs-core/`.
> **Connector gap**: a runtime-owned short TTL cache for prefix scans. `CompletionProvider` is reconstructed per request, so any in-provider cache is lost between keystrokes. The cache must move out of the provider and live where it can outlive a single request.
> **0.14.0 upside**: typing `use Mojo::Cont|` stops re-scanning `root/Mojo/` on every keystroke. The prefix-directed scan from #8498 already prevents re-scanning from the include root, but a many-tens-of-MB prefix subdir still gets rescanned per keystroke without a TTL cache.

## Status

| Phase | Issue | Builder-ready? | PR | Receipt |
|---|---|---|---|---|
| 1. Runtime-owned short TTL cache for prefix scans | [#8514](https://github.com/EffortlessMetrics/perl-lsp/issues/8514) | yes (`builder-ready`) | _pending_ | `cargo test -p perl-lsp-rs-core --lib completion -- --nocapture --test-threads=2` |

## Exit criteria

- [ ] All phases land or are explicitly deferred with a successor.
- [ ] Receipt command in this doc reproduces the closeout proof.
- [ ] Status doc updated (`docs/project/status/lsp.md` regenerated post-merge).
- [ ] Claim boundary recorded.

## Claim boundary

This rail proves that **multi-segment `use` completion does not re-walk a prefix subdir on every keystroke**: a runtime-owned TTL cache absorbs repeated lookups within the cache window.

This rail does **NOT** prove:

- The cache is correct under filesystem mutation faster than its TTL (handled separately by invalidation policy in #8514's design notes).
- The cache helps for non-prefix completion paths (qualifier resolution, sub completion, etc.).
- The cache solves cold-start scan cost on first keystroke after a session opens.

## Receipts

```bash
# Phase 1 closeout
cargo test -p perl-lsp-rs-core --lib completion -- --nocapture --test-threads=2
```

The test under `crates/perl-lsp-rs-core/` named in #8514's implementation contract must exercise the runtime-owned cache by reusing a `CompletionProvider` across two requests and asserting only one scan occurs within the TTL window.

## Related

- Umbrella issue: [#8514 — perf(completion): runtime-owned short TTL cache for prefix module scans](https://github.com/EffortlessMetrics/perl-lsp/issues/8514)
- Tracker for this rollout doc: #8625
- Substrate PR (merged): [#8498](https://github.com/EffortlessMetrics/perl-lsp/pull/8498)
- Original combined issue (closed): [#8491](https://github.com/EffortlessMetrics/perl-lsp/issues/8491)
- Architecture / spec docs: `crates/perl-lsp-rs-core/src/runtime/language/completion.rs` (the file from which the cache must be lifted)
- Status doc: [docs/project/status/lsp.md](../project/status/lsp.md)
- Adjacent rails:
  - `IMPORTS_RAIL.md` — also touches module-resolution completion paths
  - `REAL_WORKSPACE_BASELINE_RAIL.md` — provider-level coverage that this rail must not regress

## Do not combine

Do **not** roll this rail's PRs into:

- Provider-architecture refactors (separate concern: where things live, not how fast they are).
- Cache invalidation or filesystem-watcher work (handled separately; this rail is TTL-only).
- Any other completion-latency rail. One cache per rail; combining caches across module/sub/variable completion blurs ownership and makes invalidation reasoning impossible.

## Lane assignment

**Builder (sonnet)** — driven by the implementation contract in #8514. The contract is already detailed enough for direct build; no additional plan-review is needed (`builder-ready` already set).
