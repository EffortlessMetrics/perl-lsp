# Neovim Lean Latency Status

> Human-owned. Update when the Neovim lean profile, raw-RPC receipts, smoke
> scripts, trace receipts, or benchmark evidence changes.

## Current Claim

The post-cutover lean profile is present in swarm and is intended for sessions
where responsiveness matters more than full semantic/module/critic diagnostics.
Normal mode remains the richer default.

The correct claim boundary is:

- fast lean mode now
- normal rich mode unchanged
- incremental parsing later

## Lean Profile

The lean profile uses the existing runtime dials:

```text
PERL_LSP_E2E=1
PERL_LSP_DIAGNOSTIC_MODE=syntax-only
PERL_LSP_DIAGNOSTIC_DEBOUNCE_MS=0
PERL_LSP_EAGER_WORKSPACE_INDEXING=false
PERL_LSP_FILE_WATCHERS=false
```

Equivalent CLI flags are:

```bash
perllsp --stdio \
  --runtime-mode e2e \
  --diagnostic-mode syntax-only \
  --diagnostic-debounce-ms 0 \
  --eager-workspace-indexing=false \
  --file-watchers=false
```

## Receipts

Raw-RPC wiring receipts live in
[`crates/perl-lsp-ux-tests/tests/ux_latency_raw_rpc.rs`](../../../crates/perl-lsp-ux-tests/tests/ux_latency_raw_rpc.rs).
They exercise:

- open -> completion returns cleanly
- open -> hover returns cleanly
- edit -> parse-error diagnostic surfaces
- edit -> diagnostic clear publishes empty diagnostics
- rapid typing -> latest completion still returns

The manual Neovim smoke lives in
[`scripts/ux/neovim_lean_smoke.sh`](../../../scripts/ux/neovim_lean_smoke.sh).

## What This Proves

These receipts prove e2e wiring, not hard latency budgets. They show that the
server starts in the lean profile and that core editor requests complete without
waiting on full diagnostic or eager-indexing behavior.

## What This Does Not Prove

- No incremental AST reuse is provided by this profile.
- CI wall-clock timing is not a benchmark receipt.
- Full semantic/module/native critic/dead-code diagnostics are not enabled in
  syntax-only mode.
- Additional feature gating should wait for trace evidence showing the feature
  is still on the latency path.

## Next Evidence

- Add a startup trace receipt covering initialize, indexing decision, didOpen,
  first diagnostics, and first completion.
- Add a rapid-typing stale-read pressure receipt that proves older generations
  cancel before taking worker capacity while the latest request still runs.
- Capture benchmark hardware timing before claiming numeric latency budgets.
