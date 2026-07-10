# Neovim Ranged-Typing Latency Receipt — Full Edit-to-Answer Path

**Date**: 2026-07-09
**Branch base**: `origin/main` @ `d6409a100` (`perf(lsp): keep incremental_doc_update off the didChange hot path (#3396) (#3412)`)
**Crate**: `perl-lsp-rs`
**Test**: `crates/perl-lsp-rs/tests/ux_neovim_ranged_typing_latency_receipt.rs::ux_neovim_ranged_typing_medium_file_receipt`
**Lane**: "Fresh Facts Fast" — PR 2 of the LSP-freshness lane (issue #3396)

---

## What this is (and is not)

This is an **informational, debug-build, hardware-dependent instrumentation
receipt** — not a product latency budget or SLA. It exists to make the
current-main edit-to-answer cost visible and reproducible on demand, and to
show the delta that #3412 (keeping `incremental_doc_update` off the
`didChange` hot path by default) produced, on the *same* build and hardware.
CI asserts SHAPE only (a receipt is emitted, each provider returns, one full
parse per ranged edit, `full_parse` span is non-zero) — it never asserts
these millisecond numbers as pass/fail thresholds. See #1373 for why: stable
receipts first, budgets later.

## Reproducible command

```bash
cargo test -p perl-lsp-rs --features expose_lsp_test_api \
    --test ux_neovim_ranged_typing_latency_receipt -- --nocapture
```

A bare `cargo test --test ux_neovim_ranged_typing_latency_receipt` (without
`--features expose_lsp_test_api`) compiles **0 tests** and reports a false
green — the whole file is behind that feature gate. `--nocapture` is
required to see the `PERL_LSP_TIMING_RECEIPT {...}` JSON payloads on stdout
(interleaved with unrelated `publishDiagnostics` JSON-RPC lines the test
harness also prints to stdout in this build — filter for the
`PERL_LSP_TIMING_RECEIPT` marker line and the pretty-printed JSON object that
follows it).

## Scenario shape

- **Fixture**: deterministic ~78 KB synthetic Perl file (`package
  Medium::Fixture;` + 600 small `sub helper_NNNNN { ... }` blocks), 82,145
  bytes as generated.
- **Edit pattern**: 20 **ranged** edits (zero-width `#` insertions at the
  start of the blank line 3) — not full-document replacements. This
  reproduces the realistic Neovim keystroke-by-keystroke edit shape rather
  than the lean-mode full-replace receipts used elsewhere in the corpus.
- **Providers measured after the final edit** (first-response wall time +
  whether it returned): `textDocument/completion`, `textDocument/hover`,
  `textDocument/semanticTokens/full`, `textDocument/references`. All four
  already had `expose_lsp_test_api` test entrypoints
  (`test_handle_completion`, `test_handle_hover`,
  `test_handle_semantic_tokens`, `test_handle_references`) — none needed to
  be added for this PR.
- **Build profile**: debug (`cargo test`, no `--release`).
- **Hardware**: this run's machine — Windows (MSYS2/MinGW64), x86_64, 32
  logical CPUs. Numbers below are specific to this hardware and this debug
  build; do not compare across machines or profiles.

## Before/after measured on current main (this run)

Two scenarios run back-to-back in the same test, against fresh `LspServer`
instances, differing only in `set_incremental_eager(...)`:

- **AFTER** (`incremental_eager = false`) — the current production default
  since #3412: the `didChange` hot path skips eager `incremental_doc`/
  `incremental_state` maintenance entirely.
- **BEFORE** (`incremental_eager = true`) — re-enables the eager
  maintenance #3412 removed from the hot path by default, reproducing the
  pre-#3412 cost model on this same build/hardware.

| Metric | AFTER (eager off, default) | BEFORE (eager on, pre-#3412) | Delta |
|---|---|---|---|
| `didChange.incremental_doc_update` (max span, ms) | 0.004 | 118.959 | +118.955 ms |
| `didChange.total` (max span, ms) | 20.889 | 144.795 | +123.906 ms |
| `didChange.commit` (max span, ms) | 1.821 | 23.707 | +21.886 ms |
| `didChange.full_parse` (avg span, ms) | 9.899 | 11.187 | +1.288 ms (noise) |
| `test_handle_did_change` external wall time (avg, ms) | 67.538 | 138.473 | +70.935 ms |
| `test_handle_did_change` external wall time (max, ms) | 89.618 | 195.455 | +105.837 ms |
| Completion first-response (ms) | 38.335 | 51.131 | +12.796 ms |
| Hover first-response (ms) | 10.173 | 12.095 | +1.922 ms |
| Semantic tokens first-response (ms) | 21.201 | 33.058 | +11.857 ms |
| References first-response (ms) | 13.144 | 15.227 | +2.083 ms |

All four providers returned (`returned: true`) in both scenarios; completion
returned 100 items, semantic tokens returned 14,405 tokens, references
returned 500 locations (workspace-wide `$result` matches — the references
provider is not scoped per-lexical-sub for this synthetic fixture's repeated
variable name, which is expected fixture behavior, not a regression).
`parse_jobs_started` was exactly 20 (one synchronous full parse per ranged
edit) in both scenarios, confirming #3412 did not change the full-parse
cadence — only whether the (unused-on-the-read-path) incremental fields are
eagerly maintained alongside it.

**Reading the delta**: the dominant cost #3412 removed from the default
`didChange` path was the eager `incremental_doc_update` maintenance
(~119 ms of the ~124 ms `didChange.total` delta on this run) — consistent
with the #3412 PR's own claim that this work fed nothing on the read path
(every provider's committed AST comes from the full parse regardless of the
`incremental_eager` flag) while costing roughly an order of magnitude more
than the full parse itself on this fixture size.

## Provider coverage

| Provider | Test entrypoint | Status |
|---|---|---|
| `textDocument/completion` | `test_handle_completion` | Measured |
| `textDocument/hover` | `test_handle_hover` | Measured |
| `textDocument/semanticTokens/full` | `test_handle_semantic_tokens` | Measured |
| `textDocument/references` | `test_handle_references` | Measured |

No providers were descoped for this receipt — all four target entrypoints
already existed behind `expose_lsp_test_api` prior to this PR.

## Scope note

This PR is **test-only + docs**. It adds no production code changes: it
extends an existing test file to drive additional already-existing test
entrypoints and adds this status artifact. The `set_incremental_eager`
toggle used for the BEFORE scenario is pre-existing production API (added
by #3412) exposed specifically for this kind of before/after comparison; it
is not modified here.
